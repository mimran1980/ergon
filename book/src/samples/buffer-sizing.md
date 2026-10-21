# Buffer Sizing Guide

Size buffers with the generated length APIs (`compute_length_with_header()`,
the staged `*EncodedLength` builders). A `MAX_ENCODED_LENGTH` stack array is
fine when that constant is a true fixed upper bound — check first: it counts
one entry per group and only var-data that declares `maxLength`, so multi-entry
groups need the staged builder. Avoid guess constants such as
`vec![0u8; 4096]`: they hide size bugs.

**Const-sized messages** — stack array, no heap:

```rust,no_run
  let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
```

**Dynamic / ragged messages** — compute exact size first with `*EncodedLength`,
then encode into a claim or slot of that exact length:

```rust,no_run
{{#include ../../../samples/sbe-feature-tour/src/lib.rs:demo_car_size_and_encode}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

Key rules:

- `compute_length_with_header()` is `const` when the message has no var-data
  fields or groups — use it directly for stack array sizes.
- For messages with groups or var-data, use the staged `*EncodedLength` builder.
- Assert computed length equals actual encoded length after writing.
- Oversize `vec![0u8; 4096]` "guess" buffers hide size bugs — avoid them.
