# ErgoSBE

Opinionated, idiomatic Rust code generation for Simple Binary Encoding.

ErgoSBE is an early-stage open source project for generating Rust codecs from
SBE schemas without copying the Java reference implementation's shape into
Rust. The project starts private while the initial direction settles, then will
move public once the first useful generator path is ready.

## Core Values

1. Wire compatible with the official SBE.
2. Idiomatic Rust, not Java translated into Rust.
3. Performance-first suitable for low-latency trading.

## Intended Shape

ErgoSBE will focus on:

- XML schema parsing into a small Rust-first intermediate representation.
- Generated encoders and decoders that preserve official SBE wire layout.
- Borrowed, low-allocation access patterns by default.
- Predictable code generation suitable for review, benchmarking, and audit.
- Benchmarks against realistic market-data style message layouts.

## Current Status

This repository is the initial project scaffold. The public API currently
contains configuration, schema metadata, and the generator boundary. SBE XML
parsing, IR validation, and full encoder/decoder emission are next.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
