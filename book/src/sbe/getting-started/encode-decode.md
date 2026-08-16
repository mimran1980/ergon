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
order.

On a **fixed-only** message (no groups or var-data), `as_bytes_with_header`,
`as_body_bytes`, `encoded_length*`, and `into_remaining_mut` exist only after
`fixed(&FixedFields)`. Calling them on the value returned by `wrap*` is a type
error — that is what stops a reused buffer from publishing leftover body bytes
or packing the next message over a stale body.

**Individual setters** stay on the unfixed encoder after `wrap*` and also on
`raw_fixed()` (body-relative offsets). Prefer `fixed(&FixedFields)` when you
have every field; use `raw_fixed()` when you want a dedicated writer:

```rust,no_run
{{#include ../../../examples/heartbeat-encode.rs:raw_fixed}}
```
*(From `book/examples/heartbeat-encode.rs` — compiled against the feature-tour codec.)*

`raw_fixed()` writes into the buffer you already sized. It does **not** mark
the message complete: omitted required setters leave stale bytes, and
`as_bytes_with_header` / `encoded_length*` / `into_remaining_mut` stay locked.
Set every required field, then call `fixed(&FixedFields)` for the
complete-message views. Slicing `&buf[..ENCODED_LENGTH]` yourself is possible
but is not the completeness-checked path.

## Optional fields and `apply_nulls`

`try_wrap_and_apply_header` / `wrap_and_apply_header` write the message header
only — they do **not** fill optional fixed fields with schema null sentinels
(sbe-tool parity). Unwritten optional bytes retain whatever was already in the
buffer (often zero, sometimes stale).

**`fixed()` closes that gap for you.** Optional fields are `Option<T>` in the
generated `FixedFields` struct, and `fixed()` writes the schema null wire image
for every `None` — including fixed arrays and nested optional composite
members. Since `fixed()` is the only route to a message's tails *and* to
fixed-only complete byte views, the ordinary path never leaves a stale
optional behind:

```rust,ignore
// `price` is optional; None writes the schema null image, not stale bytes.
let len = QuoteEncoder::wrap_and_apply_header(&mut buf, 0)?
    .fixed(&QuoteFixedFields { symbol: *b"IBM     ", price: None })
    .encoded_length_with_header();
```

`apply_nulls()` remains on the unfixed encoder after `wrap*`, for the case
where you set individual optional fields yourself and no `FixedFields` value
describes which optionals are unset.

See [Why NullVal Instead of Option](../design-notes/nullval.md).

**Character arrays:** fixed-width `char` fields become `[u8; N]`. Pass a shorter
`&str` via the `_str` setter — auto-padded with NULs. On decode, `copy_*`
copies the raw bytes into your buffer, or read the slice with `vehicle_code()`:

```rust,no_run
{{#include ../../../examples/fixed-char-arrays.rs:char_arrays}}
```
*(From `book/examples/fixed-char-arrays.rs` — compiled against the feature-tour codec.)*

The Car encoder's fixed fields include `vehicle_code: [u8; 6]` (schema `char` array)
and `some_numbers: [u32; 4]`. See the [feature tour](../feature-tour.md) for the
complete Car example with groups and var-data.

**Start here for a full runnable map of features:**
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
(`cargo run --manifest-path samples/sbe-feature-tour/Cargo.toml`).  
More recipes: [Recipes](../recipes.md).
