## Transport latency (µs)

Each cell is the median across reps, the range in brackets. Latency counts from each message's intended send time (open loop): a round trip between hosts, one way on one host. An arm flagged in any rep (`sender late`: over 1 % of sends more than 100 µs late; `over capacity`: achieved under 98 % of the rate; `IRQ core saturated`: the IRQ CPU's ksoftirqd over 90 % of it) is shown and never compared. An arm a round's order names that failed there (failures.csv) is absent: said, and left out. The A/A arm repeats one arm in the same round: its spread is the largest per-rep difference between the two, per metric. Two arms are `not separated` where their medians differ by no more than the A/A spread or their rep ranges overlap; a max verdict is blocked while either arm has an unexplained max (spikes.py --transport). TCP's loss is its retransmits the peer did not report as duplicates and segments lost at the end; DSACK-reported (spurious) ones are shown apart, never as loss. A table holding a point that breaks a fairness rule (fairness.py) is refused.

Fairness: 0 violations.

### Set 2a, 1 stream, 10,000 msg/s: round trip (µs)

| arm                      | reps | min                 | p50                 | p99                 | p99.99              | max                    | busy threads | wire bytes | lost | reordered | dup | retrans | TCP segs lost / spurious | flags | max causes                                   |
| ------------------------ | ---- | ------------------- | ------------------- | ------------------- | ------------------- | ---------------------- | ------------ | ---------- | ---- | --------- | --- | ------- | ------------------------ | ----- | -------------------------------------------- |
| aeron-dpdk               | 3    | 9.68 (9.60–9.76)    | 12.10 (12.00–12.20) | 18.15 (18.00–18.30) | 36.30 (36.00–36.60) | 123.00 (121.00–125.00) | 4            | 138        | 0    | 0         | 0   | 0       | -                        | none  | DPDK refill 3                                |
| aeron-kernel             | 3    | 16.08 (16.00–16.16) | 20.10 (20.00–20.20) | 30.15 (30.00–30.30) | 60.30 (60.00–60.60) | 203.00 (201.00–205.00) | 4            | 138        | 0    | 0         | 0   | 0       | -                        | none  | NAK/retransmit 3                             |
| eio-udp                  | 3    | 12.11 (12.03–12.19) | 15.14 (15.04–15.24) | 22.71 (22.56–22.86) | 45.42 (45.12–45.72) | 153.40 (151.40–155.40) | 2            | 106        | 0    | 0         | 0   | 0       | -                        | none  | page fault 3                                 |
| mio-udp                  | 3    | 12.08 (12.00–12.16) | 15.10 (15.00–15.20) | 22.65 (22.50–22.80) | 45.30 (45.00–45.60) | 153.00 (151.00–155.00) | 2            | 106        | 0    | 0         | 0   | 0       | -                        | none  | IRQ mana_q1 on hot CPU 4 2, softirq NET_RX 1 |
| aa (A/A of aeron-kernel) | 3    | 16.05 (16.02–16.21) | 20.06 (20.03–20.26) | 30.09 (30.05–30.39) | 60.18 (60.09–60.78) | 202.60 (201.30–205.61) | 4            | 138        | 0    | 0         | 0   | 0       | -                        | none  | host pause 3                                 |

A/A spread (`aa` against `aeron-kernel`, 3 reps): min 0.05, p50 0.06, p99 0.09, p99.99 0.18, max 0.61.

| arms                       | min                             | p50                             | p99                             | p99.99                          | max                             |
| -------------------------- | ------------------------------- | ------------------------------- | ------------------------------- | ------------------------------- | ------------------------------- |
| aeron-dpdk vs aeron-kernel | aeron-dpdk lower by 6.40        | aeron-dpdk lower by 8.00        | aeron-dpdk lower by 12.00       | aeron-dpdk lower by 24.00       | aeron-dpdk lower by 80.00       |
| aeron-dpdk vs eio-udp      | aeron-dpdk lower by 2.43        | aeron-dpdk lower by 3.04        | aeron-dpdk lower by 4.56        | aeron-dpdk lower by 9.12        | aeron-dpdk lower by 30.40       |
| aeron-dpdk vs mio-udp      | aeron-dpdk lower by 2.40        | aeron-dpdk lower by 3.00        | aeron-dpdk lower by 4.50        | aeron-dpdk lower by 9.00        | aeron-dpdk lower by 30.00       |
| aeron-kernel vs eio-udp    | eio-udp lower by 3.97           | eio-udp lower by 4.96           | eio-udp lower by 7.44           | eio-udp lower by 14.88          | eio-udp lower by 49.60          |
| aeron-kernel vs mio-udp    | mio-udp lower by 4.00           | mio-udp lower by 5.00           | mio-udp lower by 7.50           | mio-udp lower by 15.00          | mio-udp lower by 50.00          |
| eio-udp vs mio-udp         | not separated (within A/A 0.05) | not separated (within A/A 0.06) | not separated (within A/A 0.09) | not separated (within A/A 0.18) | not separated (within A/A 0.61) |

### Set 2a, 8 streams, 10,000 msg/s: round trip (µs)

| arm                      | reps | min                 | p50                 | p99                 | p99.99              | max                    | busy threads | wire bytes | lost | reordered | dup | retrans | TCP segs lost / spurious | flags                 | max causes                                                    |
| ------------------------ | ---- | ------------------- | ------------------- | ------------------- | ------------------- | ---------------------- | ------------ | ---------- | ---- | --------- | --- | ------- | ------------------------ | --------------------- | ------------------------------------------------------------- |
| aeron-dpdk               | 3    | 10.48 (10.40–10.56) | 13.10 (13.00–13.20) | 19.65 (19.50–19.80) | 39.30 (39.00–39.60) | 133.00 (131.00–135.00) | 4            | 138        | 0    | 0         | 0   | 0       | -                        | none                  | DPDK refill 3                                                 |
| aeron-kernel             | 3    | 19.28 (19.20–19.36) | 24.10 (24.00–24.20) | 36.15 (36.00–36.30) | 72.30 (72.00–72.60) | 243.00 (241.00–245.00) | 4            | 138        | 0    | 0         | 0   | 0       | -                        | none                  | NAK/retransmit 3                                              |
| eio-udp                  | 3    | 14.08 (14.00–14.16) | 17.60 (17.50–17.70) | 26.40 (26.25–26.55) | 52.80 (52.50–53.10) | 178.00 (176.00–180.00) | 2            | 106        | 0    | 0         | 0   | 0       | -                        | sender late (rep 2)   | page fault 2, sender late 1                                   |
| mio-udp                  | 3    | 13.68 (13.60–13.76) | 17.10 (17.00–17.20) | 25.65 (25.50–25.80) | 51.30 (51.00–51.60) | 173.00 (171.00–175.00) | 2            | 106        | 0    | 0         | 0   | 0       | -                        | over capacity (rep 3) | IRQ mana_q1 on hot CPU 4 1, past capacity 1, softirq NET_RX 1 |
| aa (A/A of aeron-kernel) | 3    | 19.24 (19.23–19.42) | 24.05 (24.04–24.27) | 36.08 (36.05–36.41) | 72.16 (72.11–72.82) | 242.52 (241.36–245.73) | 4            | 138        | 0    | 0         | 0   | 0       | -                        | none                  | host pause 3                                                  |

A/A spread (`aa` against `aeron-kernel`, 3 reps): min 0.06, p50 0.07, p99 0.11, p99.99 0.22, max 0.73.

Flagged, not compared: eio-udp, mio-udp.

| arms                       | min                      | p50                       | p99                       | p99.99                    | max                        |
| -------------------------- | ------------------------ | ------------------------- | ------------------------- | ------------------------- | -------------------------- |
| aeron-dpdk vs aeron-kernel | aeron-dpdk lower by 8.80 | aeron-dpdk lower by 11.00 | aeron-dpdk lower by 16.50 | aeron-dpdk lower by 33.00 | aeron-dpdk lower by 110.00 |

### Set 2b, 1 stream, 10,000 msg/s: one way (µs)

| arm                   | reps | min              | p50              | p99                 | p99.99              | max                 | busy threads | wire bytes | lost | reordered | dup | retrans | TCP segs lost / spurious | flags | max causes                 |
| --------------------- | ---- | ---------------- | ---------------- | ------------------- | ------------------- | ------------------- | ------------ | ---------- | ---- | --------- | --- | ------- | ------------------------ | ----- | -------------------------- |
| aeron-self-udp        | 3    | 7.28 (7.20–7.36) | 9.10 (9.00–9.20) | 13.65 (13.50–13.80) | 27.30 (27.00–27.60) | 93.00 (91.00–95.00) | 4            | 138        | 0    | 0         | 0   | 0       | -                        | none  | IRQ mana_q1 on hot CPU 4 3 |
| aeron-spy             | 3    | 0.32 (0.24–0.40) | 0.40 (0.30–0.50) | 0.60 (0.45–0.75)    | 1.20 (0.90–1.50)    | 6.00 (4.00–8.00)    | 3            | 96         | 0    | 0         | 0   | 0       | -                        | none  | page fault 3               |
| aa (A/A of aeron-spy) | 3    | 0.32 (0.24–0.40) | 0.40 (0.30–0.50) | 0.60 (0.45–0.75)    | 1.20 (0.90–1.50)    | 5.99 (4.00–8.02)    | 3            | 96         | 0    | 0         | 0   | 0       | -                        | none  | host pause 3               |

A/A spread (`aa` against `aeron-spy`, 3 reps): min 0.00, p50 0.00, p99 0.00, p99.99 0.00, max 0.01.

| arms                        | min                     | p50                     | p99                      | p99.99                   | max                      |
| --------------------------- | ----------------------- | ----------------------- | ------------------------ | ------------------------ | ------------------------ |
| aeron-self-udp vs aeron-spy | aeron-spy lower by 6.96 | aeron-spy lower by 8.70 | aeron-spy lower by 13.05 | aeron-spy lower by 26.10 | aeron-spy lower by 87.00 |

### Set 3b, 1 stream, netem 0.1 %, 10,000 msg/s: round trip (µs)

| arm                 | reps | min                 | p50                 | p99                 | p99.99              | max                    | busy threads | wire bytes | lost | reordered | dup | retrans | TCP segs lost / spurious | flags | max causes                   |
| ------------------- | ---- | ------------------- | ------------------- | ------------------- | ------------------- | ---------------------- | ------------ | ---------- | ---- | --------- | --- | ------- | ------------------------ | ----- | ---------------------------- |
| aeron-kernel        | 3    | 20.08 (20.00–20.16) | 25.10 (25.00–25.20) | 37.65 (37.50–37.80) | 75.30 (75.00–75.60) | 253.00 (251.00–255.00) | 4            | 138        | 0    | 0         | 0   | 9       | -                        | none  | NAK/retransmit 3             |
| mio-tcp             | 3    | 24.08 (24.00–24.16) | 30.10 (30.00–30.20) | 45.15 (45.00–45.30) | 90.30 (90.00–90.60) | 303.00 (301.00–305.00) | 2            | 110        | 0    | 0         | 0   | 9       | 12 / 6                   | none  | TCP RTO after a netem drop 3 |
| aa (A/A of mio-tcp) | 3    | 24.04 (24.03–24.23) | 30.05 (30.04–30.29) | 45.07 (45.06–45.44) | 90.14 (90.12–90.87) | 302.40 (301.45–305.91) | 2            | 110        | 0    | 0         | 0   | 9       | 12 / 6                   | none  | TCP TLP after a netem drop 3 |

A/A spread (`aa` against `mio-tcp`, 3 reps): min 0.07, p50 0.09, p99 0.14, p99.99 0.27, max 0.91.

| arms                    | min                        | p50                        | p99                        | p99.99                      | max                         |
| ----------------------- | -------------------------- | -------------------------- | -------------------------- | --------------------------- | --------------------------- |
| aeron-kernel vs mio-tcp | aeron-kernel lower by 4.00 | aeron-kernel lower by 5.00 | aeron-kernel lower by 7.50 | aeron-kernel lower by 15.00 | aeron-kernel lower by 50.00 |

### Set 2a by stream count, 10,000 msg/s: round trip (µs)

All streams together, then stream 0 alone; medians across reps, ranges in brackets. A stream count whose table above is refused is left out.

| arm          | streams | reps | p50                 | p99                 | p99.99              | max                    | stream 0 p50        | stream 0 p99        | stream 0 p99.99     | stream 0 max           | flags                 |
| ------------ | ------- | ---- | ------------------- | ------------------- | ------------------- | ---------------------- | ------------------- | ------------------- | ------------------- | ---------------------- | --------------------- |
| aeron-dpdk   | 1       | 3    | 12.10 (12.00–12.20) | 18.15 (18.00–18.30) | 36.30 (36.00–36.60) | 123.00 (121.00–125.00) | 12.10 (12.00–12.20) | 18.15 (18.00–18.30) | 36.30 (36.00–36.60) | 123.00 (121.00–125.00) | none                  |
| aeron-kernel | 1       | 3    | 20.10 (20.00–20.20) | 30.15 (30.00–30.30) | 60.30 (60.00–60.60) | 203.00 (201.00–205.00) | 20.10 (20.00–20.20) | 30.15 (30.00–30.30) | 60.30 (60.00–60.60) | 203.00 (201.00–205.00) | none                  |
| eio-udp      | 1       | 3    | 15.14 (15.04–15.24) | 22.71 (22.56–22.86) | 45.42 (45.12–45.72) | 153.40 (151.40–155.40) | 15.14 (15.04–15.24) | 22.71 (22.56–22.86) | 45.42 (45.12–45.72) | 153.40 (151.40–155.40) | none                  |
| mio-udp      | 1       | 3    | 15.10 (15.00–15.20) | 22.65 (22.50–22.80) | 45.30 (45.00–45.60) | 153.00 (151.00–155.00) | 15.10 (15.00–15.20) | 22.65 (22.50–22.80) | 45.30 (45.00–45.60) | 153.00 (151.00–155.00) | none                  |
| aa           | 1       | 3    | 20.06 (20.03–20.26) | 30.09 (30.05–30.39) | 60.18 (60.09–60.78) | 202.60 (201.30–205.61) | 20.06 (20.03–20.26) | 30.09 (30.05–30.39) | 60.18 (60.09–60.78) | 202.60 (201.30–205.61) | none                  |
| aeron-dpdk   | 8       | 3    | 13.10 (13.00–13.20) | 19.65 (19.50–19.80) | 39.30 (39.00–39.60) | 133.00 (131.00–135.00) | 13.10 (13.00–13.20) | 19.65 (19.50–19.80) | 39.30 (39.00–39.60) | 133.00 (131.00–135.00) | none                  |
| aeron-kernel | 8       | 3    | 24.10 (24.00–24.20) | 36.15 (36.00–36.30) | 72.30 (72.00–72.60) | 243.00 (241.00–245.00) | 24.10 (24.00–24.20) | 36.15 (36.00–36.30) | 72.30 (72.00–72.60) | 243.00 (241.00–245.00) | none                  |
| eio-udp      | 8       | 3    | 17.60 (17.50–17.70) | 26.40 (26.25–26.55) | 52.80 (52.50–53.10) | 178.00 (176.00–180.00) | 17.60 (17.50–17.70) | 26.40 (26.25–26.55) | 52.80 (52.50–53.10) | 178.00 (176.00–180.00) | sender late (rep 2)   |
| mio-udp      | 8       | 3    | 17.10 (17.00–17.20) | 25.65 (25.50–25.80) | 51.30 (51.00–51.60) | 173.00 (171.00–175.00) | 17.10 (17.00–17.20) | 25.65 (25.50–25.80) | 51.30 (51.00–51.60) | 173.00 (171.00–175.00) | over capacity (rep 3) |
| aa           | 8       | 3    | 24.05 (24.04–24.27) | 36.08 (36.05–36.41) | 72.16 (72.11–72.82) | 242.52 (241.36–245.73) | 24.05 (24.04–24.27) | 36.08 (36.05–36.41) | 72.16 (72.11–72.82) | 242.52 (241.36–245.73) | none                  |
