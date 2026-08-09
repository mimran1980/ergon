# Getting Started

From zero to working codec in under 5 minutes. You'll add `ergo-sbe` as a
build dependency, point it at a schema XML file, include the generated module,
and encode/decode your first message.

- [Depend on the Generator](getting-started/depend.md) — one line in `Cargo.toml`
- [Generate in build.rs](getting-started/generate.md) — point at your schema, get a Rust module
- [Include Generated Code](getting-started/include.md) — `include!` or `sbe_mod!` the output
- [Encode and Decode](getting-started/encode-decode.md) — fixed and variable-length messages
- [Method Chaining](getting-started/method-chaining.md) — write messages in one expression the way the schema reads
- [Multi-Schema Patterns](getting-started/multi-schema.md) — share types across schemas and versions
