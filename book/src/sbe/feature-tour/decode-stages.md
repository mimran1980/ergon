# Decoder Lanes

Every generated decoder exposes **three lanes**. They read the same wire and
return the same values; they differ in how order is enforced and what each
dynamic-tail access costs. The standard group `Iterator` remains for
compatibility and partial traversal. It is not a fourth message-decoding lane:
`Iterator::next()` must learn the next entry position before yielding a
dynamic entry, so it is not the ordered fast path.

| Lane | Entry point | Ordering | Dynamic-tail cost |
|------|-------------|----------|-------------------|
| Random access | Existing decoder getters | Any order | Recalculates preceding offsets |
| Staged | `into_*` and `visit_entries` | Compile time | One wire-order pass |
| Mutable ordered | `decoder.ordered()` | Runtime `OutOfOrder` | One wire-order pass plus order checks |

Fixed fields stay random-access in every lane. Groups and variable-data must
be consumed in schema order in the staged and mutable ordered lanes.

## Random access

Simplest for sparse or genuinely out-of-order reads. You can ask for
`manufacturer` before walking `fuelFigures`. By default every dynamic-tail
getter re-walks from the fixed block. With
[`with_memoized_tail_offsets(true)`](../configuration/generation-config.md#tail-offset-memoization)
the getters lazily memoize discovered boundaries on the decoder (`Cell`, so
the flyweight becomes `Send` and not `Sync`): the first access walks from the
frontier, later accesses reuse it. Construction and fixed-field reads stay
constant-time either way.

**Advantages**

- Any-order access; no stage types to thread through the call site
- Natural for “read two fields and stop”
- Same flyweight you already wrap with `try_decode` / `wrap`

**Disadvantages**

- Nothing stops you from reading tails twice or skipping a required walk
- Full-message decode of nested groups is the slowest of the three lanes
- With memoization on, the decoder is larger and, because the cache uses
  `Cell`, `Send` but not `Sync` — one instance per thread over shareable
  immutable bytes

Reading tails twice is the case memoization exists for: the second read
becomes a cache hit instead of a fresh walk, and repeated or reverse-order
root reads improve by an order of magnitude. What it does *not* buy you is a
single cold jump to the last tail — that pays to publish every boundary it
skips past, and is faster with the cache off. `just bench-diagnostics` runs
`versioned_l3_bench`, which measures both shapes.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_random_access}}
```

## Staged (`into_*` / `visit_entries`)

Maximum safety and the expected maximum-performance sequential path.
Ownership and generated stage types make a later tail unreachable until the
current one is consumed.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_decode_stages}}
```

Each `into_*_as_str()` returns `(&'a str, NextStage<'a>)` — the `&str` borrows
from the original wire buffer, not from the consumed stage. All three strings
remain valid simultaneously while the stage chain advances.

*(This code comes from the `sbe-feature-tour` sample crate.)*

### `#[must_use]` on stages

Consuming stages (`CarDecoderAfterFuelFigures`, `…AfterManufacturer`,
`CarDecoderComplete`, …) are `#[must_use]`. Dropping a stage without
`into_*` / `finish` / `skip_remaining` **silently skips** remaining wire
tails (groups and var-data). That is easy to miss when a function returns
early — prefer advancing until `Complete` or an explicit skip.

### `finish` vs `skip_remaining`

| Method | Meaning |
|--------|---------|
| `finish()` | Advance past any **remaining entries** of the current group and hand back the next named stage (or complete). |
| `skip_remaining()` | Explicit sequential spelling of the same idea — “I am done with this group; jump to the next tail.” |

Use `skip_remaining` when you want the intent obvious in review; both move the
tail cursor in wire order.

### Ordered one-pass `visit_entries`

The group decoder keeps the next message stage until the group is fully
consumed. `remaining_entries()` / `is_empty()` are O(1) observers of the
wire-declared count (`into_*` already read `numInGroup`). `visit_entries`
walks every remaining entry once and returns the next parent stage:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_visit_entries}}
```

Dynamic-entry callbacks return the generated completion stage so the next
cursor comes from the walk, not from a pre-scan of `encoded_length()`.
Fixed-stride callbacks return `Result<(), E>`. Empty groups invoke the
callback zero times.

**Advantages**

- Wrong order is a compile error (missing method on this stage)
- One pass; no offset rescan
- Expected fastest sequential decode; maintained benches require it ≤ sbe-tool
  and ≤ iterator decode

**Disadvantages**

- Stage types appear in signatures; you cannot hold “the decoder” and pick
  tails later
- Skipping a tail still requires an explicit `finish` / `skip_remaining`
- Partial group walks use the `Iterator`, which is not the one-pass path

## Mutable ordered (`ordered()`)

The ergonomic sequential choice: one `&mut` cursor, schema-order tails, runtime
`OutOfOrder` if you jump ahead. Fixed fields stay random-access. Group methods
return a guard that borrows the parent until `visit_entries`, `finish`, or
`skip_remaining` consumes it.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_mutable_ordered}}
```

A wrong dynamic-field call returns `DecodeError::OutOfOrder { owner, expected, requested }`
and leaves the cursor unchanged, so the correct method can still be called.
Calling an already-consumed field reports the next expected name; after
completion `expected` is `"<complete>"`. `finish(self)` skips any unconsumed
suffix and returns the existing complete stage.

Group guards own a local cursor and commit the parent offset only after a
successful completion. Dropping the guard, malformed data, or a callback error
leaves the parent at the group start (retry from the beginning of that group).
`remaining_entries()` is O(1). The guard does not implement `Iterator`. Nested
group guards borrow their entry, so Rust prevents using the parent entry until
the nested guard completes. Unread suffix of a dynamic entry is skipped once
on successful callback return — you cannot omit an earlier tail and then
request a later one.

**Advantages**

- One mutable value instead of a chain of stage types
- Same one-pass walk as staged; order mistakes are `Result` errors, not
  silent rescans
- Dropped guards are retryable; operations are transactional (commit after
  success, including UTF-8 / nested-message validation)

**Disadvantages**

- Order is checked at runtime, not by the type system
- A live group guard borrows the parent, so you cannot interleave parent
  access until the guard is consumed
- Slightly more work than staged (predictable ordinal checks)

## Full-frame bytes mid-walk

| Need | API |
|------|-----|
| Full frame after finishing the walk | complete stage `as_bytes_with_header()` |
| Full frame without consuming stages | inherent `dec.as_bytes_with_header()?` (rescans tails) |
| Fixed block only (not a full frame) | `dec.get_metadata().as_fixed_region_with_header()?` |

See [Generated code](generated-code.md#metadata-limits-tailed-messages) for the
metadata `limit` vs full-frame table.
