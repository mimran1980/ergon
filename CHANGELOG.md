# Changelog

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
