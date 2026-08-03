# Encode and Decode

Two styles — pick whichever fits:

**`fixed()` struct** (fill every field at once — compile error if a field is missing):

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_fixed_heartbeat}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

**Individual setters** (chainable, set only what you need):

```rust,no_run
  let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
  let len = HeartbeatEncoder::wrap_and_apply_header(&mut buf, 0)
      .fixed(&HeartbeatFixedFields { seq: 7 })
      .encoded_length_with_header();
  let dec = HeartbeatDecoder::try_decode(&buf[..len], 0)?;
  assert_eq!(dec.seq(), 7);
```

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
