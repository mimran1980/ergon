# ergo-sbe

[![Crates.io](https://img.shields.io/crates/v/ergo-sbe)](https://crates.io/crates/ergo-sbe)
[![CI](https://github.com/mimran1980/ergon/actions/workflows/ci.yml/badge.svg)](https://github.com/mimran1980/ergon/actions/workflows/ci.yml)
[![API Docs](https://docs.rs/ergo-sbe/badge.svg)](https://docs.rs/ergo-sbe/)

`ergo-sbe` generates **binary-compatible** Rust SBE codecs with compile-time
wire-order enforcement, closure-based groups, exact buffer sizing, and zero
heap allocation on hot paths.

> **AI assistance.** Large parts of this project were written **with heavy AI
> assistance**. Humans directed the work, approved designs, and ran verification.
> Details: [AI-ASSISTANCE.md](https://github.com/mimran1980/ergon/blob/main/AI-ASSISTANCE.md).

## 📖 Full Documentation

**[The Ergon Book](https://mimran1980.github.io/ergon/)** is the comprehensive
guide for ergo-sbe, covering:

- [Getting Started](https://mimran1980.github.io/ergon/sbe/getting-started.html) — depend, generate, encode, decode
- [Feature Tour](https://mimran1980.github.io/ergon/sbe/feature-tour.html) — exact sizing, bulk arrays, decode stages, DTOs, trust boundaries
- [Core Concepts](https://mimran1980.github.io/ergon/sbe/core-concepts.html) — wire order, buffer sizing, composites, flyweight vs struct
- [Configuration](https://mimran1980.github.io/ergon/sbe/configuration.html) — `with_conversion` vs `with_domain_type`, hooks
- [Recipes](https://mimran1980.github.io/ergon/sbe/recipes.html) — Display/Debug, schema→rustdoc, domain DTOs, timestamps
- [Benchmarks](https://mimran1980.github.io/ergon/sbe/benchmarks.html) — parity methodology and gates

## API Reference

[docs.rs/ergo-sbe](https://docs.rs/ergo-sbe/) — generated Rustdoc for the published crate.

## License

Apache-2.0.
