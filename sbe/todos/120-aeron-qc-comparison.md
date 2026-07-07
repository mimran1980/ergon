# Aeron-vs-ErgoSBE quality control — compare and copy what's better

**Ref:** user request. Aeron Rust SBE is the reference implementation.

## Problem

We need systematic quality control: for every feature ErgoSBE generates, compare
against the equivalent Aeron Rust output and ask:

1. **Is the Aeron code faster?** → Copy Aeron's approach (don't reinvent).
2. **Is the Aeron code simpler?** → Copy Aeron's API shape.
3. **Is our code bloated?** → Trim to match Aeron's output size.
4. **Does Aeron handle an edge case we miss?** → Fix it.

Aeron has years of production battle-testing. We should default to "copy Aeron"
unless there's a specific Rust-idiomatic reason to diverge.

## Schemas to compare

1. **Car example** (`car.xml`) — baseline comparison, small schema
2. **Bitget spot** (`bitget-spot.xml`) — orderbook depth, real-world schema
3. **Bitget trades** (`bitget-trades.xml`) — trade ticks, high-throughput schema

For each schema, generate both ErgoSBE and Aeron Rust output, then compare
method-by-method. Think: is ErgoSBE better? If not, copy Aeron.

## Method

For each generated type/function, diff ErgoSBE output against Aeron Rust output:

| Area | Aeron approach | ErgoSBE approach | Verdict |
|------|---------------|------------------|---------|
| Scalar reads | `get_bytes_at<const N>` helper → `from_le_bytes` | Byte-by-byte `while j < N` loop inline (SLOWER) | Copy Aeron |
| Array reads | ??? | Inline loop per element | Compare |
| Composite reads | Flyweight decoder with offset | Both flyweight + value struct | Compare |
| Bounds checks | ??? | `#[cfg]` per accessor | Compare |
| Encoder | ??? | Type-state pattern | Compare |
| Group iteration | ??? | Iterator with per-entry decode | Compare |

## Acceptance criteria

- [ ] Full diff of ErgoSBE `car_example.rs` vs Aeron `aeron_car.rs` completed
- [ ] Every divergence catalogued with rationale ("copy Aeron" or "deliberate departure")
- [ ] Byte-by-byte loops replaced with `get_bytes_at`-style helper (or ReadBuf)
- [ ] Array reads use bulk `copy_from_slice` / `as_chunks` like Aeron
- [ ] Bounds-check gating centralized (not duplicated per accessor)
- [ ] Encoder closure borrow issue resolved (Aeron uses different pattern)
- [ ] Generated code size comparable (within 20% of Aeron line count)
- [ ] Perf benchmark: ErgoSBE within 10% of Aeron decode speed
- [ ] **Re-verify all previously-done todos** after each major codegen change — a todo marked `[x]` may have been verified on stale generated code. The golden file is the ground truth; if the golden file changed, re-check every `[x]` item that depends on it.
