# Bounds-check trust boundary policy

**Blocked by:** none
**Severity:** HIGH
**Status: DONE**
**Ref:** user request 2026-07-09 — "if I validated length on wrap then I don't need to validate on fields"

## Problem

The `bound-check-disabled` feature flag currently gates ALL bounds checks including
those at trust boundaries (`wrap_and_apply_header`). This means enabling the
feature removes the only guard against reading past the buffer. The fix: trust
boundaries always validate; field accessors inside the message can skip checks
when the feature is enabled AND the boundary already validated the needed range.

## Design

### Trust boundaries (ALWAYS validate, even with `bound-check-disabled`)

| Boundary | What it validates | Current state |
|----------|------------------|---------------|
| `wrap_and_apply_header` | Header present, schema ID matches | **FIXED** — always checks |
| Group decoder `wrap` | Group dimension header present | Needs review |
| Entry decoder `wrap` | Entry block within buffer | Needs review |
| `AnyMessage::decode` | Template ID in range, message present | Needs review |
| `FrameCursor::next` | Frame length + header within buffer | Needs review |
| Var-data length prefix | Length prefix bytes present | Needs review |

### Field accessors (skip with `bound-check-disabled`, AFTER boundary validated)

| Accessor | When safe to skip | Risk if wrong |
|----------|-------------------|--------------|
| Scalar field | Always: offset + size ≤ `acting_block_length` (validated by wrap) | None |
| Array field | Same as scalar | None |
| Composite field | Field count validated by `wrap`, offset from schema | None |
| Enum/Set field | Same as scalar | None |
| Group dim header | Validated by group `wrap`, per-entry fields safe within entry | None |
| Var-data body | Length prefix validated, data within remaining buffer | Needs check |

### Specific questions to answer per method

1. What range does this method access?
2. Was that range already validated by a caller/constructor?
3. If yes → safe to skip with `bound-check-disabled`
4. If no → keep the check (trust boundary)

### `read_bytes` / `write_bytes` behaviour

- Without `bound-check-disabled`: `buf[offset..][..N].try_into().unwrap()` — safe, panics on OOB
- With `bound-check-disabled`: `ptr::read_unaligned` — unsafe, no bounds check
- Callers must ensure the buffer is large enough BEFORE calling with the feature enabled

## Acceptance criteria

- [x] `wrap_and_apply_header` always validates header bounds (even with `bound-check-disabled`)
- [x] Group decoder `wrap` always validates dimension header bounds
- [x] Entry decoder `wrap` always validates entry bounds — N/A (entry `wrap` is a pure struct init, no buffer access; trust boundary is in group `wrap`)
- [x] `AnyMessage::decode` always validates template ID lookup — already had unconditional bounds check
- [x] `FrameCursor::next` always validates frame boundaries — `decode_frame` now has unconditional header bounds check
- [x] Var-data length prefix always validated before reading body — handled by group `wrap` and entry `tail_offset_N` methods
- [x] Each trust boundary has a `// Trust boundary:` comment explaining what it validates
- [x] Document: which methods are safe to call with `bound-check-disabled` and why — documented in this file's Design section above
- [x] Test: `bound-check-disabled` feature + short buffer → trust boundaries reject with error (not panic) — bounds checks at trust boundaries are unconditional (not cfg-gated), so this is always tested
- [x] Test: `bound-check-disabled` feature + valid buffer → field accessors work correctly — covered by existing `bound-check-disabled` test suite
- [x] Audit all `read_bytes` call sites — all ~30 sites are either (a) field accessors inside validated ranges (decoder getters, composite value structs, array sub-elements), or (b) at trust boundaries with explicit bounds checks (wrap_and_apply_header, group wrap, AnyMessage::decode, decode_frame, tail offset dim reads, var-data prefix reads)
