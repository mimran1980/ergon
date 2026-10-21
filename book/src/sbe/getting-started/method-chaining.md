# Method Chaining

ergo-sbe encoders are designed so **the entire encode reads as one expression**,
from `wrap_and_apply_header` through `.fixed(...)` and every dynamic tail,
ending in `.encoded_length_with_header()`. Bind only the resulting length; do not retain
intermediate encoder variables.

**Prefer (one chain, one `let`):**

```rust,no_run
{{#include ../../../examples/heartbeat-encode.rs:staged_chaining}}
```
*(From `book/examples/heartbeat-encode.rs` — compiled against the feature-tour codec.)*

### Staged chaining vs fixed-only

For a fixed-only message like `Heartbeat`, `wrap*` returns
`HeartbeatEncoder<'_, H, FieldsUnfixed>`. `.fixed(&HeartbeatFixedFields { … })`
consumes that value and returns `FieldsFixed`, which is the only phase that
exposes `as_bytes_with_header` / `as_body_bytes` / `encoded_length*` /
`into_remaining_mut`. Individual field setters stay
on the unfixed phase and on [`raw_fixed()`](encode-decode.md); they are not on
that complete view.

**Avoid (interrupted chain, rebinding):**

```rust,ignore
// Each `let` breaks the chain and splays the pipeline across the screen.
// The `.unwrap()` calls are a code smell — the fallible chain should use `?`.
let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0).fixed(&fields);
let enc = enc.fuel_figures(2, |g| { ... }).unwrap();
let enc = enc.manufacturer_as_str("Honda").unwrap();
let len = enc.encoded_length_with_header();
```

**Every encoder stage is chainable** — `fixed()` and each tail method return
the next stage (or `Result<NextStage, _>`) and compose with `?` in the same
expression. Intermediate encoder rebinding and manual `.unwrap()` defeat this
design.

For the full Car example with groups and var-data, see the
[feature tour](../feature-tour.md) page.

## Decoding

Sequential decode consumes each tail in wire order. For callbacks throughout
the chain, start with `decoder.ordered()`:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_ordered_lane}}
```

The fixed callback comes before tails and receives a fixed-fields-only view;
each group callback receives the entry and its `EntryInfo`, and var-data
callbacks receive bytes or strictly validated text. Every callback returns
`Result<_, DecodeError>`, so the closures infer. These
callbacks borrow the original wire buffer, so bytes and text may
be retained as the chain advances. The message chain ends on
`encoded_length_with_header()` / `as_bytes_with_header()`, like encode —
`.done()` is only to unwrap the staged complete. A nested `ordered()` walk
returns that completion to the parent callback directly. The
[L3 sample](../../samples/l3-book.md#ordered--one-spelling-all-the-way-down)
shows this recursive form.

The underlying staged API also supports explicit iteration and skipping. Its
group and var-data spellings can require iterator and tuple bindings:

| Group entries | `into_<group>` gives you | Why |
|---|---|---|
| Carry their own groups or var-data | a visit closure, returning the entry's completion | no stride — the completion *is* where the next entry starts |
| Fixed-stride (no tails of their own) | an iterator, `for e in &mut iter` | the next entry is `offset + block length`, so nothing has to be measured |

Var-data `into_<name>()` returns `(payload, next)`. Text fields also offer
`into_<name>_as_str()`, with strict validation and the same tuple shape.
These borrowed payloads can be retained while advancing later stages.

**Prefer:**

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_decode_stages}}
```

*(Real code from the `sbe-feature-tour` sample. `fuelFigures` and
`performanceFigures` entries carry tails, so they take closures;
`acceleration` is fixed-stride, so it is an iterator.)*

Consecutive var-data fields chain through the `next` stage each tuple
returns; the borrowed slices stay valid while later stages advance:

```rust,no_run
{{#include ../../../examples/car-decode-closures.rs:var_data_tuples}}
```

For callbacks instead of tuples, use the ordered lane's `<name>(|bytes| ..)`
and `<name>_as_str(|text| ..)`, which return `DecodeError`.

The generated staged walk advances once through dynamic tails. Random-access
getters used before that walk, including counts or lengths for later tails,
can add scans. See [Asking before walking](../feature-tour/decode-stages.md#asking-before-walking).
