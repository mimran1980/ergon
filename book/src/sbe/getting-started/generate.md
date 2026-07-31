# Generate in build.rs

[`generate_to_out_dir`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.generate_to_out_dir.html)
parses the schema file, generates codecs, writes `$OUT_DIR/{module}.rs`, and
emits `cargo::rerun-if-changed` for you:

```text
{{#include ../../../../samples/sbe-feature-tour/build.rs:build_rs_example}}
```

Schema from a string / `include_str!`:
[`generate_str_to_out_dir`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.generate_str_to_out_dir.html)
(add your own `cargo::rerun-if-changed` for the file you included).

Need multi-schema or custom output paths? Use the lower-level
[`parse_file`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.parse_file.html) +
[`Generator`](https://docs.rs/ergo-sbe/latest/ergo_sbe/struct.Generator.html)
API (same steps the helper runs). For shared types across schemas, see
[Multi-Schema Patterns](multi-schema.md) below.
