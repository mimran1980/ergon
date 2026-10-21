# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-many

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387095 | 7.02 | 24.12 | 31.74 | 41.00 | 68.38 | 0.0 | 0.0 | 0.0 | 0.0 | 1879.44 | 1449.0 | 1879.44 µs stall: loop work (4486 items) |
| m2 | 4387090 | 7.00 | 24.80 | 103.84 | 198.84 | 1203.56 | 4770.0 | 37.2 | 37.2 | 0.0 | 1850.05 | 783.0 | 1850.05 µs stall: loop work (4481 items) |
| m3 | 4387090 | 6.68 | 21.90 | 28.95 | 36.41 | 76.34 | 0.0 | 0.0 | 0.0 | 0.0 | 1915.64 | 392.0 | 1915.64 µs stall: loop work (4487 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-many, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387095 | 48.98 | 119.37 | 140.90 | 163.00 | 343.59 | 196482.8 | 0.0 | 0.0 | 0.0 |
| m2 | 4387090 | 49.98 | 128.62 | 95886.42 | 121794.01 | 125976.23 | 252281.7 | 20567.7 | 20310.1 | 18013.5 |
| m3 | 4387090 | 46.98 | 119.21 | 140.23 | 162.73 | 215.00 | 192794.2 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-many

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387099 | 42.62 | 126.56 | 150.44 | 169.96 | 206.62 | 289350.7 | 0.0 | 0.0 | 0.0 | 2248.14 | 895999.0 | 2248.14 µs stall: loop work (6209 items) |
| m2 | 4387250 | 41.91 | 122.73 | 147.75 | 168.77 | 800.86 | 242873.7 | 9.3 | 0.0 | 0.0 | 2464.61 | 894143.0 | 2464.61 µs stall: loop work (6240 items) |
| m3 | 4387088 | 43.43 | 128.02 | 152.23 | 174.51 | 215.56 | 310644.4 | 0.0 | 0.0 | 0.0 | 2497.93 | 893119.0 | 2497.93 µs stall: loop work (6181 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-many, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387099 | 79.57 | 176.45 | 199.88 | 218.96 | 257.17 | 1580697.5 | 0.0 | 0.0 | 0.0 |
| m2 | 4387250 | 78.87 | 175.12 | 199.61 | 221.24 | 823.67 | 1520998.8 | 10.3 | 0.0 | 0.0 |
| m3 | 4387088 | 82.90 | 180.74 | 204.69 | 224.09 | 300.48 | 1689369.5 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-spin-many

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386951 | 7.53 | 26.93 | 35.57 | 45.08 | 98.50 | 0.0 | 0.0 | 0.0 | 0.0 | 1423.06 | 712.0 | 1423.06 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| m2 | 4387056 | 7.68 | 28.03 | 37.43 | 48.48 | 751.51 | 113.8 | 12.4 | 0.0 | 0.0 | 1425.66 | 1337.0 | 1425.66 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| m3 | 4386957 | 7.48 | 26.19 | 34.36 | 42.79 | 258.26 | 6.2 | 0.0 | 0.0 | 0.0 | 1412.48 | 554.0 | 1412.48 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-spin-many, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386951 | 47.70 | 119.11 | 140.93 | 161.67 | 206.57 | 193767.9 | 0.0 | 0.0 | 0.0 |
| m2 | 4387056 | 48.85 | 122.94 | 147.42 | 175.40 | 813.82 | 228892.6 | 19.7 | 0.0 | 0.0 |
| m3 | 4386957 | 47.74 | 119.99 | 141.53 | 161.98 | 305.31 | 202184.6 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-many

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386952 | 6.37 | 26.29 | 45.98 | 61.19 | 82.38 | 0.0 | 0.0 | 0.0 | 0.0 | 679.31 | 4.0 | 679.31 µs stall: loop work (3883 items) |
| m2 | 4386947 | 6.36 | 28.64 | 107.38 | 220.04 | 914.54 | 5126.0 | 52.8 | 0.0 | 0.0 | 662.68 | 27.0 | 662.68 µs stall: loop work (3908 items) |
| m3 | 4386951 | 6.46 | 27.00 | 46.88 | 61.87 | 97.33 | 0.0 | 0.0 | 0.0 | 0.0 | 679.05 | 4.0 | 679.05 µs stall: loop work (3882 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-many, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386952 | 33.46 | 92.27 | 114.06 | 135.69 | 188.02 | 19493.3 | 0.0 | 0.0 | 0.0 |
| m2 | 4386947 | 31.23 | 93.29 | 55732.67 | 106791.94 | 111308.17 | 27405.4 | 9181.3 | 9108.9 | 8119.9 |
| m3 | 4386951 | 32.69 | 92.16 | 113.83 | 134.44 | 235.12 | 19944.4 | 0.0 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lab-epoll-spin-many | m1 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 1 | 13 | 0 |
| lab-epoll-spin-many | m2 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 451 | 0 | 1 | 21 | 0 |
| lab-epoll-spin-many | m3 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 341 | 0 | 1 | 13 | 0 |
| lab-recv-busy-many | m1 | 3 | 65 | 67 | 0 | 16 | 0 | 4 | 325 | 0 | 1 | 5 | 0 |
| lab-recv-busy-many | m2 | 3 | 69 | 70 | 0 | 17 | 0 | 4 | 336 | 0 | 1 | 13 | 0 |
| lab-recv-busy-many | m3 | 3 | 71 | 72 | 0 | 16 | 0 | 0 | 1142 | 0 | 1 | 4 | 0 |
| lab-uring-spin-many | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 4 | 428 | 0 | 1 | 14 | 0 |
| lab-uring-spin-many | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 4 | 423 | 0 | 0 | 24 | 0 |
| lab-uring-spin-many | m3 | 3 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 1 | 13 | 0 |
| lab-uring-sqpoll-many | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 1 | 132 | 67 | 0 | 20 | 0 |
| lab-uring-sqpoll-many | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 1 | 136 | 70 | 0 | 20 | 0 |
| lab-uring-sqpoll-many | m3 | 3 | 0 | 0 | 0 | 17 | 0 | 1 | 129 | 66 | 0 | 20 | 0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### lab-epoll-spin-many

| condition | worst events |
|---|---|
| m1 | 68.38 at 16.9 s: neither loop stalled long enough: network, hypervisor or untraced; 68.20 at 16.9 s: neither loop stalled long enough: network, hypervisor or untraced; 68.04 at 16.9 s: neither loop stalled long enough: network, hypervisor or untraced; 67.88 at 16.9 s: neither loop stalled long enough: network, hypervisor or untraced; 67.72 at 16.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 1203.56 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced; 1203.41 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced; 1203.25 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced; 1203.10 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced; 1202.94 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 76.34 at 56.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 59.05 at 2.4 s: client: loop work (3 items); 58.88 at 2.4 s: client: loop work (3 items); 52.88 at 2.4 s: client: loop work (3 items); 51.10 at 57.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-recv-busy-many

| condition | worst events |
|---|---|
| m1 | 206.62 at 37.6 s: client: loop work (26 items); 202.02 at 56.9 s: client: loop work (106 items); 201.86 at 56.9 s: client: loop work (106 items); 201.70 at 56.9 s: client: loop work (106 items); 201.54 at 56.9 s: client: loop work (106 items) |
| m2 | 800.86 at 4.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 777.14 at 4.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 761.10 at 4.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 710.04 at 4.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 708.88 at 4.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m3 | 215.56 at 22.9 s: client: loop work (89 items); 215.41 at 22.9 s: client: loop work (89 items); 215.25 at 22.9 s: client: loop work (89 items); 215.09 at 22.9 s: client: loop work (89 items); 214.91 at 22.9 s: client: loop work (89 items) |

### lab-uring-spin-many

| condition | worst events |
|---|---|
| m1 | 98.50 at 28.7 s: client: loop work (15 items); 98.32 at 28.7 s: client: loop work (15 items); 98.15 at 28.7 s: client: loop work (15 items); 88.25 at 28.7 s: client: loop work (15 items); 88.08 at 28.7 s: client: loop work (15 items) |
| m2 | 751.51 at 39.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 713.16 at 39.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 691.56 at 39.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 685.19 at 39.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 661.29 at 39.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m3 | 258.26 at 6.8 s: client: softirq TIMER; 223.31 at 6.8 s: client: softirq TIMER; 222.96 at 6.8 s: client: softirq TIMER; 216.23 at 6.8 s: client: softirq TIMER; 146.13 at 6.8 s: client: softirq TIMER |

### lab-uring-sqpoll-many

| condition | worst events |
|---|---|
| m1 | 82.38 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 82.22 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 82.06 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 81.90 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 81.74 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 914.54 at 46.3 s: client: loop work (755 items); 914.38 at 46.3 s: client: loop work (755 items); 914.22 at 46.3 s: client: loop work (755 items); 914.06 at 46.3 s: client: loop work (755 items); 913.90 at 46.3 s: client: loop work (755 items) |
| m3 | 97.33 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 97.16 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 97.00 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 96.81 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 96.62 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| m1 | unexplained: no kernel event (hypervisor or in-kernel time) 45096657.41; loop work 12806477.23; mostly unexplained (+softirq NET_RX) 1590278.78; mostly unexplained (+local timer tick) 265853.17; mostly unexplained (+preempted by kworker/3:1) 192326.20 |
| m2 | unexplained: no kernel event (hypervisor or in-kernel time) 45015637.86; loop work 12756256.58; mostly unexplained (+softirq NET_RX) 1690888.75; mostly unexplained (+local timer tick) 283155.72; mostly unexplained (+preempted by kworker/3:1) 211451.97 |
| m3 | unexplained: no kernel event (hypervisor or in-kernel time) 44968918.50; loop work 12782932.08; mostly unexplained (+softirq NET_RX) 1667774.56; mostly unexplained (+local timer tick) 289963.19; mostly unexplained (+preempted by kworker/3:1) 210405.98 |

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
| cpumap-sender/cpu1 | counter-only spinner | 1 | :07 | 1 (100%) | 1 of 2 | 711.68 | no |
| cpumap-sender/cpu3 | counter-only spinner | 1 | :07 | 1 (100%) | 1 of 2 | 717.16 | no |
| lab-epoll-spin-many | idle spare core | 0 | | | | | |
| lab-epoll-spin-many | receiver loop | 0 | | | | | |
| lab-epoll-spin-many | slow samples | 122 | :17 | 122 (100%) | 1 of 3 | 1203.56 | **yes** |
| lab-recv-busy-many | idle spare core | 3 | :53 | 3 (100%) | 1 of 3 | 380.87 | no |
| lab-recv-busy-many | receiver loop | 1 | :53 | 1 (100%) | 1 of 3 | 848.99 | no |
| lab-recv-busy-many | slow samples | 14 | :53 | 14 (100%) | 1 of 3 | 800.86 | **yes** |
| lab-uring-spin-many | idle spare core | 3 | :53 | 2 (67%) | 1 of 3 | 381.89 | no |
| lab-uring-spin-many | receiver loop | 1 | :53 | 1 (100%) | 1 of 3 | 798.14 | no |
| lab-uring-spin-many | slow samples | 23 | :53 | 23 (100%) | 1 of 3 | 751.51 | **yes** |
| lab-uring-sqpoll-many | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-many | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-many | slow samples | 210 | :47 | 210 (100%) | 1 of 3 | 914.54 | **yes** |

# Every run in detail

## m1/lab-epoll-spin-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4544532 | 2272748 | 2.00 | 165024556 | 36.31 | 2750409 | 2712530 | 0.0 | 33.1 | false | 0 | 0 | 1879.4 | 305832 / 1449 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8195 | 0 |
| epoll-spin | 1 | 8195 | 0 |
| epoll-spin | 2 | 8198 | 4 |
| epoll-spin | 3 | 8194 | 4 |
| epoll-spin | 4 | 8194 | 4 |
| epoll-spin | 5 | 8197 | 0 |
| epoll-spin | 6 | 8196 | 4 |
| epoll-spin | 7 | 8197 | 0 |
| epoll-spin | 8 | 8199 | 0 |
| epoll-spin | 9 | 8194 | 4 |
| epoll-spin | 10 | 8197 | 0 |
| epoll-spin | 11 | 8194 | 4 |
| epoll-spin | 12 | 8199 | 0 |
| epoll-spin | 13 | 8196 | 4 |
| epoll-spin | 14 | 8194 | 4 |
| epoll-spin | 15 | 8197 | 0 |
| epoll-spin | 16 | 8196 | 4 |
| epoll-spin | 17 | 8199 | 0 |
| epoll-spin | 18 | 8197 | 0 |
| epoll-spin | 19 | 8196 | 4 |
| epoll-spin | 20 | 8194 | 4 |
| epoll-spin | 21 | 8197 | 0 |
| epoll-spin | 22 | 8195 | 0 |
| epoll-spin | 23 | 8193 | 0 |
| epoll-spin | 24 | 8195 | 0 |
| epoll-spin | 25 | 8193 | 0 |
| epoll-spin | 26 | 8198 | 4 |
| epoll-spin | 27 | 8198 | 4 |
| epoll-spin | 28 | 8199 | 0 |
| epoll-spin | 29 | 8197 | 0 |
| epoll-spin | 30 | 8194 | 4 |
| epoll-spin | 31 | 8199 | 0 |
| epoll-spin | 32 | 8197 | 0 |
| epoll-spin | 33 | 8194 | 4 |
| epoll-spin | 34 | 8197 | 0 |
| epoll-spin | 35 | 8193 | 0 |
| epoll-spin | 36 | 8200 | 4 |
| epoll-spin | 37 | 8195 | 0 |
| epoll-spin | 38 | 8198 | 4 |
| epoll-spin | 39 | 8198 | 4 |
| epoll-spin | 40 | 8200 | 4 |
| epoll-spin | 41 | 8195 | 0 |
| epoll-spin | 42 | 8198 | 4 |
| epoll-spin | 43 | 8193 | 0 |
| epoll-spin | 44 | 8193 | 0 |
| epoll-spin | 45 | 8198 | 4 |
| epoll-spin | 46 | 8195 | 0 |
| epoll-spin | 47 | 8195 | 0 |
| epoll-spin | 48 | 8195 | 0 |
| epoll-spin | 49 | 8196 | 4 |
| epoll-spin | 50 | 8196 | 4 |
| epoll-spin | 51 | 8197 | 0 |
| epoll-spin | 52 | 8197 | 0 |
| epoll-spin | 53 | 8196 | 4 |
| epoll-spin | 54 | 8197 | 0 |
| epoll-spin | 55 | 8194 | 4 |
| epoll-spin | 56 | 8197 | 0 |
| epoll-spin | 57 | 8195 | 0 |
| epoll-spin | 58 | 8195 | 0 |
| epoll-spin | 59 | 8198 | 4 |
| epoll-spin | 60 | 8200 | 4 |
| epoll-spin | 61 | 8193 | 0 |
| epoll-spin | 62 | 8194 | 4 |
| epoll-spin | 63 | 8196 | 4 |
| epoll-spin | 64 | 8194 | 4 |
| epoll-spin | 65 | 8194 | 4 |
| epoll-spin | 66 | 8194 | 4 |
| epoll-spin | 67 | 8196 | 4 |
| epoll-spin | 68 | 8199 | 0 |
| epoll-spin | 69 | 8196 | 4 |
| epoll-spin | 70 | 8197 | 0 |
| epoll-spin | 71 | 8197 | 0 |
| epoll-spin | 72 | 8199 | 0 |
| epoll-spin | 73 | 8196 | 4 |
| epoll-spin | 74 | 8195 | 0 |
| epoll-spin | 75 | 8200 | 4 |
| epoll-spin | 76 | 8198 | 4 |
| epoll-spin | 77 | 8195 | 0 |
| epoll-spin | 78 | 8195 | 0 |
| epoll-spin | 79 | 8200 | 4 |
| epoll-spin | 80 | 8193 | 0 |
| epoll-spin | 81 | 8200 | 4 |
| epoll-spin | 82 | 8193 | 0 |
| epoll-spin | 83 | 8196 | 4 |
| epoll-spin | 84 | 8194 | 4 |
| epoll-spin | 85 | 8194 | 4 |
| epoll-spin | 86 | 8199 | 0 |
| epoll-spin | 87 | 8196 | 4 |
| epoll-spin | 88 | 8199 | 0 |
| epoll-spin | 89 | 8196 | 4 |
| epoll-spin | 90 | 8197 | 0 |
| epoll-spin | 91 | 8194 | 4 |
| epoll-spin | 92 | 8199 | 0 |
| epoll-spin | 93 | 8197 | 0 |
| epoll-spin | 94 | 8194 | 4 |
| epoll-spin | 95 | 8196 | 4 |
| epoll-spin | 96 | 8200 | 4 |
| epoll-spin | 97 | 8198 | 4 |
| epoll-spin | 98 | 8193 | 0 |
| epoll-spin | 99 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1017975168 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1668132 of 4387095 in 1-msg reads) | 1668132 | 4.78 | 3.69 | 7.64 | 16.14 | 24.93 | 33.55 | 64.84 |

#### Stalls: m1/lab-epoll-spin-many, loop 0 on CPU 3

162751808 iterations, longest 1879.4 µs; over 5 / 20 / 100 µs: 305832 / 1449 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 717 | 19002.9 | 1879.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 731 | 17406.1 | 51.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.7 | 35.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1879.4 | 4486 | loop work (4486 items) |  |
| 2 | 742.7 | 1878 | loop work (1878 items) |  |
| 3 | 51.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 48.8 | 4 | loop work (4 items) |  |
| 5 | 48.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 47.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 46.5 | 3 | loop work (3 items) |  |
| 8 | 44.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 41.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1368507 clocksource=tsc

### Loop counters (m1/lab-recv-busy-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4544423 | 1915650 | 2.37 | 89600000 | 19.72 | 1493333 | 14933 | 97.9 | 69.0 | true | 0 | 4002 | 2248.1 | 895999 / 895999 / 34689 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 4 |
| recv-busy | 1 | 8200 | 4 |
| recv-busy | 2 | 8195 | 0 |
| recv-busy | 3 | 8198 | 4 |
| recv-busy | 4 | 8200 | 4 |
| recv-busy | 5 | 8198 | 3 |
| recv-busy | 6 | 8195 | 0 |
| recv-busy | 7 | 8200 | 3 |
| recv-busy | 8 | 8198 | 3 |
| recv-busy | 9 | 8197 | 3 |
| recv-busy | 10 | 8194 | 3 |
| recv-busy | 11 | 8199 | 0 |
| recv-busy | 12 | 8194 | 4 |
| recv-busy | 13 | 8197 | 3 |
| recv-busy | 14 | 8200 | 3 |
| recv-busy | 15 | 8198 | 3 |
| recv-busy | 16 | 8193 | 3 |
| recv-busy | 17 | 8198 | 3 |
| recv-busy | 18 | 8200 | 3 |
| recv-busy | 19 | 8193 | 3 |
| recv-busy | 20 | 8200 | 4 |
| recv-busy | 21 | 8197 | 3 |
| recv-busy | 22 | 8196 | 3 |
| recv-busy | 23 | 8199 | 3 |
| recv-busy | 24 | 8197 | 3 |
| recv-busy | 25 | 8196 | 3 |
| recv-busy | 26 | 8196 | 3 |
| recv-busy | 27 | 8197 | 3 |
| recv-busy | 28 | 8194 | 4 |
| recv-busy | 29 | 8194 | 4 |
| recv-busy | 30 | 8199 | 3 |
| recv-busy | 31 | 8199 | 3 |
| recv-busy | 32 | 8199 | 0 |
| recv-busy | 33 | 8199 | 3 |
| recv-busy | 34 | 8193 | 3 |
| recv-busy | 35 | 8193 | 0 |
| recv-busy | 36 | 8200 | 4 |
| recv-busy | 37 | 8198 | 3 |
| recv-busy | 38 | 8198 | 3 |
| recv-busy | 39 | 8198 | 3 |
| recv-busy | 40 | 8193 | 0 |
| recv-busy | 41 | 8200 | 4 |
| recv-busy | 42 | 8195 | 3 |
| recv-busy | 43 | 8193 | 0 |
| recv-busy | 44 | 8197 | 0 |
| recv-busy | 45 | 8199 | 3 |
| recv-busy | 46 | 8196 | 3 |
| recv-busy | 47 | 8196 | 3 |
| recv-busy | 48 | 8196 | 3 |
| recv-busy | 49 | 8194 | 4 |
| recv-busy | 50 | 8195 | 0 |
| recv-busy | 51 | 8193 | 0 |
| recv-busy | 52 | 8200 | 4 |
| recv-busy | 53 | 8198 | 3 |
| recv-busy | 54 | 8193 | 0 |
| recv-busy | 55 | 8198 | 3 |
| recv-busy | 56 | 8198 | 4 |
| recv-busy | 57 | 8195 | 3 |
| recv-busy | 58 | 8198 | 3 |
| recv-busy | 59 | 8193 | 3 |
| recv-busy | 60 | 8193 | 3 |
| recv-busy | 61 | 8198 | 3 |
| recv-busy | 62 | 8195 | 0 |
| recv-busy | 63 | 8193 | 3 |
| recv-busy | 64 | 8196 | 3 |
| recv-busy | 65 | 8194 | 3 |
| recv-busy | 66 | 8194 | 4 |
| recv-busy | 67 | 8196 | 3 |
| recv-busy | 68 | 8196 | 3 |
| recv-busy | 69 | 8197 | 3 |
| recv-busy | 70 | 8199 | 0 |
| recv-busy | 71 | 8195 | 3 |
| recv-busy | 72 | 8195 | 0 |
| recv-busy | 73 | 8193 | 3 |
| recv-busy | 74 | 8200 | 3 |
| recv-busy | 75 | 8195 | 3 |
| recv-busy | 76 | 8193 | 0 |
| recv-busy | 77 | 8200 | 4 |
| recv-busy | 78 | 8197 | 3 |
| recv-busy | 79 | 8199 | 0 |
| recv-busy | 80 | 8197 | 0 |
| recv-busy | 81 | 8194 | 4 |
| recv-busy | 82 | 8196 | 3 |
| recv-busy | 83 | 8199 | 0 |
| recv-busy | 84 | 8196 | 4 |
| recv-busy | 85 | 8197 | 3 |
| recv-busy | 86 | 8194 | 4 |
| recv-busy | 87 | 8196 | 3 |
| recv-busy | 88 | 8199 | 0 |
| recv-busy | 89 | 8194 | 4 |
| recv-busy | 90 | 8197 | 3 |
| recv-busy | 91 | 8199 | 0 |
| recv-busy | 92 | 8194 | 3 |
| recv-busy | 93 | 8196 | 4 |
| recv-busy | 94 | 8198 | 4 |
| recv-busy | 95 | 8195 | 0 |
| recv-busy | 96 | 8193 | 0 |
| recv-busy | 97 | 8195 | 0 |
| recv-busy | 98 | 8198 | 3 |
| recv-busy | 99 | 8198 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1017950752 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1490284 of 4387099 in 1-msg reads) | 1490284 | 33.35 | 32.48 | 60.11 | 86.42 | 111.94 | 142.07 | 206.62 |

#### Stalls: m1/lab-recv-busy-many, loop 0 on CPU 3

896000 iterations, longest 2248.1 µs; over 5 / 20 / 100 µs: 895999 / 895999 / 34689

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 714111 | 45066840.8 | 169.2 |
| loop work | 152513 | 12778571.8 | 2248.1 |
| mostly unexplained (+softirq NET_RX) | 23443 | 1590278.8 | 162.5 |
| mostly unexplained (+local timer tick) | 3525 | 265853.2 | 132.3 |
| mostly unexplained (+preempted by kworker/3:1) | 2175 | 192326.2 | 160.2 |
| mostly unexplained (+irq work (IPI/vector)) | 200 | 15696.0 | 109.8 |
| mostly unexplained (+softirq NET_TX) | 13 | 1176.5 | 161.6 |
| mostly unexplained (+call function (IPI/vector)) | 9 | 630.7 | 87.1 |
| mostly unexplained (+ipi_send_cpu) | 5 | 381.5 | 83.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 187.7 | 63.9 |
| mostly unexplained (+softirq TASKLET) | 1 | 64.8 | 64.8 |
| mostly unexplained (+call function single (IPI/vector)) | 1 | 64.3 | 64.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2248.1 | 6209 | loop work (6209 items) |  |
| 2 | 231.3 | 84 | loop work (84 items) | softirq NET_RX 3 |
| 3 | 208.1 | 87 | loop work (87 items) | softirq NET_RX 1 |
| 4 | 205.2 | 62 | loop work (62 items) |  |
| 5 | 203.5 | 46 | loop work (46 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 201.8 | 65 | loop work (65 items) | softirq NET_RX 1 |
| 7 | 201.7 | 88 | loop work (88 items) |  |
| 8 | 201.6 | 97 | loop work (97 items) | softirq NET_RX 1 |
| 9 | 199.1 | 42 | loop work (42 items) |  |
| 10 | 198.4 | 51 | loop work (51 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4040 |
| LOC Local timer interrupts | 3910 |
| CAL Function call interrupts | 990 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-spin-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-uring-spin-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 4544426 | 2295727 | 1.98 | 3573560 | 0.79 | 59559 | 6525189 | 0.0 | 10.5 | false | 0 | 0 | 1423.1 | 288160 / 712 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8200 | 4 |
| uring-spin | 1 | 8200 | 4 |
| uring-spin | 2 | 8195 | 0 |
| uring-spin | 3 | 8198 | 4 |
| uring-spin | 4 | 8193 | 0 |
| uring-spin | 5 | 8193 | 0 |
| uring-spin | 6 | 8198 | 4 |
| uring-spin | 7 | 8196 | 4 |
| uring-spin | 8 | 8199 | 0 |
| uring-spin | 9 | 8197 | 0 |
| uring-spin | 10 | 8199 | 0 |
| uring-spin | 11 | 8196 | 4 |
| uring-spin | 12 | 8199 | 0 |
| uring-spin | 13 | 8196 | 4 |
| uring-spin | 14 | 8194 | 4 |
| uring-spin | 15 | 8196 | 4 |
| uring-spin | 16 | 8196 | 4 |
| uring-spin | 17 | 8195 | 0 |
| uring-spin | 18 | 8195 | 0 |
| uring-spin | 19 | 8193 | 0 |
| uring-spin | 20 | 8195 | 0 |
| uring-spin | 21 | 8200 | 4 |
| uring-spin | 22 | 8198 | 4 |
| uring-spin | 23 | 8198 | 4 |
| uring-spin | 24 | 8195 | 0 |
| uring-spin | 25 | 8200 | 4 |
| uring-spin | 26 | 8196 | 4 |
| uring-spin | 27 | 8196 | 4 |
| uring-spin | 28 | 8194 | 4 |
| uring-spin | 29 | 8199 | 0 |
| uring-spin | 30 | 8197 | 0 |
| uring-spin | 31 | 8197 | 0 |
| uring-spin | 32 | 8194 | 4 |
| uring-spin | 33 | 8194 | 4 |
| uring-spin | 34 | 8196 | 4 |
| uring-spin | 35 | 8196 | 4 |
| uring-spin | 36 | 8197 | 0 |
| uring-spin | 37 | 8199 | 0 |
| uring-spin | 38 | 8196 | 4 |
| uring-spin | 39 | 8193 | 0 |
| uring-spin | 40 | 8195 | 0 |
| uring-spin | 41 | 8200 | 4 |
| uring-spin | 42 | 8198 | 4 |
| uring-spin | 43 | 8195 | 0 |
| uring-spin | 44 | 8195 | 0 |
| uring-spin | 45 | 8194 | 4 |
| uring-spin | 46 | 8197 | 0 |
| uring-spin | 47 | 8196 | 4 |
| uring-spin | 48 | 8197 | 0 |
| uring-spin | 49 | 8196 | 4 |
| uring-spin | 50 | 8196 | 4 |
| uring-spin | 51 | 8195 | 0 |
| uring-spin | 52 | 8198 | 4 |
| uring-spin | 53 | 8200 | 4 |
| uring-spin | 54 | 8200 | 4 |
| uring-spin | 55 | 8193 | 0 |
| uring-spin | 56 | 8195 | 0 |
| uring-spin | 57 | 8198 | 4 |
| uring-spin | 58 | 8200 | 4 |
| uring-spin | 59 | 8195 | 0 |
| uring-spin | 60 | 8193 | 0 |
| uring-spin | 61 | 8198 | 4 |
| uring-spin | 62 | 8200 | 4 |
| uring-spin | 63 | 8198 | 4 |
| uring-spin | 64 | 8197 | 0 |
| uring-spin | 65 | 8196 | 4 |
| uring-spin | 66 | 8196 | 4 |
| uring-spin | 67 | 8194 | 4 |
| uring-spin | 68 | 8196 | 4 |
| uring-spin | 69 | 8194 | 4 |
| uring-spin | 70 | 8197 | 0 |
| uring-spin | 71 | 8200 | 4 |
| uring-spin | 72 | 8193 | 0 |
| uring-spin | 73 | 8195 | 0 |
| uring-spin | 74 | 8193 | 0 |
| uring-spin | 75 | 8193 | 0 |
| uring-spin | 76 | 8194 | 4 |
| uring-spin | 77 | 8197 | 0 |
| uring-spin | 78 | 8194 | 4 |
| uring-spin | 79 | 8194 | 4 |
| uring-spin | 80 | 8197 | 0 |
| uring-spin | 81 | 8196 | 4 |
| uring-spin | 82 | 8194 | 4 |
| uring-spin | 83 | 8199 | 0 |
| uring-spin | 84 | 8196 | 4 |
| uring-spin | 85 | 8197 | 0 |
| uring-spin | 86 | 8194 | 4 |
| uring-spin | 87 | 8197 | 0 |
| uring-spin | 88 | 8196 | 4 |
| uring-spin | 89 | 8196 | 4 |
| uring-spin | 90 | 8200 | 4 |
| uring-spin | 91 | 8200 | 4 |
| uring-spin | 92 | 8195 | 0 |
| uring-spin | 93 | 8195 | 0 |
| uring-spin | 94 | 8200 | 4 |
| uring-spin | 95 | 8200 | 4 |
| uring-spin | 96 | 8198 | 4 |
| uring-spin | 97 | 8199 | 0 |
| uring-spin | 98 | 8196 | 4 |
| uring-spin | 99 | 8197 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 1017951424 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (1685013 of 4386951 in 1-msg reads) | 1685013 | 5.31 | 4.09 | 8.40 | 17.30 | 26.06 | 35.47 | 59.73 |

#### Stalls: m1/lab-uring-spin-many, loop 0 on CPU 3

391511360 iterations, longest 1423.1 µs; over 5 / 20 / 100 µs: 288160 / 712 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 433 | 12388.2 | 1423.1 |
| loop work | 278 | 7814.0 | 753.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.6 | 31.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1423.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 753.0 | 3907 | loop work (3907 items) |  |
| 3 | 466.7 | 2408 | loop work (2408 items) |  |
| 4 | 66.8 | 15 | loop work (15 items) |  |
| 5 | 46.3 | 7 | loop work (7 items) |  |
| 6 | 45.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 44.8 | 7 | loop work (7 items) |  |
| 8 | 42.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 41.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 40.8 | 24 | loop work (24 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2018341 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4544422 | 2211310 | 2.06 | 1 | 0.00 | 0 | 6941865 | 0.0 | 0.0 | true | 0 | 0 | 679.3 | 8527 / 4 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8198 | 1 |
| uring-sqpoll | 1 | 8197 | 1 |
| uring-sqpoll | 2 | 8194 | 1 |
| uring-sqpoll | 3 | 8199 | 1 |
| uring-sqpoll | 4 | 8197 | 1 |
| uring-sqpoll | 5 | 8194 | 1 |
| uring-sqpoll | 6 | 8199 | 1 |
| uring-sqpoll | 7 | 8199 | 1 |
| uring-sqpoll | 8 | 8198 | 1 |
| uring-sqpoll | 9 | 8200 | 4 |
| uring-sqpoll | 10 | 8198 | 1 |
| uring-sqpoll | 11 | 8193 | 1 |
| uring-sqpoll | 12 | 8193 | 1 |
| uring-sqpoll | 13 | 8195 | 1 |
| uring-sqpoll | 14 | 8200 | 1 |
| uring-sqpoll | 15 | 8198 | 1 |
| uring-sqpoll | 16 | 8195 | 1 |
| uring-sqpoll | 17 | 8194 | 4 |
| uring-sqpoll | 18 | 8199 | 1 |
| uring-sqpoll | 19 | 8197 | 1 |
| uring-sqpoll | 20 | 8199 | 1 |
| uring-sqpoll | 21 | 8199 | 1 |
| uring-sqpoll | 22 | 8196 | 1 |
| uring-sqpoll | 23 | 8198 | 1 |
| uring-sqpoll | 24 | 8195 | 1 |
| uring-sqpoll | 25 | 8193 | 1 |
| uring-sqpoll | 26 | 8193 | 1 |
| uring-sqpoll | 27 | 8198 | 1 |
| uring-sqpoll | 28 | 8193 | 1 |
| uring-sqpoll | 29 | 8200 | 1 |
| uring-sqpoll | 30 | 8197 | 1 |
| uring-sqpoll | 31 | 8194 | 1 |
| uring-sqpoll | 32 | 8197 | 1 |
| uring-sqpoll | 33 | 8199 | 1 |
| uring-sqpoll | 34 | 8196 | 1 |
| uring-sqpoll | 35 | 8197 | 1 |
| uring-sqpoll | 36 | 8199 | 1 |
| uring-sqpoll | 37 | 8194 | 1 |
| uring-sqpoll | 38 | 8199 | 1 |
| uring-sqpoll | 39 | 8196 | 1 |
| uring-sqpoll | 40 | 8194 | 1 |
| uring-sqpoll | 41 | 8196 | 1 |
| uring-sqpoll | 42 | 8195 | 1 |
| uring-sqpoll | 43 | 8193 | 1 |
| uring-sqpoll | 44 | 8195 | 1 |
| uring-sqpoll | 45 | 8193 | 1 |
| uring-sqpoll | 46 | 8198 | 1 |
| uring-sqpoll | 47 | 8200 | 1 |
| uring-sqpoll | 48 | 8193 | 0 |
| uring-sqpoll | 49 | 8196 | 1 |
| uring-sqpoll | 50 | 8194 | 1 |
| uring-sqpoll | 51 | 8197 | 1 |
| uring-sqpoll | 52 | 8199 | 1 |
| uring-sqpoll | 53 | 8194 | 1 |
| uring-sqpoll | 54 | 8199 | 1 |
| uring-sqpoll | 55 | 8196 | 1 |
| uring-sqpoll | 56 | 8197 | 1 |
| uring-sqpoll | 57 | 8194 | 1 |
| uring-sqpoll | 58 | 8193 | 1 |
| uring-sqpoll | 59 | 8198 | 1 |
| uring-sqpoll | 60 | 8195 | 1 |
| uring-sqpoll | 61 | 8195 | 1 |
| uring-sqpoll | 62 | 8198 | 1 |
| uring-sqpoll | 63 | 8195 | 1 |
| uring-sqpoll | 64 | 8195 | 1 |
| uring-sqpoll | 65 | 8200 | 1 |
| uring-sqpoll | 66 | 8195 | 1 |
| uring-sqpoll | 67 | 8200 | 1 |
| uring-sqpoll | 68 | 8195 | 1 |
| uring-sqpoll | 69 | 8193 | 1 |
| uring-sqpoll | 70 | 8200 | 1 |
| uring-sqpoll | 71 | 8193 | 1 |
| uring-sqpoll | 72 | 8198 | 1 |
| uring-sqpoll | 73 | 8193 | 1 |
| uring-sqpoll | 74 | 8194 | 1 |
| uring-sqpoll | 75 | 8196 | 1 |
| uring-sqpoll | 76 | 8194 | 1 |
| uring-sqpoll | 77 | 8197 | 1 |
| uring-sqpoll | 78 | 8199 | 1 |
| uring-sqpoll | 79 | 8196 | 1 |
| uring-sqpoll | 80 | 8198 | 1 |
| uring-sqpoll | 81 | 8195 | 0 |
| uring-sqpoll | 82 | 8193 | 1 |
| uring-sqpoll | 83 | 8198 | 1 |
| uring-sqpoll | 84 | 8198 | 4 |
| uring-sqpoll | 85 | 8195 | 1 |
| uring-sqpoll | 86 | 8196 | 1 |
| uring-sqpoll | 87 | 8197 | 1 |
| uring-sqpoll | 88 | 8194 | 1 |
| uring-sqpoll | 89 | 8199 | 1 |
| uring-sqpoll | 90 | 8194 | 1 |
| uring-sqpoll | 91 | 8199 | 1 |
| uring-sqpoll | 92 | 8194 | 1 |
| uring-sqpoll | 93 | 8194 | 1 |
| uring-sqpoll | 94 | 8196 | 1 |
| uring-sqpoll | 95 | 8199 | 1 |
| uring-sqpoll | 96 | 8197 | 1 |
| uring-sqpoll | 97 | 8194 | 1 |
| uring-sqpoll | 98 | 8194 | 1 |
| uring-sqpoll | 99 | 8199 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1017950528 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1576585 of 4386952 in 1-msg reads) | 1576585 | 4.98 | 4.19 | 7.35 | 16.98 | 32.21 | 45.00 | 67.51 |

#### Stalls: m1/lab-uring-sqpoll-many, loop 0 on CPU 3

416511872 iterations, longest 679.3 µs; over 5 / 20 / 100 µs: 8527 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1088.5 | 679.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.7 | 33.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 22.3 | 22.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 679.3 | 3883 | loop work (3883 items) |  |
| 2 | 409.2 | 2362 | loop work (2362 items) |  |
| 3 | 33.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 22.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 3 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4544423 | 2257567 | 2.01 | 163627871 | 36.01 | 2727131 | 2689505 | 0.0 | 33.5 | false | 0 | 0 | 1850.0 | 295705 / 783 / 12 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8196 | 4 |
| epoll-spin | 1 | 8194 | 4 |
| epoll-spin | 2 | 8197 | 0 |
| epoll-spin | 3 | 8194 | 4 |
| epoll-spin | 4 | 8196 | 4 |
| epoll-spin | 5 | 8194 | 4 |
| epoll-spin | 6 | 8199 | 0 |
| epoll-spin | 7 | 8197 | 0 |
| epoll-spin | 8 | 8200 | 4 |
| epoll-spin | 9 | 8198 | 4 |
| epoll-spin | 10 | 8200 | 4 |
| epoll-spin | 11 | 8200 | 4 |
| epoll-spin | 12 | 8195 | 0 |
| epoll-spin | 13 | 8200 | 4 |
| epoll-spin | 14 | 8195 | 0 |
| epoll-spin | 15 | 8200 | 4 |
| epoll-spin | 16 | 8196 | 4 |
| epoll-spin | 17 | 8194 | 4 |
| epoll-spin | 18 | 8199 | 0 |
| epoll-spin | 19 | 8194 | 4 |
| epoll-spin | 20 | 8197 | 0 |
| epoll-spin | 21 | 8199 | 0 |
| epoll-spin | 22 | 8194 | 4 |
| epoll-spin | 23 | 8197 | 0 |
| epoll-spin | 24 | 8199 | 0 |
| epoll-spin | 25 | 8196 | 4 |
| epoll-spin | 26 | 8195 | 0 |
| epoll-spin | 27 | 8200 | 4 |
| epoll-spin | 28 | 8193 | 0 |
| epoll-spin | 29 | 8193 | 0 |
| epoll-spin | 30 | 8198 | 4 |
| epoll-spin | 31 | 8195 | 0 |
| epoll-spin | 32 | 8198 | 4 |
| epoll-spin | 33 | 8199 | 0 |
| epoll-spin | 34 | 8196 | 4 |
| epoll-spin | 35 | 8197 | 0 |
| epoll-spin | 36 | 8194 | 4 |
| epoll-spin | 37 | 8197 | 0 |
| epoll-spin | 38 | 8194 | 4 |
| epoll-spin | 39 | 8196 | 4 |
| epoll-spin | 40 | 8199 | 0 |
| epoll-spin | 41 | 8197 | 0 |
| epoll-spin | 42 | 8200 | 4 |
| epoll-spin | 43 | 8198 | 4 |
| epoll-spin | 44 | 8198 | 4 |
| epoll-spin | 45 | 8200 | 4 |
| epoll-spin | 46 | 8195 | 0 |
| epoll-spin | 47 | 8198 | 4 |
| epoll-spin | 48 | 8200 | 4 |
| epoll-spin | 49 | 8194 | 4 |
| epoll-spin | 50 | 8196 | 4 |
| epoll-spin | 51 | 8194 | 4 |
| epoll-spin | 52 | 8199 | 0 |
| epoll-spin | 53 | 8199 | 0 |
| epoll-spin | 54 | 8194 | 4 |
| epoll-spin | 55 | 8196 | 4 |
| epoll-spin | 56 | 8193 | 0 |
| epoll-spin | 57 | 8195 | 0 |
| epoll-spin | 58 | 8198 | 4 |
| epoll-spin | 59 | 8195 | 0 |
| epoll-spin | 60 | 8200 | 4 |
| epoll-spin | 61 | 8193 | 0 |
| epoll-spin | 62 | 8198 | 4 |
| epoll-spin | 63 | 8200 | 4 |
| epoll-spin | 64 | 8200 | 4 |
| epoll-spin | 65 | 8198 | 4 |
| epoll-spin | 66 | 8198 | 4 |
| epoll-spin | 67 | 8200 | 4 |
| epoll-spin | 68 | 8198 | 4 |
| epoll-spin | 69 | 8198 | 4 |
| epoll-spin | 70 | 8198 | 4 |
| epoll-spin | 71 | 8193 | 0 |
| epoll-spin | 72 | 8197 | 0 |
| epoll-spin | 73 | 8196 | 4 |
| epoll-spin | 74 | 8194 | 4 |
| epoll-spin | 75 | 8199 | 0 |
| epoll-spin | 76 | 8199 | 0 |
| epoll-spin | 77 | 8194 | 4 |
| epoll-spin | 78 | 8196 | 4 |
| epoll-spin | 79 | 8198 | 4 |
| epoll-spin | 80 | 8193 | 0 |
| epoll-spin | 81 | 8193 | 0 |
| epoll-spin | 82 | 8200 | 4 |
| epoll-spin | 83 | 8198 | 4 |
| epoll-spin | 84 | 8195 | 0 |
| epoll-spin | 85 | 8193 | 0 |
| epoll-spin | 86 | 8196 | 4 |
| epoll-spin | 87 | 8199 | 0 |
| epoll-spin | 88 | 8196 | 4 |
| epoll-spin | 89 | 8194 | 4 |
| epoll-spin | 90 | 8199 | 0 |
| epoll-spin | 91 | 8197 | 0 |
| epoll-spin | 92 | 8194 | 4 |
| epoll-spin | 93 | 8196 | 4 |
| epoll-spin | 94 | 8194 | 4 |
| epoll-spin | 95 | 8194 | 4 |
| epoll-spin | 96 | 8197 | 0 |
| epoll-spin | 97 | 8194 | 4 |
| epoll-spin | 98 | 8197 | 0 |
| epoll-spin | 99 | 8193 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1017950752 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1654569 of 4387090 in 1-msg reads) | 1654569 | 4.76 | 3.69 | 7.62 | 15.57 | 25.12 | 41.08 | 204.44 |

#### Stalls: m2/lab-epoll-spin-many, loop 0 on CPU 3

161370304 iterations, longest 1850.0 µs; over 5 / 20 / 100 µs: 295705 / 783 / 12

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 343 | 13396.3 | 1850.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 438 | 11329.7 | 58.8 |
| softirq TIMER | 1 | 355.2 | 355.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 63.2 | 63.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1850.0 | 4481 | loop work (4481 items) |  |
| 2 | 719.3 | 1808 | loop work (1808 items) |  |
| 3 | 355.2 | 0 | softirq TIMER | softirq TIMER 327, local timer tick 19, timer tick_nohz_handler 12, irq work (IPI/vector) 1 |
| 4 | 200.7 | 787 | loop work (787 items) |  |
| 5 | 197.0 | 763 | loop work (763 items) |  |
| 6 | 181.2 | 767 | loop work (767 items) |  |
| 7 | 142.9 | 607 | loop work (607 items) |  |
| 8 | 142.6 | 500 | loop work (500 items) |  |
| 9 | 130.1 | 510 | loop work (510 items) |  |
| 10 | 127.0 | 529 | loop work (529 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1415458 clocksource=tsc

### Loop counters (m2/lab-recv-busy-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4544692 | 1920775 | 2.37 | 89414400 | 19.67 | 1490240 | 14902 | 97.9 | 69.1 | true | 0 | 4187 | 2464.6 | 894143 / 894143 / 33315 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 3 |
| recv-busy | 1 | 8200 | 3 |
| recv-busy | 2 | 8198 | 3 |
| recv-busy | 3 | 8200 | 3 |
| recv-busy | 4 | 8195 | 0 |
| recv-busy | 5 | 8195 | 3 |
| recv-busy | 6 | 8200 | 4 |
| recv-busy | 7 | 8198 | 4 |
| recv-busy | 8 | 8193 | 0 |
| recv-busy | 9 | 8200 | 4 |
| recv-busy | 10 | 8198 | 4 |
| recv-busy | 11 | 8193 | 3 |
| recv-busy | 12 | 8198 | 4 |
| recv-busy | 13 | 8193 | 3 |
| recv-busy | 14 | 8200 | 4 |
| recv-busy | 15 | 8195 | 3 |
| recv-busy | 16 | 8200 | 4 |
| recv-busy | 17 | 8194 | 4 |
| recv-busy | 18 | 8199 | 3 |
| recv-busy | 19 | 8196 | 4 |
| recv-busy | 20 | 8194 | 3 |
| recv-busy | 21 | 8194 | 4 |
| recv-busy | 22 | 8196 | 3 |
| recv-busy | 23 | 8196 | 3 |
| recv-busy | 24 | 8198 | 3 |
| recv-busy | 25 | 8200 | 4 |
| recv-busy | 26 | 8193 | 3 |
| recv-busy | 27 | 8200 | 3 |
| recv-busy | 28 | 8193 | 0 |
| recv-busy | 29 | 8200 | 4 |
| recv-busy | 30 | 8193 | 3 |
| recv-busy | 31 | 8199 | 3 |
| recv-busy | 32 | 8194 | 3 |
| recv-busy | 33 | 8197 | 0 |
| recv-busy | 34 | 8196 | 4 |
| recv-busy | 35 | 8197 | 0 |
| recv-busy | 36 | 8196 | 4 |
| recv-busy | 37 | 8194 | 4 |
| recv-busy | 38 | 8199 | 0 |
| recv-busy | 39 | 8196 | 3 |
| recv-busy | 40 | 8194 | 4 |
| recv-busy | 41 | 8197 | 3 |
| recv-busy | 42 | 8199 | 3 |
| recv-busy | 43 | 8197 | 3 |
| recv-busy | 44 | 8195 | 0 |
| recv-busy | 45 | 8195 | 0 |
| recv-busy | 46 | 8193 | 3 |
| recv-busy | 47 | 8198 | 4 |
| recv-busy | 48 | 8200 | 4 |
| recv-busy | 49 | 8199 | 3 |
| recv-busy | 50 | 8197 | 0 |
| recv-busy | 51 | 8199 | 0 |
| recv-busy | 52 | 8199 | 3 |
| recv-busy | 53 | 8194 | 3 |
| recv-busy | 54 | 8194 | 4 |
| recv-busy | 55 | 8200 | 3 |
| recv-busy | 56 | 8200 | 3 |
| recv-busy | 57 | 8200 | 3 |
| recv-busy | 58 | 8193 | 0 |
| recv-busy | 59 | 8193 | 0 |
| recv-busy | 60 | 8195 | 3 |
| recv-busy | 61 | 8198 | 3 |
| recv-busy | 62 | 8195 | 0 |
| recv-busy | 63 | 8200 | 3 |
| recv-busy | 64 | 8193 | 3 |
| recv-busy | 65 | 8198 | 3 |
| recv-busy | 66 | 8198 | 4 |
| recv-busy | 67 | 8200 | 4 |
| recv-busy | 68 | 8198 | 4 |
| recv-busy | 69 | 8193 | 3 |
| recv-busy | 70 | 8195 | 0 |
| recv-busy | 71 | 8199 | 3 |
| recv-busy | 72 | 8199 | 3 |
| recv-busy | 73 | 8199 | 0 |
| recv-busy | 74 | 8194 | 3 |
| recv-busy | 75 | 8194 | 4 |
| recv-busy | 76 | 8194 | 3 |
| recv-busy | 77 | 8198 | 4 |
| recv-busy | 78 | 8200 | 4 |
| recv-busy | 79 | 8193 | 3 |
| recv-busy | 80 | 8195 | 0 |
| recv-busy | 81 | 8200 | 3 |
| recv-busy | 82 | 8195 | 3 |
| recv-busy | 83 | 8195 | 0 |
| recv-busy | 84 | 8196 | 3 |
| recv-busy | 85 | 8199 | 0 |
| recv-busy | 86 | 8199 | 0 |
| recv-busy | 87 | 8197 | 0 |
| recv-busy | 88 | 8199 | 3 |
| recv-busy | 89 | 8196 | 4 |
| recv-busy | 90 | 8199 | 0 |
| recv-busy | 91 | 8197 | 0 |
| recv-busy | 92 | 8196 | 3 |
| recv-busy | 93 | 8194 | 3 |
| recv-busy | 94 | 8200 | 3 |
| recv-busy | 95 | 8195 | 0 |
| recv-busy | 96 | 8193 | 0 |
| recv-busy | 97 | 8195 | 3 |
| recv-busy | 98 | 8198 | 3 |
| recv-busy | 99 | 8200 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018011008 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1490604 of 4387250 in 1-msg reads) | 1490604 | 33.61 | 33.43 | 60.49 | 85.40 | 109.97 | 140.38 | 800.86 |

#### Stalls: m2/lab-recv-busy-many, loop 0 on CPU 3

894144 iterations, longest 2464.6 µs; over 5 / 20 / 100 µs: 894143 / 894143 / 33315

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 709955 | 44986380.0 | 849.0 |
| loop work | 153090 | 12723327.1 | 2464.6 |
| mostly unexplained (+softirq NET_RX) | 24760 | 1690888.7 | 169.9 |
| mostly unexplained (+local timer tick) | 3724 | 283155.7 | 166.6 |
| mostly unexplained (+preempted by kworker/3:1) | 2400 | 211452.0 | 163.7 |
| mostly unexplained (+irq work (IPI/vector)) | 161 | 12746.0 | 110.9 |
| mostly unexplained (+softirq NET_TX) | 20 | 1585.7 | 116.0 |
| mostly unexplained (+call function (IPI/vector)) | 10 | 685.0 | 76.2 |
| mostly unexplained (+ipi_send_cpu) | 9 | 666.3 | 86.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 9 | 561.6 | 69.2 |
| softirq TIMER | 1 | 255.9 | 255.9 |
| mostly unexplained (+softirq RCU) | 3 | 220.4 | 81.0 |
| mostly unexplained (+call function single (IPI/vector)) | 1 | 66.6 | 66.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2464.6 | 6240 | loop work (6240 items) |  |
| 2 | 849.0 | 16 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 255.9 | 4 | softirq TIMER | softirq TIMER 172, local timer tick 7, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 4 | 216.8 | 104 | loop work (104 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 208.7 | 59 | loop work (59 items) | softirq NET_RX 15, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 207.9 | 68 | loop work (68 items) |  |
| 7 | 203.4 | 65 | loop work (65 items) | softirq NET_RX 2 |
| 8 | 197.9 | 72 | loop work (72 items) | softirq NET_RX 3 |
| 9 | 194.9 | 95 | loop work (95 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, softirq NET_RX 1, workqueue [ena] 1 |
| 10 | 193.3 | 39 | loop work (39 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4218 |
| LOC Local timer interrupts | 4113 |
| CAL Function call interrupts | 991 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-spin-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-uring-spin-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 4544534 | 2288659 | 1.99 | 3553399 | 0.78 | 59223 | 6538257 | 0.0 | 10.6 | false | 0 | 0 | 1425.7 | 301494 / 1337 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8198 | 4 |
| uring-spin | 1 | 8200 | 4 |
| uring-spin | 2 | 8198 | 4 |
| uring-spin | 3 | 8198 | 4 |
| uring-spin | 4 | 8195 | 0 |
| uring-spin | 5 | 8198 | 4 |
| uring-spin | 6 | 8196 | 4 |
| uring-spin | 7 | 8194 | 4 |
| uring-spin | 8 | 8197 | 0 |
| uring-spin | 9 | 8196 | 4 |
| uring-spin | 10 | 8197 | 0 |
| uring-spin | 11 | 8194 | 4 |
| uring-spin | 12 | 8197 | 0 |
| uring-spin | 13 | 8196 | 4 |
| uring-spin | 14 | 8195 | 0 |
| uring-spin | 15 | 8200 | 4 |
| uring-spin | 16 | 8198 | 4 |
| uring-spin | 17 | 8193 | 0 |
| uring-spin | 18 | 8193 | 0 |
| uring-spin | 19 | 8200 | 4 |
| uring-spin | 20 | 8193 | 0 |
| uring-spin | 21 | 8195 | 0 |
| uring-spin | 22 | 8198 | 4 |
| uring-spin | 23 | 8199 | 0 |
| uring-spin | 24 | 8194 | 4 |
| uring-spin | 25 | 8194 | 4 |
| uring-spin | 26 | 8194 | 4 |
| uring-spin | 27 | 8197 | 0 |
| uring-spin | 28 | 8199 | 0 |
| uring-spin | 29 | 8193 | 0 |
| uring-spin | 30 | 8193 | 0 |
| uring-spin | 31 | 8200 | 4 |
| uring-spin | 32 | 8198 | 4 |
| uring-spin | 33 | 8195 | 0 |
| uring-spin | 34 | 8200 | 4 |
| uring-spin | 35 | 8200 | 4 |
| uring-spin | 36 | 8198 | 4 |
| uring-spin | 37 | 8193 | 0 |
| uring-spin | 38 | 8198 | 4 |
| uring-spin | 39 | 8198 | 4 |
| uring-spin | 40 | 8193 | 0 |
| uring-spin | 41 | 8200 | 4 |
| uring-spin | 42 | 8199 | 0 |
| uring-spin | 43 | 8194 | 4 |
| uring-spin | 44 | 8199 | 0 |
| uring-spin | 45 | 8194 | 4 |
| uring-spin | 46 | 8197 | 0 |
| uring-spin | 47 | 8197 | 0 |
| uring-spin | 48 | 8194 | 4 |
| uring-spin | 49 | 8199 | 0 |
| uring-spin | 50 | 8193 | 0 |
| uring-spin | 51 | 8193 | 0 |
| uring-spin | 52 | 8200 | 4 |
| uring-spin | 53 | 8198 | 4 |
| uring-spin | 54 | 8193 | 0 |
| uring-spin | 55 | 8193 | 0 |
| uring-spin | 56 | 8194 | 4 |
| uring-spin | 57 | 8196 | 4 |
| uring-spin | 58 | 8197 | 0 |
| uring-spin | 59 | 8197 | 0 |
| uring-spin | 60 | 8199 | 0 |
| uring-spin | 61 | 8194 | 4 |
| uring-spin | 62 | 8197 | 0 |
| uring-spin | 63 | 8196 | 4 |
| uring-spin | 64 | 8199 | 0 |
| uring-spin | 65 | 8197 | 0 |
| uring-spin | 66 | 8199 | 0 |
| uring-spin | 67 | 8194 | 4 |
| uring-spin | 68 | 8197 | 0 |
| uring-spin | 69 | 8197 | 0 |
| uring-spin | 70 | 8196 | 4 |
| uring-spin | 71 | 8196 | 4 |
| uring-spin | 72 | 8199 | 0 |
| uring-spin | 73 | 8193 | 0 |
| uring-spin | 74 | 8195 | 0 |
| uring-spin | 75 | 8195 | 0 |
| uring-spin | 76 | 8198 | 4 |
| uring-spin | 77 | 8198 | 4 |
| uring-spin | 78 | 8199 | 0 |
| uring-spin | 79 | 8197 | 0 |
| uring-spin | 80 | 8194 | 4 |
| uring-spin | 81 | 8199 | 0 |
| uring-spin | 82 | 8197 | 0 |
| uring-spin | 83 | 8194 | 4 |
| uring-spin | 84 | 8193 | 0 |
| uring-spin | 85 | 8200 | 4 |
| uring-spin | 86 | 8198 | 4 |
| uring-spin | 87 | 8195 | 0 |
| uring-spin | 88 | 8193 | 0 |
| uring-spin | 89 | 8195 | 0 |
| uring-spin | 90 | 8195 | 0 |
| uring-spin | 91 | 8195 | 0 |
| uring-spin | 92 | 8195 | 0 |
| uring-spin | 93 | 8198 | 4 |
| uring-spin | 94 | 8193 | 0 |
| uring-spin | 95 | 8193 | 0 |
| uring-spin | 96 | 8197 | 0 |
| uring-spin | 97 | 8194 | 4 |
| uring-spin | 98 | 8197 | 0 |
| uring-spin | 99 | 8194 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 1017975616 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (1681510 of 4387056 in 1-msg reads) | 1681510 | 5.38 | 4.11 | 8.48 | 18.51 | 28.68 | 38.80 | 751.51 |

#### Stalls: m2/lab-uring-spin-many, loop 0 on CPU 3

392295424 iterations, longest 1425.7 µs; over 5 / 20 / 100 µs: 301494 / 1337 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 645 | 17928.1 | 1425.7 |
| loop work | 689 | 17214.9 | 734.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 74.0 | 28.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1425.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 798.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 734.1 | 3909 | loop work (3909 items) |  |
| 4 | 475.3 | 2403 | loop work (2403 items) |  |
| 5 | 51.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 49.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 47.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 47.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 46.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2009956 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4544423 | 2207920 | 2.06 | 1 | 0.00 | 0 | 7214397 | 0.0 | 0.0 | true | 0 | 0 | 662.7 | 8317 / 27 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8197 | 1 |
| uring-sqpoll | 1 | 8194 | 1 |
| uring-sqpoll | 2 | 8199 | 1 |
| uring-sqpoll | 3 | 8196 | 1 |
| uring-sqpoll | 4 | 8199 | 1 |
| uring-sqpoll | 5 | 8197 | 0 |
| uring-sqpoll | 6 | 8194 | 1 |
| uring-sqpoll | 7 | 8199 | 1 |
| uring-sqpoll | 8 | 8196 | 1 |
| uring-sqpoll | 9 | 8193 | 1 |
| uring-sqpoll | 10 | 8195 | 1 |
| uring-sqpoll | 11 | 8195 | 1 |
| uring-sqpoll | 12 | 8200 | 1 |
| uring-sqpoll | 13 | 8193 | 1 |
| uring-sqpoll | 14 | 8199 | 1 |
| uring-sqpoll | 15 | 8196 | 1 |
| uring-sqpoll | 16 | 8197 | 1 |
| uring-sqpoll | 17 | 8197 | 1 |
| uring-sqpoll | 18 | 8194 | 1 |
| uring-sqpoll | 19 | 8199 | 1 |
| uring-sqpoll | 20 | 8199 | 1 |
| uring-sqpoll | 21 | 8199 | 1 |
| uring-sqpoll | 22 | 8194 | 1 |
| uring-sqpoll | 23 | 8195 | 1 |
| uring-sqpoll | 24 | 8198 | 1 |
| uring-sqpoll | 25 | 8198 | 1 |
| uring-sqpoll | 26 | 8200 | 1 |
| uring-sqpoll | 27 | 8198 | 1 |
| uring-sqpoll | 28 | 8195 | 1 |
| uring-sqpoll | 29 | 8195 | 1 |
| uring-sqpoll | 30 | 8193 | 1 |
| uring-sqpoll | 31 | 8198 | 1 |
| uring-sqpoll | 32 | 8195 | 1 |
| uring-sqpoll | 33 | 8198 | 1 |
| uring-sqpoll | 34 | 8195 | 0 |
| uring-sqpoll | 35 | 8198 | 1 |
| uring-sqpoll | 36 | 8196 | 1 |
| uring-sqpoll | 37 | 8197 | 1 |
| uring-sqpoll | 38 | 8194 | 1 |
| uring-sqpoll | 39 | 8196 | 1 |
| uring-sqpoll | 40 | 8197 | 1 |
| uring-sqpoll | 41 | 8196 | 1 |
| uring-sqpoll | 42 | 8200 | 1 |
| uring-sqpoll | 43 | 8193 | 1 |
| uring-sqpoll | 44 | 8200 | 1 |
| uring-sqpoll | 45 | 8198 | 1 |
| uring-sqpoll | 46 | 8200 | 1 |
| uring-sqpoll | 47 | 8198 | 1 |
| uring-sqpoll | 48 | 8199 | 1 |
| uring-sqpoll | 49 | 8197 | 1 |
| uring-sqpoll | 50 | 8194 | 1 |
| uring-sqpoll | 51 | 8197 | 1 |
| uring-sqpoll | 52 | 8199 | 1 |
| uring-sqpoll | 53 | 8197 | 1 |
| uring-sqpoll | 54 | 8197 | 1 |
| uring-sqpoll | 55 | 8194 | 1 |
| uring-sqpoll | 56 | 8199 | 1 |
| uring-sqpoll | 57 | 8194 | 1 |
| uring-sqpoll | 58 | 8196 | 1 |
| uring-sqpoll | 59 | 8199 | 1 |
| uring-sqpoll | 60 | 8194 | 1 |
| uring-sqpoll | 61 | 8197 | 1 |
| uring-sqpoll | 62 | 8193 | 1 |
| uring-sqpoll | 63 | 8198 | 4 |
| uring-sqpoll | 64 | 8200 | 1 |
| uring-sqpoll | 65 | 8200 | 1 |
| uring-sqpoll | 66 | 8195 | 0 |
| uring-sqpoll | 67 | 8193 | 1 |
| uring-sqpoll | 68 | 8195 | 1 |
| uring-sqpoll | 69 | 8195 | 1 |
| uring-sqpoll | 70 | 8195 | 0 |
| uring-sqpoll | 71 | 8200 | 1 |
| uring-sqpoll | 72 | 8198 | 1 |
| uring-sqpoll | 73 | 8197 | 1 |
| uring-sqpoll | 74 | 8196 | 1 |
| uring-sqpoll | 75 | 8199 | 1 |
| uring-sqpoll | 76 | 8197 | 1 |
| uring-sqpoll | 77 | 8196 | 1 |
| uring-sqpoll | 78 | 8194 | 1 |
| uring-sqpoll | 79 | 8196 | 1 |
| uring-sqpoll | 80 | 8200 | 1 |
| uring-sqpoll | 81 | 8195 | 1 |
| uring-sqpoll | 82 | 8195 | 1 |
| uring-sqpoll | 83 | 8198 | 1 |
| uring-sqpoll | 84 | 8198 | 4 |
| uring-sqpoll | 85 | 8198 | 1 |
| uring-sqpoll | 86 | 8198 | 1 |
| uring-sqpoll | 87 | 8200 | 1 |
| uring-sqpoll | 88 | 8200 | 1 |
| uring-sqpoll | 89 | 8198 | 1 |
| uring-sqpoll | 90 | 8200 | 1 |
| uring-sqpoll | 91 | 8198 | 1 |
| uring-sqpoll | 92 | 8193 | 0 |
| uring-sqpoll | 93 | 8194 | 1 |
| uring-sqpoll | 94 | 8199 | 1 |
| uring-sqpoll | 95 | 8197 | 1 |
| uring-sqpoll | 96 | 8197 | 1 |
| uring-sqpoll | 97 | 8197 | 1 |
| uring-sqpoll | 98 | 8196 | 1 |
| uring-sqpoll | 99 | 8194 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1017950752 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1573941 of 4386947 in 1-msg reads) | 1573941 | 5.01 | 4.22 | 7.40 | 17.25 | 33.15 | 77.90 | 308.43 |

#### Stalls: m2/lab-uring-sqpoll-many, loop 0 on CPU 3

432863808 iterations, longest 662.7 µs; over 5 / 20 / 100 µs: 8317 / 27 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 25 | 2318.3 | 662.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 51.3 | 29.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 662.7 | 3908 | loop work (3908 items) |  |
| 2 | 410.2 | 2356 | loop work (2356 items) |  |
| 3 | 125.7 | 755 | loop work (755 items) |  |
| 4 | 99.2 | 595 | loop work (595 items) |  |
| 5 | 86.0 | 517 | loop work (517 items) |  |
| 6 | 79.2 | 474 | loop work (474 items) |  |
| 7 | 77.1 | 461 | loop work (461 items) |  |
| 8 | 72.3 | 426 | loop work (426 items) |  |
| 9 | 71.8 | 410 | loop work (410 items) |  |
| 10 | 70.0 | 392 | loop work (392 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m3/lab-epoll-spin-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-epoll-spin-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4544528 | 2279749 | 1.99 | 165248517 | 36.36 | 2754142 | 2716146 | 0.0 | 33.1 | false | 0 | 0 | 1915.6 | 287894 / 392 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8194 | 4 |
| epoll-spin | 1 | 8199 | 0 |
| epoll-spin | 2 | 8194 | 4 |
| epoll-spin | 3 | 8199 | 0 |
| epoll-spin | 4 | 8194 | 4 |
| epoll-spin | 5 | 8198 | 4 |
| epoll-spin | 6 | 8198 | 4 |
| epoll-spin | 7 | 8193 | 0 |
| epoll-spin | 8 | 8200 | 4 |
| epoll-spin | 9 | 8193 | 0 |
| epoll-spin | 10 | 8193 | 0 |
| epoll-spin | 11 | 8193 | 0 |
| epoll-spin | 12 | 8195 | 0 |
| epoll-spin | 13 | 8198 | 4 |
| epoll-spin | 14 | 8193 | 0 |
| epoll-spin | 15 | 8199 | 0 |
| epoll-spin | 16 | 8197 | 0 |
| epoll-spin | 17 | 8196 | 4 |
| epoll-spin | 18 | 8199 | 0 |
| epoll-spin | 19 | 8197 | 0 |
| epoll-spin | 20 | 8199 | 0 |
| epoll-spin | 21 | 8198 | 4 |
| epoll-spin | 22 | 8198 | 4 |
| epoll-spin | 23 | 8198 | 4 |
| epoll-spin | 24 | 8195 | 0 |
| epoll-spin | 25 | 8198 | 4 |
| epoll-spin | 26 | 8193 | 0 |
| epoll-spin | 27 | 8194 | 4 |
| epoll-spin | 28 | 8194 | 4 |
| epoll-spin | 29 | 8194 | 4 |
| epoll-spin | 30 | 8196 | 4 |
| epoll-spin | 31 | 8194 | 4 |
| epoll-spin | 32 | 8199 | 0 |
| epoll-spin | 33 | 8199 | 0 |
| epoll-spin | 34 | 8196 | 4 |
| epoll-spin | 35 | 8197 | 0 |
| epoll-spin | 36 | 8196 | 4 |
| epoll-spin | 37 | 8196 | 4 |
| epoll-spin | 38 | 8194 | 4 |
| epoll-spin | 39 | 8196 | 4 |
| epoll-spin | 40 | 8199 | 0 |
| epoll-spin | 41 | 8196 | 4 |
| epoll-spin | 42 | 8195 | 0 |
| epoll-spin | 43 | 8198 | 4 |
| epoll-spin | 44 | 8200 | 4 |
| epoll-spin | 45 | 8193 | 0 |
| epoll-spin | 46 | 8193 | 0 |
| epoll-spin | 47 | 8193 | 0 |
| epoll-spin | 48 | 8200 | 4 |
| epoll-spin | 49 | 8196 | 4 |
| epoll-spin | 50 | 8197 | 0 |
| epoll-spin | 51 | 8194 | 4 |
| epoll-spin | 52 | 8197 | 0 |
| epoll-spin | 53 | 8199 | 0 |
| epoll-spin | 54 | 8194 | 4 |
| epoll-spin | 55 | 8193 | 0 |
| epoll-spin | 56 | 8193 | 0 |
| epoll-spin | 57 | 8195 | 0 |
| epoll-spin | 58 | 8193 | 0 |
| epoll-spin | 59 | 8193 | 0 |
| epoll-spin | 60 | 8198 | 4 |
| epoll-spin | 61 | 8200 | 4 |
| epoll-spin | 62 | 8195 | 0 |
| epoll-spin | 63 | 8193 | 0 |
| epoll-spin | 64 | 8198 | 4 |
| epoll-spin | 65 | 8195 | 0 |
| epoll-spin | 66 | 8200 | 4 |
| epoll-spin | 67 | 8198 | 4 |
| epoll-spin | 68 | 8197 | 0 |
| epoll-spin | 69 | 8197 | 0 |
| epoll-spin | 70 | 8197 | 0 |
| epoll-spin | 71 | 8199 | 0 |
| epoll-spin | 72 | 8199 | 0 |
| epoll-spin | 73 | 8197 | 0 |
| epoll-spin | 74 | 8195 | 0 |
| epoll-spin | 75 | 8198 | 4 |
| epoll-spin | 76 | 8198 | 4 |
| epoll-spin | 77 | 8195 | 0 |
| epoll-spin | 78 | 8195 | 0 |
| epoll-spin | 79 | 8200 | 4 |
| epoll-spin | 80 | 8193 | 0 |
| epoll-spin | 81 | 8194 | 4 |
| epoll-spin | 82 | 8199 | 0 |
| epoll-spin | 83 | 8194 | 4 |
| epoll-spin | 84 | 8196 | 4 |
| epoll-spin | 85 | 8194 | 4 |
| epoll-spin | 86 | 8197 | 0 |
| epoll-spin | 87 | 8197 | 0 |
| epoll-spin | 88 | 8199 | 0 |
| epoll-spin | 89 | 8197 | 0 |
| epoll-spin | 90 | 8199 | 0 |
| epoll-spin | 91 | 8196 | 4 |
| epoll-spin | 92 | 8196 | 4 |
| epoll-spin | 93 | 8195 | 0 |
| epoll-spin | 94 | 8195 | 0 |
| epoll-spin | 95 | 8195 | 0 |
| epoll-spin | 96 | 8198 | 4 |
| epoll-spin | 97 | 8200 | 4 |
| epoll-spin | 98 | 8198 | 4 |
| epoll-spin | 99 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1017974272 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1671582 of 4387090 in 1-msg reads) | 1671582 | 4.68 | 3.66 | 7.50 | 14.69 | 22.72 | 30.57 | 76.34 |

#### Stalls: m3/lab-epoll-spin-many, loop 0 on CPU 3

162968768 iterations, longest 1915.6 µs; over 5 / 20 / 100 µs: 287894 / 392 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 310 | 7877.6 | 75.5 |
| loop work | 80 | 4618.7 | 1915.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 53.4 | 31.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1915.6 | 4487 | loop work (4487 items) |  |
| 2 | 760.6 | 1877 | loop work (1877 items) |  |
| 3 | 75.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 51.2 | 3 | loop work (3 items) |  |
| 5 | 44.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 43.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 43.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 42.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 40.9 | 7 | loop work (7 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1400915 clocksource=tsc

### Loop counters (m3/lab-recv-busy-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4544407 | 1914335 | 2.37 | 89312000 | 19.65 | 1488533 | 14885 | 97.9 | 69.1 | true | 0 | 4305 | 2497.9 | 893119 / 893119 / 35909 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 0 |
| recv-busy | 1 | 8198 | 3 |
| recv-busy | 2 | 8200 | 3 |
| recv-busy | 3 | 8195 | 3 |
| recv-busy | 4 | 8198 | 4 |
| recv-busy | 5 | 8195 | 0 |
| recv-busy | 6 | 8193 | 3 |
| recv-busy | 7 | 8195 | 0 |
| recv-busy | 8 | 8197 | 0 |
| recv-busy | 9 | 8197 | 3 |
| recv-busy | 10 | 8199 | 3 |
| recv-busy | 11 | 8194 | 3 |
| recv-busy | 12 | 8194 | 4 |
| recv-busy | 13 | 8194 | 4 |
| recv-busy | 14 | 8196 | 4 |
| recv-busy | 15 | 8199 | 3 |
| recv-busy | 16 | 8199 | 3 |
| recv-busy | 17 | 8196 | 3 |
| recv-busy | 18 | 8197 | 0 |
| recv-busy | 19 | 8196 | 4 |
| recv-busy | 20 | 8194 | 3 |
| recv-busy | 21 | 8196 | 4 |
| recv-busy | 22 | 8195 | 0 |
| recv-busy | 23 | 8200 | 4 |
| recv-busy | 24 | 8195 | 3 |
| recv-busy | 25 | 8193 | 3 |
| recv-busy | 26 | 8193 | 3 |
| recv-busy | 27 | 8200 | 4 |
| recv-busy | 28 | 8195 | 0 |
| recv-busy | 29 | 8199 | 3 |
| recv-busy | 30 | 8197 | 0 |
| recv-busy | 31 | 8197 | 0 |
| recv-busy | 32 | 8196 | 4 |
| recv-busy | 33 | 8199 | 3 |
| recv-busy | 34 | 8196 | 4 |
| recv-busy | 35 | 8194 | 3 |
| recv-busy | 36 | 8199 | 0 |
| recv-busy | 37 | 8195 | 3 |
| recv-busy | 38 | 8195 | 0 |
| recv-busy | 39 | 8193 | 0 |
| recv-busy | 40 | 8198 | 3 |
| recv-busy | 41 | 8200 | 3 |
| recv-busy | 42 | 8198 | 4 |
| recv-busy | 43 | 8200 | 4 |
| recv-busy | 44 | 8195 | 3 |
| recv-busy | 45 | 8193 | 3 |
| recv-busy | 46 | 8195 | 3 |
| recv-busy | 47 | 8195 | 3 |
| recv-busy | 48 | 8198 | 3 |
| recv-busy | 49 | 8195 | 3 |
| recv-busy | 50 | 8195 | 0 |
| recv-busy | 51 | 8197 | 3 |
| recv-busy | 52 | 8196 | 3 |
| recv-busy | 53 | 8197 | 0 |
| recv-busy | 54 | 8196 | 4 |
| recv-busy | 55 | 8196 | 4 |
| recv-busy | 56 | 8194 | 3 |
| recv-busy | 57 | 8199 | 3 |
| recv-busy | 58 | 8196 | 3 |
| recv-busy | 59 | 8194 | 4 |
| recv-busy | 60 | 8200 | 3 |
| recv-busy | 61 | 8193 | 0 |
| recv-busy | 62 | 8200 | 3 |
| recv-busy | 63 | 8198 | 3 |
| recv-busy | 64 | 8193 | 3 |
| recv-busy | 65 | 8198 | 4 |
| recv-busy | 66 | 8195 | 3 |
| recv-busy | 67 | 8195 | 0 |
| recv-busy | 68 | 8200 | 4 |
| recv-busy | 69 | 8194 | 3 |
| recv-busy | 70 | 8196 | 3 |
| recv-busy | 71 | 8199 | 3 |
| recv-busy | 72 | 8194 | 3 |
| recv-busy | 73 | 8194 | 4 |
| recv-busy | 74 | 8196 | 3 |
| recv-busy | 75 | 8194 | 3 |
| recv-busy | 76 | 8197 | 3 |
| recv-busy | 77 | 8196 | 3 |
| recv-busy | 78 | 8194 | 3 |
| recv-busy | 79 | 8197 | 3 |
| recv-busy | 80 | 8196 | 4 |
| recv-busy | 81 | 8197 | 0 |
| recv-busy | 82 | 8196 | 4 |
| recv-busy | 83 | 8200 | 4 |
| recv-busy | 84 | 8198 | 3 |
| recv-busy | 85 | 8198 | 3 |
| recv-busy | 86 | 8200 | 3 |
| recv-busy | 87 | 8193 | 0 |
| recv-busy | 88 | 8195 | 3 |
| recv-busy | 89 | 8197 | 0 |
| recv-busy | 90 | 8194 | 4 |
| recv-busy | 91 | 8196 | 3 |
| recv-busy | 92 | 8199 | 3 |
| recv-busy | 93 | 8199 | 3 |
| recv-busy | 94 | 8194 | 3 |
| recv-busy | 95 | 8199 | 3 |
| recv-busy | 96 | 8200 | 3 |
| recv-busy | 97 | 8195 | 3 |
| recv-busy | 98 | 8195 | 3 |
| recv-busy | 99 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1017947168 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1490075 of 4387088 in 1-msg reads) | 1490075 | 33.82 | 32.73 | 60.46 | 87.45 | 114.59 | 146.18 | 200.19 |

#### Stalls: m3/lab-recv-busy-many, loop 0 on CPU 3

893120 iterations, longest 2497.9 µs; over 5 / 20 / 100 µs: 893119 / 893119 / 35909

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 710816 | 44950973.7 | 169.6 |
| loop work | 151420 | 12771576.0 | 2497.9 |
| mostly unexplained (+softirq NET_RX) | 24437 | 1667774.6 | 173.9 |
| mostly unexplained (+local timer tick) | 3809 | 289963.2 | 148.5 |
| mostly unexplained (+preempted by kworker/3:1) | 2384 | 210406.0 | 168.4 |
| mostly unexplained (+irq work (IPI/vector)) | 220 | 17504.0 | 130.2 |
| mostly unexplained (+softirq NET_TX) | 15 | 1199.9 | 105.6 |
| mostly unexplained (+call function (IPI/vector)) | 9 | 638.0 | 81.9 |
| mostly unexplained (+ipi_send_cpu) | 4 | 292.1 | 80.3 |
| mostly unexplained (+softirq RCU) | 2 | 147.5 | 87.1 |
| mostly unexplained (+call function single (IPI/vector)) | 1 | 86.2 | 86.2 |
| mostly unexplained (+softirq TIMER) | 1 | 82.3 | 82.3 |
| mostly unexplained (+softirq TASKLET) | 1 | 73.0 | 73.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2497.9 | 6181 | loop work (6181 items) |  |
| 2 | 230.5 | 170 | loop work (170 items) |  |
| 3 | 221.0 | 77 | loop work (77 items) | softirq NET_TX 2, softirq NET_RX 1 |
| 4 | 220.8 | 89 | loop work (89 items) |  |
| 5 | 220.3 | 82 | loop work (82 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 208.7 | 54 | loop work (54 items) |  |
| 7 | 207.7 | 87 | loop work (87 items) | softirq NET_RX 1 |
| 8 | 207.3 | 46 | loop work (46 items) | softirq NET_RX 6 |
| 9 | 206.5 | 79 | loop work (79 items) | softirq NET_RX 3 |
| 10 | 205.7 | 66 | loop work (66 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4341 |
| LOC Local timer interrupts | 4250 |
| CAL Function call interrupts | 990 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-spin-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-uring-spin-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 4544421 | 2293783 | 1.98 | 3578334 | 0.79 | 59639 | 6599594 | 0.0 | 10.4 | false | 0 | 0 | 1412.5 | 290863 / 554 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8197 | 0 |
| uring-spin | 1 | 8196 | 4 |
| uring-spin | 2 | 8196 | 4 |
| uring-spin | 3 | 8194 | 4 |
| uring-spin | 4 | 8194 | 4 |
| uring-spin | 5 | 8196 | 4 |
| uring-spin | 6 | 8196 | 4 |
| uring-spin | 7 | 8197 | 0 |
| uring-spin | 8 | 8194 | 4 |
| uring-spin | 9 | 8197 | 0 |
| uring-spin | 10 | 8193 | 0 |
| uring-spin | 11 | 8195 | 0 |
| uring-spin | 12 | 8195 | 0 |
| uring-spin | 13 | 8198 | 4 |
| uring-spin | 14 | 8193 | 0 |
| uring-spin | 15 | 8200 | 4 |
| uring-spin | 16 | 8193 | 0 |
| uring-spin | 17 | 8196 | 4 |
| uring-spin | 18 | 8196 | 4 |
| uring-spin | 19 | 8196 | 4 |
| uring-spin | 20 | 8197 | 0 |
| uring-spin | 21 | 8199 | 0 |
| uring-spin | 22 | 8194 | 4 |
| uring-spin | 23 | 8194 | 4 |
| uring-spin | 24 | 8196 | 4 |
| uring-spin | 25 | 8194 | 4 |
| uring-spin | 26 | 8193 | 0 |
| uring-spin | 27 | 8195 | 0 |
| uring-spin | 28 | 8193 | 0 |
| uring-spin | 29 | 8198 | 4 |
| uring-spin | 30 | 8193 | 0 |
| uring-spin | 31 | 8193 | 0 |
| uring-spin | 32 | 8195 | 0 |
| uring-spin | 33 | 8198 | 4 |
| uring-spin | 34 | 8200 | 4 |
| uring-spin | 35 | 8200 | 4 |
| uring-spin | 36 | 8198 | 4 |
| uring-spin | 37 | 8198 | 4 |
| uring-spin | 38 | 8193 | 0 |
| uring-spin | 39 | 8193 | 0 |
| uring-spin | 40 | 8195 | 0 |
| uring-spin | 41 | 8198 | 4 |
| uring-spin | 42 | 8199 | 0 |
| uring-spin | 43 | 8196 | 4 |
| uring-spin | 44 | 8197 | 0 |
| uring-spin | 45 | 8194 | 4 |
| uring-spin | 46 | 8196 | 4 |
| uring-spin | 47 | 8199 | 0 |
| uring-spin | 48 | 8198 | 4 |
| uring-spin | 49 | 8193 | 0 |
| uring-spin | 50 | 8198 | 4 |
| uring-spin | 51 | 8195 | 0 |
| uring-spin | 52 | 8200 | 4 |
| uring-spin | 53 | 8195 | 0 |
| uring-spin | 54 | 8197 | 0 |
| uring-spin | 55 | 8194 | 4 |
| uring-spin | 56 | 8197 | 0 |
| uring-spin | 57 | 8199 | 0 |
| uring-spin | 58 | 8194 | 4 |
| uring-spin | 59 | 8197 | 0 |
| uring-spin | 60 | 8199 | 0 |
| uring-spin | 61 | 8199 | 0 |
| uring-spin | 62 | 8199 | 0 |
| uring-spin | 63 | 8196 | 4 |
| uring-spin | 64 | 8196 | 4 |
| uring-spin | 65 | 8197 | 0 |
| uring-spin | 66 | 8196 | 4 |
| uring-spin | 67 | 8196 | 4 |
| uring-spin | 68 | 8199 | 0 |
| uring-spin | 69 | 8196 | 4 |
| uring-spin | 70 | 8195 | 0 |
| uring-spin | 71 | 8200 | 4 |
| uring-spin | 72 | 8200 | 4 |
| uring-spin | 73 | 8200 | 4 |
| uring-spin | 74 | 8200 | 4 |
| uring-spin | 75 | 8195 | 0 |
| uring-spin | 76 | 8198 | 4 |
| uring-spin | 77 | 8196 | 4 |
| uring-spin | 78 | 8199 | 0 |
| uring-spin | 79 | 8197 | 0 |
| uring-spin | 80 | 8199 | 0 |
| uring-spin | 81 | 8197 | 0 |
| uring-spin | 82 | 8197 | 0 |
| uring-spin | 83 | 8197 | 0 |
| uring-spin | 84 | 8199 | 0 |
| uring-spin | 85 | 8200 | 4 |
| uring-spin | 86 | 8200 | 4 |
| uring-spin | 87 | 8200 | 4 |
| uring-spin | 88 | 8193 | 0 |
| uring-spin | 89 | 8195 | 0 |
| uring-spin | 90 | 8200 | 4 |
| uring-spin | 91 | 8195 | 0 |
| uring-spin | 92 | 8198 | 4 |
| uring-spin | 93 | 8200 | 4 |
| uring-spin | 94 | 8198 | 4 |
| uring-spin | 95 | 8200 | 4 |
| uring-spin | 96 | 8195 | 0 |
| uring-spin | 97 | 8193 | 0 |
| uring-spin | 98 | 8193 | 0 |
| uring-spin | 99 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 1017950304 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (1683685 of 4386957 in 1-msg reads) | 1683685 | 5.27 | 4.08 | 8.33 | 16.91 | 25.18 | 33.85 | 258.26 |

#### Stalls: m3/lab-uring-spin-many, loop 0 on CPU 3

395975616 iterations, longest 1412.5 µs; over 5 / 20 / 100 µs: 290863 / 554 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 356 | 10067.2 | 1412.5 |
| loop work | 195 | 5646.8 | 720.2 |
| softirq TIMER | 1 | 294.4 | 294.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 47.5 | 25.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1412.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 720.2 | 3861 | loop work (3861 items) |  |
| 3 | 460.7 | 2375 | loop work (2375 items) |  |
| 4 | 294.4 | 0 | softirq TIMER | softirq TIMER 255, local timer tick 19, timer tick_nohz_handler 10, irq work (IPI/vector) 1 |
| 5 | 43.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 39.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 39.7 | 126 | loop work (126 items) |  |
| 9 | 37.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.5 | 2 | loop work (2 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 9 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-many: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2009308 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-many)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4544421 | 2204956 | 2.06 | 1 | 0.00 | 0 | 6971327 | 0.0 | 0.0 | true | 0 | 0 | 679.1 | 8830 / 4 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 1 |
| uring-sqpoll | 1 | 8194 | 1 |
| uring-sqpoll | 2 | 8194 | 1 |
| uring-sqpoll | 3 | 8199 | 1 |
| uring-sqpoll | 4 | 8194 | 1 |
| uring-sqpoll | 5 | 8197 | 1 |
| uring-sqpoll | 6 | 8197 | 1 |
| uring-sqpoll | 7 | 8195 | 1 |
| uring-sqpoll | 8 | 8200 | 1 |
| uring-sqpoll | 9 | 8198 | 1 |
| uring-sqpoll | 10 | 8200 | 1 |
| uring-sqpoll | 11 | 8198 | 1 |
| uring-sqpoll | 12 | 8193 | 1 |
| uring-sqpoll | 13 | 8195 | 1 |
| uring-sqpoll | 14 | 8200 | 1 |
| uring-sqpoll | 15 | 8195 | 1 |
| uring-sqpoll | 16 | 8195 | 1 |
| uring-sqpoll | 17 | 8198 | 4 |
| uring-sqpoll | 18 | 8200 | 1 |
| uring-sqpoll | 19 | 8195 | 1 |
| uring-sqpoll | 20 | 8199 | 1 |
| uring-sqpoll | 21 | 8196 | 1 |
| uring-sqpoll | 22 | 8197 | 1 |
| uring-sqpoll | 23 | 8197 | 1 |
| uring-sqpoll | 24 | 8196 | 1 |
| uring-sqpoll | 25 | 8199 | 1 |
| uring-sqpoll | 26 | 8196 | 1 |
| uring-sqpoll | 27 | 8198 | 1 |
| uring-sqpoll | 28 | 8198 | 1 |
| uring-sqpoll | 29 | 8200 | 1 |
| uring-sqpoll | 30 | 8193 | 1 |
| uring-sqpoll | 31 | 8200 | 1 |
| uring-sqpoll | 32 | 8198 | 1 |
| uring-sqpoll | 33 | 8199 | 1 |
| uring-sqpoll | 34 | 8194 | 4 |
| uring-sqpoll | 35 | 8194 | 1 |
| uring-sqpoll | 36 | 8199 | 1 |
| uring-sqpoll | 37 | 8197 | 1 |
| uring-sqpoll | 38 | 8197 | 1 |
| uring-sqpoll | 39 | 8194 | 1 |
| uring-sqpoll | 40 | 8196 | 1 |
| uring-sqpoll | 41 | 8197 | 1 |
| uring-sqpoll | 42 | 8196 | 1 |
| uring-sqpoll | 43 | 8199 | 0 |
| uring-sqpoll | 44 | 8194 | 1 |
| uring-sqpoll | 45 | 8200 | 1 |
| uring-sqpoll | 46 | 8193 | 1 |
| uring-sqpoll | 47 | 8198 | 1 |
| uring-sqpoll | 48 | 8198 | 4 |
| uring-sqpoll | 49 | 8200 | 1 |
| uring-sqpoll | 50 | 8198 | 1 |
| uring-sqpoll | 51 | 8195 | 1 |
| uring-sqpoll | 52 | 8196 | 1 |
| uring-sqpoll | 53 | 8199 | 1 |
| uring-sqpoll | 54 | 8197 | 1 |
| uring-sqpoll | 55 | 8197 | 1 |
| uring-sqpoll | 56 | 8199 | 1 |
| uring-sqpoll | 57 | 8199 | 1 |
| uring-sqpoll | 58 | 8199 | 1 |
| uring-sqpoll | 59 | 8196 | 1 |
| uring-sqpoll | 60 | 8200 | 4 |
| uring-sqpoll | 61 | 8198 | 1 |
| uring-sqpoll | 62 | 8200 | 1 |
| uring-sqpoll | 63 | 8195 | 0 |
| uring-sqpoll | 64 | 8200 | 1 |
| uring-sqpoll | 65 | 8200 | 1 |
| uring-sqpoll | 66 | 8198 | 1 |
| uring-sqpoll | 67 | 8195 | 1 |
| uring-sqpoll | 68 | 8200 | 1 |
| uring-sqpoll | 69 | 8195 | 1 |
| uring-sqpoll | 70 | 8200 | 1 |
| uring-sqpoll | 71 | 8198 | 1 |
| uring-sqpoll | 72 | 8193 | 1 |
| uring-sqpoll | 73 | 8198 | 1 |
| uring-sqpoll | 74 | 8195 | 1 |
| uring-sqpoll | 75 | 8197 | 1 |
| uring-sqpoll | 76 | 8197 | 1 |
| uring-sqpoll | 77 | 8196 | 1 |
| uring-sqpoll | 78 | 8197 | 1 |
| uring-sqpoll | 79 | 8196 | 1 |
| uring-sqpoll | 80 | 8197 | 1 |
| uring-sqpoll | 81 | 8197 | 1 |
| uring-sqpoll | 82 | 8200 | 1 |
| uring-sqpoll | 83 | 8195 | 1 |
| uring-sqpoll | 84 | 8195 | 1 |
| uring-sqpoll | 85 | 8193 | 1 |
| uring-sqpoll | 86 | 8198 | 1 |
| uring-sqpoll | 87 | 8200 | 1 |
| uring-sqpoll | 88 | 8197 | 1 |
| uring-sqpoll | 89 | 8197 | 1 |
| uring-sqpoll | 90 | 8194 | 1 |
| uring-sqpoll | 91 | 8197 | 1 |
| uring-sqpoll | 92 | 8197 | 0 |
| uring-sqpoll | 93 | 8194 | 1 |
| uring-sqpoll | 94 | 8194 | 1 |
| uring-sqpoll | 95 | 8194 | 1 |
| uring-sqpoll | 96 | 8199 | 1 |
| uring-sqpoll | 97 | 8194 | 1 |
| uring-sqpoll | 98 | 8199 | 1 |
| uring-sqpoll | 99 | 8199 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1017950304 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1572957 of 4386951 in 1-msg reads) | 1572957 | 5.02 | 4.21 | 7.41 | 17.41 | 32.66 | 46.96 | 68.78 |

#### Stalls: m3/lab-uring-sqpoll-many, loop 0 on CPU 3

418279616 iterations, longest 679.0 µs; over 5 / 20 / 100 µs: 8830 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1090.6 | 679.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.2 | 31.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 679.0 | 3882 | loop work (3882 items) |  |
| 2 | 411.5 | 2359 | loop work (2359 items) |  |
| 3 | 31.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 20.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 3 |
| TLB TLB shootdowns | 1 |

