# Living backlog — verified-open items only

**Status:** LIVING — update when an item is closed with evidence.  
**Last audit:** 2026-07-19 · branch `first_cut` (living-backlog closeout)  
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
| Delete residual sbe-tool codec trees | Needed for head-to-head benches |
| Pillar directory renames | Permanent layout rule |
| Live exchange WebSocket in CI | Manual recipe only (samples DONE offline/CH) |

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

| Item | Status |
|------|--------|
| Promote NewLeaderEvent decode to maintained ≤1.00 | **Left open** — smoke ratio ~2.28 (Ergo 12.2 µs / sbe-tool 5.35 µs); keep diagnostic-only until equal-work re-audit of sbe-tool arm |
| Promote claim-shaped encode to maintained ≤1.00 | **Eligible** — smoke ratio ~0.989 (9.28 / 9.38 µs); ledger optional, not a product gap |
| `cargo test -p ergo-aeron-cluster --doc` clean | Left open — generated schema ASCII docblocks |
| Live harness always-on CI | Env-gated by design |
| Cluster beyond prototype quality | Explicit experimental banner |

---

## D. Closed (do not re-queue)

### 2026-07-19 living-backlog closeout

| ID | Resolution | Evidence |
|----|------------|----------|
| **SBE-81** | DONE | `into_*_as_message` / `try_*_as_message` in golden; `baseline_test` nested_message_* (4 tests) |
| **SBE-20** | DONE | All 11 error-handler fixtures reject parse; `error_handler_schemas_all_rejected` |
| **SBE-REF** | DONE | Multi-pass composite parse expands `<ref>`; Engine `[u8; 9]` + efficiency/booster accessors |
| **SBE-110** | WON'T-DO | Maintained decode ≤1.00; no fairness failure |
| **27 / 86** | DONE | Encoder `wrap` → `Result` in golden |
| **62 / 156 / 157** | DONE | Prior verify-and-close (SbeDecimal, try_*, complete as_bytes) |

### Earlier umbrella closeout

- Residual product COMPLETE; HA + RFQ + quality P0–P4  
- Maintained cluster encode/decode (header/event) ≤ 1.00  

---

## Pointers

| Doc | Role |
|-----|------|
| This file | **Only** verified-open + intentional non-goals |
| [`docs/superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md`](superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md) | Living umbrella orientation |
| [`sbe/design/DECISIONS.md`](../sbe/design/DECISIONS.md) | SBE design authority |
| [`sbe/todos/`](../sbe/todos/) | Historical inventory |
