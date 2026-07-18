# Living backlog — verified-open items only

**Status:** LIVING — update when an item is closed with evidence, or when a
new gap is proven against current code (not against historical plans).  
**Last audit:** 2026-07-19 · branch `first_cut`  
**Not this file:** process checklists in `ergosbe-performance-optimisation-goal.md`,
the full `sbe/todos/` graveyard, or historical rusteron phase docs.

## How to use

1. Prefer this list over unchecked boxes in old goal files.  
2. Close an item only with: generated/source evidence + tests (and benches if
   the item is performance-scoped).  
3. Do **not** re-open residual umbrella product work (cluster HA, RFQ unfreeze,
   maintained encode/decode gates) — that scope is COMPLETE.

## Intentional non-goals (do not add as open work)

| Item | Why |
|------|-----|
| Rust Aeron Cluster **service** | Explicit non-goal |
| SessionConnectRequest encode ≤ 1.00 as a gate | Demoted cold path |
| Delete residual sbe-tool codec trees | Needed for head-to-head benches |
| Pillar directory renames | Permanent layout rule |
| Live exchange WebSocket in CI | Manual recipe only (samples DONE offline/CH) |

---

## A. SBE generator / API (real product gaps)

| ID | Item | Evidence it is still open | Close when |
|----|------|---------------------------|------------|
| **SBE-81** | Nested var-data message bridges | Todo `81` REOPENED; golden has `into_*_as_message` / `try_*_as_message` but todo still requires full `as_decoder`/`as_message` acceptance set | Golden + tests match DECISIONS §3 bridges; todo 81 flipped DONE with AC checkboxes |
| **SBE-20** | Parser semantic validation vs upstream error-handler suite | `sbe/tests/error_validation_test.rs` lists schemas **not yet rejected** (enum range, group dimensions, cyclic refs, …) | Each listed schema produces a clear `ParseError` with tests |
| **SBE-REF** | `<ref>` inside composites | `baseline_test` notes: not handled inside composites | Fixture schema + roundtrip or documented WON'T-DO |
| **SBE-110** | Var-data tail offset cache (opt) | Todo `110` RE-OPENED for decode composition micro-opts | Only if a maintained decode ratio fails fairness after equal-work audit; else WON'T-DO |

### Verify-and-close candidates (likely stale REOPENED)

Re-open only if source audit fails; otherwise flip the todo and stop.

| ID | Claim | Current code signal |
|----|--------|---------------------|
| **27 / 86** | Encoder wrap still panics / non-Result | Golden `wrap_and_apply_header` returns `Result` for encode + decode |
| **62** | Decimal converters not shipped | `SbeDecimal` + `enable_decimal_converters` in codegen; samples use it |
| **156** | Fallible stage combinators not shipped | Golden emits `try_fixed` and related `try_*` |
| **157** | Partial `as_bytes` still publishable | Todo marked DONE 2026-07-18; guide text may lag |

---

## B. Documentation debt (not product features)

| Item | Action |
|------|--------|
| Historical rusteron gap plan residual banner | Stamped superseded (see that file header) |
| Completion prompt “Optional later” NewLeader/claim | Stamped DONE (benches exist) |
| `sbe/docs/guide/generated-api.md` “not yet shipped” for 62/156 | Aligned with shipped APIs |
| Unchecked process boxes in `ergosbe-performance-optimisation-goal.md` | Treat as historical playbook, not sprint board |
| `phase2-completion-goal.md` HA ACTIVE line | Stamped HA DONE |

---

## C. Optional polish (non-blocking)

| Item | Notes |
|------|--------|
| Promote NewLeaderEvent decode / claim-shaped benches to **maintained** ≤1.00 set | Benches exist; only promote after equal-work smoke ledgered in perf goal |
| `cargo test -p ergo-aeron-cluster --doc` clean | Generated schema description ASCII triggers rustdoc codeblock warnings — codegen/doc hygiene |
| Live `just test-aeron-cluster-harness` in CI | Env-gated (Java jars); recipe exists |
| Cluster client beyond prototype quality | Explicit experimental banner; not a residual checklist |

---

## D. Closed recently (do not re-queue)

- Umbrella residual product (completion goal FINAL COMPLETION)  
- HA sample H1–H8 + kill-leader  
- RFQ unfreeze (schema 101 ErgoSBE)  
- Quality track P0–P4 (READMEs, rustdocs, decode helpers, hygiene)  
- Maintained cluster encode (header/keep-alive) + decode (header/event) ≤ 1.00  

Evidence: master plan §5b, completion prompt, commits on `first_cut` through quality track.

---

## Pointers

| Doc | Role |
|-----|------|
| This file | **Only** verified-open + intentional non-goals |
| [`docs/superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md`](superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md) | Living umbrella orientation |
| [`docs/superpowers/plans/2026-07-18-completion-goal-prompt.md`](superpowers/plans/2026-07-18-completion-goal-prompt.md) | Residual product COMPLETE (historical closeout) |
| [`sbe/design/DECISIONS.md`](../sbe/design/DECISIONS.md) | SBE design authority |
| [`sbe/todos/`](../sbe/todos/) | Historical inventory — filter by REOPENED, do not sprint the whole tree |
