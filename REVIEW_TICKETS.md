# ergo-sbe 0.1.13 — review tickets

Hand-off for a fresh implementer. Not a review memo: each ticket is
actionable work with evidence, target shape, acceptance, and verification.

**Tree baseline:** branch `feat/0.1.13`, workspace version `0.1.13`, last
released `0.1.12` (`d1a3ecb4`). Uncommitted local work at review time (do not
discard): `book/src/sbe/feature-tour/decode-stages.md`,
`samples/sbe-feature-tour/src/lib.rs`, `sbe/tests/ordered_decoder_stages_test.rs`
(decode-stage lifetime / multi-`&str` coexistence).

**What already shipped in 0.1.12 (do not re-open as new work):**
`ClaimLengthMismatch` exact `wrap_into_claim`, `WrongTemplate`, removal of
`with_unchecked_companions`, bool `as_bool` / `try_*_bool`, text
`characterEncoding` helpers, `entry_at` / `scan_entry_at`,
`decode_frame` uses external `frame_len`, bulk-decode gate for
version-stable flat groups, staged-length `#[inline]` sweep (partial).

**Evidence method:** code reading + golden
(`sbe/tests/golden/car_example.rs`) + book/README. Full `just bench` was
**not** re-run for this review; PERF tickets require measurement before
keep/revert.

---

# 1. Quick wins (0.1.13)

S-effort, high value, no hot-path redesign. Land first.

## T-1: Repair garbled group-encode rustdoc

- Type: DOCS
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: generated group methods emit a broken sentence:

  ```text
  Closures return `GroupResult`; `?` just works. a
  separate `try_*` method name.
  ```

  Source: `sbe/src/codegen/message_encoder.rs:954-955`. Visible in golden
  `sbe/tests/golden/car_example.rs:5533-5534` and `:5621-5622`.
- Change: rewrite to one coherent paragraph, e.g. “Closures return
  `GroupResult` (`Result<(), EncodeError>`); `?` works — there is no
  separate `try_*` method.” Match the cleaner wording already used for
  `GroupResult` in `sbe/src/codegen/runtime.rs:235-236`.
- What breaks (API only), what it buys: no API break. Users stop seeing
  truncated nonsense on the primary group API.
- Acceptance criteria: golden regenerated; `rg 'just works\. a'` is empty
  under `sbe/`.
- Verification plan: `just update-golden` + `cargo test -p ergo-sbe --test
  docs_validation_test`. No bench impact (doc-only emission).

## T-2: Fix truncated `# Safety` on decoder `wrap`

- Type: DOCS
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: `CarDecoder::wrap` rustdoc is mid-sentence truncated:

  ```text
  `message_offset + HEADER_LENGTH + max(acting_block_length,
  must be ≤ `buf.len()`.
  ```

  Source: `sbe/src/codegen/message_decoder.rs:380-385`. Golden
  `sbe/tests/golden/car_example.rs:1251-1255`. The matching
  `wrap_unchecked` docs at `:1277-1279` are complete
  (`max(acting_block_length, min_readable_fixed_extent(...))`).
- Change: copy the complete extent formula from `wrap_unchecked` into
  `wrap` / any other truncated sites. While editing, stop calling public
  methods “Private zero-check” (see T-4).
- What breaks / buys: no API break. Safety contracts become copy-pastable
  into audits and Miri fixtures.
- Acceptance criteria: no truncated `max(acting_block_length,` line without
  the second arm; golden shows full formula.
- Verification plan: regenerate golden; spot-check
  `RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps`.

## T-3: Add `#[inline]` to `bulk_decode` convenience wrapper

- Type: PERF
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: `bulk_decode_into` has `#[inline]`
  (`sbe/src/codegen/group_decoder.rs:289-293`) but the public
  `bulk_decode` wrapper does not (`:324-328`). Golden still shows the
  miss (`sbe/tests/golden/car_example.rs:3414` region — methods without
  nearby `#[inline]` audit).
- Change: emit `#[inline]` on `bulk_decode` (thin Vec allocate +
  `bulk_decode_into`). Mechanism: remove a surviving call boundary on the
  materialise path for no-LTO consumers. Do **not** re-add `#[inline]` on
  enum `raw`/`from_raw` without new contrary evidence (0.1.12 deliberately
  left those off after a measured no-LTO decode regression).
- What breaks / buys: no API break. Small bulk materialisation path
  inlines through the convenience wrapper.
- Acceptance criteria: golden shows `#[inline]` on `bulk_decode`;
  `bulk_decode_into` unchanged.
- Verification plan: `just bench` LTO-off + LTO-on; maintained ratios ≤
  `1.00`. Optional diagnostic: group materialisation scenario if present;
  otherwise prove no regression on gated decode scenarios.

## T-4: Stop labelling public constructors “Private” / `HFT-008 keep=false`

- Type: DOCS
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: public safe methods still say they are private zero-check cores:

  - `message_decoder.rs:380` / golden `:1251` — `pub fn wrap` → “Private
    zero-check … (HFT-008 keep=false)”
  - `message_decoder.rs:500` / golden `:1342` — `pub fn decode`
  - `runtime.rs:1976` / golden `:7913` — `AnyMessage::decode`

  HFT-008 comments in `sbe/tests/hft_008_checked_unchecked_test.rs:1-11`
  still claim cores are “module-private (`keep: false`)” while
  `hft_001_soundness_test.rs:130-137` requires **public** `unsafe fn
  *_unchecked`.
- Change: rewrite rustdoc to the real three-tier story (see T-12). Drop
  `keep=false` / “Private” from any `pub` item. Update HFT-008 module docs
  to match 0.1.12 public unsafe twins. Optionally archive or recreate
  `docs/evidence/unchecked-keep-manifest.json` only if keep-gating is
  still a live process; otherwise delete dangling citations (T-5).
- What breaks / buys: no API break. Stops training users (and future LLMs)
  that public APIs are private.
- Acceptance criteria: `rg 'Private zero-check|HFT-008 keep=false' sbe/`
  is empty (or only in historical CHANGELOG). HFT-008 header no longer
  says “module-private”.
- Verification plan: `cargo test -p ergo-sbe --test hft_008_checked_unchecked_test
  --test hft_001_soundness_test --test hft_stale_interface_test`.

## T-5: Repair dangling `docs/SBE_COMPATIBILITY.md` / migration links

- Type: DOCS
- Stage: 0.1.13
- Priority: P0 · Effort: S
- Symptom: 0.1.12 deleted `docs/SBE_COMPATIBILITY.md` and
  `docs/MIGRATION_0_1_TO_0_1_10.md` (`git show d1a3ecb4 --stat`), but live
  links remain:

  - `sbe/README.md:30-35`
  - `book/src/introduction.md:6`, `:49`
  - `book/src/sbe/getting-started/from-sbe-tool.md:6`
  - `CHANGELOG.md:84-85`, `:100`

  `docs/` does not exist in the tree today. `hft_stale_interface_test.rs:103`
  soft-skips missing files, so CI stays green while docs.rs / GitHub 404.
- Change (pick one, prefer A):

  - **A.** Restore short normative pages under `docs/` (compatibility profile
    + 0.1.10/0.1.12 migration tables) pointing at the book trust-boundary
    chapter as the living API truth.
  - **B.** Point every link at book chapters
    (`core-concepts/trust-boundary.md`, `from-sbe-tool.md`) and delete
    remaining `docs/…` citations.

  Do not leave 404s either way.
- What breaks / buys: no API break. Normative compatibility claims become
  reachable again (road-to-1.0 docs criterion).
- Acceptance criteria: every repo link to `docs/SBE_COMPATIBILITY.md` or
  `docs/MIGRATION_*` resolves, or none remain. Book build + README paths
  checked.
- Verification plan: `rg 'docs/SBE_COMPATIBILITY|docs/MIGRATION' --glob
  '!target/**'`; `just book-ci` (or project equivalent); optional link
  checker.

## T-6: Document `GenerationConfig::profile` in the config table

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: `book/src/sbe/configuration/generation-config.md` lists every
  `with_*` knob but omits `profile(GenerationProfile)` even though
  `sbe/src/config.rs:107-121` and `:627-645` implement `Full` / `HftLean`
  presets (HFT-009). Users scanning the table miss the size-lean entry
  point.
- Change: add a row for `profile(GenerationProfile::{Full,HftLean})`
  describing which knobs it forces, and note individual `with_*` overrides
  still apply after `profile`.
- What breaks / buys: no API break. HFT-lean path is discoverable from the
  book table that people actually read.
- Acceptance criteria: row present; links to `GenerationProfile` rustdoc
  or feature-matrix.
- Verification plan: book build; no codegen change.

## T-7: Align crate-root “two-lane” wording with three-tier reality

- Type: DOCS
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: `sbe/src/lib.rs:35-38` still markets a **two-lane** boundary
  (`try_*` + `*_unchecked`) and omits the public bare `wrap` /
  `wrap_and_apply_header` / `decode` tier that 0.1.12 shipped.
  `CHANGELOG.md:56` repeats “two-lane”. Canonical chapter
  `book/src/sbe/core-concepts/trust-boundary.md:3-10` correctly says
  **three-tier**.
- Change: crate rustdoc + changelog wording → three-tier table matching
  `trust-boundary.md`. Do **not** claim panic-vs-UB properties that T-10
  has not yet made true; either (a) describe intent and link T-10, or
  (b) land T-10 first then document the final semantics only.
- What breaks / buys: no API break. First page of docs.rs stops
  contradicting the core concept chapter.
- Acceptance criteria: `rg 'two-lane' sbe/src book/src CHANGELOG.md`
  either gone or historically scoped to 0.1.10.
- Verification plan: rustdoc deny-warnings; `docs_validation_test`.

## T-8: Fix sample comments that deny the check they call

- Type: DOCS
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: `samples/sbe-feature-tour/src/lib.rs:61-62` and `:124-125`
  say “no bounds check needed” immediately before
  `try_wrap_and_apply_header(...).unwrap()`. The checked constructor
  **is** the bounds check.
- Change: comment → “buffer pre-sized via EncodedLength / const compute;
  `try_wrap_and_apply_header` still validates extent (use
  `wrap_and_apply_header` / `_unchecked` only after proof).”
- What breaks / buys: no API break. Teaching sample stops lying about the
  trust boundary.
- Acceptance criteria: those two comments corrected; sample still builds.
- Verification plan: sample tests / `cargo test` in `samples/sbe-feature-tour`.

---

# 2. Main tickets

## T-9: Close safe constructor + unchecked accessor soundness hole

- Type: CORRECTNESS
- Stage: 0.1.13
- Priority: P0 · Effort: M
- Symptom: field accessors and setters always use unchecked primitives:

  - decode getters: `read_bytes_unchecked` — golden
    `car_example.rs:1439-1441` (`serial_number`)
  - encode setters: `get_unchecked_mut` — golden `:5354-5360`

  Public **safe** constructors that do **not** prove fixed extent still
  hand out those accessors:

  - `CarDecoder::wrap` — `message_decoder.rs:386-399` / golden `:1257-1269`
  - `CarEncoder::wrap` / `wrap_and_apply_header` — `message_encoder.rs:354-429`
    (header write panics via `copy_from_slice`; body setters do not)
  - `AnyMessage::decode` — golden `:7919-7955` uses
    `read_bytes_unchecked` + `wrap_unchecked` while remaining **safe**
    `pub fn decode`

  Book claim at `book/src/sbe/core-concepts/trust-boundary.md:12-16,40`
  (“trusted panics on OOB accessor”) is **false for body fields**.
  Safe Rust must not allow UB on short buffers.
- Change (recommended design — SBE constructor-as-proof pattern):

  1. Every **safe** constructor that returns a flyweight whose methods use
     unchecked loads/stores must run the same extent proof as `try_*`
     (header + version-aware min fixed body). Prefer implementing bare
     `wrap` / `wrap_and_apply_header` / `decode` as thin wrappers that
     either check-then-build or share the `try_*` success path.
  2. Keep **one** skip-check lane: `unsafe fn *_unchecked` with complete
     `# Safety` (header + fixed body; dynamic tails remain checked on
     consume).
  3. `AnyMessage::decode`: either become the checked path (alias
     `try_decode`) or become `unsafe` and use unchecked only when marked
     unsafe. Today’s safe+unchecked combination is the worst of both.
  4. Optionally restore true “panic tier” via checked `read_bytes` /
     slice indexing **only if** benchmarks show it is free on stack
     buffers; do not ship a documented panic tier that is actually UB.

  Rationale: SBE has no self-describing total length; the constructor is
  the single trust checkpoint. Branch-free accessors are valid **after**
  a proof, not instead of one.
- What breaks (API only), what it buys:

  - Callers using bare `wrap` on undersized buffers may start getting
    `Result`/panic instead of silent UB (behaviour fix, may surface as
    new panics/errors — good).
  - If `AnyMessage::decode` becomes unsafe or is renamed, dispatch call
    sites update.
  - Buys: soundness; honest three-tier story; Miri/fuzz actually
    meaningful on constructor edges.
- Acceptance criteria:

  - Hostile tests: short buffer through every **safe** constructor must
    not be UB (error or panic only). Extend
    `hft_001_soundness_test` / hostile replay.
  - Golden + rustdoc: safe vs `unsafe` lanes match implementation.
  - Miri fixtures still green (`sbe/miri-fixtures`).
  - Wire parity unchanged (no wire format change).
- Verification plan:

  - `cargo test -p ergo-sbe --test hft_001_soundness_test --test
    hostile_input_replay_test --test hft_008_checked_unchecked_test`
  - `cargo +nightly miri test --manifest-path sbe/miri-fixtures/Cargo.toml`
  - both wire-parity suites
  - `just bench` LTO on/off — success-path ratios must stay ≤ `1.00`;
    mechanism evidence: checked constructors keep a single cold extent
    compare; accessors remain unchecked **after** proof (assembly of a
    post-`try_decode` scalar load should match pre-fix).

## T-10: Make `decode_unchecked` actually unchecked (or rename it)

- Type: CORRECTNESS
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: `CarDecoder::decode_unchecked` is `unsafe` and claims “raw
  pointer header read” (`message_decoder.rs:531-539`) but the body still
  calls **checked** `read_bytes` (panics on short header) — golden
  `:1390-1394`. Meanwhile `AnyMessage::decode` (safe!) uses
  `read_bytes_unchecked` (golden `:7920`). Lanes are inverted.
- Change: `decode_unchecked` header path must use
  `read_bytes_unchecked` (matching `# Safety`). `decode` (trusted/safe)
  must use `read_bytes` **and** prove body extent (T-9). Mirror the
  same split for encoder `*_unchecked` vs bare constructors.
- What breaks / buys: unsafe call sites that relied on panic-on-short
  header through `decode_unchecked` lose that panic (they were already
  in an `unsafe` block with a false contract). Clear lane semantics.
- Acceptance criteria: source/golden show unchecked header read only
  inside `unsafe fn *_unchecked`; safe constructors never call
  `read_bytes_unchecked` without a prior extent proof in the same
  function.
- Verification plan: unit tests for short-header behaviour per lane;
  Miri on unchecked with valid buffers; `just bench` to ensure unchecked
  path remains free of panic machinery on the success path.

## T-11: One accurate trust-boundary + migration story (finish 0.1.12 T-14)

- Type: DOCS
- Stage: 0.1.13
- Priority: P0 · Effort: M
- Symptom: contradictory teaching still ships side-by-side:

  | Source | Claims |
  |--------|--------|
  | `book/src/sbe/core-concepts/trust-boundary.md` | three-tier; `try_*` / bare / `*_unchecked` (closest to code intent) |
  | `book/src/sbe/feature-tour/trust-boundaries.md:11-21` | 0.1.10 model: no public `try_wrap*`; unchecked private |
  | `book/src/sbe/getting-started/from-sbe-tool.md:74-103` | `decode`/`wrap` return `Result`; no `try_wrap*` |
  | `book/src/sbe/design-notes/feature-matrix.md:14` | “Checked constructors (0.1.10)” unsuffixed `Result` |
  | `book/src/sbe/design-notes/api-freeze.md:49` | “two-lane” |
  | `sbe/README.md:30-35,64-65` | fallible `wrap`; `try_wrap*` removed; private unchecked; 404 docs links |
  | `sbe/src/lib.rs:35-38` | two-lane |

  Also: `hft_stale_interface_test.rs` still forbids phrases like
  “trusted wrap” (`:20-38`) while the core chapter must teach a trusted
  tier — the guardrail fights the correct docs.
- Change: after T-9 settles semantics, rewrite **all** rows above to one
  story. Feature-tour page should `{{#include}}` or deep-link the core
  chapter instead of a second outdated table. Update
  `hft_stale_interface_test` allowlist/needles for 0.1.12+ vocabulary
  (forbid 0.1.10 “no try_wrap” lies; allow “trusted” when accurate).
  Add a short 0.1.11→0.1.12→0.1.13 migration table (names only) in
  `from-sbe-tool.md` or restored `docs/MIGRATION_*.md` (T-5).
- What breaks / buys: no API break. Removes the #1 onboarding footgun
  (which constructor validates?).
- Acceptance criteria: a reader of **any one** of {crate rustdoc, sbe
  README, feature-tour trust page, core trust page} gets the same three
  tiers and the same method names. `rg 'no public .try_wrap'` under
  `book/` and `sbe/README.md` is empty. Stale-interface test green with
  updated policy.
- Verification plan: `docs_validation_test`, `hft_stale_interface_test`,
  rustdoc, book build. Doc-only after T-9; benches unchanged.

## T-12: Stop `From<BooleanType> for bool` collapsing `NullVal` → `true`

- Type: API
- Stage: 0.1.13
- Priority: P1 · Effort: S
- Symptom: golden `car_example.rs:417-420`:

  ```rust
  impl From<BooleanType> for bool {
      fn from(val: BooleanType) -> bool { val as u8 != 0 }
  }
  ```

  `NullVal = 255` (`:351`) therefore becomes **`true`**. Rustdoc on
  `as_bool` already warns (`:365-369`); CHANGELOG 0.1.12 told users to
  prefer `as_bool` / `try_*_bool`, but the infallible `From` remains the
  path `bool::from(dec.available())` and `.into()` take by default.
- Change: remove `From<BooleanType> for bool` (and peers for other bool
  enums), **or** replace with `TryFrom<BooleanType> for bool` that errors
  on `NullVal`/unknown. Keep `From<bool> for BooleanType` (F/T only).
  Prefer `as_bool()` / `try_*_bool` in book samples. SBE booleans are
  tri-state on the wire; an infallible `bool` conversion is a lie.
- What breaks / buys: `.into()` / `bool::from(enum)` call sites must use
  `as_bool()`/`try_*`. Removes a silent true-for-null footgun in trading
  flags.
- Acceptance criteria: golden has no infallible `From<BooleanType> for
  bool`; tests cover F/T/NullVal/unknown; CHANGELOG Breaking entry;
  samples updated.
- Verification plan: comprehensive/conformance/bool tests; no wire
  change; benches unaffected (conversion is cold/app path).

## T-13: Do not surface partial frames as `as_bytes_with_header` on incomplete encoders

- Type: API
- Stage: 0.1.13
- Priority: P1 · Effort: M
- Symptom: `CarEncoder::get_metadata().as_bytes_with_header()` is
  available on the **initial** encoder stage
  (`message_encoder.rs:893-921`, golden `:5508-5528`) while `pos` after
  wrap is only `msg_offset + HEADER + BLOCK_LENGTH` (fixed body end).
  For messages with groups/var-data, that slice is **not** a complete SBE
  message, yet the name matches the complete-stage API
  (`CarComplete::as_bytes_with_header`, golden `:6164`). Complete-stage
  restriction exists for inherent methods (`message_encoder.rs:1200-1235`)
  but metadata bypasses it on the start stage.
- Change:

  - Restrict encoder metadata byte views to complete stages, **or**
  - Rename start-stage helpers to `as_fixed_block_bytes` /
    `fixed_region_with_header` and rustdoc “incomplete if tails remain”.
  - Decoder metadata already returns `Result` for full-frame walks in
    places; keep encoder/decode naming parallel.

  Rationale: SBE completeness is positional; APIs that look like “the
  message bytes” must not be available before tails are written
  (project invariant: complete-message `as_bytes` only on complete
  stages).
- What breaks / buys: callers using metadata mid-encode for “full frame”
  must move to `CarComplete` or the renamed partial API. Removes
  publish-truncated-header+fixed-as-if-done footgun.
- Acceptance criteria: compile-fail or API absence of
  complete-sounding names on incomplete stages; tests for fixed-only vs
  tailed messages; book buffer-sizing / generated-code pages updated.
- Verification plan: golden + baseline stage tests; wire parity; benches
  (metadata not on hot gated path — expect flat).

## T-14: Carry source detail in `DomainConversionFailed` (and stop misusing it for null bools)

- Type: API
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom:

  1. `DecodeError::DomainConversionFailed { field }` / encode twin
     (`runtime.rs:27,68`) drop the underlying conversion error — Display
     is only `"domain conversion failed"` (`:43,85`).
  2. `try_*_bool` maps null/unknown bool to
     `DomainConversionFailed` (`message_decoder.rs:996-999`, golden
     `:1505-1508`) even though no domain converter ran — wrong concept
     for a tri-state wire enum.
- Change:

  - Add `InvalidBoolean { field }` (or reuse a null/unknown enum
    variant) for `try_*_bool`.
  - For real domain `try_*`, either embed a static reason string or use
    `thiserror`-style source when the converter error is `'static` /
    `Display`. Prefer a small `'static str` reason to stay
    `no_std`-friendly if required.
- What breaks / buys: exhaustive matches gain a variant. Operators can
  tell null-bool from decimal parse failure without guessing.
- Acceptance criteria: error quality tests assert Display + variant for
  null bool vs domain failure; golden updated.
- Verification plan: `error_validation_test`; no hot-path change
  (`#[cold]` Display already).

## T-15: Rename `DomainVarData::LossyStrings` → `Strings` (strict since 0.1.10)

- Type: API
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Status: **done** — hard rename to `DomainVarData::Strings`; no deprecated
  alias (0.1.x may break).

## T-16: `#[must_use]` on consuming decoder stages and length builders that still miss it

- Type: API
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: encoder stages / encoded-length builders often carry
  `#[must_use]` (`encoded_length.rs:80`, `message_encoder.rs:150`), but
  many decoder stage structs and domain helpers do not (golden audit:
  `CarDecoderAfter*`, `*EntryDecoder`, domain `try_from_decoder` /
  `encode_into` without nearby `must_use`). Dropping a consuming
  `into_*` result silently skips tails.
- Change: emit `#[must_use = "…"]` on:

  - all consuming decoder stage structs
  - `into_*` / `finish` / `skip_remaining` results where the value is a
    stage or unread payload
  - domain `encode` / `encoded_length*` results

  Match encoder wording quality.
- What breaks / buys: new must_use warnings at call sites that ignored
  stages — desired. Catches truncated decode walks.
- Acceptance criteria: golden shows attributes; a deliberate ignore in a
  test expects `unused_must_use` if compiled with deny.
- Verification plan: `cargo test -p ergo-sbe`; consumer warning-free test
  (`hft_005`) still green.

## T-17: `#[inline]` audit for domain DTO encode/decode helpers

- Type: PERF
- Stage: 0.1.13
- Priority: P2 · Effort: M
- Symptom: `domain_cluster.rs` emits many public `try_from_decoder`,
  `encode`, `encode_into`, `length_contribution`, `encoded_length*`
  methods with almost no `#[inline]` (file has ~2 inline attributes vs
  many `pub fn`). Golden domain section (~4409-4850) matches. These are
  not the microbench hot path but are used in app snapshot/DTO loops.
- Change: measurement-led `#[inline]` on small forwarding methods only.
  Mechanism: cross-crate inlining for DTO re-encode without LTO. Skip
  large materialise bodies if assembly shows code-size regression.
- What breaks / buys: no API break. DTO round-trip loops improve under
  no-LTO when inlining was the limiter.
- Acceptance criteria: before/after Criterion diagnostic on a domain
  encode path (feature-tour or domain_objects fixture); maintained gate
  unchanged ≤ `1.00`.
- Verification plan: `just bench` full gate; record no-LTO domain
  diagnostic in the session notes if ratios move.

## T-18: Name the field on `EncodeError::BufferTooShort`

- Type: API
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: `DecodeError::BufferTooShort` carries `field`
  (`runtime.rs:16`); `EncodeError::BufferTooShort` only has
  `needed`/`available` (`:52`). Group/var-data encode sites know the
  field name but drop it (`message_encoder.rs:967+`).
- Change: add `field: &'static str` (or `Option` for pure capacity
  constructors) to encode `BufferTooShort`; thread `stringify!` at
  emission sites. Display like decode.
- What breaks / buys: exhaustive match update. Encode failures become
  actionable in multi-field messages.
- Acceptance criteria: error tests for group dim short and var-data short
  include field names; golden Display snapshots updated if any.
- Verification plan: `error_validation_test`; cold path only.

## T-19: Sample RFQ clients still use magic `[0u8; 512]`

- Type: DOCS (samples) / CORRECTNESS hygiene
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: project hard-rule is EncodedLength sizing. Explicit TODOs
  remain:

  - `samples/cluster-rfq/examples/rfq_client.rs:53-54`
  - `auction_client.rs:46-47`, `:96-97`
  - `rfq_roundtrip.rs:56-57`

  all `let mut buf = [0u8; 512];` (or similar).
- Change: size with the generated message’s
  `compute_length_with_header` / staged `*EncodedLength` / const
  `ENCODED_LENGTH`; stack pad only when runtime length ≤ pad with assert.
- What breaks / buys: samples become the teaching path for claim-friendly
  sizing; hides fewer length bugs.
- Acceptance criteria: TODOs gone; examples still run against the Java
  harness when available.
- Verification plan: sample build; no sbe gate impact.

## T-20: `MessageVisitor::visit_unknown` default is `unimplemented!()`

- Type: API
- Stage: 0.1.13
- Priority: P2 · Effort: S
- Symptom: golden `car_example.rs:8091-8095` default method panics on
  unknown template. Dispatch APIs that default to panic violate the
  “checked entry points must not manufacture success / must report
  errors” product rule when visitors are partial.
- Change: either (a) no default method (force implementors to handle
  unknown), or (b) default returns a typed visitor-defined “ignore”
  only when `Output = ()`, or (c) default calls a provided
  `unhandled_template` hook that returns `Result`. Prefer (a) for
  library code — make incomplete visitors a compile error.
- What breaks / buys: visitors that relied on default panic must
  implement the method (usually `Ok(())` or log). Unknown templates
  become intentional policy, not a latent production panic.
- Acceptance criteria: golden + multi-template sample updated; test that
  a visitor handling only known messages compiles only when unknown is
  implemented.
- Verification plan: dispatch tests; no wire change.

## T-21: Refresh HFT-008 keep harness to match public three-tier API

- Type: CORRECTNESS (tests/tooling)
- Stage: 0.1.13
- Priority: P2 · Effort: M
- Symptom: `hft_008_checked_unchecked_test.rs` module docs and probe still
  describe private cores and, in places, time `try_wrap` against itself
  under `unsafe` (see probe around lines 57-99 in the current file —
  both arms call `try_wrap_and_apply_header`). That does not measure the
  public `wrap` / `wrap_unchecked` delta the keep rule claimed to gate.
- Change: rewrite matrix to compare the real tiers post-T-9:

  - `try_*` vs bare (if bare remains distinct) vs `*_unchecked`
  - record samples; either publish a keep manifest or document that
    public unsafe twins are permanent product surface (0.1.12 decision)
    and demote the “keep=false” process to historical.

- What breaks / buys: no product API break. Performance claims about
  unchecked become evidence-based again.
- Acceptance criteria: probe compiles against public API only; emits
  distinct arms; CI artifact or test stdout documents methodology.
- Verification plan: run the test; optional Criterion follow-up; full
  bench gate unchanged unless product code changes.

---

# 3. Roadmap cross-check

Source: `book/src/project/road-to-1.0.md`.

| Ticket(s) | Roadmap status | Notes |
|-----------|----------------|-------|
| T-9, T-10 | **already planned** | Trust-boundary criterion: “no known P0 safety issues” (`road-to-1.0.md:20-22`); API freeze constructor surface (`:11-13`). |
| T-5, T-7, T-11 | **already planned** | Docs criterion: migration, trust boundaries, buffer sizing linked from README (`:23-25`). |
| T-3, T-17 | **already planned** (gate level) | Parity/performance ceiling for three minors (`:14-17`); specific inline misses are new instances. |
| T-12, T-13, T-14, T-15, T-16, T-18, T-20 | **new** | Concrete API/error footguns not named by the roadmap. |
| T-1, T-2, T-4, T-6, T-8 | **new** | Doc/codegen hygiene found in this pass. |
| T-19, T-21 | **new** | Sample hygiene / test harness drift. |
| Every codec ticket | **already planned** (gate) | Wire compatibility (`:18-19`) — acceptance keeps parity/goldens green. |
| External pilot / case study | **already planned** | `road-to-1.0.md:26-28` — not implementable as a pure code ticket; see 1.0-only. |

---

# 4. 1.0-only (do not block 0.1.13)

## T-100: External signal for 1.0 disclaimer exit

- Type: DOCS / process
- Stage: 1.0
- Priority: P1 · Effort: L (calendar, not code)
- Symptom: road-to-1.0 still requires an external user or published case
  study (`road-to-1.0.md:26-28`). No in-repo substitute fully closes it.
- Change: pilot write-up or third-party schema latency/wire report linked
  from README / book project section.
- Acceptance: disclaimer can be lifted when **all** six criteria hold,
  including this one.

## T-101: Freeze generated stage / wrap / FixedFields names

- Type: API
- Stage: 1.0
- Priority: P0 · Effort: L
- Symptom: API freeze note (`api-freeze.md`) records decisions but 0.1.x
  still churns constructor names and metadata facets (`get_metadata`,
  three-tier renames).
- Change: after 0.1.13 trust-boundary tickets settle, run a dedicated
  freeze audit; any rename after that is a major.
- Acceptance: freeze checklist checked; golden is the review artifact;
  no pending renames in REVIEW tickets.

## T-102: Remove deprecated aliases (any remaining 0.1.x shims)

- Type: API
- Stage: 1.0
- Priority: P2 · Effort: S
- Note: `LossyStrings` already hard-removed in 0.1.13 (no alias).

## T-103: Cluster 1.0 clock (separate)

- Type: API / DOCS
- Stage: 1.0 (cluster crate)
- Priority: P2 · Effort: L
- Symptom: `cluster/src/config.rs:119-127,200-205` still reject shared
  ingress and external Aeron injection as “not yet supported”. Roadmap
  keeps cluster on a separate clock (`road-to-1.0.md:33-42`).
- Change: either implement those modes or document them as non-goals for
  cluster 1.0; lock codecs to a released `ergo-sbe` major; keep
  `just bench-cluster` baselines.
- Acceptance: session lifecycle + error types stable under multi-node
  harness; Aeron/rusteron matrix documented.

---

# Reading order for implementers

1. `CLAUDE.md` / `Claude.md` — wire + bench invariants (non-negotiable).
2. **T-9 → T-10 → T-11** (soundness then honest docs).
3. Quick wins T-1…T-8 (can parallelise once T-9 semantics are decided).
4. Remaining main tickets by priority.
5. Do not run `just bench` only at the end — any `sbe/` codegen change
   that touches hot paths needs the gate before the change is kept.

# Out of scope / not verified here

- Full `just bench` numbers on this machine (not run for this write-up).
- Whether every sample under `samples/cluster-*` builds without Java
  (RFQ magic buffers verified by source only).
- Cluster client deep review beyond config “not yet supported” markers.
- Re-opening closed 0.1.12 tickets that already match CHANGELOG 0.1.12
  behaviour unless this file cites a remaining defect.
