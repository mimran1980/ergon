# ergo-sbe 0.1.13 — review tickets

Hand-off for a fresh implementer. Not a review memo: each ticket is actionable
work with evidence, target shape, acceptance, and verification.

**Tree baseline:** branch `feat/0.1.13`, workspace version `0.1.13`, HEAD
`2f3a8b2a` (docs: drop 0.1.x migration guides). Working tree clean at review
time.

**Already landed in this series (do not re-open unless regressed):**

- Three-tier constructors with extent proof on safe paths (`try_*` → Result;
  bare → panic; `*_unchecked` → UB); `AnyMessage::decode` ≡ `try_decode`
- `From<BooleanType> for bool` removed; `InvalidBoolean`; `TryFrom` / `as_bool`
- `EncodeError::BufferTooShort { field }`; `DomainConversionFailed { reason }`
- `DomainVarData::Strings` (hard rename; no `LossyStrings`)
- Encoder metadata partial-frame rename (`as_fixed_region_with_header`)
- `MessageVisitor::visit_unknown` required
- Trust-boundary docs/README/`docs/SBE_COMPATIBILITY.md` (current API only)
- `bulk_decode` `#[inline]`; group rustdoc repair; RFQ EncodedLength samples

**Evidence method:** code + golden (`sbe/tests/golden/car_example.rs`) + book.
Full `just bench` was **not** re-run for this write-up; PERF tickets require
measurement before keep/revert.

---

# 1. Quick wins (0.1.13)

S-effort, high value, no hot-path redesign unless noted. Land first.

## T-1: Add `#[inline]` to `finish_empty` and ragged length builders that still miss it

- Type: PERF
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: `finish_empty` is emitted without `#[inline]`
  (`sbe/src/codegen/encoded_length.rs:394-395`; golden
  `car_example.rs:7002`). Generic `group_ragged` / `var_data` on
  `RaggedEntryBuilder` also lack `#[inline]`
  (`encoded_length.rs:982-999`; golden `:7690`, `:7712`). Surrounding
  length methods already carry `#[inline]` (`:951+`).
- Change: emit `#[inline]` on those public forwarding methods. Mechanism:
  remove surviving call boundaries on pre-encode sizing (runs before every
  encode). Do **not** re-add `#[inline]` on enum `raw`/`from_raw` here —
  see T-9 (measurement-led; prior no-LTO regression risk).
- What breaks / buys: no API break. Nested/ragged sizing avoids out-of-line
  calls in no-LTO consumers.
- Acceptance criteria: golden shows `#[inline]` on `finish_empty`,
  `group_ragged`, `var_data`; `encoded_length_api_test` green.
- Verification plan: `just update-golden`; `just bench` LTO-off + LTO-on —
  maintained ratios ≤ `1.00`. Sizing is not separately gated; bar is no
  encode-path regression.

## T-2: Add `#[inline]` to domain DTO thin methods

- Type: PERF
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: domain helpers lack `#[inline]` while message hot path is
  annotated: golden `try_from_decoder` / `encode` / `encode_into` /
  `length_contribution` / `encoded_length*` around
  `car_example.rs:4421-4862`; emission in
  `sbe/src/codegen/domain_cluster.rs:769+`, `:898+`, `:1019+` (file has
  ~2 `#[inline]` attributes vs many `pub fn`).
- Change: `#[inline]` on small forwarding methods only (`try_from_decoder`
  wrappers, `encode_into`, length queries). Skip huge materialise bodies if
  assembly shows code-size regression.
- What breaks / buys: no API break. DTO snapshot/re-encode loops inline
  across crate boundaries without LTO.
- Acceptance criteria: golden domain section annotated; optional Criterion
  diagnostic on a domain encode path.
- Verification plan: `just bench` full gate ≤ `1.00`; record no-LTO domain
  diagnostic if ratios move.

## T-3: Document `acting_version` / `acting_block_length` on decoders

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: generated accessors have `#[inline]` but **no rustdoc**
  (`sbe/src/codegen/message_decoder.rs:567-574`). Users cannot tell these
  are **wire acting** values (not compiled schema constants) without
  reading design notes.
- Change: emit 1–2 sentence docs: acting values come from the message
  header / wrap args; tail offsets use acting block length; fields with
  `sinceVersion` depend on acting version. Point to book versioning if
  useful.
- What breaks / buys: no API break. Fewer “why is my offset wrong” support
  questions.
- Acceptance criteria: golden shows `///` on both; rustdoc deny-warnings
  still clean for generator crate.
- Verification plan: `just update-golden`;
  `RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps`.

## T-4: `#[inline]` on `AnyMessage::visit`

- Type: PERF
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: golden `sbe/tests/golden/car_example.rs:8087` —
  `pub fn visit<V: MessageVisitor>(...)` has no `#[inline]`. Emission in
  `sbe/src/codegen/runtime.rs` MessageVisitor impl block (~2228). Dispatch
  loops call this once per frame.
- Change: emit `#[inline]` on `visit`. Mechanism: remove a call boundary so
  the enum match monomorphises into the caller's poll loop under no-LTO.
- What breaks / buys: no API break. Thin match can inline into poll loops.
- Acceptance criteria: golden + regenerate; multi-template sample still
  builds.
- Verification plan: `just bench` LTO-off + LTO-on — no dedicated visit
  gate; require no regression on dispatch-related scenarios if present,
  else full gate ratios ≤ `1.00` (evidence: before/after Criterion if a
  dispatch scenario exists).

## T-5: Implement `Error::source` for nested encode/decode errors

- Type: API
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: `EncodeError` wraps `Decode(DecodeError)` and
  `VerifyError` wraps `DecodeError`
  (`sbe/src/codegen/runtime.rs:48-51`, `:94`, VerifyError ~112-140) but
  `impl Error` is empty — no `source()`. `?` chains and `anyhow`/`thiserror`
  consumers lose the inner error.
- Change: implement `fn source(&self) -> Option<&(dyn Error + 'static)>`
  for `EncodeError::Decode` and `VerifyError::DecodeError`. Keep Display
  as today.
- What breaks / buys: no break (additive). Operators can walk causes.
- Acceptance criteria: unit test that `err.source()` is `Some` for a
  constructed wrap; golden Display unchanged.
- Verification plan: `error_validation_test` or small runtime test; cold
  path only — no bench impact.

## T-6: Replace generic `field: "encode"` BufferTooShort labels

- Type: API
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: at least one encode site still uses the useless label
  `field: "encode"` (`sbe/src/codegen/group_encoder.rs:372` region after
  the 0.1.13 field sweep). Display becomes
  `buffer too short for encode: needed …` with no group/field name.
- Change: thread the group or field name (`stringify!` / lit) at every
  remaining generic site; `rg 'field: "encode"' sbe/src` must be empty.
- What breaks / buys: Display text only (exhaustive matches using `..` OK).
  Actionable encode failures.
- Acceptance criteria: `rg 'field: "encode"' sbe/` empty; edge-case tests
  still match with `..`.
- Verification plan: `schema_edge_cases_test` / group tests; no hot path.

## T-7: Document `ItemContext` variant fields (drop `allow(missing_docs)`)

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: public hook enum `ItemContext` is
  `#[allow(missing_docs)]` (`sbe/src/config.rs:206-208`) while the crate
  warns on missing docs. Hook authors get no field docs on docs.rs.
- Change: document each variant and field (`schema`, `name`,
  `template_id`, …). Remove the allow if clean.
- What breaks / buys: no API break. Usable hook API on docs.rs.
- Acceptance criteria: `cargo doc -p ergo-sbe` with `-D warnings` green;
  allow gone or justified per-field.
- Verification plan: rustdoc only.

## T-8: Prefer `try_decode` in feature-matrix / multi-template teaching

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: `book/src/sbe/design-notes/feature-matrix.md:28` lists
  `AnyMessage::decode` as the multi-template entry without mentioning
  `try_decode`. After 0.1.13 they are equivalent, but teaching the
  `try_*` name matches the rest of the trust-boundary story and avoids
  “trusted vs checked” confusion.
- Change: matrix cell → `AnyMessage::try_decode` (and note `decode` is the
  same path today). Align `book/src/sbe/feature-tour/multi-template.md` if
  it only shows bare `decode`.
- What breaks / buys: no API break. Consistent constructor vocabulary.
- Acceptance criteria: matrix + multi-template page use `try_decode` as
  default; one line that bare `decode` is equivalent.
- Verification plan: book build; `docs_validation_test` if it greps names.

---

# 2. Main tickets

## T-9: Measurement-led `#[inline]` on enum `raw` / `from_raw` (or document intentional absence)

- Type: PERF
- Stage: 0.1.13
- Priority: P1 · Effort: M
- Symptom: enums emit `raw` / `from_raw` **without** `#[inline]`
  (`sbe/src/codegen/runtime.rs:851-855`; golden `car_example.rs:363`,
  `:442`, `:499`). Cross-crate enum set/get is on the encode path. Project
  history (0.1.12 review notes / `d7450849`-class work) left these off after
  a measured no-LTO `decode_scalar` regression — **do not blind-add**.
- Change:

  1. Baseline no-LTO `decode_scalar` / `encode_scalar` with current golden.
  2. Add `#[inline]` only if no-LTO improves or is flat within noise.
  3. If still a regression: leave off and add a one-line comment at the
     emission site citing the measured reason so the next review does not
     “fix” it again.

  Mechanism (if kept): cross-crate inlining of discriminant cast.
- What breaks / buys: no API break. Either faster no-LTO enums or permanent
  documented exception.
- Acceptance criteria: Criterion before/after with CI; comment or
  annotation; `just bench` ≤ `1.00`.
- Verification plan: LTO-off primary; LTO-on secondary; amplify if
  sub-nanosecond.

## T-10: `#[must_use]` on consuming decoder stages and `into_*` results

- Type: API
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: encoder stages / EncodedLength builders often carry
  `#[must_use]`; decoder stage structs (`CarDecoderAfter*`,
  `*EntryDecoder`, golden ~3726-3754) and many `into_*` / `finish` APIs do
  not (`sbe/src/codegen/tail_stages.rs:356+` finish/skip have inline but no
  must_use on the **struct**). Dropping a consuming stage silently skips
  tails.
- Change: emit `#[must_use = "…"]` on:

  - all consuming decoder stage structs
  - `into_*` / `finish` / `skip_remaining` when the value is a stage or
    unread payload (if not already covered by Result must_use)

  Match encoder wording quality.
- What breaks / buys: new `unused_must_use` warnings — desired. Catches
  truncated decode walks.
- Acceptance criteria: golden attributes present; intentional ignore in a
  test expects the lint under deny.
- Verification plan: `cargo test -p ergo-sbe`; `hft_005` warning-free
  consumer still green.

## T-11: Fix HFT-008 keep matrix — both arms still call `try_*`

- Type: CORRECTNESS (tests/tooling)
- Stage: 0.1.13
- Priority: P1 · Effort: M
- Symptom: injected probe in
  `sbe/tests/hft_008_checked_unchecked_test.rs:56-100` times
  `try_wrap_and_apply_header` against **itself** under `unsafe { … }` —
  not `wrap_and_apply_header` vs `wrap_and_apply_header_unchecked`. Module
  docs claim a three-tier measurement (`:1-11`) but the matrix does not
  exercise bare or unchecked constructors. Numbers cannot inform keep
  decisions.
- Change: rewrite arms to:

  | Scenario | Checked | Trusted | Unchecked |
  |----------|---------|---------|-----------|
  | WAH | `try_wrap_and_apply_header` | `wrap_and_apply_header` | `unsafe wrap_and_apply_header_unchecked` |
  | body wrap | `try_wrap` | `wrap` | `unsafe wrap_unchecked` |
  | decode | `try_decode` | `decode` | `unsafe decode_unchecked` |

  Exact stack + opaque slice; pre-sized buffers; black_box; emit machine
  lines. Optionally assert checked ≈ trusted (both prove extent) and
  unchecked ≤ checked on success path when measurable.
- What breaks / buys: no product API break. Evidence for whether unchecked
  is worth the unsafe surface.
- Acceptance criteria: probe compiles against public API only; three
  distinct method names in source; CI/test stdout documents methodology.
- Verification plan: run `hft_008_checked_unchecked_test`; no product
  codegen change required for the test-only fix.

## T-12: Teach `apply_nulls` for optional fields (book + rustdoc cross-link)

- Type: DOCS
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: optional fields are not auto-nullified on
  `wrap_and_apply_header` (by design, sbe-tool parity —
  `message_encoder.rs:392-395`, `:490-497`). Stale buffer garbage ships
  unless the user calls `apply_nulls()` or writes every optional. Book
  nullval note explains wire NullVal
  (`book/src/sbe/design-notes/nullval.md`) but **feature tour / encode
  recipes do not show `apply_nulls`** (`rg apply_nulls book/src` → no
  teaching hits).
- Change:

  - Add a short subsection under
    `book/src/sbe/getting-started/encode-decode.md` or
    `book/src/sbe/feature-tour/` with a real `{{#include}}` sample:
    `try_wrap_and_apply_header` → `apply_nulls()` → `fixed(...)` for a
    schema with an optional field.
  - Cross-link from nullval design note.
  - Ensure generated `apply_nulls` rustdoc stays loud (already present).

  Rationale: SBE optional = sentinel on the wire; unchecked buffer contents
  are not “None”.
- What breaks / buys: no API break. Removes silent optional-field garbage
  footgun for new users.
- Acceptance criteria: book page + sample include; `rg apply_nulls book/src`
  hits teaching prose; book CI green.
- Verification plan: book build; optional sample test if new include.

## T-13: Claim path for non-fixed messages (document limit or EncodedLength + claim helper)

- Type: API / DOCS
- Stage: 0.1.13
- Priority: P1 · Effort: M
- Symptom: `wrap_into_claim` is only emitted for fixed-only messages
  (`sbe/src/codegen/message_encoder.rs:458+`, `is_fixed` gate). Car-like
  messages with groups/var-data have **no** claim helper; Aeron
  `try_claim` users must hand-size with EncodedLength then
  `try_wrap_and_apply_header` on a slice. Book
  `book/src/sbe/recipes/aeron-try-claim.md` should make this explicit if it
  only shows fixed `ENCODED_LENGTH` / `wrap_into_claim`.
- Change: pick one (prefer A then B). Rationale: SBE has no self-describing
  total length; claim boundary is transport-owned.

  - **A (docs, S):** document that `wrap_into_claim` is fixed-only; for
    ragged/var-data: EncodedLength → claim exact `len` →
    `try_wrap_and_apply_header(&mut claim[..len], 0)?`. Add worked include
    from feature-tour or l3-book into `book/src/sbe/recipes/aeron-try-claim.md`.
  - **B (API, M):** generate
    `wrap_into_claim(buf, precomputed_len)` that requires
    `buf.len() == precomputed_len` and returns
    `ClaimLengthMismatch` otherwise — for use after EncodedLength.
- What breaks / buys: B is additive. A removes wrong assumption that claim
  API always exists. Users stop oversizing claims.
- Acceptance criteria: recipe accurate for both fixed and Car-shaped
  messages; if B, golden + tests for exact/short/long.
- Verification plan: sample/recipe compile; if B, wire parity unchanged;
  benches unaffected (claim setup outside timed path).

## T-14: Narrow generated module `#[allow(...)]` list

- Type: CORRECTNESS
- Stage: 0.1.13
- Priority: P2 · Effort: M
- Symptom: every generated module starts with a broad allow list (golden
  `car_example.rs:2-16`): `unused_unsafe`, `unused_imports`,
  `clipy::identity_op`, etc. That hides real generator bugs (dead
  accessors, unnecessary `unsafe` blocks).
- Change: burn down allows one at a time. Prefer fixing emission (e.g.
  only emit `unsafe` when calling unsafe fns; drop unused imports) over
  silencing. Keep only allows that schema reality forces (e.g. absurd
  comparisons on schema constants) with a short comment in codegen.
- What breaks / buys: no user API break. Generator quality signal returns.
- Acceptance criteria: golden allow list shorter; `cargo test` consumer
  crates clean; list of remaining allows documented in codegen module
  rustdoc.
- Verification plan: full `cargo test -p ergo-sbe`; golden regen is the
  review payload.

## T-15: Decoder metadata / stage completeness for mid-decode views

- Type: API
- Stage: 0.1.13
- Priority: P2 · Effort: M
- Symptom: encoder partial-frame rename landed; decoder still mixes complete-
  sounding names with mid-walk stages. Evidence in golden:

  | Location | API | Shape |
  |----------|-----|--------|
  | `sbe/tests/golden/car_example.rs:1173` | `CarDecoderMetadata::as_bytes_with_header` | `Result` (walks tails?) |
  | `sbe/tests/golden/car_example.rs:1200` | `CarDecoder::get_metadata` | initial stage only |
  | `sbe/tests/golden/car_example.rs:2852` | entry `as_bytes_with_header` | bare `&[u8]` |
  | `sbe/tests/golden/car_example.rs:3706` | group view `as_bytes_with_header` | bare `&[u8]` |
  | `sbe/tests/golden/car_example.rs:3726` | `CarDecoderAfterFuelFigures` | intermediate stage |
  | `sbe/tests/golden/car_example.rs:4383` | further stage `as_bytes_with_header` | bare `&[u8]` |

  Intermediate stages can sit before manufacturer/model/activationCode are
  consumed; a “with_header” slice name suggests a complete SBE frame.
  **Cannot verify without reading each method body** whether the slice ends
  at `pos` only (partial) or rescans tails — that is the first implementer
  step.
- Change: inventory every `as_bytes_with_header` / `as_body_bytes` emission
  in `message_decoder.rs` + `tail_stages.rs` + `group_decoder.rs`. Restrict
  complete-sounding names to complete stages, or rename partial views (mirror
  encoder `as_fixed_region_with_header`). Prefer `Result` + require
  `finish`/`verify` when claiming a full frame.
- What breaks / buys: callers using mid-decode “full frame” helpers must
  move to complete stage or verify. Prevents treating a partial walk as a
  publishable frame (SBE has no self-describing total length).
- Acceptance criteria: PR table stage → byte API; tests for fixed-only vs
  tailed Car; update
  `book/src/sbe/feature-tour/generated-code.md` if surface renames.
- Verification plan: golden + baseline; wire parity; benches (metadata not
  gated — expect flat).

---

# 3. Roadmap cross-check

Source: `book/src/project/road-to-1.0.md`.

| Ticket(s) | Roadmap status | Notes |
|-----------|----------------|-------|
| T-11 | **already planned** (trust) | Measurement for trust-boundary / unsafe surface (`road-to-1.0.md:20-22`). |
| T-9, T-1, T-2, T-4 | **already planned** (gate) | Parity/performance ceiling for three minors (`:14-17`); specific inline misses are new instances. |
| T-12, T-13 (docs), T-8 | **already planned** (docs) | Buffer sizing / trust / sbe-tool teaching (`:23-25`). |
| T-5, T-6, T-7, T-10, T-14, T-15 | **new** | Error ergonomics, must_use, allow burn-down, stage completeness. |
| Every codec ticket | **already planned** (gate) | Wire compatibility (`:18-19`) — keep parity/goldens green. |
| External pilot | **already planned** | `:26-28` — see 1.0-only. |

---

# 4. 1.0-only (do not block 0.1.13)

## T-100: External signal for disclaimer exit

- Type: DOCS
- Stage: 1.0
- Priority: P1 · Effort: L
- Symptom: `book/src/project/road-to-1.0.md:26-28` requires an external user
  or published case study before lifting the production disclaimer.
- Change: pilot write-up (wire + latency on a real schema) linked from root
  `README.md` Documentation section and `book/src/project/road-to-1.0.md`.
- What breaks / buys: no code break. Unblocks 1.0 criterion 6.
- Acceptance criteria: linked case study exists in-repo or is permanently
  linked from README; roadmap section cites it.
- Verification plan: link resolves; no codec change — benches N/A.

## T-101: API freeze audit sign-off

- Type: API
- Stage: 1.0
- Priority: P0 · Effort: L
- Symptom: freeze decisions live in
  `book/src/sbe/design-notes/api-freeze.md:1-68` but 0.1.x still churns
  constructors and metadata (this branch alone).
- Change: after 0.1.13 tickets settle, complete freeze checklist; renames only
  with a major. Golden `sbe/tests/golden/car_example.rs` is the freeze
  artifact.
- What breaks / buys: post-freeze renames are majors — users get stability.
- Acceptance criteria: freeze checklist checked in `api-freeze.md` or release
  notes; no open rename tickets for stage/wrap/FixedFields.
- Verification plan: `just check-golden`; release process `/release` gate.

## T-102: Cluster deferred config (`is_ingress_exclusive`, `owns_aeron`)

- Type: API
- Stage: 1.0
- Priority: P2 · Effort: L
- Symptom: `cluster/src/config.rs:119-127` and `:200-205` reject shared
  ingress and external Aeron injection as “not yet supported.”
- Change: implement both modes or document them as permanent non-goals for
  cluster 1.0; Aeron/rusteron version matrix; `just bench-cluster` baselines
  (`road-to-1.0.md:33-42`).
- What breaks / buys: either new capabilities or honest non-goals — users
  stop reading “future release” forever.
- Acceptance criteria: multi-node harness green **or** docs state non-goal;
  matrix page under `book/src/cluster/`.
- Verification plan: cluster tests + `just bench-cluster` if code changes.

## T-103: Three consecutive minors at `1.00` parity gate

- Type: PERF
- Stage: 1.0
- Priority: P0 · Effort: L
- Symptom: roadmap requires three consecutive released minors at the `1.00`
  ceiling (`book/src/project/road-to-1.0.md:14-17`). **Not verified here**
  whether 0.1.10–0.1.12 already have recorded Criterion artifacts.
- Change: publish Criterion gate output in release notes or CI artifacts for
  each minor; track streak explicitly. Mechanism: same work arms, LTO matrix,
  no ceiling raises.
- What breaks / buys: no API break. Evidence for 1.0 criterion 2.
- Acceptance criteria: three tags (or three release notes) with green
  `just bench` / `check-bench-gate` evidence.
- Verification plan: archive gate logs per release; do not claim streak
  without artifacts.

---

# Reading order for implementers

1. `Claude.md` / project invariants (wire + bench fairness).
2. Quick wins T-1…T-8 (parallelisable).
3. T-11 (test honesty) and T-12 (optional-field footgun) before more API
   surface.
4. T-9 only with benches in hand.
5. T-13/T-15 if claim/stage completeness bites users.
6. Any `sbe/` hot-path change: `just bench` before keep.

# Out of scope / not verified here

- Full `just bench` numbers on this host (not run for this write-up).
- Whether three released minors already meet the parity streak (T-103).
- Deep cluster client review beyond deferred config markers.
- Whether every intermediate decoder stage exposes incomplete
  `as_bytes_with_header` (T-15 requires inventory).
