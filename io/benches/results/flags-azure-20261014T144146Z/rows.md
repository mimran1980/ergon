# Rows quoted by the docs

Verbatim sections of this run's generated tables, recovered from git (`git show 128747b9^:io/benches/results/flags-azure-20261014T144146Z/tables-local.md`).
Only the sections `io/benches/BENCHMARK.md` and the book quote; nothing is recomputed. All values µs unless a column says otherwise.

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | receiver loop | 55 | :51 | 49 (89%) | 29 of 40 | 1365.12 | **yes** |
| http-recv-busy-10000 | sender loop | 68 | :49 | 68 (100%) | 37 of 40 | 1022.64 | **yes** |
| http-recv-busy-10000 | slow samples | 344 | :50 | 208 (60%) | 40 of 40 | 1193.55 | **yes** |
| md-recv-busy | receiver loop | 27 | :51 | 26 (96%) | 14 of 21 | 1350.61 | **yes** |
| md-recv-busy | slow samples | 289 | :51 | 289 (100%) | 8 of 21 | 1329.36 | **yes** |
| noise | noise probe (loop CPU) | 48 | :51 | 47 (98%) | 24 of 41 | 1082.74 | **yes** |
| pp-epoll-spin-0 | receiver loop | 14 | :51 | 14 (100%) | 7 of 12 | 1370.12 | **yes** |
| pp-epoll-spin-0 | sender loop | 17 | :49 | 17 (100%) | 10 of 11 | 444.79 | **yes** |
| pp-epoll-spin-0 | slow samples | 240 | :50 | 83 (35%) | 12 of 12 | 1424.90 | **yes** |
| pp-epoll-spin-10000 | receiver loop | 39 | :50 | 32 (82%) | 18 of 22 | 1645.67 | **yes** |
| pp-epoll-spin-10000 | sender loop | 39 | :49 | 37 (95%) | 19 of 22 | 708.69 | **yes** |
| pp-epoll-spin-10000 | slow samples | 334 | :50 | 132 (40%) | 22 of 22 | 1665.26 | **yes** |
| pp-recv-busy-0 | receiver loop | 22 | :51 | 22 (100%) | 13 of 20 | 1264.70 | **yes** |
| pp-recv-busy-0 | sender loop | 30 | :49 | 30 (100%) | 17 of 20 | 702.21 | **yes** |
| pp-recv-busy-0 | slow samples | 134 | :50 | 101 (75%) | 20 of 20 | 1296.00 | **yes** |
| pp-recv-busy-10000 | receiver loop | 58 | :51 | 38 (66%) | 24 of 40 | 1546.12 | **yes** |
| pp-recv-busy-10000 | sender loop | 72 | :49 | 70 (97%) | 36 of 41 | 912.36 | **yes** |
| pp-recv-busy-10000 | slow samples | 321 | :50 | 180 (56%) | 40 of 41 | 1571.00 | **yes** |
| pp-uring-spin-0 | receiver loop | 21 | :51 | 21 (100%) | 14 of 17 | 1595.41 | **yes** |
| pp-uring-spin-0 | sender loop | 26 | :49 | 26 (100%) | 15 of 17 | 658.57 | **yes** |
| pp-uring-spin-0 | slow samples | 318 | :50 | 124 (39%) | 17 of 17 | 1662.08 | **yes** |
| pp-uring-spin-10000 | receiver loop | 51 | :51 | 39 (76%) | 22 of 34 | 1221.89 | **yes** |
| pp-uring-spin-10000 | sender loop | 70 | :49 | 70 (100%) | 33 of 35 | 895.36 | **yes** |
| pp-uring-spin-10000 | slow samples | 525 | :50 | 204 (39%) | 34 of 35 | 2149.10 | **yes** |
| pp-uring-sqpoll-0 | receiver loop | 13 | :51 | 13 (100%) | 8 of 13 | 947.02 | **yes** |
| pp-uring-sqpoll-0 | sender loop | 30 | :49 | 30 (100%) | 13 of 14 | 559.99 | **yes** |
| pp-uring-sqpoll-0 | slow samples | 119 | :50 | 69 (58%) | 13 of 13 | 948.26 | **yes** |
| pp-uring-sqpoll-10000 | receiver loop | 32 | :51 | 32 (100%) | 19 of 26 | 1583.14 | **yes** |
| pp-uring-sqpoll-10000 | sender loop | 40 | :49 | 40 (100%) | 23 of 26 | 595.51 | **yes** |
| pp-uring-sqpoll-10000 | slow samples | 231 | :50 | 129 (56%) | 26 of 26 | 1612.84 | **yes** |
