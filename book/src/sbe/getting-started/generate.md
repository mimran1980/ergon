# Generate in build.rs

One call parses the schema and generates the codec:
**[`generate`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.generate.html)**`(source, config)`.
Then write the result:

| Write with | Output | Use when |
|---|---|---|
| `.write_to_out_dir()` | Cargo's `$OUT_DIR`; include with `ergo_sbe::sbe_mod!(name)` or `include!` | Products: nothing generated lands in the source tree |
| `.write_to(dir)` | A directory you choose, e.g. a gitignored `src/generated/` | rust-analyzer go-to-definition into the generated code |

Both publish every module or none, print `cargo::rerun-if-changed` for the
schema file and every resolved include, and print generation warnings as
`cargo::warning`.

| Source | Means |
|---|---|
| `SchemaSource::file(path)` (or a `&Path` / `PathBuf`) | Read the file; `xi:include` resolves relative to it |
| `SchemaSource::xml(text)` | Inline XML, e.g. `include_str!`; nothing to watch, so add your own `rerun-if-changed` |

A bare `&str` does not compile as a source: say whether it is a path or XML.

The feature-tour sample writes to `src/generated/`:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/build.rs:build_rs_example}}
```

`generate` is
[`parse`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.parse.html) +
`Schema::from_ir` +
[`Generator::generate`](https://docs.rs/ergo-sbe/latest/ergo_sbe/struct.Generator.html).
Call those directly to inspect the token IR or to generate several schemas
that share types: see [Multi-Schema Patterns](multi-schema.md).
