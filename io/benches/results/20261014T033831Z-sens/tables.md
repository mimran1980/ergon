## Summary: paired runs (µs; Δ = B − A, negative means B decoded first)

| run | A | B | paired msgs | Δuser p50 | Δuser p90 | Δuser p99 | Δrx p50 | Δstack p50 | Δstack p99 | A syscalls/s | B syscalls/s | A kernel CPU % | B kernel CPU % | shared RX queue |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| sens-lab-replay1x-aa-epoll-busy | epoll-busy | epoll-busy | 154482 | -0.13 | 7.62 | 28.62 | -0.22 | -0.02 | 14.54 | 4545 | 4547 | 98.3 | 98.3 | no |
| sens-lab-replay1x-epoll-busy-vs-uring-napi | epoll-busy | uring-napi | 154482 | -24.40 | 2.17 | 27.34 | -5.05 | -15.84 | 0.31 | 4548 | 5453 | 98.3 | 99.9 | no |
| sens-lab-replay1x-uring-napi-vs-epoll-busy | uring-napi | epoll-busy | 154482 | 3.22 | 11.61 | 22.62 | 0.03 | 3.93 | 18.93 | 5469 | 4589 | 99.9 | 98.3 | yes |

## sens-lab-replay1x-aa-epoll-busy: lab (A/A control), 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=200 wait_us=200 mlockall=true BusyPollRxPackets during run=108914 clocksource=tsc

### Loop counters (sens-lab-replay1x-aa-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 158917 | 121636 | 1.31 | 1636196 | 10.30 | 4545 | 4207 | 0.0 | 98.3 | true | 0 | 0 |
| epoll-busy | 158917 | 121853 | 1.30 | 1636861 | 10.30 | 4547 | 4208 | 0.0 | 98.3 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8216 | 1 |
| epoll-busy | 0 | 8210 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 39775708 | 0 | 0 | 0 | 0.000 |
| epoll-busy | 39775708 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy #0 (90176 of 154482 in 1-msg reads) | 90176 | 9.87 | 11.96 | 14.32 | 24.89 | 27.33 | 255.81 |
| epoll-busy #1 (90660 of 154482 in 1-msg reads) | 90660 | 9.70 | 11.95 | 13.81 | 24.34 | 28.76 | 48.96 |

### Paired by message, relative to epoll-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy #1 Δuser (write-order corrected)** | 154482 | -0.11 | -0.13 | 7.62 | 28.62 | 44.35 | 164.33 |
| epoll-busy #1 Δuser, even seq | 77241 | 1.92 | 1.41 | 11.54 | 32.20 | 46.64 | 162.68 |
| epoll-busy #1 Δuser, odd seq | 77241 | -2.15 | -1.68 | 4.84 | 23.53 | 41.95 | 162.78 |
| epoll-busy #1 Δrx | 154482 | -0.03 | -0.22 | 6.31 | 20.64 | 35.26 | 161.33 |
| epoll-busy #1 Δstack (1-msg reads) | 78230 | -0.16 | -0.02 | 1.26 | 14.54 | 20.82 | 39.22 |

epoll-busy #1: 154482 paired messages; decoded before epoll-busy on 51.2 % (uncorrected).

## sens-lab-replay1x-epoll-busy-vs-uring-napi: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=200 wait_us=200 mlockall=true BusyPollRxPackets during run=160160 clocksource=tsc

### Loop counters (sens-lab-replay1x-epoll-busy-vs-uring-napi)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-busy | 158917 | 122652 | 1.30 | 1637148 | 10.30 | 4548 | 4207 | 0.0 | 98.3 | true | 0 | 0 |
| uring-napi | 158917 | 128885 | 1.23 | 1963201 | 12.35 | 5453 | 5453 | 0.0 | 99.9 | true | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-busy | 0 | 8216 | 1 |
| uring-napi | 0 | 8210 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-busy | 39775708 | 0 | 0 | 0 | 0.000 |
| uring-napi | 39775708 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy (90003 of 154482 in 1-msg reads) | 90003 | 16.32 | 18.10 | 26.76 | 38.07 | 41.75 | 73.18 |
| uring-napi (98076 of 154482 in 1-msg reads) | 98076 | 3.79 | 3.69 | 4.05 | 7.15 | 24.52 | 45.41 |

### Paired by message, relative to epoll-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **uring-napi Δuser (write-order corrected)** | 154482 | -18.65 | -24.40 | 2.17 | 27.34 | 46.99 | 80.67 |
| uring-napi Δuser, even seq | 77241 | -15.53 | -20.24 | 3.67 | 29.09 | 48.85 | 76.54 |
| uring-napi Δuser, odd seq | 77241 | -21.77 | -28.56 | 1.14 | 25.43 | 45.91 | 76.51 |
| uring-napi Δrx | 154482 | -8.23 | -5.05 | 3.15 | 17.54 | 31.97 | 53.83 |
| uring-napi Δstack (1-msg reads) | 81649 | -13.38 | -15.84 | -0.49 | 0.31 | 20.16 | 41.64 |

uring-napi: 154482 paired messages; decoded before epoll-busy on 84.3 % (uncorrected).

## sens-lab-replay1x-uring-napi-vs-epoll-busy: lab, 360 s (first 10 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=200 wait_us=200 mlockall=true BusyPollRxPackets during run=193978 clocksource=tsc

### Loop counters (sens-lab-replay1x-uring-napi-vs-epoll-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-napi | 158917 | 133524 | 1.19 | 1968961 | 12.39 | 5469 | 5469 | 0.0 | 99.9 | true | 0 | 0 |
| epoll-busy | 158917 | 129282 | 1.23 | 1651906 | 10.39 | 4589 | 4230 | 0.0 | 98.3 | true | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-napi | 0 | 8214 | 2 |
| epoll-busy | 0 | 8214 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-napi | 39775708 | 0 | 0 | 0 | 0.000 |
| epoll-busy | 39775708 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| uring-napi (106325 of 154482 in 1-msg reads) | 106325 | 6.62 | 4.21 | 9.71 | 45.31 | 66.99 | 88.68 |
| epoll-busy (99698 of 154482 in 1-msg reads) | 99698 | 10.66 | 11.86 | 14.37 | 45.71 | 66.64 | 552.29 |

### Paired by message, relative to uring-napi

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| **epoll-busy Δuser (write-order corrected)** | 154482 | 2.59 | 3.22 | 11.61 | 22.62 | 35.64 | 543.88 |
| epoll-busy Δuser, even seq | 77241 | 5.26 | 5.00 | 14.69 | 26.82 | 38.64 | 157.13 |
| epoll-busy Δuser, odd seq | 77241 | -0.08 | 1.45 | 5.00 | 17.42 | 24.58 | 542.11 |
| epoll-busy Δrx | 154482 | -0.10 | 0.03 | 5.73 | 22.17 | 62.76 | 106.67 |
| epoll-busy Δstack (1-msg reads) | 84561 | 4.63 | 3.93 | 9.34 | 18.93 | 32.71 | 548.33 |

epoll-busy: 154482 paired messages; decoded before uring-napi on 30.1 % (uncorrected).

## Ping-pong round trip (client send → echo decoded)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| epoll-busy @ 1000/s | 27710 | 66.62 | 70.82 | 73.57 | 79.20 | 86.47 | 187.85 |
| epoll-busy @ 10000/s | 106057 | 66.79 | 70.87 | 73.01 | 78.56 | 85.60 | 868.70 |
| uring-napi @ 1000/s | 26692 | 53.05 | 56.22 | 58.55 | 63.27 | 69.73 | 664.81 |
| uring-napi @ 10000/s | 111617 | 52.55 | 55.84 | 58.12 | 62.41 | 68.50 | 121.48 |

