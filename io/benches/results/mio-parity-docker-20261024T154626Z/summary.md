# mio parity gate: spread and 10 % injections, Docker Linux lane

The runs that set the gate's tolerance (`io/benches/parity/rule.rs`,
`TOL_PCT` = 5) and show that a 10 % slowdown of either loop fails it.

| | |
|---|---|
| host | Apple silicon Mac, Docker Desktop, 10 vCPUs, kernel `7.0.14-linuxkit`; other containers running |
| arm64 | native |
| amd64 | Rosetta emulation |
| image | `rust:1.99.0-bookworm` |
| command | `cargo test --release --manifest-path io/benches/Cargo.toml --test mio_parity -- --test-threads=1 --nocapture` (injection runs: `round_trip` filter) |
| per test | 7 pairs; per pair and loop 2,000 warm-up and 10,000 timed round trips, the loops taking turns round trip by round trip (A B B A); blocks of 200 round trips |
| CPUs | client loop 9, echo peer 8 (the cpuset's last two) |
| code | branch `parity-ergon` after the merge of `feat/0.1.34` (the send set) |

Paired difference = the median over a pair's 100 blocks of (mio block p50 −
`epoll-spin` block p50); % = of the pair's mean p50; positive: mio slower.

## Nothing injected: 10 runs, all passed

`arm64-run1.log` … `arm64-run5.log`, `amd64-run1.log` … `amd64-run5.log`.
Every log: `test result: ok. 12 passed; 0 failed`.

| arch | N | pairs | pooled p50, µs | median diff, µs | median % | std dev % | min / max % | beyond +5 % / −5 % |
|---|---|---|---|---|---|---|---|---|
| amd64 | 1 | 35 | 5.46–15.17 | +0.23 | +2.4 | 1.3 | −0.3 / +5.2 | 1 / 0 |
| amd64 | 16 | 35 | 5.67–15.33 | +0.25 | +2.1 | 1.4 | −0.5 / +4.4 | 0 / 0 |
| arm64 | 1 | 35 | 5.33–12.58 | +0.25 | +2.7 | 1.0 | +1.0 / +5.0 | 0 / 0 |
| arm64 | 16 | 35 | 5.33–11.33 | +0.25 | +3.1 | 1.1 | +0.8 / +6.2 | 2 / 0 |

- **mio is slower by about 0.25 µs, 2–3 %:** its read to `WouldBlock`
  sits between the echo's arrival and its decode, so it is timed. The
  structural tests count exactly one such read per echo.
- **The worst test had 1 of 7 pairs beyond 5 %**; failing needs 6 of 7.
- Three more runs per arch of the committed code (`*-final{1,2,3}.log`,
  the code above after `cargo fmt` and doc edits): all passed, 84 pairs
  +0.2 to +5.2 %, one beyond 5 %.
- Taking turns per round trip cut the spread: the earlier design (20 blocks
  of 1,000 round trips per pair, `mio-parity-docker-20261024T145651Z`) had
  a paired-difference standard deviation of 0.51–0.85 µs; here it is
  1.0–1.4 % of p50, about 0.1 µs.

## 10 % added to one loop: every injected test failed

A temporary patch to `round_trip` (reverted) spun after the echo for a
tenth of that round trip, on one loop only, so its round trips were 10 %
longer. A second patch (reverted) spun 1 µs, about 10 % of p50, inside the
`Driver`'s `epoll_wait` path whenever the wait reported events.

| injection | logs | min / max % | verdicts |
|---|---|---|---|
| `epoll-spin` round trip +10 % | `*-inject-epoll10pct-{1,2}.log` | −11.0 / −6.1 | `MioAhead(7)` in all 8 tests |
| mio round trip +10 % | `*-inject-mio10pct-{1,2}.log` | +10.0 / +16.2 | `EpollAhead(7)` in all 8 tests |
| `Driver` `epoll_wait` +1 µs | `*-inject-driver-1us-1.log` | −10.0 / −5.4 | `MioAhead(7)` in all 4 tests |

- The `Driver`-side injections land near −8 %, not −10 %: the 2–3 % that
  mio's extra read costs offsets them. The closest injected pair was
  −5.4 %; the farthest uninjected pair on that side was −0.5 %.
- **Tolerance 5 %** sits between the two: above every uninjected pair but
  three (+5.0 to +6.2 %, mio's side, never more than 1 in a test), and
  below every injected one.
