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
  connect 1.001 demoted (cold path). Decode equal-work promoted: header 0.918,
  event 0.906 (maintained).
- Connect re-offer: DONE (sync + async PollResponse cadence).
- Log-recovery restart test: DONE (ae6f4c9, #[ignore] destructive).
- HA sample: `samples/cluster-ha-orderbook` + `just samples-cluster-ha` —
  try_claim publish, never-stale book, LatencyPersistor → CH (H1–H8 residual);
  multi-node kill-leader green (`just samples-cluster-ha-kill-leader`).
- RFQ codecs: **unfrozen** — vendored `cluster/schemas/protocol-codecs.xml`
  (cookbook schema 101); ErgoSBE `ergo_rfq_codecs` production path; sbe-tool
  residual retained only for wire-parity tests + benches.

OPEN residual after 2026-07-18 optionals pass (none blocking; all 3 DONE):
1. ~~Optional: promote decode benches~~ **DONE** — equal-work audit; header
   0.918, event 0.906 promoted to maintained ≤1.00.
2. ~~Optional: multi-node Java kill-leader HA harness~~ **DONE** —
   `samples-cluster-ha-kill-leader` / `ha_kill_leader` green.
3. sbe-tool trees still compile for head-to-head benches only (intentional).

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
Hot / maintained (must ≤ 1.00, five-run ledgered in
ergosbe-performance-optimisation-goal.md; fresh smoke OK to re-confirm):
- SessionMessageHeader encode (claim-shaped buffer) — five-run **0.856**
- SessionKeepAlive encode — five-run **0.916**
- SessionMessageHeader decode — equal-work smoke **0.918** (promoted 2026-07-18)
- SessionEvent decode — equal-work smoke **0.906** (promoted 2026-07-18)

Demoted / cold-path (NOT maintained; human OK 2026-07-18):
- SessionConnectRequest encode — first-run ~1.003 / five-run ~1.001 noise;
  handshake-only; do not gate residual completion on this ratio.

Optional later (benches **exist** as of 2026-07-19 quality track; not required
in the maintained ≤1.00 set until equal-work smoke is ledgered as maintained):
- ~~claim-write microbench~~ **DONE** as Criterion group
  `cluster/encode/claim_shaped_header_plus_app` (diagnostic unless promoted)
- ~~NewLeaderEvent decode~~ **DONE** as Criterion group
  `cluster/decode/new_leader_event` (diagnostic unless promoted)

Verified-open product/generator items only:
[`../../LIVING_BACKLOG.md`](../../LIVING_BACKLOG.md).

================================================================
WORK ORDER — residual product scope COMPLETE (2026-07-18)
================================================================

0) Living-doc truth — ongoing after each change (master plan, this prompt,
   perf ledger stay consistent).

1) Cluster perf honesty — DONE
   Five-run maintained encode matrix ledgered (header 0.856, keep-alive 0.916);
   connect demoted cold-path. Fresh smoke re-confirms maintained ratios ≤ 1.00.

2) HFT decode/claim benches — DONE (equal-work promote 2026-07-18)
   sbe-tool arm now matches Ergo header/template/schema + var-data gates in
   release. Smoke ratios header **0.918**, event **0.906** — maintained set.

3) Dual-codec residual cleanup — DONE for production/protocol + RFQ
   Protocol goldens/tests use ErgoSBE only; `generated/` deleted.
   RFQ unfrozen: `schemas/protocol-codecs.xml` → `ergo_rfq_codecs`.
   sbe-tool trees retained only for head-to-head benches.

4) Connect re-offer (pre-election) — DONE
   Sync handshake + async PollResponse re-offer on connect_reoffer_interval_ms.

5) Log-recovery restart — DONE
   `test_log_recovery_restart` with preserve-dirs launcher (`ae6f4c9`,
   `#[ignore]` destructive; pass with --ignored).

6) HA sample — DONE residual scope
   Crate `samples/cluster-ha-orderbook`, recipe `just samples-cluster-ha`.
   Pin: HA uses rusteron 0.2.4 via ergo-aeron-cluster; IPC advanced-bitget
   stays 0.2.1. H1–H8 residual: try_claim-shaped publish, never-stale book,
   LatencyPersistor (DynamicSchema→DynamicRow→decode→ClickhouseSink),
   offline H3-equivalent, IPC baseline green. Multi-node Java kill-leader:
   `just samples-cluster-ha-kill-leader` DONE.

7) Full umbrella gates — **re-run 2026-07-18 closeout PASS** (fresh output)
   just check
   cargo test -p ergo-aeron-cluster --lib   # 54 incl. RFQ wire parity
   cargo test -p ergo-aeron-cluster --test codec_golden_bytes  # 9
   just samples-cluster-ha
   just samples-cluster-ha-kill-leader
   just samples-orderbook
   cargo bench -p ergo-sbe-benchmarks --no-run
   cargo bench -p ergo-aeron-cluster --bench cluster_codec_bench  # maintained filter
   # optional: just test-aeron-cluster-harness (Java jars)
   rg living residual OPEN/ACTIVE product blockers (must be CLEAN)
   Maintained smoke ratios: enc hdr 0.860, ka 0.916, dec hdr 0.873, event 0.849

8) Status hygiene — DONE for residual product scope
   HA todo DONE; design IMPLEMENTED; master plan §3/§3b/§4/§5 residual DONE.
   Optional follow-ups only in OPEN residual list above.

================================================================
FINAL COMPLETION (residual product scope) — MET WHEN
================================================================
- Maintained cluster encode five-run ≤ 1.00; connect demoted with human OK
- Dual-codec production path ErgoSBE-only; RFQ ErgoSBE via vendored schema 101
- Reliability gaps closed (re-offer + log-recovery)
- HA H1–H8 residual green; multi-node kill-leader green
- Decode benches equal-work + promoted ≤1.00
- Umbrella gates green with fresh output on closeout pass
- Living docs agree DONE; commits pushed

All three former optionals (decode promote, kill-leader, RFQ unfreeze) DONE.
```
