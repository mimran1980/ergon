## A/B: A = `isolated:dpdk`, B = `isolated:dpdk@BIN=bin-base`

Each cell is the median across reps, the range in brackets. A verdict names the better side only where the two ranges do not overlap. NAKs are those either host sent during the run; the 32 B flat-out run's are the round trip and throughput snapshot's less the idle round trip's.

| metric | A reps | A | B reps | B | verdict |
| --- | --- | --- | --- | --- | --- |
| idle round trip p50 (µs) | 3 | 28.00 (27.90–28.50) | 3 | 29.60 (29.50–29.80) | A better, ranges apart |
| idle round trip p99 (µs) | 3 | 31.00 (30.90–31.50) | 3 | 33.20 (33.00–33.50) | A better, ranges apart |
| idle round trip p99.99 (µs) | 3 | 38.00 (37.90–39.00) | 3 | 44.00 (43.00–45.00) | A better, ranges apart |
| idle round trip max (µs) | 3 | 70.00 (65.00–800.00) | 3 | 120.00 (90.00–700.00) | ranges overlap |
| 32 B flat out (M msgs/s) | 3 | 10.00 (9.90–10.20) | 3 | 6.60 (6.50–6.80) | A better, ranges apart |
| 288 B sustained maximum (M msgs/s) | 3 | 2.45 (2.40–2.50) | 3 | 1.90 (1.80–2.42) | ranges overlap |
| 288 B, 2 flows at once (M msgs/s) | 2 | 4.80 (4.70–4.90) | 1 | 2.00 (2.00–2.00) | A better, ranges apart |
| 288 B at 250,000/s, p99.99 (µs) | 3 | 45.00 (42.00–50.00) | 3 | 80.00 (70.00–90.00) | A better, ranges apart |
| 288 B at 250,000/s, max (µs) | 3 | 70.00 (60.00–110.00) | 3 | 130.00 (100.00–150.00) | ranges overlap |
| 288 B, bursts of 1,000, p99.99 (µs) | 3 | 250.00 (247.00–260.00) | 3 | 300.00 (240.00–330.00) | ranges overlap |
| 288 B, bursts of 1,000, max (µs) | 3 | 255.00 (253.00–350.00) | 3 | 310.00 (245.00–340.00) | ranges overlap |
| NAKs, idle round trip | 3 | 0 (0–0) | 3 | 0 (0–4) | ranges overlap |
| NAKs, 32 B flat out | 3 | 0 (0–0) | 3 | 0 (0–30) | ranges overlap |
| NAKs, 288 B sustained maximum | 3 | 0 (0–0) | 3 | 0 (0–1500) | ranges overlap |
| NAKs, 288 B, 2 flows at once | 3 | 0 (0–0) | 2 | 20 (0–40) | ranges overlap |
| NAKs, 288 B at 250,000/s | 3 | 0 (0–0) | 2 | 4 (0–7) | ranges overlap |
| NAKs, 288 B, bursts of 1,000 | 3 | 0 (0–12) | 3 | 0 (0–7) | ranges overlap |

Runs with NAKs, every run above pooled: A 1 of 18, B 6 of 16; Fisher's exact test p = 0.035 (two-sided).

Left out of the rates over several flows at once, their flows' measured windows overlapping less than 0.9 of the shortest or by an amount not recorded: A, 288 B, 2 flows, rep 2 (0.899); B, 288 B, 2 flows, rep 1 (unknown).

### Agent poll-cycle gaps during the idle round trip, per rep (20–50/50–100/over 100 µs, both ports)

| rep | A | B |
| --- | --- | --- |
| 1 | a 1/0/0, b 2/0/0 | a 0/0/1, b 4/2/1 |
| 2 | a 2/1/0, b 0/0/0 | a 1/0/0, b n/a |
| 3 | n/a | n/a |

### PMD burst timing during the idle round trip, per rep (RUSTERON_DPDK_GAP_ATTRIB=1: max ns / ≥1 µs / ≥5 µs)

| rep | A | B |
| --- | --- | --- |
| 1 | a rx 6250/4/1, tx 1900/1/0; b n/a | n/a |
| 2 | n/a | n/a |
| 3 | n/a | n/a |
