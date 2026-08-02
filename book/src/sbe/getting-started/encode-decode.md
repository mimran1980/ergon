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
  let len = HeartbeatEncoder::wrap_and_apply_header(&mut buf, 0)?
      .fixed(&HeartbeatFixedFields { seq: 7 })
      .encoded_length_with_header();
  let dec = HeartbeatDecoder::try_from(&buf[..len])?;
  assert_eq!(dec.seq(), 7);
```

**C-style fixed-width strings:** pass a shorter `&str` — auto-padded with NULs.
On decode, `copy_*` copies the raw bytes into your buffer:

```rust,no_run
  let mut buf = [0u8; FixedStringEncoder::compute_length_with_header()];
  let len = FixedStringEncoder::wrap_and_apply_header(&mut buf, 0)?
      .code_str("ABC")?
      .encoded_length_with_header();
  let dec = FixedStringDecoder::try_from(&buf[..len])?;
  let mut code = [0u8; 6];
  dec.copy_code(&mut code);
  assert_eq!(&code, b"ABC\0\0\0");
```

**Start here for a full runnable map of features:**
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
(`cargo run --manifest-path samples/sbe-feature-tour/Cargo.toml`).  
More recipes: [Recipes](../../recipes.md).
