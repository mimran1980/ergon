# Rows quoted by the docs

Verbatim sections of this run's generated tables, recovered from git (`git show 128747b9^:io/benches/results/20261014T005221Z/tables.md`).
Only the sections `io/benches/BENCHMARK.md` and the book quote; nothing is recomputed. All values µs unless a column says otherwise.

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
