# ergo-sbe 0.1.13 — review tickets

Hand-off for a fresh implementer. Not a review memo: each ticket is actionable
work with evidence, target shape, acceptance, and verification.

**Tree baseline:** branch `feat/0.1.13`, workspace version `0.1.13`. Reconfirm
with `git rev-parse HEAD` + `git status` before implementing — this list was
grounded against a working tree that already includes placement-on-metadata
and shared reserved lists in `conversion_helpers.rs`.

**Already landed — do not re-open unless regressed (spot-check golden):**

- Three-tier constructors (`try_*` / bare / `*_unchecked`); bare encoder/decoder
  wrap direct extent check; hybrid bare `decode` / `decode_unchecked`
  (extent panic, identity `Err`)
- Placement utils **only** on `{Name}DecoderMetadata` /
  `{Name}EncoderMetadata` via `get_metadata()` — field names
  `remaining` / `buffer` / `limit` / `message_offset` keep natural accessors
  (`sbe/src/codegen/conversion_helpers.rs:74-145`,
  `sbe/tests/reserved_name_clash_test.rs`)
- **Single** `DECODER_RESERVED` / `ENCODER_RESERVED` source of truth in
  `conversion_helpers.rs` (no local copies in decoder/encoder/display);
  placement names enforced absent from reserved lists by unit + baseline tests
- `#[inline]` on bulk_decode, finish_empty / ragged builders, domain thin
  methods, EncodedLength staged group/var-data/complete length getters,
  `try_from_slice_with_header`
- `#[must_use]` on consuming decoder stages + encoder stages + entry
  EncodedLength / UniformEncodedLength
- Error variant rustdoc; `Error::source` on nested decode
- Enum `raw` / `from_raw` intentionally without `#[inline]` (measured;
  `sbe/src/codegen/runtime.rs:883-891`)
- Book: trust hybrid tables, decode-stages, metadata limits, apply_nulls,
  claim recipe, FixedFields no-Default, parity-gate archive note in
  `book/src/project/verification.md`
- Bench fairness: batch decode + encode body_only use `wrap_unchecked` vs
  sbe-tool zero-check wrap
- Cluster samples/tests use `get_metadata().remaining()`
  (`cluster/src/fragment.rs:82`, `cluster/src/codecs/tests.rs`)

**Evidence method:** code + golden (`sbe/tests/golden/car_example.rs`) + book.
Any `sbe/` hot-path codegen change requires no-LTO `just bench` gate ≤ `1.00`.

---

# 1. Quick wins (0.1.13)

S-effort, high value, no hot-path redesign. Land first.

## T-1: Add `#[inline]` to EncodedLength zero-count group forwarders

- Type: PERF
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: After a uniform group stage, the zero-count **forwarder** to the
  next group/var-data (e.g. `CarFuelFiguresUniformEncodedLength::performance_figures`,
  `…::manufacturer`) is emitted **without** `#[inline]`. Golden
  `sbe/tests/golden/car_example.rs:7195` `pub fn performance_figures`.
  Emission `sbe/src/codegen/encoded_length.rs:454-456`
  (`pub fn #next_method_name` inside the `if !has_collision` block).
  Surrounding `finish_empty` already has `#[inline]`
  (`sbe/src/codegen/encoded_length.rs:395-396`, golden `:7172-7174`).
- Change: emit `#[inline]` on that forwarding method. **Mechanism:** remove a
  call boundary on the pre-encode sizing path when groups have zero entries
  (common in sparse books). Do **not** re-add `#[inline]` on enum
  `raw`/`from_raw`.
- What breaks / buys: no API break. Nested EncodedLength chains stay
  inlinable without LTO on the zero-count path.
- Acceptance criteria: golden shows `#[inline]` immediately above those
  forwarders; `encoded_length_api_test` green.
- Verification plan: `just update-golden`; no-LTO `just bench` — maintained
  ratios ≤ `1.00` (sizing not separately gated; ensure no accidental
  hot-path churn).

## T-2: Crate README: placement utils only on `get_metadata()`

- Type: DOCS
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: `sbe/README.md` documents three-tier constructors
  (`sbe/README.md:33-34`) and links Feature Tour, but never mentions
  `get_metadata()` or that `remaining` / `buffer` / `limit` /
  `message_offset` are **field-safe** and must be read via metadata.
  Users still find the old `dec.remaining()` pattern from pre-0.1.11
  blogs / muscle memory.
- Change: short paragraph under Constructors or a one-line bullet after
  Feature Tour links: placement → `dec.get_metadata().remaining()`;
  schema field named `remaining` → `dec.remaining()` field accessor. Link
  to book `feature-tour/generated-code.md` metadata section.
- What breaks / buys: no API break. Removes the #1 post-placement footgun
  for crate consumers who never open the book.
- Acceptance criteria: README contains `get_metadata` + migrate one-liner;
  no claim that placement lives on the decoder type itself.
- Verification plan: human read of README; no codegen change; no bench.

## T-3: Rustdoc `into_remaining_mut` on complete encoders

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: Complete encoder stages emit `into_remaining_mut` with only
  `/// Unwritten region after this message.`
  (`sbe/src/codegen/message_encoder.rs:1192-1196` and the fixed-only twin
  at `:1228-1231`). Golden
  `sbe/tests/golden/car_example.rs:6341` shows the thin doc. Cluster
  README shows the intended chaining pattern
  (`cluster/README.md:110-111`) but rustdoc does not.
- Change: expand rustdoc: returns `&mut [u8]` from write cursor to end of
  original buffer; typical use
  `NextEncoder::wrap_and_apply_header(remaining, 0)`; does **not** mean
  “payload of this message.” Cross-link `get_metadata().limit()` for the
  absolute cursor when the encoder must stay live.
- What breaks / buys: no API break. Safer multi-message buffer packing.
- Acceptance criteria: golden shows multi-line `///` on
  `into_remaining_mut`; cluster README still consistent.
- Verification plan: `just update-golden` or check-golden; no bench.

## T-4: Fix stale cluster README `remaining()` prose

- Type: DOCS
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: Prose still says “Use the decoder's `remaining()` to get the
  payload” (`cluster/README.md:92`) while the code sample correctly uses
  `smh.get_metadata().remaining()` (`cluster/README.md:119-120`). Readers
  who skim prose reintroduce the removed API.
- Change: rewrite line 92 to `get_metadata().remaining()`; keep the code
  sample as the source of truth.
- What breaks / buys: no API break. Aligns advanced session framing docs
  with 0.1.11+ placement.
- Acceptance criteria: zero bare “decoder's `remaining()`” claims for
  message-level byte tails in `cluster/README.md`; sample still compiles
  as `no_run`.
- Verification plan: grep `cluster/README.md` for `remaining()`; ensure
  message-level uses are `get_metadata().remaining()`.

## T-5: Rustdoc group `remaining()` is entry count

- Type: DOCS
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: Group iterators emit `remaining()` as **entry count** (`usize`)
  with **no** rustdoc (`sbe/src/codegen/group_decoder.rs:167-169`). Golden
  `sbe/tests/golden/car_example.rs:2445` `pub const fn remaining(&self) -> usize`.
  Call sites correctly use it as capacity
  (`samples/cluster-ha-orderbook/src/follower.rs:102`). Easy to confuse
  with metadata `remaining() -> &[u8]`.
- Change: emit `/// Entries not yet advanced (count), not a byte slice.` on
  group `remaining()`. Optionally one line in
  `book/src/sbe/feature-tour/generated-code.md` under metadata.
- What breaks / buys: no API break. Stops `meta.remaining()` /
  `group.remaining()` mix-ups at the hover level.
- Acceptance criteria: golden shows the rustdoc; group accessors still
  return `usize`.
- Verification plan: `just update-golden`; no bench.

## T-6: `#[must_use]` on EncodedLength After/Complete stages

- Type: API
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: Entry EncodedLength and Uniform stages carry `#[must_use]`
  (`sbe/src/codegen/encoded_length.rs:80`, `:237`; golden
  `sbe/tests/golden/car_example.rs:7132-7133`). Intermediate
  `*EncodedLengthAfter*` and `*EncodedLengthComplete` structs are emitted
  **without** `#[must_use]` (`sbe/src/codegen/encoded_length.rs:188-193`;
  golden `sbe/tests/golden/car_example.rs:7111-7129`). Dropping a partial
  length builder after the first group silently yields a useless stage.
- Change: add `#[must_use = "length builder must be completed"]` (or
  similar) on every After/Complete EncodedLength stage struct. Rationale:
  same type-state “consume the stage” pattern as encoder/decoder stages;
  sizing chains must reach `encoded_length_with_header()`.
- What breaks / buys: new `unused_must_use` warnings at call sites that
  already drop builders mid-chain (correctness signal). No wire change.
- Acceptance criteria: golden shows `#[must_use]` on
  `CarEncodedLengthAfter*` / `Complete`; no new clippy allows to hide it.
- Verification plan: `just update-golden`; `cargo test -p ergo-sbe
  --test encoded_length_api_test`; no-LTO bench only if emission order
  changes (should not).

---

# 2. Main tickets

Slightly larger or doc-surface tickets; still 0.1.13-biased. Order by
leverage.

## T-7: Feature-matrix row for placement / reserved policy

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: Feature matrix covers three-tier constructors and exact sizing
  (`book/src/sbe/design-notes/feature-matrix.md:14-15`) but has **no** row
  for `get_metadata()` / field-safe placement names (spot check: no
  `get_metadata` hits in that file).
- Change: add a row: **Placement metadata** | Utils on
  `get_metadata()` so field names `remaining`/`buffer`/`limit`/
  `message_offset` stay natural | link
  `feature-tour/generated-code.md` + `reserved_name_clash_test`.
- What breaks / buys: no API break. Scannable matrix matches the product.
- Acceptance criteria: matrix contains `get_metadata` and names the four
  placement fields; link resolves.
- Verification plan: open matrix in book build or markdown preview; no
  bench.

## T-8: Document group vs metadata `remaining()` side-by-side

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: `generated-code.md` documents metadata
  `remaining() -> &[u8]` (`book/src/sbe/feature-tour/generated-code.md:112`)
  and field-safe names (`:70-78`) but does not put group iterator
  `remaining() -> usize` in the same table. Decode-stages chapter covers
  `skip_remaining` (`book/src/sbe/feature-tour/decode-stages.md:28-29`)
  without the count vs bytes distinction.
- Change: table in `generated-code.md`:

  | Receiver | `remaining()` means |
  |----------|---------------------|
  | Group decoder | entry count (`usize`) |
  | `get_metadata()` | bytes after acting fixed block (`&[u8]`) |
  | Schema field named `remaining` | field accessor (natural name) |

  Point session framing to `get_metadata().remaining()`.
- What breaks / buys: no API break. Complements T-5 rustdoc.
- Acceptance criteria: table present; no instruction to call
  message-level `dec.remaining()` for bytes when that name is a field.
- Verification plan: book mdbook build optional; grep for contradictory
  prose.

## T-9: Enforce reserved lists match **emitted** inherent methods

- Type: CORRECTNESS
- Stage: 0.1.13
- Priority: P2 · Effort: M
- Symptom: Shared lists in
  `sbe/src/codegen/conversion_helpers.rs:81-134` and
  `PLACEMENT_NOT_RESERVED` tests (`:139-145`, unit tests `:168+`) prevent
  re-reserving placement names, but nothing asserts every reserved name is
  actually emitted as an inherent method on `CarEncoder` / `CarDecoder`.
  Historical lag produced the stale `"header"` entry (never emitted; now
  removed). `baseline_test::reserved_name_lists_have_no_duplicates` only
  checks uniqueness + placement absence.
- Change: generate Car (or a fixture schema) and parse inherent `pub fn`
  names on the message type (not Metadata, not group stages). Assert
  `reserved ⊆ emitted ∪ known associated constructors`. Fail if reserved
  contains a name that only lives on metadata (e.g. deliberately adding
  `"remaining"`). Optional: assert placement names appear on Metadata.
- What breaks / buys: no user API break. Generator hygiene — future
  placement moves cannot leave zombie reserved renames.
- Acceptance criteria: new test green; deliberately adding `"remaining"`
  to `DECODER_RESERVED` fails the test.
- Verification plan: `cargo test -p ergo-sbe --test reserved_name_clash_test`
  (or new unit test); no bench.

## T-10: Document dual `acting_version` / `acting_block_length` placement

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: Message decoders still emit inherent
  `acting_version` / `acting_block_length`
  (`sbe/src/codegen/message_decoder.rs:572-578`) **and** the same methods
  on metadata (`:1642-1645` region; golden dual at
  `sbe/tests/golden/car_example.rs:1538` and metadata `:1248`). CHANGELOG
  0.1.11 said these “moved” to metadata; Unreleased notes document them on
  decoders. They remain in `DECODER_RESERVED`
  (`conversion_helpers.rs:97-98`), so a schema field `actingVersion`
  becomes `acting_version_field`. Book metadata section does not spell
  the dual surface.
- Change: **Do not remove inherent methods in this ticket** (hot path +
  external tests use them —
  `sbe/tests/sbe_tool_multi_schema_wire_parity_test.rs:153-154`). Document
  in `generated-code.md`: inherent hot-path getters vs
  `get_metadata().acting_version()` equivalence; note reserved rename for
  clashing field names. Optional follow-up 1.0 ticket if dual is deleted.
- What breaks / buys: no API break. Ends changelog/docs contradiction.
- Acceptance criteria: book states both access paths; reserved policy
  mentioned for those two names.
- Verification plan: book/docs only; no bench.

## T-11: Watch encode `body_only` margin under no-LTO

- Type: PERF
- Stage: 0.1.13
- Priority: P2 · Effort: M
- Symptom: Gate enforces strict `1.00` on
  `encode_scalar_body_only` (`scripts/check-bench-gate.sh:65`). Arms use
  fair `wrap_unchecked` / body-only patterns
  (`sbe/benchmarks/benches/perf_parity_bench.rs:577-590`;
  fairness policy `sbe/benchmarks/tests/fairness_policy_test.rs:217-254`).
  **Cannot verify current Criterion estimates in this hand-off** (no fresh
  bench run recorded here). Body-only was historically near the noise
  band after fairness fixes.
- Change: after any encode hot-path edit in 0.1.13, re-run no-LTO
  `just bench` and record `body_only` ergo/sbe-tool ratio + CI in the PR.
  If ratio repeatedly exceeds `1.00 + noise`, **fix codegen or fairness** —
  never raise the ceiling. Mechanism to investigate first: missing
  `#[inline]` on a setter/stage path (compare LTO-off assembly), unequal
  field work, header-mode mix.
- What breaks / buys: protects the hard 1.00 policy; no intentional API
  change.
- Acceptance criteria: PR notes include body_only estimates; gate green.
- Verification plan: `just bench` no-LTO; `scripts/check-bench-gate.sh`
  PASS; attach Criterion regression estimate + CI.

## T-12: Durable `rust,ignore` inventory before each 0.1.13 cut

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: Release skill already mandates manual recheck
  (`.claude/skills/release/SKILL.md:15-20`). Book still has many
  `rust,ignore` fences (highest count
  `book/src/sbe/feature-tour/generated-code.md` — 7 fences). Policy
  checker does **not** fail CI on these. API drift (placement,
  trust tiers) can leave stale snippets after 0.1.13 landings.
- Change: before tagging 0.1.13, inventory every ` ```rust,ignore ` under
  `book/`; open real source for each; fix signature/stage drift or convert
  to `{{#include}}` / bare `rust` / `rust,no_run` when possible. Pay
  special attention to `generated-code.md` metadata/placement examples.
- What breaks / buys: no API break. Keeps book trustworthy for release.
- Acceptance criteria: checklist attached to release notes or PR; no
  known-stale placement/`dec.remaining()` byte-slice examples remain.
- Verification plan: `rg 'rust,ignore' book`; manual open of each hit;
  optional mdbook build.

---

# 3. Roadmap cross-check

Against `book/src/project/road-to-1.0.md` criteria:

| Ticket(s) | Label | Roadmap criterion |
|-----------|-------|-------------------|
| T-1, T-11 | **new** | Perf hygiene under the existing 1.00 parity gate (criterion 2) — not a new roadmap item, but protects the gate. |
| T-2–T-5, T-7–T-8, T-10, T-12 | **new** | Docs completeness (criterion 5: book + README linkage / accuracy). Placement/metadata is post-0.1.11 surface still uneven in README/matrix. |
| T-6 | **new** | Type-state consistency (aligns with api-freeze stage philosophy; not listed as a freeze decision). |
| T-9 | **new** | Generator hygiene; supports long-term freeze stability (criterion 1). |
| T-100 | **already planned** | Criterion 1 — API freeze audit complete (`api-freeze.md`). |
| T-101 | **already planned** | Criterion 2 — three consecutive released minors ≤ 1.00 with recorded Criterion runs. |
| T-102 | **already planned** | Criterion 6 — external user / pilot signal. |
| T-103 | **already planned** | Cluster separate clock (road-to-1.0 “ergo-aeron-cluster” section). |

Wire compatibility (criterion 3) and trust-boundary fuzz/Miri (criterion 4)
are **already enforced in CI** for 0.1.x; no open ticket re-opens them
unless a regression appears.

---

# 4. 1.0-only

Process and freeze items. Do not mix into 0.1.13 code slices unless
explicitly scoped.

## T-100: Complete API freeze audit for 1.0

- Type: API
- Stage: 1.0
- Priority: P1 · Effort: L
- Symptom: `book/src/sbe/design-notes/api-freeze.md` records decisions
  (message-start wrap, exhaustive FixedFields, named stages, three-tier
  unchecked) but road-to-1.0 still requires the freeze audit **complete**
  with no pending renames without a major.
- Change: walk golden `car_example.rs` public surface against api-freeze;
  open issues only for true rename candidates; mark freeze “stable” in
  road-to-1.0 when done.
- What breaks / buys: freezes the generated API contract users depend on.
- Acceptance criteria: api-freeze + road-to-1.0 updated; no open rename
  P0s.
- Verification plan: golden diff review; parity + bench gates green on
  the freeze commit.

## T-101: Three consecutive released minors at ≤ 1.00

- Type: PERF
- Stage: 1.0
- Priority: P1 · Effort: L
- Symptom: road-to-1.0 criterion 2 requires **three consecutive released
  minors** with recorded Criterion runs at ≤ `1.00` under the published
  LTO matrix. Mechanism: maintain fairness policy + no-LTO evidence; never
  raise ceilings.
- Change: publish Criterion artifacts / release-note tables for each minor
  (0.1.11 → 0.1.12 → 0.1.13 as candidates once each is released with
  artifacts).
- What breaks / buys: exit criterion for “not production-ready” disclaimer
  on sbe.
- Acceptance criteria: three released minors with archived ratios; all
  maintained scenarios ≤ 1.00 + noise.
- Verification plan: `just bench` + `check-bench-gate.sh` per release;
  store artifacts per `verification.md`.

## T-102: External pilot / case study

- Type: DOCS
- Stage: 1.0
- Priority: P2 · Effort: L
- Symptom: road-to-1.0 criterion 6 — at least one external user or
  production pilot with wire + latency results, or equivalent published
  case study.
- Change: land a short `book/` or repo case study with schema class,
  latency notes, and wire-parity approach (no customer secrets).
- What breaks / buys: external signal for 1.0.
- Acceptance criteria: linked from road-to-1.0 / README; reproducible
  claims only.
- Verification plan: editorial review; no bench gate change.

## T-103: Cluster 1.0 on a separate clock

- Type: API
- Stage: 1.0
- Priority: P2 · Effort: L
- Symptom: road-to-1.0 states cluster 1.0 is **not** tied to sbe 1.0;
  needs stable session lifecycle, Aeron/rusteron matrix, codecs locked to
  a released sbe major, and `just bench-cluster` baselines.
- Change: track cluster criteria separately; do not block sbe 1.0 on
  cluster 1.0.
- What breaks / buys: clear product sequencing.
- Acceptance criteria: cluster checklist document; sbe 1.0 can ship first.
- Verification plan: multi-node harness + `just bench-cluster` when
  cluster is ready.

---

## Implementation order (suggested)

1. T-1 (inline forwarders) — pure codegen, golden, no API risk.
2. T-4, T-2, T-5, T-3 (docs/rustdoc) — ship with T-1 or alone.
3. T-6 (`must_use` EncodedLength stages) — may surface real drop bugs.
4. T-7, T-8, T-10, T-12 (book/matrix).
5. T-9 (reserved ⊆ emitted) before more reserved-list edits.
6. T-11 on every encode hot-path PR.
7. T-100–T-103 only on the 1.0 track.

## Out of scope for this hand-off

- Raising any bench ceiling.
- Re-opening three-tier trust, hybrid decode, or placement-on-metadata
  design unless a regression is proven.
- Re-duplicating `DECODER_RESERVED` (already centralized).
- Hand-editing `sbe/tests/sbe_tool_reference/**`.
