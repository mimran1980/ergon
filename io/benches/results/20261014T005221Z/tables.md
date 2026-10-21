## Summary: paired runs (µs; Δ = B − A, negative means B decoded first)

| run | A | B | paired msgs | Δuser p50 | Δuser p90 | Δuser p99 | Δrx p50 | Δstack p50 | Δstack p99 | A syscalls/s | B syscalls/s | A kernel CPU % | B kernel CPU % | shared RX queue |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lab-burst-aa-epoll-busy | epoll-busy | epoll-busy | 11499863 | 0.46 | 10.59 | 38.35 | 0.22 | 0.02 | 20.66 | 64788 | 64749 | 94.9 | 95.1 | no |
| lab-burst-aa-epoll-spin | epoll-spin | epoll-spin | 11500300 | -0.04 | 12.06 | 27.79 | -0.03 | -1.45 | 33.94 | 4168878 | 4263459 | 39.2 | 40.5 | yes |
| lab-burst-epoll-busy-vs-uring-napi | epoll-busy | uring-napi | 11499822 | 5.12 | 46.40 | 68.70 | 4.04 | -0.04 | 35.77 | 67733 | 56991 | 94.8 | 97.3 | no |
| lab-burst-epoll-busy-vs-uring-sqpoll | epoll-busy | uring-sqpoll | 11499801 | -28.38 | 0.19 | 7.80 | -12.39 | -8.60 | 2.48 | 53857 | 0 | 95.6 | 0.0 | no |
| lab-burst-epoll-spin-vs-uring-spin | epoll-spin | uring-spin | 11500243 | 0.24 | 12.56 | 25.14 | 0.03 | 1.80 | 39.35 | 4104738 | 58020 | 40.1 | 15.1 | yes |
| lab-burst-uring-napi-vs-epoll-busy | uring-napi | epoll-busy | 11500289 | 19.33 | 50.19 | 72.40 | 7.13 | 1.01 | 37.49 | 74195 | 60437 | 96.9 | 95.7 | no |
| lab-burst-uring-spin-vs-epoll-spin | uring-spin | epoll-spin | 11499900 | -0.02 | 11.26 | 23.95 | 0.03 | 1.50 | 33.41 | 57510 | 4221783 | 13.7 | 39.9 | yes |
| lab-burst-uring-sqpoll-vs-epoll-busy | uring-sqpoll | epoll-busy | 11499815 | -1.32 | 11.39 | 31.54 | -0.89 | -0.25 | 17.36 | 0 | 72308 | 0.0 | 94.9 | no |
| lab-replay1x-aa-epoll-busy | epoll-busy | epoll-busy | 285807 | -0.43 | 4.48 | 19.59 | -0.15 | -0.11 | 13.14 | 7354 | 7383 | 95.9 | 95.8 | no |
| lab-replay1x-aa-epoll-spin | epoll-spin | epoll-spin | 285807 | 0.03 | 5.89 | 17.26 | -0.03 | 0.03 | 5.24 | 4800353 | 4902820 | 31.0 | 31.7 | yes |
| lab-replay1x-epoll-busy-vs-uring-napi | epoll-busy | uring-napi | 285821 | -3.81 | 5.74 | 31.83 | -2.46 | -3.89 | 17.66 | 7400 | 10403 | 96.0 | 99.4 | no |
| lab-replay1x-epoll-busy-vs-uring-sqpoll | epoll-busy | uring-sqpoll | 285807 | -9.66 | 28.52 | 63.41 | -4.02 | -9.46 | 20.37 | 7370 | 0 | 95.3 | 0.0 | no |
| lab-replay1x-epoll-spin-vs-uring-spin | epoll-spin | uring-spin | 285807 | 0.22 | 6.04 | 16.92 | 0.03 | 0.23 | 5.19 | 4829121 | 1340 | 31.0 | 0.2 | yes |
| lab-replay1x-uring-napi-vs-epoll-busy | uring-napi | epoll-busy | 285807 | 2.82 | 20.62 | 37.89 | 1.46 | 1.87 | 22.79 | 10395 | 7376 | 99.4 | 96.0 | no |
| lab-replay1x-uring-spin-vs-epoll-spin | uring-spin | epoll-spin | 285807 | -0.18 | 7.74 | 25.47 | 0.03 | -0.17 | 17.08 | 1309 | 4967014 | 0.2 | 31.9 | yes |
| lab-replay1x-uring-sqpoll-vs-epoll-busy | uring-sqpoll | epoll-busy | 285821 | 8.25 | 24.77 | 39.75 | 4.21 | 9.13 | 24.45 | 0 | 7377 | 0.0 | 95.8 | no |
| live-aa-epoll-busy | epoll-busy | epoll-busy | 129597 | 572.16 | 775.08 | 5096.20 | 572.20 | -0.30 | 10.89 | 6682 | 6679 | 95.0 | 95.1 | no |
| live-epoll-busy-vs-uring-napi | epoll-busy | uring-napi | 241456 | 438.93 | 655.82 | 3599.48 | 443.32 | -7.02 | 7.87 | 6809 | 9814 | 95.2 | 99.4 | yes |
| live-epoll-busy-vs-uring-sqpoll | epoll-busy | uring-sqpoll | 181558 | -204.91 | 202.44 | 1910.37 | -198.17 | -8.34 | 0.03 | 6780 | 0 | 95.0 | 0.0 | yes |
| live-epoll-spin-vs-uring-spin | epoll-spin | uring-spin | 206197 | -543.82 | -29.31 | 1404.99 | -544.21 | 0.43 | 5.68 | 4796994 | 551 | 30.9 | 0.1 | yes |
| live-uring-napi-vs-epoll-busy | uring-napi | epoll-busy | 161827 | -32.45 | 169.45 | 1570.78 | -33.21 | 6.06 | 18.68 | 9704 | 6717 | 99.4 | 95.1 | no |
| live-uring-spin-vs-epoll-spin | uring-spin | epoll-spin | 267539 | 575.46 | 1192.85 | 31002.04 | 575.76 | -0.37 | 3.57 | 675 | 4968448 | 0.1 | 31.8 | yes |
| live-uring-sqpoll-vs-epoll-busy | uring-sqpoll | epoll-busy | 172489 | 57.52 | 328.21 | 5854.21 | 50.64 | 8.54 | 22.29 | 0 | 6742 | 0.0 | 94.9 | yes |

## lab-burst-aa-epoll-busy: lab (A/A control), 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=12511486 clocksource=tsc

### Loop counters (lab-burst-aa-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 12003532 | 7298405 | 1.64 | 15549216 | 1.30 | 64788 | 34378 | 0.0 | 94.9 | true | 0 | 0 |
| epoll-busy | 12003513 | 7295828 | 1.65 | 15539860 | 1.29 | 64749 | 34350 | 0.0 | 95.1 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8216 | 1 |
| epoll-busy | 0 | 8209 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 2997427712 | 0 | 0 | 0 | 0.000 |
| epoll-busy | 2997422917 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy #0 (3272324 of 11499882 in 1-msg reads) | 3272324 | 4.92 | 4.01 | 5.57 | 24.07 | 27.38 | 81.93 |
| epoll-busy #1 (3263900 of 11499863 in 1-msg reads) | 3263900 | 5.19 | 4.03 | 6.50 | 24.66 | 29.70 | 108.83 |

### Paired by message, relative to epoll-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy #1 Δuser (write-order corrected)** | 11499863 | 0.70 | 0.46 | 10.59 | 38.35 | 53.95 | 1027.73 |
| epoll-busy #1 Δuser, even seq | 5749931 | 2.43 | 2.57 | 13.19 | 39.83 | 55.65 | 1025.62 |
| epoll-busy #1 Δuser, odd seq | 5749932 | -1.03 | -1.66 | 8.01 | 36.84 | 52.30 | 1025.61 |
| epoll-busy #1 Δrx | 11499863 | 0.49 | 0.22 | 8.64 | 25.49 | 39.06 | 993.86 |
| epoll-busy #1 Δstack (1-msg reads) | 1303271 | 0.24 | 0.02 | 1.43 | 20.66 | 25.44 | 76.48 |

epoll-busy #1: 11499863 paired messages; decoded before epoll-busy on 49.0 % (uncorrected).

## lab-burst-aa-epoll-spin: lab (A/A control), 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-burst-aa-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 12004000 | 7512546 | 1.60 | 1000530658 | 83.35 | 4168878 | 4137575 | 0.0 | 39.2 | false | 0 | 0 |
| epoll-spin | 12004000 | 7400617 | 1.62 | 1023230185 | 85.24 | 4263459 | 4232623 | 0.0 | 40.5 | false | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8210 | 0 |
| epoll-spin | 0 | 8210 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 2997545726 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 2997545726 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin #0 (3655904 of 11500300 in 1-msg reads) | 3655904 | 22.72 | 21.82 | 39.60 | 62.66 | 80.19 | 200.11 |
| epoll-spin #1 (3444625 of 11500300 in 1-msg reads) | 3444625 | 22.47 | 21.69 | 39.73 | 62.69 | 79.76 | 313.06 |

### Paired by message, relative to epoll-spin (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin #1 Δuser (write-order corrected)** | 11500300 | -0.06 | -0.04 | 12.06 | 27.79 | 41.47 | 988.80 |
| epoll-spin #1 Δuser, even seq | 5750150 | 1.72 | 1.99 | 15.66 | 34.35 | 43.59 | 990.83 |
| epoll-spin #1 Δuser, odd seq | 5750150 | -1.84 | -2.07 | 7.68 | 23.16 | 39.34 | 602.88 |
| epoll-spin #1 Δrx | 11500300 | -0.21 | -0.03 | 0.21 | 48.62 | 70.92 | 976.22 |
| epoll-spin #1 Δstack (1-msg reads) | 1874771 | -0.27 | -1.45 | 11.10 | 33.94 | 59.17 | 100.55 |

epoll-spin #1: 11500300 paired messages; decoded before epoll-spin on 51.1 % (uncorrected).

## lab-burst-epoll-busy-vs-uring-napi: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=10969700 clocksource=tsc

### Loop counters (lab-burst-epoll-busy-vs-uring-napi)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 12003347 | 7650798 | 1.57 | 16256016 | 1.35 | 67733 | 35855 | 0.0 | 94.8 | true | 0 | 0 |
| uring-napi | 12003322 | 6688696 | 1.79 | 13677889 | 1.14 | 56991 | 56991 | 0.0 | 97.3 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8213 | 0 |
| uring-napi | 0 | 8212 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 2997381047 | 0 | 0 | 0 | 0.000 |
| uring-napi | 2997375061 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy (3718367 of 11499847 in 1-msg reads) | 3718367 | 4.37 | 3.87 | 4.50 | 22.29 | 24.96 | 128.81 |
| uring-napi (2783394 of 11499822 in 1-msg reads) | 2783394 | 8.94 | 3.97 | 23.67 | 39.68 | 44.07 | 173.42 |

### Paired by message, relative to epoll-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-napi Δuser (write-order corrected)** | 11499822 | 14.64 | 5.12 | 46.40 | 68.70 | 82.63 | 905.35 |
| uring-napi Δuser, even seq | 5749911 | 13.66 | 4.14 | 44.97 | 67.39 | 81.30 | 901.49 |
| uring-napi Δuser, odd seq | 5749911 | 15.63 | 6.09 | 47.82 | 70.00 | 83.97 | 906.33 |
| uring-napi Δrx | 11499822 | 8.10 | 4.04 | 29.03 | 49.59 | 63.05 | 851.42 |
| uring-napi Δstack (1-msg reads) | 1454672 | 4.18 | -0.04 | 19.82 | 35.77 | 40.20 | 169.91 |

uring-napi: 11499822 paired messages; decoded before epoll-busy on 31.0 % (uncorrected).

## lab-burst-epoll-busy-vs-uring-sqpoll: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=6987491 clocksource=tsc

### Loop counters (lab-burst-epoll-busy-vs-uring-sqpoll)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 12003279 | 6000458 | 2.00 | 12925670 | 1.08 | 53857 | 28855 | 0.0 | 95.6 | true | 0 | 0 |
| uring-sqpoll | 12003251 | 7844413 | 1.53 | 0 | 0.00 | 0 | 255373447 | 0.0 | 0.0 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8214 | 1 |
| uring-sqpoll | 0 | 8211 | 6 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 2997364199 | 0 | 0 | 0 | 0.000 |
| uring-sqpoll | 2997357124 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy (2219849 of 11499830 in 1-msg reads) | 2219849 | 14.39 | 11.70 | 26.47 | 41.73 | 46.38 | 286.99 |
| uring-sqpoll (4000729 of 11499801 in 1-msg reads) | 4000729 | 4.14 | 3.98 | 4.59 | 7.85 | 11.42 | 48.67 |

### Paired by message, relative to epoll-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser (write-order corrected)** | 11499801 | -27.75 | -28.38 | 0.19 | 7.80 | 15.51 | 565.23 |
| uring-sqpoll Δuser, even seq | 5749900 | -29.07 | -29.81 | -2.00 | 6.27 | 13.84 | 563.81 |
| uring-sqpoll Δuser, odd seq | 5749901 | -26.43 | -26.96 | 2.29 | 9.36 | 17.21 | 563.77 |
| uring-sqpoll Δrx | 11499801 | -14.97 | -12.39 | 2.40 | 8.54 | 16.60 | 562.93 |
| uring-sqpoll Δstack (1-msg reads) | 1294293 | -10.53 | -8.60 | 0.26 | 2.48 | 6.29 | 43.85 |

uring-sqpoll: 11499801 paired messages; decoded before epoll-busy on 89.9 % (uncorrected).

## lab-burst-epoll-spin-vs-uring-spin: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-burst-epoll-spin-vs-uring-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 12003850 | 7593459 | 1.58 | 985137203 | 82.07 | 4104738 | 4073099 | 0.0 | 40.1 | false | 0 | 0 |
| uring-spin | 12003850 | 7663677 | 1.57 | 13924828 | 1.16 | 58020 | 253275650 | 0.0 | 15.1 | false | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8210 | 0 |
| uring-spin | 0 | 8210 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 2997507847 | 0 | 0 | 0 | 0.000 |
| uring-spin | 2997507847 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin (3664671 of 11500243 in 1-msg reads) | 3664671 | 22.75 | 21.43 | 43.18 | 62.02 | 72.85 | 267.07 |
| uring-spin (3834582 of 11500243 in 1-msg reads) | 3834582 | 22.87 | 21.42 | 43.06 | 62.01 | 72.95 | 439.22 |

### Paired by message, relative to epoll-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 11500243 | 0.43 | 0.24 | 12.56 | 25.14 | 40.12 | 972.28 |
| uring-spin Δuser, even seq | 5750122 | -1.23 | -2.14 | 9.33 | 22.10 | 36.86 | 969.73 |
| uring-spin Δuser, odd seq | 5750121 | 2.09 | 2.61 | 15.43 | 28.88 | 42.98 | 974.66 |
| uring-spin Δrx | 11500243 | 0.14 | 0.03 | 3.45 | 49.98 | 67.94 | 976.69 |
| uring-spin Δstack (1-msg reads) | 2006620 | 0.38 | 1.80 | 12.08 | 39.35 | 59.39 | 136.44 |

uring-spin: 11500243 paired messages; decoded before epoll-spin on 47.9 % (uncorrected).

## lab-burst-uring-napi-vs-epoll-busy: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=10975704 clocksource=tsc

### Loop counters (lab-burst-uring-napi-vs-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-napi | 12003751 | 8261628 | 1.45 | 17806913 | 1.48 | 74195 | 74195 | 0.0 | 96.9 | true | 0 | 0 |
| epoll-busy | 12003750 | 6829820 | 1.76 | 14504956 | 1.21 | 60437 | 31980 | 0.0 | 95.7 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-napi | 0 | 8212 | 1 |
| epoll-busy | 0 | 8213 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-napi | 2997482912 | 0 | 0 | 0 | 0.000 |
| epoll-busy | 2997482660 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-napi (4555725 of 11500290 in 1-msg reads) | 4555725 | 3.77 | 3.59 | 4.23 | 7.38 | 16.25 | 52.49 |
| epoll-busy (3098360 of 11500292 in 1-msg reads) | 3098360 | 11.04 | 4.64 | 25.14 | 41.13 | 47.33 | 179.57 |

### Paired by message, relative to uring-napi

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy Δuser (write-order corrected)** | 11500289 | 20.96 | 19.33 | 50.19 | 72.40 | 91.59 | 989.20 |
| epoll-busy Δuser, even seq | 5750145 | 18.92 | 17.33 | 48.25 | 70.51 | 89.56 | 987.19 |
| epoll-busy Δuser, odd seq | 5750144 | 23.00 | 21.34 | 52.14 | 74.29 | 93.64 | 987.20 |
| epoll-busy Δrx | 11500289 | 11.22 | 7.13 | 32.05 | 52.40 | 69.73 | 967.84 |
| epoll-busy Δstack (1-msg reads) | 1442211 | 7.44 | 1.01 | 21.59 | 37.49 | 43.71 | 149.72 |

epoll-busy: 11500289 paired messages; decoded before uring-napi on 19.3 % (uncorrected).

## lab-burst-uring-spin-vs-epoll-spin: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-burst-uring-spin-vs-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 12003350 | 7572339 | 1.59 | 13802369 | 1.15 | 57510 | 257738559 | 0.0 | 13.7 | false | 0 | 0 |
| epoll-spin | 12003350 | 7547489 | 1.59 | 1013227873 | 84.41 | 4221783 | 4190335 | 0.0 | 39.9 | false | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8212 | 1 |
| epoll-spin | 0 | 8212 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 2997381806 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 2997381806 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-spin (3658262 of 11499900 in 1-msg reads) | 3658262 | 19.74 | 18.35 | 37.67 | 53.44 | 67.16 | 140.76 |
| epoll-spin (3620296 of 11499900 in 1-msg reads) | 3620296 | 19.84 | 18.49 | 37.59 | 53.32 | 67.10 | 521.59 |

### Paired by message, relative to uring-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser (write-order corrected)** | 11499900 | 0.25 | -0.02 | 11.26 | 23.95 | 39.96 | 990.96 |
| epoll-spin Δuser, even seq | 5749950 | -1.35 | -2.22 | 8.31 | 21.02 | 36.63 | 704.57 |
| epoll-spin Δuser, odd seq | 5749950 | 1.85 | 2.18 | 14.08 | 27.16 | 42.66 | 993.16 |
| epoll-spin Δrx | 11499900 | 0.04 | 0.03 | 4.87 | 44.20 | 63.53 | 1029.65 |
| epoll-spin Δstack (1-msg reads) | 1765165 | 0.16 | 1.50 | 9.06 | 33.41 | 52.61 | 89.25 |

epoll-spin: 11499900 paired messages; decoded before uring-spin on 47.2 % (uncorrected).

## lab-burst-uring-sqpoll-vs-epoll-busy: lab, 240 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=11236783 clocksource=tsc

### Loop counters (lab-burst-uring-sqpoll-vs-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 12003365 | 8057655 | 1.49 | 0 | 0.00 | 0 | 252213401 | 0.0 | 0.0 | true | 0 | 0 |
| epoll-busy | 12003387 | 8211210 | 1.46 | 17353866 | 1.45 | 72308 | 38094 | 0.0 | 94.9 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8212 | 6 |
| epoll-busy | 0 | 8213 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 2997385585 | 0 | 0 | 0 | 0.000 |
| epoll-busy | 2997391116 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-sqpoll (4450766 of 11499815 in 1-msg reads) | 4450766 | 6.52 | 4.28 | 13.11 | 25.90 | 29.86 | 104.50 |
| epoll-busy (4654782 of 11499837 in 1-msg reads) | 4654782 | 4.77 | 4.03 | 5.61 | 21.45 | 27.19 | 86.59 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy Δuser (write-order corrected)** | 11499815 | -3.41 | -1.32 | 11.39 | 31.54 | 47.25 | 615.38 |
| epoll-busy Δuser, even seq | 5749907 | -5.47 | -3.51 | 7.67 | 29.06 | 44.94 | 613.19 |
| epoll-busy Δuser, odd seq | 5749908 | -1.35 | 0.87 | 15.41 | 33.94 | 49.57 | 613.33 |
| epoll-busy Δrx | 11499815 | -1.47 | -0.89 | 7.83 | 21.88 | 34.56 | 585.04 |
| epoll-busy Δstack (1-msg reads) | 2057601 | -1.81 | -0.25 | 1.09 | 17.36 | 23.46 | 81.85 |

epoll-busy: 11499815 paired messages; decoded before uring-sqpoll on 56.3 % (uncorrected).

## lab-replay1x-aa-epoll-busy: lab (A/A control), 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=163517 clocksource=tsc

### Loop counters (lab-replay1x-aa-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 292591 | 208910 | 1.40 | 2647374 | 9.05 | 7354 | 6774 | 0.0 | 95.9 | true | 0 | 0 |
| epoll-busy | 292591 | 213513 | 1.37 | 2657801 | 9.08 | 7383 | 6790 | 0.0 | 95.8 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8212 | 2 |
| epoll-busy | 0 | 8213 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 73066446 | 0 | 0 | 0 | 0.000 |
| epoll-busy | 73066446 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy #0 (143194 of 285807 in 1-msg reads) | 143194 | 10.93 | 12.72 | 16.41 | 26.41 | 40.30 | 63.59 |
| epoll-busy #1 (147126 of 285807 in 1-msg reads) | 147126 | 9.63 | 12.13 | 15.07 | 23.57 | 29.28 | 56.13 |

### Paired by message, relative to epoll-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy #1 Δuser (write-order corrected)** | 285807 | -4.44 | -0.43 | 4.48 | 19.59 | 34.07 | 128.66 |
| epoll-busy #1 Δuser, even seq | 142903 | -2.57 | 0.26 | 6.86 | 22.58 | 39.85 | 72.97 |
| epoll-busy #1 Δuser, odd seq | 142904 | -6.32 | -1.12 | 2.21 | 12.96 | 25.56 | 127.97 |
| epoll-busy #1 Δrx | 285807 | -2.29 | -0.15 | 6.33 | 18.62 | 28.90 | 153.38 |
| epoll-busy #1 Δstack (1-msg reads) | 106711 | -0.63 | -0.11 | 1.56 | 13.14 | 20.20 | 34.76 |

epoll-busy #1: 285807 paired messages; decoded before epoll-busy on 56.7 % (uncorrected).

## lab-replay1x-aa-epoll-spin: lab (A/A control), 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-replay1x-aa-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 292591 | 241418 | 1.21 | 1728127178 | 5906.29 | 4800353 | 4799683 | 0.0 | 31.0 | false | 0 | 0 |
| epoll-spin | 292591 | 241441 | 1.21 | 1765015201 | 6032.36 | 4902820 | 4902149 | 0.0 | 31.7 | false | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8215 | 1 |
| epoll-spin | 0 | 8215 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 73066446 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 73066446 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin #0 (186777 of 285807 in 1-msg reads) | 186777 | 5.95 | 5.79 | 8.61 | 15.14 | 24.78 | 60.98 |
| epoll-spin #1 (187108 of 285807 in 1-msg reads) | 187108 | 5.99 | 5.83 | 8.66 | 15.23 | 25.04 | 58.94 |

### Paired by message, relative to epoll-spin (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin #1 Δuser (write-order corrected)** | 285807 | 0.00 | 0.03 | 5.89 | 17.26 | 32.74 | 63.35 |
| epoll-spin #1 Δuser, even seq | 142903 | -2.24 | -2.05 | 4.12 | 10.48 | 24.27 | 47.20 |
| epoll-spin #1 Δuser, odd seq | 142904 | 2.25 | 2.10 | 7.80 | 25.27 | 36.36 | 65.43 |
| epoll-spin #1 Δrx | 285807 | -0.01 | -0.03 | 6.66 | 20.54 | 35.69 | 66.46 |
| epoll-spin #1 Δstack (1-msg reads) | 157036 | 0.03 | 0.03 | 2.31 | 5.24 | 17.12 | 53.75 |

epoll-spin #1: 285807 paired messages; decoded before epoll-spin on 50.4 % (uncorrected).

## lab-replay1x-epoll-busy-vs-uring-napi: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=231286 clocksource=tsc

### Loop counters (lab-replay1x-epoll-busy-vs-uring-napi)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 292606 | 217262 | 1.35 | 2663961 | 9.10 | 7400 | 6796 | 0.0 | 96.0 | true | 0 | 0 |
| uring-napi | 292605 | 225974 | 1.29 | 3745153 | 12.80 | 10403 | 10403 | 0.0 | 99.4 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8214 | 1 |
| uring-napi | 0 | 8211 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 73070257 | 0 | 0 | 0 | 0.000 |
| uring-napi | 73069981 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy (151737 of 285822 in 1-msg reads) | 151737 | 12.72 | 13.01 | 24.54 | 29.92 | 40.91 | 134.40 |
| uring-napi (164704 of 285821 in 1-msg reads) | 164704 | 6.93 | 4.64 | 13.70 | 23.57 | 31.12 | 68.20 |

### Paired by message, relative to epoll-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-napi Δuser (write-order corrected)** | 285821 | -7.88 | -3.81 | 5.74 | 31.83 | 48.09 | 137.27 |
| uring-napi Δuser, even seq | 142910 | -5.59 | -1.93 | 8.26 | 34.73 | 51.28 | 125.55 |
| uring-napi Δuser, odd seq | 142911 | -10.17 | -5.68 | 3.46 | 28.69 | 44.40 | 135.40 |
| uring-napi Δrx | 285821 | -3.46 | -2.46 | 6.22 | 21.47 | 35.12 | 109.35 |
| uring-napi Δstack (1-msg reads) | 119818 | -6.68 | -3.89 | 0.89 | 17.66 | 24.76 | 52.15 |

uring-napi: 285821 paired messages; decoded before epoll-busy on 70.8 % (uncorrected).

## lab-replay1x-epoll-busy-vs-uring-sqpoll: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=223135 clocksource=tsc

### Loop counters (lab-replay1x-epoll-busy-vs-uring-sqpoll)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 292591 | 210521 | 1.39 | 2653081 | 9.07 | 7370 | 6785 | 0.0 | 95.3 | true | 0 | 0 |
| uring-sqpoll | 292591 | 225297 | 1.30 | 0 | 0.00 | 0 | 271831012 | 0.0 | 0.0 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8210 | 0 |
| uring-sqpoll | 0 | 8215 | 6 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 73066446 | 0 | 0 | 0 | 0.000 |
| uring-sqpoll | 73066446 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy (142988 of 285807 in 1-msg reads) | 142988 | 12.55 | 13.30 | 22.98 | 29.29 | 42.13 | 58.46 |
| uring-sqpoll (165774 of 285807 in 1-msg reads) | 165774 | 5.27 | 4.27 | 6.34 | 25.57 | 41.20 | 66.03 |

### Paired by message, relative to epoll-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser (write-order corrected)** | 285807 | -4.78 | -9.66 | 28.52 | 63.41 | 83.11 | 566.19 |
| uring-sqpoll Δuser, even seq | 142903 | -6.63 | -10.79 | 27.01 | 61.93 | 81.69 | 565.06 |
| uring-sqpoll Δuser, odd seq | 142904 | -2.93 | -8.53 | 29.96 | 64.74 | 84.39 | 408.60 |
| uring-sqpoll Δrx | 285807 | -2.05 | -4.02 | 14.15 | 44.23 | 60.80 | 553.26 |
| uring-sqpoll Δstack (1-msg reads) | 119297 | -8.92 | -9.46 | 0.01 | 20.37 | 36.29 | 57.80 |

uring-sqpoll: 285807 paired messages; decoded before epoll-busy on 72.6 % (uncorrected).

## lab-replay1x-epoll-spin-vs-uring-spin: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-replay1x-epoll-spin-vs-uring-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 292591 | 245668 | 1.19 | 1738483556 | 5941.69 | 4829121 | 4828439 | 0.0 | 31.0 | false | 0 | 0 |
| uring-spin | 292591 | 245421 | 1.19 | 482318 | 1.65 | 1340 | 309978903 | 0.0 | 0.2 | false | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8211 | 0 |
| uring-spin | 0 | 8211 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 73066446 | 0 | 0 | 0 | 0.000 |
| uring-spin | 73066446 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin (194434 of 285807 in 1-msg reads) | 194434 | 6.15 | 6.01 | 9.24 | 14.28 | 22.65 | 91.72 |
| uring-spin (193926 of 285807 in 1-msg reads) | 193926 | 6.34 | 6.19 | 9.30 | 14.14 | 22.34 | 52.30 |

### Paired by message, relative to epoll-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 285807 | 0.22 | 0.22 | 6.04 | 16.92 | 32.88 | 221.24 |
| uring-spin Δuser, even seq | 142903 | 2.45 | 2.39 | 7.70 | 25.50 | 36.40 | 219.04 |
| uring-spin Δuser, odd seq | 142904 | -2.01 | -1.96 | 4.59 | 10.17 | 23.24 | 219.06 |
| uring-spin Δrx | 285807 | 0.04 | 0.03 | 6.57 | 20.71 | 36.01 | 218.94 |
| uring-spin Δstack (1-msg reads) | 162083 | 0.21 | 0.23 | 2.56 | 5.19 | 9.35 | 44.90 |

uring-spin: 285807 paired messages; decoded before epoll-spin on 50.0 % (uncorrected).

## lab-replay1x-uring-napi-vs-epoll-busy: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=241579 clocksource=tsc

### Loop counters (lab-replay1x-uring-napi-vs-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-napi | 292591 | 223581 | 1.31 | 3742209 | 12.79 | 10395 | 10395 | 0.0 | 99.4 | true | 0 | 0 |
| epoll-busy | 292591 | 212762 | 1.38 | 2655386 | 9.08 | 7376 | 6785 | 0.0 | 96.0 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-napi | 0 | 8216 | 2 |
| epoll-busy | 0 | 8209 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-napi | 73066446 | 0 | 0 | 0 | 0.000 |
| epoll-busy | 73066446 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-napi (161629 of 285807 in 1-msg reads) | 161629 | 6.73 | 4.60 | 13.45 | 21.71 | 26.73 | 227.30 |
| epoll-busy (144809 of 285807 in 1-msg reads) | 144809 | 11.28 | 12.61 | 21.62 | 27.91 | 40.38 | 61.85 |

### Paired by message, relative to uring-napi

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy Δuser (write-order corrected)** | 285807 | 5.13 | 2.82 | 20.62 | 37.89 | 48.75 | 231.76 |
| epoll-busy Δuser, even seq | 142903 | 7.31 | 4.68 | 22.52 | 41.81 | 52.03 | 233.62 |
| epoll-busy Δuser, odd seq | 142904 | 2.95 | 0.96 | 18.73 | 32.72 | 45.34 | 219.80 |
| epoll-busy Δrx | 285807 | 2.29 | 1.46 | 12.81 | 26.32 | 36.81 | 196.34 |
| epoll-busy Δstack (1-msg reads) | 110621 | 5.35 | 1.87 | 17.17 | 22.79 | 36.19 | 56.81 |

epoll-busy: 285807 paired messages; decoded before uring-napi on 31.8 % (uncorrected).

## lab-replay1x-uring-spin-vs-epoll-spin: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (lab-replay1x-uring-spin-vs-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 292591 | 242735 | 1.21 | 471191 | 1.61 | 1309 | 306993183 | 0.0 | 0.2 | false | 0 | 0 |
| epoll-spin | 292591 | 242298 | 1.21 | 1788125178 | 6111.35 | 4967014 | 4966341 | 0.0 | 31.9 | false | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8210 | 0 |
| epoll-spin | 0 | 8210 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 73066446 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 73066446 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-spin (190309 of 285807 in 1-msg reads) | 190309 | 10.82 | 6.57 | 23.29 | 46.11 | 66.44 | 108.48 |
| epoll-spin (190027 of 285807 in 1-msg reads) | 190027 | 10.60 | 6.43 | 22.98 | 46.10 | 66.06 | 105.74 |

### Paired by message, relative to uring-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser (write-order corrected)** | 285807 | -0.17 | -0.18 | 7.74 | 25.47 | 40.58 | 106.79 |
| epoll-spin Δuser, even seq | 142903 | -2.61 | -2.35 | 5.77 | 19.52 | 35.01 | 92.84 |
| epoll-spin Δuser, odd seq | 142904 | 2.28 | 2.00 | 9.61 | 31.71 | 43.45 | 108.96 |
| epoll-spin Δrx | 285807 | -0.18 | 0.03 | 3.36 | 34.16 | 60.40 | 117.66 |
| epoll-spin Δstack (1-msg reads) | 158203 | -0.08 | -0.17 | 2.46 | 17.08 | 35.17 | 79.21 |

epoll-spin: 285807 paired messages; decoded before uring-spin on 49.4 % (uncorrected).

## lab-replay1x-uring-sqpoll-vs-epoll-busy: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=236759 clocksource=tsc

### Loop counters (lab-replay1x-uring-sqpoll-vs-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 292605 | 231751 | 1.26 | 0 | 0.00 | 0 | 269184017 | 0.0 | 0.0 | true | 0 | 0 |
| epoll-busy | 292607 | 212242 | 1.38 | 2655885 | 9.08 | 7377 | 6788 | 0.0 | 95.8 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8214 | 6 |
| epoll-busy | 0 | 8211 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 73069981 | 0 | 0 | 0 | 0.000 |
| epoll-busy | 73070509 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-sqpoll (176279 of 285821 in 1-msg reads) | 176279 | 5.45 | 4.31 | 6.89 | 26.32 | 41.73 | 88.76 |
| epoll-busy (142882 of 285823 in 1-msg reads) | 142882 | 11.99 | 12.72 | 23.35 | 29.41 | 42.54 | 70.37 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy Δuser (write-order corrected)** | 285821 | 4.02 | 8.25 | 24.77 | 39.75 | 49.25 | 376.85 |
| epoll-busy Δuser, even seq | 142910 | 5.93 | 10.18 | 27.19 | 43.66 | 53.20 | 308.96 |
| epoll-busy Δuser, odd seq | 142911 | 2.11 | 6.32 | 22.60 | 33.23 | 44.80 | 374.92 |
| epoll-busy Δrx | 285821 | 1.96 | 4.21 | 15.28 | 27.36 | 37.41 | 343.54 |
| epoll-busy Δstack (1-msg reads) | 113302 | 8.46 | 9.13 | 19.10 | 24.45 | 37.32 | 65.27 |

epoll-busy: 285821 paired messages; decoded before uring-sqpoll on 28.5 % (uncorrected).

## live-aa-epoll-busy: binance (A/A control), 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=4066 clocksource=tsc

### Loop counters (live-aa-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 132030 | 59255 | 2.23 | 2004554 | 15.18 | 6682 | 6484 | 0.0 | 95.0 | true | 0 | 0 |
| epoll-busy | 132029 | 59007 | 2.24 | 2003728 | 15.18 | 6679 | 6482 | 0.0 | 95.1 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8211 | 0 |
| epoll-busy | 1 | 8216 | 1 |
| epoll-busy | 2 | 8210 | 0 |
| epoll-busy | 0 | 8214 | 1 |
| epoll-busy | 1 | 8209 | 0 |
| epoll-busy | 2 | 8215 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 28218217 | 194905 | 0 | 0 | 0.691 |
| epoll-busy | 28217342 | 188506 | 0 | 0 | 0.668 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy #0 (37436 of 129598 in 1-msg reads) | 37436 | 13.64 | 13.27 | 16.11 | 26.05 | 29.74 | 43.26 |
| epoll-busy #1 (37540 of 129597 in 1-msg reads) | 37540 | 13.18 | 13.04 | 14.87 | 23.83 | 28.45 | 40.77 |

### Binance event time → kernel rx (`rx_sw − E`)

Includes Binance's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| btcusdt@trade epoll-busy #0 | 4890 | 42525.97 | 41507.39 | 43557.79 | 78243.21 | 85129.58 | 87905.58 |
| btcusdt@trade epoll-busy #1 | 4890 | 42247.51 | 41151.52 | 43390.96 | 78284.62 | 85251.03 | 87960.71 |
| ethusdt@trade epoll-busy #0 | 5403 | 44339.72 | 41728.94 | 42878.25 | 90058.23 | 90110.58 | 90112.58 |
| ethusdt@trade epoll-busy #1 | 5403 | 44573.76 | 41559.02 | 42712.64 | 90113.72 | 90165.71 | 90167.71 |
| solusdt@trade epoll-busy #0 | 811 | 41251.56 | 41115.62 | 42245.98 | 67041.41 | 67044.41 | 67044.41 |
| solusdt@trade epoll-busy #1 | 811 | 41058.52 | 40892.97 | 42092.14 | 67167.40 | 67170.40 | 67170.40 |
| btcusdt@bookTicker epoll-busy #0 | 61002 | 40904.56 | 40686.95 | 42047.61 | 52103.53 | 60502.67 | 66868.36 |
| btcusdt@bookTicker epoll-busy #1 | 61002 | 41608.48 | 41261.54 | 42727.65 | 52910.73 | 64437.74 | 67370.17 |
| ethusdt@bookTicker epoll-busy #0 | 55767 | 41013.99 | 41093.08 | 41983.36 | 52179.57 | 60747.98 | 64029.36 |
| ethusdt@bookTicker epoll-busy #1 | 55766 | 41733.94 | 41712.74 | 42618.37 | 53947.14 | 64507.59 | 65240.27 |
| btcusdt@aggTrade epoll-busy #0 | 1725 | 40757.28 | 40647.93 | 42172.85 | 49224.40 | 50224.40 | 50224.40 |
| btcusdt@aggTrade epoll-busy #1 | 1725 | 41243.55 | 41189.46 | 42541.32 | 49492.63 | 50492.63 | 50492.63 |

### Paired by message, relative to epoll-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own Binance connection, so Δuser also contains Binance's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy #1 Δuser** | 129597 | 645.65 | 572.16 | 775.08 | 5096.20 | 12037.54 | 25801.71 |
| epoll-busy #1 Δrx | 129597 | 645.65 | 572.20 | 774.31 | 5096.12 | 12037.02 | 25801.18 |
| epoll-busy #1 Δstack (1-msg reads) | 36913 | -0.47 | -0.30 | 2.26 | 10.89 | 16.32 | 30.38 |

epoll-busy #1: 129597 paired messages; decoded before epoll-busy on 8.1 % (uncorrected).

## live-epoll-busy-vs-uring-napi: binance, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=47225 clocksource=tsc

### Loop counters (live-epoll-busy-vs-uring-napi)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 249676 | 86413 | 2.89 | 2042656 | 8.18 | 6809 | 6521 | 0.0 | 95.2 | true | 0 | 0 |
| uring-napi | 249683 | 91369 | 2.73 | 2944110 | 11.79 | 9814 | 9814 | 0.0 | 99.4 | true | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8213 | 4 |
| epoll-busy | 1 | 8211 | 0 |
| epoll-busy | 2 | 8211 | 0 |
| uring-napi | 0 | 8213 | 0 |
| uring-napi | 1 | 8214 | 4 |
| uring-napi | 2 | 8214 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 52911253 | 959682 | 0 | 0 | 1.814 |
| uring-napi | 52894260 | 1380857 | 0 | 0 | 2.611 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy (49398 of 241457 in 1-msg reads) | 49398 | 13.57 | 13.23 | 16.18 | 25.36 | 30.38 | 686.16 |
| uring-napi (50162 of 241456 in 1-msg reads) | 50162 | 8.71 | 6.50 | 13.88 | 17.83 | 26.58 | 46.71 |

### Binance event time → kernel rx (`rx_sw − E`)

Includes Binance's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| btcusdt@trade epoll-busy | 10187 | 39036.49 | 38510.52 | 39836.02 | 71348.11 | 72496.52 | 76923.97 |
| btcusdt@trade uring-napi | 10187 | 39318.96 | 38704.29 | 40107.57 | 70481.91 | 71309.10 | 75891.10 |
| ethusdt@trade epoll-busy | 7920 | 38609.40 | 38335.80 | 39926.87 | 40655.51 | 72032.52 | 72034.52 |
| ethusdt@trade uring-napi | 7920 | 38907.86 | 38706.57 | 40234.96 | 40935.04 | 70949.10 | 76665.79 |
| solusdt@trade epoll-busy | 1732 | 38498.33 | 38110.01 | 39517.67 | 40399.69 | 75284.52 | 75285.52 |
| solusdt@trade uring-napi | 1732 | 38791.78 | 38492.09 | 39851.00 | 40462.30 | 74209.10 | 74210.10 |
| btcusdt@bookTicker epoll-busy | 123801 | 40415.34 | 38426.43 | 45066.40 | 73580.80 | 112668.15 | 118059.55 |
| btcusdt@bookTicker uring-napi | 123801 | 39724.32 | 38841.63 | 42543.81 | 55007.06 | 72900.77 | 82555.98 |
| ethusdt@bookTicker epoll-busy | 91569 | 41706.09 | 38756.68 | 50379.43 | 87044.83 | 115004.15 | 117059.55 |
| ethusdt@bookTicker uring-napi | 91568 | 40440.43 | 39139.76 | 44818.29 | 68334.60 | 73926.68 | 82555.98 |
| btcusdt@aggTrade epoll-busy | 6248 | 40352.97 | 38645.72 | 43026.08 | 79329.62 | 85578.15 | 85578.15 |
| btcusdt@aggTrade uring-napi | 6248 | 40289.29 | 38618.71 | 43131.07 | 80212.33 | 87357.91 | 87357.91 |

### Paired by message, relative to epoll-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own Binance connection, so Δuser also contains Binance's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-napi Δuser** | 241456 | -816.80 | 438.93 | 655.82 | 3599.48 | 15597.79 | 39817.88 |
| uring-napi Δrx | 241456 | -812.79 | 443.32 | 660.01 | 3609.46 | 15602.84 | 39825.23 |
| uring-napi Δstack (1-msg reads) | 47890 | -4.88 | -7.02 | 0.69 | 7.87 | 13.93 | 24.57 |

uring-napi: 241456 paired messages; decoded before epoll-busy on 20.2 % (uncorrected).

## live-epoll-busy-vs-uring-sqpoll: binance, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=136280 clocksource=tsc

### Loop counters (live-epoll-busy-vs-uring-sqpoll)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 186570 | 79358 | 2.35 | 2034000 | 10.90 | 6780 | 6515 | 0.0 | 95.0 | true | 0 | 0 |
| uring-sqpoll | 186570 | 80642 | 2.31 | 1 | 0.00 | 0 | 155464895 | 0.0 | 0.0 | true | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8209 | 0 |
| epoll-busy | 1 | 8214 | 6 |
| epoll-busy | 2 | 8216 | 6 |
| uring-sqpoll | 0 | 8216 | 6 |
| uring-sqpoll | 1 | 8214 | 6 |
| uring-sqpoll | 2 | 8216 | 6 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 39766305 | 198034 | 0 | 0 | 0.498 |
| uring-sqpoll | 39783098 | 164813 | 0 | 0 | 0.414 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy (47199 of 181558 in 1-msg reads) | 47199 | 13.68 | 13.53 | 15.48 | 24.63 | 30.24 | 384.85 |
| uring-sqpoll (47599 of 181558 in 1-msg reads) | 47599 | 5.47 | 4.99 | 7.15 | 11.23 | 15.13 | 34.73 |

### Binance event time → kernel rx (`rx_sw − E`)

Includes Binance's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| btcusdt@trade epoll-busy | 10905 | 37635.57 | 36506.40 | 38548.40 | 86224.87 | 89167.49 | 89172.49 |
| btcusdt@trade uring-sqpoll | 10905 | 37662.07 | 37042.25 | 39016.17 | 40456.82 | 88269.84 | 88274.84 |
| ethusdt@trade epoll-busy | 5299 | 36561.69 | 36524.37 | 37502.38 | 39489.52 | 39842.50 | 40030.93 |
| ethusdt@trade uring-sqpoll | 5299 | 37346.85 | 37040.39 | 38408.97 | 40517.35 | 52388.45 | 52390.45 |
| solusdt@trade epoll-busy | 1132 | 36485.91 | 36106.13 | 38193.47 | 42248.18 | 42295.18 | 42296.18 |
| solusdt@trade uring-sqpoll | 1132 | 37086.96 | 36779.91 | 38438.09 | 40101.78 | 40448.98 | 40458.17 |
| btcusdt@bookTicker epoll-busy | 98694 | 37287.02 | 36896.40 | 39115.05 | 43700.71 | 56991.05 | 63754.13 |
| btcusdt@bookTicker uring-sqpoll | 98694 | 37061.87 | 36684.15 | 39065.71 | 43111.94 | 51484.44 | 63603.84 |
| ethusdt@bookTicker epoll-busy | 61794 | 37096.87 | 36790.78 | 38525.71 | 41837.77 | 50066.61 | 63242.91 |
| ethusdt@bookTicker uring-sqpoll | 61794 | 36900.61 | 36531.99 | 38518.40 | 43014.26 | 51270.78 | 63060.17 |
| btcusdt@aggTrade epoll-busy | 3734 | 37773.91 | 37025.70 | 40244.44 | 48267.82 | 61896.98 | 63434.47 |
| btcusdt@aggTrade uring-sqpoll | 3734 | 37473.86 | 37155.14 | 38688.16 | 43523.41 | 73813.69 | 76078.94 |

### Paired by message, relative to epoll-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own Binance connection, so Δuser also contains Binance's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser** | 181558 | -173.25 | -204.91 | 202.44 | 1910.37 | 5922.26 | 32358.58 |
| uring-sqpoll Δrx | 181558 | -167.19 | -198.17 | 205.55 | 1917.57 | 5930.92 | 32359.33 |
| uring-sqpoll Δstack (1-msg reads) | 46255 | -8.21 | -8.34 | -5.88 | 0.03 | 3.08 | 21.08 |

uring-sqpoll: 181558 paired messages; decoded before epoll-busy on 78.9 % (uncorrected).

## live-epoll-spin-vs-uring-spin: binance, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (live-epoll-spin-vs-uring-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 207547 | 83145 | 2.50 | 1439098203 | 6933.84 | 4796994 | 4796717 | 0.0 | 30.9 | false | 0 | 0 |
| uring-spin | 207547 | 82876 | 2.50 | 165251 | 0.80 | 551 | 192745780 | 0.0 | 0.1 | false | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8209 | 0 |
| epoll-spin | 1 | 8211 | 0 |
| epoll-spin | 2 | 8213 | 0 |
| uring-spin | 0 | 8209 | 0 |
| uring-spin | 1 | 8211 | 0 |
| uring-spin | 2 | 8213 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 44031876 | 162488 | 0 | 0 | 0.369 |
| uring-spin | 44021397 | 168745 | 0 | 0 | 0.383 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-spin (50886 of 206197 in 1-msg reads) | 50886 | 4.36 | 4.13 | 5.01 | 8.63 | 16.48 | 53.14 |
| uring-spin (50426 of 206197 in 1-msg reads) | 50426 | 4.95 | 4.50 | 6.20 | 11.80 | 20.93 | 41.04 |

### Binance event time → kernel rx (`rx_sw − E`)

Includes Binance's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| btcusdt@trade epoll-spin | 9423 | 40820.00 | 41178.49 | 43015.89 | 43419.49 | 43481.63 | 43483.63 |
| btcusdt@trade uring-spin | 9423 | 40775.81 | 41083.81 | 43059.27 | 45086.60 | 66016.90 | 68508.90 |
| ethusdt@trade epoll-spin | 10397 | 40826.77 | 41415.00 | 43023.86 | 44440.86 | 44457.86 | 44459.86 |
| ethusdt@trade uring-spin | 10397 | 40786.03 | 41356.31 | 43063.48 | 43689.71 | 52856.43 | 75070.74 |
| solusdt@trade epoll-spin | 1853 | 40692.91 | 41189.22 | 42739.42 | 43523.77 | 45010.86 | 50167.82 |
| solusdt@trade uring-spin | 1853 | 40753.54 | 41129.52 | 42628.78 | 45017.60 | 57085.90 | 57086.90 |
| btcusdt@bookTicker epoll-spin | 95379 | 41572.87 | 41896.89 | 43655.71 | 45375.02 | 56240.55 | 72129.70 |
| btcusdt@bookTicker uring-spin | 95379 | 41080.05 | 41347.63 | 43137.41 | 45906.86 | 65477.43 | 110114.57 |
| ethusdt@bookTicker epoll-spin | 84091 | 41338.06 | 41887.35 | 43485.96 | 44895.91 | 47903.31 | 67361.64 |
| ethusdt@bookTicker uring-spin | 84091 | 40852.56 | 41360.86 | 42973.74 | 45365.61 | 50741.82 | 102114.57 |
| btcusdt@aggTrade epoll-spin | 5054 | 40317.50 | 40566.12 | 42451.96 | 43238.42 | 80912.72 | 85912.72 |
| btcusdt@aggTrade uring-spin | 5054 | 40352.03 | 40577.20 | 42482.92 | 43963.58 | 81844.05 | 86844.05 |

### Paired by message, relative to epoll-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own Binance connection, so Δuser also contains Binance's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-spin Δuser** | 206197 | -424.64 | -543.82 | -29.31 | 1404.99 | 16554.73 | 68188.56 |
| uring-spin Δrx | 206197 | -428.26 | -544.21 | -29.16 | 1403.94 | 14348.69 | 67393.23 |
| uring-spin Δstack (1-msg reads) | 49699 | 0.59 | 0.43 | 1.39 | 5.68 | 14.21 | 35.31 |

uring-spin: 206197 paired messages; decoded before epoll-spin on 90.4 % (uncorrected).

## live-uring-napi-vs-epoll-busy: binance, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=34277 clocksource=tsc

### Loop counters (live-uring-napi-vs-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-napi | 166743 | 69829 | 2.39 | 2911279 | 17.46 | 9704 | 9704 | 0.0 | 99.4 | true | 0 | 0 |
| epoll-busy | 166756 | 68123 | 2.45 | 2015084 | 12.08 | 6717 | 6490 | 0.0 | 95.1 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-napi | 0 | 8212 | 2 |
| uring-napi | 1 | 8212 | 2 |
| uring-napi | 2 | 8210 | 0 |
| epoll-busy | 0 | 8213 | 0 |
| epoll-busy | 1 | 8213 | 0 |
| epoll-busy | 2 | 8215 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-napi | 35489391 | 77096 | 0 | 0 | 0.217 |
| epoll-busy | 35477497 | 83372 | 0 | 0 | 0.235 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-napi (41445 of 161827 in 1-msg reads) | 41445 | 9.58 | 7.82 | 14.62 | 25.49 | 29.18 | 39.85 |
| epoll-busy (41034 of 161827 in 1-msg reads) | 41034 | 13.85 | 13.45 | 16.55 | 26.60 | 30.76 | 58.05 |

### Binance event time → kernel rx (`rx_sw − E`)

Includes Binance's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| btcusdt@trade uring-napi | 8506 | 42902.42 | 42761.61 | 44491.23 | 51281.97 | 56645.04 | 56647.04 |
| btcusdt@trade epoll-busy | 8506 | 42810.80 | 42652.86 | 44545.22 | 50074.59 | 55409.03 | 55411.03 |
| ethusdt@trade uring-napi | 5629 | 42885.00 | 42349.19 | 43948.00 | 68620.51 | 68717.44 | 68718.44 |
| ethusdt@trade epoll-busy | 5629 | 42851.26 | 42300.42 | 43932.51 | 67448.95 | 67455.95 | 67457.08 |
| solusdt@trade uring-napi | 1039 | 42454.88 | 42462.94 | 44285.19 | 45739.17 | 45743.17 | 45745.17 |
| solusdt@trade epoll-busy | 1039 | 42445.10 | 42405.01 | 44234.14 | 47981.58 | 47985.58 | 47989.58 |
| btcusdt@bookTicker uring-napi | 76171 | 42252.91 | 41923.14 | 43952.22 | 52078.59 | 58672.65 | 69817.98 |
| btcusdt@bookTicker epoll-busy | 76171 | 42284.54 | 41921.79 | 43884.51 | 50909.21 | 80935.27 | 88935.27 |
| ethusdt@bookTicker uring-napi | 67996 | 42313.16 | 41924.80 | 43961.86 | 53891.59 | 58684.79 | 68737.53 |
| ethusdt@bookTicker epoll-busy | 67996 | 42088.66 | 41883.56 | 43751.86 | 49936.48 | 60107.93 | 88935.27 |
| btcusdt@aggTrade uring-napi | 2486 | 45245.80 | 43211.90 | 49334.99 | 78379.51 | 81673.94 | 81673.94 |
| btcusdt@aggTrade epoll-busy | 2486 | 44016.17 | 42964.02 | 46830.64 | 61590.18 | 85505.01 | 90692.29 |

### Paired by message, relative to uring-napi

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own Binance connection, so Δuser also contains Binance's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy Δuser** | 161827 | -100.45 | -32.45 | 169.45 | 1570.78 | 39031.18 | 46236.60 |
| epoll-busy Δrx | 161827 | -104.38 | -33.21 | 165.67 | 1561.37 | 38856.27 | 46104.44 |
| epoll-busy Δstack (1-msg reads) | 40146 | 4.28 | 6.06 | 8.90 | 18.68 | 23.50 | 49.64 |

epoll-busy: 161827 paired messages; decoded before uring-napi on 58.2 % (uncorrected).

## live-uring-spin-vs-epoll-spin: binance, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (live-uring-spin-vs-epoll-spin)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 276197 | 101617 | 2.72 | 202373 | 0.73 | 675 | 194538064 | 0.0 | 0.1 | false | 0 | 0 |
| epoll-spin | 276199 | 97923 | 2.82 | 1490534549 | 5396.60 | 4968448 | 4968122 | 0.0 | 31.8 | false | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8213 | 0 |
| uring-spin | 1 | 8215 | 1 |
| uring-spin | 2 | 8211 | 0 |
| epoll-spin | 0 | 8212 | 1 |
| epoll-spin | 1 | 8215 | 1 |
| epoll-spin | 2 | 8211 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 58581859 | 662594 | 0 | 0 | 1.131 |
| epoll-spin | 58545626 | 428164 | 0 | 0 | 0.731 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-spin (56747 of 267539 in 1-msg reads) | 56747 | 5.05 | 4.50 | 6.21 | 18.33 | 21.14 | 38.97 |
| epoll-spin (54775 of 267539 in 1-msg reads) | 54775 | 4.47 | 4.19 | 5.09 | 10.54 | 19.13 | 43.41 |

### Binance event time → kernel rx (`rx_sw − E`)

Includes Binance's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| btcusdt@trade uring-spin | 10654 | 36562.90 | 36340.02 | 37179.36 | 38354.22 | 72824.06 | 72829.06 |
| btcusdt@trade epoll-spin | 10654 | 36691.57 | 36553.66 | 37067.28 | 37718.85 | 73287.85 | 73290.85 |
| ethusdt@trade uring-spin | 9128 | 36569.13 | 36342.56 | 36926.24 | 37591.22 | 72530.06 | 76553.45 |
| ethusdt@trade epoll-spin | 9128 | 36721.36 | 36431.98 | 37095.85 | 37786.15 | 72965.85 | 77069.71 |
| solusdt@trade uring-spin | 1857 | 36770.59 | 36205.27 | 36783.11 | 72595.06 | 72604.06 | 72606.06 |
| solusdt@trade epoll-spin | 1857 | 36933.39 | 36327.51 | 36973.79 | 73070.85 | 73074.85 | 73075.85 |
| btcusdt@bookTicker uring-spin | 134960 | 37209.02 | 36127.37 | 37490.25 | 72237.77 | 86078.01 | 96071.56 |
| btcusdt@bookTicker epoll-spin | 134960 | 38223.08 | 36698.60 | 38493.47 | 77121.99 | 115211.95 | 123343.07 |
| ethusdt@bookTicker uring-spin | 104753 | 37535.88 | 36072.55 | 39335.92 | 73237.77 | 90128.91 | 96071.56 |
| ethusdt@bookTicker epoll-spin | 104753 | 39007.64 | 36648.63 | 40726.68 | 85959.09 | 112548.86 | 123343.07 |
| btcusdt@aggTrade uring-spin | 6187 | 39007.74 | 37027.73 | 41188.20 | 77489.54 | 109934.68 | 110934.68 |
| btcusdt@aggTrade epoll-spin | 6187 | 37580.64 | 36671.75 | 39299.24 | 59016.51 | 79442.30 | 97326.60 |

### Paired by message, relative to uring-spin

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own Binance connection, so Δuser also contains Binance's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser** | 267539 | 1067.43 | 575.46 | 1192.85 | 31002.04 | 71988.74 | 79678.40 |
| epoll-spin Δrx | 267539 | 1066.97 | 575.76 | 1192.14 | 30987.26 | 71977.59 | 79688.39 |
| epoll-spin Δstack (1-msg reads) | 53705 | -0.59 | -0.37 | 0.14 | 3.57 | 14.37 | 28.43 |

epoll-spin: 267539 paired messages; decoded before uring-spin on 10.6 % (uncorrected).

## live-uring-sqpoll-vs-epoll-busy: binance, 300 s (first 10 s dropped), 3 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=100 mlockall=true BusyPollRxPackets during run=67770 clocksource=tsc

### Loop counters (live-uring-sqpoll-vs-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 175230 | 73576 | 2.38 | 1 | 0.00 | 0 | 166108276 | 0.0 | 0.0 | true | 0 | 0 |
| epoll-busy | 175230 | 71001 | 2.47 | 2022507 | 11.54 | 6742 | 6505 | 0.0 | 94.9 | true | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8211 | 6 |
| uring-sqpoll | 1 | 8215 | 6 |
| uring-sqpoll | 2 | 8215 | 6 |
| epoll-busy | 0 | 8211 | 6 |
| epoll-busy | 1 | 8210 | 0 |
| epoll-busy | 2 | 8214 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 37282044 | 117306 | 0 | 0 | 0.315 |
| epoll-busy | 37270017 | 186190 | 0 | 0 | 0.500 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-sqpoll (43686 of 172489 in 1-msg reads) | 43686 | 5.16 | 4.71 | 6.77 | 10.49 | 13.44 | 18.13 |
| epoll-busy (43666 of 172489 in 1-msg reads) | 43666 | 14.11 | 13.46 | 17.72 | 27.65 | 33.90 | 50.98 |

### Binance event time → kernel rx (`rx_sw − E`)

Includes Binance's publish delay, the Azure→AWS path and the clock offset; context only, not a loop comparison.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| btcusdt@trade uring-sqpoll | 7486 | 37988.92 | 36992.22 | 38743.71 | 69368.52 | 73429.76 | 73434.76 |
| btcusdt@trade epoll-busy | 7486 | 37631.77 | 36678.45 | 38131.84 | 69238.46 | 73370.65 | 73372.65 |
| ethusdt@trade uring-sqpoll | 7484 | 37260.48 | 36752.12 | 38112.84 | 55314.62 | 62428.06 | 62682.06 |
| ethusdt@trade epoll-busy | 7484 | 37070.67 | 36489.20 | 37840.32 | 58068.58 | 65840.71 | 67545.31 |
| solusdt@trade uring-sqpoll | 1311 | 37286.78 | 37071.06 | 38405.98 | 49617.98 | 56415.43 | 56417.43 |
| solusdt@trade epoll-busy | 1311 | 36915.50 | 36840.42 | 37666.98 | 48964.62 | 56280.46 | 56280.46 |
| btcusdt@bookTicker uring-sqpoll | 72237 | 37383.87 | 37105.32 | 38468.53 | 44180.90 | 56337.96 | 63578.31 |
| btcusdt@bookTicker epoll-busy | 72237 | 37580.14 | 37155.15 | 38616.54 | 46741.43 | 63377.69 | 71222.36 |
| ethusdt@bookTicker uring-sqpoll | 81504 | 37441.03 | 37185.76 | 38400.89 | 45890.27 | 57548.41 | 60441.49 |
| ethusdt@bookTicker epoll-busy | 81504 | 37638.70 | 37249.94 | 38522.56 | 50222.36 | 63993.25 | 95222.36 |
| btcusdt@aggTrade uring-sqpoll | 2467 | 42142.75 | 36981.63 | 58845.11 | 89947.16 | 91061.60 | 92166.70 |
| btcusdt@aggTrade epoll-busy | 2467 | 52064.71 | 37760.62 | 83463.85 | 195820.29 | 217820.29 | 227820.29 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). Live: each variant has its own Binance connection, so Δuser also contains Binance's per-connection send skew; use it as confirmation of the lab result, not as a verdict.
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy Δuser** | 172489 | 299.05 | 57.52 | 328.21 | 5854.21 | 37779.09 | 148244.37 |
| epoll-busy Δrx | 172489 | 291.53 | 50.64 | 320.36 | 5853.20 | 37777.86 | 148237.98 |
| epoll-busy Δstack (1-msg reads) | 42698 | 8.97 | 8.54 | 11.65 | 22.29 | 27.49 | 46.27 |

epoll-busy: 172489 paired messages; decoded before uring-sqpoll on 32.2 % (uncorrected).

## Ping-pong round trip (client send → echo decoded)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy @ 0/s | 516644 | 56.06 | 53.94 | 78.06 | 85.81 | 90.98 | 381.42 |
| epoll-busy @ 100/s | 2879 | 71.05 | 73.56 | 80.92 | 88.59 | 118.21 | 191.44 |
| epoll-busy @ 1000/s | 27236 | 68.20 | 70.99 | 77.48 | 84.21 | 90.55 | 569.23 |
| epoll-busy @ 10000/s | 129963 | 66.81 | 70.15 | 76.09 | 81.32 | 87.21 | 659.86 |
| epoll-spin @ 0/s | 538678 | 53.76 | 56.70 | 65.60 | 70.94 | 77.39 | 1022.99 |
| epoll-spin @ 100/s | 2899 | 64.41 | 66.47 | 73.62 | 85.63 | 101.51 | 237.08 |
| epoll-spin @ 1000/s | 28998 | 58.82 | 62.24 | 67.73 | 73.75 | 81.42 | 199.64 |
| epoll-spin @ 10000/s | 289709 | 57.73 | 61.34 | 66.81 | 72.17 | 78.35 | 428.29 |
| uring-napi @ 0/s | 505014 | 57.34 | 54.10 | 79.57 | 87.16 | 95.86 | 123.69 |
| uring-napi @ 100/s | 2882 | 59.88 | 59.78 | 77.96 | 86.73 | 106.05 | 117.36 |
| uring-napi @ 1000/s | 28419 | 57.71 | 56.96 | 73.62 | 79.38 | 87.26 | 117.09 |
| uring-napi @ 10000/s | 176828 | 56.46 | 56.22 | 73.54 | 79.98 | 88.44 | 139.03 |
| uring-spin @ 0/s | 494968 | 58.51 | 60.46 | 69.26 | 74.85 | 87.55 | 640.14 |
| uring-spin @ 100/s | 2899 | 65.68 | 67.36 | 74.44 | 88.06 | 99.89 | 790.55 |
| uring-spin @ 1000/s | 28999 | 59.02 | 62.66 | 67.91 | 71.47 | 79.17 | 296.37 |
| uring-spin @ 10000/s | 289906 | 58.11 | 61.76 | 67.14 | 72.66 | 79.91 | 874.86 |
| uring-sqpoll @ 0/s | 549342 | 52.72 | 53.72 | 61.61 | 66.97 | 72.77 | 805.76 |
| uring-sqpoll @ 100/s | 2899 | 58.03 | 60.36 | 63.60 | 69.89 | 88.23 | 92.88 |
| uring-sqpoll @ 1000/s | 28998 | 54.66 | 57.77 | 60.42 | 64.14 | 71.13 | 420.25 |
| uring-sqpoll @ 10000/s | 289929 | 54.28 | 57.37 | 59.90 | 64.37 | 70.56 | 124.02 |

