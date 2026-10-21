# Type-state and runtime cost

Stage identity adds no runtime tag. The named structs carry buffer and cursor
state; markers such as `HeaderPresent` are zero-sized. A transition moves that
state into the next concrete type:

```rust,ignore
(buf, msg_offset, pos)  +  PhantomData / zero-sized stage identity
```

No heap, no vtable, no enum discriminant, no “which stage am I in?” branch —
the stage is the **type**, so the methods that exist are the ones legal at
that point. Inlining can remove the intermediate moves. Decoder stages also
carry acting version and block length; dynamic group callbacks check that the
returned completion belongs to the supplied entry.

The sbe-tool gate compares two different codecs, so it cannot isolate the
cost of type-state, and no benchmark here measures that cost on its own. A
measurable type-state tax would be a codegen defect. Gate and method:
[Benchmarks](../benchmarks.md).

Named stages (`CarEncoder` → `CarAfterFuelFigures` → `CarComplete`) **are**
the type-state pattern. The other spelling is `Encoder<'a, Stage>` with
phantom markers. This crate uses named stages; the two were not compared.

| Concern | Choice | Why |
|---------|--------|-----|
| Linear tail (groups / var-data) | **Named structs** per stage | Compile errors name the group you skipped (`expected CarAfterFuelFigures`); rustdoc stays scannable |
| Header vs body-only | **One** `H: HeaderState` on every stage | Avoids doubling the graph. Default `H = HeaderPresent` |
| Completeness of the fixed block | `F: FieldsState` on the root encoder | `as_bytes_with_header` exists only after `fixed(&FixedFields)` |

Tail stages drop `F` — they are already past the fixed block.

```rust,ignore
pub struct BookEncoder<'a, H: HeaderState = HeaderPresent, F: FieldsState = FieldsUnfixed> { /* … */ }
pub struct BookAfterBids<'a, H: HeaderState = HeaderPresent> { /* same layout */ }

impl BookEncoder<'a, H, FieldsFixed> {
    pub fn bids(self, …) -> Result<BookAfterBids<'a>, …> { … }
    // no asks() — bids first on the wire
}
```

Stage names: `After{GroupPascal}` (`fuelFigures` → `CarAfterFuelFigures`).
Reserved method names: `reserved_name_clash_test`. Product rationale:
[Wire order](../core-concepts/wire-order-stages.md). Migration:
[Coming from sbe-tool](../getting-started/from-sbe-tool.md).
