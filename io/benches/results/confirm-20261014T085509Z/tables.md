### Loop counters (pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 527357 | 527357 | 1.00 | 162846651 | 308.80 | 5428222 | 5393065 | 0.0 | 36.5 | false | 0 | 0 | 794.6 | 3815 / 54 / 4 | 0 | 0 |

### Loop counters (pp-epoll-spin-100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3000 | 3000 | 1.00 | 160616496 | 53538.83 | 5353883 | 5353683 | 0.0 | 28.0 | false | 0 | 0 | 68.3 | 2344 / 30 / 0 | 0 | 0 |

### Loop counters (pp-epoll-spin-1000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 29998 | 29998 | 1.00 | 161830940 | 5394.72 | 5394365 | 5392365 | 0.0 | 28.4 | false | 0 | 0 | 42.8 | 1541 / 25 / 0 | 0 | 0 |

### Loop counters (pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 568649 | 568649 | 1.00 | 33371210 | 58.69 | 1112374 | 1093419 | 98.3 | 84.9 | true | 0 | 0 | 841.3 | 32542 / 52 / 6 | 0 | 0 |

### Loop counters (pp-recv-busy-100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3000 | 3000 | 1.00 | 36802680 | 12267.56 | 1226756 | 1226656 | 100.0 | 82.0 | true | 0 | 0 | 68.1 | 5013 / 25 / 0 | 0 | 0 |

### Loop counters (pp-recv-busy-1000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 29988 | 29988 | 1.00 | 36899940 | 1230.49 | 1229998 | 1228998 | 99.9 | 82.3 | true | 0 | 0 | 64.2 | 7831 / 30 / 0 | 0 | 0 |

### Loop counters (pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 536849 | 536849 | 1.00 | 2147398 | 4.00 | 71580 | 51166071 | 0.0 | 10.0 | false | 0 | 0 | 65.7 | 5301 / 21 / 0 | 0 | 0 |

### Loop counters (pp-uring-spin-100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 3000 | 3000 | 1.00 | 12000 | 4.00 | 400 | 27709129 | 0.0 | 0.3 | false | 0 | 0 | 60.4 | 2650 / 25 / 0 | 0 | 0 |

### Loop counters (pp-uring-spin-1000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 29998 | 29998 | 1.00 | 119992 | 4.00 | 4000 | 29421551 | 0.0 | 0.8 | false | 0 | 0 | 792.2 | 1601 / 36 / 5 | 0 | 0 |

### Loop counters (pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 564872 | 564872 | 1.00 | 564874 | 1.00 | 18829 | 46810052 | 0.0 | 0.0 | true | 0 | 0 | 611.1 | 151 / 39 / 4 | 0 | 0 |

### Loop counters (pp-uring-sqpoll-100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3000 | 3000 | 1.00 | 3001 | 1.00 | 100 | 23110121 | 0.0 | 0.0 | true | 0 | 0 | 851.8 | 132 / 29 / 4 | 0 | 0 |

### Loop counters (pp-uring-sqpoll-1000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 30000 | 30000 | 1.00 | 30001 | 1.00 | 1000 | 24103373 | 0.0 | 0.0 | true | 0 | 0 | 63.2 | 127 / 21 / 0 | 0 | 0 |

## Ping-pong round trip (client send → echo decoded)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| curl-distro/8.5.0 @ 0/s | 369718 | 78.07 | 79.59 | 85.74 | 92.09 | 101.18 | 667.46 |
| curl-distro/8.5.0 @ 1000/s | 28990 | 73.15 | 75.15 | 80.30 | 86.76 | 93.71 | 248.95 |
| curl-source/8.23.0 @ 0/s | 380369 | 75.89 | 78.01 | 83.42 | 89.02 | 95.23 | 1257.94 |
| curl-source/8.23.0 @ 1000/s | 28998 | 71.88 | 73.89 | 78.68 | 85.43 | 93.03 | 470.79 |
| ergon/epoll-spin @ 0/s | 502849 | 57.51 | 59.51 | 67.08 | 72.66 | 81.23 | 914.70 |
| ergon/epoll-spin @ 1000/s | 29000 | 63.01 | 65.25 | 68.34 | 71.48 | 80.01 | 348.20 |
| ergon/recv-busy @ 0/s | 531114 | 54.43 | 55.37 | 63.62 | 68.61 | 72.57 | 441.98 |
| ergon/recv-busy @ 1000/s | 29000 | 55.39 | 58.35 | 61.74 | 69.73 | 73.92 | 151.17 |
| ergon/uring-spin @ 0/s | 497658 | 57.43 | 59.15 | 66.95 | 73.93 | 81.71 | 462.51 |
| ergon/uring-spin @ 1000/s | 29000 | 64.31 | 65.73 | 73.49 | 77.96 | 92.58 | 332.79 |
| ergon/uring-sqpoll @ 0/s | 658561 | 43.74 | 41.64 | 55.09 | 65.75 | 71.39 | 723.36 |
| ergon/uring-sqpoll @ 1000/s | 29000 | 58.01 | 60.73 | 62.67 | 64.89 | 69.53 | 1415.84 |
| epoll-spin @ 0/s | 509747 | 56.80 | 59.19 | 67.85 | 73.40 | 79.91 | 611.54 |
| epoll-spin @ 100/s | 2899 | 60.38 | 63.43 | 70.25 | 74.19 | 84.44 | 94.00 |
| epoll-spin @ 1000/s | 28998 | 58.80 | 62.90 | 67.45 | 73.15 | 82.63 | 279.32 |
| recv-busy @ 0/s | 549205 | 52.71 | 54.46 | 63.40 | 68.02 | 71.80 | 1081.16 |
| recv-busy @ 100/s | 2900 | 56.21 | 59.02 | 63.81 | 69.18 | 85.94 | 97.17 |
| recv-busy @ 1000/s | 28988 | 53.84 | 57.30 | 60.35 | 64.80 | 71.04 | 142.59 |
| uring-spin @ 0/s | 519524 | 55.06 | 57.89 | 66.69 | 72.24 | 80.54 | 889.79 |
| uring-spin @ 100/s | 2899 | 61.95 | 65.69 | 71.61 | 76.96 | 99.62 | 118.32 |
| uring-spin @ 1000/s | 28997 | 59.02 | 63.07 | 67.76 | 73.90 | 84.72 | 1060.63 |
| uring-sqpoll @ 0/s | 546087 | 52.89 | 54.55 | 63.79 | 68.48 | 72.99 | 642.37 |
| uring-sqpoll @ 100/s | 2899 | 55.51 | 57.29 | 63.28 | 70.00 | 85.90 | 88.68 |
| uring-sqpoll @ 1000/s | 28999 | 54.44 | 58.02 | 60.73 | 65.13 | 71.10 | 441.07 |

