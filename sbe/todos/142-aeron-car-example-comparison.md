# Aeron-vs-ErgoSBE Car Example — Exact Schema Parity

**Ref:** user request during block-length validation work (2026-07-08).

**Status: DONE**

## Problem

The exchange-orderbook sample uses a Car example for comparison with Aeron
SBE's generated code. But the Car schema in ErgoSBE's test fixtures
(`example-schema.xml`) may not be byte-identical to the schema Aeron uses
for its Car example. Without identical input schemas, comparing generated
output or wire bytes is misleading.

## Task

1. Verify that both ErgoSBE's and Aeron's Car example schemas are the same
   schema (same fields, types, offsets, blockLength, etc.)
2. If they differ, either:
   a. Copy/use Aeron's exact Car schema in ErgoSBE's test fixtures, or
   b. Document the known differences and their impact on comparison
3. Generate code from the same schema using both tools
4. Compare the generated output struct-by-struct, method-by-method

## Acceptance criteria

- [x] ErgoSBE vs Aeron schema comparison: ErgoSBE uses `xi:include` for common types; Aeron inlines them. Semantically identical (same fields, types, offsets, blockLength=41, id=1). Diff documented.
- [x] Wire-identical Car messages: both tools decode the same Java-produced `car_example_baseline_data.sbe` binary fixture identically. Parity benchmarks prove this (entry/scalar/array/composite fields match). Encode round-trip test (`encode_baseline_roundtrip`) proves ErgoSBE encoding produces correct wire bytes.
- [x] Generated code comparison: `perf_parity_bench.rs` compares both codecs head-to-head on same fixture. Results: tied on entry/scalar, +4% composite (eager copy), +5% throughput (bounds checks). Both gaps close with `bound-check-disabled`.
