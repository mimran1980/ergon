# Completion goal prompt (paste into /goal or any LLM session)

HFT-first residual completion for the ErgoSBE experimental umbrella.
**Docs-truth updated 2026-07-18 evening** — do not re-run finished codec
migration work. No pillar directory renames (`sbe`, `persist`, `cluster`,
`samples` stay forever).

```text
Drive residual ErgoSBE umbrella work (cluster reliability + perf honesty +
HA sample) to completion with verification-first, small test-driven slices.
Keep running until every acceptance item below has fresh command output as
evidence. Do not stop because work is large, a doc says DONE, or
Java/Docker/Gradle is missing — install or start what is needed.

================================================================
READ FIRST
================================================================
- CLAUDE.md (gitignored local guidance — never commit it)
- docs/superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md
- sbe/design/DECISIONS.md (canonical SBE design authority)
- ergosbe-performance-optimisation-goal.md (bench evidence ledger)
- phase2-completion-goal.md (sbe/persist/samples IPC path ledger)
- cluster/README.md
- docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md
- samples/todo/02-cluster-ha-orderbook-latency.md
- docs/superpowers/plans/2026-07-18-rusteron-cluster-current-state-and-gap-plan.md
  (historical; residual reliability risks still valid)

================================================================
HARD LAYOUT (never change)
================================================================
Pillar directories are permanent: sbe/  persist/  cluster/  samples/
Also leave cluster-test-support/ and crate package names as-is
(ergosbe, ergo-clickhouse-persist, ergo-aeron-cluster, …).
Dir names may differ from crate names — that is intentional. Never rename
directories or packages for cosmetic alignment.

================================================================
PRIORITY LADDER (never invert)
================================================================
1. Official-SBE wire compatibility is non-negotiable.
2. Every MAINTAINED measured scenario median ratio ≤ 1.00 (ErgoSBE vs Aeron
   SBE for codegen; ErgoSBE vs sbe-tool for cluster codecs while dual-codec
   exists). Both sides must do byte-identical work — dump and compare bytes
   before trusting a ratio.
3. Safer/easier Rust API only when zero-cost or off the hot path.
4. No free safety tax on a benchmarked hot path unless explicit opt-in.
5. Simplicity only when 1–4 are equal.

================================================================
HFT LATENCY SURFACES (cluster client — rank by importance)
================================================================
1. try_claim → SessionMessageHeader encode into claim (every app message)
2. Egress decode dispatch (SessionEvent / NewLeaderEvent / app fragments)
3. SessionKeepAlive encode (periodic)
4. Connect / auth / failover (cold path — correctness > nanoseconds)

================================================================
HONEST CURRENT STATE (2026-07-18, branch first_cut)
================================================================
DONE for current scope:
- sbe: 10/10 maintained ErgoSBE/Aeron ratios ≤ 1.00 (encode/throughput_10k
  = 0.917 after Aeron header/body-overlap bench fix ba82368); MSRV 1.95.
- persist: 7/7 live ClickHouse tests (Docker).
- samples IPC path: exchange-orderbook + advanced-bitget live-green via
  just samples-orderbook; advanced-bitget three-thread AppMessage + dynamic
  V2 path implemented (rusteron 0.2.1 IPC — NOT cluster).
- cluster production codecs: ErgoSBE build.rs → OUT_DIR; call sites on
  ergo_codecs; 18/18 golden parity; harness suite previously green vs Java;
  53 lib tests.
- cluster encode 5-run matrix ledgered: header 0.856, keep-alive 0.916 PASS;
  connect 1.001 demoted (cold path). Decode benches present (f01e334).
- Connect re-offer: DONE (sync + async PollResponse cadence).
- Log-recovery restart test: DONE (ae6f4c9, #[ignore] destructive).
- HA sample pure modules: ha_book + latency in advanced-bitget (unit-tested).
- RFQ codecs: frozen sbe-tool (no schema XML); keep compiling; do not unfreeze
  without human OK.

OPEN residual after 2026-07-18 completion pass (none blocking umbrella):
1. Optional: promote decode benches into maintained ≤1.00 set after equal-work audit.
2. Optional: multi-node Java kill-leader HA harness (offline H3 + connect harness green).
3. sbe-tool trees still compile for benches + frozen RFQ only (intentional).

================================================================
HARD INVARIANTS
================================================================
- Never run cargo … --workspace --all-features without
  --exclude ergo-aeron-cluster (test-harness pulls Java/Gradle).
- Preserve dirty simple-binary-encoding submodule; stage files explicitly,
  never git add -A. Never commit aeron-cluster-[0-9]/ runtime dirs.
- Keep cluster/rustfmt.toml (edition 2024, max_width 120).
- Commit messages: one sentence, conventional prefix. Commit only coherent
  verified slices; push after each milestone when authorised.
- New Rust tests: Result<(), Box<dyn Error>> with ?, no unwrap.
- Generated hot paths allocate no heap; no transmute from wire buffers.
- Do not claim tests/benches ran unless they ran in the current worktree.

================================================================
MAINTAINED CLUSTER BENCH SCENARIOS
================================================================
Hot (must ≤ 1.00, five-run medians + Criterion CIs, ledgered in
ergosbe-performance-optimisation-goal.md):
- SessionMessageHeader encode (claim-shaped buffer)
- SessionKeepAlive encode
- SessionEvent / NewLeaderEvent / SessionMessageHeader decode (ADD if missing)

Cold but currently maintained (still ≤ 1.00 until demoted with human OK):
- SessionConnectRequest encode — OPEN at ~1.003 on first Criterion run

Optional later: claim-write microbench (header + fixed app payload mimicking
try_claim layout). Measure; do not invent a new public API for the bench.

================================================================
WORK ORDER (residual only — do not re-migrate production codecs)
================================================================

0) Living-doc truth
   Keep master plan + this prompt + perf ledger consistent after every slice.

1) Cluster perf honesty
   a. just bench-cluster five warmed runs; record medians, CIs, hardware,
      toolchain, profile, date in ergosbe-performance-optimisation-goal.md.
   b. SessionConnectRequest: if stable ratio > 1.00 after five runs, either
      (i) smallest measured fix that does not regress header/keep-alive, or
      (ii) ask human to demote connect from maintained (cold-path rationale).
   c. Acceptance: every maintained scenario five-run median ≤ 1.00 OR written
      demotion with human OK. Single Criterion run is not acceptance.

2) HFT benches (measure the real path)
   - Egress decode of SessionEvent + NewLeaderEvent (+ SessionMessageHeader
     if not covered).
   - Optional claim-shaped header+payload encode.
   - Record in perf ledger. Gate: ≤ 1.00 vs sbe-tool while dual-codec remains;
     after cleanup, absolute ErgoSBE baselines + no regression vs archived table.

3) Dual-codec residual cleanup
   - Production already uses ergo_codecs. Residual: test boilerplate,
     cluster_codecs/ + generated/, benches’ sbe-tool arms, just recipes
     generate-aeron-cluster-codecs / check-aeron-cluster-codec-drift.
   - Before deleting sbe-tool protocol codecs: goldens must stand alone;
     archive sbe-tool baseline numbers if head-to-head arms go away.
   - RFQ stays frozen sbe-tool; writer_impls.rs stays while RFQ needs it.
   - Acceptance: 53 lib + harness green; clippy -D warnings + fmt; no broken
     examples.

4) Reliability — connect re-offer (pre-election)
   Reality: SessionConnectRequest is sent once. If the first offer lands on a
   pre-election node that neither leads nor redirects, connect times out.
   Fix: periodic re-offer while connecting / PollResponse, within timeout
   (mirror Java AeronCluster where applicable).
   Process: failing harness test first → fix → green.

5) Reliability — log-recovery restart
   Current restart tests use fresh cluster (dirDeleteOnStart). Add a test that
   preserves archive/cluster dirs, restarts, reconnects, asserts recovered
   continuity. May stay #[ignore] if destructive, but must pass with --ignored.

6) HA sample (cluster feed + never-stale book + dynamic latency)
   Authority:
   - docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md
   - samples/todo/02-cluster-ha-orderbook-latency.md
   Depends on slices 4–5 being green enough to host failover tests.

   Path (keep samples/ directory; extend advanced path — no new top-level pillar):
     feed → normalize AppMessage(L2Book|Trade)
       → ergo-aeron-cluster try_claim ingress (HA)
       → Java clustered service (v1 harness; Rust service non-goal for first cut)
       → egress follower with leadership-aware book
       → never serve stale book across NewLeader / session release / reconnect
       → latency DynamicSchema + DynamicRow → ClickHouse

   Stale-book policy (default):
     serving=false on NewLeaderEvent / reconnect / session release;
     clear or freeze book; only resume after term-valid snapshot;
     increments require same leadership_term_id and continuous sequence;
     never silent merge across term.

   Latency: register runtime DynamicSchema (no new Persist DTO required);
   DynamicRow columns include instrument, leadership_term_id,
   cluster_session_id, sequence, exchange_ts_ns, receive_ts_ns,
   ingress_claim_ts_ns, egress_decode_ts_ns, book_apply_ts_ns, and deltas.
   Match advanced-bitget drop/batch policy — do not block the book path on CH.
   rusteron pin: advanced-bitget is 0.2.1, cluster is 0.2.4 — decide one pin
   explicitly before coding (ask human if bumping).

   Acceptance H1–H8 (see design spec): try_claim publish; stale on release;
   consistent book after failover; CH latency schema+rows; hot-path policy;
   IPC baseline still green; just samples-cluster-ha (or equivalent) recipe.

7) Full umbrella gates (fresh output required)
   just check
   cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
   cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1
   cargo test --workspace --exclude ergo-aeron-cluster -- --include-ignored --test-threads=1
   just samples-orderbook
   just check-aeron-cluster
   just build-aeron-jars && just test-aeron-cluster-harness
   just bench-cluster   # then five-run + ledger entry
   cargo bench -p ergosbe-benchmarks --no-run
   # full perf_parity_bench matrix: every maintained ErgoSBE/Aeron ratio ≤ 1.00
   cargo +1.95.0 check --workspace --all-targets --all-features --exclude ergo-aeron-cluster
   # HA sample recipe when implemented
   rg -n "ACTIVE|IN PROGRESS|PARKED|DEFERRED|ENV-GATED|OFFLINE" \
     sbe/todos persist/todo samples/todo docs -g '*.md'

8) Status hygiene
   After every slice update master plan, affected todos, and evidence ledgers
   with dated output. Mark nothing DONE without a passing command. Label
   superseded claims; do not delete history. Resolve or formally CLOSE every
   live status-scan hit (HA todo may stay ACTIVE until H1–H8 pass).

================================================================
PER-SLICE LOOP
================================================================
Inspect git status; write or identify a failing test first; smallest correct
change; targeted checks then relevant full gate; inspect generated output when
codegen changes; update evidence; commit the coherent slice when authorised.

Ask the human only when a choice would trade away wire compatibility or
measured performance, needs credentials/paid access, requires unfreezing RFQ,
demoting a maintained bench scenario, or bumping the HA sample rusteron pin.

================================================================
FINAL COMPLETION REQUIRES ALL OF
================================================================
- Maintained cluster benches five-run ≤ 1.00 (or demoted with human OK)
- HFT decode/claim benches recorded when in maintained set
- Dual-codec residual cleaned or explicitly residual-documented
- Both reliability gaps closed with asserting harness tests
- HA sample H1–H8 green OR explicit human deferral recorded in master plan
- Every gate in step 7 green with fresh output
- Status scan clean except accepted live/manual exceptions
- Master plan + perf ledger updated; coherent commits pushed

A compiling skeleton, a stale checkbox, ratio 1.003 labelled “pass”, or a
compile-only benchmark result does not count.
```
