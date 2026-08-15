# Wire Order via Named Stages

### Why not parent hopping?

sbe-tool's flyweight API uses `.parent()` to hand ownership back up the tree.
In Rust that pattern hits the **borrow checker**: move a group encoder in, get
stuck returning the parent, lose the thread of the code.

ergo-sbe leans on **scoped closures** and **chaining** so nested schemas stay
readable and you rarely pass encoder ownership field-to-field by hand:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:encode_sample_car}}
```
*(Real code from the `sbe-feature-tour` sample — compiles and runs in CI.)*

### Wire order via **named stage structs**

SBE is a **positional** wire format: groups and var-data appear in a fixed
schema order with no per-field tags on the wire. That matters a lot in
financial markets, where it is common to have **two nearly identical repeating
groups** back-to-back — e.g. **bids then asks** (same entry layout, different
meaning). If you encode or decode them in the wrong order, the bytes still look
like a valid message: prices and sizes land in the opposite book side. You only
discover the disaster at **runtime** (wrong trades, inverted books, silent
corruption). Compile-time order exists so that mistake becomes a **type error**
while you still have the schema in front of you, not a production incident.

Order is enforced with the **same idea** as the classic type-state pattern
(`Encoder<State>` / `PhantomData`), but **not** that implementation.

Each wire-order transition returns a **named concrete stage struct** (e.g.
`CarAfterFuelFigures`). The `H: HeaderState` generic on every stage is a
zero-sized orthogonal marker for header-present vs body-only mode — it tracks
header capability, not wire-order progression. Duplicating the stage graph
for `HeaderPresent` and `HeaderAbsent` would provide no latency advantage.

All maintained SBE parity comparisons pass at or below the `1.00×` ceiling
under both LTO-on and LTO-off profiles. Current results are in
[Benchmarks](../benchmarks.md); see the methodology page for reproduction.

Generated code emits **separate types** for each stage, same fields, different
methods:

```rust,ignore
// Approximate generated shape — not Encoder<AfterBids>:
pub struct BookEncoder<'a> { /* buf, pos, … */ }
pub struct BookAfterBids<'a> { /* same layout */ }
pub struct BookAfterAsks<'a> { /* same layout */ }
// …

impl BookEncoder<'a> {
    pub fn bids(self, …) -> Result<BookAfterBids<'a>, …> { … }
    // no asks() here — bids first on the wire
}
impl BookAfterBids<'a> {
    pub fn asks(self, …) -> Result<BookAfterAsks<'a>, …> { … }
    // no bids() here — already done
}
```

So after fixed fields you may only call the **next** group/var-data in schema
order. Calling `asks` before `bids` is a **type error** (`BookEncoder` has no
`asks` method). Decoders use the same idea: consuming stages
(`BookDecoder` → `BookDecoderAfterBids` → …).

Group bodies use **`|g| { g.add(|e| { … }) }`** so the outer encoder is not left
half-borrowed while you fill nested levels — the closure ends, then chaining
continues. That is intentional **API ergonomics for Rust** (avoids `.parent()`
style ownership hand-offs that fight the borrow checker on deep books).
