# Living backlog — verified-open items only

**Status:** LIVING — update when an item is closed with evidence.  
**Last audit:** 2026-07-19 · branch `first_cut` (URI builder + Result/? + thiserror)  
**Not this file:** process checklists in `ergosbe-performance-optimisation-goal.md`,
the full `sbe/todos/` graveyard, or historical rusteron phase docs.

## How to use

1. Prefer this list over unchecked boxes in old goal files.  
2. Close an item only with generated/source evidence + tests.  
3. Do **not** re-open residual umbrella product work — that scope is COMPLETE.

## Intentional non-goals (do not add as open work)

| Item | Why |
|------|-----|
| Rust Aeron Cluster **service** | Explicit non-goal |
| SessionConnectRequest encode ≤ 1.00 as a gate | Demoted cold path |
| NewLeaderEvent decode ≤ 1.00 as a gate | Diagnostic-only; smoke ratio ≫ 1.00 after equal-work audit |
| Delete residual sbe-tool codec trees | Needed for head-to-head benches |
| Pillar directory renames | Permanent layout rule |
| Live exchange WebSocket in CI | Manual recipe only (IPC + HA samples offline/CH) |
| Third sample crate (exchange-orderbook) | Merged into advanced-bitget |
| Live harness always-on CI | Env-gated by design |
| Cluster beyond prototype quality | Explicit experimental banner on crate |

---

## A. SBE generator / API

**Open product gaps:** *(none)*

All prior §A items closed 2026-07-19 — see section D.

---

## B. Documentation debt

| Item | Status |
|------|--------|
| Historical rusteron / phase2 / perf playbook stamps | DONE (prior commit) |
| `generated-api.md` 62/156 alignment | DONE (prior) |

---

## C. Optional polish (non-blocking)

**Open optional items:** *(none)*

All prior §C items closed 2026-07-19 — see section D.

---

## D. Closed (do not re-queue)

### 2026-07-19 rusteron URI / Result / thiserror polish

| Item | Resolution | Evidence |
|------|------------|----------|
| Tests/main `Result` + `?` for fallible setup | DONE | Cluster/persist/sbe/samples signatures; setup paths prefer `?` |
| Cached channel `CString`s on `SessionBuilder` | DONE | `ingress_cstr` / `egress_cstr`; connect reuses |
| `AeronUriStringBuilder` for all channels | DONE | `cluster/src/uri.rs` + client reconnect/redirect |
| `thiserror` for public errors | DONE | `ClusterError`, `SinkError` |
| No Tokio in `ergo-aeron-cluster` | DONE | Aeron poll-driven `AsyncClusterConnect` only; crate grep clean |
| Living backlog / perf note | DONE | This entry; ledger note under perf goal (API hygiene, ratios unchanged) |

### 2026-07-19 README + ErgoSBE claim/nested API polish

| Item | Resolution | Evidence |
|------|------------|----------|
| README start-here / claim recipe | DONE | Root, sbe, cluster, samples READMEs; `sbe/docs/guide/claim-nested-encode.md` |
| `HEADER_LENGTH` / `after_this_message` | DONE | Fixed messages; samples + cluster use `ENCODED_LENGTH` |
| Group `try_add` | DONE | Generated; HA + advanced-bitget publish paths |
| `with_external_sbe_rt` | DONE | `GenerationConfig` |
| Stale sample todos / root roadmap bloat | DONE | Historical stamps; compressed root status |

### 2026-07-19 §C optional polish closeout

| Item | Resolution | Evidence |
|------|------------|----------|
| claim_shaped encode maintained ≤1.00 | DONE | Bench acceptance lists claim_shaped; smoke 2026-07-19 Ergo 9.34 µs / sbe-tool 9.37 µs ≈ **0.997** |
| NewLeaderEvent decode ≤1.00 gate | WON'T-DO | Smoke Ergo 12.3 µs / sbe-tool 5.46 µs ≈ **2.26**; diagnostic-only non-goal |
| `cargo test -p ergo-aeron-cluster --doc` | DONE | Multi-line schema descriptions fenced as text code blocks in generator; `cargo test -p ergo-aeron-cluster --doc` green |
| Live harness always-on CI | WON'T-DO | Intentional env gate |
| Cluster beyond prototype | WON'T-DO | Experimental banner retained |

### 2026-07-19 living-backlog closeout

| ID | Resolution | Evidence |
|----|------------|----------|
| **SBE-81** | DONE | `into_*_as_message` / `try_*_as_message` in golden; `baseline_test` nested_message_* (4 tests) |
| **SBE-20** | DONE | All 11 error-handler fixtures reject parse; `error_handler_schemas_all_rejected` |
| **SBE-REF** | DONE | Multi-pass `<ref>` + nested enum/set/composite; Engine `[u8; 10]` / Booster `[u8; 2]` / Car BLOCK_LENGTH 45; nested BeginComposite size resolve; `composite_ref_engine_roundtrip_compile` + full `baseline_test` (92) encode/decode vs Aeron fixture |
| **SBE-110** | WON'T-DO | Maintained decode ≤1.00; no fairness failure |
| **27 / 86** | DONE | Encoder `wrap` → `Result` in golden |
| **62 / 156 / 157** | DONE | Prior verify-and-close (SbeDecimal, try_*, complete as_bytes) |

### Earlier umbrella closeout

- Residual product COMPLETE; HA + RFQ + quality P0–P4  
- Maintained cluster encode/decode (header/event/claim_shaped) ≤ 1.00  

---

## Pointers

| Doc | Role |
|-----|------|
| This file | **Only** verified-open + intentional non-goals |
| [`docs/superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md`](superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md) | Living umbrella orientation |
| [`sbe/design/DECISIONS.md`](../sbe/design/DECISIONS.md) | SBE design authority |
| [`sbe/todos/`](../sbe/todos/) | Historical inventory |
