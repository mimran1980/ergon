# Rows quoted by the docs

Verbatim sections of this run's generated tables, recovered from git (`git show 128747b9^:io/benches/results/flags-aws-20261015T051742Z/tables.md`).
Only the sections `io/benches/BENCHMARK.md` and the book quote; nothing is recomputed. All values µs unless a column says otherwise.

## HTTP round trip, http-curl-source-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1982338 | 55.17 | 67.68 | 95.36 | 101.92 | 152.54 | 156.2 | 0.0 | 0.0 | 0.0 | 871.21 | 41.5 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1981759 | 54.31 | 68.61 | 96.12 | 102.54 | 269.40 | 203.8 | 0.0 | 0.0 | 0.0 | 865.58 | 85.5 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-curl-source-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189917 | 55.99 | 100.78 | 109.72 | 121.04 | 262.11 | 7608.9 | 0.0 | 0.0 | 0.0 | 846.24 | 41.5 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1189914 | 56.53 | 73.83 | 101.67 | 111.19 | 345.45 | 1490.5 | 0.0 | 0.0 | 0.0 | 863.80 | 53.5 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-ergon-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1980506 | 48.96 | 96.85 | 103.82 | 109.81 | 226.33 | 1794.6 | 0.0 | 0.0 | 0.0 | 179.88 | 11.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| h2 | 1980380 | 48.92 | 96.53 | 99.87 | 108.40 | 193.86 | 1155.0 | 0.0 | 0.0 | 0.0 | 115.99 | 13.0 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-ergon-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189937 | 48.64 | 90.37 | 95.48 | 102.28 | 340.29 | 85.7 | 0.0 | 0.0 | 0.0 | 287.94 | 14.5 | client: softirq TIMER |
| h2 | 1189936 | 49.37 | 96.22 | 106.80 | 116.36 | 1013.67 | 2616.4 | 0.5 | 0.5 | 0.0 | 32.17 | 10.5 | neither loop stalled long enough: network, hypervisor or untraced |
