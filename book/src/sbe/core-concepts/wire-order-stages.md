# Wire order via named stages

SBE is positional: groups and var-data appear in a fixed schema order with no
per-field tags. Two nearly identical groups back-to-back (bids then asks)
are common. Swap them and the bytes still look valid; prices land on the
wrong side of the book. That mistake is a type error here, not a
production incident.

sbe-tool uses `.parent()` to hand encoder ownership back up the tree. In Rust
that fights the borrow checker. ergo-sbe uses scoped closures so nested
schemas stay readable:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:encode_sample_car}}
```
*(From `sbe-feature-tour`; compiles and runs in CI.)*

Order is the type-state pattern, implemented as named concrete stage
structs (`CarAfterFuelFigures`), not `Encoder<State>`. The `H: HeaderState`
generic on every stage is a zero-sized marker for header-present vs body-only.
It does not track wire-order. Duplicating the stage graph for
`HeaderPresent` / `HeaderAbsent` would provide no latency advantage.

```rust,ignore
pub struct BookEncoder<'a> { /* buf, msg_offset, offset, … */ }
pub struct BookAfterBids<'a> { /* same layout */ }
pub struct BookAfterAsks<'a> { /* same layout */ }

impl BookEncoder<'a> {
    pub fn bids(self, …) -> Result<BookAfterBids<'a>, …> { … }
    // no asks(): bids come first on the wire
}
impl BookAfterBids<'a> {
    pub fn asks(self, …) -> Result<BookAfterAsks<'a>, …> { … }
}
```

<div class="ergon-anim" data-anim="wire-order-stages" data-title="Type-state encoder: wire order you cannot get wrong"></div>

*Animation, in words:* the sample `Car` above is 142 bytes. Wrapping the
buffer writes the 8-byte header, checks once that header and fixed block (53
bytes) fit, and sets `offset` to 53. The type is `CarEncoder` with
`F = FieldsUnfixed`, whose only way forward is `fixed()` (its field setters
return `&mut Self` and never change the type); calling
`fuel_figures` first fails with rustc's E0599, whose note says the method
exists on `CarEncoder<'a, H, FieldsFixed>`. `fixed()` writes the 45-byte
fixed block with no bounds checks and returns the same struct with
`F = FieldsFixed`. Calling `performance_figures` before `fuel_figures` also
fails with E0599, naming the type you hold. `fuel_figures(2, …)` writes a
4-byte group header and two entries (15 and 17 bytes); each `add` closure
must return `CarFuelFiguresEntryComplete`, which only
`usage_description_as_str` produces. A count check then moves the value into
`CarAfterFuelFigures`. `performance_figures` (21 bytes, with a nested group),
`manufacturer`, `model` and `activation_code` follow, each returning the next
type, ending in `CarComplete` at byte 142. Only `CarComplete` has
`as_bytes_with_header`. Every stage is the same 32 bytes on a 64-bit target
(slice pointer and length, `msg_offset`, `offset`, plus zero-sized markers):
no tag, no branch. The cost is one forward pass in wire order, with group
counts declared up front. Byte and check counts come from the generated code;
they are not timings.

After fixed fields you may only call the next group or var-data.
`BookEncoder` has no `asks` method. Decoders use the same idea
(`BookDecoder`, then `BookDecoderAfterBids`, and so on).

Group bodies use `|g| { g.add(|e| { … }) }`: the closure ends, then chaining
continues. See [Type-state](../design-notes/type-state.md) and
[Benchmarks](../benchmarks.md) for the cost claim and the `1.00×` gate.
