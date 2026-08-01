# Buffer Sizing Guide

Every protocol buffer must be sized using the generated EncodedLength API.
Never guess with `vec![0u8; 4096]`.

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
