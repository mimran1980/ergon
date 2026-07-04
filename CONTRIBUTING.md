# Contributing

ErgoSBE is intentionally strict about wire compatibility and generated-code
quality. Contributions should preserve the three project values in the README.

Before opening changes, run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

When adding generator behavior, include tests that cover the schema input, the
normalized representation, and the generated Rust surface.
