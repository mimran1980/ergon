# L3 Order Book

Deep nested / ragged L3 order-book sample for **ergo-sbe**. `publish = false`.

## Conversion style: `with_domain_type` only

```rust,no_run
{{#include ../../examples/conversion-config.rs:with_domain_type}}
```
*(From `book/examples/conversion-config.rs`. Full L3 `build.rs`: [l3-book](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/build.rs).)*

**Generated API (concrete):**

```rust,ignore
enc.try_price(rust_decimal::Decimal::new(100, 0))?;
let p: rust_decimal::Decimal = dec.try_price()?;
let ts: DateTime<Utc> = dec.try_exchange_timestamp()?;
```

`with_domain_type` **already enables conversion** for that selector. Do not also
call `with_conversion(Decimal)` here — it would not change the surface.

| Need | Use |
|------|-----|
| Concrete `price() -> Decimal` | **This sample** (`with_domain_type`) |
| Generic `price_as::<T>()` / app adapters | [Exchange Example](exchange-example.md) (`with_conversion`) |
| Side-by-side both styles | [SBE Feature Tour](sbe-feature-tour.md) |

## Encoding: size exactly, then write

Nothing here guesses a buffer size. The staged `L3BookEncodedLength` builder
walks the same shape the encoder will write — `bids_ragged` with a nested
`orders` count per level, then `asks_ragged`, then the var-data `symbol` —
and returns the exact header-inclusive length:

```rust,ignore
{{#include ../../../samples/l3-book/src/lib.rs:book_encoded_length}}
```

`_ragged` is the point for an order book: every level has a different number
of orders, so the count cannot be multiplied out. `g.add()?.orders(|og| …)`
declares one level and then that level's own order count.

The encoder then writes into a buffer of precisely that length, in wire order,
one chained expression:

```rust,ignore
{{#include ../../../samples/l3-book/src/lib.rs:encode_book}}
```

Both functions end with `encoded_length_with_header()`, and the tests assert
the two agree — a length that does not match the bytes written is a bug in the
sizing API, not something to paper over with a larger buffer.

```rust,ignore
let len = book_encoded_length(bids, asks, symbol)?;   // exact, header-inclusive
let mut storage = vec![0u8; len];                     // no oversize, no truncate
let actual = encode_book(&mut storage, bids, asks, symbol)?;
assert_eq!(len, actual);
```

See [Exact sizing](../sbe/feature-tour/exact-sizing.md) and
[Buffer sizing](../sbe/core-concepts/buffer-sizing.md).

## Decoding: four lanes over the same nested book

`L3Book` is the shape that makes the [decoder lanes](../sbe/feature-tour/decode-stages.md)
worth distinguishing: two sibling groups, each entry carrying a nested
`orders` group, then a trailing var-data `symbol`. Reaching `symbol` means
walking past every order of every level.

`tests/l3_tests.rs` encodes one ragged fixture and decodes it four ways,
asserting all four produce an identical snapshot. Note what the snapshot holds:
the schema's **domain** types (`rust_decimal::Decimal`, `DateTime<Utc>`,
`bool`) because that is what `with_domain_type` generates, and a **borrowed**
`&'a str` for `symbol` — copying it would defeat the flyweight.

```rust,ignore
{{#include ../../../samples/l3-book/tests/l3_tests.rs:lane_snapshot}}
```

```rust,ignore
{{#include ../../../samples/l3-book/tests/l3_tests.rs:lane_fixture}}
```

### Random access — any order, `Sync`, re-walks

```rust,ignore
{{#include ../../../samples/l3-book/tests/l3_tests.rs:decode_random_access}}
```

Note it reads `symbol` — the *last* tail — first, then goes back to the
groups. Only a lane that resolves each tail independently can do that. The
cost is that every dynamic-tail getter re-walks from the fixed block.

### Staged — wrong order is a compile error

```rust,ignore
{{#include ../../../samples/l3-book/tests/l3_tests.rs:decode_staged}}
```

`into_symbol` exists only on the stage reached after `asks` completes, so the
out-of-order read above is not expressible here. One wire-order pass.

### Memoized — any order, each boundary walked once

```rust,ignore
{{#include ../../../samples/l3-book/tests/l3_tests.rs:decode_memoized}}
```

Same getter names as random access. Read in the order shown — `symbol`, then
back to the groups, then `symbol` again — the base lane re-walks every bid and
ask order to return to `symbol`; this lane reuses the boundary it already
found. Build it **once** and pass `&L3BookMemoizedDecoder` around: calling
`.memoized()` in each function creates a separate empty cache.

### Mutable ordered — one cursor, runtime order checks

```rust,ignore
{{#include ../../../samples/l3-book/tests/l3_tests.rs:decode_ordered}}
```

Asking for `symbol` before the groups returns
`DecodeError::OutOfOrder` and leaves the cursor untouched, so the correct call
still works (`ordered_lane_rejects_out_of_order_tails` pins this). Nested
guards borrow their entry, so the borrow checker prevents touching a level
while its `orders` walk is live.

### On a hot path, collect nothing

The four functions above build owned `Vec`s because a test has to materialise
something to compare. Real consumption does not: the decoders are flyweights
over the wire buffer, `&str` and `&[u8]` borrow from it, and a full nested walk
needs no allocation at all.

```rust,ignore
{{#include ../../../samples/l3-book/tests/l3_tests.rs:decode_hot_path}}
```

`hot_path_walk_borrows_everything` asserts the returned `&str` points inside
the wire buffer rather than at a copy. Repository-wide, `allocation_count_test`
pins the same property for generated decode under a counting allocator.

### Which one

| You are doing | Lane |
|---------------|------|
| Reading a couple of fields, or one tail | random access — smallest, `Sync` |
| Decoding the whole book in wire order | `.ordered()`, or staged for compile-time enforcement |
| Several helpers reading multiple tails, same thread | `.memoized()` |

Full comparison, including how each differs from sbe-tool's single `limit`
cursor: [Decoder lanes](../sbe/feature-tour/decode-stages.md).

## Layout

| Path | Role |
|------|------|
| `schemas/l3-book.xml` | Nested bids/asks, orders, var-data tails |
| `build.rs` | `generate_to_dir` into `src/generated/` + domain objects / `with_domain_type` (**build-dep only**) |
| `src/lib.rs` | `#[path = "generated/l3_codec.rs"]` + EncodedLength helpers |
| `src/main.rs` | Runnable demos |
| `tests/l3_tests.rs` | Round-trips, exact-length proofs, and the four-lane decode comparison |

## Run

```sh
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
```
