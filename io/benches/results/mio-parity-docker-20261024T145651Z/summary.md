# mio parity gate: spread on the Docker Linux lane

The runs that set the gate's tolerance (`io/benches/parity/rule.rs`,
`TOL_ABS_NS` and `TOL_PCT`).

| | |
|---|---|
| host | Apple silicon Mac, Docker Desktop, 10 vCPUs, kernel `7.0.14-linuxkit`; other containers running |
| arm64 | native: `arm64-run1.log` … `arm64-run5.log` |
| amd64 | Rosetta emulation: `amd64-run1.log` … `amd64-run5.log` |
| image | `rust:1.99.0-bookworm` |
| command | `cargo test --release --manifest-path io/benches/Cargo.toml --test mio_parity -- --test-threads=1 --nocapture` |
| per test | 7 pairs; per pair and arm 2,000 warm-up and 10,000 timed round trips in 10 blocks of 1,000, the arms alternating A B B A |
| CPUs | client loop 9, echo peer 8 (the cpuset's last two) |

The logs print each pair's verdict line under the tolerance in force
when they ran: 0.5 µs for `arm64-run1`–`5` and `amd64-run1`–`3`, the final
max(1 µs, 10 %) for `amd64-run4`–`5`. The paired differences are raw
measurements and do not depend on it; the table below is computed from them.

Paired difference = the median over a pair's 10 sub-pairs of (mio block
p50 − `epoll-spin` block p50), µs; positive: mio slower.

| arch | N | pairs | pooled p50 | median diff | std dev | min / max | beyond +0.5 / −0.5 | beyond +1.0 / −1.0 |
|---|---|---|---|---|---|---|---|---|
| arm64 | 1 | 35 | 9.33–12.12 | +0.27 | 0.63 | −1.17 / +1.98 | 10 / 2 | 3 / 1 |
| arm64 | 16 | 35 | 9.79–13.12 | +0.21 | 0.85 | −2.50 / +1.83 | 10 / 4 | 4 / 2 |
| amd64 | 1 | 35 | 5.54–14.42 | +0.25 | 0.51 | −0.94 / +1.98 | 7 / 3 | 2 / 0 |
| amd64 | 16 | 35 | 5.58–14.08 | +0.19 | 0.54 | −0.54 / +2.00 | 10 / 1 | 3 / 0 |

- **All 20 gate tests passed under both tolerances.** The largest one-sided
  count was 3 of 7 (`epoll_ahead`, 0.5 µs).
- **mio is slower by 0.19–0.27 µs at the median.** Its read to
  `WouldBlock` sits between the echo's arrival and its decode, so it is
  timed; the structural tests count exactly one such read per echo.
- **The pooled p50 jumps between about 5.5 and 10 µs** within a run, in
  both arms: the VM's CPUs move between the Mac's core types. A pooled p50
  difference swung by up to 4 µs; the paired difference stayed within
  −2.50 to +2.00.
- **Tolerance:** at 0.5 µs a cell's worst rate on one side was 10 of 35, so
  6 of 7 pairs by chance about once in 350 tests. At 1.0 µs it was 4 of 35:
  about once in 70,000. `TOL_PCT` = 10 % keeps the band proportional on a
  slower loopback (standard deviation 4–8 % of p50 here) and equals the
  floor at 10 µs.
- **A harness defect this lane caught:** with `mio::Events` sized 1,024 the
  amd64 mio loop was 5.7–9.1 µs slower in every pair, `EpollAhead(7)` at
  N = 1 and 16 (`amd64-events1024.log`, reproduced for this file); with 64,
  the `Driver`'s `epoll_wait` batch, it is level. Under Rosetta
  `epoll_wait`'s cost grows with `maxevents`. Native arm64 did not show it.
- **The gate fails the other way too:** 2 µs of spinning added to the
  `Driver` arm before it timestamps an echo gave paired differences of
  −1.33 to −2.23 µs and `MioAhead(7)` (`arm64-driver-plus-2us.log`).
