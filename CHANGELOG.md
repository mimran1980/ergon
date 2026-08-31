# Changelog

## [0.1.24] — 2026-08-31

### Fixed
- **Domain objects + domain types generate compilable code again.** Fifteen
  codegen defects made `with_domain_type` / `with_manual_domain_type` unusable
  alongside `with_domain_objects` for most field shapes. Every one produced a
  generated module that failed to compile, so no wire format changed and no
  working code can have depended on the old behaviour.

  Reported against 0.1.22 and 0.1.23, then found while root-causing it:
  - A `sinceVersion > 0` **composite** wrapped `try_*` in `Some(..)`. `try_*`
    on a versioned field already returns `Result<Option<T>, E>`, so the DTO
    built `Option<Option<T>>` for an `Option<T>` field.
  - The same double-wrap on a `sinceVersion > 0` **primitive scalar**.
  - A **fixed array** matched by a selector generated converters against the
    element type (`u8`) instead of the array type (`[u8; 4]`).
  - An **array or optional scalar** with a conversion read the raw flyweight
    getter under its pre-rename name, though a conversion renames it `*_wire`.
  - An **array or optional scalar** DTO kept the wire type but wrote through
    the domain `try_*` setter, which takes the domain type.

  Found by the combination matrix below:
  - A **versioned array** was treated as `Option`-returning, though an array
    accessor returns a zero-filled array, never `None`.
  - A **constant** field matched by a selector generated conversion accessors
    against a `*_wire` setter that constants do not have.
  - `Display` for a **group-entry composite** with a domain type re-derived the
    conversion and fed `Option<T>` to `TryFromSbe<T>`; it now goes through the
    generated `try_*` accessor.
  - `Display` for a **converted primitive** read the raw getter under its
    pre-rename name, at message and group-entry level.
  - Conversion-only `*_as` accessors did not `Option`-unwrap a versioned or
    optional raw getter, at message and group-entry level.
  - `with_all_enums_as_option` makes enum accessors return `Option<T>`, which
    neither the `*_bool` accessors nor the domain DTO accounted for.
  - An **optional bool enum** produced a group-entry accessor that matched
    `Option` against a plain `BooleanType`.
  - An **optional enum with a domain type** was treated as `Option`-returning;
    an optional enum carries `NullVal` as a variant, so its accessor is plain.
  - `null_as_option` **combined with an enum domain type** left `try_*`
    disagreeing with the accessor it delegates to, in both directions.
  - Group-entry and message-level boolean accessors disagreed on when they
    return `Option`; both now do so only when version-gated.

  Most shared one root cause: the same decision — "is the domain type applied
  to this field?", "is this accessor `Option`-returning?" — was made
  independently in several places and drifted apart. Those now live in single
  predicates (`conversion_helpers::dto_domain_type`,
  `converter_impls::is_optional_domain_field`).
- `ConversionSelector::FieldPath` selects fields again. It is documented as the
  primary selector form and validation accepted it, but codegen matched only
  `NamedType` and `SemanticType`, so every `FieldPath` selector was a silent
  no-op. Group and nested-group fields extend the path with their group names
  (`"Message.group.field"`).
- Documented selector precedence is enforced: `FieldPath` beats `SemanticType`
  beats `NamedType`. Resolution was "first selector in registration order", so
  precedence silently depended on call order.
- Domain types configured for **enum** and **set** fields reach the DTO.
  `try_*` accessors were generated for them all along, but the domain-DTO
  generator ignored them and emitted the raw generated type. Boolean enums are
  unchanged — they still materialise as `bool`.
- Scratch-crate test helpers no longer force `--offline` / `CARGO_NET_OFFLINE`,
  and they strip that flag when inherited from `cargo llvm-cov`. Nested
  compile-and-run crates can download extra transitives (`bit-set`, `ahash`)
  that are not in the workspace lock — the cause of red `test`/`coverage`
  jobs on 0.1.19–0.1.22 and Dependabot PRs.
- The coverage job timeout is 120 minutes (was 30). Instrumented
  compile-and-run tests cannot finish in half an hour; the 0.1.23 coverage
  job was cancelled mid-run.
- Nightly `cargo fuzz` pins `x86_64-unknown-linux-gnu`. cargo-fuzz's Linux
  default is musl+ASAN, which fails on a minimal nightly without that target.
- Group-dimension IR extraction matches the required `blockLength` and
  `numInGroup` members by exact name. Padding fields such as `account`,
  `countPadding`, or `blockLengthPadding` no longer overwrite the layout.
- Generated group-entry `Display` formats a domain-mapped set through
  `try_*` and `{:?}`. The raw accessor is `*_wire`; calling `self.<field>()`
  failed to compile.
- `poll_connect_until_done` idles with work-count 0 while the handshake is
  still in progress. `AsyncClusterConnect::poll`'s `true` means more polling
  is needed, not that Aeron performed work; a positive count reset backoff
  and pinned a core.
- `parse_ingress_endpoints` rejects empty comma-separated entries (leading,
  trailing, or repeated commas) instead of skipping them.
- `write_module_set` restore after a mid-commit failure replaces an
  already-promoted destination (Windows-safe) and surfaces restore errors
  rather than discarding them. Successful publication fails if a `.bak`
  backup cannot be removed.

### Added
- **Codegen combination matrix** (`sbe/tests/codegen_matrix_test.rs`). Every
  field shape x `sinceVersion` x mapped/unmapped cell, at all three codegen
  locations (message, group entry, nested group entry), crossed with every
  `GenerationConfig` combination that selects a distinct codegen path — and
  each case **compiles** the generated module rather than inspecting its
  source. Source-string assertions were what let the defects above ship: they
  stay green while the module fails to compile.

  The fixture is generated by `scripts/regenerate-codegen-matrix-fixture.py`,
  whose `SHAPES` list is the single source of truth; the test recomputes the
  same cross product and fails when the fixture falls behind, and `--check`
  (wired into `just preflight`, `just test`, and CI) fails on drift.

### Changed
- **Breaking (generated API):** the decoder's boolean getter is `try_{field}_bool`
  at every shape and location. Message-level code named the `Option`-returning
  one `{field}_bool` while group entries named both shapes `try_*`, so the same
  schema shape had two different accessor names depending on where it sat —
  which is what let the domain DTO call an accessor that did not exist.
  `{field}_bool` is now unambiguously the *encoder setter*; `try_` marks the
  fallible decode. The only remaining shape difference is the return type:
  `Result<bool, _>`, or `Result<Option<bool>, _>` when `sinceVersion > 0` makes
  the field absent on older wire versions.
- **Breaking (generated API):** conversion-only `{field}_as::<T>()` returns
  `Result<Option<T>, _>` for a `sinceVersion > 0` or optional field, matching
  the domain-typed `try_*` accessors. It previously generated a body that fed
  `Option<W>` to `TryFromSbe<W>`, so no compiling code can depend on the old
  signature.
- Hook `FieldInfo::rust_type` for groups and var-data is a real type
  (`LevelsDecoder`, `&[u8]`) instead of the sentinels `"group"` / `"data"`.
  `FieldInfo::kind()` returns `FieldKind` (`Fixed`, `Group`, or `Data`).
- Consumers are advised to build with `lto = true` / `codegen-units = 1`.
  Generated codecs are many small `#[inline]` methods whose inlining only fully
  collapses across codegen units; `sbe/BENCHMARKS.md` and the book's benchmark
  chapter now document the recommendation. Benchmark methodology no longer
  describes the LTO profile as informational — both profiles have been blocking
  gates with the same literal `1.00` ceiling.
- Hook `FieldInfo` carries a `kind: FieldKind` field recorded when the field
  is built. `kind()` returns it instead of inferring from `rust_type` suffix
  and substring checks, so renaming a generated type cannot silently change a
  field's meaning.
- `just check-local`, `just check-products`, and `just test` no longer run the
  `ergo-aeron-cluster` suite twice: the crate is excluded from the workspace
  run that already covers it explicitly, and its doctests run once via
  `cargo test -p ergo-aeron-cluster`. Same for CI's workspace doctest job.
- CI, `just test`, and the local product gates run
  `cargo test -p ergo-aeron-cluster` (integration tests, compile-fail API
  suite) rather than `--lib` only.
- Generated public-API freeze records trait associated-type bounds,
  defaults, generics, where-clauses, and semver attributes.

## [0.1.23] — 2026-08-28

### Fixed
- Generated `Display`/`Debug` bodies no longer assume a `with_domain_type`
  target implements `Display`. Domain values format with `{:?}`, so a domain
  type only has to be `Debug`.
- `with_display_debug(false)` now also omits the group entry decoder's
  `Display` impl, which was emitted unconditionally.

### Changed
- **Breaking:** optional var-data accessors (`into_*_as_compact_str`,
  `into_*_as_smol_str`, `into_*_as_bytes`) are emitted only when the
  *generator* was built with the matching `compact_str` / `smol_str` / `bytes`
  feature — reverting the 0.1.x switch to gating them on the consumer's own
  features. That gate was a `#[cfg(feature = "…")]` in the emitted source,
  which resolved against the consumer crate's feature table; consumer crates do
  not declare features by those names, so the accessors were unreachable dead
  code in every generated module. Enable the feature on `ergo-sbe` in both
  `[build-dependencies]` and `[dependencies]`.
- The generated public-API snapshots no longer record those accessors, since
  their presence now depends on the generator's feature set.

## [0.1.22] — 2026-08-27

### Added
- Generated observers named by T-1 (`after_this_message`, message/group
  `min_readable_fixed_extent`, `MessageHeader` peeks, `schema_id_from_header`,
  domain `to_wire_entry`) carry specific `#[must_use]` diagnostics.
- `StaticCredentials::new` accepts `impl Into<Vec<u8>>` (moved `Vec`, slice,
  array, embedded NUL, invalid UTF-8) with byte-identical connect and challenge
  credentials. `from_utf8` remains the text convenience.
- Poll-driven connect recipe in rustdoc, `cluster/README.md`, and the session
  builder book chapter: `SessionBuilder` → `connect_async` → `default_idle` →
  `poll_connect_until_done` → `finish`. `AsyncClusterConnect` is a poll-driven
  Aeron state machine, not a Rust `Future`.
- Group-dimension composites accept unsigned `u8`/`u16`/`u32`/`u64` members
  (both endians, reordered offsets, padded composites over 32 bytes). Parser
  rejects signed/optional/array/missing/overlapping/out-of-bounds members.
  Over-limit counts and non-representable 32-bit `u64` wire values are typed
  range errors.
- Unpublished workspace crate `ergo-aeron-cluster-test-harness` owns the Java
  ClusterLauncher, harness tests, and examples. Release automation unpacks
  `ergo-aeron-cluster` with every advertised feature and fails closed.

### Changed
- **Breaking:** `ergo-aeron-cluster` no longer advertises a `test-harness`
  feature or `test_support` module. Depend on `ergo-aeron-cluster-test-harness`
  in this repository.
- `AnyMessage::decode_frame` rejects every `frame_len` in `0..HEADER_LENGTH`
  before reading header fields, even when the backing slice contains a later
  valid header.

## [0.1.21] — 2026-08-24

### Added
- `GenerationConfig::with_manual_domain_type` and `with_domain_type` now emit
  correctly `Option`-wrapped `try_*` accessors for optional (or
  `sinceVersion`-gated) domain-typed fields inside repeating groups, matching
  the message-level behaviour. Generated `Display`/`Debug` for group-entry
  domain-typed `Primitive`/`Enum` fields now shows the converted domain value
  instead of failing to compile.

### Changed
- **Breaking:** `BuildError` is `#[non_exhaustive]`. `Io(std::io::Error)` is
  now `Io { action: &'static str, path: PathBuf, source: std::io::Error }`,
  naming the exact attempted step and destination instead of a bare I/O
  message.
- `generate_multi_to_dir` / `write_module_set` publish a complete generated
  module set as one all-or-nothing unit: every module is staged to a unique
  temp file before any destination is touched, and a mid-commit failure
  restores every pre-existing destination and leaves no temp/backup debris.
  A cleanup step that itself fails is reported in preference to the failure
  that triggered it, so a rollback never reports itself clean while leaving
  debris. Deleting the backups of a *successful* commit is past the point of
  no return and does not roll back: every backup that can be removed is
  removed, then the publication fails naming the first that could not.
- `GenerationConfig::with_keyword_append_token` is validated at `generate()`:
  an empty or invalid token now returns
  `GenerateError::InvalidConfiguration` instead of generating uncompilable
  Rust.
- **Breaking:** `SessionBuilder::ingress_endpoints` returns
  `Result<Self, ClusterError>` and validates the endpoint map (grammar,
  duplicate IDs, empty entries) at the call site instead of at `validate()`
  or the first `poll()`.
- **Breaking:** `poller::parse_event` returns `Result<EgressEvent, ClusterError>`
  (was `Result<Option<EgressEvent>, ClusterError>` — every decoded branch
  was already `Some`, so `None` was unreachable).
  `poller::parse_leader_endpoint` returns `Result<Option<String>, ClusterError>`
  (was `Option<String>`) — a malformed endpoint map is now `Err` with the
  parser's specific reason, distinct from `Ok(None)` for a well-formed map
  that simply lacks the requested leader.
- Message-bearing `#[must_use]` on pure cluster decision observers:
  `PublicationFailure::is_retryable`/`raw_code`, `ClusterError::is_retryable`,
  `AeronCluster::{cluster_session_id, leadership_term_id, leader_member_id,
  is_ingress_connected, is_ingress_closed, ingress_position,
  is_egress_connected, state}`, `ClusterClaim::position`, and
  `AsyncClusterConnect::{step, is_complete}`.
- **Breaking:** `PublicationFailure` is `#[non_exhaustive]` and gains
  `TooManyParts`, mapped directly from `AeronOfferError::TooManyParts`
  instead of the fabricated sentinel `Other(-100)`. `raw()` is replaced by
  `raw_code() -> Option<i64>` (`None` for `TooManyParts`, which has no real
  Aeron wire code).

- The generated public-API freeze gate (`api/generated/*.txt`) now snapshots
  the canonical semver-relevant surface — struct fields, enum variant
  payloads/discriminants, full fn/method signatures (receiver, generics,
  args, return type, where-clause), associated types/consts, type aliases,
  and cfg/non_exhaustive/repr/deprecated/must_use attributes — instead of
  item names only. A cfg-gated item is now recorded (with its condition)
  instead of being silently excluded from the freeze.
- The `SbeMessage` sealing-trait path is now explicit per-schema state
  (`GenerationContext`, passed by reference into decoder/encoder generation)
  instead of a `thread_local!`. A hook that invokes a nested `Generator`
  (e.g. to emit a companion crate) can no longer leak its sealed path into
  the outer generation's remaining messages.
- `sbe/BENCHMARKS.md` and `book/src/sbe/benchmarks.md`'s "Group encode: LTO
  on and off" section no longer quotes unprovenanced point estimates
  (`414.1 ns`, etc., with no run-id/commit/host and duplicated verbatim
  across both files) as if they were current — it states the qualitative
  ranking and the reproduction command instead.
- Generated domain-object `encoded_length()` / `encoded_length_with_header()`
  rustdoc no longer contradicts `encode()`: `encoded_length()` is documented
  as body-only, and `encoded_length_with_header()` as the exact buffer size
  and return value of `encode()` for both fixed and dynamic messages.
- Generated `{Group}Encoder::wrap`, `ENTRY_BLOCK_LENGTH`, and
  `GROUP_DIM_TEMPLATE` now carry authored rustdoc explaining the standalone
  group framing contract (offset is the first entry, the dimension header is
  the caller's responsibility) instead of a placeholder.

### Fixed
- Domain-object generation for a required (non-`sinceVersion`) boolean field
  no longer panics on an unknown wire discriminant; `try_from_decoder`
  propagates `DecodeError::InvalidBoolean` instead.
- Single-line schema `description` text containing `&`, `<`, or `>` (e.g.
  `Option<u32>`) is now escaped before emission, so it can no longer break a
  downstream `-D warnings` rustdoc build with `rustdoc::invalid_html_tags`.

## [0.1.20] — 2026-08-20

### Added
- `GeneratedModuleSet::into_parts` consumes the set and returns owned modules
  and warnings without cloning generated source.
- `SchemaFile` plus `generate_multi_to_dir` / `generate_multi_to_out_dir`
  generate a shared schema and its consumers in one transaction, using
  supplied module names and watching every resolved include.
- `EncodeError::InvalidAscii { field }` for ASCII fixed-array `*_str` writes.
- Generated public-API snapshots under `api/generated/` (`car_lean`,
  `car_domain`, `multi_schema_shared`) and
  `scripts/check-generated-public-api.sh`, invoked from `just preflight`.
- `GenerationConfig::with_manual_domain_type` for additive Manual
  `TryFromSbe`/`TryToSbe` mappings. `with_domain_type(selector, path)` is
  the two-argument Generated path.

### Changed
- `GenerationConfig::with_domain_type` stays the two-argument
  `(selector, path)` Generated path it has always been. Use
  [`with_manual_domain_type`] for additive Manual impls.
- **Breaking:** `ParseError` is `#[non_exhaustive]`. Root file reads are
  `Io { path, source }`. Include failures are `Include { href, attempted,
  cause }` with typed `IncludeCause` (`Cycle` / `Io` / `NotFound`).
- **Breaking:** `SessionBuilder::ingress_channel`, `egress_channel`,
  `message_timeout`, and `new_leader_timeout` return `Result` and store only
  validated values. `is_ingress_exclusive` and `owns_aeron` setters are
  removed.
- **Breaking:** generated `*_str` exists only for default-ASCII `char` and
  encodings `ASCII` / `US-ASCII` / `UTF-8` / `UTF8`. Unencoded numeric arrays
  and encodings such as GB18030 keep the raw array setter only — schemas
  relying on the old setter for those types need the raw accessor instead.
- Optional var-data accessors (`into_*_as_compact_str`, `into_*_as_smol_str`,
  `into_*_as_bytes`) are always emitted and gated on the *consumer's*
  `compact_str` / `smol_str` / `bytes` feature, not the generator's. Fixes a
  feature leak where the generator's own feature flags decided what a
  consumer crate could see, independent of that crate's own Cargo features.
- Generated rustdoc snippets referencing a `DomainImpl::Manual` field render
  as plain text instead of an ignored `rust,ignore` fence.
- Side-effect-free generated observers (`as_option` / `as_bool`, completed
  tail views/lengths, encoder metadata, group `written`, exact-length
  terminals) carry message-bearing `#[must_use]`.

### Fixed
- One-byte fixed-array getters (`char` / `uint8`) return the bulk-read
  `[u8; N]` instead of reconstructing each element with `from_le_bytes`.
- `parse_with_xsd_validation` accepts schema-declared enum `nullValue`
  (signed and unsigned) and still rejects unknown non-namespaced enum
  attributes.
- `generate_to_dir` emits `cargo::rerun-if-changed` for the root schema and
  every nested or transitive include, so an include-only edit rebuilds.
- Unresolved-type diagnostics name the containing field as well as the
  invalid type.

### Deprecated
- `GenerationConfig::with_error_from_impls` — it converts generated encode/decode
  errors through `format!` and `From<String>`, dropping fields such as `needed`
  and `available`. Implement `From<generated::sbe_rt::EncodeError>` /
  `From<generated::sbe_rt::DecodeError>` instead. Removal is scheduled for 1.0.

## [0.1.19] — 2026-08-19

### Fixed
- A composite member with `presence="optional"` and a schema `nullValue`
  (e.g. a `PriceNull9`-style Decimal's `mantissa`) now decodes as `Option<T>`,
  checked against the wire null sentinel, instead of silently misreading the
  null image as a real value. `with_domain_type` mappings onto such a
  composite (e.g. `rust_decimal::Decimal`) now fail closed with a typed error
  on the null image rather than decoding the sentinel as a huge/wrong number.

## [0.1.18] — 2026-08-18

### Added
- Reject unknown attributes on `<messageSchema>`, `<message>`, `<field>`,
  `<group>`, and `<data>` at parse time, so an authoring typo (`presense`,
  `semanticTpye`) is an error instead of being silently ignored. Namespaced
  attributes (`xsi:*`, `xi:*`, and vendor namespaces such as Binance's
  `mbx:*`) are outside the SBE grammar and still pass.
- Multiple distinct decimal composites (`Decimal64`, `Decimal128`, …) can each
  map to `rust_decimal::Decimal` via `with_domain_type`; each gets its own
  `TryFromSbe`/`TryToSbe` impl.

### Fixed
- `validate_against_sbe_xsd` no longer rejects valid real-world schemas. It
  dropped every namespaced vendor attribute check (`mbx:exponent`) on the
  floor, and its allow-lists omitted `characterEncoding` on `<data>`, `unit`
  on `<type>`, `jsonValue` on `<validValue>` / `<choice>`, and `package` on
  `<types>` — enough to reject checked-in `l3-book` and `binance-spot`
  schemas. Attribute allow-lists are now shared with the parser so the two
  cannot drift apart.
- Domain-typed optional fields no longer produce a type mismatch. An optional
  `rust_decimal::Decimal` composite decodes to a plain `Decimal` (a composite
  has no null image), an optional `chrono::DateTime<Utc>` decodes to
  `Option<DateTime>`, and the encoder setters take the plain domain value.
- A boolean enum mapped to `bool` via `with_domain_type` now emits its
  `TryFromSbe`/`TryToSbe` impl once per shared module instead of in every
  consumer, which caused a "conflicting implementation" across multi-schema
  modules.

### Changed
- Fixed-only encoders now carry `FieldsState`: `as_bytes_with_header`,
  `as_body_bytes`, `encoded_length*`, and `into_remaining_mut` exist only
  after `fixed(&FixedFields)`, so a reused buffer cannot publish or pack
  unwritten required fields.
- Cluster bench gate uses the same literal `1.00` ceiling as SBE and requires
  `--run-id` provenance (`just bench-cluster` stamps per-estimate run ids).
- `check-public-api.sh` strips a leading `v` from `baseline_tag` before
  passing `--baseline-version` to cargo-semver-checks.

### Fixed
- Schema `nullValue` / `minValue` / `maxValue` that do not fit the declared
  primitive width are rejected at parse time (no more `256_u64 as u8`).
  Signed enum `validValue`s are compared as signed, so `int8` `-1` is
  accepted when `minValue="-5"` and `maxValue="5"`.
- Maintained cluster decode benches wrap both arms at the body offset without
  extra template/schema/version work on the sbe-tool side.
- Docs-validation temp crates `cargo fetch` before `CARGO_NET_OFFLINE=true`,
  so a clean CI runner is not missing optional transitive crates.

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
