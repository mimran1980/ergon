# Many streams: layouts, sharding, settings and memory

Measured with `rusteron-client/benches/many_streams.rs` on omarchy (i5-8500T, 6 cores, no SMT, Linux 7.2, a desktop box without isolated CPUs): structure and relative costs, not latency verdicts. One process holds the driver(s) and both clients. Ping sends 10,000 msg/s in total (32-byte messages, round robin over the logical streams), pong echoes, ping records the round trip. 10 s per arm, after a warm-up echo on every stream. Every max is attributed: "preempted" means the stalled thread logged at least one involuntary context switch since its previous >100 µs stall (strong evidence, not proof); "no switch" stalls are unexplained.

## Arms

| Arm | What it is |
|---|---|
| `plain` (per-port) | N channels on N ports, one stream id each, each way |
| `sameport` | one channel (one port) each way, N stream ids on it |
| `samestream` | N channels on N ports, all the same stream id |
| `mux` | 1 port, 1 stream id each way, N logical streams keyed in the message |
| `mux4`, `mux8` | N logical streams spread over 4 or 8 stream ids on one port each way |
| `shardK` | N per-port streams split over K drivers (dedicated sender and receiver threads); ping and pong on one client thread |
| `snshardK` | as `shardK`, each driver's sender and receiver on one thread (as SHARED_NETWORK) |
| `idle*` | the layout connected, no traffic: per-cycle cost |
| run C | concurrent instead of exclusive publications |
| run D | `AERON_UDP_CHANNEL_TRANSPORT_POLLER_ITERATION_THRESHOLD=0` (epoll for any socket count; default 5) |
| runs E64K, E16M | per-port N=128 with `term-length=65536` or `16777216` on every publication channel |

## Setup

- Driver: C driver of Aeron 1.54.0 in INVOKER mode; the bench runs each agent's `do_work` in a busy loop on its own pinned thread, as dedicated threading does, and times every cycle with the TSC. Conductor cycle threshold 10 µs. Driver default term 64 KiB (`term_buffer_length`, `ipc_term_buffer_length`); runs E set the term on the channel. Logs sparse (`AERON_TERM_BUFFER_SPARSE_FILE` default true), not pre-touched (`AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY` default false), storage checks on, Aeron dir in `/dev/shm`. Everything else default.
- CPUs: cpu0 housekeeping, every driver conductor (yields when idle) and the clients' conductor threads. Single driver: sender cpu1, receiver cpu2, ping cpu3, pong cpu4, cpu5 free. `shard1`: sender 1, receiver 2, client 4. `shard2`: driver a 1 and 2, driver b 3 and 5, client 4. `snshardK`: driver nets on 1, 2, 3, 5 in order, client 4.
- Syscalls counted per thread with an `LD_PRELOAD` shim (recvmmsg, recvmsg, recvfrom, epoll_wait, sendmmsg, sendmsg, sendto); per cycle = calls / cycles over the driver's whole life, warm-up included.
- The sender's syscalls per cycle fall below 1 from N=32 (`epoll_wait` 0.28 at 32, 0.05 at 128) although an idle sender should poll every cycle; not explained.
- Scripts: `bench-run.sh` (one run under `~/ergon-work/.bench.lock`), `sweep.sh` (these runs), `sysccount.c` (the shim), `report.py` (the CSVs).
- Memory: log buffer files in each driver's dir (apparent size and allocated blocks), and the process's `/proc/self/status` RSS, taken when the timed run ends. RSS covers driver(s) and both clients, and grows across arms in one process (the allocator keeps freed memory), so compare RSS only within a run or between the single-arm runs E64K and E16M.

## Findings

- Round-trip p50 tracks the number of Aeron streams, not ports: per-port, same port with N stream ids and one stream id on N ports are within 1.5 µs of each other at every N (49.6 / 47.2 / 49.7 µs at 128). Keying many logical streams into a few Aeron streams flattens it (mux 11.9 µs, mux8 14.0 µs at 128).
- Per stream the driver's sender walk dominates (p50 24 µs per cycle at 2 x 128 publications); the receiver's slow tick grows less (5 µs); the client pass about 4 µs.
- Sharding: the driver side follows the per-driver stream count (sender cycle 10.1 µs for 2 x 64 against 9.8 µs for 1 x 64), but one client thread polling all 2N subscriptions keeps its pass at the total count, so RTT p50 at 128 is 47.2 µs on 1 driver, 40.4 µs on 2 (dedicated), 71.9 / 49.5 / 34.3 µs on 1 / 2 / 4 shared-network drivers.
- Concurrent against exclusive publications: no change beyond noise.
- Poller iteration threshold 0: few-socket layouts save 0.15-0.2 µs per idle cycle (one `epoll_wait` instead of one `recvmmsg` per socket); mux8 RTT p50 13.6 -> 13.3 µs at 32, 14.0 -> 13.2 µs at 128; no change for same-port.
- 16 MiB terms against 64 KiB at per-port N=128: RTT p50 68.0 against 50.9 µs, sender cycle p50 35.6 against 25.7 µs, receiver 8.1 against 5.1 µs. Log apparent size 24 GiB against 98 MiB, allocated 28 MiB in both, RSS 213 MiB in both.

## Settings checked for a shorter walk

| Setting | Touches the walk? |
|---|---|
| poller iteration threshold | No, but sets syscalls per cycle for 5 or fewer sockets: measured (run D) |
| `AERON_SEND_TO_STATUS_POLL_RATIO`, `AERON_RECEIVER_POLL_TO_SLOW_WORK_RATIO` | No: they only space out control polls in cycles that did work; idle cycles always poll and walk |
| `AERON_RCV_STATUS_MESSAGE_TIMEOUT` | No: the receiver's slow tick still checks every image each idle cycle |
| heartbeat and setup intervals | Fixed constants (100 ms); the per-publication check runs every cycle regardless |
| io vector capacities, max messages per send | No: batching within one publication or socket |
| idle strategies | No: they only add sleep between cycles |
| term length | Not the walk, but measured (runs E): larger terms cost more per cycle |
| threading mode | Measured as `snshardK`: one thread doing both walks is slower per stream |

## Tables

### Round trip (µs): p50 / p99 / max, with the max's cause

| Arm | Run | N=1 | N=32 | N=64 | N=128 |
|---|---|---|---|---|---|
| per-port | L plain | 11.67 / 18.24 / 1504.26 | 18.03 / 2652.16 / 45875.20 | 28.24 / 46.11 / 2390.01 | 49.57 / 81.02 / 5746.69 |
| same port, N stream ids | L sameport | 11.67 / 23.89 / 495.10 | 17.17 / 35.42 / 16310.27 | 27.25 / 46.17 / 1211.39 | 47.20 / 83.39 / 21053.44 |
| same stream id, N ports | L samestream | 11.31 / 24.22 / 3256.32 | 17.76 / 34.98 / 10952.70 | 28.35 / 45.44 / 1674.24 | 49.70 / 79.17 / 5890.05 |
| mux (1 stream) | L mux | 11.52 / 25.60 / 2334.72 | 11.79 / 17.68 / 842.75 | 11.72 / 27.93 / 12206.08 | 11.85 / 26.40 / 21676.03 |
| mux groups of 4 | L mux4 | 11.65 / 17.74 / 76.03 | 12.56 / 28.24 / 15433.73 | 12.63 / 32.29 / 21200.90 | 12.70 / 21.15 / 593.41 |
| mux groups of 8 | L mux8 | 11.61 / 24.18 / 68.54 | 13.64 / 2822.14 / 29933.57 | 13.74 / 29.02 / 19677.18 | 13.99 / 6893.57 / 58982.40 |
| 1 driver, shared client thread | B1 shard1 | 11.44 / 16.01 / 42.59 | 20.37 / 30.59 / 3723.26 | 30.06 / 44.99 / 378.37 | 47.20 / 76.48 / 995.33 |
| 2 drivers | B2 shard2 | – | 17.20 / 579.58 / 14532.61 | 22.72 / 255.87 / 34275.33 | 40.35 / 60.67 / 6574.08 |
| 1 driver, shared-network | B1 snshard1 | 13.31 / 25.30 / 645.63 | 24.38 / 53.50 / 34439.17 | 39.45 / 65.60 / 233.98 | 71.87 / 129.02 / 622.08 |
| 2 drivers, shared-network | B2 snshard2 | – | 20.69 / 29.92 / 563.71 | 29.68 / 42.69 / 3319.81 | 49.47 / 74.37 / 175.36 |
| 4 drivers, shared-network | B2 snshard4 | – | 19.18 / 80.19 / 26935.29 | 22.99 / 459.26 / 34406.40 | 34.34 / 126.72 / 6635.52 |
| same port, concurrent pubs | C sameport | – | 17.58 / 34.11 / 13131.77 | 28.37 / 3194.88 / 31326.21 | 48.41 / 14819.33 / 51249.15 |
| mux, concurrent pubs | C mux | – | 11.64 / 25.65 / 2105.34 | 11.78 / 19.71 / 872.96 | 11.62 / 71.94 / 35717.12 |
| mux8, concurrent pubs | C mux8 | – | 13.87 / 22.02 / 382.72 | 13.83 / 27.65 / 11182.08 | 13.78 / 28.05 / 92.29 |
| same port, poller threshold 0 | D sameport | – | 17.31 / 32.83 / 12107.77 | – | 47.94 / 1716.22 / 61472.77 |
| mux8, poller threshold 0 | D mux8 | – | 13.25 / 28.21 / 8519.68 | – | 13.21 / 26.51 / 276.74 |
| per-port, term 64 KiB | E64K plain | – | – | – | 50.85 / 94.46 / 19283.97 |
| per-port, term 16 MiB | E16M plain | – | – | – | 67.97 / 111.87 / 21921.79 |

Max causes (round trip): the thread stall overlapping the worst round trip.

| Run arm | N | max µs | cause |
|---|---|---|---|
| L plain | 1 | 1504.26 | ping stall 539.37µs, no switch |
| L plain | 32 | 45875.20 | ping stall 31332.86µs, preempted |
| L plain | 64 | 2390.01 | receiver stall 2328.39µs, preempted |
| L plain | 128 | 5746.69 | receiver stall 4677.79µs, preempted |
| L sameport | 1 | 495.10 | no overlapping thread stall |
| L sameport | 32 | 16310.27 | receiver stall 13432.86µs, preempted |
| L sameport | 64 | 1211.39 | ping stall 654.92µs, preempted |
| L sameport | 128 | 21053.44 | sender stall 20586.90µs, preempted |
| L samestream | 1 | 3256.32 | receiver stall 3262.99µs, preempted |
| L samestream | 32 | 10952.70 | ping stall 6339.15µs, preempted |
| L samestream | 64 | 1674.24 | ping stall 899.88µs, no switch |
| L samestream | 128 | 5890.05 | ping stall 5548.53µs, preempted |
| L mux | 1 | 2334.72 | sender stall 2201.56µs, preempted |
| L mux | 32 | 842.75 | ping stall 316.66µs, preempted |
| L mux | 64 | 12206.08 | receiver stall 12400.56µs, preempted |
| L mux | 128 | 21676.03 | pong stall 21644.65µs, preempted |
| L mux4 | 1 | 76.03 | under 100µs |
| L mux4 | 32 | 15433.73 | receiver stall 14029.20µs, preempted |
| L mux4 | 64 | 21200.90 | pong stall 18798.04µs, preempted |
| L mux4 | 128 | 593.41 | receiver stall 519.94µs, preempted |
| L mux8 | 1 | 68.54 | under 100µs |
| L mux8 | 32 | 29933.57 | sender stall 26647.55µs, preempted |
| L mux8 | 64 | 19677.18 | pong stall 15818.52µs, preempted |
| L mux8 | 128 | 58982.40 | receiver stall 35624.20µs, preempted |
| B1 shard1 | 1 | 42.59 | under 100µs |
| B1 shard1 | 32 | 3723.26 | receiver-a stall 2746.74µs, preempted |
| B1 shard1 | 64 | 378.37 | conductor-a stall 2050.61µs, preempted |
| B1 shard1 | 128 | 995.33 | sender-a stall 498.38µs, preempted |
| B2 shard2 | 32 | 14532.61 | client stall 14409.06µs, preempted |
| B2 shard2 | 64 | 34275.33 | receiver-b stall 36830.77µs, preempted |
| B2 shard2 | 128 | 6574.08 | client stall 6609.55µs, preempted |
| B1 snshard1 | 1 | 645.63 | net-a stall 346.88µs, preempted |
| B1 snshard1 | 32 | 34439.17 | net-a stall 17009.15µs, preempted |
| B1 snshard1 | 64 | 233.98 | client stall 207.42µs, preempted |
| B1 snshard1 | 128 | 622.08 | net-a stall 439.54µs, preempted |
| B2 snshard2 | 32 | 563.71 | net-b stall 526.41µs, preempted |
| B2 snshard2 | 64 | 3319.81 | client stall 3381.09µs, preempted |
| B2 snshard2 | 128 | 175.36 | client stall 110.00µs, preempted |
| B2 snshard4 | 32 | 26935.29 | net-d stall 26006.31µs, preempted |
| B2 snshard4 | 64 | 34406.40 | client stall 34214.47µs, preempted |
| B2 snshard4 | 128 | 6635.52 | client stall 6672.58µs, preempted |
| C sameport | 32 | 13131.77 | pong stall 9881.25µs, preempted |
| C sameport | 64 | 31326.21 | pong stall 26165.22µs, preempted |
| C sameport | 128 | 51249.15 | ping stall 35224.49µs, preempted |
| C mux | 32 | 2105.34 | receiver stall 2025.39µs, preempted |
| C mux | 64 | 872.96 | receiver stall 857.85µs, no switch |
| C mux | 128 | 35717.12 | receiver stall 17710.62µs, preempted |
| C mux8 | 32 | 382.72 | receiver stall 377.61µs, no switch |
| C mux8 | 64 | 11182.08 | ping stall 4956.03µs, preempted |
| C mux8 | 128 | 92.29 | under 100µs |
| D sameport | 32 | 12107.77 | sender stall 4080.70µs, preempted |
| D sameport | 128 | 61472.77 | sender stall 21423.46µs, preempted |
| D mux8 | 32 | 8519.68 | ping stall 5809.88µs, preempted |
| D mux8 | 128 | 276.74 | ping stall 179.66µs, preempted |
| E64K plain | 128 | 19283.97 | ping stall 16668.64µs, preempted |
| E16M plain | 128 | 21921.79 | ping stall 15873.68µs, preempted |

### Idle duty cycle (µs): p50 / p99 / max (cause), syscalls per cycle

| Layout | N | client pass | sender | receiver | conductor | receiver syscalls/cycle | sender syscalls/cycle |
|---|---|---|---|---|---|---|---|
| L idle | 1 | 0.03 / 0.03 / 304.38 (preempted) | 0.88 / 0.91 / 566.27 (preempted) | 0.90 / 0.94 / 2557.95 (preempted) | 0.11 / 0.11 / 40.80 (under 100 us) | recvmmsg=0.99 sendmmsg=0.00 | recvmsg=0.97 |
| L idle | 32 | 0.61 / 0.66 / 27.71 (under 100 us) | 2.22 / 2.35 / 108.99 (preempted) | 1.06 / 1.11 / 27.60 (under 100 us) | 1.37 / 1.46 / 30.59 (under 100 us) | epoll_wait=0.99 recvmmsg=0.02 sendmmsg=0.00 | epoll_wait=0.28 recvmsg=0.01 |
| L idle | 64 | 1.55 / 1.62 / 68.73 (under 100 us) | 3.91 / 4.13 / 173.95 (preempted) | 1.49 / 1.57 / 38.08 (under 100 us) | 3.18 / 3.35 / 991.23 (preempted) | epoll_wait=0.99 recvmmsg=0.02 sendmmsg=0.00 | epoll_wait=0.12 recvmsg=0.01 |
| L idle | 128 | 3.86 / 4.09 / 26148.86 (preempted) | 9.72 / 12.34 / 425.73 (no switch) | 2.55 / 2.73 / 44.77 (under 100 us) | 9.21 / 11.86 / 666.62 (preempted) | epoll_wait=1.00 recvmmsg=0.01 sendmmsg=0.00 | epoll_wait=0.05 recvmsg=0.00 |
| L idle-sameport | 1 | 0.03 / 0.03 / 25.73 (under 100 us) | 0.87 / 0.93 / 27.41 (under 100 us) | 0.91 / 1.00 / 28.21 (under 100 us) | 0.11 / 0.15 / 26.64 (under 100 us) | recvmmsg=1.00 sendmmsg=0.00 | recvmsg=0.97 |
| L idle-sameport | 32 | 0.65 / 0.71 / 968.19 (preempted) | 2.32 / 2.48 / 350.46 (preempted) | 1.33 / 1.41 / 1021.95 (preempted) | 1.34 / 1.44 / 519.68 (preempted) | recvmmsg=1.00 sendmmsg=0.00 | recvmsg=0.28 |
| L idle-sameport | 64 | 1.50 / 1.61 / 24920.06 (preempted) | 4.43 / 4.70 / 488.70 (no switch) | 1.79 / 1.87 / 484.35 (preempted) | 3.34 / 3.52 / 476.16 (no switch) | recvmmsg=1.00 sendmmsg=0.00 | recvmsg=0.11 |
| L idle-sameport | 128 | 3.71 / 3.96 / 144.13 (preempted) | 9.44 / 11.78 / 299.26 (preempted) | 2.85 / 3.07 / 66.05 (under 100 us) | 8.92 / 11.62 / 164.74 (preempted) | recvmmsg=1.00 sendmmsg=0.00 | recvmsg=0.05 |
| L idle-samestream | 1 | 0.03 / 0.03 / 191.74 (preempted) | 0.89 / 0.92 / 34.75 (under 100 us) | 0.91 / 0.94 / 26.59 (under 100 us) | 0.10 / 0.11 / 24.61 (under 100 us) | recvmmsg=1.00 sendmmsg=0.00 | recvmsg=0.97 |
| L idle-samestream | 32 | 0.62 / 0.68 / 1181.69 (preempted) | 2.02 / 2.13 / 2091.01 (preempted) | 1.07 / 1.14 / 642.05 (preempted) | 1.29 / 1.41 / 48.58 (under 100 us) | epoll_wait=0.99 recvmmsg=0.02 sendmmsg=0.00 | epoll_wait=0.30 recvmsg=0.01 |
| L idle-samestream | 64 | 1.48 / 1.55 / 9306.11 (preempted) | 3.92 / 4.11 / 13631.49 (preempted) | 1.51 / 1.57 / 13148.16 (preempted) | 3.07 / 3.26 / 368.13 (preempted) | epoll_wait=0.99 recvmmsg=0.02 sendmmsg=0.00 | epoll_wait=0.12 recvmsg=0.01 |
| L idle-samestream | 128 | 3.81 / 4.00 / 279.55 (preempted) | 9.67 / 12.23 / 13828.09 (preempted) | 2.54 / 2.72 / 12124.16 (preempted) | 9.24 / 11.90 / 285.95 (preempted) | epoll_wait=0.99 recvmmsg=0.02 sendmmsg=0.00 | epoll_wait=0.04 recvmsg=0.00 |
| D idle | 1 | 0.03 / 0.03 / 236.03 (preempted) | 0.74 / 0.79 / 236.80 (preempted) | 0.71 / 0.77 / 2834.43 (preempted) | 0.11 / 0.19 / 33.79 (under 100 us) | epoll_wait=1.00 recvmmsg=0.00 sendmmsg=0.00 | epoll_wait=0.98 recvmsg=0.00 |
| D idle | 4 | 0.07 / 0.09 / 1006.59 (preempted) | 0.81 / 0.88 / 1758.21 (preempted) | 0.76 / 0.84 / 353.28 (preempted) | 0.20 / 0.25 / 31.14 (under 100 us) | epoll_wait=0.99 recvmmsg=0.00 sendmmsg=0.00 | epoll_wait=0.89 recvmsg=0.00 |
| D idle | 32 | 0.60 / 0.66 / 44.19 (under 100 us) | 2.28 / 2.40 / 106.81 (preempted) | 1.09 / 1.16 / 377.09 (no switch) | 1.40 / 1.49 / 2022.40 (preempted) | epoll_wait=1.00 recvmmsg=0.00 sendmmsg=0.00 | epoll_wait=0.29 recvmsg=0.00 |
| D idle-sameport | 1 | 0.03 / 0.03 / 25.18 (under 100 us) | 0.71 / 0.79 / 27.52 (under 100 us) | 0.71 / 0.81 / 26.00 (under 100 us) | 0.11 / 0.16 / 24.57 (under 100 us) | epoll_wait=0.99 recvmmsg=0.00 sendmmsg=0.00 | epoll_wait=0.97 recvmsg=0.00 |
| D idle-sameport | 4 | 0.07 / 0.09 / 336.89 (preempted) | 0.81 / 0.89 / 206.72 (preempted) | 0.76 / 0.84 / 171.26 (preempted) | 0.20 / 0.25 / 24.98 (under 100 us) | epoll_wait=0.99 recvmmsg=0.00 sendmmsg=0.00 | epoll_wait=0.88 recvmsg=0.00 |
| D idle-sameport | 32 | 0.59 / 0.67 / 25919.49 (preempted) | 2.02 / 2.17 / 9879.55 (preempted) | 1.08 / 1.15 / 10723.33 (preempted) | 1.31 / 1.41 / 552.45 (preempted) | epoll_wait=0.99 recvmmsg=0.00 sendmmsg=0.00 | epoll_wait=0.31 recvmsg=0.00 |

### Traffic duty cycle p50 / p99 (µs)

| Run arm | N | ping or client pass | sender (or net) | receiver | conductor |
|---|---|---|---|---|---|
| L plain | 1 | 0.04 / 0.04 | 1.74 / 3.94 | 1.75 / 2.27 | 0.13 / 0.19 |
| L plain | 32 | 0.65 / 0.75 | 4.09 / 8.67 | 1.50 / 2.42 | 3.24 / 3.56 |
| L plain | 64 | 1.73 / 1.93 | 9.55 / 18.88 | 2.54 / 2.82 | 9.18 / 12.14 |
| L plain | 128 | 4.05 / 4.66 | 24.14 / 40.29 | 5.06 / 11.38 | 20.11 / 34.21 |
| L sameport | 1 | 0.04 / 0.04 | 1.74 / 3.93 | 1.75 / 2.27 | 0.13 / 0.20 |
| L sameport | 32 | 0.65 / 0.73 | 4.83 / 8.35 | 2.63 / 2.77 | 2.99 / 3.25 |
| L sameport | 64 | 1.75 / 1.94 | 10.04 / 18.35 | 3.60 / 4.27 | 9.01 / 11.89 |
| L sameport | 128 | 4.03 / 4.63 | 22.32 / 39.01 | 6.28 / 12.49 | 19.90 / 33.98 |
| L samestream | 1 | 0.04 / 0.04 | 1.80 / 3.95 | 1.79 / 2.28 | 0.13 / 0.21 |
| L samestream | 32 | 0.65 / 0.75 | 3.93 / 8.62 | 1.51 / 2.42 | 3.23 / 3.51 |
| L samestream | 64 | 1.72 / 1.90 | 9.55 / 18.72 | 2.59 / 2.85 | 9.22 / 12.12 |
| L samestream | 128 | 4.07 / 4.68 | 24.48 / 40.41 | 4.97 / 11.34 | 20.11 / 34.49 |
| L mux | 1 | 0.04 / 0.04 | 1.73 / 3.93 | 1.76 / 2.29 | 0.13 / 0.20 |
| L mux | 32 | 0.04 / 0.04 | 1.77 / 3.96 | 1.77 / 2.28 | 0.13 / 0.20 |
| L mux | 64 | 0.04 / 0.04 | 1.73 / 3.92 | 1.77 / 2.30 | 0.13 / 0.21 |
| L mux | 128 | 0.04 / 0.04 | 1.78 / 3.94 | 1.77 / 2.29 | 0.13 / 0.21 |
| L mux4 | 1 | 0.04 / 0.04 | 1.73 / 3.95 | 1.75 / 2.29 | 0.13 / 0.20 |
| L mux4 | 32 | 0.09 / 0.11 | 1.98 / 4.32 | 1.83 / 2.27 | 0.28 / 0.39 |
| L mux4 | 64 | 0.09 / 0.11 | 1.96 / 4.35 | 1.83 / 2.29 | 0.28 / 0.39 |
| L mux4 | 128 | 0.09 / 0.11 | 1.99 / 4.36 | 1.83 / 2.31 | 0.29 / 0.41 |
| L mux8 | 1 | 0.04 / 0.04 | 1.73 / 3.97 | 1.76 / 2.31 | 0.13 / 0.21 |
| L mux8 | 32 | 0.16 / 0.23 | 2.48 / 5.05 | 1.92 / 2.33 | 0.66 / 0.76 |
| L mux8 | 64 | 0.15 / 0.22 | 2.51 / 5.00 | 1.95 / 2.38 | 0.67 / 0.80 |
| L mux8 | 128 | 0.15 / 0.22 | 2.51 / 5.01 | 2.00 / 2.38 | 0.68 / 0.81 |
| B1 shard1 | 1 | 0.06 / 0.07 | 1.79 / 3.92 | 1.78 / 2.28 | 0.13 / 0.22 |
| B1 shard1 | 32 | 1.56 / 1.75 | 4.13 / 8.58 | 1.53 / 2.43 | 3.57 / 3.79 |
| B1 shard1 | 64 | 3.91 / 4.50 | 9.83 / 19.39 | 2.61 / 2.85 | 9.18 / 11.81 |
| B1 shard1 | 128 | 9.78 / 12.47 | 21.38 / 31.26 | 4.77 / 7.62 | 20.16 / 23.09 |
| B2 shard2 | 32 | 1.87 / 2.08 | 2.25 / 6.14 | 1.12 / 2.40 | 2.09 / 2.26 |
| B2 shard2 | 64 | 4.24 / 5.05 | 4.25 / 8.89 | 1.61 / 2.50 | 4.99 / 5.36 |
| B2 shard2 | 128 | 11.11 / 14.18 | 10.09 / 16.11 | 2.73 / 3.01 | 10.56 / 13.18 |
| B1 snshard1 | 1 | 0.05 / 0.07 | 3.32 / 5.87 | – | 0.13 / 0.20 |
| B1 snshard1 | 32 | 1.48 / 1.66 | 5.67 / 13.32 | – | 3.09 / 3.40 |
| B1 snshard1 | 64 | 3.56 / 4.23 | 13.23 / 24.50 | – | 8.88 / 11.60 |
| B1 snshard1 | 128 | 9.32 / 24.46 | 27.20 / 45.66 | – | 19.23 / 32.83 |
| B2 snshard2 | 32 | 1.61 / 1.79 | 3.14 / 8.29 | – | 1.79 / 1.96 |
| B2 snshard2 | 64 | 3.85 / 4.40 | 5.80 / 11.42 | – | 4.74 / 5.00 |
| B2 snshard2 | 128 | 9.93 / 12.72 | 14.18 / 21.61 | – | 10.05 / 12.65 |
| B2 snshard4 | 32 | 1.79 / 2.02 | 2.19 / 7.22 | – | 1.16 / 1.29 |
| B2 snshard4 | 64 | 4.48 / 5.13 | 3.27 / 8.52 | – | 2.58 / 2.77 |
| B2 snshard4 | 128 | 11.62 / 14.87 | 5.91 / 11.86 | – | 5.28 / 5.62 |
| C sameport | 32 | 0.64 / 0.72 | 5.09 / 8.65 | 2.65 / 2.79 | 3.24 / 3.52 |
| C sameport | 64 | 1.71 / 1.91 | 10.69 / 19.15 | 3.65 / 4.47 | 9.16 / 12.04 |
| C sameport | 128 | 3.85 / 4.58 | 22.83 / 39.90 | 6.21 / 12.38 | 20.21 / 34.40 |
| C mux | 32 | 0.04 / 0.04 | 1.73 / 3.92 | 1.76 / 2.28 | 0.13 / 0.22 |
| C mux | 64 | 0.04 / 0.04 | 1.74 / 3.98 | 1.78 / 2.31 | 0.14 / 0.24 |
| C mux | 128 | 0.04 / 0.04 | 1.77 / 3.96 | 1.80 / 2.32 | 0.14 / 0.21 |
| C mux8 | 32 | 0.16 / 0.23 | 2.50 / 5.13 | 1.98 / 2.38 | 0.65 / 0.76 |
| C mux8 | 64 | 0.15 / 0.22 | 2.51 / 5.04 | 1.97 / 2.39 | 0.63 / 0.75 |
| C mux8 | 128 | 0.15 / 0.21 | 2.53 / 5.01 | 1.97 / 2.39 | 0.68 / 0.80 |
| D sameport | 32 | 0.66 / 0.76 | 3.70 / 8.33 | 1.50 / 2.37 | 3.12 / 3.38 |
| D sameport | 128 | 4.16 / 4.78 | 21.25 / 39.13 | 5.24 / 11.38 | 19.81 / 33.85 |
| D mux8 | 32 | 0.15 / 0.22 | 1.42 / 4.93 | 0.88 / 2.26 | 0.69 / 0.82 |
| D mux8 | 128 | 0.15 / 0.22 | 1.38 / 4.92 | 0.87 / 2.26 | 0.66 / 0.78 |
| E64K plain | 128 | 4.11 / 4.78 | 25.66 / 41.38 | 5.05 / 11.34 | 21.34 / 36.00 |
| E16M plain | 128 | 5.22 / 8.10 | 35.58 / 54.21 | 8.10 / 15.51 | 27.14 / 43.52 |

### Memory after the timed run (MiB)

| Run arm | N | log files | log apparent | log allocated (du) | process RSS | RSS shmem | RSS anon |
|---|---|---|---|---|---|---|---|
| L idle | 1 | 2 | 0.4 | 0.0 | 114.8 | 47.5 | 62.9 |
| L idle | 32 | 64 | 12.2 | 0.5 | 196.3 | 80.4 | 111.4 |
| L idle | 64 | 128 | 24.5 | 1.0 | 213.6 | 81.6 | 127.5 |
| L idle | 128 | 256 | 49.0 | 2.0 | 218.3 | 83.8 | 130.0 |
| L idle-sameport | 1 | 2 | 0.4 | 0.0 | 182.0 | 47.5 | 130.0 |
| L idle-sameport | 32 | 64 | 12.2 | 0.5 | 215.2 | 80.4 | 130.3 |
| L idle-sameport | 64 | 128 | 24.5 | 1.0 | 217.5 | 81.6 | 131.5 |
| L idle-sameport | 128 | 256 | 49.0 | 2.0 | 219.9 | 83.7 | 131.7 |
| L idle-samestream | 1 | 2 | 0.4 | 0.0 | 183.7 | 47.5 | 131.7 |
| L idle-samestream | 32 | 64 | 12.2 | 0.5 | 217.5 | 80.4 | 132.6 |
| L idle-samestream | 64 | 128 | 24.5 | 1.0 | 219.4 | 81.6 | 133.3 |
| L idle-samestream | 128 | 256 | 49.0 | 2.0 | 223.9 | 83.8 | 135.6 |
| L plain | 1 | 4 | 0.8 | 0.8 | 220.4 | 49.0 | 166.9 |
| L plain | 32 | 128 | 24.5 | 24.5 | 267.9 | 96.6 | 166.8 |
| L plain | 64 | 256 | 49.0 | 26.0 | 288.1 | 99.9 | 183.7 |
| L plain | 128 | 512 | 98.0 | 28.0 | 299.4 | 104.5 | 190.4 |
| L sameport | 1 | 4 | 0.8 | 0.8 | 245.7 | 49.0 | 192.1 |
| L sameport | 32 | 128 | 24.5 | 24.5 | 291.5 | 96.6 | 190.3 |
| L sameport | 64 | 256 | 49.0 | 26.0 | 296.4 | 99.8 | 192.2 |
| L sameport | 128 | 512 | 98.0 | 28.0 | 299.2 | 104.3 | 190.5 |
| L samestream | 1 | 4 | 0.8 | 0.8 | 245.7 | 49.0 | 192.1 |
| L samestream | 32 | 128 | 24.5 | 24.5 | 291.5 | 96.6 | 190.3 |
| L samestream | 64 | 256 | 49.0 | 26.0 | 296.6 | 99.9 | 192.2 |
| L samestream | 128 | 512 | 98.0 | 28.0 | 306.1 | 104.5 | 197.2 |
| L mux | 1 | 4 | 0.8 | 0.8 | 250.5 | 49.0 | 197.0 |
| L mux | 32 | 4 | 0.8 | 0.8 | 248.7 | 49.0 | 195.2 |
| L mux | 64 | 4 | 0.8 | 0.8 | 248.7 | 49.0 | 195.2 |
| L mux | 128 | 4 | 0.8 | 0.8 | 250.6 | 49.0 | 197.0 |
| L mux4 | 1 | 4 | 0.8 | 0.8 | 248.7 | 49.0 | 195.1 |
| L mux4 | 32 | 16 | 3.1 | 3.1 | 238.5 | 53.6 | 180.4 |
| L mux4 | 64 | 16 | 3.1 | 3.1 | 240.4 | 53.6 | 182.3 |
| L mux4 | 128 | 16 | 3.1 | 3.1 | 238.5 | 53.6 | 180.4 |
| L mux8 | 1 | 4 | 0.8 | 0.8 | 235.8 | 49.0 | 182.2 |
| L mux8 | 32 | 32 | 6.1 | 6.1 | 244.7 | 59.8 | 180.4 |
| L mux8 | 64 | 32 | 6.1 | 6.1 | 244.6 | 59.8 | 180.4 |
| L mux8 | 128 | 32 | 6.1 | 6.1 | 246.5 | 59.8 | 182.3 |
| B1 shard1 | 1 | 4 | 0.8 | 0.8 | 131.6 | 48.8 | 78.5 |
| B1 shard1 | 32 | 128 | 24.5 | 24.5 | 239.7 | 96.3 | 139.0 |
| B1 shard1 | 64 | 256 | 49.0 | 26.0 | 248.1 | 99.4 | 144.2 |
| B1 shard1 | 128 | 512 | 98.0 | 28.0 | 277.9 | 103.8 | 169.8 |
| B1 snshard1 | 1 | 4 | 0.8 | 0.8 | 222.9 | 48.8 | 169.8 |
| B1 snshard1 | 32 | 128 | 24.5 | 24.5 | 272.7 | 96.3 | 172.0 |
| B1 snshard1 | 64 | 256 | 49.0 | 26.0 | 278.1 | 99.4 | 174.2 |
| B1 snshard1 | 128 | 512 | 98.0 | 28.0 | 287.3 | 103.8 | 179.1 |
| B2 shard2 | 32 | 128 | 24.5 | 24.5 | 307.9 | 175.4 | 128.1 |
| B2 shard2 | 64 | 256 | 49.0 | 26.0 | 418.9 | 178.6 | 235.8 |
| B2 shard2 | 128 | 512 | 98.0 | 28.0 | 433.5 | 182.8 | 246.1 |
| B2 snshard2 | 32 | 128 | 24.5 | 24.5 | 427.4 | 175.4 | 247.5 |
| B2 snshard2 | 64 | 256 | 49.0 | 26.0 | 432.9 | 178.6 | 249.9 |
| B2 snshard2 | 128 | 512 | 98.0 | 28.0 | 437.9 | 182.8 | 250.6 |
| B2 snshard4 | 32 | 128 | 24.5 | 24.5 | 618.5 | 333.8 | 280.2 |
| B2 snshard4 | 64 | 256 | 49.0 | 26.0 | 652.4 | 336.8 | 311.0 |
| B2 snshard4 | 128 | 512 | 98.0 | 28.0 | 679.5 | 341.1 | 333.9 |
| C sameport | 32 | 128 | 24.5 | 24.5 | 195.7 | 96.6 | 94.8 |
| C sameport | 64 | 256 | 49.0 | 26.0 | 257.9 | 99.8 | 153.7 |
| C sameport | 128 | 512 | 98.0 | 28.0 | 267.6 | 104.3 | 158.9 |
| C mux | 32 | 4 | 0.8 | 0.8 | 243.5 | 49.0 | 190.1 |
| C mux | 64 | 4 | 0.8 | 0.8 | 214.2 | 49.0 | 160.8 |
| C mux | 128 | 4 | 0.8 | 0.8 | 243.9 | 49.0 | 190.4 |
| C mux8 | 32 | 32 | 6.1 | 6.1 | 223.4 | 59.8 | 159.2 |
| C mux8 | 64 | 32 | 6.1 | 6.1 | 254.6 | 59.8 | 190.4 |
| C mux8 | 128 | 32 | 6.1 | 6.1 | 223.4 | 59.8 | 159.2 |
| D idle | 1 | 2 | 0.4 | 0.0 | 114.8 | 47.5 | 62.9 |
| D idle | 4 | 8 | 1.5 | 0.1 | 162.3 | 47.6 | 110.3 |
| D idle | 32 | 64 | 12.2 | 0.5 | 196.9 | 80.4 | 112.0 |
| D idle-sameport | 1 | 2 | 0.4 | 0.0 | 177.6 | 47.5 | 125.6 |
| D idle-sameport | 4 | 8 | 1.5 | 0.1 | 192.9 | 47.6 | 140.8 |
| D idle-sameport | 32 | 64 | 12.2 | 0.5 | 226.0 | 80.4 | 141.1 |
| D sameport | 32 | 128 | 24.5 | 24.5 | 274.3 | 96.6 | 173.2 |
| D sameport | 128 | 512 | 98.0 | 28.0 | 284.3 | 104.3 | 175.6 |
| D mux8 | 32 | 32 | 6.1 | 6.1 | 241.8 | 59.8 | 177.6 |
| D mux8 | 128 | 32 | 6.1 | 6.1 | 240.0 | 59.8 | 175.8 |
| E64K plain | 128 | 512 | 98.0 | 28.0 | 213.4 | 104.5 | 104.5 |
| E16M plain | 128 | 512 | 24578.0 | 28.0 | 213.5 | 104.5 | 104.6 |

Full rows (every thread, every arm): `layout-latency.csv` and `layout-memory.csv` here.
