# Changelog

## [0.1.17] — 2026-08-14

### Changed
- **Breaking:** writing a message's tail now requires the fixed block first.
  `wrap*` returns `{Msg}UnfixedEncoder`; groups and var-data are reachable only
  after `.fixed(&{Msg}FixedFields { … })`. "Tails before the fixed block" is now
  unrepresentable rather than a runtime hazard.
- Gate the fixed-stride group proof APIs (`add_checked`, `start_entry`,
  `complete`, `EntryComplete`) on groups that actually have a fixed stride. A
  dynamic entry with a var-data tail can no longer claim completion.
- `ClusterError` is `#[non_exhaustive]`, so adding a variant is not a breaking
  change for downstream matches.
- Decode errors keep their UTF-8 cause in the error chain instead of flattening it.
- Malformed XML and schema-invalid SBE now surface as distinct errors.

### Added
- Schema `description` text is emitted as rustdoc on messages, fields,
  composite members, enum variants, group entries, and var-data.
- `#[must_use]` on pure generated observers — getters, set predicates, raw
  enum/set values, and metadata queries — so discarding them is a warning.
- Multi-schema generation validates that same-named shared types have identical
  wire fingerprints, and rejects duplicate or non-identifier module names.
- Fail-closed benchmark evidence packaging (`scripts/package-bench-artifacts.sh`)
  wired into both the release workflow and `just release`.
- sbe-tool parity benchmarks for optional-enum nullification and
  group-with-data, both gated at the strict `1.00` ceiling.

### Fixed
- Optional primitive fields write the schema null image when set to `None`,
  including fixed arrays and nested optional composite members.
- Cluster egress decoding runs through one canonical fail-closed path; short,
  truncated, and invalid-text frames report the real error instead of a
  misleading timeout.
- `parse_event` reports the real template id for decoded-but-unprojected frames
  instead of `0`.
- Session builder timeouts reject zero, sub-millisecond, and overflow values,
  and each timeout field tracks its own error so one cannot mask another.
- `StaticCredentials` and `SessionBuilder` redact credential material from
  `Debug` output.
- Duplicate member IDs in an ingress endpoint map are rejected.
- Generated multi-paragraph rustdoc is emitted one line per `#[doc]`, so it is
  no longer parsed as an indented Markdown code block and compiled as a doctest.

## [0.1.16] — 2026-08-10

### Changed
- Decoder structs use `offset` field (matching sbe-tool naming), replacing internal `base_addr` cache. `read_addr_unchecked` helper removed.
- Generated encoder cursors renamed `offset` for consistency across all decoder/encoder types.

### Added
- Historic ergo regression benchmarks — null-as-option, optional fields, Decimal converters. Gate: `just bench-historic`, `scripts/check-bench-historic.sh`.
- Multi-message/framing documentation: `AnyMessage` dispatch, back-to-back and `remaining()` framing approaches.
- miette error diagnostics page with rendered examples of schema parse errors.

### Fixed
- toml 1.x `FromStr` compat: `toml::Value::from_str` replaced with `toml::from_str` for full-document parsing.
- Matrix test crate missing feature flags (`compact_str`, `smol_str`, `bytes`).
- Book: removed incorrect "Java-style" label from parent-hopping sections.

## [0.1.15] — 2026-08-09

### Added
- **`with_null_as_option(ConversionSelector::named_type("EventCode"))`** — enum fields become `Option<EventCode>`. `NullVal` wire byte → `None`.
- **`with_all_enums_as_option()`** — every enum in the schema → `Option<Enum>`.
- **`as_option()` on every generated enum** — `event_code.as_option()` → `Option<EventCode>`.
- **`nullValue` on `<enum>` elements** now parsed from XML. `<enum encodingType="uint8" nullValue="99">` → `NullVal = 99`.

### Changed
- Crate description: `"Simple Binary Encoding (SBE)..."` for crates.io visibility.
- Group-entry enum fields support `with_null_as_option`.

## [0.1.14] — 2026-08-08

### Breaking
- `SessionState::AwaitingNewLeaderConnection` removed (merged into `AwaitingNewLeader`).
- `ClusterError` now `#[non_exhaustive]`, gains `InvalidTimeout` and `PayloadTooLarge`.
- `AsyncClusterConnect::step()` returns `ConnectStep` (non-exhaustive, wildcard required).
- `GenerationConfig::profile(Lean)` no longer clears explicit conversions/domain-types. Use `.lean()` for a clean baseline.

### Added
- `#[must_use]` on `SessionBuilder`, `AsyncClusterConnect`, `ClusterClaim`.
- `ConnectStep` re-exported at crate root.
- `checked_deadline` helper — zero/overflow timeouts return `InvalidTimeout`.
- `PayloadTooLarge` error with `operation`, `requested`, `maximum` fields.
- Zero-alloc fragmented path via `offer_parts`.
- Feature-gated integrations: `compact_str`, `smol_str`, `bytes`, `chrono`.
- `DomainVarData::CompactStrings`, `SmolStrings`, `BytesCrate` variants.
- `lean()` constructor on `GenerationConfig`.
- Module name validation + path containment.
- `#[must_use]` on codec tail stages.
- FIX SBE conformance suite documented.

### Fixed
- Malformed egress frames (< 8 bytes) → `ProtocolError`, not silently unknown.
- Decode errors propagated on poll path.
- `checked_add` overflow in `offer()`.
- Stale channel parse errors cleared on re-set.
- Benchmark docs match gate: SBE zero tolerance, cluster 0.005.

## [0.1.13] — 2026-08-07

### Breaking
- `SbeMessage` is a sealed trait — only generated types can implement it.
- Staged `EncodedLength` types renamed for stage-just-completed convention.
- Group decoders no longer expose `wrap_with_parent`.
- Safe constructors prove fixed extent and panic if short; `unsafe *_unchecked` skips.
- `From<BooleanType> for bool` removed — use `as_bool()`.
- `EncodeError::BufferTooShort` carries `field`.
- `DomainConversionFailed` carries `reason`.

### Added
- `add_checked` — group entries proven complete at compile time.
- Group decoders poison themselves on malformed entry.
- `#{inline}` on `bulk_decode`, staged finish helpers, domain DTO methods.
- `Error::source` on `EncodeError::Decode` and `VerifyError::DecodeError`.
- `#[must_use]` on consuming stages and `EncodedLength` stages.

### Fixed
- Constant-presence fields no longer count toward readable fixed extent.
- Aeron `try_claim` recipe rewritten against real API.
- Bench batch-decode uses `wrap_unchecked` for equal work.
- Deduped reserved method-name lists.
