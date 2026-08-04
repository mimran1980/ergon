# ergo-sbe 0.1.12 review tickets

# 1. Quick wins (0.1.12)

Review baseline: `cargo check` passed. The targeted baseline, both sbe-tool wire
parity suites, HFT soundness/checked-unchecked suites, consuming-stage suites,
versioning suites, schema edge cases, Java-parity features, and docs validation
all passed. `RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features
--no-deps` passed. The requested `cargo doc -p sbe --no-deps` does not name a
workspace package; the package is `ergo-sbe`. The full benchmark gate was not
run, by instruction.

## T-1: Make `wrap_into_claim` require the exact fixed-frame length

- Type: API
- Stage: 0.1.12
- Priority: P1 · Effort: S
- Symptom: `wrap_into_claim` promises a buffer “sized exactly to
  `ENCODED_LENGTH`”, but accepts every oversized slice because it checks only
  `buf.len() < Self::ENCODED_LENGTH` (`sbe/src/codegen/message_encoder.rs:462-480`).
  With an Aeron claim, passing the whole larger claim can publish trailing bytes
  even though the method name and rustdoc imply an exact frame.
- Change: keep `wrap_into_claim`, but require
  `buf.len() == Self::ENCODED_LENGTH`. Add an encode error that reports expected
  and actual lengths rather than misusing `BufferTooShort` for the oversized
  case. SBE frames have no self-contained total length, so the claim boundary is
  part of the protocol contract; an “exact claim” helper must enforce it.
- What breaks (API only), what it buys: callers that passed oversized slices
  must slice them explicitly or use the ordinary checked constructor. Correct
  callers are unchanged. The win is that a helper designed for exact transport
  claims can no longer silently publish padding or bytes from a following frame.
- Acceptance criteria: add generated-code runtime tests for exact, one-byte
  short, and one-byte oversized fixed-message buffers; document the error
  variant; regenerate the golden; add a 0.1.12 changelog entry. Wire bytes for
  the exact case remain byte-identical.
- Verification plan: run the fixed-message baseline and both wire-parity suites.
  Compare the LTO and no-LTO `encode_scalar_header_and_body` assembly and gate
  ratios: replacing `<` with `!=` must not add a second hot-path check or move a
  maintained ratio above `1.00`.

## T-2: Report template-ID mismatches as `WrongTemplate`

- Type: API
- Stage: 0.1.12
- Priority: P1 · Effort: S
- Symptom: the template-ID check emits `DecodeError::WrongSchema` and supplies
  the schema package as `expected_name`
  (`sbe/src/codegen/message_decoder.rs:436-459`). The runtime formats that as
  “wrong schema” (`sbe/src/codegen/runtime.rs:14-40`), so a routing error names
  the wrong protocol concept and gives no expected message/template name.
- Change: add `DecodeError::WrongTemplate { expected, actual, expected_name }`.
  Use the SBE message name for `expected_name`; reserve `WrongSchema` for
  `schemaId`. Apply the same distinction in generated message decode and
  multi-template dispatch. SBE headers deliberately carry both identifiers;
  diagnostics should preserve that distinction.
- What breaks (API only), what it buys: exhaustive matches on `DecodeError`
  gain a variant. Users get an immediately actionable “expected template X
  (Car), got Y” error instead of a misleading schema failure.
- Acceptance criteria: update generated runtime/golden output and mismatch
  tests to assert both variants and their `Display` text. Constant header
  layouts must still omit checks for constant members as they do today.
- Verification plan: run `error_validation_test`, baseline dispatch tests, and
  both wire-parity suites. This is a cold error-path change; confirm the success
  path assembly and maintained LTO/no-LTO gate ratios are unchanged.

## T-3: Remove the no-op `with_unchecked_companions` option

- Type: API
- Stage: 0.1.12
- Priority: P1 · Effort: S
- Symptom: `GenerationConfig::with_unchecked_companions` promises generated
  methods such as `serial_number_unchecked`
  (`sbe/src/config.rs:28-38`, `sbe/src/config.rs:586-587`), but codegen only
  passes the flag into ignored parameters
  (`sbe/src/codegen/message_decoder.rs:43`,
  `sbe/src/codegen/message_encoder.rs:33`). The book presents the knob as a
  supported production surface
  (`book/src/sbe/configuration/generation-config.md:14-16`,
  `book/src/sbe/feature-tour/trust-boundaries.md:23-43`). Enabling it produces
  no distinct field API.
- Change: remove the builder, stored flag, codegen parameters, and all promises
  of field-level unchecked companions. T-7 supplies the one meaningful unsafe
  lane at the constructor trust boundary; fixed accessors remain branch-free
  after that proof. Do not add duplicate per-field unsafe methods merely to make
  the dead option appear functional.
- What breaks (API only), what it buys: build scripts that call the no-op builder
  stop compiling and must delete the call. Users no longer believe they enabled
  a safety/performance mode that does nothing, and generated surface policy has
  one clear source of truth.
- Acceptance criteria: `rg with_unchecked_companions` finds no public API or
  documentation references; config tests compare the intended generated
  surfaces directly; changelog includes the removal.
- Verification plan: generate the default and HFT-lean profiles before and
  after and compare their public APIs intentionally. Run config/profile tests,
  rustdoc with warnings denied, and the maintained benchmark gate in both LTO
  modes; removal of an unused generator flag must not change codec machine code.

## T-4: Inline only the staged length methods whose no-LTO calls survive

- Type: PERF
- Stage: 0.1.12
- Priority: P1 · Effort: S
- Symptom: many public generated encoded-length transitions lack inline intent,
  including nested group/var-data stage methods
  (`sbe/src/codegen/encoded_length.rs:257-407`), generic ragged-builder methods
  (`sbe/src/codegen/encoded_length.rs:948-999`), schema-specific nested and
  var-data forwarding methods (`sbe/src/codegen/encoded_length.rs:1063-1093`),
  and the public `bulk_decode` wrapper
  (`sbe/src/codegen/group_decoder.rs:312-321`). These are small forwarding/state
  methods on pre-encode sizing or bulk materialisation paths, and the project
  requires explicit inline intent on public generated hot methods.
- Change: perform a measurement-led audit and add `#[inline]` to these public
  transitions only where no-LTO assembly currently contains an out-of-line call.
  Mechanism: remove call/return overhead and expose constant layout sizes and
  state moves to the caller for folding. Do **not** restore `#[inline]` on enum
  `raw`/`from_raw` or composite `new` without new contrary evidence: commit
  `d7450849` removed those annotations after a measured no-LTO
  `decode_scalar` regression.
- What breaks (API only), what it buys: no source break. Nested/ragged exact
  sizing and the small bulk wrapper avoid surviving call boundaries in non-LTO
  consumers without increasing allocations or adding branches.
- Acceptance criteria: generated-source tests require inline intent on the
  proven methods; add or extend a no-LTO nested/ragged sizing microbenchmark and
  the bulk-decode diagnostic. Record before/after instruction or assembly
  evidence for each retained annotation; discard annotations with no mechanism
  or a regression.
- Verification plan: inspect `cargo asm`/`objdump` for the representative nested
  builder and bulk wrapper, then run their Criterion diagnostics in LTO-on and
  LTO-off profiles. Run all ten maintained ergon/sbe-tool pairs through
  `scripts/check-bench-gate.sh`; every ratio must remain at or below `1.00`.

## T-5: Make instruction-count documentation describe the benchmark that exists

- Type: DOCS
- Stage: 0.1.12
- Priority: P1 · Effort: S
- Symptom: `sbe/BENCHMARKS.md:343-355` and `justfile:276-278` say
  `instruction_counts` uses Iai-Callgrind/Valgrind. The source says Iai was
  removed and is a Criterion harness
  (`sbe/benchmarks/benches/instruction_counts.rs:1-18`); the benchmark manifest
  also calls it a dependency-free `Instant` harness even though it depends on
  Criterion (`sbe/benchmarks/Cargo.toml:17-21`). The current text therefore
  claims stable instruction evidence that cannot be produced by the command.
- Change: describe it consistently as an amplified Criterion timing diagnostic,
  rename it if “instruction_counts” would remain misleading, and document the
  exact estimator/output it provides. If stable instruction counts are still a
  release requirement, create a separate future tool-backed lane rather than
  describing one that is absent.
- What breaks (API only), what it buys: no codec API break; a renamed `just`
  target or bench binary may affect developer commands. Readers can reproduce
  the stated evidence and will not mistake nanosecond timing for instruction
  counts.
- Acceptance criteria: the source header, Cargo comment, `justfile`,
  `sbe/BENCHMARKS.md`, and the book benchmark methodology use the same name,
  tool, and claim; every documented command runs on its stated platform.
- Verification plan: run the documented diagnostic command and docs validation;
  this ticket does not alter generated code, so the benchmark gate should be
  byte-for-byte unaffected.

## T-6: Expose raw enum discriminants consistently

- Type: API
- Stage: 0.1.12
- Priority: P1 · Effort: S
- Symptom: typed enum decoding maps every unknown wire discriminant to
  `NullVal` (`sbe/src/codegen/runtime.rs:810-829`). Message enum getters and
  composite member getters expose only that lossy typed value
  (`sbe/src/codegen/message_decoder.rs:933-964`,
  `sbe/src/codegen/runtime.rs:1187-1208`,
  `sbe/src/codegen/runtime.rs:1436-1454`), while group entries already expose
  `raw_<field>` (`sbe/src/codegen/group_decoder.rs:835-883`). A forward enum
  value is therefore indistinguishable from the null sentinel outside groups.
- Change: extend the existing group pattern to message fields and composite
  value/flyweight members: emit `raw_<field>() -> EncodingType` (or
  `Option<EncodingType>` when the field is not present in the acting version)
  alongside the typed getter. Keep the typed enum and wire representation
  unchanged. Raw access is the SBE-compatible escape hatch for forward schema
  values; it is strictly safer than silently treating “new value” as null.
- What breaks (API only), what it buys: additive unless a schema already
  collides with the reserved `raw_` name; route such collisions through the
  existing name resolver. Users can log, relay, or version-negotiate unknown
  enum values without allocation and without losing the original byte.
- Acceptance criteria: add message, composite value, composite flyweight, group,
  optional, since-version, and big-endian tests that inject an unknown raw value
  and recover it exactly. Typed getters must retain current wire-compatible
  behavior. Update generated rustdoc with the typed-vs-raw distinction.
- Verification plan: run enum/set/composite edge tests and both wire-parity
  suites. Inspect scalar decode assembly to confirm existing typed getters are
  unchanged; raw getters are paid for only when called. Run both benchmark gate
  profiles.

# 2. Main tickets

## T-7: Make every safe constructor prove the unchecked-access extent

- Type: API
- Stage: 0.1.12
- Priority: P0 · Effort: M
- Symptom: the new bare encoder constructors are safe and skip extent checks
  (`sbe/src/codegen/message_encoder.rs:349-458`), but primitive setters use
  `get_unchecked_mut` (`sbe/src/codegen/message_encoder.rs:575-665`). Decoder
  `wrap` and `decode` are also safe zero-check constructors
  (`sbe/src/codegen/message_decoder.rs:381-425`,
  `sbe/src/codegen/message_decoder.rs:500-561`) while fixed accessors use
  `read_bytes_unchecked` (`sbe/src/codegen/message_decoder.rs:709-1029`).
  `AnyMessage::decode` performs an unchecked header read from a safe function
  (`sbe/src/codegen/runtime.rs:1936-1969`), and domain materialisation selects
  bare `decode` (`sbe/src/codegen/domain_cluster.rs:678-693`). Public group and
  entry helpers are documented as private safety-boundary functions but emitted
  as safe `pub fn` (`sbe/src/codegen/group_decoder.rs:165-188`,
  `sbe/src/codegen/group_decoder.rs:518-572`,
  `sbe/src/codegen/group_encoder.rs:400-419`). A short slice can therefore reach
  undefined behavior through entirely safe user code. Official sbe-tool's
  corresponding accessors use safe slice indexing and panic on an invalid
  extent (`sbe/tests/sbe_tool_reference/baseline/src/car_codec.rs:55-154`,
  `sbe/tests/sbe_tool_reference/baseline/src/car_codec.rs:694-760`).
- Change: restore a two-lane trust boundary. Unsuffixed public constructors are
  the safe, one-check lane and return `Result`: `Encoder::wrap`,
  `Encoder::wrap_and_apply_header`, `Decoder::wrap`, `Decoder::decode`, and
  `AnyMessage::decode`. The zero-check lane is explicitly
  `unsafe fn *_unchecked`, with the complete extent precondition in rustdoc.
  Remove redundant public `try_*` aliases in this breaking release. Make group
  dimension/entry zero-check wraps private `unsafe fn` (or `pub(crate) unsafe`
  only where generated module structure requires it). Do not add any per-field
  bounds check: the single constructor proof continues to justify branch-free
  accessors.
- What breaks (API only), what it buys: every current `try_wrap*`/`try_decode`
  call moves to the unsuffixed name; every intentional zero-check call moves to
  an explicit `unsafe { *_unchecked(...) }`. This is wide but mechanical. It
  makes the trust transfer reviewable in Rust source and removes safe-code UB
  while preserving the same unchecked HFT machine code.
- Acceptance criteria: hostile tests call at least one fixed accessor/setter
  after **every** safe constructor on short, overflowing, header-only, and
  version-short buffers and observe `Err`, never panic or UB. Generated-source
  tests reject a safe public function carrying a `# Safety` contract. Domain
  and dispatch entry points use the safe lane. Private group/entry wraps are
  invoked only from proven extents. Regenerate goldens and add a complete
  0.1.11→0.1.12 rename table. Wire parity is unchanged.
- Verification plan: run the HFT soundness/hostile/Miri fixtures, targeted
  versioning tests, all parity/golden tests, and allocation checks. Maintained
  benchmark arms that intentionally compare sbe-tool's lean wrap must call the
  explicit unsafe lane so the timed work remains identical; run both LTO gate
  profiles and compare entry-point and encode assembly. No maintained ratio may
  exceed `1.00`.

## T-8: Finish the fixed and complete-stage API so partial frames cannot look complete

- Type: API
- Stage: 0.1.12
- Priority: P1 · Effort: L
- Symptom: initial encoder metadata exposes `as_body_bytes` and
  `as_bytes_with_header` for dynamic messages while `pos` is only the end of the
  fixed block (`sbe/src/codegen/message_encoder.rs:882-923`). Initial decoder
  metadata gives the fixed block the complete-looking names `as_body_bytes`,
  `as_bytes_with_header`, and `remaining`
  (`sbe/src/codegen/message_decoder.rs:1505-1544`), while the initial decoder
  also performs full-tail scans through complete-message length/byte methods
  (`sbe/src/codegen/message_decoder.rs:1324-1348`). True terminal-stage views
  already exist (`sbe/src/codegen/tail_stages.rs:370-405`). On encode, `fixed()`
  returns the same initial type and tail transitions can be called without any
  fixed-phase transition (`sbe/src/codegen/message_encoder.rs:824-834`). The
  attempted `RawFixedWriter` has no setters or way back to the encoder, yet
  consumes it (`sbe/src/codegen/message_encoder.rs:837-865`). A user can publish
  a truncated “frame” or accidentally skip the fixed phase despite the type-state
  design.
- Change: make the fixed phase an explicit consuming stage. The target shape is
  `Encoder -> AfterFixed -> ... -> Complete`: `fixed(self, &FixedFields)` returns
  `AfterFixed`; a closure- or owned-body-view alternative exposes the existing
  random-access fixed setters and consumes back into `AfterFixed` without heap
  allocation. Repair or replace `RawFixedWriter` with that usable zero-cost body
  view. Tail transitions exist only on `AfterFixed`. For fixed-only messages,
  `AfterFixed` is the complete stage. Keep placement/version facts on initial
  metadata; if partial slices remain, name them `fixed_block_bytes`,
  `header_and_fixed_block_bytes`, and `remaining_after_fixed_block`. Reserve
  `encoded_length*`, `as_body_bytes`, `as_bytes_with_header`, and post-message
  `remaining` for fixed-complete or terminal complete stages. This follows SBE's
  positional fixed-block-then-tail grammar and preserves random-access fixed
  writes without pretending they are a complete message.
- What breaks (API only), what it buys: direct fixed setters on the initial
  encoder move behind the body-view transition; tail calls require an explicit
  fixed-phase completion; partial metadata names change or disappear; initial
  decoder full scans move to a verifier/complete-stage API. Users gain compile-time
  proof that a returned full-frame slice has passed every required wire phase.
- Acceptance criteria: compile-fail tests reject a tail transition before the
  fixed phase and reject full-frame bytes/lengths on incomplete stages. Compile
  tests cover both exhaustive `FixedFields` and random-access body-view styles.
  Fixed fields remain writable in any order; complete methods remain available
  after the final tail and on fixed-only messages. Remove literal `#fixed_name`
  from generated rustdoc and update goldens, samples, AnyMessage integration,
  and migration docs. Type sizes and allocation counts do not grow.
- Verification plan: run type-state compile-fail/size/Send tests, exact-length
  tests, all samples, and byte-parity suites. Compare LTO/no-LTO assembly for
  both fixed styles and all maintained encode/decode gate scenarios. Stage moves
  must compile away, generated hot paths stay allocation-free, and every ratio
  remains at or below `1.00`.

## T-9: Restrict bulk group decode to version-stable entry layouts

- Type: CORRECTNESS
- Stage: 0.1.12
- Priority: P0 · Effort: M
- Symptom: `bulk_decode_into` validates and advances by the wire
  `acting_block_length`, but materialises every latest-schema field
  unconditionally (`sbe/src/codegen/group_decoder.rs:201-309`). Its output is
  the encoder's concrete entry struct, whose fields are never `Option`
  (`sbe/src/codegen/group_encoder.rs:230-306`). Older-version fixtures contain
  shorter group blocks with `sinceVersion` fields
  (`sbe/tests/fixtures/schemas/group-versioning-v2.xml:24-31`,
  `sbe/tests/fixtures/schemas/group-versioned-types-schema.xml:33-42`). Ordinary
  entry accessors correctly test acting version and complete field extent
  (`sbe/tests/schema_edge_cases_test.rs:581-729`), but bulk decode can read a
  later field from the next entry or panic at the end of the group.
- Change: emit `bulk_decode`/`bulk_decode_into` only when every non-constant
  materialised field is `sinceVersion=0`, non-optional, and wholly within the
  minimum supported entry block. Reuse the conservative eligibility pattern
  already used for domain bulk encode
  (`sbe/src/codegen/domain_cluster.rs:58-79`). Versioned/optional groups use the
  canonical iterator, whose accessors represent absence. A future distinct
  `DecodedEntry` with `Option` fields is acceptable, but do not return the
  latest encoder entry struct with invented values.
- What breaks (API only), what it buys: bulk methods disappear for schemas where
  they were semantically incapable of representing older wire entries. Stable
  flat groups keep the same fast API. Users get version-aware behavior instead
  of cross-entry reads, panics, or fabricated latest-version values.
- Acceptance criteria: generated-source tests assert absence of bulk methods for
  optional/versioned layouts and presence for stable flat layouts. Add runtime
  tests that decode the existing version-0 frames with the version-1/2 schemas,
  including two entries chosen to expose cross-entry reads. Preserve current
  multi-byte-array bulk tests (`sbe/tests/java_parity_features_test.rs:739-779`).
- Verification plan: run schema-edge/versioning, hostile-input, bulk group,
  golden, and wire-parity tests. Benchmark eligible 1,000-entry bulk decode
  before/after in both profiles; eligibility checks occur at generation time, so
  eligible assembly and latency must be unchanged. Run the maintained gate.

## T-10: Make boolean enum conversion preserve `NullVal`

- Type: API
- Stage: 0.1.12
- Priority: P1 · Effort: M
- Symptom: `impl From<BooleanType> for bool` implements “raw != 0”, so the SBE
  null sentinel (commonly 255) becomes `true`
  (`sbe/src/codegen/runtime.rs:753-768`). Message and group `_bool` getters repeat
  that collapse (`sbe/src/codegen/message_decoder.rs:949-973`,
  `sbe/src/codegen/group_decoder.rs:887-905`), and domain conversion/DTO code
  materialises plain `bool` (`sbe/src/codegen/conversion_traits.rs:38-66`,
  `sbe/src/codegen/domain_cluster.rs:425-439`). A test currently codifies the
  footgun (`sbe/tests/comprehensive_test.rs:288-318`). Official Rust preserves
  the enum and does not supply this lossy reverse conversion
  (`sbe/tests/sbe_tool_reference/baseline/src/boolean_type.rs:1-29`).
- Change: keep `From<bool> for BooleanType`; remove the infallible reverse
  `From`. Generate `BooleanType::as_bool() -> Option<bool>` and a fallible
  `TryFrom<BooleanType> for bool` (or an equivalent generated error). Required
  field/domain helpers use `try_<field>_bool() -> Result<bool, DecodeError>` and
  reject `NullVal`/unknown; optional or absent-version surfaces retain absence
  rather than converting it to either boolean. DTO materialisation either keeps
  the wire enum by default or fails on null when an explicit bool domain mapping
  requested plain `bool`. This reflects SBE's tri-state wire domain without
  allocating or changing one byte.
- What breaks (API only), what it buys: `bool::from(wire_enum)`, infallible
  `_bool()` getters, and DTOs that silently accepted null require explicit
  handling. Valid F/T values stay ergonomic. Invalid, null, and forward values
  can no longer become a business `true`.
- Acceptance criteria: tests cover F, T, `NullVal`, an unknown raw discriminant,
  required/optional/since-version message fields, group entries, composites,
  configured bool domain types, and DTO materialisation. Raw and typed enum
  access remains available; all valid F/T encodings remain byte-identical.
- Verification plan: run comprehensive/domain/versioning and both wire-parity
  suites. Inspect valid-value scalar assembly: the fallible helper should be a
  small compare/branch only when called, while typed enum access stays unchanged.
  Run both gate profiles; no maintained scenario may regress.

## T-11: Make text helpers public, encoding-aware, and symmetric

- Type: API
- Stage: 0.1.12
- Priority: P1 · Effort: M
- Symptom: the non-consuming `*_as_str` message accessor is generated as private
  `fn` (`sbe/src/codegen/message_decoder.rs:1264-1279`), while the public unsafe
  variant is emitted without the same character-encoding gate
  (`sbe/src/codegen/message_decoder.rs:1281-1300`). Consuming tail stages gate on
  UTF-8/ASCII but validate both with only `from_utf8`
  (`sbe/src/codegen/tail_stages.rs:222-272`), so bytes above 0x7f pass an ASCII
  schema. `DecodeError::InvalidAscii` exists but is unused
  (`sbe/src/codegen/runtime.rs:14-40`). Raw bytes remain the only clear encoder
  API for var-data text.
- Change: for schema `characterEncoding` UTF-8/UTF8 or ASCII/US-ASCII, emit public
  checked non-consuming and consuming string accessors; do not emit string
  helpers for binary/unknown encodings. UTF-8 uses `from_utf8`; ASCII first
  requires `is_ascii` and returns `InvalidAscii`. Add allocation-free encoder
  `*_str(&str)` aliases for declared text; ASCII encoding rejects non-ASCII with
  a matching encode error, while UTF-8 writes `str::as_bytes()`. Keep raw byte
  APIs canonical and unchanged. Official Java exposes character-encoding-aware
  string methods; this Rust shape is safer than the official Rust baseline at
  negligible cost when unused.
- What breaks (API only), what it buys: unsafe string helpers disappear for
  binary fields; ASCII callers that relied on UTF-8-but-not-ASCII data now get an
  error. Valid text gains a discoverable, symmetric, zero-allocation API without
  manual UTF-8 boilerplate.
- Acceptance criteria: golden/rustdoc tests cover UTF-8, ASCII, binary, and an
  unknown encoding at message and group-entry levels. Runtime tests cover valid
  UTF-8, invalid UTF-8, ASCII, non-ASCII UTF-8 under ASCII, empty/max-length
  strings, and raw-byte round trips. Checked helpers are public; unsafe helpers
  state both extent and encoding preconditions.
- Verification plan: run conformance, schema-edge, var-data hostile, docs, and
  wire-parity suites. Confirm existing byte access and maintained full-message
  traversal assembly are unchanged. Benchmark checked UTF-8 and ASCII helpers as
  diagnostics; any scan cost must appear only when a string helper is called,
  and all maintained gate ratios stay at or below `1.00`.

## T-12: Rename group random access so it cannot masquerade as `Iterator::nth`

- Type: API
- Stage: 0.1.12
- Priority: P1 · Effort: S
- Symptom: generated group decoders have an inherent `nth(&self, idx)` that
  indexes from the beginning without consuming the iterator. It is O(1) for
  fixed entries (`sbe/src/codegen/group_decoder.rs:356-402`) but silently scans
  from the group start for entries with nested tails
  (`sbe/src/codegen/group_decoder.rs:403-438`). This shadows the familiar
  consuming `Iterator::nth(&mut self, n)` name while having different cursor and
  complexity semantics; comments claim dynamic entries must be walked but do
  not make that cost visible at the call site.
- Change: rename fixed-stride random access to `entry_at(&self, idx)`. Omit
  random access for dynamic entries, or expose it only as an explicitly costly
  `scan_entry_at(&self, idx)`. Leave `Iterator::nth` and `skip_n` as the
  canonical consuming traversal APIs. SBE repeating groups are positional
  streams; names should reveal whether access is fixed-stride random access or
  a linear tail scan.
- What breaks (API only), what it buys: current inherent `.nth(idx)` calls move
  to `.entry_at`, `.scan_entry_at`, or `Iterator::nth(&mut group, idx)` according
  to intended semantics. Users can no longer accidentally turn a loop into
  quadratic rescanning or assume the cursor advanced when it did not.
- Acceptance criteria: rustdoc states origin, cursor effect, and O(1)/O(n)
  complexity for each API. Tests distinguish random-from-start from
  consume-from-current behavior and cover fixed, var-data, nested, empty, and
  out-of-range groups. Update the book feature tour and samples if they use the
  old name.
- Verification plan: run group iterator/consuming-stage/versioning tests and
  wire parity. This is a name-only change to the retained implementations;
  compare assembly and run both maintained gate profiles to prove no latency
  change.

## T-13: Use an external frame length instead of rescanning every dynamic tail

- Type: PERF
- Stage: 0.1.12
- Priority: P1 · Effort: M
- Symptom: `AnyMessage::decode_frame(buf, pos, frame_len)` already receives an
  authoritative external frame length, yet for every known template it calls
  `decoder.encoded_length_with_header()` and walks all groups/var-data before
  returning (`sbe/src/codegen/runtime.rs:1974-2005`). A caller that then consumes
  the message traverses the dynamic tail twice. `FrameCursor` calls this path for
  every externally framed message (`sbe/src/codegen/runtime.rs:1840-1864`).
- Change: make the fast `decode_frame` treat `frame_len` as the authoritative
  transport boundary: overflow-check and slice the input to exactly that frame,
  validate header plus version-aware fixed extent once, and return a decoder
  bounded to that slice without scanning tails. Define decoder metadata inside
  this API as frame-local and retain the original absolute range on
  `DecodedFrame`. Provide a separately named `decode_frame_verified` (or use the
  existing `verify`) when the caller wants a full structural tail walk before
  access. SBE carries no total message length; when Aeron or a length prefix
  supplies one, rediscovering it by parsing is redundant work.
- What breaks (API only), what it buys: `DecodedFrame.len`/range semantics become
  the external frame length for known templates, and decoder-local offsets may
  be frame-relative; callers needing the current “actual SBE tail length” scan
  use the verified API. Mechanism: remove one O(entries + var-data) pre-scan and
  its length-header reads/branches per frame; no allocation and no per-field
  validation are added.
- Acceptance criteria: tests cover exact, too-short, oversized/overflowing,
  padded, concatenated, unknown-template, nested-group, and malformed-tail
  frames. The fast decoder cannot read into the next frame because its borrowed
  slice ends at `frame_len`. The verified API detects malformed tails and
  reports actual structural length. Add rustdoc explaining external versus SBE
  length ownership.
- Verification plan: add Criterion cases for a fixed message and a large
  dynamic message (for example 1,000 flat entries and a nested/var-data shape),
  measuring `decode_frame` alone and decode-plus-consume. Prove the mechanism
  with assembly/instruction evidence: the fast path contains no generated
  tail-scan loop, while verified does. Run allocation tests, hostile fuzz seeds,
  and the full LTO/no-LTO maintained gate; every maintained ratio stays at or
  below `1.00` and the dynamic frame diagnostic must improve materially.

## T-14: Publish one accurate 0.1.12 API and trust-boundary story

- Type: DOCS
- Stage: 0.1.12
- Priority: P0 · Effort: M
- Symptom: crate rustdoc still says bare constructors are checked `Result`s and
  zero-check cores are private (`sbe/src/lib.rs:29-37`,
  `sbe/src/lib.rs:198-205`), and the crate README repeats the obsolete
  checked-unsuffixed surface (`sbe/README.md:35`, `sbe/README.md:64`). The core
  trust chapter instead advertises a safe
  panic-only bare tier (`book/src/sbe/core-concepts/trust-boundary.md:3-21`,
  `book/src/sbe/core-concepts/trust-boundary.md:45-59`), while the feature-tour
  page documents the older checked-unsuffixed API
  (`book/src/sbe/feature-tour/trust-boundaries.md:11-21`). The migration chapter
  mixes those surfaces and documents the no-op companions option
  (`book/src/sbe/getting-started/from-sbe-tool.md:74-104`); the feature matrix
  still labels checked constructors as 0.1.10 behavior
  (`book/src/sbe/design-notes/feature-matrix.md:14`). Generated-code docs present
  complete-looking initial-stage metadata
  (`book/src/sbe/feature-tour/generated-code.md:66-99`). The golden-path sample
  calls checked `try_*` methods while saying no bounds check is needed
  (`samples/sbe-feature-tour/src/lib.rs:61-62`,
  `samples/sbe-feature-tour/src/lib.rs:124-125`), and `samples/README.md:94-103`
  incorrectly says invalid UTF-8 becomes an empty string. Finally,
  `CHANGELOG.md:3-5` has no 0.1.12 record despite wide breaking changes.
- Change: after T-1 through T-13 settle the API, make the crate rustdoc, README,
  book, sample comments, and changelog agree on exactly two trust lanes, stage
  completeness, acting-version behavior, text encoding, enum/null handling,
  and benchmark selection. Put the 0.1.11→0.1.12 rename table in the book's
  “Coming from sbe-tool” chapter or a single linked migration page, not in
  multiple contradictory sources. Replace the current link to the absent
  `docs/SBE_COMPATIBILITY.md` at
  `book/src/sbe/getting-started/from-sbe-tool.md:3-9` with the maintained
  compatibility/profile location. Add a full `[0.1.12]` changelog section with
  Breaking, Added, Fixed, Performance, and Migration subsections. Every unsafe
  item gets a precise `# Safety`; every public stage/metadata item states whether
  it represents fixed-only, complete body, or header-inclusive frame bytes.
- What breaks (API only), what it buys: no additional API break beyond the
  tickets it documents. A new user sees one compilable constructor pattern,
  understands where bounds are proved, and cannot mistake a fixed block for a
  complete SBE frame or `NullVal` for `true`.
- Acceptance criteria: update the exact locations above plus README constructor,
  buffer-sizing, dispatch, and text sections; ensure `book/src/SUMMARY.md` links
  the canonical migration/trust pages. All examples use generated APIs from the
  final golden and explain checked versus unsafe benchmark calls. Add docs tests
  that assert signatures/visibility, not only method-name substrings. The
  feature-tour, l3-book, and codegen samples build without stale-API comments.
- Verification plan: run rustdoc with warnings denied, crate doctests,
  `docs_validation_test`, mdBook build/link checks, and service-free sample
  tests. Search for removed names and the false phrases “panic-only trusted”,
  “bad UTF-8 → empty”, and the deleted compatibility path. Documentation changes
  do not alter codec code; the maintained benchmark gate remains unchanged.

# 3. Roadmap cross-check

`book/src/project/road-to-1.0.md:7-28` defines six exit criteria. The tickets map
to them as follows:

| Ticket(s) | Roadmap status | Cross-check |
|---|---|---|
| T-7 | already planned | Directly closes the “no known P0 safety issues” trust-boundary criterion (`road-to-1.0.md:20-22`) and is part of the pre-freeze constructor audit (`:11-13`). |
| T-8, T-12 | already planned at category level | Concrete pending stage/wrap/FixedFields naming and invariant decisions under the API-freeze audit (`:11-13`); the specific defects are new. |
| T-4, T-13 | already planned at gate level | Must preserve the three-minor parity criterion (`:14-17`); the specific inline misses and external-frame pre-scan are new. |
| T-14 | already planned | Implements the migration, trust-boundary, buffer-sizing, and type-state documentation criterion (`:23-25`). |
| T-1, T-2, T-3, T-5, T-6, T-9, T-10, T-11 | new | Not named by the roadmap; these are code-grounded 0.1.12 findings. |
| Every code ticket | already planned gate | Acceptance retains byte-exact parity/goldens under the wire-compatibility criterion (`:18-19`). |

## 1.0-only tickets

None recommended from this review. Every grounded breaking API fix is cheaper
and safer to land in 0.1.12 before the freeze. The external-user/production-pilot
criterion (`book/src/project/road-to-1.0.md:26-28`) remains valid roadmap work,
but it needs external evidence and is not an implementable repository ticket
that this code review can specify.
