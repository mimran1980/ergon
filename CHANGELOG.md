# Changelog

## [Unreleased]

### Removed
- **`bulk_add` removed from group encoders.** Despite hoisting bounds checks
  outside the loop, benchmarks showed it is consistently 1.5-2× *slower* than
  `add_closure` and `add_struct` across primitive and Decimal types. The entry
  writer pattern (`add` with `&mut self` chainable methods) compiles to tighter
  code that LLVM optimises better. DTO encode now always uses the closure path.

### Added
- Benchmark fairness documentation (`sbe/benchmarks/README.md`) — mandatory
  checklist for every parity benchmark.
- `just test-all` guard against `#[ignore]` and `--skip` — prevents tests
  from being silently skipped.
- `group_encode_decimal_bench` — encode comparison with `rust_decimal::Decimal`
  converters.
- sbe-tool comparison arm in `group_encode_bench`.

### Fixed
- **Composite explicit-offset bug**: `get_token_block_size` summed child sizes
  ignoring `offset="N"` attributes on composite members. A composite with a
  field at `offset="8"` reported size 9 instead of 16, cascading into wrong
  `BLOCK_LENGTH` and `ENCODED_LENGTH` constants. Present since 0.1.0.
- **Double bounds checks in `add_struct`/`bulk_add`**: `self.buf[pos..][..N]`
  triggers two slice bounds checks; changed to `self.buf[pos..pos+N]` (one check).
- **Unfair `parity/encode/throughput_10k`**: ergon used body-only `wrap(buf,8)`
  while sbe-tool wrote headers via `header(0)` — fixed to header-inclusive comparison.
- **Wrong sbe-tool wrap offset in `parity/encode/scalar`**: `wrap(buf,0)` instead
  of `wrap(buf,8)` (`message_header_codec::ENCODED_LENGTH`), causing header overwrite.
- All parity benchmarks now assert byte-identical output and matching encoded lengths.
- Removed `--skip explicit_implicit` from `justfile` — the test now passes.

### Changed
- Benchmarks use `wrap_and_apply_header` (infallible) instead of
  `try_wrap_and_apply_header` — sbe-tool's `header()` does no validation,
  so ergon's validation was extra work.

### Notes
- **sbe-tool comparison ratios are unusually good (~0.4-0.5×).** The gap is
  attributed to sbe-tool's `Option<parent>` indirection on every field write
  and `advance()` overhead. Both arms produce byte-identical output with
  correct `black_box` usage. Review requested — if you spot a fairness issue,
  please report it.

## [0.1.3] — 2026-07-28

### Added
- `bulk_add(&[Entry])` and `bulk_decode() -> Vec<Entry>` for flat groups — 15-17% faster than per-entry loops
- `compute_length_with_header()` — single method name across fixed, flat, and complex messages
- `compute_length_with_header(params)` and `try_compute_length_with_header(params)` — short aliases
- DTO encode uses `bulk_add` automatically for flat groups without conversions
- `just test-all` — runs full suite + miri UB detection + fuzz corpus replay
- Fuzz targets: `bulk_decode`, `flat_group_decode`
- README: "Why ergo-sbe" standout features section
- README: DTO latency warnings — never use DTOs on the hot path

### Changed
- README: 0 `rust,ignore` fences — bare `rust` compiles, `text`/`xml`/`toml` for schematics
- All docs use consistent `compute_length_with_header` naming
- `Cargo.toml` documents fuzz/miri exclusion rationale
- Miri fixtures updated to current naming

### Fixed
- `rust_decimal` converter for constant-exponent Decimal composites
- `bulk_decode` primitive arrays use typed elements, not raw bytes
- Removed broken `TryFromSbe<u64>` clash example from timestamp section

## [0.1.1] — 2026-07-27

### Added
- Wire parity with sbe-tool across all example and unit schemas
- Comprehensive tests: composite layout, consuming stages, hostile input replay
- Java parity: Display/FromStr for enums/sets, field metadata, fixed array helpers
- Multi-schema versioning with shared types
- XML descriptions → rustdoc provenance
- Property-based round-trip tests

### Fixed
- SBE header always little-endian regardless of schema `byteOrder`

## [0.1.0] — 2026-07-26

### Added
- Initial release
- SBE XML parsing with XSD validation
- Rust codec generation: type-state encoders, flyweight decoders
- Zero-copy composite wire images
- Compile-time wire-order enforcement
- Exact buffer sizing: `ENCODED_LENGTH`, `EncodedLength` staged builder
- Domain objects (DTOs)
- `with_conversion` / `with_domain_type` converter seam
- Multi-template dispatch (`AnyMessage`, `FrameCursor`)
- Schema evolution (`sinceVersion`, `deprecated`)
- `build.rs` integration
