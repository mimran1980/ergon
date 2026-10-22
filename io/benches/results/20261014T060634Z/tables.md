## Summary: paired runs (µs; Δ = B − A, negative means B decoded first)

| run | A | B | paired msgs | Δuser p50 | Δuser p90 | Δuser p99 | Δrx p50 | Δstack p50 | Δstack p99 | A syscalls/s | B syscalls/s | A kernel CPU % | B kernel CPU % | shared RX queue |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lab-burst-aa-epoll-spin | epoll-spin | epoll-spin | 11499950 | 0.81 | 17.12 | 35.38 | -3.04 | 1.02 | 18.16 | 3640341 | 3616027 | 36.4 | 37.6 | no |
| lab-burst-aa-recv-busy | recv-busy | recv-busy | 11500004 | -0.34 | 12.89 | 41.69 | 0.26 | -0.13 | 20.46 | 1193207 | 1189637 | 78.8 | 78.0 | no |
| lab-burst-epoll-spin-vs-uring-spin | epoll-spin | uring-spin | 11499950 | -0.65 | 8.20 | 25.52 | -0.03 | -1.76 | 32.60 | 3679619 | 54390 | 38.6 | 12.3 | yes |
| lab-burst-recv-busy-vs-uring-spin | recv-busy | uring-spin | 11500000 | 9.16 | 27.21 | 261.58 | 6.28 | 3.59 | 24.46 | 1237135 | 46476 | 77.7 | 13.4 | no |
| lab-burst-recv-busy-vs-uring-sqpoll | recv-busy | uring-sqpoll | 11500012 | 5.62 | 23.70 | 59.78 | 3.36 | 1.56 | 24.83 | 1184007 | 0 | 78.7 | 0.0 | no |
| lab-burst-uring-spin-vs-epoll-spin | uring-spin | epoll-spin | 11499967 | -0.10 | 9.19 | 24.43 | 0.03 | 1.32 | 33.24 | 61332 | 3540137 | 14.6 | 38.7 | yes |
| lab-burst-uring-spin-vs-recv-busy | uring-spin | recv-busy | 11500019 | -8.60 | 3.24 | 19.13 | -5.76 | -2.30 | 13.97 | 45362 | 1109507 | 11.9 | 79.8 | no |
| lab-burst-uring-sqpoll-vs-recv-busy | uring-sqpoll | recv-busy | 11500000 | -3.70 | 15.49 | 43.77 | -2.90 | -0.89 | 18.64 | 0 | 1217805 | 0.0 | 78.3 | no |
| lab-replay1x-aa-epoll-spin | epoll-spin | epoll-spin | 190068 | -0.12 | 8.09 | 30.45 | -0.03 | 0.05 | 16.10 | 4148130 | 4109450 | 30.4 | 29.9 | no |
| lab-replay1x-aa-recv-busy | recv-busy | recv-busy | 189997 | 0.13 | 8.60 | 41.79 | -1.46 | 0.10 | 12.11 | 1208451 | 1208437 | 79.4 | 79.1 | no |
| lab-replay1x-epoll-spin-vs-uring-spin | epoll-spin | uring-spin | 190078 | 0.28 | 8.37 | 24.75 | -0.03 | -1.35 | 16.02 | 4134654 | 808 | 30.1 | 0.2 | yes |
| lab-replay1x-recv-busy-vs-uring-spin | recv-busy | uring-spin | 190069 | 10.76 | 32.77 | 466.54 | 7.98 | 2.63 | 30.23 | 1229773 | 733 | 78.9 | 0.2 | no |
| lab-replay1x-recv-busy-vs-uring-sqpoll | recv-busy | uring-sqpoll | 190091 | 0.86 | 7.34 | 27.32 | -0.32 | 0.92 | 7.65 | 1189318 | 0 | 79.6 | 0.0 | no |
| lab-replay1x-uring-spin-vs-epoll-spin | uring-spin | epoll-spin | 190083 | -0.17 | 12.66 | 29.54 | -0.10 | -0.20 | 17.59 | 700 | 4317183 | 0.2 | 31.2 | no |
| lab-replay1x-uring-spin-vs-recv-busy | uring-spin | recv-busy | 190068 | -10.90 | 4.59 | 22.44 | -7.94 | -3.38 | 5.82 | 719 | 1234120 | 0.2 | 78.7 | no |
| lab-replay1x-uring-sqpoll-vs-recv-busy | uring-sqpoll | recv-busy | 190070 | -0.88 | 6.56 | 34.79 | -0.21 | -0.87 | 7.54 | 0 | 1211522 | 0.0 | 79.0 | no |
| live-aa-recv-busy | recv-busy | recv-busy | 444558 | 42.07 | 1337.68 | 10424.55 | 42.19 | -0.26 | 3.07 | 1227332 | 1225968 | 81.3 | 80.0 | no |
| live-epoll-spin-vs-uring-spin | epoll-spin | uring-spin | 465986 | -399.90 | 392.38 | 5665.51 | -401.06 | 0.62 | 24.27 | 4137900 | 1098 | 29.9 | 0.2 | no |
| live-recv-busy-vs-uring-spin | recv-busy | uring-spin | 246447 | 108.38 | 274.07 | 977.57 | 105.78 | 1.39 | 30.03 | 1243527 | 648 | 79.8 | 0.2 | yes |
| live-recv-busy-vs-uring-sqpoll | recv-busy | uring-sqpoll | 198120 | -642.59 | 260.95 | 7724.96 | -641.97 | 0.41 | 11.57 | 1068218 | 0 | 82.9 | 0.0 | yes |
| live-uring-spin-vs-epoll-spin | uring-spin | epoll-spin | 324030 | -355.06 | -45.06 | 367.46 | -355.38 | 0.02 | 34.61 | 772 | 4115110 | 0.2 | 29.8 | yes |
| live-uring-spin-vs-recv-busy | uring-spin | recv-busy | 145901 | 619.37 | 755.85 | 1022.67 | 622.06 | -1.74 | -0.27 | 428 | 1250794 | 0.1 | 79.7 | yes |
| live-uring-sqpoll-vs-recv-busy | uring-sqpoll | recv-busy | 219745 | 173.13 | 397.27 | 1311.19 | 172.58 | -1.02 | 14.64 | 0 | 976330 | 0.0 | 83.9 | yes |

## lab-burst-aa-epoll-spin: lab (A/A control), 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-burst-aa-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 12004000 | 6711933 | 1.79 | 873681725 | 72.78 | 3640341 | 3612374 | 0.0 | 36.4 | false | 0 | 0 | 6106.3 | 2799369 / 27879 / 925 | 0 | 0 |
| epoll-spin | 12004003 | 6770031 | 1.77 | 867846383 | 72.30 | 3616027 | 3587818 | 0.0 | 37.6 | false | 0 | 0 | 3883.8 | 3035374 / 71066 / 1137 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8210 | 0 |
| epoll-spin | 0 | 8216 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 2992192798 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 2992193470 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin #0 (3044042 of 11499950 in 1-msg reads) | 3044042 | 9.69 | 8.17 | 16.07 | 25.57 | 60.56 | 6661.05 |
| epoll-spin #1 (3123708 of 11499953 in 1-msg reads) | 3123708 | 11.62 | 10.37 | 20.90 | 27.20 | 72.74 | 6133.42 |

### Paired by message, relative to epoll-spin (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin #1 Δuser (write-order corrected)** | 11499950 | -1.75 | 0.81 | 17.12 | 35.38 | 203.59 | 7541.82 |
| epoll-spin #1 Δuser, even seq | 5749975 | -3.15 | -2.84 | 15.43 | 31.10 | 200.49 | 7538.17 |
| epoll-spin #1 Δuser, odd seq | 5749975 | -0.34 | 4.47 | 18.42 | 39.65 | 206.74 | 7538.18 |
| epoll-spin #1 Δrx | 11499950 | -4.00 | -3.04 | 14.17 | 36.36 | 201.01 | 7507.51 |
| epoll-spin #1 Δstack (1-msg reads) | 1383652 | 1.39 | 1.02 | 8.19 | 18.16 | 55.18 | 3153.87 |

epoll-spin #1: 11499950 paired messages; decoded before epoll-spin on 50.4 % (uncorrected).

## lab-burst-aa-recv-busy: lab (A/A control), 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=5381313 clocksource=tsc

### Loop counters (lab-burst-aa-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 12003660 | 7451703 | 1.61 | 286369792 | 23.86 | 1193207 | 1193207 | 97.4 | 78.8 | true | 0 | 0 | 7497.0 | 2382082 / 76927 / 515 | 0 | 0 |
| recv-busy | 12003664 | 7463346 | 1.61 | 285512960 | 23.79 | 1189637 | 1189637 | 97.4 | 78.0 | true | 0 | 0 | 4780.0 | 2248005 / 59995 / 518 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8211 | 1 |
| recv-busy | 0 | 8214 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 2992108799 | 0 | 0 | 0 | 0.000 |
| recv-busy | 2992109811 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| recv-busy #0 (3735714 of 11500004 in 1-msg reads) | 3735714 | 7.47 | 5.18 | 14.63 | 24.58 | 42.71 | 5444.96 |
| recv-busy #1 (3762445 of 11500008 in 1-msg reads) | 3762445 | 6.81 | 4.51 | 12.38 | 25.40 | 42.76 | 5017.80 |

### Paired by message, relative to recv-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **recv-busy #1 Δuser (write-order corrected)** | 11500004 | -0.80 | -0.34 | 12.89 | 41.69 | 160.43 | 5579.61 |
| recv-busy #1 Δuser, even seq | 5750002 | -2.79 | -3.51 | 10.55 | 39.59 | 157.43 | 5572.95 |
| recv-busy #1 Δuser, odd seq | 5750002 | 1.19 | 2.83 | 14.68 | 43.68 | 163.08 | 5582.78 |
| recv-busy #1 Δrx | 11500004 | -0.08 | 0.26 | 13.08 | 30.86 | 158.59 | 7389.82 |
| recv-busy #1 Δstack (1-msg reads) | 1678628 | -0.56 | -0.13 | 4.93 | 20.46 | 34.59 | 4998.74 |

recv-busy #1: 11500004 paired messages; decoded before recv-busy on 49.5 % (uncorrected).

## lab-burst-epoll-spin-vs-uring-spin: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-burst-epoll-spin-vs-uring-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 12004100 | 7116736 | 1.69 | 883108672 | 73.57 | 3679619 | 3649966 | 0.0 | 38.6 | false | 0 | 0 | 3423.4 | 2290930 / 49899 / 932 | 0 | 0 |
| uring-spin | 12004100 | 7232169 | 1.66 | 13053620 | 1.09 | 54390 | 60764197 | 0.0 | 12.3 | false | 0 | 0 | 2594.3 | 1414154 / 30140 / 802 | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8216 | 0 |
| uring-spin | 0 | 8216 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 2992215932 | 0 | 0 | 0 | 0.000 |
| uring-spin | 2992215932 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin (3214973 of 11499950 in 1-msg reads) | 3214973 | 21.11 | 18.55 | 36.75 | 60.54 | 116.66 | 6060.49 |
| uring-spin (3329251 of 11499950 in 1-msg reads) | 3329251 | 21.12 | 18.76 | 36.61 | 60.42 | 114.02 | 6048.93 |

### Paired by message, relative to epoll-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 11499950 | -0.66 | -0.65 | 8.20 | 25.52 | 43.35 | 5827.64 |
| uring-spin Δuser, even seq | 5749975 | 0.76 | 0.87 | 11.15 | 28.11 | 46.52 | 5829.16 |
| uring-spin Δuser, odd seq | 5749975 | -2.08 | -2.17 | 5.94 | 22.38 | 40.38 | 5333.56 |
| uring-spin Δrx | 11499950 | -0.17 | -0.03 | 0.13 | 45.39 | 72.55 | 5816.67 |
| uring-spin Δstack (1-msg reads) | 1759672 | -0.51 | -1.76 | 7.05 | 32.60 | 58.87 | 5229.47 |

uring-spin: 11499950 paired messages; decoded before epoll-spin on 55.2 % (uncorrected).

## lab-burst-recv-busy-vs-uring-spin: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2384032 clocksource=tsc

### Loop counters (lab-burst-recv-busy-vs-uring-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 12004050 | 7183012 | 1.67 | 296912320 | 24.73 | 1237135 | 1237135 | 97.6 | 77.7 | true | 0 | 1 | 5744.3 | 2088134 / 24689 / 411 | 0 | 0 |
| uring-spin | 12004050 | 6725959 | 1.78 | 11154145 | 0.93 | 46476 | 60281242 | 0.0 | 13.4 | false | 0 | 0 | 4333.6 | 2674653 / 87199 / 724 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8213 | 0 |
| uring-spin | 0 | 8210 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 2992204341 | 0 | 0 | 0 | 0.000 |
| uring-spin | 2992204341 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| recv-busy (3368912 of 11500000 in 1-msg reads) | 3368912 | 6.54 | 4.83 | 10.74 | 23.18 | 34.82 | 6015.90 |
| uring-spin (2939143 of 11500000 in 1-msg reads) | 2939143 | 11.64 | 10.22 | 21.28 | 28.42 | 61.73 | 5337.23 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 11500000 | 21.44 | 9.16 | 27.21 | 261.58 | 2619.53 | 9241.03 |
| uring-spin Δuser, even seq | 5750000 | 19.63 | 8.09 | 25.38 | 257.61 | 2615.52 | 9234.97 |
| uring-spin Δuser, odd seq | 5750000 | 23.26 | 10.24 | 29.12 | 265.55 | 2625.61 | 9242.10 |
| uring-spin Δrx | 11500000 | 16.14 | 6.28 | 20.06 | 237.61 | 2575.46 | 9192.38 |
| uring-spin Δstack (1-msg reads) | 1336439 | 4.91 | 3.59 | 12.69 | 24.46 | 57.91 | 3484.45 |

uring-spin: 11500000 paired messages; decoded before recv-busy on 25.5 % (uncorrected).

## lab-burst-recv-busy-vs-uring-sqpoll: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=4087450 clocksource=tsc

### Loop counters (lab-burst-recv-busy-vs-uring-sqpoll)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 12004015 | 6605120 | 1.82 | 284161792 | 23.67 | 1184007 | 1184007 | 97.7 | 78.7 | true | 0 | 0 | 7317.5 | 2089670 / 80373 / 531 | 0 | 0 |
| uring-sqpoll | 12004012 | 5728013 | 2.10 | 0 | 0.00 | 0 | 62926464 | 0.0 | 0.0 | true | 0 | 1 | 122.4 | 1558 / 107 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8210 | 0 |
| uring-sqpoll | 0 | 8216 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 2992196158 | 0 | 0 | 0 | 0.000 |
| uring-sqpoll | 2992195486 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| recv-busy (2821228 of 11500015 in 1-msg reads) | 2821228 | 7.14 | 4.98 | 13.37 | 28.35 | 50.42 | 3965.24 |
| uring-sqpoll (2063345 of 11500012 in 1-msg reads) | 2063345 | 11.17 | 10.03 | 21.43 | 30.54 | 58.85 | 8946.11 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser (write-order corrected)** | 11500012 | 5.79 | 5.62 | 23.70 | 59.78 | 252.50 | 9270.51 |
| uring-sqpoll Δuser, even seq | 5750006 | 4.28 | 3.70 | 22.54 | 58.05 | 249.54 | 9268.33 |
| uring-sqpoll Δuser, odd seq | 5750006 | 7.30 | 7.53 | 24.82 | 61.50 | 254.87 | 9272.43 |
| uring-sqpoll Δrx | 11500012 | 2.41 | 3.36 | 18.05 | 47.21 | 225.64 | 9263.09 |
| uring-sqpoll Δstack (1-msg reads) | 969956 | 3.24 | 1.56 | 12.85 | 24.83 | 46.28 | 8943.30 |

uring-sqpoll: 11500012 paired messages; decoded before recv-busy on 36.5 % (uncorrected).

## lab-burst-uring-spin-vs-epoll-spin: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-burst-uring-spin-vs-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 12004800 | 8029464 | 1.50 | 14719616 | 1.23 | 61332 | 59034621 | 0.0 | 14.6 | false | 0 | 0 | 5245.9 | 1776832 / 30801 / 970 | 0 | 0 |
| epoll-spin | 12004800 | 8031611 | 1.49 | 849632763 | 70.77 | 3540137 | 3506671 | 0.0 | 38.7 | false | 0 | 0 | 3315.8 | 2328816 / 48213 / 1050 | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8214 | 1 |
| epoll-spin | 0 | 8214 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 2992391279 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 2992391279 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-spin (4310271 of 11499967 in 1-msg reads) | 4310271 | 20.49 | 17.58 | 37.20 | 59.24 | 111.46 | 6217.47 |
| epoll-spin (4364426 of 11499967 in 1-msg reads) | 4364426 | 20.34 | 17.10 | 36.84 | 58.95 | 111.17 | 6638.58 |

### Paired by message, relative to uring-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser (write-order corrected)** | 11499967 | 0.08 | -0.10 | 9.19 | 24.43 | 46.52 | 6210.62 |
| epoll-spin Δuser, even seq | 5749984 | -1.79 | -2.60 | 6.46 | 20.30 | 37.85 | 5092.49 |
| epoll-spin Δuser, odd seq | 5749983 | 1.95 | 2.40 | 12.11 | 28.94 | 51.73 | 6213.12 |
| epoll-spin Δrx | 11499967 | 0.01 | 0.03 | 0.20 | 43.91 | 69.52 | 6666.64 |
| epoll-spin Δstack (1-msg reads) | 2229554 | 0.22 | 1.32 | 7.09 | 33.24 | 58.09 | 6200.84 |

epoll-spin: 11499967 paired messages; decoded before uring-spin on 48.4 % (uncorrected).

## lab-burst-uring-spin-vs-recv-busy: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=4704741 clocksource=tsc

### Loop counters (lab-burst-uring-spin-vs-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 12004100 | 6960390 | 1.72 | 10886971 | 0.91 | 45362 | 61362337 | 0.0 | 11.9 | false | 0 | 0 | 4377.6 | 2158903 / 17425 / 619 | 0 | 0 |
| recv-busy | 12004100 | 8019316 | 1.50 | 266281600 | 22.18 | 1109507 | 1109507 | 97.0 | 79.8 | true | 0 | 1 | 2549.7 | 1329458 / 34739 / 340 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8211 | 1 |
| recv-busy | 0 | 8216 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 2992215932 | 0 | 0 | 0 | 0.000 |
| recv-busy | 2992215932 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-spin (3089502 of 11500019 in 1-msg reads) | 3089502 | 7.46 | 7.23 | 9.69 | 23.03 | 45.10 | 6353.56 |
| recv-busy (4325355 of 11500020 in 1-msg reads) | 4325355 | 4.67 | 3.26 | 8.77 | 21.40 | 29.30 | 9509.82 |

### Paired by message, relative to uring-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **recv-busy Δuser (write-order corrected)** | 11500019 | -21.97 | -8.60 | 3.24 | 19.13 | 34.12 | 3248.67 |
| recv-busy Δuser, even seq | 5750010 | -23.79 | -10.74 | 1.48 | 16.90 | 31.07 | 3246.53 |
| recv-busy Δuser, odd seq | 5750009 | -20.15 | -6.45 | 4.74 | 21.34 | 37.36 | 3246.32 |
| recv-busy Δrx | 11500019 | -18.71 | -5.76 | 3.75 | 12.99 | 29.24 | 2267.49 |
| recv-busy Δstack (1-msg reads) | 1576642 | -2.71 | -2.30 | 2.20 | 13.97 | 21.41 | 3237.09 |

recv-busy: 11500019 paired messages; decoded before uring-spin on 81.5 % (uncorrected).

## lab-burst-uring-sqpoll-vs-recv-busy: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3390753 clocksource=tsc

### Loop counters (lab-burst-uring-sqpoll-vs-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 12004150 | 6020978 | 1.99 | 0 | 0.00 | 0 | 58866442 | 0.0 | 0.0 | true | 0 | 1 | 878.6 | 3326 / 278 / 9 | 0 | 0 |
| recv-busy | 12004150 | 6609176 | 1.82 | 292273280 | 24.35 | 1217805 | 1217805 | 97.7 | 78.3 | true | 0 | 2 | 3622.7 | 2386517 / 83084 / 619 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8214 | 6 |
| recv-busy | 0 | 8212 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 2992228276 | 0 | 0 | 0 | 0.000 |
| recv-busy | 2992228276 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-sqpoll (2369390 of 11500000 in 1-msg reads) | 2369390 | 10.92 | 9.35 | 22.02 | 30.05 | 56.20 | 9316.59 |
| recv-busy (2871026 of 11500000 in 1-msg reads) | 2871026 | 8.57 | 7.90 | 16.14 | 26.09 | 50.95 | 6126.56 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **recv-busy Δuser (write-order corrected)** | 11500000 | -3.64 | -3.70 | 15.49 | 43.77 | 211.52 | 7618.91 |
| recv-busy Δuser, even seq | 5750000 | -1.97 | -1.71 | 17.59 | 45.45 | 217.64 | 7617.04 |
| recv-busy Δuser, odd seq | 5750000 | -5.32 | -5.68 | 13.39 | 42.08 | 206.22 | 7616.93 |
| recv-busy Δrx | 11500000 | -1.37 | -2.90 | 14.76 | 36.97 | 196.03 | 5929.88 |
| recv-busy Δstack (1-msg reads) | 1021836 | -1.86 | -0.89 | 5.80 | 18.64 | 37.96 | 2919.23 |

recv-busy: 11500000 paired messages; decoded before uring-sqpoll on 60.0 % (uncorrected).

## lab-replay1x-aa-epoll-spin: lab (A/A control), 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-replay1x-aa-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 196131 | 142159 | 1.38 | 1493326671 | 7613.92 | 4148130 | 4147735 | 0.0 | 30.4 | false | 0 | 0 | 814.5 | 90826 / 6691 / 115 | 0 | 0 |
| epoll-spin | 196131 | 141121 | 1.39 | 1479401857 | 7542.93 | 4109450 | 4109058 | 0.0 | 29.9 | false | 0 | 0 | 537.1 | 59546 / 5638 / 107 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8215 | 1 |
| epoll-spin | 0 | 8216 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 48876600 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 48876600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin #0 (101020 of 190068 in 1-msg reads) | 101020 | 12.99 | 8.26 | 25.64 | 56.00 | 165.98 | 2496.60 |
| epoll-spin #1 (100471 of 190068 in 1-msg reads) | 100471 | 12.49 | 7.61 | 25.10 | 55.24 | 153.81 | 9087.54 |

### Paired by message, relative to epoll-spin (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin #1 Δuser (write-order corrected)** | 190068 | -1.21 | -0.12 | 8.09 | 30.45 | 70.92 | 9080.73 |
| epoll-spin #1 Δuser, even seq | 95034 | -3.39 | -0.33 | 4.54 | 23.95 | 55.39 | 9080.53 |
| epoll-spin #1 Δuser, odd seq | 95034 | 0.96 | 0.08 | 13.30 | 33.70 | 84.22 | 9080.50 |
| epoll-spin #1 Δrx | 190068 | 0.25 | -0.03 | 9.75 | 29.67 | 115.24 | 9034.11 |
| epoll-spin #1 Δstack (1-msg reads) | 88267 | -0.17 | 0.05 | 3.80 | 16.10 | 53.92 | 9069.05 |

epoll-spin #1: 190068 paired messages; decoded before epoll-spin on 54.0 % (uncorrected).

## lab-replay1x-aa-recv-busy: lab (A/A control), 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=191473 clocksource=tsc

### Loop counters (lab-replay1x-aa-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 196060 | 148985 | 1.32 | 435042240 | 2218.92 | 1208451 | 1208451 | 100.0 | 79.4 | true | 0 | 0 | 1138.2 | 81158 / 1392 / 42 | 0 | 0 |
| recv-busy | 196060 | 149891 | 1.31 | 435037312 | 2218.90 | 1208437 | 1208437 | 100.0 | 79.1 | true | 0 | 0 | 272.7 | 62509 / 1339 / 24 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8215 | 2 |
| recv-busy | 0 | 8214 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 48858670 | 0 | 0 | 0 | 0.000 |
| recv-busy | 48858670 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| recv-busy #0 (108234 of 189997 in 1-msg reads) | 108234 | 5.72 | 3.93 | 10.64 | 22.21 | 38.53 | 810.55 |
| recv-busy #1 (109361 of 189997 in 1-msg reads) | 109361 | 5.71 | 4.05 | 10.29 | 21.97 | 42.85 | 168.62 |

### Paired by message, relative to recv-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **recv-busy #1 Δuser (write-order corrected)** | 189997 | -0.02 | 0.13 | 8.60 | 41.79 | 95.63 | 1371.74 |
| recv-busy #1 Δuser, even seq | 94999 | 1.43 | 2.25 | 9.55 | 43.01 | 97.42 | 1365.43 |
| recv-busy #1 Δuser, odd seq | 94998 | -1.47 | -1.99 | 7.20 | 40.47 | 94.08 | 1369.62 |
| recv-busy #1 Δrx | 189997 | 0.03 | -1.46 | 8.46 | 29.34 | 87.63 | 1358.12 |
| recv-busy #1 Δstack (1-msg reads) | 95605 | 0.10 | 0.10 | 2.37 | 12.11 | 36.77 | 161.80 |

recv-busy #1: 189997 paired messages; decoded before recv-busy on 50.7 % (uncorrected).

## lab-replay1x-epoll-spin-vs-uring-spin: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-replay1x-epoll-spin-vs-uring-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 196144 | 152070 | 1.29 | 1488475590 | 7588.69 | 4134654 | 4134232 | 0.0 | 30.1 | false | 0 | 0 | 4413.2 | 40969 / 1748 / 72 | 0 | 0 |
| uring-spin | 196141 | 151091 | 1.30 | 290879 | 1.48 | 808 | 71216364 | 0.0 | 0.2 | false | 0 | 0 | 4048.4 | 30513 / 1031 / 64 | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8210 | 0 |
| uring-spin | 0 | 8210 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 48879891 | 0 | 0 | 0 | 0.000 |
| uring-spin | 48879132 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin (112278 of 190081 in 1-msg reads) | 112278 | 14.03 | 8.55 | 26.78 | 57.78 | 180.26 | 4962.00 |
| uring-spin (110811 of 190078 in 1-msg reads) | 110811 | 13.83 | 8.43 | 26.77 | 56.25 | 184.79 | 4960.07 |

### Paired by message, relative to epoll-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 190078 | 0.24 | 0.28 | 8.37 | 24.75 | 53.41 | 1413.38 |
| uring-spin Δuser, even seq | 95039 | 2.07 | 2.60 | 8.28 | 28.47 | 60.49 | 1415.71 |
| uring-spin Δuser, odd seq | 95039 | -1.60 | -2.05 | 6.72 | 21.30 | 46.04 | 1167.78 |
| uring-spin Δrx | 190078 | -0.01 | -0.03 | 0.13 | 34.70 | 66.11 | 926.74 |
| uring-spin Δstack (1-msg reads) | 98463 | -0.10 | -1.35 | 3.71 | 16.02 | 46.92 | 1415.65 |

uring-spin: 190078 paired messages; decoded before epoll-spin on 51.4 % (uncorrected).

## lab-replay1x-recv-busy-vs-uring-spin: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=96846 clocksource=tsc

### Loop counters (lab-replay1x-recv-busy-vs-uring-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 196132 | 149180 | 1.31 | 442718272 | 2257.25 | 1229773 | 1229773 | 100.0 | 78.9 | true | 0 | 1 | 742.5 | 57884 / 1206 / 31 | 0 | 0 |
| uring-spin | 196132 | 143039 | 1.37 | 263854 | 1.35 | 733 | 71162317 | 0.0 | 0.2 | false | 0 | 0 | 342.9 | 48889 / 1184 / 49 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8212 | 2 |
| uring-spin | 0 | 8214 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 48876853 | 0 | 0 | 0 | 0.000 |
| uring-spin | 48876853 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| recv-busy (108147 of 190069 in 1-msg reads) | 108147 | 5.29 | 3.85 | 9.67 | 17.21 | 31.29 | 492.19 |
| uring-spin (102530 of 190069 in 1-msg reads) | 102530 | 10.76 | 7.45 | 20.59 | 39.42 | 134.61 | 1230.07 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 190069 | 29.38 | 10.76 | 32.77 | 466.54 | 2677.50 | 8179.77 |
| uring-spin Δuser, even seq | 95035 | 30.75 | 11.55 | 34.79 | 466.95 | 2575.56 | 8180.56 |
| uring-spin Δuser, odd seq | 95034 | 28.02 | 9.96 | 30.61 | 465.75 | 2702.91 | 7516.50 |
| uring-spin Δrx | 190069 | 23.55 | 7.98 | 22.07 | 429.34 | 2652.47 | 8142.79 |
| uring-spin Δstack (1-msg reads) | 90374 | 5.36 | 2.63 | 13.04 | 30.23 | 138.83 | 1226.55 |

uring-spin: 190069 paired messages; decoded before recv-busy on 8.5 % (uncorrected).

## lab-replay1x-recv-busy-vs-uring-sqpoll: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=193536 clocksource=tsc

### Loop counters (lab-replay1x-recv-busy-vs-uring-sqpoll)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 196156 | 154044 | 1.27 | 428154560 | 2182.72 | 1189318 | 1189318 | 100.0 | 79.6 | true | 0 | 0 | 1232.4 | 56913 / 749 / 33 | 0 | 0 |
| uring-sqpoll | 196154 | 154042 | 1.27 | 1 | 0.00 | 0 | 62240014 | 0.0 | 0.0 | true | 0 | 0 | 593.5 | 1896 / 92 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8210 | 2 |
| uring-sqpoll | 0 | 8216 | 6 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 48882927 | 0 | 0 | 0 | 0.000 |
| uring-sqpoll | 48882421 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| recv-busy (113880 of 190093 in 1-msg reads) | 113880 | 5.12 | 3.76 | 8.92 | 16.75 | 28.42 | 385.81 |
| uring-sqpoll (113669 of 190091 in 1-msg reads) | 113669 | 6.08 | 4.72 | 10.11 | 17.53 | 32.78 | 421.19 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser (write-order corrected)** | 190091 | 0.91 | 0.86 | 7.34 | 27.32 | 135.30 | 1377.02 |
| uring-sqpoll Δuser, even seq | 95046 | 2.63 | 2.57 | 9.10 | 29.72 | 140.06 | 1378.72 |
| uring-sqpoll Δuser, odd seq | 95045 | -0.81 | -0.84 | 5.58 | 24.73 | 132.03 | 800.51 |
| uring-sqpoll Δrx | 190091 | -0.18 | -0.32 | 5.87 | 19.74 | 123.54 | 1105.14 |
| uring-sqpoll Δstack (1-msg reads) | 96656 | 0.94 | 0.92 | 2.82 | 7.65 | 22.09 | 88.85 |

uring-sqpoll: 190091 paired messages; decoded before recv-busy on 45.2 % (uncorrected).

## lab-replay1x-uring-spin-vs-epoll-spin: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-replay1x-uring-spin-vs-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 196146 | 143203 | 1.37 | 251995 | 1.28 | 700 | 71139663 | 0.0 | 0.2 | false | 0 | 0 | 3128.0 | 48946 / 1458 / 80 | 0 | 0 |
| epoll-spin | 196146 | 144593 | 1.36 | 1554185937 | 7923.62 | 4317183 | 4316782 | 0.0 | 31.2 | false | 0 | 0 | 3124.5 | 57060 / 1900 / 60 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8212 | 1 |
| epoll-spin | 0 | 8216 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 48880397 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 48880397 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-spin (102330 of 190083 in 1-msg reads) | 102330 | 10.14 | 6.66 | 19.19 | 42.41 | 145.29 | 4163.26 |
| epoll-spin (104067 of 190083 in 1-msg reads) | 104067 | 10.01 | 6.77 | 19.35 | 42.13 | 141.62 | 4160.35 |

### Paired by message, relative to uring-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser (write-order corrected)** | 190083 | 0.18 | -0.17 | 12.66 | 29.54 | 68.13 | 1708.83 |
| epoll-spin Δuser, even seq | 95042 | -2.67 | -1.37 | 6.94 | 25.87 | 58.94 | 742.33 |
| epoll-spin Δuser, odd seq | 95041 | 3.03 | 1.02 | 16.84 | 32.78 | 72.12 | 1710.02 |
| epoll-spin Δrx | 190083 | -0.10 | -0.10 | 9.80 | 27.67 | 80.16 | 3389.17 |
| epoll-spin Δstack (1-msg reads) | 90427 | -0.31 | -0.20 | 4.53 | 17.59 | 51.30 | 1631.98 |

epoll-spin: 190083 paired messages; decoded before uring-spin on 50.3 % (uncorrected).

## lab-replay1x-uring-spin-vs-recv-busy: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=92687 clocksource=tsc

### Loop counters (lab-replay1x-uring-spin-vs-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 196131 | 142844 | 1.37 | 258909 | 1.32 | 719 | 71089996 | 0.0 | 0.2 | false | 0 | 0 | 1334.0 | 67604 / 4713 / 92 | 0 | 0 |
| recv-busy | 196131 | 147958 | 1.33 | 444283072 | 2265.24 | 1234120 | 1234120 | 100.0 | 78.7 | true | 0 | 0 | 963.0 | 62889 / 952 / 13 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8209 | 0 |
| recv-busy | 0 | 8216 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 48876600 | 0 | 0 | 0 | 0.000 |
| recv-busy | 48876600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-spin (102134 of 190068 in 1-msg reads) | 102134 | 12.97 | 8.42 | 25.97 | 50.16 | 164.98 | 4330.92 |
| recv-busy (106336 of 190068 in 1-msg reads) | 106336 | 5.88 | 4.16 | 11.18 | 21.86 | 34.14 | 382.25 |

### Paired by message, relative to uring-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **recv-busy Δuser (write-order corrected)** | 190068 | -29.21 | -10.90 | 4.59 | 22.44 | 48.67 | 1556.09 |
| recv-busy Δuser, even seq | 95034 | -27.29 | -9.18 | 7.11 | 23.19 | 50.81 | 1554.37 |
| recv-busy Δuser, odd seq | 95034 | -31.13 | -12.63 | -0.95 | 21.54 | 45.84 | 1554.37 |
| recv-busy Δrx | 190068 | -22.79 | -7.94 | 7.30 | 24.07 | 56.49 | 1538.95 |
| recv-busy Δstack (1-msg reads) | 88634 | -7.40 | -3.38 | -0.10 | 5.82 | 16.41 | 211.60 |

recv-busy: 190068 paired messages; decoded before uring-spin on 88.1 % (uncorrected).

## lab-replay1x-uring-sqpoll-vs-recv-busy: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=186900 clocksource=tsc

### Loop counters (lab-replay1x-uring-sqpoll-vs-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 196133 | 147245 | 1.33 | 1 | 0.00 | 0 | 62517809 | 0.0 | 0.0 | true | 0 | 0 | 2284.1 | 4300 / 338 / 17 | 0 | 0 |
| recv-busy | 196133 | 146941 | 1.33 | 436147904 | 2223.74 | 1211522 | 1211522 | 100.0 | 79.0 | true | 0 | 0 | 2248.6 | 44104 / 1187 / 29 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8211 | 6 |
| recv-busy | 0 | 8215 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 48877106 | 0 | 0 | 0 | 0.000 |
| recv-busy | 48877106 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-sqpoll (106771 of 190070 in 1-msg reads) | 106771 | 5.78 | 4.49 | 9.13 | 23.59 | 40.21 | 1158.35 |
| recv-busy (106549 of 190070 in 1-msg reads) | 106549 | 4.86 | 3.62 | 7.96 | 22.77 | 40.05 | 780.21 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **recv-busy Δuser (write-order corrected)** | 190070 | -1.12 | -0.88 | 6.56 | 34.79 | 85.24 | 2158.23 |
| recv-busy Δuser, even seq | 95035 | -3.19 | -3.08 | 4.33 | 32.35 | 82.60 | 2156.03 |
| recv-busy Δuser, odd seq | 95035 | 0.95 | 1.33 | 8.80 | 37.24 | 88.39 | 2148.05 |
| recv-busy Δrx | 190070 | -0.05 | -0.21 | 7.95 | 28.07 | 78.11 | 2085.11 |
| recv-busy Δstack (1-msg reads) | 92299 | -0.87 | -0.87 | -0.02 | 7.54 | 29.50 | 776.17 |

recv-busy: 190070 paired messages; decoded before uring-sqpoll on 52.0 % (uncorrected).

## live-aa-recv-busy: live-feed (A/A control), 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=260812 clocksource=tsc

### Loop counters (live-aa-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 456330 | 136533 | 3.34 | 368199571 | 806.87 | 1227332 | 409111 | 100.0 | 81.3 | true | 0 | 0 | 462.6 | 163511 / 1723 / 9 | 0 | 0 |
| recv-busy | 456339 | 138117 | 3.30 | 367790417 | 805.96 | 1225968 | 408656 | 100.0 | 80.0 | true | 0 | 0 | 862.7 | 166914 / 1813 / 17 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8216 | 2 |
| recv-busy | 1 | 8214 | 2 |
| recv-busy | 2 | 8214 | 2 |
| recv-busy | 0 | 8215 | 4 |
| recv-busy | 1 | 8213 | 4 |
| recv-busy | 2 | 8212 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 95737607 | 4528643 | 0 | 0 | 4.730 |
| recv-busy | 95800572 | 4078877 | 0 | 0 | 4.258 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| recv-busy #0 (70154 of 444558 in 1-msg reads) | 70154 | 4.93 | 4.56 | 6.92 | 11.53 | 15.89 | 57.18 |
| recv-busy #1 (71582 of 444558 in 1-msg reads) | 71582 | 4.47 | 3.88 | 6.34 | 10.86 | 15.62 | 232.57 |

### upstream feed event time → kernel rx (`rx_sw − E`)

Includes the upstream feed's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| instrument-a@trade recv-busy #0 | 30495 | 25251.43 | 24969.44 | 25474.33 | 32083.84 | 57776.60 | 57792.60 |
| instrument-a@trade recv-busy #1 | 30495 | 25427.27 | 25099.79 | 25542.25 | 34228.43 | 60290.31 | 60307.31 |
| instrument-b@trade recv-busy #0 | 19288 | 25207.73 | 24898.58 | 25412.69 | 43081.87 | 46156.97 | 46161.97 |
| instrument-b@trade recv-busy #1 | 19288 | 25437.63 | 25103.32 | 25572.42 | 45410.61 | 47454.01 | 49404.98 |
| instrument-c@trade recv-busy #0 | 3885 | 24891.89 | 24821.68 | 25314.96 | 29049.18 | 30811.72 | 30814.72 |
| instrument-c@trade recv-busy #1 | 3885 | 25091.11 | 25016.48 | 25451.94 | 28951.98 | 32852.00 | 32853.00 |
| instrument-a@bookTicker recv-busy #0 | 232814 | 31267.77 | 25558.48 | 45494.13 | 97487.85 | 161618.59 | 169981.05 |
| instrument-a@bookTicker recv-busy #1 | 232814 | 31281.05 | 25573.01 | 46786.63 | 77924.82 | 122093.94 | 158821.80 |
| instrument-b@bookTicker recv-busy #0 | 148493 | 32212.94 | 25553.40 | 47606.18 | 91965.99 | 160474.00 | 168981.05 |
| instrument-b@bookTicker recv-busy #1 | 148493 | 32259.28 | 25521.84 | 49380.51 | 77824.02 | 144971.91 | 158821.80 |
| instrument-a@aggTrade recv-busy #0 | 9583 | 26373.88 | 25147.38 | 25948.23 | 62883.58 | 63392.64 | 64074.10 |
| instrument-a@aggTrade recv-busy #1 | 9583 | 26247.34 | 25063.21 | 25692.58 | 63140.69 | 64140.69 | 64140.69 |

### Paired by message, relative to recv-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own upstream feed connection, so Δuser also contains the upstream feed's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **recv-busy #1 Δuser** | 444558 | 42.97 | 42.07 | 1337.68 | 10424.55 | 21220.64 | 28528.57 |
| recv-busy #1 Δrx | 444558 | 43.48 | 42.19 | 1342.18 | 10428.77 | 21225.61 | 28542.00 |
| recv-busy #1 Δstack (1-msg reads) | 68580 | -0.45 | -0.26 | 0.63 | 3.07 | 5.30 | 129.09 |

recv-busy #1: 444558 paired messages; decoded before recv-busy on 39.1 % (uncorrected).

## live-epoll-spin-vs-uring-spin: live-feed, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (live-epoll-spin-vs-uring-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 483900 | 159673 | 3.03 | 1241370060 | 2565.34 | 4137900 | 4137368 | 0.0 | 29.9 | false | 0 | 0 | 1269.6 | 44636 / 1508 / 88 | 0 | 0 |
| uring-spin | 485119 | 165847 | 2.93 | 329461 | 0.68 | 1098 | 63985845 | 0.0 | 0.2 | false | 0 | 0 | 683.7 | 29597 / 892 / 85 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8210 | 0 |
| epoll-spin | 1 | 8215 | 1 |
| epoll-spin | 2 | 8216 | 0 |
| uring-spin | 0 | 8211 | 1 |
| uring-spin | 1 | 8212 | 1 |
| uring-spin | 2 | 8212 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 102442035 | 275767 | 0 | 0 | 0.269 |
| uring-spin | 102815500 | 566856 | 0 | 0 | 0.551 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin (85823 of 465987 in 1-msg reads) | 85823 | 6.54 | 4.85 | 9.13 | 28.05 | 132.52 | 1384.17 |
| uring-spin (89347 of 465986 in 1-msg reads) | 89347 | 8.11 | 5.39 | 15.70 | 34.78 | 135.46 | 1915.81 |

### upstream feed event time → kernel rx (`rx_sw − E`)

Includes the upstream feed's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| instrument-a@trade epoll-spin | 18388 | 24668.07 | 24615.97 | 25072.77 | 25925.79 | 47181.32 | 61517.32 |
| instrument-a@trade uring-spin | 18388 | 24717.03 | 24682.40 | 25125.93 | 25590.15 | 47133.49 | 61469.49 |
| instrument-b@trade epoll-spin | 15081 | 24654.43 | 24562.26 | 25019.66 | 25662.39 | 61282.32 | 63439.88 |
| instrument-b@trade uring-spin | 15081 | 24728.70 | 24614.89 | 25112.11 | 25822.68 | 61234.49 | 63363.76 |
| instrument-c@trade epoll-spin | 4190 | 24642.35 | 24516.40 | 24901.15 | 25739.44 | 62329.32 | 63313.88 |
| instrument-c@trade uring-spin | 4190 | 24713.19 | 24581.83 | 25048.94 | 25619.92 | 62281.49 | 63237.76 |
| instrument-a@bookTicker epoll-spin | 243896 | 25233.09 | 25147.15 | 25613.58 | 27968.54 | 39000.50 | 51384.01 |
| instrument-a@bookTicker uring-spin | 243895 | 25136.36 | 24786.22 | 26173.31 | 31962.54 | 43936.77 | 51036.21 |
| instrument-b@bookTicker epoll-spin | 176065 | 25150.19 | 25065.66 | 25513.43 | 27731.59 | 40699.95 | 51202.64 |
| instrument-b@bookTicker uring-spin | 176065 | 25067.74 | 24684.06 | 26026.45 | 32164.57 | 47546.36 | 52278.58 |
| instrument-a@aggTrade epoll-spin | 8367 | 26709.25 | 25118.43 | 31382.41 | 44283.13 | 51764.27 | 174114.62 |
| instrument-a@aggTrade uring-spin | 8367 | 26379.22 | 24686.64 | 30766.38 | 46291.42 | 57145.86 | 173738.82 |

### Paired by message, relative to epoll-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own upstream feed connection, so Δuser also contains the upstream feed's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-spin Δuser** | 465986 | -82.59 | -399.90 | 392.38 | 5665.51 | 14711.83 | 70376.21 |
| uring-spin Δrx | 465986 | -82.73 | -401.06 | 394.12 | 5667.69 | 14725.51 | 70376.30 |
| uring-spin Δstack (1-msg reads) | 84053 | 1.69 | 0.62 | 7.23 | 24.27 | 113.89 | 1894.50 |

uring-spin: 465986 paired messages; decoded before epoll-spin on 80.2 % (uncorrected).

## live-recv-busy-vs-uring-spin: live-feed, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=101383 clocksource=tsc

### Loop counters (live-recv-busy-vs-uring-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 262010 | 100638 | 2.60 | 373058130 | 1423.83 | 1243527 | 414509 | 100.0 | 79.8 | true | 0 | 0 | 793.4 | 138699 / 2898 / 23 | 0 | 0 |
| uring-spin | 262007 | 97943 | 2.68 | 194489 | 0.74 | 648 | 64002793 | 0.0 | 0.2 | false | 0 | 0 | 168.0 | 26339 / 724 / 54 | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8212 | 2 |
| recv-busy | 1 | 8216 | 2 |
| recv-busy | 2 | 8213 | 2 |
| uring-spin | 0 | 8212 | 2 |
| uring-spin | 1 | 8210 | 0 |
| uring-spin | 2 | 8215 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 55645083 | 124634 | 0 | 0 | 0.224 |
| uring-spin | 55624638 | 166873 | 0 | 0 | 0.300 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| recv-busy (54586 of 246448 in 1-msg reads) | 54586 | 6.29 | 4.86 | 11.46 | 17.74 | 19.54 | 1613.04 |
| uring-spin (53772 of 246447 in 1-msg reads) | 53772 | 10.60 | 6.66 | 21.11 | 40.80 | 132.67 | 1572.82 |

### upstream feed event time → kernel rx (`rx_sw − E`)

Includes the upstream feed's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| instrument-a@trade recv-busy | 8879 | 24944.49 | 24976.31 | 25324.21 | 25579.74 | 25777.02 | 32103.63 |
| instrument-a@trade uring-spin | 8879 | 24696.95 | 24719.09 | 25089.35 | 25326.13 | 25452.47 | 25729.35 |
| instrument-b@trade recv-busy | 10650 | 24932.37 | 24902.25 | 25354.55 | 25746.34 | 26355.31 | 27357.17 |
| instrument-b@trade uring-spin | 10650 | 24669.73 | 24647.38 | 25082.35 | 25323.52 | 25408.41 | 26766.56 |
| instrument-c@trade recv-busy | 1898 | 24826.27 | 24854.24 | 25215.40 | 25612.75 | 26747.76 | 26749.76 |
| instrument-c@trade uring-spin | 1898 | 24609.10 | 24633.32 | 24987.94 | 25350.62 | 25521.38 | 25522.38 |
| instrument-a@bookTicker recv-busy | 118654 | 24927.95 | 24842.06 | 25259.13 | 27328.04 | 35269.05 | 46842.41 |
| instrument-a@bookTicker uring-spin | 118654 | 25059.09 | 24970.53 | 25403.70 | 27628.79 | 34759.07 | 49609.35 |
| instrument-b@bookTicker recv-busy | 102900 | 24848.77 | 24781.28 | 25177.08 | 26840.57 | 34634.30 | 51671.76 |
| instrument-b@bookTicker uring-spin | 102899 | 24977.15 | 24910.76 | 25319.03 | 27393.35 | 34037.08 | 51824.38 |
| instrument-a@aggTrade recv-busy | 3467 | 24448.50 | 24434.88 | 25138.80 | 25715.22 | 33809.84 | 41042.44 |
| instrument-a@aggTrade uring-spin | 3467 | 24615.10 | 24582.85 | 25315.93 | 26091.71 | 26619.97 | 35669.89 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own upstream feed connection, so Δuser also contains the upstream feed's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-spin Δuser** | 246447 | 100.33 | 108.38 | 274.07 | 977.57 | 3434.36 | 24977.34 |
| uring-spin Δrx | 246447 | 97.14 | 105.78 | 270.26 | 961.11 | 3428.35 | 24947.47 |
| uring-spin Δstack (1-msg reads) | 52568 | 4.34 | 1.39 | 12.25 | 30.03 | 127.80 | 1554.85 |

uring-spin: 246447 paired messages; decoded before recv-busy on 19.6 % (uncorrected).

## live-recv-busy-vs-uring-sqpoll: live-feed, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=135545 clocksource=tsc

### Loop counters (live-recv-busy-vs-uring-sqpoll)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 204847 | 77353 | 2.65 | 320465490 | 1564.41 | 1068218 | 356073 | 100.0 | 82.9 | true | 0 | 0 | 1024.5 | 103382 / 3169 / 19 | 0 | 0 |
| uring-sqpoll | 204847 | 76204 | 2.69 | 1 | 0.00 | 0 | 59761075 | 0.0 | 0.0 | true | 0 | 0 | 783.5 | 4872 / 173 / 6 | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8214 | 6 |
| recv-busy | 1 | 8211 | 6 |
| recv-busy | 2 | 8209 | 2 |
| uring-sqpoll | 0 | 8214 | 2 |
| uring-sqpoll | 1 | 8216 | 6 |
| uring-sqpoll | 2 | 8211 | 6 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 43309629 | 192325 | 0 | 0 | 0.444 |
| uring-sqpoll | 43338251 | 171458 | 0 | 0 | 0.396 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| recv-busy (43817 of 198120 in 1-msg reads) | 43817 | 7.97 | 5.91 | 16.89 | 21.17 | 24.30 | 64.82 |
| uring-sqpoll (44576 of 198120 in 1-msg reads) | 44576 | 7.19 | 6.28 | 10.96 | 19.64 | 26.77 | 776.70 |

### upstream feed event time → kernel rx (`rx_sw − E`)

Includes the upstream feed's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| instrument-a@trade recv-busy | 8175 | 25374.52 | 24277.73 | 24877.91 | 62429.68 | 63207.68 | 72182.37 |
| instrument-a@trade uring-sqpoll | 8175 | 25511.87 | 24444.67 | 25038.61 | 60895.67 | 61972.46 | 69951.46 |
| instrument-b@trade recv-busy | 10883 | 25085.18 | 24235.43 | 24902.39 | 51376.03 | 73662.53 | 73665.53 |
| instrument-b@trade uring-sqpoll | 10883 | 25253.69 | 24509.54 | 25045.86 | 49940.78 | 71609.46 | 71612.46 |
| instrument-c@trade recv-busy | 2586 | 24928.63 | 24144.57 | 24850.61 | 47670.90 | 73174.53 | 73296.53 |
| instrument-c@trade uring-sqpoll | 2586 | 25103.05 | 24396.65 | 25030.86 | 46436.65 | 71107.46 | 71249.46 |
| instrument-a@bookTicker recv-busy | 83607 | 25381.13 | 25331.92 | 25763.61 | 27027.08 | 30005.06 | 51664.22 |
| instrument-a@bookTicker uring-sqpoll | 83607 | 25177.16 | 24749.21 | 25394.17 | 37069.27 | 55334.97 | 56833.22 |
| instrument-b@bookTicker recv-busy | 90054 | 25306.23 | 25271.92 | 25676.01 | 26813.68 | 30602.29 | 51048.72 |
| instrument-b@bookTicker uring-sqpoll | 90054 | 24966.29 | 24626.79 | 25138.83 | 34496.38 | 55167.56 | 56833.22 |
| instrument-a@aggTrade recv-busy | 2815 | 27501.25 | 24531.95 | 30816.25 | 91448.91 | 93135.00 | 98135.00 |
| instrument-a@aggTrade uring-sqpoll | 2815 | 28523.59 | 24874.02 | 39810.00 | 77968.13 | 78968.13 | 82038.95 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own upstream feed connection, so Δuser also contains the upstream feed's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser** | 198120 | -209.33 | -642.59 | 260.95 | 7724.96 | 29699.09 | 36929.57 |
| uring-sqpoll Δrx | 198120 | -209.11 | -641.97 | 260.05 | 7723.58 | 29707.39 | 36946.83 |
| uring-sqpoll Δstack (1-msg reads) | 42617 | -0.75 | 0.41 | 3.40 | 11.57 | 19.27 | 771.19 |

uring-sqpoll: 198120 paired messages; decoded before recv-busy on 87.3 % (uncorrected).

## live-uring-spin-vs-epoll-spin: live-feed, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (live-uring-spin-vs-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 336130 | 115518 | 2.91 | 231593 | 0.69 | 772 | 64058342 | 0.0 | 0.2 | false | 0 | 0 | 1360.0 | 23702 / 1156 / 80 | 0 | 0 |
| epoll-spin | 336131 | 117081 | 2.87 | 1234532972 | 3672.77 | 4115110 | 4114720 | 0.0 | 29.8 | false | 0 | 0 | 1349.9 | 49902 / 3440 / 89 | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8215 | 1 |
| uring-spin | 1 | 8209 | 0 |
| uring-spin | 2 | 8216 | 0 |
| epoll-spin | 0 | 8209 | 0 |
| epoll-spin | 1 | 8215 | 1 |
| epoll-spin | 2 | 8214 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 71156068 | 301597 | 0 | 0 | 0.424 |
| epoll-spin | 71162949 | 261739 | 0 | 0 | 0.368 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-spin (62853 of 324030 in 1-msg reads) | 62853 | 8.58 | 6.13 | 13.32 | 35.00 | 147.14 | 3630.51 |
| epoll-spin (63215 of 324030 in 1-msg reads) | 63215 | 10.74 | 5.94 | 22.39 | 50.81 | 159.19 | 3622.89 |

### upstream feed event time → kernel rx (`rx_sw − E`)

Includes the upstream feed's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| instrument-a@trade uring-spin | 14205 | 24990.55 | 24875.63 | 25452.64 | 29093.13 | 32201.85 | 32205.85 |
| instrument-a@trade epoll-spin | 14205 | 24962.44 | 24856.29 | 25361.23 | 32233.85 | 38364.51 | 38370.51 |
| instrument-b@trade uring-spin | 13238 | 24935.09 | 24895.42 | 25371.30 | 27291.25 | 27585.25 | 27588.25 |
| instrument-b@trade epoll-spin | 13238 | 24882.99 | 24863.46 | 25390.86 | 26314.94 | 26417.35 | 27122.68 |
| instrument-c@trade uring-spin | 2480 | 24794.53 | 24735.57 | 25189.80 | 26519.89 | 32951.14 | 32952.14 |
| instrument-c@trade epoll-spin | 2480 | 24765.80 | 24701.92 | 25255.82 | 26505.69 | 33000.63 | 33001.63 |
| instrument-a@bookTicker uring-spin | 168115 | 24832.35 | 24744.68 | 25181.42 | 27286.51 | 33630.59 | 48209.28 |
| instrument-a@bookTicker epoll-spin | 168115 | 24449.61 | 24391.21 | 24843.88 | 26403.26 | 29916.83 | 48033.80 |
| instrument-b@bookTicker uring-spin | 120509 | 24748.82 | 24660.87 | 25080.95 | 27345.43 | 33734.13 | 51897.92 |
| instrument-b@bookTicker epoll-spin | 120509 | 24353.34 | 24308.67 | 24736.69 | 26244.73 | 30668.91 | 51601.79 |
| instrument-a@aggTrade uring-spin | 5483 | 24880.87 | 24833.99 | 25461.81 | 25846.22 | 29769.44 | 34812.32 |
| instrument-a@aggTrade epoll-spin | 5483 | 24794.13 | 24762.89 | 25329.98 | 25685.72 | 26846.05 | 33989.58 |

### Paired by message, relative to uring-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own upstream feed connection, so Δuser also contains the upstream feed's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser** | 324030 | -350.36 | -355.06 | -45.06 | 367.46 | 1988.78 | 14358.82 |
| epoll-spin Δrx | 324030 | -350.71 | -355.38 | -48.42 | 368.80 | 1991.73 | 14370.05 |
| epoll-spin Δstack (1-msg reads) | 60942 | 2.17 | 0.02 | 10.31 | 34.61 | 131.75 | 1820.08 |

epoll-spin: 324030 paired messages; decoded before uring-spin on 93.0 % (uncorrected).

## live-uring-spin-vs-recv-busy: live-feed, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=66838 clocksource=tsc

### Loop counters (live-uring-spin-vs-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 156960 | 64499 | 2.43 | 128479 | 0.82 | 428 | 64802487 | 0.0 | 0.1 | false | 0 | 0 | 758.5 | 17616 / 472 / 26 | 0 | 0 |
| recv-busy | 156957 | 66344 | 2.37 | 375238291 | 2390.71 | 1250794 | 416931 | 100.0 | 79.7 | true | 0 | 0 | 127.4 | 97133 / 332 / 2 | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8209 | 4 |
| uring-spin | 1 | 8210 | 0 |
| uring-spin | 2 | 8214 | 1 |
| recv-busy | 0 | 8215 | 4 |
| recv-busy | 1 | 8212 | 4 |
| recv-busy | 2 | 8209 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 33374572 | 130785 | 0 | 0 | 0.392 |
| recv-busy | 33381676 | 110928 | 0 | 0 | 0.332 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-spin (38996 of 145901 in 1-msg reads) | 38996 | 8.63 | 5.60 | 18.14 | 29.33 | 70.35 | 4012.46 |
| recv-busy (39370 of 145901 in 1-msg reads) | 39370 | 4.26 | 3.82 | 5.66 | 9.13 | 12.77 | 18.87 |

### upstream feed event time → kernel rx (`rx_sw − E`)

Includes the upstream feed's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| instrument-a@trade uring-spin | 6823 | 24620.91 | 24606.93 | 25111.55 | 25419.69 | 25432.69 | 27565.36 |
| instrument-a@trade recv-busy | 6823 | 24323.18 | 24307.24 | 24839.67 | 25055.77 | 25170.97 | 25172.97 |
| instrument-b@trade uring-spin | 6669 | 24612.99 | 24611.57 | 25068.39 | 25470.96 | 25505.23 | 25507.23 |
| instrument-b@trade recv-busy | 6669 | 24330.29 | 24316.57 | 24715.14 | 24981.25 | 32196.73 | 32474.73 |
| instrument-c@trade uring-spin | 1731 | 24462.28 | 24441.04 | 24896.21 | 25371.91 | 27031.52 | 27033.52 |
| instrument-c@trade recv-busy | 1731 | 24273.63 | 24239.38 | 24606.91 | 27553.10 | 32441.09 | 32442.09 |
| instrument-a@bookTicker uring-spin | 68030 | 24932.20 | 24732.76 | 25166.10 | 28447.11 | 47600.01 | 49987.75 |
| instrument-a@bookTicker recv-busy | 68030 | 25459.40 | 25367.14 | 25776.31 | 27485.39 | 36460.78 | 50644.90 |
| instrument-b@bookTicker uring-spin | 60720 | 24864.43 | 24655.99 | 25072.83 | 29539.00 | 47610.77 | 48378.54 |
| instrument-b@bookTicker recv-busy | 60720 | 25392.85 | 25296.93 | 25698.90 | 28980.84 | 36257.02 | 48262.35 |
| instrument-a@aggTrade uring-spin | 1928 | 24855.60 | 24828.88 | 25512.09 | 25846.50 | 27444.98 | 29329.96 |
| instrument-a@aggTrade recv-busy | 1928 | 24500.82 | 24507.14 | 25091.69 | 25409.46 | 25574.52 | 28424.80 |

### Paired by message, relative to uring-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own upstream feed connection, so Δuser also contains the upstream feed's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **recv-busy Δuser** | 145901 | 428.92 | 619.37 | 755.85 | 1022.67 | 2300.40 | 8652.50 |
| recv-busy Δrx | 145901 | 431.56 | 622.06 | 757.63 | 1024.31 | 2301.43 | 8655.21 |
| recv-busy Δstack (1-msg reads) | 38287 | -4.41 | -1.74 | -0.86 | -0.27 | 2.58 | 14.35 |

recv-busy: 145901 paired messages; decoded before uring-spin on 8.3 % (uncorrected).

## live-uring-sqpoll-vs-recv-busy: live-feed, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=138876 clocksource=tsc

### Loop counters (live-uring-sqpoll-vs-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 226222 | 87225 | 2.59 | 0 | 0.00 | 0 | 59768821 | 0.0 | 0.0 | true | 0 | 1 | 763.5 | 6744 / 293 / 9 | 0 | 0 |
| recv-busy | 226222 | 87692 | 2.58 | 292898899 | 1294.74 | 976330 | 325443 | 100.0 | 83.9 | true | 0 | 0 | 867.2 | 158737 / 3161 / 10 | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8211 | 6 |
| uring-sqpoll | 1 | 8210 | 4 |
| uring-sqpoll | 2 | 8212 | 6 |
| recv-busy | 0 | 8210 | 4 |
| recv-busy | 1 | 8212 | 4 |
| recv-busy | 2 | 8216 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 48032278 | 281488 | 0 | 0 | 0.586 |
| recv-busy | 48021176 | 255025 | 0 | 0 | 0.531 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-sqpoll (49529 of 219745 in 1-msg reads) | 49529 | 7.24 | 6.06 | 10.80 | 20.74 | 26.90 | 115.89 |
| recv-busy (49489 of 219745 in 1-msg reads) | 49489 | 7.30 | 5.42 | 15.38 | 21.76 | 25.39 | 552.91 |

### upstream feed event time → kernel rx (`rx_sw − E`)

Includes the upstream feed's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| instrument-a@trade uring-sqpoll | 10575 | 24854.39 | 24804.14 | 25270.35 | 26290.37 | 37326.69 | 37329.69 |
| instrument-a@trade recv-busy | 10575 | 25635.07 | 24915.10 | 25527.54 | 50882.01 | 68079.83 | 68085.83 |
| instrument-b@trade uring-sqpoll | 7604 | 24967.58 | 24794.22 | 25206.00 | 37739.09 | 37774.69 | 37775.69 |
| instrument-b@trade recv-busy | 7604 | 25708.69 | 24835.19 | 25365.73 | 47151.20 | 67138.42 | 67143.42 |
| instrument-c@trade uring-sqpoll | 2101 | 24585.03 | 24573.11 | 24967.63 | 25461.01 | 25497.59 | 25497.59 |
| instrument-c@trade recv-busy | 2101 | 25100.80 | 24767.27 | 25130.78 | 43799.57 | 63164.73 | 63167.73 |
| instrument-a@bookTicker uring-sqpoll | 110101 | 25062.55 | 24803.22 | 25399.22 | 31776.87 | 45745.58 | 52980.51 |
| instrument-a@bookTicker recv-busy | 110101 | 25049.49 | 24961.43 | 25418.85 | 28203.15 | 34520.66 | 40560.57 |
| instrument-b@bookTicker uring-sqpoll | 86122 | 25079.96 | 24739.01 | 25677.14 | 32272.94 | 48500.31 | 52980.51 |
| instrument-b@bookTicker recv-busy | 86122 | 25009.90 | 24900.25 | 25396.25 | 28387.94 | 36783.17 | 40973.12 |
| instrument-a@aggTrade uring-sqpoll | 3242 | 25711.56 | 24746.54 | 26092.87 | 56608.93 | 56608.93 | 56608.93 |
| instrument-a@aggTrade recv-busy | 3242 | 25388.75 | 24713.69 | 25655.95 | 48511.73 | 54447.66 | 54447.66 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own upstream feed connection, so Δuser also contains the upstream feed's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **recv-busy Δuser** | 219745 | 29.58 | 173.13 | 397.27 | 1311.19 | 21891.71 | 43114.75 |
| recv-busy Δrx | 219745 | 29.84 | 172.58 | 397.71 | 1312.03 | 21893.83 | 43118.65 |
| recv-busy Δstack (1-msg reads) | 47966 | 0.05 | -1.02 | 8.13 | 14.64 | 18.03 | 546.35 |

recv-busy: 219745 paired messages; decoded before uring-sqpoll on 19.5 % (uncorrected).

### Loop counters (pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 501672 | 501672 | 1.00 | 144082833 | 287.21 | 4802761 | 4769316 | 0.0 | 37.2 | false | 0 | 0 | 499.9 | 501843 / 605 / 34 | 0 | 0 |

### Loop counters (pp-epoll-spin-100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3000 | 3000 | 1.00 | 144582384 | 48194.13 | 4819413 | 4819213 | 0.0 | 28.4 | false | 0 | 0 | 150.6 | 5881 / 70 / 5 | 0 | 0 |

### Loop counters (pp-epoll-spin-1000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 29870 | 29870 | 1.00 | 145056412 | 4856.26 | 4835214 | 4833222 | 0.0 | 28.9 | false | 0 | 0 | 604.6 | 5298 / 134 / 20 | 0 | 0 |

### Loop counters (pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 296685 | 296685 | 1.00 | 143947354 | 485.19 | 4798245 | 4778466 | 0.0 | 33.3 | false | 0 | 0 | 727.7 | 2943 / 289 / 31 | 0 | 0 |

### Loop counters (pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 643068 | 643068 | 1.00 | 33911357 | 52.73 | 1130379 | 1108943 | 98.1 | 83.2 | true | 0 | 0 | 155.6 | 644340 / 526 / 9 | 0 | 0 |

### Loop counters (pp-recv-busy-100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3000 | 3000 | 1.00 | 38228088 | 12742.70 | 1274270 | 1274170 | 100.0 | 79.8 | true | 0 | 0 | 812.5 | 5003 / 95 / 3 | 0 | 0 |

### Loop counters (pp-recv-busy-1000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 29989 | 29989 | 1.00 | 38044261 | 1268.61 | 1268142 | 1267142 | 99.9 | 80.1 | true | 0 | 0 | 141.8 | 1243 / 37 / 2 | 0 | 0 |

### Loop counters (pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 298708 | 298708 | 1.00 | 35527444 | 118.94 | 1184248 | 1174291 | 99.2 | 81.8 | true | 0 | 0 | 151.8 | 13139 / 113 / 7 | 0 | 0 |

### Loop counters (pp-uring-napi-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-napi | 596736 | 596736 | 1.00 | 5618758 | 9.42 | 187292 | 207183 | 0.0 | 94.5 | true | 84760 | 0 | 6903.9 | 3909944 / 8668 / 366 | 0 | 0 |

### Loop counters (pp-uring-napi-100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-napi | 2999 | 2999 | 1.00 | 5686091 | 1896.00 | 189536 | 189636 | 0.0 | 95.0 | true | 253016 | 0 | 6194.8 | 4342088 / 5312 / 332 | 0 | 0 |

### Loop counters (pp-uring-napi-1000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-napi | 29523 | 29523 | 1.00 | 5714226 | 193.55 | 190474 | 191458 | 0.0 | 94.9 | true | 72339 | 0 | 7536.1 | 4068164 / 5492 / 849 | 0 | 0 |

### Loop counters (pp-uring-napi-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-napi | 289920 | 289920 | 1.00 | 5912193 | 20.39 | 197073 | 206737 | 0.0 | 94.5 | true | 135943 | 0 | 4559.4 | 4248857 / 5755 / 303 | 0 | 0 |

### Loop counters (pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 587017 | 587017 | 1.00 | 1174036 | 2.00 | 39135 | 56956971 | 0.0 | 11.5 | false | 0 | 0 | 220.6 | 3180 / 267 / 27 | 0 | 0 |

### Loop counters (pp-uring-spin-100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 3000 | 3000 | 1.00 | 9001 | 3.00 | 300 | 27429382 | 0.0 | 0.2 | false | 0 | 0 | 340.4 | 6100 / 57 / 4 | 0 | 0 |

### Loop counters (pp-uring-spin-1000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 29886 | 29886 | 1.00 | 89551 | 3.00 | 2985 | 29431260 | 0.0 | 0.7 | false | 0 | 0 | 316.6 | 4267 / 153 / 19 | 0 | 0 |

### Loop counters (pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 296370 | 296370 | 1.00 | 887576 | 2.99 | 29586 | 43731465 | 0.0 | 5.9 | false | 0 | 0 | 861.6 | 1768 / 236 / 24 | 0 | 0 |

### Loop counters (pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 642471 | 642471 | 1.00 | 0 | 0.00 | 0 | 55985242 | 0.0 | 0.0 | true | 0 | 0 | 652.6 | 296 / 39 / 3 | 0 | 0 |

### Loop counters (pp-uring-sqpoll-100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3000 | 3000 | 1.00 | 2 | 0.00 | 0 | 22215209 | 0.0 | 0.0 | true | 0 | 0 | 66.1 | 243 / 23 / 0 | 0 | 0 |

### Loop counters (pp-uring-sqpoll-1000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 29999 | 29999 | 1.00 | 2 | 0.00 | 0 | 26005888 | 0.0 | 0.0 | true | 0 | 0 | 156.0 | 777 / 33 / 2 | 0 | 0 |

### Loop counters (pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 299768 | 299768 | 1.00 | 1 | 0.00 | 0 | 37833276 | 0.0 | 0.0 | true | 0 | 0 | 627.8 | 195 / 45 / 3 | 0 | 0 |

## Ping-pong round trip (client send → echo decoded)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin @ 0/s | 484208 | 59.81 | 59.34 | 71.10 | 82.22 | 256.63 | 8246.27 |
| epoll-spin @ 100/s | 2899 | 99.38 | 78.66 | 100.95 | 629.66 | 3023.26 | 6544.93 |
| epoll-spin @ 1000/s | 28870 | 67.58 | 51.81 | 69.38 | 298.24 | 2612.37 | 8186.01 |
| epoll-spin @ 10000/s | 286774 | 53.99 | 49.59 | 65.42 | 79.80 | 347.38 | 6615.22 |
| recv-busy @ 0/s | 622988 | 46.48 | 44.60 | 55.28 | 64.63 | 72.27 | 3591.56 |
| recv-busy @ 100/s | 2899 | 64.42 | 64.89 | 73.25 | 99.21 | 135.07 | 290.75 |
| recv-busy @ 1000/s | 28989 | 50.09 | 47.95 | 60.14 | 67.38 | 153.54 | 363.11 |
| recv-busy @ 10000/s | 288749 | 46.58 | 45.47 | 48.89 | 61.06 | 78.50 | 1100.51 |
| uring-napi @ 0/s | 576153 | 50.21 | 48.02 | 59.20 | 69.69 | 112.35 | 6959.00 |
| uring-napi @ 100/s | 2899 | 67.47 | 65.65 | 84.99 | 105.50 | 286.62 | 1183.12 |
| uring-napi @ 1000/s | 28525 | 54.89 | 48.60 | 63.14 | 82.34 | 728.54 | 7148.68 |
| uring-napi @ 10000/s | 280289 | 50.12 | 47.10 | 60.67 | 66.16 | 90.38 | 4604.54 |
| uring-spin @ 0/s | 567233 | 51.04 | 48.89 | 54.95 | 73.62 | 159.19 | 6204.22 |
| uring-spin @ 100/s | 2899 | 91.36 | 79.63 | 91.64 | 260.77 | 2689.47 | 7194.88 |
| uring-spin @ 1000/s | 28889 | 70.74 | 54.15 | 71.72 | 410.90 | 2220.83 | 8128.68 |
| uring-spin @ 10000/s | 286419 | 54.79 | 51.29 | 61.80 | 81.19 | 371.01 | 12160.96 |
| uring-sqpoll @ 0/s | 621346 | 46.61 | 44.84 | 52.91 | 63.46 | 73.62 | 4994.00 |
| uring-sqpoll @ 100/s | 2899 | 61.73 | 60.82 | 69.93 | 93.63 | 117.62 | 207.56 |
| uring-sqpoll @ 1000/s | 28998 | 57.01 | 55.43 | 65.08 | 80.98 | 94.51 | 271.62 |
| uring-sqpoll @ 10000/s | 289784 | 48.10 | 47.02 | 50.46 | 62.49 | 79.67 | 1214.06 |
