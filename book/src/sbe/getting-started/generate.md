# Generate in build.rs

Two APIs — pick whichever fits your project:

**[`generate_to_dir`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.generate_to_dir.html)**
writes generated codecs to a directory you control (prefer `src/generated/` for
IDE go-to-definition). The feature-tour sample uses this pattern:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/build.rs:build_rs_example}}
```

**[`generate_to_out_dir`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.generate_to_out_dir.html)**
writes to Cargo's `$OUT_DIR` and emits `cargo::rerun-if-changed` automatically.
Use with `include!(concat!(env!("OUT_DIR"), …))` — simpler, but
rust-analyzer usually cannot jump into generated code.

Schema from a string / `include_str!`:
[`generate_str_to_out_dir`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.generate_str_to_out_dir.html).

Need multi-schema or custom output paths? Use the lower-level
[`parse_file`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.parse_file.html) +
[`Generator`](https://docs.rs/ergo-sbe/latest/ergo_sbe/struct.Generator.html)
API (same steps the helper runs). For shared types across schemas, see
[Multi-Schema Patterns](multi-schema.md) below.
