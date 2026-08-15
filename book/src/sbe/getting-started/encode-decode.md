# Encode and Decode

Two styles — pick whichever fits:

**`fixed()` struct** (fill every field at once — compile error if a field is missing):

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_fixed_heartbeat}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

`{Msg}FixedFields` has **no** `Default`. Every required scalar must appear in
the struct literal (optional fields use `Option` only when schema
`presence="optional"`). That is intentional: zero-filling required IDs hides
bugs. For large messages, build the literal next to the encode call in wire
order — there is no staged builder in 0.1.13.

**Individual setters** (chainable, set only what you need):

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_fixed_heartbeat}}
```

## Optional fields and `apply_nulls`

`try_wrap_and_apply_header` / `wrap_and_apply_header` write the message header
only — they do **not** fill optional fixed fields with schema null sentinels
(sbe-tool parity). Unwritten optional bytes retain whatever was already in the
buffer (often zero, sometimes stale).

**`fixed()` closes that gap for you.** Optional fields are `Option<T>` in the
generated `FixedFields` struct, and `fixed()` writes the schema null wire image
for every `None` — including fixed arrays and nested optional composite
members. Since `fixed()` is the only route to a message's tails, the ordinary
path never leaves a stale optional behind:

```rust,ignore
// `price` is optional; None writes the schema null image, not stale bytes.
let len = QuoteEncoder::wrap_and_apply_header(&mut buf, 0)?
    .fixed(&QuoteFixedFields { symbol: *b"IBM     ", price: None })
    .encoded_length_with_header();
```

`apply_nulls()` remains for the `raw_fixed()` writer, where you set individual
fields yourself and no `FixedFields` value describes which optionals are unset.

See [Why NullVal Instead of Option](../design-notes/nullval.md).

**Character arrays:** fixed-width `char` fields become `[u8; N]`. Pass a shorter
`&str` via the `_str` setter — auto-padded with NULs. On decode, `copy_*`
copies the raw bytes into your buffer:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_fixed_heartbeat}}
```

The Car encoder's fixed fields include `vehicle_code: [u8; 6]` (schema `char` array)
and `some_numbers: [u32; 4]`. See the [feature tour](../feature-tour.md) for the
complete Car example with groups and var-data.

**Start here for a full runnable map of features:**
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
(`cargo run --manifest-path samples/sbe-feature-tour/Cargo.toml`).  
More recipes: [Recipes](../recipes.md).
