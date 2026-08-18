# Type-state is zero-cost (and the hybrid design)

## The question evaluators often ask

> Did compile-time wire-order enforcement cost anything on the hot path?

**No.** Named stage structs and marker generics are zero-sized compile-time
constructs. Every transition is a **move** of the same three runtime fields:

```rust,ignore
(buf, msg_offset, pos)  +  PhantomData / zero-sized stage identity
```

There is no heap allocation, no vtable, no enum discriminant on the wire path,
and no extra branch for “which stage am I in?” — the stage is in the **type**,
so the methods that exist are exactly the ones legal at that point in the
schema. Generated machine code is identical in shape to a single-struct
encoder with the same field writes.

Benchmarks that show “no difference vs a single struct / vs sbe-tool at the
1.00 ceiling” are therefore the **expected proof** that the abstraction is
zero-cost — not a lucky accident and not a reason to doubt the design. If a
type-state transition ever showed up as a measurable cost under a fair,
amplified, dual-LTO comparison, that would be a codegen defect.

All maintained SBE parity scenarios pass at or below the strict `1.00×`
sbe-tool ceiling under **both** LTO-on and LTO-off profiles. Methodology and
ceilings: [Benchmarks](../benchmarks.md).

## Type-state = multiple named structs

“Type-state” and “multiple different structs” are not alternatives. Named
stages (`CarEncoder` → `CarAfterFuelFigures` → … → `CarComplete`) **are** the
type-state pattern. The other spelling is a single generic
`Encoder<'a, Stage>` with phantom stage markers. Both compile the same way;
only the API surface differs.

## Why the hybrid (named stages + one header marker)

| Concern | Choice | Why |
|---------|--------|-----|
| Linear tail (groups / var-data in wire order) | **Named structs** per stage | Best compile errors (`expected CarAfterFuelFigures, found CarEncoder` names the group you skipped); best rustdoc; scannable API surface |
| Header present vs body-only mode | **One** `H: HeaderState` marker on every stage | Avoids doubling the entire stage graph (`CarAfterX` × Present/Absent). Orthogonal to wire order. Default `H = HeaderPresent` so the common case needs no turbofish |
| Default inference | `HeaderPresent` | Matches “encode a full frame” as the usual path; body-only is explicit via `wrap` / `HeaderAbsent` |

Duplicating every stage for header mode would provide **no** latency advantage
and would double the generated type count.

## What users see

```rust,ignore
// Approximate generated shape — not Encoder<AfterBids>:
pub struct BookEncoder<'a, H: HeaderState = HeaderPresent, F: FieldsState = FieldsUnfixed> {
    /* buf, msg_offset, pos + ZST markers */
}
pub struct BookAfterBids<'a, H: HeaderState = HeaderPresent> { /* same layout */ }
pub struct BookAfterAsks<'a, H: HeaderState = HeaderPresent> { /* same layout */ }

impl BookEncoder<'a, H, FieldsFixed> {
    pub fn bids(self, …) -> Result<BookAfterBids<'a>, …> { … }
    // no asks() — bids first on the wire
}
```

`F` is why `wrap*` cannot publish `as_bytes_with_header` until `fixed(&FixedFields)`
has written the required body. Tail stages drop `F` — they are already past the
fixed block.

Calling stages out of order is a type error. See
[Wire order via named stages](../core-concepts/wire-order-stages.md) for the
product rationale (bids/asks inversion) and
[Coming from sbe-tool](../getting-started/from-sbe-tool.md) for the migration
mapping (`.parent()` hopscotch → closures + stages).

## API freeze note

Stage names use `After{GroupPascal}` (e.g. `fuelFigures` →
`CarAfterFuelFigures`). Multi-word group names are PascalCased the same way
as other generated types. Reserved method names on decoder/encoder stages are
covered by `reserved_name_clash_test` so field collisions rename accessors
without shadowing stage transition methods.
