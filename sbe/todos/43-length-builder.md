# Type-safe encoded length builder

**Blocked by:** `03-group-vardata-wire-parity`

Calculating the encoded size of a message with repeating groups and var-data is
error-prone. Users must manually sum header + block + each group (dimension
header + N × entry size) + each var-data field (length prefix + data). Getting
it wrong means `BufferTooShort` at encode time — or worse, silent truncation.

The fix: generate a `LengthBuilder` that uses type-state to force the user to
specify every variable-length element. Cannot forget a group. Cannot forget a
var-data field. Cannot get the arithmetic wrong.
**Status: SUPERSEDED / PARKED FULL BUILDER**

**Decision after todo-coherence recheck (2026-07-08):** do not treat this
original full type-state builder as active v1 work. The best path is staged:
todo 116 covers simple exact `encoded_length(...)` helpers first; todo 118
covers the narrower hard case of nested groups and entry-level var-data. Bring
back this broad builder only if the simpler APIs cannot express real schemas
ergonomically.


## API shape

```rust
// Fixed message — no groups or var-data. Const, no builder needed.
let len: usize = Car::ENCODED_LENGTH;

// Message with groups + var-data. Type-state builder.
let len = Order::length_builder()
    .bids_count(12)            // 12 bid entries → adds 12 × BidsEntry::BLOCK_LENGTH
    .asks_count(8)             // 8 ask entries
    .symbol("AAPL")            // var-data: adds 4 + 4 = 8 bytes (length prefix + data)
    .build();                  // → usize, exact total

// Builder that skips an optional group:
let len = Order::length_builder()
    .bids_count(5)
    .skip_asks()               // explicitly skip optional asks group
    .symbol("AAPL")
    .build();
```

## Nested groups with per-entry var-data

When a group entry itself contains var-data, each entry can have a different
size. The builder lets you specify per-entry lengths:

```rust
let len = Order::length_builder()
    .legs_count(2)
        .leg_0(|e| e.leg_symbol("ES"))    // leg 0: 2 bytes symbol
        .leg_1(|e| e.leg_symbol("NQ"))    // leg 1: 2 bytes symbol
    .symbol("AAPL")
    .build();
```

For many entries with the same shape, use a template:

```rust
let len = Order::length_builder()
    .bids_count_with(100, |e| e)        // 100 identical entries
    .symbol("AAPL")
    .build();
```

## Type-state sequence

The builder walks the schema's group/var-data fields in wire order:

```
LengthBuilder<NeedsBids>
  → .bids_count(N)  → LengthBuilder<NeedsAsks>
  → .bids_count_with(N, |e| e) → LengthBuilder<NeedsAsks>
  → .skip_bids()    → LengthBuilder<NeedsAsks> (zero entries, but acknowledged)
  → .asks_count(N)  → LengthBuilder<NeedsSymbol>
  → .symbol("AAPL") → LengthBuilder<Complete>
  → .build()        → usize
```

Calling a method out of order is a compile error. Skipping a required field is
a compile error. `.build()` only exists on the terminal state.

## What the builder computes

```
total = header_size
      + block_length
      + Σ group_dimension_header_size + N × entry_block_length (per group)
      + Σ var_data_length_header_size + data.len() (per var-data field)
```

The header size, block length, group dimension header size, entry block length,
and var-data length header size are all schema constants — the builder just
multiplies and sums. No runtime work beyond addition.

## Acceptance criteria

- [x] `LengthBuilder<State>` generated per message with groups or var-data
- [x] One method per group: `foo_count(N)`, `foo_count_with(N, |e| e)`, `skip_foo()`
- [x] One method per var-data field: `foo(data)`, `foo_len(N)` (just length, no data)
- [x] Type-state enforces wire order — cannot skip ahead
- [x] `.build()` returns exact `usize` — no Result, computation is infallible
- [x] Fixed messages (no groups or var-data): no builder generated, just `ENCODED_LENGTH`
- [x] Optional groups can be skipped explicitly via `.skip_foo()`
- [x] Nested groups: per-entry var-data lengths via closure on `_with` methods
- [x] Generated doc: example showing a complete builder invocation
- [x] Test: builder output matches actual encoded message length
- [x] Test: builder output matches `encoded_length()` runtime method
