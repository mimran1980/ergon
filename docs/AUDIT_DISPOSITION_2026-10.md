# Documentation and benchmark audit — disposition

Review record for the 2026-10 cleanup of unsupported claims, broken
benchmark accounting and unverified recommendations. This is a **review
artifact**: it records what the audit found, what evidence backed each
finding, and what was done with it. It is not a tutorial and not a source
of performance figures. Live figures live in the run artifacts under
`target/md-dpdk-bench/`, `io/benches/results/` and the per-crate
`BENCHMARKS.md` files, each quoted from a named run.

**Verdicts used:** `wrong-aggregation` (figure is real but summarised
under a different definition than the prose claims), `no-artifact` (no run
row contains the figure), `wrong-fact` (contradicted by code or by the
run it cites), `contradicted-by-code`, `quantitative` (directional claim
with no number, no max, no run id), `unsupported` (claim whose evidence
does not establish it).

**Dispositions used:** `rewrite` (keep the measurement, fix the summary),
`correct` (replace with artifact values), `remove` (delete the claim),
`unverified` (label explicitly; never invent a number).

**Status, 2026-10-20, second entry.** Stage 1: the io rows below have
landed and were re-checked against the run artifacts; the clock and
cluster-md rows are open. Stage 2: `Work::from_meta` accounting, flat
sampling in the parity and cluster benches, the `slope: null` → median
estimator reader, the A/A verdict semantics, the unequal-validation-work
fix and the `sbe/BENCHMARKS.md` rows have landed. The AWS
polling-versus-interrupt recommendation is reconciled against the Graviton
fair run `20261019T175248Z` and scoped to it, with x86 labelled not
measured.

**Validation class is now matched, and it is enforced.** The gated body-wrap
decode pairs and body-only encode pairs time `wrap_unchecked` against
sbe-tool's zero-check wrap, with the extent proved once in an untimed
preflight (`assert_baseline_wrap_extent`, `assert_stream_wrap_extent`,
`assert_encode_extent`). `bench_decode_entry_point`'s gated arm is
`ergo-sbe_wrap`; the checked constructor is a separate informational arm,
`ergo-sbe_try_from`, which the gate does not read. `fairness_policy_test`
enforces the class for the composite, full-message and entry-point decode
arms, so the next pair cannot silently reintroduce a validating wrap in the
timer. Observation barriers are matched the same way
(`parity_arms_use_identical_observation_barriers`): every value one arm
makes opaque, its partner makes opaque at the same number of barriers and
the same program points.

**The clock-bench gap pass now measures its own memory, not the process's.**
`the_gap_pass_takes_no_page_faults_while_it_times` failed once under a
full-suite load with 226 page faults. Its fault count is `getrusage`'s
`ru_minflt`, which is the calling thread's only on Linux
(`RUSAGE_THREAD`); Darwin has no `RUSAGE_THREAD`, so the count there is the
whole process's and another thread's faults fail the test. Its gap assertion
could not see a fault at all: the read under test is an atomic counter, so
gaps are differences of the counter, not of time (a mutation that left
8,387,584 of 8,388,608 pages cold still reported `max_ns: 1.0`). The pass
now proves its buffer is resident with `mincore` — scoped to the buffer, so
nothing else in the process moves it — by probing the unvisited tail as it
runs, and only asserts the fault count where the kernel scopes it to the
thread. `mincore` is tested against a fresh anonymous mapping, which is
guaranteed cold, because a heap buffer may land on recycled resident pages.
30/30 runs of the crate suite held under ~3.2 TB of allocation churn.

**The cluster gate's "resolved breach" verdicts did not reproduce.** Three
`just bench-cluster` runs of *identical* code at `bfb4f1d3` on
`aarch64-apple-darwin` produced three different breach sets. Within-run ratio
CIs were ±0.5%; the same pair moved 3.5% between invocations
(`cluster_decode_session_message_header` measured 1.0236, 0.9916, 0.9887) and
the identity of the "resolved breach" rotated through
`decode_session_message_header`, `encode_claim_shaped_header_plus_app`, then
`encode_session_message_header` / `encode_session_keep_alive` /
`decode_session_event` / `encode_claim_shaped_header_plus_app`. The A/A null
band moved too — [0.9985, 1.0126] → [0.9918, 1.0078] → [0.9922, 1.0003] — but
by less than the pairs, so a single invocation's band was narrower than the
noise it is meant to represent. The gate was claiming a resolution the
harness does not have on this host.

The cause is aggregation, not ergon: no product code and no ceiling changed
between the three runs. One invocation's confidence interval is the wrong
uncertainty for a between-invocation effect. The gate now grades the
**envelope of every independent invocation** the runner took, against a
**repeated** A/A control (its own envelope across invocations), and resolves a
pair only where that envelope clears the ceiling and the control's envelope in
every invocation. `just bench-cluster` and `run-sbe-bench.sh` take
`BENCH_REPEATS` invocations (default 5) and snapshot each under
`repeats/<k>/`; a manifest declaring more invocations than are on disk is
refused. A pair the harness cannot separate across invocations establishes no
winner — which is what these three runs showed. Ceilings are unchanged and no
exception was added; `a_breach_stable_across_repeats_still_fails` holds that a
regression repeating in every invocation still fails.

**Keep-alive repeatability — the 1.0191 did not repeat.** A second full
`just bench-cluster` on the same host (run `cluster-20261020T072915Z-e565d087`,
commit `e565d087`, rustc 1.99.0, `aarch64-apple-darwin`) measures
no-LTO `cluster_encode_session_keep_alive` at **1.0001** (ergo 6985.70 ns /
sbe-tool 6984.90 ns, ratio CI [0.9968, 1.0039]) against the previous run's
**1.0191** (CI [1.0144, 1.0242]). The 1.9% gap is run-to-run, not a code
property. Both profiles now pass the gate with three pairs each marked
inconclusive and no established winner. A repeatable sbe-tool win has not
been shown on this host.

**Harness resolution on this host is about 2%.** The A/A control's own
ratio walked with it: the no-LTO null band moved from [0.9970, 1.0129]
(centre 1.005) to [0.9752, 0.9946] (centre 0.985) between the two runs.
Under LTO the same effect is visible inside one run — an identical keep-alive
closure measures **7.3092 µs** in `cluster/control/aa_encode_keep_alive`
against **7.3760 µs** in `cluster/encode/session_keep_alive`, 0.9% on
identical code, which is larger than that pair's own 0.63% spread. Every
maintained ratio difference (0.01%–1.1%) sits at or below that, so no
maintained cluster pair is resolvable on `aarch64-apple-darwin` today.

**Landed and re-verified since the first entry.** The clock rows
`clock/README.md:44` and `book/src/clock/overview.md:59` (the read-rate
cutoff and the mandatory recommendation): the cutoff is gone and the page
now reads "One option among four, not a requirement. Choose by contract,
not by read rate", with `Instant`/quanta/minstant/`SystemTime` each scoped
to what they actually give. The stage-4 example rows: mixed-scale price
comparison (`samples/l3-book/tests/l3_tests.rs` `mixed_scale_book` and
`best_bid_compares_prices_across_scales`, which assert `99` outranks
`1.00`), checked timestamp conversion (`book/src/sbe/recipes/timestamps.md`
`try_from_sbe` → `Result`, `timestamp_*_opt().ok_or(...)`), fallible DTO
decoding (`book/src/sbe/recipes/domain-dtos.md` `try_from_decoder` /
`try_from_slice_with_header` → `Result`, "materialisation is never
infallible"), and OUT_DIR as the product default (`book/src/samples/build-patterns.md`
and `samples/README.md` both label `src/generated/` "samples default
(gitignored, unpublished)" and `generate_to_out_dir` + `sbe_mod!` "product
default"). The three `scripts/book-fence-allowlist.txt` line numbers for
`book/src/tune/overview.md` (190, 198, 237) match the live fences.
`just policy` passes.

**Open, stage 2.** (a) The Callgrind instruction-count check
(`just bench-cluster-instructions`, `./scripts/run-cluster-instruction-probes.sh
--all-profiles`) needs Linux + Valgrind + `llvm-objdump` and fails closed;
it has not run, so `cluster_encode_session_keep_alive` has no mechanism
check. (b) The A/A null band is order-sensitive: each control is a single
`arm_a`-then-`arm_b` pairing, so within-run arm-order bias sits in the band's
centre. *Repeated* A/A controls are in place — the gate grades each pair
against the envelope of the control across independent invocations, which
absorbs a between-invocation offset — but the repeats are the same pairing
each time. The band is not widened to hide this: a pair outside a measured
band still fails. Open for arm order and per-pair placement only; see the
placement row below.

**Stage 3, 2026-10-20.** Clock is done (above). Deployment: the io_uring
grant is scoped to apps that use it (`latency-pods.md`: "Aeron's C media
driver does not use io_uring"); `k8s/nri-latency-pods` states "no IPC_LOCK,
no capabilities, no privileged, and no scheduler change"; and the three
experimental/diagnostic JVM flags are **removed** from
`cluster-test-harness/jvm/aeron.jvm` along with the two `Unlock*VMOptions`
that only admitted them. What is left of stage 3 is TLS: `io/src/tls.rs`
drives `UnbufferedClientConnection` for the handshake, then
`dangerous_into_kernel_connection` hands the secrets to a custom
`RecordLayer` that runs ring AEAD in place and owns post-handshake parsing,
reassembly, KeyUpdate and the confidentiality limit. That layer is the live
data path (`stream.rs`, `ws_conn.rs`, `net.rs`), so replacing it with
rustls's maintained unbuffered API is a refactor of the post-handshake
path, not a deletion. The only evidence for keeping it is a decrypt-only
microbench whose aggregation this audit has already rejected
(`io/benches/BENCHMARK.md:122`), so it is **not justified** — it is
scheduled for `feat/0.1.31`. The `SCHED_FIFO` knob in `k8s/nri-shared-cpus`
stays, default-off and measured no gain on an isolated AWS CPU; whether to
delete it is the same branch's decision.

**Closed, stage 4, 2026-10-21.** The example rows are done (above). The
reconciliation sweep is done: README, rustdoc, the disclosure, the charts
and the CI descriptions were read against each other, and every claim named
in section 1 is now fixed in the tree — verified by re-reading the cited
line, not by assuming. Section 6 gives a disposition line for every book
file (84 of 84). Concision is a separate axis from correctness: the
Deployment part was trimmed to a scannable form on 2026-10-21 (method moved
out of `deploy/benchmarks.md` into `md-dpdk/BENCHMARKS.md`, status essays
cut from `deploy/shared-cpus.md`, the notation preamble cut from
`deploy/hft.md`). Nothing in this file certifies every historical
experiment, and nothing here establishes that any library meets an
unspecified HFT latency budget.

---

# ergon documentation cleanup disposition

Root: `/Users/mimra/RustroverProjects/project/ergon`. Evidence quotes are artifact rows or source lines as given in the verified inventory. No new measurements.

## 1. Disposition table

### clock

| file:line | claim | verdict | evidence | disposition | exact fix |
|---|---|---|---|---|---|
| `clock/BENCHMARKS.md:58` | Discipline residual table: "median of 5 reps"; c7i `2 \| 2,726 \| 9,443 (1,447–9,443)`, c8g `3 \| 282 \| 1,447 (1,447–2,100)`; c8g/c9g update cost both "max 1.8–2.9 µs" | wrong-aggregation | Six `discipline_summary,residual_at_samples_ns` rows, one 1800 s run per node (n=1801): c7i `1801,3,2726,7019,9443`/`1801,2,685,2058,3950`; c8g `…532…2154`/`…291…1447`; c9g `…282…2580`/`…397…2657`. c7i's range low 1,447 is c8g-b's max. Update maxima c8g 2894/2895, c9g 1759/1840 | rewrite | Per-host ranges across the two nodes: c7i p50 2–3, p99 685–2,726, max 3,950–9,443; c8g p50 2, p99 291–532, max 1,447–2,154; c9g p50 2, p99 282–397, max 2,580–2,657; update maxima c8g 2.89–2.90 µs, c9g 1.76–1.84 µs. Header: "one 1800 s run per node, n=1801 samples, two nodes". |
| `clock/BENCHMARKS.md:64` | c9g row `2 \| 310 \| 1,610 (1,610–2,400)`; c8g max high `2,100` | no-artifact | No `discipline_summary` row has 310, 1,610, 2,400 or 2,100. Real c9g: p99 282–397, max 2,580–2,657; c8g-a max 2154 | correct | Replace with artifact values (c9g p99 282–397, max 2,580–2,657; c8g max 1,447–2,154). |
| `clock/BENCHMARKS.md:13` | `rdtscp` 20.00 (21.08) etc.; read-cost "ordered `now_ns`" column `20.00 (21.08) \| 19.48 (19.79) \| 11.36 (11.67)` (also :44) | wrong-fact | Those figures are `cost,ergon_ticks_ordered` (`20.00,20.94,21.08`). Real `cost,ergon_now_ns_ordered`: `22.73,23.79,23.88` / `21.93,22.33,22.34` / `11.35,11.65,11.65`. `raw_rdtscp` max is 21.11, not 21.08 | correct | Quote `ergon_now_ns_ordered` in the now_ns column (22.73 (23.88) / 21.93 (22.34) / 11.35 (11.65)); or rename the column "ordered ticks read" (costs stay ns). On :13 label as `ergon_ticks_ordered`, or use raw_rdtscp's 20.00 (21.11). |
| `clock/BENCHMARKS.md:52` | Drift "within 0.01 ppm" on all three hosts beside "rate vs kernel scaling \| −0.048 to 0.000 ppm" | wrong-fact | `drift,ergon_now_ns,60,…,-2851,-0.048,raw` (c7i) and `…,3,0.000,raw` (c8g). c9g `drift_secs=0`, no drift rows. The two columns are one measurement | rewrite | c7i "−0.048 ppm at 60 s and 120 s"; c8g "0.000 ppm"; c9g "not measured (drift_secs=0)". Drop or relabel the duplicate "rate vs kernel scaling" column. |
| `clock/BENCHMARKS.md:12` | quanta process disagreement "1.9–6.9 ppm" (also :56, :94) | wrong-aggregation | `calib,quanta` spread: 6.866, 6.866, 4.768, **217.389**, 1.907, 1.907 (aws-20261016-223331/b: one process 224 ppm low) | rewrite | Report per-node spreads (1.9–6.9 ppm except 217.389 ppm on one c8g node), or "up to 217 ppm". |
| `clock/BENCHMARKS.md:11` | quanta held cost 15.99 / 12.50 / 10.92 with no brackets under "slowest batch in brackets" (also :44) | unsupported | `cost,quanta_now_ns` maxima 16.95 / 12.89 / 11.23 | correct | Add maxima: 15.99 (16.95), 12.50 (12.89), 10.92 (11.23). |
| `clock/BENCHMARKS.md:24` | Cost method "10,000 reads per batch, 200 batches"; calibration 20 processes | wrong-fact | c9g both nodes: `batches=50 … drift_secs=0 spawns=5`; `calib,ergon,5,…` | rewrite | State per-host settings, or mark the c9g column "50-batch / 5-process quick run". |
| `clock/BENCHMARKS.md:83` | Ordered read "3–8 ns dearer on every host; c7i's gap is the largest (7.5 ns)" | wrong-fact | now_ns gaps: c7i 7.57, c8g 9.76, c9g 0.43 | correct | Give per-host gaps (7.6 / 9.8 / 0.4 ns) and say c8g's is largest. |
| `clock/BENCHMARKS.md:84` | "gaps between consecutive reads are sub-ns on Graviton, 1–2 ns on c7i" | no-artifact | `delta,ergon_now_ns` medians 11–15 ns; Open item :100 admits the fixed gap pass has not run | delete | Delete the sentence, or write "not measured" until the fixed gap pass runs. |
| `clock/BENCHMARKS.md:14` | Calibration "40–70 ms, at most 70.226 ms" (also :52) | wrong-fact | Cited chrony runs give calib max 70.223 (aws-20261016-231320); 70.226 is aws-20261016-085442 | correct | Quote 70.223 from the cited run and name the CSV; or name 085442 if 70.226 is kept. |
| `clock/BENCHMARKS.md:94` | Stamps "must agree … Keep it on every host in the stack" | overstated | Same page :16-18 says quanta is fine within one process; no workload requirement behind "must"/"every host" | rewrite | "Where stamps are compared across processes or hosts, keep one rate on every host that exchanges stamps." |
| `clock/README.md:16` | calibrate "plus up to 50 ms for the time-sync probe" | no-artifact | 50 ms is `chrony_timeout` (clock/src/timesync.rs:969); probe duration not measured | relabel not-measured | "plus a short time-sync probe (50 ms chronyd answer timeout; duration not measured)". |
| `clock/README.md:44` | "Under about 100,000 reads a second: `std::time::Instant` is enough" | overstated | No artifact or workload requirement for the cutoff | rewrite | State the workload requirement or "not measured"; match the overview.md change. |
| `book/src/clock/overview.md:59` | 100,000 reads/sec cutoff; "needs UTC stamps … ergon-clock"; "busy-spin … ergon-clock" | overstated | No artifact for the cutoff; SystemTime/minstant also produce UTC (minstant-0.1.7 `as_nanos()`); clock/BENCHMARKS.md:11 "speed is no reason either way" | rewrite | State the workload requirement behind the cutoff (or "not measured"); scope UTC/busy-spin rows to "UTC at counter cost, or stamps compared across processes/hosts"; note quanta is equally fast in one process. |
| `book/src/clock/overview.md:135` | minstant "reads a plain `_rdtsc()`"; table "ordered read \| yes \| no \| no" | contradicted-by-code | minstant-0.1.7: `_rdtsc` is x86-only (tsc_now.rs); on Arm/macOS it is `SystemTime` (ordered wall clock) | correct | "on x86 minstant reads a plain `_rdtsc()` (unordered); on Arm and macOS it is `SystemTime`, an ordered wall-clock read". Table cell: "x86 no / Arm yes (wall clock)". |
| `book/src/clock/wall-clock.md:52,76` | Links to `#disciplined-clock-residual` and `#time-sync-in-these-runs`; "slews after boot reached 533 ppm" | unsupported | Neither anchor exists in current clock/BENCHMARKS.md (time-sync section deleted in 128747b9). 533.462 ppm is real in aws-20261016-083958/084001/085442 | correct | Retarget both links to live sections or the reference-run CSVs; keep 533.462 ppm with the run named. |
| `book/src/clock/overview.md:257` | Stall "was 7.4 / 3.2 / 3.9 µs — the VM pausing the vCPU" | overstated | Figures match `delta` maxima 7351/3153/3867 ns, but clock-summary.md marks traced gaps "unexplained" | rewrite | "the cause is unexplained in the guest; the platform's vCPU is suspected" — or "not measured". |

### deploy

| file:line | claim | verdict | evidence | disposition | exact fix |
|---|---|---|---|---|---|
| `book/src/deploy/latency-pods.md:12,145,150` | "`SCHED_FIFO`, any `sched_setscheduler`, a `--fifo` wrapper" under "Deliberately not built"; "The plugin never changes a scheduler policy" | contradicted-by-code | k8s/nri-shared-cpus builds it: `cmd/ergon-cpus/main.go:9,213` `--fifo`/`SetFifo`; `internal/wrap/wrap.go:312-326` `SCHED_FIFO`; `sched-fifo.ergon.io/` in names.go:45, grants.go:80-96; shared-cpus.md:248 documents `schedFifo: off\|thread`. Default-off and measured worse | rewrite | 1. Remove the SCHED_FIFO row from "Deliberately not built" (keep SR-IOV, netns-sysctl, IRQ). L12: "built but default-off (`ergon-cpus --fifo`, `schedFifo: thread`), measured to give no gain on an isolated CPU on AWS". 2. L150: "The NRI plugin only grants `RLIMIT_RTPRIO` and `ERGON_SCHED_FIFO`; the wrapper or the app/prelock takes `SCHED_FIFO` on the hot thread. On the default no-FIFO path PID 1 stays `SCHED_OTHER`; with `--fifo` the exec'd command runs `SCHED_FIFO`." 3. Keep shared-cpus.md:132 consistent. **Fixed 2026-10-20** (docs). The plumbing itself stays: default-off, measured no gain, a 0.1.31 decision whether to delete the knob. |
| `book/src/deploy/java.md:68` | `GuaranteedSafepointInterval=300000` "Removes the stop-the-world safepoint that otherwise runs every second" (also overview.md:51) | unsupported | aeron-samples/scripts/java-common:31 sets the flag only. JDK 27 idle 6 s: zero safepoints logged | remove | The flag is gone from `cluster-test-harness/jvm/aeron.jvm`, with `TrustFinalNonStaticFields` and `DebugNonSafepoints` and the two `Unlock*VMOptions` that only admitted them. Every option left is either required to start or has a measured reason. overview.md:51 now defines a safepoint and points at the logging. **Fixed 2026-10-20.** |
| `book/src/deploy/java.md:91` | "A cluster's durability comes from its quorum, not from fsync" (justifies sync level 0) | overstated | aeron-archive README "Recording Durability": level 0 is page-cache only. aeron-cluster README: quorum is durability for a majority; single-node clusters exist | rewrite | "In a 3- or 5-member cluster the quorum is the durability mechanism, so sync level 0 is the usual choice; a single-node cluster's durability is fsync — raise both sync levels there." |
| `book/src/deploy/overview.md:50` | "`ParallelGC` … wins" | overstated | java.md:70 "convention, not a measured collector A/B"; :129 "**not measured**" | relabel not-measured | "so `ParallelGC` is what every Aeron script uses (convention, not a measured collector A/B)". |

### io

| file:line | claim | verdict | evidence | disposition | exact fix |
|---|---|---|---|---|---|
| `io/benches/BENCHMARK.md:101` | "stock rustls by default for TLS … not the default recommendation" (also :122) | contradicted-by-code | io/src/stream.rs:24,47: `tls: RecordLayer` required on every `TlsStream`; io/src/tls.rs:1-2 rustls handshake only, in-place decrypt. Repeats confirmed defect (book/src/io/overview.md:177) | rewrite | State the in-place `tls::RecordLayer` is the stack used (rustls handshake only); delete the "stock rustls by default" recommendation at 101 and 122. Keep decrypt-only figures as a labeled microbench. |
| `io/benches/BENCHMARK.md:151` | "ENA adaptive moderation off moved `recv-busy`'s p99.99 by about −3.0 µs (the 40 µs figure is p99.99 under load)" | wrong-fact | flags-aws-20261015T070339Z tables.md pp-recv-busy-10000: `c1-nodim 40.88 (−3.02) … 71.89 (−46.96)`, `c2-nodim 40.93 (−2.97) … 76.02 (−42.83)`. −3.0 is p50; p99.99 delta is −47/−43. Same file :50,:87,:125 say "p99.99 falls by about 40 µs" | correct | "adaptive-rx off moved `recv-busy`'s p50 by about −3.0 µs and its p99.99 by −47/−43 µs (runs `062428Z`, `070339Z`)"; delete the parenthetical. |
| `io/benches/BENCHMARK.md:56` (also :65, :99, :120, :149; `book/src/io/http-client.md:99-100`) | "HMAC-SHA256 262 ns against Ed25519 21,558 ns" as cost per request | wrong-aggregation | sign_bench.md header "ns per signed request (best of 7)" — minima, no max | relabel not-measured | Label "best of 7, max not recorded" everywhere, or re-run `sign_bench` with p50 and max. Fix http-client.md:100 nested/broken link while there. |
| `io/benches/BENCHMARK.md:122` | decrypt-only "in-place 105/175/429/1,464; stock rustls 181–2,607; OpenSSL 460–1,951" | wrong-aggregation | tls_bench.md "best of 5", no max. "181–2,607" merges rustls-unbuffered (`181.0…1669.6`) and rustls-buffered-ring-copy (`231.4…2607.1`) | rewrite | One implementation per payload as in the artifact, each "best of 5, max not recorded"; do not merge the two rustls builds. Decision cell per the TLS row above. |
| `book/src/io/http-client.md:116` | "One copy of every figure, with its max: …#4-http-clients" | wrong-fact | Anchor gone (section cut in 128747b9). Surviving BENCHMARK.md quotes p50 only, no max | rewrite | Restore the p50/p99/p99.9/max HTTP rows under a live anchor, or "p50 only; maxima live in git history" with a working link. |
| `book/src/io/rate-limits.md:185` | `try_charge` cost table (187-197), "the run is **not archived**" | no-artifact | Nothing in tree or git holds `limit_bench` results | relabel not-measured | Delete the table or mark every cell "not measured"; keep the reproduce recipe until a run is archived. |
| `book/src/io/rate-limits.md:183` | "p50 and p99 per call" = percentiles of 64-call batch means; max = slowest batch in µs | wrong-aggregation | Header "p50 ns \| p99 ns \| max batch µs" mixes per-call percentiles with a per-batch max; :211 "p50 is the number to use" | rewrite | If kept after archiving: per-call p50/p99/max in ns; per-batch max in its own table. |

### sbe

| file:line | claim | verdict | evidence | disposition | exact fix |
|---|---|---|---|---|---|
| `sbe/benchmarks/benches/perf_parity_bench.rs:16` | "Fair comparison uses `wrap` (sbe-tool's `wrap` does no bounds check)" | contradicted-by-code | ergo arm :333 runs the version-aware extent proof + panic branch inside the timer (`codegen/message_decoder.rs:346-371`); sbe-tool arm :352 is field assigns only. Violates the keep-alive rule quoted at BENCHMARKS.md:394-396 and the validation-class rule :57-64 | rewrite | Pair `wrap_unchecked` against sbe-tool's zero-check wrap (extent proven in the untimed preflight), or charge both arms the check; replace :16 with the keep-alive rule; correct the :329 "lean wrap (4 field assigns)" comment. **Fixed 2026-10-20.** |
| `sbe/BENCHMARKS.md:62` | Diagnostic arms `ergo-sbe_wrap_checked`, `ergo-sbe_try_from`, `ergo-sbe_header_and_body_checked`; "the gate does not read them" | wrong-fact | `ergo-sbe_wrap_checked` / `ergo-sbe_header_and_body_checked` exist only in this sentence and methodology.md:53. The gate reads `ergo-sbe_wrap`, which runs the checked constructor | rewrite | List only `ergo-sbe_try_from`; state that until the entry-point arm is swapped, the gated `ergo-sbe_wrap` runs the checked constructor. **Fixed 2026-10-20.** |
| `sbe/BENCHMARKS.md:422` | "The checked constructor's own cost is 21 instructions" | overstated | 21 is llvm-objdump static text size, mostly never-executed cold panic path; success path delta is `cmp`/`ja` | relabel not-measured | "static text-size delta of 21 instructions (mostly never-executed cold panic path)"; executed cost "not measured" or cite Callgrind Ir/op. **Fixed 2026-10-20.** |
| `sbe/BENCHMARKS.md:497` | equal 80/80 counts prove "the arms retire the same work" and exonerate the wall-clock gap | unsupported | Static text sizes only; line 365-367 "Differing instruction texts are 14 per pair"; Callgrind lane still needed | relabel not-measured | State only that static text sizes match across profiles; drop the exoneration, or write "retired work not measured". **Fixed 2026-10-20.** |
| `sbe/BENCHMARKS.md:582` | "Packing does **not** beat the wire-image design … ~60x slower than any single-field access" | overstated | Unprovenanced run: one composite, one field, test buffer | rewrite | Scope to the measured case ("in this unprovenanced run, for the f15 field, …") or "not measured". **Fixed 2026-10-20** (claim removed). |
| `sbe/benchmarks/tests/bench_gate_test.rs:372` | overrides "stay within 1.01" | wrong-fact | Test enforces `PROFILE_SCOPED_OVERRIDE_MAX` with `session_event` → 1.05 (`scripts/check-bench-gate.sh`) | correct | "stay within its documented maximum in `PROFILE_SCOPED_OVERRIDE_MAX`". Do not change any ceiling. |

### cluster-md

| file:line | claim | verdict | evidence | disposition | exact fix |
|---|---|---|---|---|---|
| `book/src/md-dpdk/overview.md:243` (also :20-23, :82, :159-161) | Localhost row `5.48 / 8.18 / 10.73` vs `5.58 / 7.39 / 8.86`; glossary "max = the slowest single round trip" | wrong-aggregation | aws-20261015-012400 rtt isolated:local-dpdk per-rep maxima 10727, 664575, 11159, 9983, 9967 ns. Printed 10.73 is the median of those; absolute worst 664.58 µs. kernel 8.86 vs 9.46 (small) | rewrite | Print "max 10.73 (worst rep 664.6)" or the absolute worst; reword :82 and :159-161 so "max" is labeled median-of-rep-maxima and the worst rep is named; fix the banner :20. |
| `md-dpdk/README.md:23` (also :23-29) | "p50 / p99.99 / max: 5.48 / 8.18 / 10.73 µs against … 5.58 / 7.39 / 8.86 µs"; Graviton "max higher (19 against 15)" | wrong-aggregation | Same rows; Graviton aws-20261016-072005 maxima 18.66/35.07/12.30/19.86/16.86 vs 20.25/12.81/15.03/13.83/15.29. No method note; README drops the range BENCHMARKS.md prints | rewrite | Quote absolute maxima (664.6 / 9.5 µs) or label "median of 5 rep maxima" with the worst rep beside it; add the one-line method. |
| `book/src/md-dpdk/overview.md:222` (also :220-221, :201-203, :179-181) | Between-hosts max columns (e.g. Azure DPDK "20.11 / 30.29 / 71", kernel-busy "101.50 / 21,152") under "max is the slowest one" | wrong-aggregation | Printed medians of 5 rep maxima hide the worst: 71 hides 447.5; 289 hides 2,740; 82 hides 449; 21,152 hides 33,325; 2,574 hides 13,943; 599 hides 6,664; G1 75 hides 154.9, 107 hides 166.1 | rewrite | Give each max as the absolute worst, or median (worst), matching BENCHMARKS.md's "max 71 (48–845)"; never define the column as "the slowest one" while printing a median. |
| `book/src/md-dpdk/overview.md:149` | busy-poll ring "under load its max reached 0.6 ms on c7i and 21 ms on Azure" | wrong-aggregation | 0.6/21 ms are medians of rep maxima; peaks reached 6.7 ms (aws-20261015-185813) and 33.3 ms (20261014-173745) | correct | "its max reached 6.7 ms on c7i and 33 ms on Azure (median of rep maxima 0.6 and 21 ms)". |
| `md-dpdk/README.md:153` | "two 288 B flows 5,070 NAKs → none" | no-artifact | 5,070 appears nowhere in BENCHMARKS.md, summaries or results. aws-20261015-174836/isolated/run.log: two-flow points discarded ("windows overlapped 0.001 … analyse.py leaves the run out") | delete | Delete the clause, or "two-flow effect not measured (points discarded: overlapping windows)". Keep 400 k → 600 k. |
| `md-dpdk/BENCHMARKS.md:27` | "the 8 cores of a c7i.4xlarge pair ($1.428)" | wrong-fact | c7i.4xlarge is 8 cores × 2 with HT = 16 physical per pair (overview.md:195); c9g.2xlarge is 8, no SMT (overview.md:173) | correct | "the 8 cores of a c7i.4xlarge (16 per pair) at half the price" — name cores per host or the pair's 16. |

### rest

| file:line | claim | verdict | evidence | disposition | exact fix |
|---|---|---|---|---|---|
| `README.md:101` (also `CONTRIBUTING.md:64`, book include) | PR CI "executes 32-bit x86 plus big-endian s390x codec tests" through cross/QEMU | no-artifact | No such step in any workflow. Only BE-adjacent lane is weekly Miri on sbe/miri-fixtures (nightly.yml:48) | delete | Delete the clause in README.md and CONTRIBUTING.md. Optionally: "not run in CI yet; LE/BE fixture codecs are plain-tested on PR CI and Miri-tested weekly". |
| `README.md:102` (also `CONTRIBUTING.md:60-61`) | "critical-path mutation testing weekly" | contradicted-by-code | justfile:410-413 "MANUAL gate — too slow for CI"; no workflow runs `cargo mutants`; only `check-mutation-config.sh` | rewrite | "mutation testing is manual (`just check-mutation`); CI enforces only the mutation config (`check-mutation-config.sh`)". |
| `README.md:102` (also `CONTRIBUTING.md:62`) | "Miri fixtures nightly" | wrong-fact | nightly.yml cron `"23 17 * * 0"` — weekly Sunday | correct | "weekly (Sundays)" in both files (or change the cron). |
| `book/src/project/road-to-1.0.md:86` | semver-checks "against the 0.1.21 baseline" | wrong-fact | api/public-api-baseline.toml:63 `baseline_tag = "v0.1.26"` | correct | "the `baseline_tag` baseline in `api/public-api-baseline.toml` (currently 0.1.26)". |
| `book/src/tune/overview.md:3,129` | "Each finding says … what it cost in this repository's measurements"; "One A/B per check" | overstated | Same page :135-137: T4, T5, T6, T9, N2, N3 not measured | rewrite | "where measured (unmeasured checks say so and cite the vendor's documentation)"; "One A/B per measured check". |
| `book/src/samples/build-patterns.md:9` | `src/generated/` pattern is "(product / samples default)"; OUT_DIR "fine for apps; poor IDE go-to-def" | contradicted-by-code | cluster/build.rs:36 uses `generate_to_out_dir`; introduction.md:9, README.md:18 document OUT_DIR + `sbe_mod!`. Only unpublished samples write `src/generated/` | rewrite | Label `src/generated/` "samples default (unpublished, gitignored)"; OUT_DIR + `sbe_mod!` is the product/default pattern. |
| `book/src/samples/overview.md:12` (also `samples/README.md:12`) | L3 Order Book "build-dep only (plain `include!`)" | contradicted-by-code | samples/l3-book/src/lib.rs:21 is `#[path = "generated/l3_codec.rs"]` | correct | "build-dep only (`#[path = "generated/…"]`, not `include!`)" in both files. |
| `book/src/project/package-scope.md:18` | "Cluster codecs stay crate-private" | contradicted-by-code | cluster/src/lib.rs:106-111 `#[doc(hidden)] pub mod cluster_codec_types` | correct | "not a stable consumer API (`#[doc(hidden)] pub mod cluster_codec_types`, tests and benches only); use `AeronCluster`". |
| `book/src/samples/buffer-sizing.md:3` | "Every protocol buffer must be sized using the generated EncodedLength API. Never guess with `vec![0u8; 4096]`" | overstated | Generated MAX_ENCODED_LENGTH doc sanctions a true fixed bound stack array; :26 of the same page is the defensible wording | rewrite | "Size buffers with the generated length APIs; a `MAX_ENCODED_LENGTH` stack array is fine when that constant is a true fixed bound. Avoid guess constants like 4096." |

### Prior confirmed defects still needing cleanup

| file:line | claim | verdict | evidence | disposition | exact fix |
|---|---|---|---|---|---|
| `book/src/deploy/benchmarks.md:28,37` (metal row) | "max is the slowest single round trip"; metal DPDK idle max 71 µs | wrong-aggregation | aws-20261015-012400 cross-host per-rep maxima; worst repetition 845311 ns; 71 is the median of 5 rep maxima | rewrite | Report the absolute worst (or "median (worst)"); reword the method prose so "max" is not defined as the slowest single round trip. |
| `io/benches/src/stats.rs` (`Work::from_meta`) | counts accepted without `Work::check()` | contradicted-by-code | Contradictory counts (scheduled 1, issued 8, completed 7, censored 5) accepted | correct | Call `Work::check()` in `from_meta` (or at the load boundary) and report malformed meta instead of accepting it. |
| `samples/l3-book/tests/l3_tests.rs` (`best_bid_and_depth`) | Decimal equality of prices | wrong-fact | Compares mantissas, ignores exponents | correct | Compare `rust_decimal::Decimal` values (mantissa and scale), or normalise both sides before asserting. |
| `scripts/book-fence-allowlist.txt` | three fence allowlist entries at 186, 194, 233 | wrong-fact | Live fences are at 188, 196, 235 (`book/src/tune/overview.md`) | correct | Update the three line numbers to the live fence lines. |

## 2. Stages

### (1) Restore trustworthy evidence
Wrong aggregation, no artifact, mixed runs.

- clock: `clock/BENCHMARKS.md:58`, `:64`, `:13`, `:52`, `:12`, `:11`, `:24`, `:83`, `:84`, `:14`
- io: `io/benches/BENCHMARK.md:56` (+ repeats), `:122`, `:151`; `book/src/io/http-client.md:116`; `book/src/io/rate-limits.md:185`, `:183`
- cluster-md: `book/src/md-dpdk/overview.md:243`, `:222`, `:149`; `md-dpdk/README.md:23`, `:153`; `book/src/deploy/benchmarks.md:28,37`
- rest: none

### (2) Repair benchmark validity
Accounting, completeness, barriers, ceilings preserved.

- `sbe/benchmarks/benches/perf_parity_bench.rs:16` (unequal validation work inside the timer) — landed
- `sbe/BENCHMARKS.md:62` (nonexistent diagnostic arms; gate runs the checked constructor) — landed
- `sbe/BENCHMARKS.md:422`, `:497` (static text size sold as executed cost / retired work) — landed
- `sbe/benchmarks/tests/bench_gate_test.rs:372` (comment says 1.01; ceilings untouched) — landed
- `io/benches/src/stats.rs` (`Work::from_meta` accounting)
- Preserve: `PROFILE_SCOPED_OVERRIDE_MAX` ceilings, `NOISE_FLOOR_CEILING_EXCEPTIONS`, bench-gate ratios ≤ 1.00

### (3) Simplify components
Clock recommendation, TLS custom record layer, deployment machinery, NRI/DPDK optional.

- Clock recommendation: `book/src/clock/overview.md:59`, `:135`; `clock/README.md:44`; `clock/BENCHMARKS.md:94`
- TLS record layer: `io/benches/BENCHMARK.md:101`, `:122` (and `book/src/io/overview.md:177`, already confirmed)
- Deployment machinery: `book/src/deploy/java.md:68`, `:91`; `book/src/deploy/overview.md:50`
- NRI/DPDK optional: `book/src/deploy/latency-pods.md:12,145,150` (SCHED_FIFO is built, default-off)

### (4) Correct examples and rewrite the book
l3 price compare, timestamps, DTOs, OUT_DIR, links, categorical claims.

- l3 price compare: `samples/l3-book/tests/l3_tests.rs`
- Timestamps: `book/src/clock/wall-clock.md:52,76` (anchors, 533 ppm provenance); `book/src/clock/overview.md:257` (stall attribution)
- DTOs / generated-code examples: `book/src/project/package-scope.md:18`; `book/src/samples/buffer-sizing.md:3`
- OUT_DIR / codegen patterns: `book/src/samples/build-patterns.md:9`; `book/src/samples/overview.md:12`; `samples/README.md:12`
- Links: `book/src/io/http-client.md:116`; `book/src/clock/wall-clock.md:52,76`; `book/src/project/road-to-1.0.md:86`
- Categorical claims: `README.md:101`, `:102`; `CONTRIBUTING.md:60-64`; `book/src/tune/overview.md:3,129`; `sbe/BENCHMARKS.md:582`
- Allowlist: `scripts/book-fence-allowlist.txt`

## 3. Delete outright (do not correct)

| claim | file:line | reason |
|---|---|---|
| "gaps between consecutive reads are sub-ns on Graviton, 1–2 ns on c7i" | `clock/BENCHMARKS.md:84` | No artifact; fixed gap pass not run. Corrupting to a number would invent one. |
| "two 288 B flows 5,070 NAKs → none" | `md-dpdk/README.md:153` | No artifact; the two-flow points were discarded by the analysis. Nothing to correct to. |
| `rate-limits` cost table cells | `book/src/io/rate-limits.md:185-197` | "the run is not archived" — every cell is unpublished. Keep the reproduce recipe; delete or mark "not measured". |
| "executes 32-bit x86 plus big-endian s390x codec tests" | `README.md:101`, `CONTRIBUTING.md:64` | The lane does not exist. Relabeling as "not implemented" is acceptable only if the actual coverage sentence replaces it. |
| "stock rustls by default" recommendation | `io/benches/BENCHMARK.md:101`, `:122`; `book/src/io/overview.md:177` | The stack does not have a stock-rustls record path. The recommendation is not a mis-measurement; it describes software that is not there. |
| "rate vs kernel scaling" column | `clock/BENCHMARKS.md:52` | Duplicate of the drift column with different numbers; one measurement cannot be two columns. |
| "(the 40 µs figure is p99.99 under load)" | `io/benches/BENCHMARK.md:151` | A delta is not a level; the parenthetical misdefines the figure. |
| `ergo-sbe_wrap_checked`, `ergo-sbe_header_and_body_checked` as named arms | `sbe/BENCHMARKS.md:62`; `sbe/benchmarks/methodology.md:53` | No bench registers these labels. Listing them invents arms. |
| c9g discipline cells `310 / 1,610 (1,610–2,400)` and c8g `2,100` | `clock/BENCHMARKS.md:64` | No artifact row contains these as residual statistics. Replace with artifact values (preferred) or delete. |
| Broken anchors `#4-http-clients`, `#disciplined-clock-residual`, `#time-sync-in-these-runs` | `book/src/io/http-client.md:116`; `book/src/clock/wall-clock.md:52,76` | Targets no longer exist. Delete the link text or point at a live section; do not leave a dead "one copy of every figure, with its max" promise. |

## 4. Defects the inventory missed

| file:line | claim | verdict | evidence | fix |
|---|---|---|---|---|
| `samples/README.md:42` | OUT_DIR "fine for apps; **poor IDE go-to-def**"; `src/generated/` implied default | contradicted-by-code | Same text as `book/src/samples/build-patterns.md:9`, which the inventory covers; this copy is not in any row | Same fix as build-patterns.md:9 (product default is OUT_DIR + `sbe_mod!`). |
| `book/src/deploy/benchmarks.md:37` | Table header "idle … max \| 1 M msgs/s … max" with method "max is the slowest single round trip" | wrong-aggregation | Confirmed defect covered only the metal 71 µs cell. Every max cell in this table is a median of 5 rep maxima under the same absolute definition | Apply the overview.md:222 fix to all rows of this table, not just the metal cell. **Fixed 2026-10-20**: the page now defines max as the largest single round trip in any rep and names the five per-rep maxima. |
| `io/benches/BENCHMARK.md:56,62,116,149` | HTTP client latency "10–11 µs against libcurl's 28–30 (p50)" | quantitative | p50 only, no max, against the file's own rule at :3 ("every latency figure keeps its max"). Pre-cut git history had the maxima (e.g. libcurl `79.7 / 95.7 / 126.6 / 269`) | Restore the maxima beside the p50s, or label "p50 only; maxima in git history". **Fixed 2026-10-20**: the HTTP table is p50 / p99 / max on both platforms. |
| `book/src/md-dpdk/overview.md:237-238` | Localhost method prose | wrong-aggregation | Same "median of 5 reps / max is the slowest" contradiction as :159-161; noted as adjacent in verification, never given a row | Same caveat as the :243 fix. **Fixed 2026-10-20**: the five per-rep maxima are printed beside the max column. |
| `sbe/BENCHMARKS.md:160` | Gate table row lists `wrap(…, 8, …)` for the entry-point pair | wrong-fact | Contradicts the same validation-class rule that the perf_parity finding enforces; the bare `wrap` runs the extent proof | Update the row once `perf_parity_bench.rs:16` is fixed. **Fixed 2026-10-20**: the row now reads `wrap_unchecked(…, 8, …)`, extent proved untimed. |
| `CONTRIBUTING.md:60-64` | "Nightly CI runs every fuzz target …" | wrong-fact | `nightly.yml` is weekly (`cron: "23 17 * * 0"`); the workflow name lies | Change "Nightly CI" to "Weekly CI (Sundays)" wherever it appears. **Fixed 2026-10-20.** |
| `book/src/io/kubernetes.md:92-93`; `book/src/io/overview.md:133,260` | "a little at p50 … more at p99"; "lowest p99 of the polling backends from about 10 sockets" | quantitative | Directional p50/p99 claims with no max, no run id, no numbers | Give the figures with their max and the run, or delete the comparative claim. **Fixed 2026-10-20**: maxes and run ids restored in `kubernetes.md` and `overview.md`. |
| `io/benches/BENCHMARK.md:21,52,136,139,243,247` | "Round trips … 1 connection only"; "Graviton: not measured"; "HTTP many connections — estimates only"; "Graviton comparison — not measured" | contradicted-by-artifact | `results/flags-aws-20261019T175248Z/fair.md` holds WebSocket round trips at N = 1/4/16/64 and an HTTP pool of 1 and 20, on Graviton, in the same file | Scoped to x86 and pointed at the fair run; the status table gained a 2026-10-19 row. Fixed 2026-10-20. |
| `io/benches/BENCHMARK.md` fair-run work sentence | "every point issued and completed 600,000 of 600,001 scheduled" | wrong-aggregation | `tables.md`: scheduled 600,001–600,265, issued/completed 599,994–600,005, late 0–47, censored 0–1 | Quote the ranges and name `tables.md` as the source. Fixed 2026-10-20. |
| `scripts/check-bench-gate.sh` A/A verdict | "a pair inside it has no established winner" was printed but the pair was still graded FAIL | contradicted-by-code | The A/A band only produced a note; `resolution` came from the ratio CI against the ceiling alone. `cluster_encode_claim_shaped_header_plus_app` printed "resolved breach" and "inside A/A null band" in the same breath | The band now decides: a ratio above its ceiling whose ratio CI overlaps a measured band is inconclusive, not a breach. A pair with no measured control still fails closed. Ceilings unchanged. Fixed 2026-10-20. |
| A/A control placement (cluster encode) | the null band is not placed like the pair it grades | unsupported-as-a-null | The same ergon keep-alive loop measures 7,061–7,068 ns in `cluster/control/aa_encode_keep_alive` and 7,250 ns in `cluster/encode/session_keep_alive` — 2.7 % between two copies of identical code. Re-run `cluster-20261020T072915Z-e565d087` measures it again under LTO: 7.3092 µs in the control against 7.3760 µs in the pair's ergo arm, 0.9 % on identical code and larger than that pair's own 0.63 % spread. The control's two arms sit next to each other; the pair's two arms sit in different groups | Partly closed 2026-10-20. The gate now grades each pair against the envelope of a **repeated** control across independent invocations, which absorbs a between-invocation offset of the kind that produced the rotating breaches. A *systematic* within-run group offset would still bias every invocation the same way, so a pair that is stably separated across every repeat is the case where a per-pair control is still needed. Open for that case only. Fixing it means placing the control arms like the pair (or per-arm processes), not widening a ceiling. |
| `scripts/check-bench-gate.sh` A/A band, second property | one `arm_a`-then-`arm_b` pairing per control, so the band's centre carries arm-order bias | wrong-aggregation | The no-LTO null band moves from [0.9970, 1.0129] (centre 1.005) to [0.9752, 0.9946] (centre 0.985) between two runs of identical code on one host. With the band sitting entirely below 1.0, a pair at 1.0001 is reported "outside A/A null band" | Closed 2026-10-20 for the between-invocation part: the band is now the envelope of the control over `BENCH_REPEATS` independent invocations (default 5), so its centre is no longer one run's arm order. Within-run arm-order bias is unchanged and still one pairing; running each control with the arms in both orders would centre the band on the true null without widening a ceiling. |
| `md-dpdk/BENCHMARKS.md:133`; `clock/BENCHMARKS.md:170` | "Full tables are in git history"; "Full every-run, gap, cross-core and gate tables are in git history" | no-artifact | `git log --all --diff-filter=A` is empty for result and table paths under `clock/**` and `md-dpdk/**` (the only md-dpdk csv in history is `benches/cloud/test/fixture/results.csv`), and `git log --all --diff-filter=D -- clock/**` is empty. Neither body's tables were ever in the repository | Write what is true: the raw run output was written under `target/` and is not retained; the tables in these files are the record. **Fixed 2026-10-21** (`bb12f2f6`). The `io/benches` claims are accurate and were left alone — those tables were deleted in the documentation cut `128747b9` and recover from `128747b9^`, and `book/src/io/kubernetes.md:97` names `2517309d`, which exists. |
| `clock/BENCHMARKS.md:192` | "Assembly listings are in git history (`clock/src/lib.rs`, `clock/src/hot.rs`)" | wrong-fact | `clock/src/hot.rs` does not exist and never did. The bare and ordered counter reads (`rdtscp`, `mrs cntvctss_el0`, `isb; mrs cntvct_el0`) are in `clock/src/arch.rs`, which is in the tree, not only in history | Name `clock/src/arch.rs`; stop describing tracked source as "in git history". **Fixed 2026-10-21** (`bb12f2f6`). |
| `sbe/benchmarks/benches/perf_parity_bench.rs` `parity/decode/scalar` | the first arm `warmup` is a throwaway that absorbs "a position penalty"; the gate matches only `ergo-sbe` / `sbe-tool` | contradicted-by-artifact | `warmup` and `ergo-sbe` are byte-identical — same ergo `CarDecoder`, same `MICRO_BATCH_SIZE` loop. Run `b20261021T015521Z-47835-bb12f2f6e721`, no-LTO invocation 1, measured them **657 ns and 921 ns in one process** (ergo CI 826–1030 ns, ±11 %) while the `sbe-tool` twin was tight (819.83–831.54, ±0.7 %). Invocations 2–5 agree at 649/651 ns. The cause is in the estimator, not the code: invocation 1's `ergo-sbe` **median** is 666.9 ns (CI 659–672) against 640–644 ns in invocations 2–5 — the same speed within 4 % — while its `std_dev` is 746 ns against 28 ns and its `median_abs_dev` 27.5 ns against 6.5 ns, so on a two-point fit about an eighth of the 200 samples sat near 3 µs and dragged the **mean**, which is the statistic every ratio is built on. The byte-identical `warmup` arm in the same process is clean (657.3 / 651.4 / 29.0): the disturbance was temporal — host noise on this M1 Pro landing inside one arm's measure window — not a property of the arm's code or data. The suite's A/A control is tight throughout (`parity/control/aa_encode_throughput` arm_a/arm_b = 1.0001, 1.0087, 0.9973, 1.0036, 0.9885, ratio CI [0.9819, 1.0137]), but it is an 8.4 µs benchmark in its own windows and does not exonerate a disturbance that hit one short arm | **Open.** The group already holds a ready-made A/A pair and the gate ignores it, so an outlier-ridden `ergo-sbe` estimate silently widens the ergo/sbe-tool ratio envelope. Its consequence in that run: `decode_scalar` 0.7962–1.1163 and `encode_scalar_header_and_body` 0.7455–1.1015 both leave the 1.00 ceiling, and because their ratio CIs are enormous ([0.7864, 1.2569] and [0.7396, 1.2685]) they **overlap** the measured A/A null band [0.9819, 1.0137], so the gate reports both `inconclusive — no winner established` and passes overall. That is the designed verdict for an inconclusive pair — no ergon win, no ergon loss, ceilings unchanged — but here the ambiguity is manufactured by one invocation: four of the five put `decode_scalar`'s **mean** ratio at 0.7962–0.8010 and the first at 1.1163, while all five put its **median** ratio at 0.798–0.820 including the first (0.8204). `encode_scalar_header_and_body` is the same: mean ratio 1.1015 in invocation 1 against 0.7455–0.7556 in the rest, median ratio 0.7748 against 0.749–0.761, and that arm's invocation-1 `std_dev` 1926 ns against 48–58 ns. Neither code path was slower in invocation 1; only its means were dragged. Gate the `warmup`/`ergo-sbe` ratio against the null band like any other control. That is stricter, not looser: it adds a control and touches no ceiling. Do not discard invocation 1 and do not widen 1.00 — either would be making a lane green by filtering. |

| `sbe/benchmarks/benches/perf_parity_bench.rs` LTO `parity/decode/entry_point` `ergo-sbe_wrap` and `parity/encode/scalar` `sbe-tool_header_only` | that a maintained pair's mean ratio compares the two arms' central speed | estimator-bias | Under LTO one arm of each of these two pairs carries a right skew reproduced in **all five** invocations of run `b20261021T015521Z-47835-bb12f2f6e721` and absent under no-LTO. `ergo-sbe_wrap`: mean/median 1.087–1.095, `std_dev`/median 0.164–0.169; its twin `sbe-tool_wrap` clean at 1.001–1.005. `sbe-tool_header_only`: mean/median 1.095–1.100, `std_dev`/median 0.184–0.189; its twin `ergo-sbe_header_only` clean at 0.998–1.005. The affected arm alternates between the two pairs, so this is not one library being noisy. Under no-LTO all four arms are 1.000–1.013. Consequence for the two maintained ratios: `decode_entry_point` mean 0.6367–0.6448 against median 0.5844–0.5937 (**+8.6 to +9.4 %**); `encode_scalar_header_only` mean 0.4090–0.4115 against median 0.4463–0.4507 (**−8.2 to −9.0 %**), the skew landing on the denominator. Both pairs graded `resolved` outside the A/A null band and sit far inside 1.00 on either statistic, so **no verdict turns on it** | **Open.** The mean is not measuring the body of these two distributions, so the two statistics are not interchangeable and a future pair within a few per cent of its ceiling would be judged on the wrong one. The mechanism is untraced; it is reproducible, so it is a property of the LTO build's measurement of that arm rather than host noise. Report the median beside the mean for every maintained pair, or drop the skewed subpopulation by a stated, uniform rule applied to every arm. Do not pick a rule after seeing which way a pair leans, and do not touch a ceiling. |

## 5. Second review, 2026-10-20 — nine findings

A code review of the stage-2 work. Every finding was judged valid and fixed
before the release commit, because a release gate that can false-PASS would
publish exactly what this audit set out to repair.

| # | Finding | Verdict | Fix |
|---|---|---|---|
| 1 | "Inconclusive results now pass the strict gate." A 2× slowdown with a wide CI and no A/A control exited 0 | confirmed | An over-ceiling pair is non-blocking only when a **measured** band reaches it. Otherwise "unproven — blocking". Six tests pin it. |
| 2 | "A malformed later repetition silently disappears." One good repetition graded alone; `NaN` passed | confirmed | The reader parses everything before using any of it, requires finite **positive** values, and refuses the key on any failure. |
| 3 | "Repeat completeness counts directories rather than complete comparisons." A pair missing from repetition two still passed | confirmed | Every declared repetition must carry every consumed pair, matched by repetition ID. |
| 4 | "Unverified A/A controls can change the verdict." A `foreign` control turned a 2× regression into PASS | confirmed | Both control arms are provenance-checked against the run manifest. A missing control is still "not measured", not a failure. |
| 5 | "New comments still assert unproved benchmark causes" (descheduling, arm order, scheduler, branch prediction) | confirmed | The causal claims are removed from the bench sources; observed figures stay as observations. |
| 6 | "`--fair` hides the newly collected sender-schedule warning." 80 % late batches published clean | confirmed | `fair.rs` emits the send schedule beside the work counts and names the slipped rows. |
| 7 | "The evidence exporter silently accepts incomplete or invalid accounts." `late_100us=-80` → `-80.00% … ok`, exit 0 | confirmed | `extract_work_counts.py` now matches `SendSchedule::from_meta` and `Work::from_meta`/`check`, and fails closed. The analogous hole on the `work.*` copy path was real too. |
| 8 | "The new residency test assumes page-aligned allocation." Expected 2 pages, reported 1 | confirmed | Expectations come from `pages_wholly_inside`; the offset-slice case is pinned. |
| 9 | "Aeron's underlying clock cost is still described as outside the hot path" | confirmed | Now "One underlying read per duty cycle; no workload benefit demonstrated from replacing it." |

Host provenance was added alongside: `aarch64-apple-darwin` does not
distinguish an M1 from an M4, and 0.1.30's local figures are M1 where
0.1.29's were M4. Both manifest writers record `host`; the gate requires and
prints it. Ceilings are host-independent ratios and were not changed.

## 6. Chapter dispositions — every book file

One line per file, 2026-10-21. A **high-risk claim** is a latency figure, a
comparative, a "required/always/never/zero-cost", or a setting sold as making
a difference. **clean** = no high-risk claim, or every one traced. A row that
says **fixed** names what was wrong and points at section 1.

### Front matter

| file | disposition |
|---|---|
| `SUMMARY.md` | clean — structure only |
| `introduction.md` | clean — `generate_to_out_dir` + `sbe_mod!` is the product pattern; no latency claim |

### ergo-sbe

| file | disposition |
|---|---|
| `sbe/getting-started.md` | clean — index |
| `sbe/getting-started/depend.md` | clean — dependency setup |
| `sbe/getting-started/generate.md` | clean — build-script recipe, no performance claim |
| `sbe/getting-started/include.md` | clean — `include!` / `sbe_mod!` mechanics |
| `sbe/getting-started/encode-decode.md` | clean — encode/decode walkthrough |
| `sbe/getting-started/method-chaining.md` | clean — type-state wiring |
| `sbe/getting-started/multi-schema.md` | clean — multi-schema layout |
| `sbe/getting-started/from-sbe-tool.md` | clean — migration map; differences are stated, not ranked |
| `sbe/getting-started/error-diagnostics.md` | clean — error reporting |
| `sbe/feature-tour.md` | clean — index |
| `sbe/feature-tour/exact-sizing.md` | clean — length APIs |
| `sbe/feature-tour/bulk-arrays.md` | clean — array accessors |
| `sbe/feature-tour/decode-stages.md` | clean — decode lanes |
| `sbe/feature-tour/generated-code.md` | clean — shape of the output |
| `sbe/feature-tour/trust-boundaries.md` | clean — where validation happens |
| `sbe/feature-tour/domain-objects.md` | clean — DTOs; the DTO-vs-flyweight latency A/B is written **not measured** |
| `sbe/feature-tour/multi-template.md` | clean — template dispatch |
| `sbe/core-concepts.md` | clean — index |
| `sbe/core-concepts/trust-boundary.md` | clean — the fail-closed rule restated, no figure |
| `sbe/core-concepts/wire-order-stages.md` | clean — type-state wire order |
| `sbe/core-concepts/buffer-sizing.md` | clean — generated length APIs |
| `sbe/core-concepts/flyweight-vs-struct.md` | clean — design trade-off, no figure |
| `sbe/core-concepts/composite-layout.md` | clean — layout and endianness |
| `sbe/configuration.md` | clean — index |
| `sbe/configuration/feature-integrations.md` | clean — feature wiring |
| `sbe/configuration/conversion-vs-domain.md` | clean — `with_conversion` vs `with_domain_type` |
| `sbe/configuration/generation-config.md` | clean — option reference |
| `sbe/configuration/hooks.md` | clean — codegen hooks |
| `sbe/recipes.md` | clean — index |
| `sbe/recipes/aeron-try-claim.md` | **fixed** — claim zeroing's latency cost is written **not measured**; no figure invented |
| `sbe/recipes/display-debug.md` | clean |
| `sbe/recipes/schema-rustdoc.md` | clean |
| `sbe/recipes/domain-dtos.md` | **fixed** — DTO-vs-flyweight latency A/B written **not measured** |
| `sbe/recipes/app-types-composites.md` | clean — type mapping |
| `sbe/recipes/timestamps.md` | clean — conversions; UTC discipline lives in the clock chapter |
| `sbe/design-notes.md` | clean — index |
| `sbe/design-notes/type-state.md` | **fixed** — the title "Type-state is zero-cost" was a categorical claim standing alone in the TOC. Retitled "Type-state and runtime cost"; the body keeps the scoped claim (no runtime tag) and the proof obligation ("whether a revision passes requires a fresh run") |
| `sbe/design-notes/api-freeze.md` | clean — API decisions |
| `sbe/design-notes/nullval.md` | clean — null-value representation |
| `sbe/design-notes/feature-matrix.md` | clean — support matrix |
| `sbe/benchmarks.md` | **fixed** — diagnostic arms corrected, static text size no longer sold as executed cost, exoneration dropped (section 1, `sbe`) |
| `sbe/benchmarks/methodology.md` | **fixed** — arm list matches what the gate reads; validation-class rule restated |

### ergo-aeron-cluster

| file | disposition |
|---|---|
| `cluster/overview.md` | clean — client scope; not a consensus module |
| `cluster/session-builder.md` | clean — builder surface |
| `cluster/egress-listeners.md` | clean — egress wiring |
| `cluster/chained-decoding.md` | clean — decode chain |
| `cluster/compatibility.md` | clean — version policy |

### ergon-io

| file | disposition |
|---|---|
| `io/overview.md` | **fixed** — TLS stack is the in-place `tls::RecordLayer` (not "stock rustls by default"); comparative claims now carry a max and a run id (section 1 + section 5) |
| `io/backends.md` | clean — backend comparison is qualitative where unmeasured, and says so |
| `io/websocket.md` | **fixed** — "more sockets on the loop: not measured"; figures carry max and run |
| `io/http-client.md` | **fixed** — signing figures labelled "best of 7, max not recorded"; the dead `#4-http-clients` anchor retargeted to `BENCHMARK.md#conclusions`; many-connection written **not measured** |
| `io/rate-limits.md` | **fixed** — the unarchived `try_charge` table deleted; section says **not measured** and keeps the reproduce recipe; per-call vs per-batch aggregation no longer mixed |
| `io/kubernetes.md` | **fixed** — pod-placement claims carry max and run id |

### ergon-clock

| file | disposition |
|---|---|
| `clock/overview.md` | **fixed** — the 100,000-reads/sec cutoff removed; minstant described as `_rdtsc` on x86 and `SystemTime` on Arm/macOS; the vCPU stall written "unexplained in the guest; the platform's vCPU is suspected" |
| `clock/wall-clock.md` | **fixed** — dead anchors retargeted; 533.462 ppm kept with its run named |

### ergon-tune

| file | disposition |
|---|---|
| `tune/overview.md` | **fixed** — "what it cost in this repository's measurements **where measured**"; "One A/B per **measured** check"; the unmeasured checks (T4, T5, T6, T9, N2, N3) named |

### ergon-md-dpdk

| file | disposition |
|---|---|
| `md-dpdk/overview.md` | **fixed** — every "max" column is the absolute worst, or a median of per-rep maxima labelled as such with the worst named (localhost 664.6 µs, Azure 447 µs, c7i 6.7 ms / Azure 33 ms); proof-of-concept status stated once |
| `md-dpdk/configuration.md` | clean — configuration reference |

### Deployment

| file | disposition |
|---|---|
| `deploy/overview.md` | **fixed** — ParallelGC labelled "convention, not a measured collector A/B"; checklist before glossary; trimmed 2026-10-21 |
| `deploy/hft.md` | **fixed** — notation preamble cut to one line 2026-10-21 so the settings table is first on screen; every row's figure traced to a named run; the two `app` network rows cite fair run `20261019T175248Z` |
| `deploy/hosts.md` | clean — host settings; the C-state wake delay is the vendor's, written **not measured here** |
| `deploy/kubernetes.md` | clean — node bootstrap; the two node scripts are a table as of 2026-10-21 |
| `deploy/shared-cpus.md` | **fixed** — SCHED_FIFO is built and default-off, not "deliberately not built"; status essay and release log cut 2026-10-21; ends on a decision, with the unmeasured/unverified list in one place |
| `deploy/latency-pods.md` | **fixed** — the SCHED_FIFO row moved out of "Deliberately not built"; the grant wording matches the plugin (section 1, `deploy`) |
| `deploy/benchmarks.md` | **fixed** — absolute maxima with the per-rep spread; "max" is never defined as the slowest single round trip while printing a median. Method and aggregation moved to one footnote each (2026-10-21); page is tables |
| `deploy/java.md` | **fixed** — `GuaranteedSafepointInterval` removed (it removed nothing); quorum-vs-fsync scoped to cluster size; ParallelGC labelled convention |

### Samples

| file | disposition |
|---|---|
| `samples/overview.md` | **fixed** — `src/generated/` is the **samples** default; OUT_DIR + `sbe_mod!` is the **product** default; L3 build-dep is `#[path]`, not `include!` |
| `samples/sbe-feature-tour.md` | clean — sample walkthrough |
| `samples/l3-book.md` | **fixed** — the price assertion compares `Decimal` values (mantissa **and** scale), not mantissas |
| `samples/exchange-example.md` | clean |
| `samples/codegen-library.md` | clean |
| `samples/cluster-tutorial.md` | clean |
| `samples/cluster-ha-orderbook.md` | clean |
| `samples/cluster-rfq.md` | clean |
| `samples/build-patterns.md` | **fixed** — the three build patterns labelled samples-default / product-default correctly |
| `samples/buffer-sizing.md` | **fixed** — "every buffer must use EncodedLength" softened to the generated length APIs, with `MAX_ENCODED_LENGTH` sanctioned where it is a true fixed bound |

### Project

| file | disposition |
|---|---|
| `project/contributing.md` | **fixed** — mutation testing is manual; Miri is weekly (Sundays); 32-bit x86 and s390x codec tests are stated as **not run in CI** |
| `project/ai-assistance.md` | clean — process disclosure |
| `project/verification.md` | clean — release gates; names `sbe-bench-gate-no-lto.txt` as the CI artifact |
| `project/road-to-1.0.md` | **fixed** — semver-checks cite the `baseline_tag` in `api/public-api-baseline.toml` (0.1.26), not a hardcoded 0.1.21 |
| `project/external-pilot.md` | clean |
| `project/package-scope.md` | **fixed** — cluster codecs are `#[doc(hidden)] pub`, not crate-private; `AeronCluster` is the consumer API |

84 of 84 named. Count check: 2 front matter + 42 sbe + 5 cluster + 6 io + 2
clock + 1 tune + 2 md-dpdk + 8 deploy + 10 samples + 6 project = 84.

## 7. Flag and setting latency evidence — 2026-10-21

Release goal B asks whether every recommended flag or setting has a confirmed
latency effect with evidence. It does not hold as written. Every recommended
setting was traced to the run artefact that supports it and given one of four
verdicts. Settings are counted once per claim, so a setting that is required
for correctness *and* credited with a latency win appears twice.

| verdict | count | meaning |
|---|---|---|
| **EVIDENCED** | 18 | a run A/B exists and the docs quote it |
| **UNMEASURED** | 27 | recommended, often with a mechanism, no latency A/B |
| **REQUIRED-NOT-PERF** | 28 | correctness, access or observability; makes no latency claim |
| **CONTRADICTED** | 3 | the cited data disagrees — **all three fixed** |

What can be claimed at release: every setting presented as a latency win
either has a run behind it or is written "not measured" at the site. No
setting is presented as a measured win without one. The 28 correctness
settings are labelled as such in `book/src/deploy/hft.md` ("required for
whole cores or fail closed"). **Not** claimable: "all flags have confirmed
to make a difference in latencies" — 27 of them have not been measured, and
28 do not claim to.

### EVIDENCED — the settings that carry the latency claim

| setting | measured effect | source |
|---|---|---|
| `isolcpus` / `nohz_full` / `rcu_nocbs` / `irqaffinity=0` (bundle) | Azure DPDK idle median of rep maxima **216 (69–663) → 71 (65–447)** µs | `md-dpdk/BENCHMARKS.md` |
| `transparent_hugepage=never` | idle p99.99 **67.01 → 53.15**; idle median of rep maxima **545.79 → 106.50**, absolute max **632 → 286** | `md-dpdk/BENCHMARKS.md` rounds 4, 6 |
| SMT off (`ThreadsPerCore: 1`, `nosmt`) | a busy sibling costs **+0.5 µs p50** at 1 conn, **+2.3–2.7 µs p50** and **+9–12 µs p99** at 10–100 | `io/benches/BENCHMARK.md` run `122127Z` |
| `ethtool -C <if> adaptive-rx off` (rx-usecs stays 20) | `recv-busy` p99.99 falls **43–47 µs**, median about **3 µs** | `io/benches/BENCHMARK.md` runs `062428Z`, `070339Z` |
| NIC queue IRQs on a spare core; `irqbalance` stopped | placement costs the tail: p99/max **45.9–47.2 / 135–161** vs **47.0–53.9 / 108–425**; IRQs on housekeeping lost 1 M/s in 5 of 5 | `io/benches/BENCHMARK.md`; fair run `20261019T175248Z`; `md-dpdk/BENCHMARKS.md` G1 |
| ENA RX ring `ethtool -G <if> rx 4096` | no NIC drop or NAK where the default ring dropped at 1 M msgs/s of 288 B (loss, not tail) | `md-dpdk/BENCHMARKS.md` round 5 |
| keep-alive about 1 kHz on sparse streams | one lost ping cost **134 ms** (Aeron's next heartbeat) | `md-dpdk/BENCHMARKS.md` |
| quiet cores only (two-minute spin at pod start) | some Azure vCPUs stop for up to **3.7 ms** once a minute | `md-dpdk/BENCHMARKS.md` experiment log 2026-10-14 |
| `net.core.busy_read/busy_poll=50` (Azure) | busy polling p50 **21.15 / max 99.52** vs plain kernel UDP **32.03 / 354.81** | `md-dpdk/BENCHMARKS.md` round 11 |
| plain kernel UDP, no isolation (counter-arm) | idle median of rep maxima **354.81 (272.13–2,203.65)** vs **57.89** isolated DPDK | `md-dpdk/BENCHMARKS.md` round 11 |
| `SHARED_NETWORK` (counter-arm) | 1,344 B bursts p50 **522 vs 234**, median of rep maxima **748 vs 493** | `md-dpdk/BENCHMARKS.md`, correction 2026-10-17 |
| busy polling on Graviton at 288 B, 1 M/s (counter-arm) | collapse: p50 **4,973 µs**, absolute max 102,826, 61,488 NAKs | `md-dpdk/BENCHMARKS.md` G1 |
| `mlockall`, pacing, RX burst 64, 1 MiB window, C1 on metal (counter-arms) | each lost its A/B; C1 on metal made kernel UDP's p50 **2 µs** worse | `md-dpdk/BENCHMARKS.md` |
| `rcv-wnd=80k` alone vs window + 20 µs park | flat-out falls **6.5–6.8×**; loss-free ceiling **drops** 157 k → 121 k msgs/s without the park | `md-dpdk/BENCHMARKS.md` rounds 12–13 |
| `SCHED_FIFO` built, default off | no gain on an isolated CPU (p50 **+0.5/−1.8 µs**, p99.99 **+1/−45 µs**; one A/B, sender inflated, directional only) | `io/benches/BENCHMARK.md` run `163114Z` |
| `uring-sqpoll` on a sibling, never a second isolated core | sibling costs **+0.5 µs p50 / +9–12 µs p99**, still beats single-vCPU by about 8 µs | `io/benches/BENCHMARK.md` run `122127Z` |
| x86 VM C-state cap `intel_idle.max_cstate=1` | mixed: helped one round, cost the 288 B loss-free rate in another; worse on metal | `book/src/deploy/hosts.md`; `md-dpdk/BENCHMARKS.md` |
| ENA interrupt path `adaptive-rx off rx-usecs 0 tx-usecs 0`, IRQs on a core of their own | within **1.7–3.3 µs** of NIC polling at 1 conn, level from 4 (Graviton only) | fair run `20261019T175248Z` |

### UNMEASURED — recommended without a latency A/B

Each of these is recommended for a stated mechanism. None has a run behind
it. Sites that admit it in place are noted; the rest carry no number.

| area | settings | source of the gap |
|---|---|---|
| Memory / hugepages | boot hugepages + `vm.nr_hugepages`; Aeron dir on sized hugetlbfs with `AERON_FILE_PAGE_SIZE`, `AERON_TERM_BUFFER_SPARSE_FILE=false`, pre-touch; latency-pods extension 1 | T4: tmpfs and hugetlbfs never A/B'd (`tune/src/mem.rs`) |
| Kernel memory policy | KSM off; `numa_balancing=0`; `memory.high` pressure off the pod | T5, K6 |
| CPU placement | `topologyManagerPolicy: single-numa-node` **latency effect** (the placement itself is required); co-tenancy kept off the node | T6: every lab host had one NUMA node |
| CPU quota | the 100 ms CFS-quota stall behind "no CPU quota on the busy container" | T9, `tune/src/quota.rs` |
| CPU power / idle | x86 C-state exit latency; Graviton `nohlt`; `tuned` profiles | admitted at each site |
| NIC | `napi_defer_hard_irqs=2`, `gro_flush_timeout=200000`; N3's Azure accelerated-networking VF | N2's timer wait and N3 |
| Shared CPUs | timer slack 1 ns; helpers (dial, TLS, warnings, Aeron conductors) off the loop's core; `hostNetwork: true` | admitted at each site |
| Java / Aeron JVM | `-XX:+UseParallelGC`; `-Xms=-Xmx` + `AlwaysPreTouch`; metaspace and code-cache sizes; `-Xlog:async`; `-Dagrona.disable.bounds.checks=true`; `aeron.*.idle.strategy=spin/noop`; hot-thread pinning; `-XX:+UseSerialGC`; the four "measure first" options | the Source column is Aeron scripts or external `aeron-io/benchmarks`; no Java latency A/B exists in this repo |
| Build flags | `-march=native`, `target-cpu=neoverse-*`, `-mcpu=…` | codegen claim only |
| Latency-pods | extension 2 (`cpuset.cpus.partition=isolated`) and the set's latency effect | two EKS spot labs reached no capacity |

The repo's own summary agrees: `book/src/tune/overview.md` — "Not measured:
T4, T5, T6, T9, N2's timer wait and N3. T8's effect was weak."

### CONTRADICTED — three claims the cited data disagreed with

| # | claim | what the data says | fix |
|---|---|---|---|
| 1 | `latency-pods.md`: "Deep C-states cost every update from idle; a C1 cap cut the idle **max** hard", and `node-checks.md` attributing the **3.5 µs** clock-update cost (max **6.1–18.7**) to "waking from deep idle" | the only C1 A/Bs are mixed on a VM and **worse** on metal (`md-dpdk/BENCHMARKS.md`, `hosts.md`); `clock/BENCHMARKS.md` says the clock-update cost is untraced and "the harness never sleeps, so this is not an idle-exit effect" | the device grant is now stated as access, not a latency win, and points at the C-state table; the false attribution is gone from the book, `node-checks.md` and `eks-nri-userdata.sh`'s comment |
| 2 | `shared-cpus.md`: `schedFifo: "off"` — "keep off: **measured worse**" | the one A/B measured **no gain** and is flagged directional only (a sleeping sender inflated it); "worse" is the opposite claim | wording corrected; the recommendation (keep off) was already right |
| 3 | `tune/src/nic/mod.rs` N1: "plain kernel UDP at 100 k msgs/s went from p50 **106 → 71 µs** (max **606 → 560**; 3 of 5 reps)" | that pair is **withdrawn** in `md-dpdk/BENCHMARKS.md`: 106/606 came from an idle, single-rep run with moderation **on**; no c7i moderation A/B at 100 k/s exists in any run | clause deleted from the live finding. A second unsourced number in the same sentence ("round trips over 100 µs went from 73–117 to 15–32 a minute", which appears nowhere else in the tree) went with it. The Graviton G1–G3 support is kept, per architecture. `tune/tests/nic_checks.rs` now asserts the withdrawn figures **stay** out |

A fourth recommendation went with claim 1: `cpu.idle` "off for latency
pods", in `node-checks.md`, its sample `node.json` and the book's node-setup
table. Its only justification was claim 1's idle-exit attribution, and
nothing in the tree reads or writes the setting. If a measured effect turns
up, it can return with the run that shows it.

## 8. Artefact provenance and one unjudged setting — 2026-10-21

Every figure quoted in section 7 was traced to the run that produced it, and
then to the artefact that still holds its output. Two things turned up.

### Where each body of numbers actually lives

| body | raw output in the tree? | how to recover a figure |
|---|---|---|
| `io/benches`, runs through `142240Z` | env, cpumap and an empty `analyse.err` only | `git show 128747b9^:io/benches/results/<run>/tables.md` — commit `128747b9` ("tables out of the tree") deleted the per-run tables; `io/benches/BENCHMARK.md:17` says so and is accurate |
| `io/benches`, fair run `20261019T175248Z` | `tables.md`, `fair.md`, `work-counts.md`, frozen | in place. `records.bin` was left on the receiver by `KEEP_RECORDS=0` and is gone with the VM, so the tables cannot be re-derived |
| `md-dpdk` rounds G1–G3, 4–13, experiment log | **nothing** | prose tables in `md-dpdk/BENCHMARKS.md` are the record. No md-dpdk run output was ever committed; the only hit in git history is a test fixture |
| local codec benches | Criterion trees + run manifests under `target/bench-runs/` | in place; re-stamped per commit by `just bench` / `just bench-cluster` |

Consequence: section 7's io figures are verifiable to a run, through git
history. Its md-dpdk figures are not verifiable to any artefact — the
tables are the record. `md-dpdk/BENCHMARKS.md:417` named
`target/md-dpdk-bench/20261019-071438/` (`results.csv`, `summary.md`) as
round 13's artefact; `target/` is gitignored (`.gitignore:2`), the directory
is gone and was never committed. The dead path is removed and the line now
states what is retained.

### `mitigations=off` — measured, never judged

Four runs captured a boot ladder on the receiver
(`env-rcv-{stock,isolcpus,full,full-nomitig}.txt`):
`flags-aws-20261014T163114Z`, `flags-aws-20261015T045527Z`,
`flags-azure-20261014T144146Z` and `flags-aws-20261015T051741Z` (that one
`full`/`full-sqtick`). The difference between `full` and `full-nomitig` is
`mitigations=off` on the kernel command line — spectre_v2, IBPB, STIBP,
SSB and the spectre_v1 swapgs barriers, as the env capture records.

Only `163114Z` analysed the ladder. Its tables are in git history at
`128747b9^`. `045527Z` captured the same env pair and analysed something
else — **no `nomitig` row in its tables** — so this is one run, not two.

HTTP round trip `http-recv-busy-10000` (n ≈ 1.19 M, µs), `boot-full-nomitig`
against `boot-full`:

| | p50 | p99 | p99.9 | p99.99 | max |
|---|---|---|---|---|---|
| `full-nomitig` | 46.10 | 58.65 | 70.45 | **88.99** | **1,045.66** |
| `full` | 46.31 | 58.80 | 72.65 | 119.69 | 269.07 |

Market-data burst `md-recv-busy` (n ≈ 2.9 M, µs), the same pair:

| | p50 | p99 | p99.9 | p99.99 | max |
|---|---|---|---|---|---|
| `full-nomitig` | **2.43** | **6.69** | **11.88** | 30.58 | 46.37 |
| `full` | 3.28 | 7.16 | 14.09 | **33.42** | **39.73** |

Two other arms in the same tables favour `full-nomitig` on the tail (one
WebSocket arm's max **245.79** against **1,035.94**; another **166.61**
against **534.91**). So `mitigations=off` wins the tail on some backends and
loses it on others, with one arm's absolute max four times worse with it on.
**Mixed, one run, no repetition: this establishes no winner.** Per the rule
that an inconclusive result claims nothing, `mitigations=off` is recorded as
measured and undecided, and is recommended nowhere. If it is ever
recommended, it needs a second run on the same pair first.
