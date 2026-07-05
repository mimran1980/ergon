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

For wire-shape or hot-path changes, also cover the relevant compatibility and
performance contract: byte-exact fixture parity, optional/null semantics,
versioned field absence, configured `headerType`/`dimensionType`, external
framing, and zero allocations in generated decode/encode paths.
