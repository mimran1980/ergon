# Rows quoted by the docs

Verbatim sections of this run's generated tables, recovered from git (`git show 128747b9^:io/benches/results/20261014T060634Z/tables.md`).
Only the sections `io/benches/BENCHMARK.md` and the book quote; nothing is recomputed. All values µs unless a column says otherwise.

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
