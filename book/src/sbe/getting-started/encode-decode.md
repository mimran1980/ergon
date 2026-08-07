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
  let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
  let len = HeartbeatEncoder::wrap_and_apply_header(&mut buf, 0)
      .fixed(&HeartbeatFixedFields { seq: 7 })
      .encoded_length_with_header();
  let dec = HeartbeatDecoder::try_decode(&buf[..len], 0)?;
  assert_eq!(dec.seq(), 7);
```

## Optional fields and `apply_nulls`

`try_wrap_and_apply_header` / `wrap_and_apply_header` write the message header
only — they do **not** fill optional fixed fields with schema null sentinels
(sbe-tool parity). Unwritten optional bytes retain whatever was already in the
buffer (often zero, sometimes stale).

If you leave any optional field unset, call **`apply_nulls()`** after wrap so
every optional carries its schema null:

```rust,ignore
// Schematic — call after wrap_and_apply_header when any optional may be unset:
// enc.apply_nulls();
// then fixed(...) / required setters only
```

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
More recipes: [Recipes](../../recipes.md).
