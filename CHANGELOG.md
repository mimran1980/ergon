# Changelog

## [0.1.14] — 2026-08-08

### Breaking
- **`SessionState::AwaitingNewLeaderConnection` variant removed.** The
  separate reconnection-pending state was merged into `AwaitingNewLeader`.
  Matches on `SessionState` that reference the removed variant need a
  wildcard arm — the enum was already `#[non_exhaustive]`.
- **`ClusterError` changes.** `AeronErrorSource` is now `pub` (was
  `pub(crate)`) with a private inner value and an `as_aeron_error()` accessor.
  `ClusterError` gains `InvalidTimeout` and `PayloadTooLarge` variants. The
  enum remains `#[non_exhaustive]`, so exhaustive matches must include a
  wildcard.
- **`AsyncClusterConnect::step()` returns `ConnectStep`.** The return type
  changed from an opaque state to the named `#[non_exhaustive]` enum. Callers
  that previously inspected internal fields must now match on `ConnectStep`
  variants with a wildcard arm for forward compatibility.
- **`GenerationConfig::profile(Lean)` no longer clears explicit
  domain/conversion settings.** Settings set before `profile()` are preserved.
  Use `GenerationConfig::lean(module_name)` for a clean Lean baseline.

### Added
- **`#[must_use]` on `SessionBuilder`, `AsyncClusterConnect`, `ClusterClaim`.**
  Discarding any of these values under `#![deny(unused_must_use)]` produces a
  compile error with a descriptive message. Normal chaining and completion
  remain warning-free.
- **`ConnectStep` re-exported at crate root.** Import as
  `ergo_aeron_cluster::ConnectStep`. The enum is `#[non_exhaustive]` with
  variants `CreateTransport`, `SendConnect`, `PollResponse`, `Done`.
- **`GenerationConfig: Clone`.** Clone a base config and override individual
  settings per schema with `with_module_name`.
- **`GenerateError::InvalidGeneratedSource`.** Carries the failing module name
  and syntax error, replacing the previous generic fallback.
- **`with_module_name`** on `GenerationConfig` for per-schema module naming.
- **Single-decode egress dispatch.** Known frames are decoded exactly once;
  malformed frames (including frames shorter than the 8-byte SBE header)
  produce `ClusterError::ProtocolError` instead of being silently classified
  as unknown.
- **Large-offer fragmentation fallback** with `offer_parts` gathering (zero
  allocation on the fragmented path).
- **`AeronErrorSource` public** with `as_aeron_error()` accessor, re-exported
  from the crate root for `Error::source()` downcasting.
- **Strict documentation gates.** `check-generated-docs` uses syn-based
  parsing; `check-book-fences.sh` enforces an allowlist. Both run in
  `check-products` and `release-check`.

### Fixed
- **Parser source-name diagnostics.** Schema parse errors now report the
  failing file name and line number.
- **Fail-closed generation.** Module name validation rejects path separators,
  `..`, absolute paths, and empty keyword suffixes. Generated output paths are
  contained within the declared output directory.
- **Generated consumers warning-free.** Removed blanket `allow(unused,
  dead_code)` from consumer tests; emitters omit unused locals, unnecessary
  `mut`, and spurious `unsafe` blocks for schema shapes that don't need them.
- **Timeout precision.** `SessionBuilder` setters now accept and store
  `Duration` directly; sub-millisecond values are preserved and
  `Instant::checked_add` failures return a typed error instead of panicking.
- **Cluster rustdoc.** All manual public items in `ergo-aeron-cluster` are
  documented; `#[allow(missing_docs)]` is confined to the unstable generated
  codec seam.
- **`#[must_use]` on session lifecycle types.** `SessionBuilder`,
  `AsyncClusterConnect`, and `ClusterClaim` detect discarded values at compile
  time.

## [0.1.13] — 2026-08-07

### Breaking
- **`SbeMessage` is a sealed trait.** Only types emitted by
  `ergo_sbe::Generator` can implement it — the supertrait lives in a private
  generated module. Hand-rolled `impl SbeMessage for MyType` no longer compiles;
  use the generated decoder/encoder types.
- **Staged `EncodedLength` types are named for the stage just completed**, not
  the next one, matching the encoder's `{Msg}After{Element}` convention.
  `CarEncodedLength::fuel_figures_ragged(…)` now returns
  `CarEncodedLengthAfterFuelFigures` (was `…AfterPerformanceFigures`). The
  chain and the type count are unchanged; only the names shift by one. These
  types are `#[doc(hidden)]` and reached by chaining, so code that never names
  a stage explicitly is unaffected.
- **Generated group decoders no longer expose `wrap_with_parent`.** It was an
  `unsafe` constructor that only the generated tail stages can call correctly;
  it is now private. Use `wrap` / the group accessor on the parent stage.
- **Safe constructors prove fixed extent.** Bare `wrap` / `wrap_and_apply_header`
  / `decode` run the same header+fixed-body proof as `try_*` and **panic** if
  short. Field accessors remain unchecked after that proof. Only
  `unsafe fn *_unchecked` skips the proof (UB on OOB). `AnyMessage::decode`
  matches `try_decode` (no longer uses unchecked header reads in safe code).
- **`From<BooleanType> for bool` removed.** Use `as_bool()` / `try_*_bool` /
  `TryFrom` — `NullVal` is not a Rust `bool`.
- **`EncodeError::BufferTooShort` carries `field`.** Exhaustive matches need
  the new field (or `..`).
- **`DomainConversionFailed` carries `reason`.** Bool null/unknown uses
  `InvalidBoolean` instead.
- **`MessageVisitor::visit_unknown` has no panicking default** — implementors
  must handle unknown templates.
- Encoder **and decoder** metadata on messages with tails: complete-sounding
  `as_bytes_with_header` replaced by `as_fixed_region_with_header` /
  `as_fixed_body_bytes` on the metadata facet (fixed block only). Complete
  stages and decoder inherent tail-rescan helpers keep full-frame names.

### Added
- **`add_checked` — group entries proven complete at compile time.** The
  closure takes the entry encoder by value and must return the entry's
  `{Group}EntryComplete`, a type reachable only by writing every required tail
  in wire order. An entry that skips, reorders, or repeats a tail fails to
  compile instead of emitting a short entry at run time. Flat entries reach it
  through `EntryEncoder::complete()`. `add` stays for entries checked
  elsewhere.
- **Group decoders poison themselves on a malformed entry.** A dynamic-entry
  group that hits a bad entry can neither yield another entry nor construct a
  later message stage, so a truncated tail cannot be mistaken for a short
  group. Fixed-stride groups keep a constant proven stride and carry no poison
  state or extra field.
- **`DomainVarData::Strings`** replaces `LossyStrings` (strict UTF-8; same
  behaviour as 0.1.10+).
- Restored `docs/SBE_COMPATIBILITY.md`.
- `#[inline]` on `bulk_decode`, staged `finish_empty` / ragged length builders,
  domain DTO thin methods, `AnyMessage::visit`, staged `EncodedLength`
  transitions / complete length getters, EncodedLength zero-count group
  forwarders, and `try_from_slice_with_header`.
- `Error::source` on `EncodeError::Decode` and `VerifyError::DecodeError`.
- `#[must_use]` on message/consuming decoder stage structs and on EncodedLength
  After/Complete stages (`length builder must be completed`).
- Reserved ⊆ emitted enforcement test; crate README + feature-matrix placement
  metadata row; group vs metadata `remaining()` table; dual
  `acting_version` docs; expanded `into_remaining_mut` / group `remaining`
  rustdoc.
- Documented `ItemContext` hook fields; `acting_version` / `acting_block_length`
  on decoders and consuming stages; metadata
  `message_offset` / `limit` / `buffer` / `remaining`; field-level error
  variant rustdoc; book guidance for `apply_nulls`, claim sizing, decode
  stages (`finish` / `skip_remaining` / must_use), metadata limits, FixedFields
  (no Default), hybrid bare `decode`, and parity-gate artifact archiving.

### Fixed
- Encoder metadata `message_offset` / `limit` / `buffer` are `const fn` again;
  moving them onto the metadata facet had dropped the qualifier.
- A `presence="constant"` field no longer counts toward the readable fixed
  extent. Constants carry no wire bytes, so a message whose fields are all
  constant encodes to a bare header — and the decoder used to reject the frame
  its own encoder had just produced. A group with a tail masked this, so it
  only showed on a tail-free message.
- Aeron `try_claim` recipe rewritten against the real API — the previous
  snippets mixed `usize` and `i32` for the same binding, called a
  `buffer_mut()` that does not exist, imported generated codecs from
  `ergo_sbe`, and hand-rolled an 8-byte framing prefix that the claimed region
  already excludes.
- Truncated `# Safety` docs and garbled group-encode rustdoc.
- Trust-boundary docs/README aligned to three-tier constructors; bare
  `decode` / `decode_unchecked` rustdoc describe the hybrid identity+extent
  contract.
- `decode_unchecked` uses unchecked header reads as documented.
- Generic encode `BufferTooShort` labels now use group-field names.
- Keep-matrix times three constructor tiers (try / bare / unchecked).
- Narrower generated `#[allow]` list (no blanket `unused_unsafe` /
  `unused_imports` / `needless_borrow`); remaining allows documented in codegen.
- Bare decoder `wrap` uses a direct extent check (no `Result` on the success
  path), matching encoder bare wrap; cold panics use static strings.
- Deduped encoder/decoder reserved method-name lists into a single source of
  truth (`codegen/conversion_helpers.rs`). Placement utils (`remaining` /
  `buffer` / `limit` / `message_offset` / fixed-block-only `as_fixed_*`) are
  **only** on `get_metadata()` and are **not** reserved field renames — schema
  fields may use those names without a `_field` suffix. Migrate
  `dec.remaining()` → `dec.get_metadata().remaining()`. Stale reserved entry
  `header` (never emitted) removed.
- Bench fairness: batch decode arm uses `wrap_unchecked` to match sbe-tool's
  zero-check wrap (equal work).

## [0.1.12] — 2026-08-04

### Breaking
- **Two-lane trust boundary.** Safe checked constructors (`try_wrap`,
  `try_wrap_and_apply_header`, `try_decode`) validate the buffer extent and
  return `Result`. The zero-check lane (`wrap_unchecked`,
  `wrap_and_apply_header_unchecked`, `decode_unchecked`) is `unsafe` with the
  extent precondition in `# Safety`. Group dimension zero-check wrap
  (`wrap_trusted`) is now `pub(crate) unsafe`.
- **`wrap_into_claim` requires exact-length buffer.** Uses new
  `ClaimLengthMismatch` error instead of `BufferTooShort`. Aeron claim
  buffers must be sliced to exactly `ENCODED_LENGTH`.
- **`WrongTemplate` error variant.** Template-ID mismatches now report the
  expected message name, not the schema name. `WrongSchema` is reserved for
  `schemaId` mismatches.
- **`as_bool()` on boolean enums.** `From<BooleanType> for bool` is kept but
  collapses `NullVal` to `true`. Prefer `as_bool() -> Option<bool>` or
  `try_<field>_bool() -> Result<bool, DecodeError>` for required fields.
- **Group random access renamed.** `nth(&self)` → `entry_at(&self)` (O(1)
  fixed-stride) and `scan_entry_at(&self)` (O(n) dynamic entries). The old
  name shadowed `Iterator::nth(&mut self, n)`.
- **Removed `with_unchecked_companions`.** The option was a documented no-op.
  The unchecked lane is the constructor-level `*_unchecked` API instead.

### Added
- **`raw_<field>()` on message decoders and composite members.** Returns the
  raw wire discriminant alongside the typed enum getter. Use for logging,
  relaying, or version-negotiating unknown enum values.
- **Text var-data helpers gated on `characterEncoding`.** Public checked
  `_as_str()` and unsafe `_as_str_unchecked()` only emitted for UTF-8/ASCII.
  ASCII accessors use `InvalidAscii` error. Consuming `into_<field>_as_str()`
  handles both UTF-8 and ASCII correctly.
- **`#[inline]` on staged encoded-length transitions.** Ragged builder
  methods (`add`, `entries`, `group`), nested group/var-data forwarding, and
  `bulk_decode` wrapper.
- **`ClaimLengthMismatch` encode error** for exact-claim buffer validation.

### Fixed
- **`bulk_decode_into` restricted to version-stable flat groups.** No longer
  emitted for groups with `sinceVersion > 0` or optional fields, preventing
  cross-entry reads and fabricated values.
- **`decode_frame` uses external `frame_len` as authoritative boundary.**
  No longer rescans all group/var-data tails. Decoder is bounded to the
  frame slice.

### Performance
- **`#[inline]`** on entries/counts, ragged builders, stage transitions,
  and `bulk_decode` wrapper (no-LTO evidence).

### Docs
- `instruction_counts` described as amplified Criterion timing (Iai removed).
- Crate rustdoc describes the two-lane trust boundary.
- Removed all `with_unchecked_companions` references from book and docs.

## [0.1.11] — 2026-08-03

### Breaking
- **`get_metadata()` zero-copy metadata struct.** Decoder utility methods (`limit()`,
  `buffer()`, `message_offset()`, `as_body_bytes()`, `as_bytes_with_header()`,
  `acting_version()`, `remaining()`) moved from the decoder to a zero-copy
  `XxxDecoderMetadata` struct returned by `dec.get_metadata()`. Encoder gains the
  equivalent. This prevents schema field names from colliding with utility methods.
  Migrate: `dec.limit()` → `dec.get_metadata().limit()`.

### Added
- `ENCODED_LENGTH` constant emitted for all messages (was only emitted for
  messages without var-data/groups)
- Codegen produces a readable diagnostic (field names + suggested rename) on
  keyword collisions
- `union` added to Rust keyword list
- Expanded error-message quality tests
- Generated-code showcase in book with `get_metadata()` example

### Fixed
- Keyword collision handling with custom append token verified end-to-end
- Reserved keyword field test covers both with and without `with_keyword_append_token`

## [0.1.10] — 2026-08-02

Breaking dual-lane soundness release. See `docs/MIGRATION_0_1_TO_0_1_10.md` and
`docs/SBE_COMPATIBILITY.md`.

### Breaking
- Safe constructors are fallible: `wrap` / `wrap_and_apply_header` / `decode` return `Result`; `try_wrap*` removed
- Zero-check twins are `unsafe fn *_unchecked` (doc-hidden until keep gate)
- Public safe `read_bytes_unchecked` / `write_bytes_unchecked` removed (private unsafe only)
- Domain DTOs: no panicking `From`; use `try_from_decoder`; domain converters are `try_*`
- `LossyStrings` no longer invents empty strings for invalid UTF-8 (`InvalidUtf8`)

### Fixed
- Version-aware decoder min fixed extent (header-only / blockLength=0 no longer UB)
- Optional null sentinels: exact width + endian for message and group fields
- Group var-data schema `maxLength` enforced; domain group counts use `try_from`

### Added
- `docs/SBE_COMPATIBILITY.md`, `docs/MIGRATION_0_1_TO_0_1_10.md`
- `soundness_hostile_constructors_test` hostile/safe-constructor gates
- `GenerationProfile::{Full, Lean}` preset (`GenerationConfig::profile`)
- Typestate compile-fail + size_of/Send budgets; checked/unchecked
  identity + keep-sample harness; lean profile matrix tests
- Book: type-state design note, API freeze decisions, Coming from sbe-tool,
  Road to 1.0, generated-code showcase, benchmarks methodology split
- Crate READMEs and crate-level rustdoc link the ergo-sbe book (visible on docs.rs)

### Changed
- Size knobs `with_display_debug` / `with_meta_attributes` / `with_dispatch` now
  honored by codegen (were API-only no-ops in 0.1.9)
- `with_unchecked_companions` reframed as supported post-validation opt-in
- Crate-root clippy allows burned down; XML parser split to modules
- `Cargo.lock` untracked (library workspace)

### Internal
- `deny.toml` Apache-2.0 / ecosystem licenses; CI thin bench lane restored

## [0.1.9] — 2026-08-01

Shipped as crates.io `ergo-sbe` / `ergo-aeron-cluster` **0.1.9**, git tag
`v0.1.9` @ `0b008696`. Section matches that tree (not later `feat-0.1.9`
commits).

### Changed
- **Codegen split:** mod.rs 9,075 → 1,723 lines (-81%), 14 responsibility modules extracted
- **Config API unified:** `enable_*` renamed to `with_*`; all boolean toggles take `(bool)`
- **Config size knobs (API surface only):** `with_display_debug(bool)`,
  `with_meta_attributes(bool)`, `with_dispatch(bool)` — defaults true in config.
  **Caveat:** 0.1.9 codegen did not read these flags, so `false` was a no-op;
  real omit-on-disable behaviour ships in 0.1.10.
- **Benchmark methodology** requires self-comparison against previous release (not just sbe-tool)
- **No-LTO bench gate is canonical** hard gate; LTO moved to soft warning (thermal variance on shared hardware)
- Nightly CI → weekly schedule
- `rust,ignore` fences allowed unconditionally in `.md` files for syntax highlighting

### Added
- `with_unchecked_companions` safety contract documented in rustdoc
- `cargo-deny` + `cargo-audit` in CI and justfile (`deny.toml`)
- Aeron `try_claim` integration recipe page in book
- bench-cold diagnostic wired into `just release-check`

### Fixed
- Outdated 17% performance claims replaced with current benchmark evidence
- Book introduction redesigned with runnable code above the fold
- Book landing pages filled with orientation prose
- AI-ASSISTANCE.md book page converted to `{{#include}}`
- Recipes page broken markdown fence fixed
- Missing `libbsd-dev` across CI jobs; `just` in pages workflow
- Release notes existence checked before publish, not after

### Internal
- Public facade tightened: `ir`, `resolve`, `xml` modules doc-hidden
- Dead code removed: `generate_raw_fixed_impls`, `has_nested_dynamic_tail`, `generate_encoded_length_builder`
- Cluster lockstep mechanized: `version.workspace = true`
- `Cargo.lock` tracked at release; per-sample READMEs deleted; stale directories cleaned
- Semver checks extended to cluster crate

## [0.1.8] — 2026-08-01

### Fixed
- **AnyMessage dispatch marker collision.** Dispatch now uses the same
  pre-allocated marker names as message generation; colliding template names
  (e.g. `Msg` / `MsgMessage` with a `MsgSchema` composite) no longer route
  template 2 through template 1's dispatch arm.
- **AnyMessage::encode short-buffer panic.** Both known and unknown variant
  arms now check destination capacity and return `BufferTooShort` instead of
  slicing past the buffer end.
- **Decoder struct regression (22% vs 0.1.7).** The `msg_offset: usize` field
  added 8 bytes to every decoder and one extra store per `wrap()` call. It is
  now computed lazily as `pos - HEADER_LENGTH`, valid on the initial decoder
  before stage transitions. `decode_entry_point` returns to 0.73× sbe-tool
  (beating 0.1.7).
- **Auto-boolean discriminant validation.** Auto-detection now requires
  discriminants 0 and 1. Arbitrary encodings (e.g. `Yes=5, No=3`) must use
  explicit `with_conversion`.
- **Documentation gate bypass.** `docs_validation_test` had deferred success
  paths for five compilation failures; all are now hard errors. `pages.yml`
  uses `just book-ci` directly. Stray Rust and unmatched fences in
  `timestamps.md` removed. Broken book links, stale sample counts, and MSRV
  mismatches fixed.
- **Release workflow.** Version extraction uses `cargo metadata` instead of
  `grep` on `Cargo.toml` (which contains `version.workspace = true`). The
  crates.io wait loop now fails on timeout. `release-check` dry-runs only
  `ergo-sbe`; cluster dry-run is deferred until `ergo-sbe` is published.

### Changed
- **Benchmark methodology** now requires comparing against both sbe-tool
  (ceiling 1.00) and the previous release's absolute times (self-comparison).
- **Policy checker** allows `rust,ignore` fences in `.md` files when the
  fence body starts with `{{#include` — code verified by the project's own
  build.
- **Code style sweep** across samples, examples, and book:
  `EncodedLength::new()` → `Encoder::compute_length()`,
  `BooleanType::True`/`False` → `true.into()`/`false.into()`,
  `wrap_and_apply_header` → `wrap_and_apply_header` on known-size buffers.

## [0.1.7] — 2026-07-30

### Added
- Hook system for custom code generation — append custom impls (e.g. serde) or extra tokens after generated types
- `remaining()`, `whole_buffer()`, and `message_offset()` on flat decoders; `remaining()` and `remaining_mut()` on encoders
- Field name clash renaming — `_field` suffix when a schema field collides with a reserved method

### Fixed
- Decoder Display and encoder `fixed()` now use clash-resolved field names
- Missing `#[allow(missing_docs)]` on `ItemContext`

## [0.1.6] — 2026-07-30

### Changed
- **Chainable set API** — set bit setters drop the `set_` prefix and return `&mut Self` for chaining; bool getters use `is_` prefix. `OptionalExtras::default().cruise_control(true).sports_pack(true)` in one expression.
- README restructured — features table and compilable example before narrative

### Fixed
- 5 broken hyperlinks in docs.rs rustdoc replaced with correct file paths
- Docs validation — all bare `rust` fences compile against the docs_codec schema

## [0.1.5] — 2026-07-30

### Added
- `just release` — single-command publish gate (test + bench → crates.io → tag → GitHub release)
- Header-mode fairness policy tests — every encode gate pair is audited for mixed work
- `SECURITY.md`, issue/PR templates, and `dependabot.yml`
- GitHub Discussions enabled

### Fixed
- **Cluster benchmark fairness** — encode gates were unfair: ergon wrote headers while sbe-tool did not. Both arms are now body-only. Fixed 1.65× and 1.12× regressions.
- **XML parser** — `constantValue` attribute on constant fields was validated but never read; accessors were silently not generated
- **Group/perf length math** — replaced `encoded_length() + 8` with `get_limit()` for sbe-tool full-wire length

### Changed
- README restructured — features and example first; crates.io/docs.rs visitors see the pitch immediately
- `just check-mutation` uses `--jobs 1`; weekly mutation CI replaced with manual task

## [0.1.4] — 2026-07-30

### Added
- Executable `test-lanes.tsv` ownership for every tracked Rust test, doctest,
  benchmark, fuzz target, and Miri fixture. `just policy` self-tests the
  checker before applying it to the repository.
- Policy enforcement that rejects ignored tests, ignored Rust fences,
  runtime `SKIP` reporting, command-level test-selection bypasses, multiline
  conditional test execution, failure-to-success wrappers, workflow
  `continue-on-error`, and custom skip-CI conditions.
- Fail-closed coverage and mutation ratchets with adversarial shell self-tests.
  The mutation checker rejects a missing, incomplete, or empty tool run instead
  of treating it as zero missed mutants. Mutation scope expanded from 180 to
  196 critical-path candidates.
- Pull-request coverage enforcement and 32-bit x86/big-endian s390x execution
  through `cross`/QEMU, plus nightly Miri and ten-minute-per-target fuzz jobs
  and a weekly critical-path mutation job.
- A dedicated `just update-golden` command plus non-mutating
  `just check-golden`; golden regeneration is no longer hidden inside an
  ignored test.
- `fairness_policy_test` for maintained SBE and Cluster parity sources:
  `std::hint::black_box`, pre-timing correctness assertions, and the
  sceptical/LTO disclosures are machine-checked.
- Benchmark fairness documentation (`sbe/benchmarks/README.md`) — mandatory
  checklist for every parity benchmark.
- LTO-on and LTO-off group benchmark matrix. This exposed an ergon inlining
  defect that the previous LTO-only results hid.
- `group_encode_decimal_bench` — encode comparison with `rust_decimal::Decimal`
  converters.
- sbe-tool comparison arm in `group_encode_bench`.
- `bulk_add(&[Entry])` regression coverage for exact bytes, count overflow,
  short buffers, and empty groups.
- `bulk_add_domain(&[EntryDomain])` for eligible flat DTO groups, sharing the
  wire bulk writer's single-region validation without a temporary allocation.
- `just check-mutation` — manual-only mutation gate (removed from CI: too slow).
- README badges (Crates.io, CI, docs.rs) for root, sbe, and cluster crates.
- AI-ASSISTANCE.md: documented LLM test-suppression pattern with specific
  project incidents.

### Fixed
- Restored all three allocation-count tests to the ordinary sample lane; they
  already passed when the stale ignored attributes were removed.
- Restored all Cluster restart/quorum tests to the required Java lane. The
  log-recovery test exposed and fixed four harness defects: a client outlived
  its embedded media driver, restart returned before Java readiness, the custom
  launcher class was shadowed by a stale copy inside `aeron-all.jar`, and crash
  recovery restarted before Aeron's 10-second archive-mark lease expired.
- Cluster parity benchmarks now assert local encode bytes and decode values
  before timing, use exact-sized frames, make both mutable buffer inputs opaque,
  and use `std::hint::black_box` symmetrically. The connect-request case now
  uses the same three fixed-field setters and observes encoded length in both
  arms.
- Moved full-message wire parity ahead of Criterion timing; the old check ran
  after measurements. Added source guards for this ordering and the Cluster
  connect-request equal-work contract.
- Replaced schema-loop `SKIP`/`continue` paths with asserted parse outcomes.
  Missing production fixtures and unreadable directory entries now fail rather
  than disappearing from the test count.
- Replaced ignored Rustdoc examples with compile-checked `rust,no_run` examples
  or explicitly schematic `text` fences. Generated goldens were regenerated
  through the dedicated command.
- Removed the unused `encoded_length_api.txt` file, which advertised a
  regeneration test that did not exist and was not checked by any test.
- **Composite explicit-offset bug**: `get_token_block_size` summed child sizes
  ignoring `offset=”N”` attributes on composite members. A composite with a
  field at `offset=”8”` reported size 9 instead of 16, cascading into wrong
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
- **XML parser**: constant fields with `constantValue` attribute were validated
  but their value was never read — only `valueRef` was read. Constant field
  accessors were silently not generated for group entry fields.
- **Mutation coverage gaps**: added hostile-frame tests for group array
  boundaries, versioned enum/set/boolean/optional-primitive extents, nested
  group and var-data counter mutations, and Display output on group entries.
- **Dead locals** in the code generator removed (unused `ng_idx`-adjacent
  variables).

### Changed
- `just test` now requires and runs the Aeron Java lifecycle/recovery suite and
  Cluster HA sample. Missing Java, Gradle, jars, or another lane dependency is
  a failure rather than a partial green run.
- CI jobs depend on the policy gate, and the release workflow runs
  `just release-check` (including the coverage ratchet) before publishing.
- Shared GitHub runners publish LTO and no-LTO Criterion diagnostics without
  using noisy nanosecond ratios as merge gates. Strict ratio gates remain local
  and dedicated-stable-runner checks.
- Benchmarks use `wrap_and_apply_header` (infallible) instead of
  `wrap_and_apply_header` — sbe-tool's `header()` does no validation,
  so ergon's validation was extra work.
- Every maintained ergon/sbe-tool benchmark now has a strict `1.00` ceiling
  under both LTO and no LTO. A repeatable sbe-tool win blocks the change until
  the benchmark or generated hot path is fixed.
- Weekly mutation CI replaced with manual `just check-mutation` task
  (too slow for CI; ~6-10 hours with `--jobs 1`).

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

### Yank audit

crates.io records 0.1.0 as yanked. The repository contains no contemporaneous
commit, issue, or release note naming one authoritative yank reason, so later
defects must not be presented as a proven private motivation.

The history does prove that 0.1.0 predated the little-endian SBE header fix and
the 0.1.1 official sbe-tool parity expansion. It also contained the composite
explicit-offset defect later found in 0.1.4. The explicit/implicit regression
itself therefore existed from 0.1.0, but the command-line filter that hid it
was introduced much later during 0.1.3 work. These are verified release
confidence failures; only the exact decision to yank remains undocumented.

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
