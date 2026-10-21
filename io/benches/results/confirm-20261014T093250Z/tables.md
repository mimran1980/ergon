## Ping-pong round trip (client send → echo decoded)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | max µs |
|---|---|---|---|---|---|---|---|
| client-curl-perform/distro @ 0/s | 385250 | 75.06 | 76.78 | 85.05 | 91.21 | 96.86 | 974.53 |
| client-curl-perform/distro @ 0/s | 384836 | 75.16 | 76.88 | 84.83 | 90.57 | 96.31 | 711.55 |
| client-curl-perform/distro @ 1000/s | 28995 | 70.94 | 73.65 | 80.67 | 85.06 | 93.29 | 280.28 |
| client-curl-perform/distro @ 1000/s | 28997 | 71.53 | 74.73 | 80.29 | 84.24 | 94.42 | 203.13 |
| client-curl-perform/source @ 0/s | 396143 | 73.00 | 74.33 | 83.88 | 90.04 | 99.72 | 1321.57 |
| client-curl-perform/source @ 0/s | 394447 | 73.31 | 74.62 | 84.13 | 90.18 | 98.73 | 737.24 |
| client-curl-perform/source @ 1000/s | 28998 | 67.59 | 70.25 | 76.89 | 82.59 | 90.15 | 839.36 |
| client-curl-perform/source @ 1000/s | 28994 | 71.08 | 73.78 | 80.13 | 85.45 | 92.94 | 1486.14 |
| client-curl-socket-action/distro @ 0/s | 381989 | 75.72 | 77.46 | 85.03 | 91.42 | 100.04 | 682.17 |
| client-curl-socket-action/distro @ 0/s | 381584 | 75.80 | 77.56 | 85.54 | 92.18 | 100.58 | 442.39 |
| client-curl-socket-action/distro @ 1000/s | 28993 | 73.05 | 75.84 | 82.47 | 86.08 | 91.93 | 596.46 |
| client-curl-socket-action/distro @ 1000/s | 28994 | 72.66 | 75.85 | 81.33 | 85.36 | 96.31 | 451.99 |
| client-curl-socket-action/source @ 0/s | 395131 | 73.20 | 74.69 | 83.73 | 89.80 | 102.19 | 521.58 |
| client-curl-socket-action/source @ 0/s | 388935 | 74.35 | 75.57 | 84.06 | 90.90 | 102.15 | 880.51 |
| client-curl-socket-action/source @ 1000/s | 28989 | 76.43 | 76.40 | 87.20 | 99.84 | 114.54 | 313.60 |
| client-curl-socket-action/source @ 1000/s | 28994 | 71.82 | 74.91 | 80.95 | 83.73 | 89.80 | 402.58 |
| client-native/recv-busy @ 0/s | 601274 | 48.05 | 47.44 | 56.49 | 66.21 | 70.38 | 831.49 |
| client-native/recv-busy @ 0/s | 709320 | 40.70 | 40.17 | 42.27 | 53.04 | 61.85 | 649.62 |
| client-native/recv-busy @ 1000/s | 29000 | 53.84 | 53.92 | 61.06 | 63.47 | 67.65 | 95.42 |
| client-native/recv-busy @ 1000/s | 29000 | 54.49 | 54.95 | 61.40 | 63.91 | 69.17 | 1532.39 |
| client-native/uring-sqpoll @ 0/s | 569038 | 50.61 | 51.04 | 57.88 | 66.98 | 72.04 | 659.18 |
| client-native/uring-sqpoll @ 0/s | 560026 | 51.46 | 51.94 | 59.23 | 68.38 | 76.35 | 616.37 |
| client-native/uring-sqpoll @ 1000/s | 29000 | 55.01 | 55.15 | 62.12 | 64.44 | 67.34 | 87.57 |
| client-native/uring-sqpoll @ 1000/s | 29000 | 55.15 | 55.23 | 62.21 | 64.57 | 68.26 | 340.33 |
| curl-distro/8.5.0 @ 0/s | 383812 | 75.18 | 76.57 | 84.85 | 90.45 | 98.08 | 1356.99 |
| curl-distro/8.5.0 @ 0/s | 517647 | 55.62 | 54.99 | 57.61 | 67.04 | 75.78 | 437.61 |
| curl-distro/8.5.0 @ 1000/s | 28995 | 69.57 | 71.73 | 79.54 | 83.45 | 93.80 | 700.17 |
| curl-distro/8.5.0 @ 1000/s | 28996 | 69.18 | 71.12 | 78.87 | 82.24 | 87.38 | 1304.20 |
| ergon/recv-busy @ 0/s | 569746 | 50.72 | 51.45 | 57.52 | 67.14 | 73.58 | 641.03 |
| ergon/recv-busy @ 0/s | 573009 | 50.43 | 51.06 | 57.52 | 66.87 | 70.92 | 401.39 |
| ergon/recv-busy @ 1000/s | 29000 | 53.14 | 53.51 | 60.17 | 62.49 | 66.48 | 87.29 |
| ergon/recv-busy @ 1000/s | 29000 | 55.12 | 54.78 | 60.59 | 63.34 | 73.25 | 117.11 |

