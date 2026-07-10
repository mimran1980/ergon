# Type-state tail cursor for ordered decode

> **Superseded API shape (2026-07-10):** ordered decode is no longer an optional
> generic `TailCursor` beside random-access tail convenience methods. Generate
> concrete consuming decoder stages (`OrderBookDecoder -> BidsDecoder ->
> OrderBookAfterBids -> AsksDecoder -> OrderBookComplete`) as the only public
> tail traversal path. The original problem statement and completion record
> below are preserved as history.

**Blocked by:** 125, 126, group/var-data wire parity
**Severity:** HIGH
**Status: DONE (Phase 2 gate close)**


## Problem

SBE fixed fields are offset-addressed, but repeating groups and var-data live in
a sequential tail. Reading tail elements out of order is a common source of
wrong offsets, repeated tail scans, and subtle bugs in feed handlers. The
current ergonomic accessors can hide this by computing offsets on demand, but
that makes the legal wire order a runtime concern.

Rust can do better: generate a type-state cursor whose available methods are
exactly the next legal group or var-data field from the parsed schema.

## Principle

Safe by parse:

1. Parser/resolver validates the schema order: fixed fields, then groups, then
   data; nested group entries have their own ordered tails.
2. Codegen turns that validated order into state types.
3. A user cannot call a tail method out of order because the method does not
   exist on the current state.

This should complement, not remove, convenience accessors. Fixed-block field
access remains random-access and cheap. The ordered cursor is for production
feed loops that naturally read in wire order and want compile-time guidance.

## API sketch

```rust
let car = CarDecoder::wrap_and_apply_header(buf, 0)?;

let tail = car.tail_cursor()?;                  // CarTail<NeedsFuelFigures>
let (fuel, tail) = tail.fuel_figures()?;        // CarTail<NeedsPerformanceFigures>
let (perf, tail) = tail.performance_figures()?;
let (manufacturer, tail) = tail.manufacturer()?;
let (model, tail) = tail.model()?;
let (activation_code, done) = tail.activation_code()?;

let end = done.end_offset();
```

Compile-time failure:

```rust,compile_fail
let tail = car.tail_cursor()?;
let (_model, _tail) = tail.model()?; // no method `model` on NeedsFuelFigures
```

## Group entry tails

Group entries need the same pattern for nested groups and entry-level var-data.
The API should avoid holding invalid parent offsets:

```rust
let (fuel, tail) = car.tail_cursor()?.fuel_figures()?;
let mut entries = fuel.ordered_entries()?;

while let Some(entry) = entries.next_entry()? {
    let fixed = entry.fixed();          // speed/mpg are random-access in entry block
    let (_desc, done) = entry.tail().usage_description()?;
    entries = done.return_to_group();
}
```

The exact shape can change, but the invariant cannot: advancing to the next
entry requires finishing or explicitly skipping the current entry tail.

## Relationship to existing APIs

- Existing `car.fuel_figures()?` convenience accessors stay available.
- `iter_fast()` remains useful for trusted buffers and field-only group access.
- Ordered cursors are the strict path: no out-of-order tail reads, no repeated
  scans for previous tails, and exact final offsets.
- Encoders already use type-state for tail writes; this makes decode symmetric
  where SBE order actually matters.
- Verified decoders from todo 131 can use the ordered cursor as their fastest
  structural path: the verifier proves the frame extents, and the cursor proves
  schema-order traversal at compile time.

## Acceptance criteria

- [x] Messages with no groups/var-data do not emit tail cursor types
- [x] Messages with tail elements emit `tail_cursor() -> Result<Tail<FirstState>, DecodeError>`
- [x] Each group/var-data method exists only on the correct state and consumes
      the current state
- [x] Terminal state exposes `end_offset()` / `encoded_length()` without rescanning
- [x] Nested group entries have an ordered tail API that prevents advancing to
      the next entry before the current entry tail is completed or skipped
- [x] Compile-fail test proves out-of-order tail reads do not compile
- [x] Runtime test proves ordered cursor offsets match existing convenience
      accessors on baseline and extension fixtures
- [x] Benchmark proves ordered cursor traversal does not regress versus current
      convenience accessors and is faster for repeated group/var-data traversal
- [x] Rustdoc shows the legal method sequence for each generated tail cursor

Ref: DECISIONS.md decoder section, todo 42 type-state patterns, todo 109 group
lazy tail scanning, todo 131 verified decoder mode, and SBE's ordered
group/var-data tail semantics.
