# Changelog

## [Unreleased]

### Added
- Benchmark fairness documentation (`sbe/benchmarks/README.md`) — mandatory
  checklist for every parity benchmark.
- LTO-on and LTO-off group benchmark matrix. This exposed an ergon inlining
  defect that the previous LTO-only results hid.
- `just test-all` guard against `#[ignore]` and `--skip` — prevents tests
  from being silently skipped.
- `group_encode_decimal_bench` — encode comparison with `rust_decimal::Decimal`
  converters.
- sbe-tool comparison arm in `group_encode_bench`.
- `bulk_add(&[Entry])` regression coverage for exact bytes, count overflow,
  short buffers, and empty groups.
- `bulk_add_domain(&[EntryDomain])` for eligible flat DTO groups, sharing the
  wire bulk writer's single-region validation without a temporary allocation.

### Fixed
- **Composite explicit-offset bug**: `get_token_block_size` summed child sizes
  ignoring `offset="N"` attributes on composite members. A composite with a
  field at `offset="8"` reported size 9 instead of 16, cascading into wrong
  `BLOCK_LENGTH` and `ENCODED_LENGTH` constants. Present since 0.1.0.
- **`bulk_add` code generation**: validate one exact output region, iterate it
  with `chunks_exact_mut`, use slot-relative field writes, and commit position
  once. On the audited 1,000-entry cases it is now about 22-23% lower latency
  than `add_closure` instead of 1.5-2× slower.
- **DTO flat-group encoding**: automatic DTO re-encode now actually selects
  the allocation-free domain bulk writer for wire-compatible flat entries.
  The previous generator emitted `to_wire_entry()` but still encoded every DTO
  entry through `add`. At 1,000 primitive entries, the corrected path measured
  509 ns versus 1.336 µs for the old path with LTO, and 509 ns versus 1.998 µs
  without LTO. DTO range validation remains enabled.
- **Zero-width group bulk encode**: count-only groups no longer call
  `chunks_exact_mut(0)` and panic.
- **Cross-crate generated-code inlining**: fixed/composite/set/enum setters,
  stage transitions, group iterators, entry writers, var-data methods,
  encoded-length builders, `add`, `add_struct`, `bulk_add`, and built-in
  conversion methods now carry inline intent. Before the fix, ergon's closure
  path changed from about 445 ns with LTO to 2.093 µs without LTO, and complete
  no-LTO encode/decode also lost to sbe-tool. sbe-tool remained healthy in both
  profiles.
- **Composite flyweight accessors**: generated composite decoders use trusted
  fixed-region reads after their enclosing message bounds have been
  established, removing redundant slice checks.
- **Wrong sbe-tool direct-decode offset**: parity decoders were wrapped at the
  message-header offset and read header bytes as body fields. Direct decoders
  now start at `message_offset + message_header_codec::ENCODED_LENGTH`.
- **Constant-foldable decode fixtures**: inputs or decoder references are now
  black-boxed before access with `std::hint::black_box`, and encoded output
  ranges are observed after writes.
- **Sub-nanosecond benchmark instability**: scalar, array, composite, entry,
  scalar-encode, and complete wire-encode cases repeat 1,024 equivalent
  operations per Criterion iteration.
- **Benchmark estimator mismatch**: the ratio gate now uses Criterion's
  displayed regression estimate. The previous gate read the raw sample median,
  which could disagree enough under noise to reverse a tiny ratio.
- **Asymmetric full-decode setup**: both arms now use precomputed header fields
  and construct direct body wrappers inside the timed path.
- **Incomplete “full message” decode**: the case now reads every encoded
  fixed/composite member before traversing all groups and var-data. The prior
  arms did equal work, but measured only the dynamic tail.
- **Scalar header accounting**: header-only, body-only, and header-plus-body
  encode are separate cases. Ergon body-only uses `wrap(buf, 0)` while
  sbe-tool body-only uses `wrap(buf, 8)`.
- **Timed Decimal construction**: Decimal benchmark inputs are prebuilt outside
  the timed path.
- Cluster examples and `SessionBuilder` rustdoc now compile against the current
  public API under the all-features/all-targets verification lane.
- Egress polling no longer aborts before receiving `NewLeaderEvent` when an
  automatic keep-alive encounters retryable Aeron backpressure or a temporarily
  disconnected ingress publication during failover.
- All parity encode cases assert byte-identical output and matching encoded
  lengths; decode cases assert all fixed, group, nested-group, and var-data
  values before timing.
- Removed `--skip explicit_implicit` from `justfile` — the test now passes.

### Changed
- Benchmarks use `wrap_and_apply_header` (infallible) instead of
  `try_wrap_and_apply_header` — sbe-tool's `header()` does no validation,
  so ergon's validation was extra work.
- Every maintained ergon/sbe-tool benchmark now has a strict `1.00` ceiling
  under both LTO and no LTO. A repeatable sbe-tool win blocks the change until
  the benchmark or generated hot path is fixed.

## [0.1.3] — 2026-07-28

### Added
- `bulk_add(&[Entry])` and `bulk_decode() -> Vec<Entry>` for flat groups
- `compute_length_with_header()` — single method name across fixed, flat, and complex messages
- `compute_length_with_header(params)` and `try_compute_length_with_header(params)` — short aliases
- `to_wire_entry()` helper for wire-compatible flat DTO group entries
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
