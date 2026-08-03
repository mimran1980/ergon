# ergo-sbe 0.1.12 — review tickets

Hand-off spec for an implementing engineer with no other context. Every claim
below is grounded in a file:line citation from the tree at commit `85564c24`
(branch `feat/0.1.12`, workspace version `0.1.12`, last released `0.1.11`).

## Baseline verification (done by the reviewer, 2026-08-03)

| Check | Command | Result |
|---|---|---|
| Compile | `cargo check --all-targets` | clean, exit 0 |
| Wire parity | `cargo test -p ergo-sbe --test sbe_tool_wire_parity_test` | 23 passed, 0 failed |
| Soundness gates | `cargo test -p ergo-sbe --test hft_001_soundness_test` | passed |

`just bench` was **not** run (out of scope for this review). Benchmark
methodology was assessed by reading `sbe/BENCHMARKS.md`,
`sbe/benchmarks/benches/`, and `scripts/check-bench-gate.sh`. The gate covers
ten SBE pairs and five cluster pairs, all at a `1.00` ceiling with 0.5% noise
tolerance (`scripts/check-bench-gate.sh:58-69`, `:98-104`).

**Not verified, flagged explicitly:** T-1's panic is proven by code reading, not
by a running repro. Its first acceptance step is to write the failing test. No
claim in T-2 depends on a measurement — it is a compile-time-only change.

## Reading order for the implementer

`CLAUDE.md` (invariants + benchmark fairness rules) → this file → the cited
source. T-1 and T-2 both touch the trust boundary; read both before starting
either.

---

# 1. Quick wins

Small, high-value, no hot-path risk. Land these first.

## QW-1: Add `#[inline]` to generated enum and composite constructors

- Type: PERF · Stage: 0.1.12 · Priority: P0 · Effort: S
- **Symptom:** `sbe/src/codegen/runtime.rs:820` emits `pub fn raw(self)` and
  `:824` emits `pub const fn from_raw(val)` with no `#[inline]`.
  `runtime.rs:1260` emits composite `pub fn new(...)` with no `#[inline]`.
  Visible in the golden at `sbe/tests/golden/car_example.rs:344`, `:347`,
  `:905` (`Booster::new`), `:977` (`Engine::new`).
  `CLAUDE.md` states missing `#[inline]` on a public generated hot-path method
  is a defect, and the 0.1.4 cycle already fixed this exact class for setters
  and stage transitions (`CHANGELOG.md:236-242`) — enums and composites were
  missed.
- **Change:** add `#[inline]` to the three emission sites. `Enum::raw()` is
  called by every enum field setter; `Composite::new()` is the only way to
  build a composite value for `.engine(val)`-style setters, so both sit on the
  user's encode path.
- **Mechanism:** without `#[inline]`, a downstream crate cannot inline these
  across the crate boundary unless LTO is on. This is precisely the failure the
  project already measured: the closure encode path went from ~445 ns with LTO
  to 2.093 µs without it before inline intent was added
  (`CHANGELOG.md:236-242`, `sbe/BENCHMARKS.md` "Group encode: LTO on and off").
- **What it buys:** no-LTO consumers stop paying a real call per enum/composite
  construction. No API change.
- **Acceptance:** golden regenerated (`just update-golden`) showing `#[inline]`
  on `raw`, `from_raw`, and every composite `new`; diff is the review payload.
- **Verification:** `just bench` with the LTO-off matrix published alongside
  LTO-on. Both profiles must stay at or below the `1.00` ceiling; the no-LTO
  numbers are where the improvement should appear.

## QW-2: Add `#[inline]` to `acting_block_length()` and stop emitting it from a string

- Type: PERF · Stage: 0.1.12 · Priority: P1 · Effort: S
- **Symptom:** `sbe/src/codegen/message_decoder.rs:509-511` emits both accessors
  through `syn::parse_str` on a hand-written string literal. `acting_version`
  gets `#[inline]`; `acting_block_length` does not
  (golden `car_example.rs:1313-1319`). `acting_block_length()` is read by every
  tail-offset computation.
- **Change:** replace the `syn::parse_str(...)` call with a `quote!` block and
  put `#[inline]` on both. `CLAUDE.md` requires generated Rust to be built with
  `syn`/`quote!`; parsing a hand-written source string is the same class of
  defect as the forbidden `push_str(&format!(...))`.
- **What it buys:** consistent inlining on a decode-path accessor, and one less
  place where generated code is assembled as text.
- **Acceptance:** golden shows `#[inline]` on both; no `parse_str` of a
  multi-line function body remains in `message_decoder.rs`.
- **Verification:** `just bench` — `decode_full_message` and
  `decode_entry_point` are the gated scenarios that read this accessor.

## QW-3: Add `#[inline]` to the ragged length-builder methods

- Type: PERF · Stage: 0.1.12 · Priority: P2 · Effort: S
- **Symptom:** golden `car_example.rs:6615` (`add`) and `:6622` (`uniform`) on
  `CarFuelFiguresRaggedBuilder` carry no `#[inline]`, while the surrounding
  `EncodedLength` surface does (`sbe/src/codegen/encoded_length.rs:80`, `:237`).
- **Change:** add `#[inline]` at the emission site in `encoded_length.rs`.
- **What it buys:** buffer sizing runs immediately before every encode; these
  are trivial forwarding methods that should compile to nothing.
- **Acceptance:** golden regenerated; `encoded_length_api_test` still green.
- **Verification:** `just bench`; sizing is not separately gated, so the bar is
  simply no regression in the gated encode scenarios.

## QW-4: Correct the trust-boundary documentation — it currently describes the inverse of the shipped API

- Type: DOCS · Stage: 0.1.12 · Priority: P0 · Effort: S
- **Symptom:** four documents state that `try_wrap*` was removed and that
  `wrap` returns `Result`. The generated code does the opposite.
  - `docs/MIGRATION_0_1_TO_0_1_10.md:13-17` — "`Encoder::try_wrap` → `Encoder::wrap` → `Result`",
    "Safe `Encoder::wrap` → `Result` (zero-check core is **private**)".
  - `docs/SBE_COMPATIBILITY.md:65-68` — lists `wrap` / `wrap_and_apply_header` /
    `decode` in the **checked (safe)** lane, and says "`try_wrap*` aliases are
    **removed** in 0.1.10".
  - `book/src/sbe/core-concepts/trust-boundary.md:7,17` — "There is **no**
    public `try_wrap*` alias"; "`Encoder::wrap` / `wrap_and_apply_header` |
    Capacity check for header + fixed block".
  - `book/src/sbe/feature-tour/trust-boundaries.md:16,18` — same claim.

  Reality: `sbe/tests/golden/car_example.rs:1175` `pub fn try_wrap` (checked,
  returns `Result`), `:1211` `pub fn wrap` (infallible, no check), `:4922`
  `pub fn try_wrap`, `:4955` `pub fn try_wrap_and_apply_header`, `:4970`
  `pub fn wrap_and_apply_header` (infallible, no check).
- **Change:** rewrite all four locations to describe the API that ships. The
  fastest correct wording distinguishes *checked* (`try_*`, returns `Result`,
  validates extent) from *trusted* (`wrap*`, infallible, caller owns the
  extent precondition). Add one worked example per lane.
- **What it buys:** the single most safety-critical page in the docs stops
  telling users that an unchecked constructor is checked.
- **Dependency:** if **T-2** lands in the same release, write the post-T-2 state
  directly and skip the intermediate wording. If T-2 slips, this still ships —
  the docs must not stay wrong for another release.
- **Acceptance:** `docs_validation_test` green; no occurrence of "no public
  `try_wrap*`" or "try_wrap* aliases are removed" survives; a reader can
  determine from the trust-boundary page alone which constructor validates.
- **Verification:** `just book-ci`, `cargo test -p ergo-sbe --test docs_validation_test`.

## QW-5: Restore or delete the cited `docs/evidence/` manifest

- Type: DOCS · Stage: 0.1.12 · Priority: P1 · Effort: S
- **Symptom:** `docs/SBE_COMPATIBILITY.md:70` cites
  `docs/evidence/unchecked-keep-manifest.json` as the normative record of
  `_unchecked` keep decisions. `docs/MIGRATION_0_1_TO_0_1_10.md:39` and
  `sbe/src/codegen/message_encoder.rs:328` cite the same path. The directory
  does not exist (`ls docs/evidence` → no such file).
- **Change:** either create the manifest with the real keep decisions (all
  currently `keep: false`, per the codegen comments), or delete all three
  citations and state the policy inline in `SBE_COMPATIBILITY.md`. Prefer
  creating it — `SBE_COMPATIBILITY.md` is a normative compatibility claim and a
  dangling reference in it is a credibility defect.
- **Acceptance:** every `docs/evidence/` reference resolves, or none remains.
  Add the path check to `hft_stale_interface_test.rs` (which already knows about
  this directory at line 92).
- **Verification:** `cargo test -p ergo-sbe --test hft_stale_interface_test`.

## QW-6: Replace `peek_header`'s `(u16, u16)` return with a named type

- Type: API · Stage: 0.1.12 · Priority: P1 · Effort: S
- **Symptom:** golden `car_example.rs:682` —
  `pub fn peek_header(data: &[u8]) -> Option<(u16, u16)>` returns
  `(template_id, schema_id)`. Both elements are `u16`, so
  `let (schema_id, template_id) = MessageHeader::peek_header(buf)?;` compiles
  and is silently wrong — and this is the dispatch primitive, so getting it
  backwards routes frames to the wrong decoder.
- **Change:** return a `#[derive(Clone, Copy)] pub struct PeekedHeader { pub
  template_id: u16, pub schema_id: u16 }` from the emission site in
  `sbe/src/codegen/runtime.rs:1287`. Keep `peek_template_id` and
  `peek_for_schema` as-is (single-value returns are unambiguous). Update
  `peek_for_schema` (golden `:710`) to destructure by field name.
- **SBE rationale:** template id and schema id are distinct identifiers in the
  message header composite; the generated API should not flatten two
  semantically different `u16`s into a positional pair.
- **What breaks:** any caller destructuring the tuple. This is exactly the class
  of break 0.1.12 is for; the fix at each call site is mechanical.
- **What it buys:** an illegal state (transposed ids) becomes unrepresentable.
- **Acceptance:** golden regenerated; `baseline_test` and any dispatch test
  updated; CHANGELOG "Breaking" entry.
- **Verification:** `cargo test -p ergo-sbe`; wire parity unaffected (no wire
  bytes change).

## QW-7: Delete `as_ref_opt()`

- Type: API · Stage: 0.1.12 · Priority: P2 · Effort: S
- **Symptom:** golden `car_example.rs:2026-2031` emits
  `pub fn as_ref_opt(&self) -> Option<&[u8]>`, whose entire body is
  `self.as_bytes_with_header().ok()`. Its own rustdoc says "Prefer
  [`Self::as_bytes_with_header`] for explicit error handling." It also lacks
  `#[inline]`.
- **Change:** remove the emission. A generated method that discards a typed
  `DecodeError` and whose doc comment tells you not to use it is pure surface
  area; `README.md:78` states checked entry points must report malformed input
  rather than manufacture empty values, and this one throws the reason away.
- **What breaks:** callers of `as_ref_opt()` — replace with
  `.as_bytes_with_header().ok()` verbatim if they genuinely want the `Option`.
- **What it buys:** one fewer way to silently lose an error at the trust
  boundary.
- **Acceptance:** golden regenerated; no test references `as_ref_opt`.
- **Verification:** `cargo test -p ergo-sbe`.

## QW-8: Narrow the generated module's blanket `#[allow]` list

- Type: CORRECTNESS · Stage: 0.1.12 · Priority: P2 · Effort: S
- **Symptom:** golden `car_example.rs:2-21` applies eleven module-wide allows to
  every generated module, including `dead_code`, `unused_variables`,
  `unused_mut`, and `unused_assignments`. These mask genuine generator bugs: an
  accessor that is generated but unreachable, or a `let mut` that is never
  mutated, produces no diagnostic anywhere.
  Concrete instance: `min_readable_fixed_extent` emits `let mut m = 45; m` with
  no intervening mutation whenever a schema has no versioned fields
  (`sbe/src/codegen/message_decoder.rs:305-325`, golden `:1158-1161`) — that is
  what `unused_mut` is suppressing.
- **Change:** emit `let mut` only when at least one version arm follows
  (`versions` is non-empty in `message_decoder.rs:308`), then drop
  `unused_mut`, `unused_assignments`, and `unused_variables` from the module
  allow list. Keep the clippy allows that are genuinely unavoidable in generated
  arithmetic (`identity_op`, `erasing_op`, `absurd_extreme_comparisons`).
- **What it buys:** the generator's own output starts warning about generator
  bugs instead of hiding them.
- **Acceptance:** golden regenerated; `hft_005_warning_free_consumer_test` still
  green (it is the test that proves consumers see no warnings); generation over
  every fixture schema in `sbe/tests/fixtures/schemas` produces no new warnings.
- **Verification:** `cargo test -p ergo-sbe`, plus
  `RUSTFLAGS="-D warnings" cargo check` on a sample crate.

---

# 2. Main tickets

## T-1: Flat var-data accessors panic on a truncated tail instead of returning `BufferTooShort`

- Type: CORRECTNESS · Stage: 0.1.12 · Priority: P0 · Effort: M

**Symptom.** The non-consuming var-data accessor reads its 4-byte length prefix
with the panicking helper and no preceding bounds check:

`sbe/src/codegen/message_decoder.rs:1164-1185` emits

```rust
pub fn manufacturer(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
    let offset = self.tail_offset_2()?;
    let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);   // <-- panics
    …
}
```

(golden `sbe/tests/golden/car_example.rs:1700-1702`), and `read_bytes` is

```rust
buf[offset..offset + N].try_into().expect("read_bytes: buffer too short")
```

(golden `:7496-7498`).

The consuming twin does the right thing — `sbe/src/codegen/tail_stages.rs:152-158`
checks `offset + prefix_size > self.buf.len()` and returns
`DecodeError::BufferTooShort`. So the same field reached two different ways
behaves differently: `into_manufacturer()` returns an error, `manufacturer()`
aborts the process.

`tail_offset_N` does not cover this. Each `tail_offset_{k+1}` validates the
prefix or dimension of element *k* and returns the offset *of element k+1*
(`message_decoder.rs:1022-1044` for groups, `:1059-1069` for var-data), and
`tail_offset_0` has no check at all (`:978-983`). No generated code validates
that element k+1's own prefix is in bounds before the flat accessor reads it.

**Reachability.** `try_decode` validates only the header and the version-aware
fixed extent (`message_decoder.rs:444-473`), never the tail. For the golden Car
schema: a 61-byte buffer holds the 8-byte header, the 45-byte block, and both
zero-count group dimensions, so `CarDecoder::try_decode(buf, 0)` succeeds;
`.manufacturer()` then reads `buf[61..65]` on a 61-byte slice and panics. For a
schema whose first tail element is var-data, `tail_offset_0` returns
`pos + acting_block_length` unchecked and the panic needs only a
header-plus-block buffer.

**Why the existing hostile-input tests miss it.**
`sbe/tests/hostile_input_replay_test.rs:105-147` gates its traversal behind
`verify().is_ok()` and then exercises only the **consuming** stages
(`into_bids`, `into_orders`, `into_order_id`). It never calls a flat var-data
accessor on a decoder obtained from `try_decode` alone.
`hft_001_soundness_test.rs:80-95` catch-unwinds only the constructors, not any
field accessor.

**Change.** In `message_decoder.rs:1164-1185`, replace the bare
`read_bytes::<N>` with the same guarded read the consuming stage uses: bounds
check → `DecodeError::BufferTooShort { field, needed, available }` → unchecked
read. Factor the check into one helper so the two paths cannot drift again —
`tail_stages.rs:152-176` and the flat accessor should call the same emitted
code. Audit every other `read_bytes::<N>` in a `Result`-returning generated
accessor for the same pattern (`message_decoder.rs:1033` reads a group
dimension, but that one *is* preceded by a check at `:1026`).

**SBE-pattern rationale.** SBE messages carry no length; a decoder that has
validated the header and fixed block has proven nothing about the dynamic tail.
Every tail read is therefore an untrusted read and must be checked exactly
once, at the read. `README.md:78` already states this as a project boundary:
"Checked entry points must report malformed input rather than manufacture
default, empty, or lossy values" — panicking is a third failure mode that
contract does not permit.

**Bench-gate safety (important).** This adds **no** runtime work. `read_bytes`
already bounds-checks — `buf[o..o+N]` is a slice index. The change converts a
panicking check into an error-returning check. It should be *marginally faster*:
`expect` on a `TryFromSliceError` forces a panic landing pad and a formatting
call into an `#[inline]` function on the decode path; an explicit `if` with an
early `Err` return removes the unwind edge. This does not contradict the
standing "no new bounds checks on the hot path" rule — no check is added.

**What it buys.** A documented-fallible accessor stops aborting the process on
hostile input. For a market-data consumer this is the difference between a
logged `BufferTooShort` and a dead process.

**Acceptance criteria.**
1. A new test in `sbe/tests/hostile_input_replay_test.rs` (or a new
   `flat_accessor_hostile_test.rs` — the `sbe/tests/*.rs` glob in
   `test-lanes.tsv:5` already owns it, no new row needed) that, for every
   schema with var-data, calls `try_decode` on a buffer truncated at each byte
   from `HEADER_LENGTH + BLOCK_LENGTH` up to the full frame and asserts every
   flat var-data accessor returns `Err`, never panics. **Write this first and
   watch it fail** — it is the proof the defect is real.
2. Golden regenerated; the diff shows the guarded read.
3. `sbe_tool_wire_parity_test` and
   `sbe_tool_multi_schema_wire_parity_test` unchanged and green (no wire bytes
   move).
4. CHANGELOG "Fixed" entry.

**Verification plan.** `cargo test -p ergo-sbe`; then `just bench` with both LTO
profiles. `decode_full_message` and `throughput/batch_10k` are the gated
scenarios that traverse var-data. Expect flat or slightly improved ratios; if
any gated ratio rises above `1.00`, the guarded read was emitted in the wrong
place (inside the loop rather than once per field) — fix the codegen, do not
move the ceiling.

---

## T-2: Make the trust boundary honest — safe `wrap*` constructors can cause UB from safe code

- Type: API · Stage: 0.1.12 · Priority: P0 · Effort: M

**Symptom.** Three generated constructors are **safe** `pub fn` with documented
memory-safety preconditions, and violating them is undefined behaviour reachable
without writing a single `unsafe` block.

| Item | golden | Body |
|---|---|---|
| `Encoder::wrap_and_apply_header` | `car_example.rs:4970-4988` | `unsafe { copy_nonoverlapping(HEADER_TEMPLATE.as_ptr(), buf.as_mut_ptr().add(pos), 8) }`, no bounds check |
| `Encoder::wrap` | `car_example.rs:4937-4948` | sets `pos = body_pos + 45` with no check; every subsequent setter uses `get_unchecked_mut` (`:5142-5149`) |
| `Decoder::wrap` | `car_example.rs:1211-1224` | no check; every accessor uses `read_bytes_unchecked` (`:1321-1324`) |

Each carries a `# Safety` section in its rustdoc, and each is documented as a
"Private zero-check … core (HFT-008 keep=false)" — but is emitted as `pub fn`,
not private and not `unsafe`
(`sbe/src/codegen/message_encoder.rs:349-355`, `:388-394`;
`sbe/src/codegen/message_decoder.rs:381-401`).

Concretely, this is safe code that corrupts memory:

```rust
let mut buf = [0u8; 2];
let _ = CarEncoder::wrap_and_apply_header(&mut buf, 0); // writes 8 bytes into 2
```

A `# Safety` section on a safe `fn` is a contradiction: safety preconditions
are only expressible on `unsafe fn`. Today the compiler cannot help a user who
picks the shorter name, and the shorter name is the one every doc example and
every benchmark uses.

**Change.** Invert the naming so the safe name is the safe function — which is
what all four documents already claim ships (see QW-4):

| Today | Target |
|---|---|
| `try_wrap` (checked, `Result`) | `wrap` (checked, `Result`) |
| `try_wrap_and_apply_header` (checked, `Result`) | `wrap_and_apply_header` (checked, `Result`) |
| `try_decode` (checked, `Result`) | `decode` (checked, `Result`) |
| `wrap` (infallible, unchecked) | `unsafe fn wrap_unchecked` |
| `wrap_and_apply_header` (infallible, unchecked) | `unsafe fn wrap_and_apply_header_unchecked` |
| `decode` (unchecked read, validates ids) | `unsafe fn decode_unchecked` |

Emission sites: `message_encoder.rs:338-394`, `message_decoder.rs:350-401`,
`:444-505`, and the `AnyMessage` twins in `runtime.rs:1893-1934`.

**SBE-pattern rationale.** sbe-tool's Rust `wrap` is infallible and unchecked,
so ergon needs an equally cheap path to stay at parity on the gated benchmarks —
that path must exist. But Rust has a first-class way to express "cheap, and the
caller owns the precondition", and it is `unsafe fn`. Keeping the trusted core
public-and-safe gives sbe-tool's ergonomics *and* sbe-tool's lack of guarantees;
making it `unsafe fn` keeps the performance while making the trust transfer
explicit at every call site. `docs/SBE_COMPATIBILITY.md:61-66` already describes
exactly this two-lane model ("Checked (safe)" / "Trusted (unsafe)") — the
generated code simply does not implement it.

**Zero performance cost.** `unsafe fn` is a compile-time marker; the emitted
machine code is byte-identical. Nothing is added to any hot path. Benchmarks
keep calling the same infallible core, now inside an `unsafe { }` block —
`CLAUDE.md`'s benchmark rule ("always use the infallible constructor, never
`try_*`, in timed paths") remains satisfiable and must be re-worded to name the
new `*_unchecked` spelling.

**What breaks.** Every call site of `wrap` / `wrap_and_apply_header` / `decode`
and of `try_wrap*` / `try_decode`. In the repository that is: all of
`sbe/benchmarks/benches/` (ten files), `samples/` (notably
`samples/cluster-rfq/`, `samples/sbe-feature-tour/`), `cluster/`, the book
examples, and `docs/MIGRATION_0_1_TO_0_1_10.md`. Mechanical, but wide — budget
for it. Breaking API changes are in scope for 0.1.12; wire format is untouched.

**What it buys.** The most dangerous function in the generated API stops being
callable by accident. A reviewer grepping for `unsafe` in application code now
sees every place a codec's bounds guarantee was waived — today those sites are
invisible.

**Blocking prerequisite — test assertions must be changed deliberately.**
`sbe/tests/hft_001_soundness_test.rs` encodes the *current* decision and will
fail:
- `:55` comment "No unsafe in the generated public API surface"
- `:57-74` assert `pub fn try_wrap_and_apply_header`, `pub fn wrap_and_apply_header`,
  `pub fn try_decode`, `pub fn wrap` all exist with those exact spellings
- `:128-136` assert `pub unsafe fn` count ≤ `_unchecked` string-accessor count

Per `CLAUDE.md`, tests are never edited to go green. These are not parity tests,
but they are deliberate, so **get explicit user sign-off before touching them**,
and rewrite them to assert the *new* invariant (every zero-check constructor is
`unsafe fn`; every safe constructor returns `Result`) rather than deleting them.
If sign-off is refused, stop and report — do not ship a partial rename.

**Acceptance criteria.**
1. Generated API: no safe `pub fn` retains a `# Safety` rustdoc section. Add a
   codegen-level test asserting this (grep the generated source for a `# Safety`
   heading not preceded by `pub unsafe fn`).
2. `hft_001_soundness_test` rewritten to the new invariant, with sign-off.
3. Golden regenerated; wire parity tests unchanged and green.
4. QW-4's documentation rewrite reflects the final state.
5. `docs/MIGRATION_0_1_TO_0_1_10.md` superseded by a new
   `docs/MIGRATION_0_1_11_TO_0_1_12.md` with the rename table above.
6. CHANGELOG "Breaking" entry.

**Verification plan.** `cargo test -p ergo-sbe` and `just test`. Then `just bench`
both profiles — every gated ratio must be **unchanged**, since the machine code
is identical. A moved ratio means a benchmark arm accidentally switched from the
unchecked core to the checked one; per `CLAUDE.md`'s fairness rules that would
be an invalid comparison against sbe-tool's unchecked `wrap`, so fix the
benchmark, not the ceiling. Record Criterion regression estimates before and
after to prove the no-op.

---

## T-3: Add an allocation-free bulk group decode

- Type: PERF · Stage: 0.1.12 · Priority: P1 · Effort: M

**Symptom.** `bulk_decode` is the only generated method that allocates:
`sbe/src/codegen/group_decoder.rs:272-295` emits
`pub fn bulk_decode(&mut self) -> Result<Vec<Entry>, DecodeError>` with
`Vec::with_capacity(cap)` inside. `CLAUDE.md` states generated hot paths
allocate no heap memory; this one does, once per group per message, and it is
the API the DTO/snapshot path steers users toward. It also lacks `#[inline]`.

**Change.** Add
`bulk_decode_into(&mut self, dst: &mut Vec<Entry>) -> Result<usize, DecodeError>`
that clears and extends the caller's buffer, and keep `bulk_decode` as a thin
wrapper over it for the convenience case. Emit both from `group_decoder.rs:272`.
Mirror the existing encode-side precedent: `bulk_add(&[Entry])` already writes
into a caller-owned region rather than allocating (golden `:6536`).

**Mechanism.** Removes one allocation + one deallocation per call. A consumer
decoding a 1,000-entry book at message rate currently performs one
`malloc`/`free` pair per message; with a reused `Vec` it performs none after
warm-up. The single bounds check for the whole batch (`group_decoder.rs:279`)
is retained unchanged — no validation is added or removed.

**Evidence plan.** Add a Criterion pair to
`sbe/benchmarks/benches/group_encode_bench.rs`'s decode counterpart (or a new
`group_decode_bench.rs`): `bulk_decode` vs `bulk_decode_into` with a pre-sized
reused `Vec`, at 1,000 entries, both arms decoding the identical fixture and
asserting identical values before timing. Per `CLAUDE.md`, pre-size the buffer
once outside `b.iter` — do **not** use `iter_batched` with a closure that
allocates. Expect the delta to equal one allocator round-trip per iteration;
confirm with an allocation-count assertion in
`sbe/tests/allocation_count_test.rs`, which already has the harness for exactly
this (three allocation-count tests are tracked there).

**What breaks.** Nothing. `bulk_decode` keeps its signature. This is additive —
listed here rather than as a quick win only because it needs a benchmark and an
allocation-count test.

**What it buys.** A zero-allocation path for the one generated API that had
none, without forcing users off the convenient one.

**Acceptance criteria.** New method emitted with `#[inline]`; allocation-count
test proves zero allocations for the `_into` variant across repeated calls;
golden regenerated; new benchmark passes `fairness_policy_test` (black_box,
pre-timing value assertions, no timed allocation).

**Verification plan.** `cargo test -p ergo-sbe --test allocation_count_test`;
`just bench` — the ten maintained ergon/sbe-tool ratios are untouched by this
addition and must stay at or below `1.00`. The new pair is an
ergon-vs-ergon self-comparison, so it does **not** get a gate entry against
sbe-tool (sbe-tool has no equivalent bulk API); record it as a diagnostic.

---

## T-4: Publish the missing 0.1.11 changelog section

- Type: DOCS · Stage: 0.1.12 · Priority: P1 · Effort: M

**Symptom.** `CHANGELOG.md` jumps from `[0.1.10] — 2026-08-02` (line 3) to
`[0.1.9]` (line 40). There is no `0.1.11` section, yet `9f46bc3c` is
"feat: ergon v0.1.11 — get_metadata() zero-copy, keyword handling, error quality
tests" and the workspace is now at `0.1.12` (`Cargo.toml:21`). A released
version has no user-facing record of what changed.

The gap is not cosmetic — `get_metadata()` is a **breaking** relocation of the
decoder's utility methods behind a metadata view (golden
`car_example.rs:1080-1147`), motivated by schema-field name collisions, and no
document explains the migration. It is mentioned only in passing in
`book/src/sbe/feature-tour/generated-code.md` and
`book/src/cluster/chained-decoding.md`.

**Change.**
1. Reconstruct the `[0.1.11]` section from `git log 4ccb5e80..9f46bc3c` and
   `git show 9f46bc3c --stat`, following the existing house style: imperative
   mood, user-facing entries only, categorised Added/Changed/Fixed/Breaking by
   what the change *is*.
2. Add a short book subsection under
   `book/src/sbe/core-concepts/` (new page, linked from `SUMMARY.md:23-28`)
   explaining the `get_metadata()` seam: why buffer/limit/frame accessors live
   on a separate zero-copy view (so no schema field name can collide with a
   utility method), and how to migrate `dec.limit()` → `dec.get_metadata().limit()`.
3. Refresh `sbe/BENCHMARKS.md`'s "Latest run" block — it still reads
   "Release 0.1.10", two releases stale.

**Definition of done.** A user upgrading 0.1.10 → 0.1.12 can read `CHANGELOG.md`
alone and find every breaking change, including `get_metadata()`. The book has a
page that answers "why is `limit()` not on the decoder any more?".

**Acceptance criteria.** `[0.1.11]` section present with a Breaking subsection
naming `get_metadata()`; new book page linked from `SUMMARY.md`;
`docs_validation_test` and `just book-ci` green (the docs gate compiles bare
`rust` fences, so any example must compile).

**Verification plan.** `just book-ci`;
`cargo test -p ergo-sbe --test docs_validation_test`. Cross-check the
reconstructed entries against `git show 9f46bc3c --stat` so nothing user-facing
is omitted.

---

## T-5: Document the random-access limitation on groups with nested tails

- Type: DOCS · Stage: 0.1.12 · Priority: P2 · Effort: S

**Symptom.** `nth()` is emitted only when a group's entries have no nested tail:
`sbe/src/codegen/group_decoder.rs:332` guards the emission with
`if total_tail == 0`. A user with a nested-group schema calls `nth()` on the
flat group, finds it, then calls it on the nested one and gets a compile error
with no explanation. The reason is inherent to SBE — entries containing
groups or var-data have no constant stride, so index *n* cannot be computed
without walking entries `0..n` — but nothing in the book says so.

**Change.** Add the rule to
`book/src/sbe/feature-tour/decode-stages.md`: fixed-shape group entries get
`nth()` (O(1), constant stride); entries with nested groups or var-data do not,
and must be reached with the iterator or `skip_n()`. State the wire reason.
Also add it to the generated rustdoc on the group decoder so it appears in
`cargo doc` for the type that lacks the method — emit a doc line on the group
decoder struct in the `total_tail != 0` branch explaining why `nth` is absent
and pointing at `skip_n` (`group_decoder.rs:298-328`).

**Definition of done.** A user who cannot find `nth()` learns from the type's
own rustdoc why it does not exist and what to use instead.

**Acceptance criteria.** Book page updated; golden shows the explanatory doc
comment on a nested-tail group decoder; `just book-ci` green.

**Verification plan.** `just book-ci`, `cargo doc -p ergo-sbe --no-deps`, golden
regenerated.

---

# 3. Deferred to 1.0

## T-6: Reconsider the four-positional-argument `Decoder::wrap` signature

- Type: API · Stage: 1.0 · Priority: P2 · Effort: M
- **Symptom:** `wrap(buf, message_offset, acting_block_length, acting_version)`
  (golden `car_example.rs:1211-1216`) takes two adjacent `usize` parameters —
  `message_offset` and `acting_block_length` — which can be transposed silently.
  The result is not a compile error and not a runtime error; it decodes garbage
  at a valid-looking offset.
- **Change:** wrap the wire metadata in a small `ActingHeader { block_length,
  version }` value produced by the header decoder, reducing the call to
  `wrap(buf, message_offset, acting)`.
- **Why 1.0 and not 0.1.12:** T-2 already rewrites every constructor call site
  in the tree. Doing both in one release makes the migration guide hard to
  follow and the regression surface hard to attribute. Land T-2 in 0.1.12, let
  it settle for a release, then do this.
- **Acceptance:** golden regenerated; migration note; wire parity green.
- **Verification:** `just bench` — the struct is two fields passed by value, so
  the gated `decode_entry_point` ratio must not move; if it does, the parameter
  is being spilled and the change is rejected.

---

# 4. Roadmap cross-check

Against `book/src/project/road-to-1.0.md`.

| Ticket | Status vs roadmap |
|---|---|
| QW-1, QW-2, QW-3 | **Already planned.** Criterion 2 requires every maintained ratio at or below `1.00` under the published LTO matrix for three consecutive minors. Missing `#[inline]` is the known mechanism by which the no-LTO half of that matrix fails (`CHANGELOG.md:236-242`). |
| QW-4 | **Already planned.** Criterion 5 requires published book chapters for trust boundaries. It is currently published and wrong, which is worse than absent. |
| QW-5 | **New.** No roadmap criterion covers evidence-manifest integrity, though criterion 4 ("no known P0 safety issues open") depends on the keep-manifest being real. |
| QW-6, QW-7 | **Already planned.** Criterion 1, API freeze audit — both are renames/removals that must happen before the freeze, not after. |
| QW-8 | **New.** |
| **T-1** | **Already planned**, and blocking. Criterion 4 requires the fuzz corpus on decode entry to stay green and no known P0 safety issues open. A reachable panic in a documented-fallible accessor is a P0 that the current corpus does not reach. |
| **T-2** | **Already planned**, and blocking. Criterion 1 (API freeze: "no pending renames of generated stage / wrap / FixedFields surface without a major") — this is exactly that rename, and it must land before 1.0 or never. Criterion 4 also applies: safe-code UB is a P0 safety issue. |
| T-3 | **New.** No roadmap criterion mentions allocation behaviour, though `CLAUDE.md` treats allocation-free hot paths as a load-bearing invariant. |
| T-4 | **New.** The roadmap assumes a per-release record exists; nothing enforces it. Worth adding "changelog section published" to the release skill's gate. |
| T-5 | **Already planned.** Criterion 5, docs completeness. |
| T-6 | **Already planned.** Criterion 1, API freeze audit — explicitly deferred here to keep 0.1.12's migration coherent. |

**Suggested roadmap amendment.** Criterion 2 counts three consecutive released
minors at or below the ceiling. T-2 rewrites every benchmark call site; although
the machine code is identical, the benchmark *sources* change, so record the
before/after Criterion estimates in the 0.1.12 release notes to keep the
three-release chain auditable rather than restarting it.
