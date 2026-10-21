# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|

## WebSocket round trip, pp-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| s1-full | 1503631 | 38.61 | 47.99 | 53.99 | 99.46 | 252.36 | 151.5 | 0.0 | 0.0 | 0.0 | 33.71 | 41.0 | neither loop stalled long enough: network, hypervisor or untraced |
| s1-full-sqtick | 1586809 | 36.64 | 45.72 | 57.75 | 71.33 | 171.69 | 61.0 | 0.0 | 0.0 | 0.0 | 37.89 | 22.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| s2-full | 1574227 | 37.05 | 43.90 | 50.91 | 65.88 | 431.57 | 54.9 | 0.0 | 0.0 | 0.0 | 43.00 | 130.0 | neither loop stalled long enough: network, hypervisor or untraced |
| s2-full-sqtick | 1593484 | 36.57 | 44.05 | 53.50 | 71.06 | 139.86 | 58.0 | 0.0 | 0.0 | 0.0 | 31.98 | 53.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| s1-full | 1189999 | 37.28 | 43.86 | 53.73 | 69.93 | 375.71 | 9.6 | 0.0 | 0.0 | 0.0 | 65.73 | 213.5 | neither loop stalled long enough: network, hypervisor or untraced |
| s1-full-sqtick | 1189999 | 37.97 | 49.14 | 57.17 | 91.72 | 294.54 | 40.8 | 0.0 | 0.0 | 0.0 | 64.47 | 7.0 | neither loop stalled long enough: network, hypervisor or untraced |
| s2-full | 1189999 | 38.40 | 45.45 | 56.66 | 69.46 | 199.27 | 18.2 | 0.0 | 0.0 | 0.0 | 254.77 | 290.5 | neither loop stalled long enough: network, hypervisor or untraced |
| s2-full-sqtick | 1189999 | 37.77 | 43.73 | 51.35 | 71.59 | 267.60 | 24.2 | 0.0 | 0.0 | 0.0 | 64.48 | 25.5 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| s1-full | 1189999 | 37.58 | 44.17 | 54.10 | 70.73 | 376.16 | 13.6 | 0.0 | 0.0 | 0.0 |
| s1-full-sqtick | 1189999 | 38.27 | 49.46 | 57.57 | 105.86 | 1528.58 | 66.1 | 31.3 | 26.2 | 0.0 |
| s2-full | 1189999 | 38.70 | 45.76 | 57.01 | 70.64 | 258.87 | 21.2 | 0.0 | 0.0 | 0.0 |
| s2-full-sqtick | 1189999 | 38.06 | 44.03 | 51.71 | 72.50 | 267.88 | 25.2 | 0.0 | 0.0 | 0.0 |

## WebSocket round trip, pp-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| s1-full | 1502548 | 38.51 | 47.51 | 54.89 | 97.93 | 1036.93 | 147.5 | 1.0 | 1.0 | 0.0 | 28.88 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |
| s1-full-sqtick | 1530842 | 37.84 | 45.48 | 53.09 | 81.39 | 237.89 | 103.7 | 0.0 | 0.0 | 0.0 | 33.54 | 1.0 | server: softirq TIMER |
| s2-full | 1513131 | 38.35 | 46.01 | 55.31 | 75.46 | 296.04 | 70.2 | 0.0 | 0.0 | 0.0 | 28.60 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |
| s2-full-sqtick | 1538091 | 37.63 | 47.23 | 56.44 | 76.55 | 193.20 | 92.5 | 0.0 | 0.0 | 0.0 | 27.52 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| s1-full | 1189998 | 38.98 | 45.68 | 51.21 | 65.93 | 154.40 | 8.6 | 0.0 | 0.0 | 0.0 | 42.72 | 1.0 | server: softirq TIMER |
| s1-full-sqtick | 1189998 | 38.75 | 45.12 | 53.70 | 77.67 | 188.54 | 18.7 | 0.0 | 0.0 | 0.0 | 34.66 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |
| s2-full | 1189998 | 38.91 | 46.90 | 88.24 | 172.56 | 945.39 | 347.9 | 1.0 | 0.0 | 0.0 | 28.11 | 1.5 | neither loop stalled long enough: network, hypervisor or untraced |
| s2-full-sqtick | 1189998 | 38.84 | 44.75 | 54.16 | 89.75 | 508.03 | 40.3 | 0.5 | 0.0 | 0.0 | 35.19 | 0.5 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-uring-sqpoll-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| s1-full | 1189998 | 39.05 | 45.75 | 51.30 | 66.32 | 154.45 | 8.6 | 0.0 | 0.0 | 0.0 |
| s1-full-sqtick | 1189998 | 38.86 | 45.24 | 53.93 | 78.29 | 274.75 | 20.7 | 0.0 | 0.0 | 0.0 |
| s2-full | 1189998 | 39.00 | 47.12 | 145.01 | 899.31 | 1198.59 | 799.2 | 290.9 | 27.7 | 0.0 |
| s2-full-sqtick | 1189998 | 38.99 | 44.91 | 54.53 | 91.50 | 508.20 | 43.9 | 0.5 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs. The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | receiving CPU | LOC | IWI | RES | CAL |
|---|---|---|---|---|---|---|---|---|---|---|---|
| pp-recv-busy-0 | s1-full | 3 | 42 | 41 | 0 | 16 | 0 | 1142 | 0 | 1 | 22 |
| pp-recv-busy-0 | s1-full-sqtick | 3 | 40 | 39 | 0 | 16 | 4 | 1012 | 0 | 0 | 11 |
| pp-recv-busy-0 | s2-full | 3 | 43 | 42 | 0 | 16 | | | | | | |
| pp-recv-busy-0 | s2-full-sqtick | 3 | 45 | 45 | 0 | 16 | 4 | 1013 | 0 | 1 | 13 |
| pp-recv-busy-10000 | s1-full | 3 | 15 | 15 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | s1-full-sqtick | 3 | 32 | 32 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | s2-full | 3 | 23 | 23 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | s2-full-sqtick | 3 | 24 | 24 | 0 | 8 | | | | | | |
| pp-uring-sqpoll-0 | s1-full | 3 | 0 | 0 | 0 | 17 | 2 | 85 | 41 | 1 | 17 |
| pp-uring-sqpoll-0 | s1-full-sqtick | 3 | 0 | 0 | 0 | 17 | 2 | 1003 | 0 | 0 | 17 |
| pp-uring-sqpoll-0 | s2-full | 3 | 0 | 0 | 0 | 17 | 2 | 92 | 43 | 1 | 17 |
| pp-uring-sqpoll-0 | s2-full-sqtick | 3 | 0 | 0 | 0 | 17 | 2 | 1003 | 0 | 0 | 17 |
| pp-uring-sqpoll-10000 | s1-full | 3 | 0 | 0 | 0 | 8 | 2 | 54 | 24 | 1 | 8 |
| pp-uring-sqpoll-10000 | s1-full-sqtick | 3 | 0 | 0 | 0 | 8 | 2 | 1001 | 0 | 0 | 8 |
| pp-uring-sqpoll-10000 | s2-full | 3 | 0 | 0 | 0 | 8 | 2 | 61 | 27 | 1 | 8 |
| pp-uring-sqpoll-10000 | s2-full-sqtick | 3 | 0 | 0 | 0 | 8 | 2 | 1001 | 0 | 0 | 8 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### pp-recv-busy-0

| condition | worst events |
|---|---|
| s1-full | 252.36 at 49.6 s: neither loop stalled long enough: network, hypervisor or untraced; 163.49 at 39.8 s: neither loop stalled long enough: network, hypervisor or untraced; 161.88 at 35.9 s: neither loop stalled long enough: network, hypervisor or untraced; 152.40 at 29.8 s: neither loop stalled long enough: network, hypervisor or untraced; 151.89 at 41.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| s1-full-sqtick | 171.69 at 57.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 154.07 at 15.0 s: neither loop stalled long enough: network, hypervisor or untraced; 138.03 at 15.0 s: neither loop stalled long enough: network, hypervisor or untraced; 132.71 at 7.1 s: neither loop stalled long enough: network, hypervisor or untraced; 131.46 at 40.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| s2-full | 431.57 at 17.6 s: neither loop stalled long enough: network, hypervisor or untraced; 137.05 at 25.3 s: neither loop stalled long enough: network, hypervisor or untraced; 129.17 at 58.3 s: neither loop stalled long enough: network, hypervisor or untraced; 125.88 at 41.6 s: neither loop stalled long enough: network, hypervisor or untraced; 125.02 at 37.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| s2-full-sqtick | 139.86 at 29.9 s: neither loop stalled long enough: network, hypervisor or untraced; 134.19 at 47.0 s: server: softirq TIMER; 124.70 at 43.1 s: neither loop stalled long enough: network, hypervisor or untraced; 124.60 at 26.6 s: neither loop stalled long enough: network, hypervisor or untraced; 124.56 at 5.5 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| s1-full | 375.71 at 91.2 s: neither loop stalled long enough: network, hypervisor or untraced; 357.26 at 48.0 s: neither loop stalled long enough: network, hypervisor or untraced; 120.40 at 87.3 s: neither loop stalled long enough: network, hypervisor or untraced; 116.09 at 80.5 s: neither loop stalled long enough: network, hypervisor or untraced; 113.15 at 98.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| s1-full-sqtick | 294.54 at 112.1 s: neither loop stalled long enough: network, hypervisor or untraced; 279.24 at 112.1 s: neither loop stalled long enough: network, hypervisor or untraced; 265.37 at 112.1 s: neither loop stalled long enough: network, hypervisor or untraced; 237.44 at 112.1 s: neither loop stalled long enough: network, hypervisor or untraced; 222.43 at 112.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| s2-full | 199.27 at 96.4 s: neither loop stalled long enough: network, hypervisor or untraced; 195.59 at 69.0 s: server: softirq TIMER; 144.52 at 25.3 s: neither loop stalled long enough: network, hypervisor or untraced; 135.81 at 108.4 s: neither loop stalled long enough: network, hypervisor or untraced; 125.30 at 59.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| s2-full-sqtick | 267.60 at 90.4 s: neither loop stalled long enough: network, hypervisor or untraced; 133.14 at 49.4 s: client: softirq TIMER; 123.28 at 23.3 s: neither loop stalled long enough: network, hypervisor or untraced; 122.25 at 1.1 s: neither loop stalled long enough: network, hypervisor or untraced; 120.59 at 110.4 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-0

| condition | worst events |
|---|---|
| s1-full | 1036.93 at 55.7 s: neither loop stalled long enough: network, hypervisor or untraced; 215.22 at 43.4 s: neither loop stalled long enough: network, hypervisor or untraced; 175.33 at 39.3 s: neither loop stalled long enough: network, hypervisor or untraced; 174.75 at 43.3 s: neither loop stalled long enough: network, hypervisor or untraced; 171.60 at 3.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| s1-full-sqtick | 237.89 at 24.1 s: server: softirq TIMER; 164.02 at 44.8 s: neither loop stalled long enough: network, hypervisor or untraced; 158.12 at 58.8 s: neither loop stalled long enough: network, hypervisor or untraced; 155.51 at 29.8 s: neither loop stalled long enough: network, hypervisor or untraced; 153.89 at 12.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| s2-full | 296.04 at 24.3 s: neither loop stalled long enough: network, hypervisor or untraced; 181.53 at 11.8 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 131.98 at 44.6 s: neither loop stalled long enough: network, hypervisor or untraced; 127.59 at 39.6 s: neither loop stalled long enough: network, hypervisor or untraced; 126.59 at 35.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| s2-full-sqtick | 193.20 at 31.8 s: neither loop stalled long enough: network, hypervisor or untraced; 171.58 at 38.8 s: neither loop stalled long enough: network, hypervisor or untraced; 155.07 at 39.8 s: neither loop stalled long enough: network, hypervisor or untraced; 153.22 at 10.8 s: neither loop stalled long enough: network, hypervisor or untraced; 151.10 at 16.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-10000

| condition | worst events |
|---|---|
| s1-full | 154.40 at 39.5 s: server: softirq TIMER; 116.41 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 115.56 at 95.2 s: neither loop stalled long enough: network, hypervisor or untraced; 115.29 at 33.0 s: neither loop stalled long enough: network, hypervisor or untraced; 112.63 at 15.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| s1-full-sqtick | 188.54 at 8.1 s: neither loop stalled long enough: network, hypervisor or untraced; 185.81 at 8.1 s: neither loop stalled long enough: network, hypervisor or untraced; 128.73 at 104.9 s: neither loop stalled long enough: network, hypervisor or untraced; 128.22 at 54.9 s: neither loop stalled long enough: network, hypervisor or untraced; 126.78 at 38.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| s2-full | 945.39 at 83.9 s: neither loop stalled long enough: network, hypervisor or untraced; 937.51 at 13.8 s: neither loop stalled long enough: network, hypervisor or untraced; 415.32 at 52.6 s: neither loop stalled long enough: network, hypervisor or untraced; 384.81 at 52.8 s: neither loop stalled long enough: network, hypervisor or untraced; 377.92 at 52.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| s2-full-sqtick | 508.03 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced; 165.26 at 17.2 s: neither loop stalled long enough: network, hypervisor or untraced; 159.93 at 115.2 s: neither loop stalled long enough: network, hypervisor or untraced; 158.23 at 102.4 s: server: softirq TIMER; 137.28 at 19.2 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| s1-full | mostly unexplained (+preempted by kworker/3:1) 6797.04; mostly unexplained (+softirq NET_RX) 2899.10; softirq NET_RX 1109.26; VM paused: the idle spare core stalled too (hypervisor) 187.30; unexplained: no kernel event (hypervisor or in-kernel time) 163.65 |
| s1-full-sqtick | mostly unexplained (+preempted by kworker/3:1) 354.67; VM paused: the idle spare core stalled too (hypervisor) 185.45; mostly unexplained (+irq work (IPI/vector)) 117.68; mostly unexplained (+softirq NET_RX) 113.75; unexplained: no kernel event (hypervisor or in-kernel time) 77.06 |
| s2-full | mostly unexplained (+preempted by kworker/3:1) 11399.08; mostly unexplained (+softirq NET_RX) 2919.06; softirq NET_RX 1470.10; softirq TIMER 254.77; mostly unexplained (+local timer tick) 209.49 |
| s2-full-sqtick | mostly unexplained (+preempted by kworker/3:1) 1389.23; mostly unexplained (+softirq NET_RX) 319.25; softirq NET_RX 222.75; VM paused: the idle spare core stalled too (hypervisor) 168.56; unexplained: no kernel event (hypervisor or in-kernel time) 121.69 |

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
| pp-recv-busy-0 | idle spare core | 0 | | | | | |
| pp-recv-busy-0 | receiver loop | 0 | | | | | |
| pp-recv-busy-0 | sender loop | 0 | | | | | |
| pp-recv-busy-0 | slow samples | 1 | :13 | 1 (100%) | 1 of 5 | 431.57 | no |
| pp-recv-busy-10000 | idle spare core | 1 | :20 | 1 (100%) | 1 of 8 | 304.67 | no |
| pp-recv-busy-10000 | receiver loop | 0 | | | | | |
| pp-recv-busy-10000 | sender loop | 0 | | | | | |
| pp-recv-busy-10000 | slow samples | 2 | :32 | 1 (50%) | 1 of 8 | 375.71 | no |
| pp-uring-sqpoll-0 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-0 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-0 | slow samples | 1 | :29 | 1 (100%) | 1 of 4 | 1036.93 | no |
| pp-uring-sqpoll-10000 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-10000 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-10000 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-10000 | slow samples | 11 | :29 | 8 (73%) | 1 of 8 | 415.32 | **yes** |

# Every run in detail

### Loop counters (s1-full/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1529251 | 1529251 | 1.00 | 109523684 | 71.62 | 1825395 | 1799907 | 98.6 | 71.8 | true | 0 | 2449 | 33.7 | 49828 / 41 / 0 | 0 | 0 |

#### Stalls: s1-full/pp-recv-busy-0, loop 0 on CPU 3

107994432 iterations, longest 33.7 µs; over 5 / 20 / 100 µs: 49828 / 41 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 18 | 385.1 | 28.1 |
| softirq NET_RX | 6 | 133.3 | 25.5 |
| mostly unexplained (+softirq NET_RX) | 6 | 130.0 | 22.8 |
| mostly unexplained (+local timer tick) | 5 | 103.4 | 21.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 91.8 | 24.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.7 | 33.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 32.9 | 32.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.7 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 32.9 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, softirq NET_RX 1, ipi_send_cpu 0 |
| 3 | 28.1 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 25.5 | 0 | softirq NET_RX | softirq NET_RX 13, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 24.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 24.2 | 1 | softirq NET_RX | softirq NET_RX 19, local timer tick 4, timer tick_nohz_handler 3, preempted by kworker/3:1 2 |
| 7 | 24.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.1 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 23.0 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 22.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2493 |
| IWI IRQ work interrupts | 2454 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: s1-full/pp-recv-busy-0 (median 38.61)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 252.36 | 213.75 | 0.00 |  | 0.00 |  | 213.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 163.49 | 124.88 | 0.00 |  | 0.00 |  | 124.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 161.88 | 123.28 | 0.00 |  | 0.00 |  | 123.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 152.40 | 113.79 | 0.00 |  | 0.00 |  | 113.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 151.89 | 113.28 | 0.00 |  | 0.00 |  | 113.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 151.68 | 113.07 | 0.00 |  | 0.00 |  | 113.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 151.56 | 112.95 | 0.00 |  | 0.00 |  | 112.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 149.54 | 110.93 | 0.00 |  | 0.00 |  | 110.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 148.91 | 110.30 | 0.00 |  | 0.00 |  | 110.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 148.55 | 109.94 | 0.00 |  | 0.00 |  | 109.94 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s1-full/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 235788992 | 196.49 | 1964908 | 1954908 | 99.5 | 67.8 | true | 0 | 1806 | 65.7 | 4474 / 427 / 0 | 0 | 0 |

#### Stalls: s1-full/pp-recv-busy-10000, loop 0 on CPU 3

234588992 iterations, longest 65.7 µs; over 5 / 20 / 100 µs: 4474 / 427 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 265 | 6412.0 | 31.0 |
| mostly unexplained (+softirq NET_RX) | 111 | 2769.1 | 31.0 |
| softirq NET_RX | 43 | 975.9 | 29.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 71.9 | 30.3 |
| softirq TIMER | 1 | 65.7 | 65.7 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 64.7 | 42.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 47.3 | 26.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 65.7 | 0 | softirq TIMER | softirq TIMER 53, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 42.3 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 31.0 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 31.0 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 30.5 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 30.3 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 30.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 29.6 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 29.3 | 0 | softirq NET_RX | softirq NET_RX 15, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 29.2 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1814 |
| LOC Local timer interrupts | 1809 |
| CAL Function call interrupts | 1001 |
| MCP Machine check polls | 1 |

#### Slowest round trips: s1-full/pp-recv-busy-10000 (median 37.28)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 375.71 | 338.43 | 0.00 |  | 0.00 |  | 338.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 357.26 | 319.98 | 65.73 | softirq TIMER | 0.00 |  | 254.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 120.40 | 83.12 | 0.00 |  | 0.00 |  | 83.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 116.09 | 78.81 | 0.00 |  | 0.00 |  | 78.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 113.15 | 75.87 | 0.00 |  | 0.00 |  | 75.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 112.28 | 75.00 | 0.00 |  | 0.00 |  | 75.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 112.26 | 74.98 | 0.00 |  | 0.00 |  | 74.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 112.09 | 74.81 | 0.00 |  | 0.00 |  | 74.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 111.47 | 74.19 | 0.00 |  | 0.00 |  | 74.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 110.51 | 73.23 | 0.00 |  | 0.00 |  | 73.23 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s1-full/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1528106 | 1528106 | 1.00 | 1 | 0.00 | 0 | 34101203 | 0.0 | 0.0 | true | 0 | 0 | 28.9 | 50 / 1 / 0 | 0 | 0 |

#### Stalls: s1-full/pp-uring-sqpoll-0, loop 0 on CPU 3

2046072192 iterations, longest 28.9 µs; over 5 / 20 / 100 µs: 50 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.9 | 28.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 28.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: s1-full/pp-uring-sqpoll-0 (median 38.51)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1036.93 | 998.42 | 0.00 |  | 0.00 |  | 998.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 215.22 | 176.71 | 0.00 |  | 0.00 |  | 176.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 175.33 | 136.83 | 0.00 |  | 0.00 |  | 136.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 174.75 | 136.24 | 0.00 |  | 0.00 |  | 136.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 171.60 | 133.09 | 0.00 |  | 0.00 |  | 133.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 164.04 | 125.53 | 0.00 |  | 0.00 |  | 125.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 163.24 | 124.73 | 0.00 |  | 0.00 |  | 124.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 159.83 | 121.32 | 0.00 |  | 0.00 |  | 121.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 159.79 | 121.28 | 0.00 |  | 0.00 |  | 121.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 159.61 | 121.10 | 0.00 |  | 0.00 |  | 121.10 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s1-full/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 28326938 | 0.0 | 0.0 | true | 0 | 0 | 42.7 | 70 / 2 / 0 | 0 | 0 |

#### Stalls: s1-full/pp-uring-sqpoll-10000, loop 0 on CPU 3

3399232576 iterations, longest 42.7 µs; over 5 / 20 / 100 µs: 70 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 77.4 | 42.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 42.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 34.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: s1-full/pp-uring-sqpoll-10000 (median 38.98)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 154.40 | 115.41 | 0.00 |  | 150.89 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 116.41 | 77.43 | 0.00 |  | 0.00 |  | 77.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 115.56 | 76.58 | 0.00 |  | 0.00 |  | 76.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 115.29 | 76.30 | 0.00 |  | 0.00 |  | 76.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 112.63 | 73.64 | 0.00 |  | 0.00 |  | 73.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 111.64 | 72.66 | 0.00 |  | 0.00 |  | 72.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 110.78 | 71.79 | 0.00 |  | 0.00 |  | 71.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 110.54 | 71.56 | 0.00 |  | 0.00 |  | 71.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 108.01 | 69.02 | 0.00 |  | 0.00 |  | 69.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 107.92 | 68.93 | 0.00 |  | 0.00 |  | 68.93 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s1-full-sqtick/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1613974 | 1613974 | 1.00 | 108463703 | 67.20 | 1807728 | 1780829 | 98.5 | 71.8 | true | 0 | 2359 | 37.9 | 48083 / 22 / 0 | 0 | 0 |

#### Stalls: s1-full-sqtick/pp-recv-busy-0, loop 0 on CPU 3

106849728 iterations, longest 37.9 µs; over 5 / 20 / 100 µs: 48083 / 22 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 6 | 148.4 | 37.9 |
| mostly unexplained (+softirq NET_RX) | 5 | 113.8 | 27.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 77.1 | 28.0 |
| softirq NET_RX | 3 | 74.9 | 26.4 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 51.8 | 30.5 |
| mostly unexplained (+local timer tick) | 2 | 48.8 | 25.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 36.8 | 36.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 37.9 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 15, workqueue [ena] 10, local timer tick 5, timer tick_nohz_handler 3 |
| 2 | 36.8 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 30.5 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 28.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 27.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 3, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 26.4 | 0 | softirq NET_RX | softirq NET_RX 14, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 25.6 | 0 | softirq NET_RX | softirq NET_RX 13, preempted by ksoftirqd/3 5, local timer tick 4, timer tick_nohz_handler 3 |
| 8 | 25.6 | 1 | mostly unexplained (+local timer tick) | local timer tick 4, preempted by kworker/3:1 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 25.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 24.5 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2399 |
| IWI IRQ work interrupts | 2366 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: s1-full-sqtick/pp-recv-busy-0 (median 36.64)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 171.69 | 135.05 | 0.00 |  | 134.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.36 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 154.07 | 117.43 | 0.00 |  | 0.00 |  | 117.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 138.03 | 101.39 | 0.00 |  | 0.00 |  | 101.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 132.71 | 96.06 | 0.00 |  | 0.00 |  | 96.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 131.46 | 94.82 | 0.00 |  | 0.00 |  | 94.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 129.02 | 92.38 | 0.00 |  | 0.00 |  | 92.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 128.53 | 91.89 | 0.00 |  | 0.00 |  | 91.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 126.76 | 90.12 | 0.00 |  | 0.00 |  | 90.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 126.31 | 89.67 | 0.00 |  | 0.00 |  | 89.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 125.65 | 89.01 | 0.00 |  | 0.00 |  | 89.01 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s1-full-sqtick/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 233631872 | 194.69 | 1946932 | 1936932 | 99.5 | 67.9 | true | 0 | 3868 | 64.5 | 35972 / 14 / 0 | 0 | 0 |

#### Stalls: s1-full-sqtick/pp-recv-busy-10000, loop 0 on CPU 3

232431872 iterations, longest 64.5 µs; over 5 / 20 / 100 µs: 35972 / 14 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 9 | 206.3 | 32.8 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 65.9 | 33.2 |
| softirq TIMER | 1 | 64.5 | 64.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 60.0 | 35.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 64.5 | 0 | softirq TIMER | softirq TIMER 51, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 35.3 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 33.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 32.8 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 32.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, softirq NET_RX 1, ipi_send_cpu 0 |
| 6 | 27.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 24.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 22.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 21.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 21.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3881 |
| IWI IRQ work interrupts | 3876 |
| CAL Function call interrupts | 1002 |
| MCP Machine check polls | 1 |

#### Slowest round trips: s1-full-sqtick/pp-recv-busy-10000 (median 37.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 294.54 | 256.57 | 0.00 |  | 0.00 |  | 256.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 279.24 | 241.26 | 0.00 |  | 0.00 |  | 241.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 265.37 | 227.40 | 0.00 |  | 0.00 |  | 227.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 237.44 | 199.46 | 0.00 |  | 0.00 |  | 199.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 222.43 | 184.45 | 0.00 |  | 0.00 |  | 184.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 212.76 | 174.79 | 0.00 |  | 0.00 |  | 174.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 179.48 | 141.51 | 0.00 |  | 0.00 |  | 141.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 178.25 | 140.28 | 0.00 |  | 0.00 |  | 140.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 173.14 | 135.16 | 0.00 |  | 0.00 |  | 135.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 165.12 | 127.14 | 0.00 |  | 0.00 |  | 127.14 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s1-full-sqtick/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1557021 | 1557021 | 1.00 | 1 | 0.00 | 0 | 23679167 | 0.0 | 0.0 | true | 0 | 0 | 33.5 | 45 / 1 / 0 | 0 | 0 |

#### Stalls: s1-full-sqtick/pp-uring-sqpoll-0, loop 0 on CPU 3

1420750016 iterations, longest 33.5 µs; over 5 / 20 / 100 µs: 45 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.5 | 33.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: s1-full-sqtick/pp-uring-sqpoll-0 (median 37.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 237.89 | 200.05 | 0.00 |  | 153.28 | softirq TIMER | 46.77 | server: softirq TIMER |
| 2 | 164.02 | 126.19 | 0.00 |  | 0.00 |  | 126.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 158.12 | 120.28 | 0.00 |  | 0.00 |  | 120.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 155.51 | 117.67 | 0.00 |  | 25.09 | mostly unexplained (+preempted by kworker/1:0) | 92.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 153.89 | 116.05 | 0.00 |  | 0.00 |  | 116.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 153.77 | 115.94 | 0.00 |  | 0.00 |  | 115.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 151.49 | 113.65 | 0.00 |  | 0.00 |  | 113.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 148.09 | 110.25 | 0.00 |  | 0.00 |  | 110.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 148.04 | 110.21 | 0.00 |  | 0.00 |  | 110.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 146.88 | 109.04 | 0.00 |  | 0.00 |  | 109.04 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s1-full-sqtick/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 13760115 | 0.0 | 0.0 | true | 0 | 0 | 34.7 | 89 / 2 / 0 | 0 | 0 |

#### Stalls: s1-full-sqtick/pp-uring-sqpoll-10000, loop 0 on CPU 3

1651213824 iterations, longest 34.7 µs; over 5 / 20 / 100 µs: 89 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.1 | 34.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 34.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 20.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: s1-full-sqtick/pp-uring-sqpoll-10000 (median 38.75)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 188.54 | 149.78 | 0.00 |  | 0.00 |  | 149.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 185.81 | 147.05 | 0.00 |  | 0.00 |  | 147.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 128.73 | 89.98 | 0.00 |  | 0.00 |  | 89.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 128.22 | 89.47 | 0.00 |  | 0.00 |  | 89.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 126.78 | 88.03 | 0.00 |  | 0.00 |  | 88.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 123.89 | 85.14 | 0.00 |  | 0.00 |  | 85.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 121.96 | 83.21 | 0.00 |  | 0.00 |  | 83.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 121.53 | 82.78 | 0.00 |  | 0.00 |  | 82.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 121.36 | 82.60 | 0.00 |  | 0.00 |  | 82.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 120.26 | 81.50 | 0.00 |  | 0.00 |  | 81.50 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s2-full/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1601105 | 1601105 | 1.00 | 107988306 | 67.45 | 1799805 | 1773120 | 98.5 | 71.9 | true | 0 | 2524 | 43.0 | 48148 / 130 / 0 | 0 | 0 |

#### Stalls: s2-full/pp-recv-busy-0, loop 0 on CPU 3

106387200 iterations, longest 43.0 µs; over 5 / 20 / 100 µs: 48148 / 130 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 75 | 1683.8 | 43.0 |
| mostly unexplained (+softirq NET_RX) | 27 | 606.2 | 30.2 |
| softirq NET_RX | 12 | 254.0 | 23.5 |
| mostly unexplained (+local timer tick) | 6 | 136.1 | 26.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 5 | 118.8 | 27.1 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 104.5 | 27.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.4 | 28.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 43.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 33.6 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 30.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 10, local timer tick 4, preempted by kworker/3:1 3, irq work (IPI/vector) 1 |
| 4 | 28.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 28.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 4 |
| 6 | 27.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 27.6 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 27.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 26.9 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 26.7 | 0 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 3, preempted by kworker/3:1 2, irq work (IPI/vector) 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2552 |
| IWI IRQ work interrupts | 2531 |
| CAL Function call interrupts | 989 |

#### Slowest round trips: s2-full/pp-recv-busy-0 (median 37.05)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 431.57 | 394.52 | 0.00 |  | 0.00 |  | 394.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 137.05 | 100.00 | 0.00 |  | 0.00 |  | 100.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 129.17 | 92.12 | 0.00 |  | 0.00 |  | 92.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 125.88 | 88.83 | 0.00 |  | 0.00 |  | 88.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 125.02 | 87.97 | 0.00 |  | 0.00 |  | 87.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 122.62 | 85.58 | 0.00 |  | 0.00 |  | 85.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 120.68 | 83.64 | 0.00 |  | 0.00 |  | 83.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 119.89 | 82.85 | 0.00 |  | 0.00 |  | 82.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 119.75 | 82.70 | 0.00 |  | 0.00 |  | 82.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 119.00 | 81.96 | 0.00 |  | 0.00 |  | 81.96 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s2-full/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 231737856 | 193.11 | 1931149 | 1921149 | 99.5 | 68.2 | true | 0 | 2773 | 254.8 | 25925 / 581 / 1 | 0 | 0 |

#### Stalls: s2-full/pp-recv-busy-10000, loop 0 on CPU 3

230537856 iterations, longest 254.8 µs; over 5 / 20 / 100 µs: 25925 / 581 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 420 | 9715.3 | 37.9 |
| mostly unexplained (+softirq NET_RX) | 99 | 2312.8 | 36.5 |
| softirq NET_RX | 54 | 1216.1 | 49.7 |
| softirq TIMER | 1 | 254.8 | 254.8 |
| mostly unexplained (+local timer tick) | 2 | 73.4 | 39.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 53.6 | 28.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 44.6 | 23.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 36.7 | 36.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 254.8 | 0 | softirq TIMER | softirq TIMER 240, local timer tick 8, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 49.7 | 0 | softirq NET_RX | softirq NET_RX 27, preempted by kworker/3:1 10, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 39.7 | 1 | mostly unexplained (+local timer tick) | local timer tick 4, preempted by kworker/3:1 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 4 | 37.9 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, local timer tick 4, irq work (IPI/vector) 1, softirq TIMER 1 |
| 5 | 36.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, softirq NET_RX 1, ipi_send_cpu 0 |
| 6 | 36.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/3:1 6, local timer tick 4, irq work (IPI/vector) 1 |
| 7 | 33.7 | 0 | mostly unexplained (+local timer tick) | local timer tick 4, preempted by kworker/3:1 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 8 | 31.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 31.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 31.0 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 13, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2814 |
| IWI IRQ work interrupts | 2780 |
| CAL Function call interrupts | 1003 |
| MCP Machine check polls | 1 |

#### Slowest round trips: s2-full/pp-recv-busy-10000 (median 38.40)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 199.27 | 160.87 | 0.00 |  | 0.00 |  | 160.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 195.59 | 157.19 | 0.00 |  | 163.33 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 144.52 | 106.12 | 0.00 |  | 0.00 |  | 106.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 135.81 | 97.41 | 0.00 |  | 0.00 |  | 97.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 125.30 | 86.89 | 0.00 |  | 0.00 |  | 86.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 120.70 | 82.30 | 0.00 |  | 0.00 |  | 82.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 120.42 | 82.02 | 0.00 |  | 0.00 |  | 82.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 119.87 | 81.47 | 0.00 |  | 0.00 |  | 81.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 118.19 | 79.79 | 0.00 |  | 0.00 |  | 79.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 117.96 | 79.56 | 0.00 |  | 0.00 |  | 79.56 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s2-full/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1538945 | 1538945 | 1.00 | 1 | 0.00 | 0 | 36179861 | 0.0 | 0.0 | true | 0 | 0 | 28.6 | 70 / 1 / 0 | 0 | 0 |

#### Stalls: s2-full/pp-uring-sqpoll-0, loop 0 on CPU 3

2170791680 iterations, longest 28.6 µs; over 5 / 20 / 100 µs: 70 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.6 | 28.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 28.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: s2-full/pp-uring-sqpoll-0 (median 38.35)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 296.04 | 257.70 | 0.00 |  | 22.89 | mostly unexplained (+preempted by kworker/1:0) | 234.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 181.53 | 143.18 | 0.00 |  | 156.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 131.98 | 93.63 | 0.00 |  | 0.00 |  | 93.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 127.59 | 89.24 | 0.00 |  | 0.00 |  | 89.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 126.59 | 88.24 | 0.00 |  | 0.00 |  | 88.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 126.27 | 87.92 | 0.00 |  | 0.00 |  | 87.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 126.27 | 87.92 | 0.00 |  | 0.00 |  | 87.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 126.09 | 87.75 | 0.00 |  | 0.00 |  | 87.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 124.62 | 86.28 | 0.00 |  | 0.00 |  | 86.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 124.43 | 86.08 | 0.00 |  | 0.00 |  | 86.08 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s2-full/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 21104433 | 0.0 | 0.0 | true | 0 | 0 | 28.1 | 99 / 3 / 0 | 0 | 0 |

#### Stalls: s2-full/pp-uring-sqpoll-10000, loop 0 on CPU 3

2532531968 iterations, longest 28.1 µs; over 5 / 20 / 100 µs: 99 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 48.1 | 28.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 21.6 | 21.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 28.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 21.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 20.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: s2-full/pp-uring-sqpoll-10000 (median 38.91)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 945.39 | 906.48 | 0.00 |  | 0.00 |  | 906.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 937.51 | 898.60 | 0.00 |  | 0.00 |  | 898.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 415.32 | 376.40 | 0.00 |  | 0.00 |  | 376.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 384.81 | 345.89 | 0.00 |  | 0.00 |  | 345.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 377.92 | 339.01 | 0.00 |  | 0.00 |  | 339.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 362.89 | 323.98 | 0.00 |  | 0.00 |  | 323.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 354.13 | 315.22 | 0.00 |  | 0.00 |  | 315.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 334.73 | 295.82 | 0.00 |  | 0.00 |  | 295.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 304.56 | 265.65 | 0.00 |  | 0.00 |  | 265.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 301.09 | 262.18 | 0.00 |  | 0.00 |  | 262.18 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s2-full-sqtick/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1620695 | 1620695 | 1.00 | 106920792 | 65.97 | 1782013 | 1755002 | 98.5 | 72.1 | true | 0 | 2671 | 32.0 | 49180 / 53 / 0 | 0 | 0 |

#### Stalls: s2-full-sqtick/pp-recv-busy-0, loop 0 on CPU 3

105300096 iterations, longest 32.0 µs; over 5 / 20 / 100 µs: 49180 / 53 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 28 | 604.2 | 31.3 |
| mostly unexplained (+softirq NET_RX) | 10 | 224.4 | 28.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 99.6 | 27.2 |
| mostly unexplained (+local timer tick) | 4 | 91.0 | 24.8 |
| softirq NET_RX | 4 | 85.6 | 24.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.0 | 32.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 29.4 | 29.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 31.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 29.4 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 28.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 10, local timer tick 5, preempted by kworker/3:1 4, timer tick_nohz_handler 2 |
| 5 | 27.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 25.0 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 24.9 | 0 | softirq NET_RX | softirq NET_RX 17, local timer tick 5, preempted by kworker/3:1 3, timer tick_nohz_handler 3 |
| 9 | 24.8 | 1 | mostly unexplained (+local timer tick) | local timer tick 3, softirq RCU 1, timer tick_nohz_handler 1 |
| 10 | 24.7 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 9, preempted by kworker/3:1 6, local timer tick 3, irq work (IPI/vector) 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2713 |
| IWI IRQ work interrupts | 2676 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: s2-full-sqtick/pp-recv-busy-0 (median 36.57)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 139.86 | 103.29 | 0.00 |  | 0.00 |  | 103.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 134.19 | 97.62 | 0.00 |  | 62.35 | softirq TIMER | 35.26 | server: softirq TIMER |
| 3 | 124.70 | 88.13 | 0.00 |  | 0.00 |  | 88.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 124.60 | 88.03 | 0.00 |  | 0.00 |  | 88.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 124.56 | 87.99 | 0.00 |  | 0.00 |  | 87.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 124.21 | 87.64 | 0.00 |  | 0.00 |  | 87.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 122.72 | 86.15 | 0.00 |  | 0.00 |  | 86.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 121.23 | 84.66 | 0.00 |  | 0.00 |  | 84.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 120.66 | 84.09 | 0.00 |  | 0.00 |  | 84.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 120.58 | 84.01 | 0.00 |  | 0.00 |  | 84.01 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s2-full-sqtick/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 231116032 | 192.60 | 1925967 | 1915967 | 99.5 | 67.9 | true | 0 | 2841 | 64.5 | 23797 / 51 / 0 | 0 | 0 |

#### Stalls: s2-full-sqtick/pp-recv-busy-10000, loop 0 on CPU 3

229916032 iterations, longest 64.5 µs; over 5 / 20 / 100 µs: 23797 / 51 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 35 | 785.1 | 30.4 |
| softirq NET_RX | 6 | 137.1 | 30.1 |
| mostly unexplained (+softirq NET_RX) | 4 | 94.9 | 27.0 |
| softirq TIMER | 1 | 64.5 | 64.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 56.9 | 28.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 48.9 | 25.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 22.1 | 22.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 64.5 | 0 | softirq TIMER | softirq TIMER 55, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 2 | 30.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, workqueue [ena] 2, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 30.1 | 0 | softirq NET_RX | softirq NET_RX 16, preempted by kworker/3:1 6, workqueue [ena] 2, irq work (IPI/vector) 1 |
| 4 | 28.9 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 28.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 27.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 27.2 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 27.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 27.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 26.9 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2872 |
| IWI IRQ work interrupts | 2848 |
| CAL Function call interrupts | 1004 |
| MCP Machine check polls | 1 |

#### Slowest round trips: s2-full-sqtick/pp-recv-busy-10000 (median 37.77)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 267.60 | 229.83 | 0.00 |  | 0.00 |  | 229.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 133.14 | 95.37 | 64.48 | softirq TIMER | 0.00 |  | 30.89 | client: softirq TIMER |
| 3 | 123.28 | 85.51 | 0.00 |  | 0.00 |  | 85.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 122.25 | 84.48 | 0.00 |  | 0.00 |  | 84.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 120.59 | 82.82 | 0.00 |  | 0.00 |  | 82.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 119.83 | 82.06 | 0.00 |  | 18.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 63.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 119.49 | 81.72 | 0.00 |  | 0.00 |  | 81.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 119.31 | 81.54 | 0.00 |  | 0.00 |  | 81.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 119.25 | 81.48 | 0.00 |  | 0.00 |  | 81.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 118.78 | 81.00 | 0.00 |  | 0.00 |  | 81.00 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s2-full-sqtick/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1564242 | 1564242 | 1.00 | 1 | 0.00 | 0 | 33580585 | 0.0 | 0.0 | true | 0 | 0 | 27.5 | 48 / 1 / 0 | 0 | 0 |

#### Stalls: s2-full-sqtick/pp-uring-sqpoll-0, loop 0 on CPU 3

2014835072 iterations, longest 27.5 µs; over 5 / 20 / 100 µs: 48 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.5 | 27.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 27.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: s2-full-sqtick/pp-uring-sqpoll-0 (median 37.63)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 193.20 | 155.57 | 0.00 |  | 0.00 |  | 155.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 171.58 | 133.95 | 0.00 |  | 0.00 |  | 133.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 155.07 | 117.44 | 0.00 |  | 0.00 |  | 117.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 153.22 | 115.59 | 0.00 |  | 0.00 |  | 115.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 151.10 | 113.47 | 0.00 |  | 0.00 |  | 113.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 150.90 | 113.27 | 0.00 |  | 0.00 |  | 113.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 150.73 | 113.10 | 0.00 |  | 0.00 |  | 113.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 150.41 | 112.78 | 0.00 |  | 0.00 |  | 112.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 150.17 | 112.54 | 0.00 |  | 0.00 |  | 112.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 142.68 | 105.05 | 0.00 |  | 0.00 |  | 105.05 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (s2-full-sqtick/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 13950133 | 0.0 | 0.0 | true | 0 | 0 | 35.2 | 77 / 1 / 0 | 0 | 0 |

#### Stalls: s2-full-sqtick/pp-uring-sqpoll-10000, loop 0 on CPU 3

1674016000 iterations, longest 35.2 µs; over 5 / 20 / 100 µs: 77 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.2 | 35.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 35.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: s2-full-sqtick/pp-uring-sqpoll-10000 (median 38.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 508.03 | 469.19 | 0.00 |  | 0.00 |  | 469.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 165.26 | 126.42 | 0.00 |  | 0.00 |  | 126.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 159.93 | 121.09 | 0.00 |  | 0.00 |  | 121.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 158.23 | 119.39 | 0.00 |  | 155.12 | softirq TIMER | 0.00 | server: softirq TIMER |
| 5 | 137.28 | 98.44 | 0.00 |  | 0.00 |  | 98.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 130.68 | 91.83 | 0.00 |  | 0.00 |  | 91.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 127.06 | 88.22 | 0.00 |  | 0.00 |  | 88.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 125.23 | 86.39 | 0.00 |  | 0.00 |  | 86.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 121.98 | 83.14 | 0.00 |  | 0.00 |  | 83.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 121.71 | 82.87 | 0.00 |  | 0.00 |  | 82.87 | neither loop stalled long enough: network, hypervisor or untraced |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| s1-full/pp-recv-busy-0: recv-busy @ 0/s | 1503631 | 39.14 | 38.61 | 42.98 | 47.99 | 53.99 | 99.46 | 252.36 |
| s1-full/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 37.55 | 37.28 | 39.72 | 43.86 | 53.73 | 69.93 | 375.71 |
| s1-full/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 37.85 | 37.58 | 40.02 | 44.17 | 54.10 | 70.73 | 376.16 |
| s1-full/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1502548 | 38.97 | 38.51 | 42.33 | 47.51 | 54.89 | 97.93 | 1036.93 |
| s1-full/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 39.35 | 38.98 | 41.86 | 45.68 | 51.21 | 65.93 | 154.40 |
| s1-full/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 39.42 | 39.05 | 41.92 | 45.75 | 51.30 | 66.32 | 154.45 |
| s1-full-sqtick/pp-recv-busy-0: recv-busy @ 0/s | 1586809 | 37.08 | 36.64 | 39.70 | 45.72 | 57.75 | 71.33 | 171.69 |
| s1-full-sqtick/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 38.41 | 37.97 | 40.62 | 49.14 | 57.17 | 91.72 | 294.54 |
| s1-full-sqtick/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 38.76 | 38.27 | 40.92 | 49.46 | 57.57 | 105.86 | 1528.58 |
| s1-full-sqtick/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1530842 | 38.24 | 37.84 | 40.81 | 45.48 | 53.09 | 81.39 | 237.89 |
| s1-full-sqtick/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 39.07 | 38.75 | 41.30 | 45.12 | 53.70 | 77.67 | 188.54 |
| s1-full-sqtick/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 39.18 | 38.86 | 41.40 | 45.24 | 53.93 | 78.29 | 274.75 |
| s2-full/pp-recv-busy-0: recv-busy @ 0/s | 1574227 | 37.38 | 37.05 | 39.93 | 43.90 | 50.91 | 65.88 | 431.57 |
| s2-full/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 38.75 | 38.40 | 41.04 | 45.45 | 56.66 | 69.46 | 199.27 |
| s2-full/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 39.05 | 38.70 | 41.34 | 45.76 | 57.01 | 70.64 | 258.87 |
| s2-full/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1513131 | 38.73 | 38.35 | 41.31 | 46.01 | 55.31 | 75.46 | 296.04 |
| s2-full/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 39.39 | 38.91 | 41.74 | 46.90 | 88.24 | 172.56 | 945.39 |
| s2-full/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 39.90 | 39.00 | 41.84 | 47.12 | 145.01 | 899.31 | 1198.59 |
| s2-full-sqtick/pp-recv-busy-0: recv-busy @ 0/s | 1593484 | 36.93 | 36.57 | 39.51 | 44.05 | 53.50 | 71.06 | 139.86 |
| s2-full-sqtick/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 38.05 | 37.77 | 40.21 | 43.73 | 51.35 | 71.59 | 267.60 |
| s2-full-sqtick/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 38.35 | 38.06 | 40.51 | 44.03 | 51.71 | 72.50 | 267.88 |
| s2-full-sqtick/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1538091 | 38.11 | 37.63 | 40.95 | 47.23 | 56.44 | 76.55 | 193.20 |
| s2-full-sqtick/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 39.11 | 38.84 | 41.31 | 44.75 | 54.16 | 89.75 | 508.03 |
| s2-full-sqtick/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 39.26 | 38.99 | 41.45 | 44.91 | 54.53 | 91.50 | 508.20 |

