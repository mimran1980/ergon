# Aeron-vs-ErgoSBE Car Example — Exact Schema Parity

**Ref:** user request during block-length validation work (2026-07-08).

**Status: DESIGN / ROADMAP**
**Status: DESIGN / ROADMAP**

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

- [ ] ErgoSBE Car schema (`example-schema.xml`) confirmed identical to Aeron
      Car schema (`sbe-tool/src/test/resources/example-schema.xml`), or
      differences are documented
- [ ] Wire-identical Car messages can be produced by both tools from the
      same schema
- [ ] Generated code comparison produces meaningful results (not artifacts
      of different schemas)
