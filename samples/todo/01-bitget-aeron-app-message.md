# Three-thread Bitget AppMessage to Aeron IPC to ClickHouse sample

**Blocked by:** `sbe/todos/81-vardata-as-decoder-as-message.md`,
`sbe/todos/156-fallible-stage-combinators.md`, Decimal-array dynamic messages,
and foreground ClickHouse persistence
**Severity:** HIGH
**Status: ACTIVE (approved design 2026-07-11; not implemented)**

## Authority

Implement the complete design in
`docs/superpowers/specs/2026-07-10-bitget-aeron-clickhouse-sample-design.md`.
Canonical generated-interface and performance decisions remain in
`sbe/design/DECISIONS.md`.

This is the advanced successor to the historically completed offline sample in
`00-e2e-orderbook-persist.md`. Do not reinterpret that older DONE record as
evidence for this pipeline.

## Application envelope

The normalized application schema contains `AppMessage`, `L2Book`, and `Trade`.
Every L2 book and trade publication on the typed IPC stream is wrapped:

```text
AppMessage
  sentTs: uint64 Unix epoch nanoseconds
  appName: UTF-8 var-data
  payload: complete same-schema L2Book or Trade, including SBE header
```

`DynamicSchema`, `DynamicRow`, `DynamicSchemaV2`, and `DynamicRowV2` are
platform infrastructure messages. Publish them directly and unwrapped on the
separate dynamic stream.

## Decimal contract

Every normalized price and quantity uses the SBE `Decimal` composite
`{ mantissa: int64, exponent: int8 }`. Enable the generated generic
`SbeDecimal` conversion seam for that composite and implement it in the sample
for `rust_decimal::Decimal`; keep raw `*_wire` methods available. Persist book
arrays as `Array(Decimal(38,18))` through exact checked rescaling. Reject
overflow, unsupported adapter ranges, rounding, and non-zero precision loss.

## Required implementation sequence

- [ ] Implement and prove the nested var-data decode bridge in SBE todo 81.
- [ ] Implement and benchmark both manual stages and fallible closure helpers
      in SBE todo 156.
- [ ] Define the normalized application schema and exercise every supported XML
      documentation source.
- [ ] Implement and prove the generic `SbeDecimal` converter seam, the
      `rust_decimal::Decimal` sample adapter, raw wire access, mixed exponents,
      exact reverse conversion, zero allocation, and comparable Aeron work.
- [ ] Prove official-Aeron bytes for outer `AppMessage` plus nested `L2Book` and
      `Trade`.
- [ ] Add exact inner and outer
      `compute_encoded_length_with_message_header` calculations.
- [ ] Encode both layers directly into one Rusteron 0.2.1 `try_claim_owned`
      buffer with no temporary message or copy.
- [ ] Keep dynamic infrastructure messages unwrapped on their separate stream.
- [ ] Implement the approved three-thread ownership model with a SHARED media
      driver and foreground ClickHouse work.
- [ ] Persist and compare typed and dynamic books, and persist trades.
- [ ] Run deterministic captured-Bitget E2E through real Aeron IPC and Docker
      ClickHouse.
- [ ] Run and record the dated live Bitget BTCUSDT smoke test.
- [ ] Reach 100 percent line, function, region, and branch coverage for all new
      or changed handwritten production code without regressing workspace
      coverage.
- [ ] Pass all wire, allocation, compile-fail, runtime, integration, and
      documentation gates.
- [ ] Run five comparable warmed-up manual, fallible-helper, previous ErgoSBE,
      and Aeron benchmarks for every maintained case.

## Done means done

Do not close this todo for a compiling skeleton, mock transport, copied encode,
auto-skipped integration, one benchmark run, partial coverage, or unavailable
live tooling. Install or start authorised dependencies, record genuine external
blockers, and continue independent slices. Close only when every definition of
done item in the approved design passes in the same worktree.
