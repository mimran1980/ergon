# Wire Order via Named Stages

SBE is positional: groups and var-data appear in a fixed schema order with no
per-field tags. Two nearly identical groups back-to-back — **bids then asks** —
are common. Swap them and the bytes still look valid; prices land on the
wrong side of the book. That mistake is a **type error** here, not a
production incident.

sbe-tool uses `.parent()` to hand encoder ownership back up the tree. In Rust
that fights the borrow checker. ergo-sbe uses **scoped closures** so nested
schemas stay readable:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:encode_sample_car}}
```
*(From `sbe-feature-tour` — compiles and runs in CI.)*

Order is the type-state pattern, implemented as **named concrete stage
structs** (`CarAfterFuelFigures`), not `Encoder<State>`. The `H: HeaderState`
generic on every stage is a zero-sized marker for header-present vs body-only
— it does not track wire-order. Duplicating the stage graph for
`HeaderPresent` / `HeaderAbsent` would provide no latency advantage.

```rust,ignore
pub struct BookEncoder<'a> { /* buf, pos, … */ }
pub struct BookAfterBids<'a> { /* same layout */ }
pub struct BookAfterAsks<'a> { /* same layout */ }

impl BookEncoder<'a> {
    pub fn bids(self, …) -> Result<BookAfterBids<'a>, …> { … }
    // no asks() — bids first on the wire
}
impl BookAfterBids<'a> {
    pub fn asks(self, …) -> Result<BookAfterAsks<'a>, …> { … }
}
```

After fixed fields you may only call the **next** group or var-data.
`BookEncoder` has no `asks` method. Decoders use the same idea
(`BookDecoder` → `BookDecoderAfterBids` → …).

Group bodies use `|g| { g.add(|e| { … }) }`: the closure ends, then chaining
continues. See [Type-state](../design-notes/type-state.md) and
[Benchmarks](../benchmarks.md) for the zero-cost claim and the `1.00×` gate.
