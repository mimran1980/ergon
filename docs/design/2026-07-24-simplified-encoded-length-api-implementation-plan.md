# Simplified SBE Encoded-Length Interface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate the smallest exact-length interface justified by each SBE message shape: constants for fixed messages, direct helpers for structurally flat tails, and a zero-allocation fluent builder for nested groups or group-entry varData, including concise uniform-shape syntax and an explicit ragged-shape path.

**Architecture:** Add a schema-shape classifier at the encoded-length code-generation seam. Directly computable messages keep arithmetic helpers and do not generate builder types; structurally dynamic messages generate concrete consuming stages backed by a small checked length accumulator. Uniform group methods multiply one declared entry shape by the group count, while ragged and unknown-size methods use closures whose completed entry shapes are counted automatically.

**Tech Stack:** Rust 2024, stable Rust, `syn`, `quote`, `proc_macro2`, `prettyplease`, `roxmltree`, SBE XML fixtures, Cargo integration tests, Criterion/Aeron parity benchmarks

## Implementation audit — 2026-07-24

This section records the review of the implementation in commits
`0380323..f6899f9`, using `644f93d` as the pre-implementation baseline. It
supersedes commit-message claims about completed tasks. The detailed task
instructions later in this document remain the implementation playbook, but a
task is complete only when the evidence and acceptance checks in this audit are
green.

No product implementation was changed during this audit. The only intended
edit is this plan.

### Audit conclusion

The three-strategy foundation is useful and should be retained:

- fixed-only messages omit length builders;
- directly computable tails use encoder helpers and omit length builders;
- structurally dynamic messages receive a stack-only staged builder;
- the checked accumulator is emitted once per generated schema module;
- the narrow uniform case represented by one dynamic entry tail is concise and
  uses weighted accumulation.

The feature is **not implementation-complete or merge-ready**. ~~Known ragged and
unknown-size builders can return incorrect lengths~~ (**FIXED 2026-07-25**:
ragged arithmetic corrected — per-entry nested groups scale by parent multiplier,
not `pm×count`; unknown-size verified correct by `l3book_unknown_size_length_matches_encoded`).
~~Recursive entry-tail generation does not cover the repository's actual schema
combinations~~ (**FIXED 2026-07-25**: recursive `generate_ragged_wrappers` handles
arbitrary depth — verified by `depth3_staged_length_matches_encoded`).
The new test matrix executes staged length-builder chains (8 tests, all green).
Several existing exact-length assertions were replaced with oversized buffers or
positivity checks — **all replaced with staged-builder-computed exact sizes**
(zero oversized buffers in src or tests). The mandatory performance gate
remains open (decode 1.97×, encode 1.18× — bounds-check overhead, not arithmetic).

**Do not extend or document the current generic `RaggedEntryBuilder` as if it
were the final design.** (**DONE**: replaced with schema-specific ragged wrapper
types that have field-named methods — `og.add()?.order_id(len)?` instead of
`og.var_data(prefix, len)?`; zero user-derived constants.)

### Task status at the audit point

| Task | Status | Verified implementation | Remaining work |
|---|---|---|---|
| 1. Length-strategy classifier | **Complete** | `LengthStrategy::{Fixed, Direct, Staged}` and representative repository-shape unit tests exist in `sbe/src/codegen/encoded_length.rs`. | Add new classifier cases only when new structural fixtures expose a missed shape. |
| 2. Direct helpers | **Partial** | Direct-only generation, typed group counts, checked arithmetic, varData validation, and one exact direct encode comparison exist. | Make both checked helpers const-capable; add custom-header, endian, boundary, decoder, DTO, and exact-buffer coverage. |
| 3. Checked accumulator | **Partial** | The stack-only accumulator is emitted once and uses checked multiply/add internally. | Add direct accumulator behavior tests; use or remove the currently unused `finish`; prevent unchecked header addition at the complete stage. |
| 4. Uniform staged path | **Partial** | One-level uniform entry varData and one nested-tail shape use concise closure-free transitions. | Generate the complete ordered entry-tail graph recursively: nested group followed by parent varData, multiple varData fields, sibling groups, and depth-three or deeper nesting. Add real compile-and-run and exact-wire tests. |
| 5. Known ragged path | **Complete** (2026-07-25) | Schema-specific ragged wrapper types with field-named methods (`og.add()?.order_id(len)?`). Arithmetic verified by `l3book_staged_length_matches_encoded` (256 bytes exact). `uniform(count)` shortcut for identical entries. | — |
| 6. Unknown-size paths | **Complete** (2026-07-25) | `*_unknown_size` methods use correct per-entry counting. Verified by `l3book_unknown_size_length_matches_encoded`. | — |
| 7. Empty forwarding and collisions | **Partial** | `finish_empty()` exists and rejects a non-zero declared count. | Make it const; add normal zero forwarding; suppress forwarding on actual method collisions; compile and run the collision fixture; add zero-pending terminals. |
| 8. Comprehensive conformance matrix | **Partial foundation** | The one-temp-crate/many-tests runner exists and the direct flat case encodes once. | Build the full staged matrix. Current staged tests only search source text and do not call a generated builder. Add codec/DTO/decoder cross-products, exact buffers, compile-fail cases, error cases, allocation proof, and matrix cardinality assertions. |
| 9. Caller migration | **Complete** (2026-07-25) | All callers use staged-builder-computed exact sizes. Zero oversized buffers. L3 sample compares builder, encoder, bytes, decoder, and DTO (byte-identical round-trip). | — |
| 10. Production schemas | **Partial** | Five sources are parsed with `syn`. | Fail when required fixtures are missing; include L3, conformance, cluster, and custom-header schemas; type-check generated crates rather than merely parsing their syntax. |
| 11. Clean API audit | **Partial** | Basic substring audits and formatting idempotence exist. | Implement the promised `syn` method/visibility index, compile-pass/fail snippets, const audit, collision audit, and a real automatically compared API golden. Delete the unused legacy length-builder generator. |
| 12. Car golden | **Partial** | The generated Car source is stable against its golden. | Regenerate after the staged redesign; add length-specific allocation and API checks. The separate signature golden is currently inaccurate and untested. |
| 13. Documentation | **Complete** (2026-07-25) | Ragged/unknown-size docs use field-named wrapper methods. Examples compile and are tested. | — |
| 14. Full gates | **Not done; currently failing** | Formatting, focused API tests, conformance, L3, stability, ordered decoder tests, allocation tests, and the L3 sample were run during this audit. | Make the complete suite, strict Clippy, domain-object suite, all samples, product gate, and three clean benchmark sessions pass. `just bench` currently fails a maintained ratio. |

### Critical correctness findings

#### P0. Known ragged arithmetic is incorrect

Current code enters the outer group with its full declared count before the
closure:

```text
outer multiplier = parent multiplier × declared outer count
```

Every subsequent generic `group(...)` or `var_data(...)` operation for an
individual ragged entry is therefore scaled by the entire outer count. For two
entries with different shapes, the current calculation is effectively:

```text
outer_count × (entry_1_dynamic_tail + entry_2_dynamic_tail)
```

The required calculation is:

```text
entry_1_dynamic_tail + entry_2_dynamic_tail
```

`RaggedEntryBuilder::add()` increments `written` but adds zero bytes. The
already-added outer fixed-block contribution happens to cover fixed entry
blocks for a known count, but the entry-specific dynamic contributions are
wrong.

Required redesign:

- retain the parent multiplier while entering a single ragged entry;
- add that entry's fixed block exactly once;
- expose its ordered nested groups and varData through schema-specific borrowed
  stages;
- increment the outer group's completed-entry count only when the entry reaches
  its terminal stage;
- add each entry's dynamic tail at the parent multiplier, not at
  `parent × declared_count`;
- compare completed entries with the declared count when the closure returns.

#### P0. Unknown-size arithmetic omits entry bytes

Current code calls `enter_group(0, ...)`. This sets the multiplier to zero.
`add()` then contributes no fixed block, while nested groups and varData are
also multiplied by zero. The method can return success with only the group
dimension contribution included.

Unknown-size must use the same completed-entry state machine as known ragged:

- add the group dimension once;
- start each entry with the parent multiplier;
- add the fixed entry block and all completed dynamic tails once;
- increment a checked `written` count only at terminal completion;
- reject `written > <wire count type>::MAX`;
- for flat fixed-width entries, provide `entries(n)` so callers do not loop or
  call `add()`.

#### P0. Uniform generation does not model complete entry tail order

The current implementation special-cases immediate children rather than
recursively generating an owner-tail state graph:

- a flat nested group returns directly to the next message-level stage, so a
  parent entry shaped `orders → venue` cannot express both tails;
- each of several parent-entry varData methods independently returns to the
  outer continuation, so two varData fields cannot be chained;
- a dynamic nested group generates only its immediate varData methods and
  ignores deeper nested groups;
- a child nested group followed by parent-entry varData cannot return to the
  correct parent-entry continuation;
- depth-three `x → y → z` structures dead-end before completion.

Replace these special cases with one recursive generator operating on an
ordered owner-tail model shared conceptually with encoder/decoder generation.
Every owner—message or group entry—needs:

1. an initial stage;
2. one transition for each nested group in wire order;
3. one transition for each varData field in wire order;
4. a complete stage that returns to its owning parent with the correct
   multiplier and completion action.

#### P0. Complete-stage header addition bypasses checked arithmetic

`EncodedLengthAccumulator` has a checked `finish(header_length)`, but generated
complete stages perform plain `self.state.len + HEADER_LENGTH`. An extreme
length can panic in debug or wrap in release at the final operation.

Choose one consistent terminal contract:

- preferably validate the header-inclusive total before constructing the
  infallible complete stage, retaining non-fallible
  `encoded_length_with_header()`; or
- make terminal retrieval fallible everywhere.

The first option preserves the requested fluent API. Test body overflow and
header-only overflow separately.

### Test audit

The focused command

```sh
cargo test -p ergo-sbe --test encoded_length_api_test -- --test-threads=1
```

passes 18 tests, but that does not prove the staged feature:

- the only runtime exact encode comparison uses the simple direct helper;
- uniform Car, L3, and nested-group cases only check that source contains type
  names;
- no test invokes `{Message}EncodedLength::new()` outside generated golden
  source;
- no ragged or unknown-size result is compared with encoder output;
- no staged exact-size buffer is allocated;
- no staged DTO/domain or decoder length is compared;
- no compile-fail type-state proof exists;
- no length-calculation allocation count is measured;
- the collision fixture is not referenced;
- production sources are parsed but not compiled;
- the API signature golden is never loaded by a test and lists direct Car
  helpers that are not generated.

The `no_add_or_add_n_in_staged_builders` check is also insufficient. It is a
line-neighbor substring scan and explicitly permits `RaggedEntryBuilder::add()`,
which is the boilerplate this design is meant to remove.

### Required conformance matrix

Implement the matrix test-first. Each applicable row must execute generated
consumer code; source-presence assertions may supplement but never replace
behavior.

| Axis | Required cases |
|---|---|
| Length interface | fixed constant; compatibility direct helper; checked direct helper; uniform known-size builder; known ragged builder; unknown-size builder; zero forwarding; `finish_empty()` |
| Fixed-field encoder mode | `fixed(&FixedFields)`; `raw_fixed()` with individual setters; any supported whole-entry struct/add-struct encoder path |
| Tail encoder mode | known-size closures; encoder `*_unknown_size`; fixed leaf `add_struct`; individual field setters; mixed known/unknown nested groups |
| Decoder verification | initial decoder; consuming stages; nested entry decoder; decoder-reported full length; exact consumed tail order |
| DTO/domain verification | domain length; encode from domain; decode into domain; byte identity against flyweight encoding |
| Shape | fixed-only; message varData; one and multiple flat groups; entry varData; multiple entry varData; nested fixed; nested dynamic; nested + parent varData; sibling nested groups; depth three; two top-level staged groups |
| Cardinality | zero; one; several; maximum practical `u8`; `u8` overflow; known count too few; known count too many |
| VarData | zero; one byte; differing ragged lengths; declared maximum; one over maximum; non-`u32` length prefix |
| Schema metadata | default header; custom header; little endian; big endian; single-message and multi-message generated modules |

For every successful logical payload, assert all applicable invariants:

```text
precomputed body length
    == encoder encoded_length()
    == decoder body length
    == DTO/domain body length

precomputed header-inclusive length
    == encoder encoded_length_with_header()
    == encoder as_bytes().len()
    == decoder header-inclusive length
    == DTO/domain full length
    == exact allocated buffer length
```

Also require:

- an exact buffer succeeds;
- a one-byte-short buffer returns `BufferTooShort`;
- a one-byte-long buffer encodes the same message length and `as_bytes()` does
  not expose the unused byte;
- exact wire bytes match between `fixed`, `raw_fixed`, individual field,
  entry-struct, and domain-object paths where all are applicable;
- compile-fail cases prove incomplete and out-of-order length stages cannot
  report a complete length;
- measured length calculation performs zero heap allocations;
- the generated number of logical cases is asserted so a loop/filter mistake
  cannot silently shrink the matrix.

### First tests to add before changing the generator

Add these as named `GeneratedRustTest` cases, grouped into a small number of
temporary crates:

- `uniform_l3_fixed_orders_exact`;
- `uniform_l3_vardata_orders_exact`;
- `uniform_nested_group_then_parent_vardata_exact`;
- `uniform_two_entry_vardata_fields_exact`;
- `uniform_child_vardata_then_parent_vardata_exact`;
- `uniform_depth_three_exact`;
- `ragged_outer_entries_exact`;
- `ragged_inner_entries_exact`;
- `ragged_known_too_few`;
- `ragged_known_too_many`;
- `unknown_outer_entries_exact`;
- `unknown_inner_entries_exact`;
- `unknown_u8_count_max`;
- `unknown_u8_count_overflow`;
- `unknown_flat_entries_bulk_exact`;
- `zero_forward_to_next_group`;
- `zero_forward_to_message_vardata`;
- `zero_finish_empty_exact`;
- `zero_nonempty_finish_rejected`;
- `zero_forward_collision_requires_finish_empty`;
- `custom_header_exact`;
- `big_endian_structural_length_matches_little_endian`;
- `exact_buffer_and_one_short`;
- `fixed_raw_domain_decoder_equivalence`;
- `length_builder_allocates_zero`.

Each “exact” test must encode and compare lengths. A test that only checks
`len > 0` is not an encoded-length conformance test.

### Revised remaining implementation sequence

This sequence replaces the optimistic “Tasks 1–9 complete” claim in commit
`02ee223`. Preserve the completed classifier and direct-helper seam while
reworking the staged internals.

#### Phase A — establish red tests

- [ ] Add the compile-and-run cases listed above for all representative
  repository schemas.
- [ ] Restore the removed L3 exact-length tests before changing arithmetic.
- [ ] Make README examples mirror named compile tests.
- [ ] Add explicit expected-failure evidence for current ragged,
  unknown-size, multi-tail, and depth-three behavior.
- [ ] Add a structural matrix cardinality assertion.

#### Phase B — generate an ordered recursive stage graph

- [ ] Introduce one internal owner-tail description for nested groups and
  varData in wire order.
- [ ] Generate uniform stages recursively for arbitrary depth.
- [ ] Return from a completed child group to the next parent-entry tail, not
  directly to the next message tail.
- [ ] Support multiple sibling nested groups and multiple varData fields.
- [ ] Keep all generated state stack-only and concrete; do not allocate a
  runtime descriptor tree.
- [ ] Give stages names based on the component actually consumed
  (`AfterFuelFigures`), rather than the next component not yet processed.

#### Phase C — replace the ragged placeholder

- [ ] Delete the public generic `RaggedEntryBuilder` API.
- [ ] Generate group-specific borrowed entry stages with field-named methods.
- [ ] Start an entry automatically when its first required shape method is
  called.
- [ ] Add the entry fixed block once.
- [ ] Mark an entry complete only when its final nested/varData tail completes.
- [ ] Validate known declared counts after the closure.
- [ ] Prove outer-ragged/inner-uniform and outer-uniform/inner-ragged cases.
- [ ] Prove differing nested counts and differing varData lengths in the same
  outer group.

#### Phase D — implement unknown-size and zero behavior

- [ ] Reuse the completed-entry mechanism for `*_unknown_size`.
- [ ] Add checked count accumulation using the resolved wire count primitive.
- [ ] Add `entries(n)` for fixed-width unknown-size groups.
- [ ] Emit const zero-forwarding methods only when no method-name collision
  exists.
- [ ] Keep `finish_empty()` always available and make it `const fn`.
- [ ] Add fallible pending terminals so a skipped non-empty shape cannot
  silently succeed.
- [ ] Compile and run
  `sbe/tests/fixtures/schemas/encoded-length-method-collision.xml`.

#### Phase E — finish checked and const behavior

- [ ] Mark `try_compute_encoded_length` as `const fn`.
- [ ] Rewrite `try_compute_encoded_length_with_header` with explicit
  `match`/early return and mark it `const fn`.
- [ ] Route completion through checked header-inclusive finalization.
- [ ] Add const compile proofs for direct, uniform nested, zero-forward, and
  `finish_empty()` chains.
- [ ] Keep closure-taking ragged/unknown methods runtime-only.
- [ ] Remove unused accumulator methods or exercise them in the final design.

#### Phase F — restore and expand integrations

- [ ] Restore exact-size allocation in the L3 sample `main`.
- [ ] Restore exact builder equality in L3 fixed-order and nested-varData
  tests.
- [ ] Restore exact builder equality in baseline, conformance, ordered-decoder,
  and domain-object tests changed by this branch.
- [ ] Cross the length modes with `fixed`, `raw_fixed`, individual field,
  decoder, and domain-object paths.
- [ ] Compile representative production schemas as external crates.
- [ ] Fail rather than silently skip a required production fixture.

#### Phase G — clean generated and documented interfaces

- [ ] Remove the unused legacy `generate_encoded_length_builder()` and
  `has_nested_dynamic_tail()` code from `sbe/src/codegen/mod.rs`.
- [ ] Build a real inherent-method and visibility index with `syn`.
- [ ] Assert no generated length type exposes `add()` or `add_n()`.
- [ ] Assert plumbing is `#[doc(hidden)]` and entry/complete surfaces are
  intentional.
- [ ] Generate `encoded_length_api.txt` from the AST and compare it in a normal
  test.
- [ ] Provide an ignored updater test whose name and command actually exist.
- [ ] Correct README examples only after their mirrored compile tests pass.
- [ ] Re-run formatting idempotence and the Car stability golden.

#### Phase H — acceptance gates

- [ ] Run every focused test in Task 14.
- [ ] Run the complete all-feature SBE suite.
- [ ] Run strict Clippy for the crate and sample.
- [ ] Run all offline sample checks.
- [ ] Run the product gate.
- [ ] Run three clean SBE benchmark sessions and require every maintained
  ergon/Aeron ratio to be `<= 1.00`.
- [ ] Record the exact toolchain, host, medians, confidence intervals, and
  ratios.
- [ ] Review the final public generated API diff separately from encoder and
  decoder output.

### Const-method audit

The requested constification is only partially implemented.

| Method/category | Current state | Plan |
|---|---|---|
| Existing unchecked compatibility `compute_encoded_length*` | const | Keep const for compatibility. |
| Checked direct `try_compute_encoded_length` | runtime despite const-compatible body | Mark `const fn`; retain explicit checked matches. |
| Checked direct header helper | runtime and uses `?` | Replace `?` with `match`, then mark `const fn`. |
| Accumulator `new`, `multiplier`, `add_scaled`, `enter_group`, `leave_group`, `fail`, `check`, `finish` | const | Keep const; ensure every retained method is used or tested. |
| Builder `new` | const | Keep const. |
| Uniform known-size group transitions | const | Keep const after recursive redesign. |
| Uniform varData and flat nested completion | const | Keep const and add compile proofs. |
| `finish_empty()` | runtime though implementation is const-compatible | Mark `const fn`. |
| Zero-forward and pending terminal methods | absent | Generate as const-capable methods. |
| Complete length getters | const, but header addition unchecked | Preserve const after moving overflow validation before complete-stage construction. |
| Ragged and unknown closure-taking methods | runtime | Keep runtime-only. |
| Borrowed ragged entry methods | runtime-only path | Const qualification is not a user benefit; prioritize correctness and typed API. |
| Domain-object length methods | runtime iteration | Keep runtime-only. |

Do not broaden this work into unrelated encoder/decoder constification.

### Generated-code and documentation improvements

- Remove stale generator code rather than keeping two encoded-length
  implementations in `sbe/src/codegen/mod.rs` and
  `sbe/src/codegen/encoded_length.rs`.
- Replace comments containing “simplified”, “follow-up”, or internal scratch
  language with finished API documentation only after behavior exists.
- Avoid generic public methods that require callers to supply wire dimensions
  or block sizes (`group(dim, block, count)` and
  `var_data(prefix, byte_len)`). Those are generator facts, not user inputs.
- Narrow broad `#![allow(clippy::all, ...)]` usage in new tests.
- Make new matrix-runner unit tests return `Result` and use `?` consistently.
- Stop silently skipping required fixture files in production-schema tests.
- Make API-stage names describe completed work so compiler diagnostics are
  understandable.
- Reconcile this plan's location with the newer repository contribution rule
  that active feature specs live under `.scratch/<feature>/spec.md`; do not
  create a duplicate plan during the implementation pass.

### Verification evidence from this audit

Environment:

```text
Date: 2026-07-24
Host: macmini.local, Apple arm64, Darwin 25.5.0
Rust: rustc 1.95.0 (59807616e 2026-04-14)
Baseline: 644f93d
Reviewed HEAD: f6899f9
```

Passed:

- `git diff --check 644f93d...HEAD`
- `cargo fmt --all -- --check`
- encoded-length classifier unit test: 1 passed
- `encoded_length_api_test`: 18 passed
- `stability_test`: 4 passed, 1 ignored updater
- `conformance_test`: 19 passed
- `l3_orderbook_test`: 9 passed
- `allocation_count_test`: 7 passed, but none measures length calculation
- `ordered_decoder_stages_test`: 6 passed
- L3 sample tests: 4 passed

Failed or incomplete:

- full `cargo test -p ergo-sbe --all-features -- --test-threads=1` stops at the
  pre-existing `bounds_checking_switch` assertion: 98 passed, 1 failed in that
  integration target;
- `domain_objects_test`: 14 passed, 1 pre-existing Binance generated-domain
  compile failure;
- strict Clippy stops on pre-existing Rust-2024
  `unsafe_op_in_unsafe_fn` warnings promoted to errors;
- the L3 sample passes but emits warnings and no longer verifies a staged
  precomputed length;
- `just bench` exits 1. The gate reports
  `throughput/batch_10k = 8348.03 / 8083.79 = 1.0327`, above the allowed
  ratio; the strict local policy would also require rechecking the
  `decode_composite` ratio reported as `1.0032`;
- commit `02ee223` itself records only “7/8 bench ratios pass”, which is failure
  evidence rather than acceptance evidence;
- no three-session acceptance run has been completed; repeat three clean,
  uncontended sessions after correctness is restored.

These broader pre-existing failures must be tracked separately from the new
encoded-length defects, but Task 14 cannot be marked complete while the
repository gate remains red.

## Global Constraints

- Planning scope only for this document; implementation begins only in a separately authorised execution session.
- Preserve official SBE wire compatibility.
- Do not change generated encoder or decoder method names, arguments, return types, ordering, or wire behaviour.
- Length calculation must remain zero allocation at runtime and stack-only.
- Do not add dependencies.
- Use checked `usize` addition and multiplication on every new length-calculation path.
- Preserve known-group exact-count validation.
- Preserve unknown-size group count-width validation.
- Preserve varData maximum-length validation.
- Generated complete length stages expose body and header-inclusive lengths; incomplete stages cannot report a successful complete length.
- Fixed-only and flat-tail messages must not generate staged builder types.
- Complex messages must support uniform, ragged, known-size, unknown-size, empty, and arbitrarily nested group combinations.
- New and edited Rust tests return `Result<(), Box<dyn std::error::Error>>` and use `?` for fallible calls.
- Generated hot paths must not allocate.
- Mark direct arithmetic and closure-free uniform length methods `const fn`
  where Rust 1.95 permits it; do not force closure-based ragged/unknown paths
  through unstable const features.
- Any implementation change under `sbe/` must finish with `just bench`; every maintained ergon/Aeron ratio must be at or below `1.00`.
- Do not recreate deleted planning or backlog structures. This plan lives under `docs/design/` because the repository guide forbids recreating `docs/superpowers/`.

In this plan, “DTO” means the generator's existing owned
`{Message}Domain`/`{Group}EntryDomain` types enabled by
`GenerationConfig::enable_domain_objects()`.

---

## 1. Evidence from the repository schema inventory

A read-only structural scan covered these repo-owned schema roots:

- `sbe/tests/fixtures/**/*.xml`
- `samples/**/schemas/*.xml`
- `cluster/schemas/*.xml`

It found 136 XML files and 594 `<message>` declarations. The scan is structural,
so its count includes message declarations inside intentionally invalid test
fixtures; implementation acceptance must use the repository parser's existing
valid/invalid allow-lists rather than assuming every XML file is generatable.

| Structural shape | Messages | Length strategy |
|---|---:|---|
| Fixed fields only | 255 | Existing constants; no builder |
| Message-level varData only | 106 | Direct helper; no builder |
| One flat group | 97 | Direct helper; no builder |
| One flat group plus message varData | 14 | Direct helper; no builder |
| Multiple flat groups | 35 | Direct helper; no builder |
| Multiple flat groups plus message varData | 8 | Direct helper; no builder |
| Group-entry varData, no nesting | 35 | Staged builder |
| Group-entry varData plus message varData | 14 | Staged builder |
| Nested fixed groups | 4 | Staged builder |
| Nested fixed groups plus message varData | 3 | Staged builder |
| Nested groups with entry varData | 9 | Staged builder |
| Nested groups with entry and message varData | 14 | Staged builder |

The resulting split is:

- 255 fixed-only messages: 42.9%
- 260 directly computable dynamic messages: 43.8%
- 79 structurally dynamic messages requiring a builder: 13.3%

This is the reason to classify before generating an interface. Generating a
builder for every dynamic message makes the common 86.7% of examples learn a
staged interface they do not need.

### Representative schemas that must drive the implementation

| Shape | Existing schema and message | Required proof |
|---|---|---|
| Fixed-only | `sbe/tests/fixtures/schemas/basic-schema.xml` / `TestMessage50001` | Existing `BLOCK_LENGTH` and `ENCODED_LENGTH`; no length builder |
| Message varData only | `sbe/tests/fixtures/schemas/basic-variable-length-schema.xml` / `TestMessage1` | Direct checked helper with one length |
| One flat group | `sbe/tests/fixtures/schemas/basic-group-schema.xml` / `TestMessage1` | Direct checked helper with one count |
| Multiple flat groups plus varData | `sbe/tests/fixtures/conformance_schema.xml` / `FlatGroup` | Direct checked helper with two counts and one byte length |
| Entry varData | `sbe/tests/fixtures/schemas/group-with-data-schema.xml` / `TestMessage1`, `TestMessage2`, `TestMessage4` | Uniform and ragged entry lengths; one and multiple entry varData fields |
| Nested group plus parent and child varData | `sbe/tests/fixtures/schemas/group-with-data-schema.xml` / `TestMessage3` | Mixed nesting and staged continuation after a nested group |
| Two top-level nested groups | `sbe/tests/fixtures/schemas/l3-orderbook-schema.xml` / `L3Book` | Desired `bids → orders → asks` fluent interface and exact wire agreement |
| Nested fixed groups plus message varData | `samples/l3-book/schemas/l3-book.xml` / `L3Book` | Flat nested leaf completion and zero-count forwarding |
| Nested child varData plus message varData | `samples/l3-book/schemas/l3-book.xml` / `L3BookVarData` | Desired `.bids(2).orders(2).order_id(5)?` syntax |
| Three nesting levels | `sbe/tests/fixtures/schemas/nested-group-schema.xml` / `Top` | Recursive generation with `u8` counts |
| Multiple nested siblings at depth three | `samples/exchange-example/schemas/binance-spot.xml` / `ExchangeInfoResponse` | Production-scale source generation and method-name scoping |
| `u8` group and varData prefixes | `sbe/tests/fixtures/schemas/u8-dimension-schema.xml` / `CompactMsg` | Count and varData boundary validation |
| Big-endian generation | `sbe/tests/fixtures/schemas/example-bigendian-test-schema.xml` | Byte order must not alter arithmetic |
| Custom header | `sbe/tests/fixtures/schemas/custom-header-type.xml` | Header-inclusive length uses the resolved header size, never a literal `8` |

---

## 2. Chosen generated interface

### 2.1 Strategy A: fixed-only message

Do not generate `{Message}EncodedLength`.

Use the constants already generated on the encoder:

```rust
let body_len = TestMessage50001Encoder::BLOCK_LENGTH;
let full_len = TestMessage50001Encoder::ENCODED_LENGTH;
```

The implementation must preserve the existing fixed-message constants. It
must not add an object whose only purpose is to return a constant.

### 2.2 Strategy B: directly computable tail

A message is directly computable when every top-level group entry is fixed
width:

```rust
group.groups.is_empty() && group.var_data.is_empty()
```

The message may have zero, one, or many flat groups and zero, one, or many
message-level varData fields. The exact length is fully determined by:

- the message block length;
- one count per top-level group;
- one byte length per message-level varData field.

Do not generate `{Message}EncodedLength` for this strategy.

Keep the existing arithmetic-only compatibility methods unchanged:

```rust
pub const fn compute_encoded_length(
    bids_count: usize,
    asks_count: usize,
    description_len: usize,
) -> usize;

pub const fn compute_encoded_length_with_message_header(
    bids_count: usize,
    asks_count: usize,
    description_len: usize,
) -> usize;
```

Add checked preferred `const fn` methods whose group count types match each schema's
`numInGroup` primitive:

```rust
pub const fn try_compute_encoded_length(
    bids_count: u16,
    asks_count: u16,
    description_len: usize,
) -> Result<usize, sbe_rt::EncodeError>;

pub const fn try_compute_encoded_length_with_header(
    bids_count: u16,
    asks_count: u16,
    description_len: usize,
) -> Result<usize, sbe_rt::EncodeError>;
```

Example:

```rust
let len = FlatGroupEncoder::try_compute_encoded_length_with_header(2, 1, 17)?;
```

The checked methods:

1. start with `BLOCK_LENGTH`;
2. add every group dimension header;
3. add `count * ENTRY_BLOCK_LENGTH` for each flat group;
4. validate each varData byte length using the resolved length encoding;
5. add each varData prefix and byte length;
6. add the resolved message-header size only in the header-inclusive method;
7. return `EncodedLengthOverflow` on failed checked arithmetic.

The compatibility methods stay source-compatible. Documentation changes make
the checked methods the recommended path for exact buffer allocation.

### 2.3 Strategy C: structurally dynamic tail

A message requires the staged builder when any top-level group entry contains
a nested group or entry-level varData:

```rust
!group.groups.is_empty() || !group.var_data.is_empty()
```

Generate `{Message}EncodedLength::new()` and concrete consuming stages only for
this strategy.

#### Uniform shape

When all entries in a group have the same dynamic shape, the declared count
and one shape chain are sufficient:

```rust
let len = L3BookVarDataEncodedLength::new()
    .bids(2)
    .orders(2)
    .order_id(5)?
    .asks(0)
    .symbol(7)?
    .encoded_length_with_header();
```

Semantics:

- `bids(2)` means two bid entries with one repeated bid-entry shape.
- `orders(2)` means each bid has two order entries with one repeated order-entry shape.
- `order_id(5)` means each order has a five-byte `orderId`.
- No `add()` or `add_n()` call is needed.
- The builder adds `2 * bid_block_length`.
- The nested dimension header is added twice, once per bid.
- The builder adds `2 * 2 * order_block_length`.
- The orderId prefix and payload are added four times.

For a nested group whose entries are fixed width, the nested group method
finishes that part of the entry and is fallible because it performs checked
arithmetic:

```rust
let len = L3BookEncodedLength::new()
    .bids(2)
    .orders(3)?
    .asks(0)
    .symbol(7)?
    .encoded_length_with_header();
```

For an entry with multiple dynamic tails, the generated stages remain in SBE
wire order:

```rust
let len = NestedGroupEncodedLength::new()
    .bids(2)
    .orders(3)?
    .venue(4)?
    .asks(0)
    .comment(7)?
    .encoded_length_with_header();
```

#### Ragged known-size shape

Rust cannot overload `bids(count)` and `bids(count, closure)` by argument
count. Keep the concise name for the common uniform path and give the
different-shape path an explicit suffix:

```rust
let len = L3BookVarDataEncodedLength::new()
    .bids_ragged(2, |bids| {
        bids.orders(2).order_id(5)?;
        bids.orders(1).order_id(3)?;
        Ok(())
    })?
    .asks(0)
    .symbol(7)?
    .encoded_length_with_header();
```

Each completed entry-tail chain automatically registers one parent entry.
The closure contains one chain per different entry shape, but no `add()` call.

An outer ragged group can use an inner uniform group:

```rust
.bids_ragged(2, |bids| {
    bids.orders(2).order_id(5)?;
    bids.orders(1).order_id(3)?;
    Ok(())
})?
```

An outer uniform group can contain a ragged nested group whose complete ragged
shape is repeated for every uniform outer entry:

```rust
.bids(2)
.orders_ragged(2, |orders| {
    orders.order_id(5)?;
    orders.order_id(3)?;
    Ok(())
})?
```

Both levels can be ragged:

```rust
.bids_ragged(2, |bids| {
    bids.orders_ragged(2, |orders| {
        orders.order_id(5)?;
        orders.order_id(3)?;
        Ok(())
    })?;
    bids.orders_ragged(1, |orders| {
        orders.order_id(9)?;
        Ok(())
    })?;
    Ok(())
})?
```

Known ragged methods validate that the number of completed entry chains equals
the declared count. Starting or completing an extra chain returns
`GroupFull`; returning from the closure too early returns
`GroupCountMismatch`.

#### Unknown-size shape

Unknown-size groups retain a closure because the number of completed entry
shapes is the value being discovered:

```rust
.bids_unknown_size(|bids| {
    bids.orders(2).order_id(5)?;
    bids.orders(1).order_id(3)?;
    Ok(())
})?
```

No `add()` call is required for structurally dynamic entries. Completion of
each entry tail increments the count. The final count is checked against the
group dimension's `numInGroup` primitive and returns `GroupCountOverflow` when
it does not fit.

A flat unknown-size group has no dynamic entry tail from which to infer entry
completion. Its closure exposes one explicit bulk method:

```rust
.rate_limits_unknown_size(|entries| entries.entries(6))?
```

Use `entries(n)`, not `add_n(n)`, because the operation records a count of
fixed-width entries and does not describe individual entry construction.

#### Empty dynamic groups

The normal case must support the desired concise zero-count chain:

```rust
let len = L3BookVarDataEncodedLength::new()
    .bids(0)
    .asks(0)
    .symbol(0)?
    .encoded_length_with_header();
```

A runtime value argument cannot change a Rust method's return type. Therefore
`bids(0)` returns the same pending uniform stage as `bids(2)`. That pending
stage forwards later owner-tail methods only when its effective entry
multiplier is zero.

Every pending uniform stage also exposes a canonical explicit escape:

```rust
let after_bids = L3BookVarDataEncodedLength::new()
    .bids(0)
    .finish_empty()?;
```

The generated method is `pub const fn finish_empty(self) -> Result<NextStage,
sbe_rt::EncodeError>`.

`finish_empty()`:

- succeeds only when the current group's effective entry count is zero;
- returns the next owner stage;
- returns `GroupCountMismatch { declared, actual: 0 }` for a non-zero group;
- remains available when a schema field name collides with a forwarded method.

The generator emits zero-count forwarding sugar only when the pending group's
own next-tail method names do not collide with the continuation's method
names. A collision uses `finish_empty()` and then the next method explicitly.
The generator must never reject an otherwise valid SBE schema because of this
ergonomic forwarding feature.

#### Terminal methods

After the final required tail has successfully flushed checked arithmetic, the
complete stage remains infallible:

```rust
pub const fn encoded_length(&self) -> usize;
pub const fn encoded_length_with_header(&self) -> usize;
```

If a caller reaches the end while still holding a zero-count pending stage,
that pending stage exposes fallible terminal methods:

```rust
pub const fn encoded_length(self) -> Result<usize, sbe_rt::EncodeError>;
pub const fn encoded_length_with_header(self) -> Result<usize, sbe_rt::EncodeError>;
```

This covers a complex message whose final field is a zero-entry dynamic group
and has no later varData call on which to surface checked errors.

---

## 3. Internal generated representation

### 3.1 Length strategy seam

Move encoded-length generation out of the already large
`sbe/src/codegen/mod.rs` into:

```text
sbe/src/codegen/encoded_length.rs
```

The new module owns:

- schema-shape classification;
- direct checked-helper generation;
- complex builder-stage generation;
- recursive uniform-stage generation;
- ragged borrowed-stage generation;
- zero-count forwarding and collision filtering;
- generated type naming for length stages.

Its external seam inside the generator is:

```rust
pub(super) enum LengthStrategy {
    Fixed,
    Direct,
    Staged,
}

pub(super) struct GeneratedEncodedLength {
    pub(super) encoder_impl: proc_macro2::TokenStream,
    pub(super) standalone: proc_macro2::TokenStream,
}

pub(super) fn strategy(message: &MessageStructure) -> LengthStrategy;

pub(super) fn generate_support(
    messages: &[MessageStructure],
) -> proc_macro2::TokenStream;

pub(super) fn generate(
    message: &MessageStructure,
    block_length: usize,
    header_size: usize,
    elements: &SchemaElements,
    multi_message: bool,
) -> GeneratedEncodedLength;
```

`encoder_impl` is inserted into the initial encoder's `impl` block. It
contains direct helper methods only. `standalone` is appended after encoder
stage generation and contains complex builder types only.

`generate_support()` emits one crate-private accumulator when at least one message
uses `LengthStrategy::Staged`; otherwise it returns an empty token stream. It
runs after `MessageStructure` values are parsed and before per-message codec
generation. The support type is module-local rather than part of `sbe_rt`, so
`GenerationConfig::external_sbe_rt_path` remains compatible.

### 3.2 Checked accumulator

Generate this crate-private helper once per generated schema module that contains a
staged message:

```rust
#[doc(hidden)]
pub(crate) struct EncodedLengthAccumulator {
    len: usize,
    multiplier: usize,
    error: Option<sbe_rt::EncodeError>,
}

impl EncodedLengthAccumulator {
    pub(crate) const fn new(block_length: usize) -> Self {
        Self {
            len: block_length,
            multiplier: 1,
            error: None,
        }
    }

    pub(crate) const fn multiplier(&self) -> usize {
        self.multiplier
    }

    pub(crate) const fn add_scaled(&mut self, unit_len: usize, repetitions: usize) {
        if self.error.is_some() {
            return;
        }
        let contribution = unit_len.checked_mul(repetitions);
        self.len = match contribution {
            Some(contribution) => match self.len.checked_add(contribution) {
                Some(len) => len,
                None => {
                    self.error = Some(
                        sbe_rt::EncodeError::EncodedLengthOverflow,
                    );
                    self.len
                }
            },
            None => {
                self.error = Some(
                    sbe_rt::EncodeError::EncodedLengthOverflow,
                );
                self.len
            }
        };
    }

    pub(crate) const fn enter_group(
        &mut self,
        count: usize,
        dimension_length: usize,
        entry_block_length: usize,
    ) -> usize {
        let parent_multiplier = self.multiplier;
        self.add_scaled(dimension_length, parent_multiplier);
        self.multiplier = match parent_multiplier.checked_mul(count) {
            Some(multiplier) => multiplier,
            None => {
                self.error = Some(
                    sbe_rt::EncodeError::EncodedLengthOverflow,
                );
                0
            }
        };
        self.add_scaled(entry_block_length, self.multiplier);
        parent_multiplier
    }

    pub(crate) const fn leave_group(&mut self, parent_multiplier: usize) {
        self.multiplier = parent_multiplier;
    }

    pub(crate) const fn fail(&mut self, error: sbe_rt::EncodeError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    pub(crate) const fn check(&self) -> Result<(), sbe_rt::EncodeError> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub(crate) const fn finish(
        self,
        header_length: usize,
    ) -> Result<(usize, usize), sbe_rt::EncodeError> {
        if let Err(error) = self.check() {
            return Err(error);
        }
        match self.len.checked_add(header_length) {
            Some(full) => Ok((self.len, full)),
            None => Err(sbe_rt::EncodeError::EncodedLengthOverflow),
        }
    }
}
```

The implementation may factor private operations differently, but it must
preserve these behaviours and tests. The helper is stack-only and has no
collection, trait object, boxed closure, or heap-backed error.

### 3.3 Weighted uniform accumulation

Uniform shapes do not build a shape tree. They add contributions directly
using the product of ancestor counts:

```text
message multiplier = 1
bids(2):      add 1 × bids dimension
              add 2 × bid block
orders(3):    add 2 × orders dimension
              add 6 × order block
order_id(5):  add 6 × (orderId prefix + 5)
```

Each generated pending group stage stores:

```rust
#[doc(hidden)]
pub struct L3BookBidsUniformEncodedLength {
    state: EncodedLengthAccumulator,
    parent_multiplier: usize,
    declared_count: u32,
}
```

Nested pending stages store the accumulator plus the multiplier to restore
when the nested group completes. All types are concrete and consuming.
Generated callers never name them directly.

### 3.4 Ragged accumulation

A ragged group adds its dimension header once per effective parent entry.
Each completed ragged entry chain adds one fixed block and its own dynamic
tail, scaled only by uniform ancestor multipliers.

The borrowed ragged accumulator has:

```rust
#[doc(hidden)]
pub struct L3BookBidsRaggedEncodedLength {
    state: EncodedLengthAccumulator,
    declared_count: Option<usize>,
    maximum_count: usize,
    written: usize,
    parent_multiplier: usize,
}
```

Its first entry-tail method starts an entry. Its terminal entry-tail method
increments `written`. Intermediate entry stages borrow the ragged accumulator,
so a complete chain returns control to the closure without allocation.

For an entry with `orders` followed by `venue`, generation produces the
logical transitions:

```text
BidsRaggedEncodedLength::orders(count)
    -> BidsRaggedEntryOrdersUniformEncodedLength

BidsRaggedEntryOrdersUniformEncodedLength::order_id(length)
    -> BidsRaggedEntryAfterOrdersEncodedLength

BidsRaggedEntryAfterOrdersEncodedLength::venue(length)
    -> sbe_rt::GroupResult
```

The actual generated names may be `#[doc(hidden)]`, but their naming function
must be deterministic and golden-tested.

### 3.5 Generated test-matrix architecture

Do not generate test modules into customer codec output. Generate tests only
inside the ergo-sbe integration-test harness:

```text
schema XML
    → ergo-sbe generated codec source
    → test-only matrix renderer
    → one temporary Cargo crate containing many named #[test] functions
```

Add this test-only interface under `sbe/tests/common/encoded_length_matrix.rs`:

```rust
#[derive(Clone, Debug)]
pub struct GeneratedRustTest {
    pub name: String,
    pub body: String,
}

pub fn compile_and_run_generated_tests(
    crate_name: &str,
    generated_source: &str,
    shared_support: &str,
    tests: &[GeneratedRustTest],
) -> Result<(), Box<dyn std::error::Error>>;
```

The helper writes the generated codec once, writes shared logical-case and
assertion helpers once, emits each `GeneratedRustTest` as a separately named
`#[test] fn`, and runs:

```sh
cargo test -- --test-threads=1
```

This gives precise failing test names without recompiling the same schema for
every combination.

The logical dynamic cases use a schema-independent shape description:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LengthGroupMode {
    UniformKnown,
    RaggedKnown,
    UnknownSize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncoderGroupMode {
    Known,
    UnknownSize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedWriteMode {
    FixedStruct,
    RawFixedFields,
}

#[derive(Clone, Debug)]
pub struct NestedPayloadCase {
    pub name: &'static str,
    pub bids: &'static [&'static [&'static [u8]]],
    pub asks: &'static [&'static [&'static [u8]]],
    pub symbol: &'static [u8],
}
```

For L3-like schemas:

- the outer slice is the list of bid or ask entries;
- each inner slice is the list of nested orders for one outer entry;
- each byte slice is one order's varData payload.

The matrix renderer emits:

- one length function for every applicable
  `LengthGroupMode × LengthGroupMode` pair;
- one encoder function for every
  `EncoderGroupMode × EncoderGroupMode × FixedWriteMode` combination;
- one decoder verifier;
- one domain-object constructor and verifier;
- named tests crossing those functions with logical payload cases.

Uniform modes are invoked only for shapes they can represent:

- outer uniform requires every outer entry to have the same complete nested
  shape;
- nested uniform requires every nested entry in that repeated shape to have
  the same dynamic lengths;
- ragged and unknown-size modes accept every logical shape.

The renderer must assert that every mode pair receives at least one applicable
case, so filtering cannot accidentally erase coverage.

Use at least these logical cases:

```rust
const EMPTY: NestedPayloadCase = NestedPayloadCase {
    name: "empty",
    bids: &[],
    asks: &[],
    symbol: b"",
};

const UNIFORM_SINGLE: NestedPayloadCase = NestedPayloadCase {
    name: "uniform_single",
    bids: &[&[b"ORD-1"]],
    asks: &[&[b"ASK-1"]],
    symbol: b"BTCUSDT",
};

const UNIFORM_MANY: NestedPayloadCase = NestedPayloadCase {
    name: "uniform_many",
    bids: &[&[b"AAAAA", b"BBBBB"], &[b"CCCCC", b"DDDDD"]],
    asks: &[&[b"EEEEE", b"FFFFF"], &[b"GGGGG", b"HHHHH"]],
    symbol: b"ETHUSDT",
};

const RAGGED_COUNTS: NestedPayloadCase = NestedPayloadCase {
    name: "ragged_counts",
    bids: &[&[], &[b"A"], &[b"BB", b"CCC", b"DDDD"]],
    asks: &[&[b"E", b"FF"], &[]],
    symbol: b"SOLUSDT",
};

const RAGGED_LENGTHS: NestedPayloadCase = NestedPayloadCase {
    name: "ragged_lengths",
    bids: &[&[b"A", b"BBBBB"], &[b"CCC"]],
    asks: &[&[b"D", b"EE", b"FFF"]],
    symbol: b"X",
};

const BINARY_AND_UTF8: NestedPayloadCase = NestedPayloadCase {
    name: "binary_and_utf8",
    bids: &[&[b"\0A\0", "東京".as_bytes()]],
    asks: &[&["é".as_bytes()]],
    symbol: b"BIN",
};
```

Use a binary-varData fixture for `BINARY_AND_UTF8` when a text field enforces
ASCII. UTF-8 lengths are always measured with `.len()` on encoded bytes, not
character counts.

Each generated matrix test follows this invariant:

```rust
let logical = case_to_domain(case)?;
let computed = length_with_selected_modes(case)?;
let bytes = encode_with_selected_modes(case, computed)?;
let decoded = MessageDecoder::try_from(bytes.as_slice())?;
verify_decoded_case(decoded, case)?;

assert_eq!(computed, bytes.len());
assert_eq!(computed, logical.encoded_length_with_header()?);

let mut domain_bytes = vec![0_u8; logical.encoded_length_with_header()?];
let domain_written = logical.encode(&mut domain_bytes)?;
assert_eq!(computed, domain_written);
assert_eq!(bytes, domain_bytes);
```

If known-size and unknown-size encoder modes legitimately differ only in how
the count is supplied, require byte-identical output.

### 3.6 Generated-code cleanliness and usability gates

The large mechanical matrix proves behaviour; it does not prove that the
interface is pleasant. Add independent gates:

1. **Curated compile snippets.** Handwrite short examples for fixed, direct,
   uniform, ragged, unknown-size, zero, and depth-three cases. These snippets
   are the readability contract and must not be rendered from metadata.
2. **AST interface audit.** Parse generated source with `syn` and index every
   inherent method by `(self type, method name)`. Assert there are no duplicate
   methods and no length-builder `add` or `add_n` methods.
3. **Strategy audit.** Assert fixed/direct messages have no public
   `EncodedLength` structs and staged messages do.
4. **Visibility audit.** Assert plumbing types such as uniform pending stages,
   ragged borrowed stages, and the accumulator are `#[doc(hidden)]`; only
   entry-point and complete-stage types are normal public documentation.
5. **Ordered-stage audit.** Use compile-fail snippets to prove out-of-order and
   incomplete chains do not compile.
6. **API signature golden.** Extract only public encoded-length type and method
   signatures into `sbe/tests/golden/encoded_length_api.txt`. This makes an
   ergonomic interface change reviewable without reading the full generated
   Car source.
7. **Formatting audit.** Run generated source through the existing
   `syn`/`prettyplease` path and assert a second format pass is byte-identical.
8. **Production-schema audit.** Generate Binance, CME, iLink, L3, conformance,
   `u8` dimensions, big-endian, and custom-header sources and parse them with
   `syn`.
9. **Allocation audit.** Measure length calculation separately from fixture
   construction and require zero allocations.
10. **Documentation compile audit.** Keep README examples mirrored by curated
    compile tests so copy-paste examples cannot drift.

### 3.7 Const-evaluation policy

The workspace minimum Rust version is `1.95`. A local compiler probe confirms:

- `checked_add`, `checked_mul`, pattern matching, mutable local state, and
  constructing `Result` are usable in `const fn`;
- the `?` operator is not usable in `const fn` because const `Try` and
  `FromResidual` are not stable;
- `Result::unwrap()` is not const.

Mark methods `const fn` only when their entire implementation can use explicit
`match`/early-return control flow and does not invoke a closure.

#### Must be const-capable

- Existing fixed-message `BLOCK_LENGTH`, `HEADER_LENGTH`, and
  `ENCODED_LENGTH` constants.
- Existing compatibility `compute_encoded_length*` direct helpers.
- New checked `try_compute_encoded_length*` direct helpers.
- `EncodedLengthAccumulator::new`, `add_scaled`, `enter_group`, `leave_group`,
  `fail`, `check`, `multiplier`, and `finish`.
- `{Message}EncodedLength::new()`.
- Uniform known-size group transitions that do not accept a closure.
- Uniform entry varData transitions.
- Flat nested-group completion methods.
- Zero forwarding and `finish_empty()`.
- Complete-stage `encoded_length()` and `encoded_length_with_header()`.
- Fallible zero-pending terminal methods.

#### Must remain runtime-only

- `group_ragged(count, closure)`.
- `group_unknown_size(closure)`.
- Borrowed ragged entry operations reached only through those closures.
- Domain-object length methods that iterate `Vec` values.
- Any method that formats errors, accesses a runtime buffer, invokes user
  code, or depends on a non-const standard-library operation.

Do not expand this task into unrelated encoder/decoder constification. Existing
const decoder metadata/accessors remain unchanged.

Because `?` is unavailable in const contexts, add module-scope compile proofs
using explicit matches:

```rust
const DIRECT_LENGTH: Result<usize, sbe_rt::EncodeError> =
    FlatGroupEncoder::try_compute_encoded_length_with_header(2, 1, 7);

const fn uniform_length() -> Result<usize, sbe_rt::EncodeError> {
    let after_bids = match L3BookVarDataEncodedLength::new()
        .bids(2)
        .orders(2)
        .order_id(5)
    {
        Ok(stage) => stage,
        Err(error) => return Err(error),
    };

    let complete = match after_bids
        .asks(0)
        .symbol(7)
    {
        Ok(stage) => stage,
        Err(error) => return Err(error),
    };

    Ok(complete.encoded_length_with_header())
}

const UNIFORM_LENGTH: Result<usize, sbe_rt::EncodeError> = uniform_length();
```

At runtime, assert both constants equal the corresponding normal call-chain
results.

---

## 4. File map

### Create

- `sbe/src/codegen/encoded_length.rs` — classification and all encoded-length code generation.
- `sbe/tests/encoded_length_api_test.rs` — generated-source and runtime interface matrix.
- `sbe/tests/common/encoded_length_matrix.rs` — deterministic test-only renderer and one-crate many-test runner.
- `sbe/tests/fixtures/schemas/encoded-length-method-collision.xml` — focused zero-forwarding collision fixture.
- `sbe/tests/golden/encoded_length_api.txt` — compact public encoded-length signature snapshot.

### Modify

- `sbe/src/codegen/mod.rs` — declare the new module, expose required helper functions as `pub(super)`, call the new generation seam, and delete the old inline builder/direct-helper implementation.
- `sbe/tests/common/mod.rs` — export `encoded_length_matrix` and add fixture path helpers.
- `sbe/tests/conformance_test.rs` — switch flat shapes to direct helpers and add uniform/ragged exact-length proofs for existing conformance messages.
- `sbe/tests/l3_orderbook_test.rs` — replace the weak positive-length test and old `add()` builder syntax.
- `sbe/tests/baseline_test.rs` — migrate Car length calculations.
- `sbe/tests/domain_objects_test.rs` — migrate builder syntax and retain domain/encoder/decoder equality.
- `sbe/tests/ordered_decoder_stages_test.rs` — migrate exact-buffer setup only.
- `sbe/tests/allocation_count_test.rs` — prove complex length building allocates zero times.
- `sbe/tests/stability_test.rs` — regenerate the golden output through the existing ignored test.
- `sbe/tests/golden/car_example.rs` — regenerated output; never hand-edit.
- `samples/l3-book/src/main.rs` — show concise uniform or ragged length syntax.
- `samples/l3-book/tests/l3_tests.rs` — cover uniform, ragged, zero, and nested varData length syntax.
- `sbe/README.md` — explain the complexity-based interface selection and all three strategies.
- `samples/l3-book/README.md` — update the exact-length example.
- `docs/design/2026-07-23-exact-length-builder-and-conformance-tests.md` — add a short supersession note pointing to this plan's interface rules; retain the earlier correctness rationale.

### Must not modify

- Generated encoder method names or signatures.
- Generated decoder method names or signatures.
- SBE wire layouts, dimension encodings, varData encodings, or header layouts.
- `simple-binary-encoding/`.
- Unrelated cluster, persistence, Aeron, or sample logic.

---

## 5. Implementation tasks

### Task 1: Lock the length-strategy classifier with repository-schema tests — audited complete

**Files:**

- Create: `sbe/src/codegen/encoded_length.rs`
- Modify: `sbe/src/codegen/mod.rs`
- Test: unit tests in `sbe/src/codegen/encoded_length.rs`

**Interfaces:**

- Produces: `LengthStrategy`, `GeneratedEncodedLength`, `strategy()`, and `generate()`.
- Consumes: `MessageStructure`, `MessageGroup`, `SchemaElements`, and existing dimension/varData resolution helpers.

- [ ] **Step 1: Add failing classification tests using existing schemas**

Add test helpers that parse a named message from an existing fixture:

```rust
#[cfg(test)]
mod tests {
    use super::{LengthStrategy, strategy};
    use crate::structured_ir::{
        parse_message_structure,
        partition_tokens,
    };
    use std::path::{Path, PathBuf};

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("schemas")
            .join(name)
    }

    fn strategy_for(
        path: &Path,
        message_name: &str,
    ) -> Result<LengthStrategy, Box<dyn std::error::Error>> {
        let ir = crate::parse_file(path)?;
        let elements = partition_tokens(&ir.tokens);
        let message_tokens = elements
            .messages
            .iter()
            .find(|tokens| tokens[0].name == message_name)
            .ok_or_else(|| format!("missing message {message_name}"))?;
        let message = parse_message_structure(message_tokens, &elements);
        Ok(strategy(&message))
    }

    #[test]
    fn classifies_repository_message_shapes()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            strategy_for(&fixture("basic-schema.xml"), "TestMessage50001")?,
            LengthStrategy::Fixed,
        );
        assert_eq!(
            strategy_for(
                &fixture("basic-variable-length-schema.xml"),
                "TestMessage1",
            )?,
            LengthStrategy::Direct,
        );
        assert_eq!(
            strategy_for(&fixture("basic-group-schema.xml"), "TestMessage1")?,
            LengthStrategy::Direct,
        );
        assert_eq!(
            strategy_for(&fixture("group-with-data-schema.xml"), "TestMessage1")?,
            LengthStrategy::Staged,
        );
        assert_eq!(
            strategy_for(&fixture("nested-group-schema.xml"), "Top")?,
            LengthStrategy::Staged,
        );
        assert_eq!(
            strategy_for(&fixture("l3-orderbook-schema.xml"), "L3Book")?,
            LengthStrategy::Staged,
        );
        Ok(())
    }
}
```

- [ ] **Step 2: Run the focused unit test and confirm the missing symbols fail**

Run:

```sh
cargo test -p ergo-sbe codegen::encoded_length::tests::classifies_repository_message_shapes -- --nocapture
```

Expected result: compilation fails because `LengthStrategy` and `strategy` do
not exist.

- [ ] **Step 3: Implement the classifier**

Use this rule exactly:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LengthStrategy {
    Fixed,
    Direct,
    Staged,
}

pub(super) fn strategy(message: &MessageStructure) -> LengthStrategy {
    if message.groups.is_empty() && message.var_data.is_empty() {
        return LengthStrategy::Fixed;
    }

    let has_dynamic_entry = message
        .groups
        .iter()
        .any(|group| !group.groups.is_empty() || !group.var_data.is_empty());

    if has_dynamic_entry {
        LengthStrategy::Staged
    } else {
        LengthStrategy::Direct
    }
}
```

Add the module declaration in `sbe/src/codegen/mod.rs`:

```rust
mod encoded_length;
```

- [ ] **Step 4: Run the focused test and all codegen unit tests**

Run:

```sh
cargo test -p ergo-sbe codegen::encoded_length -- --nocapture
cargo test -p ergo-sbe --lib codegen -- --nocapture
```

Expected result: both commands pass.

- [ ] **Step 5: Commit**

```sh
git add sbe/src/codegen/mod.rs sbe/src/codegen/encoded_length.rs
git commit -m "refactor: classify encoded length strategies"
```

### Task 2: Make direct helpers the only generated interface for simple tails — audited partial

**Files:**

- Modify: `sbe/src/codegen/encoded_length.rs`
- Modify: `sbe/src/codegen/mod.rs`
- Create: `sbe/tests/encoded_length_api_test.rs`
- Test: `sbe/tests/encoded_length_api_test.rs`

**Interfaces:**

- Produces: compatibility `compute_encoded_length*` helpers, checked
  `try_compute_encoded_length*` helpers, and no `{Message}EncodedLength` type
  for `LengthStrategy::Direct`.
- Consumes: resolved group dimension sizes/count primitives and resolved
  varData prefix sizes/maxima.

- [ ] **Step 1: Add failing source-surface tests**

Generate the fixed-only, varData-only, one-flat-group, and `FlatGroup`
conformance schemas. Assert:

```rust
assert!(!fixed_source.contains("TestMessage50001EncodedLength"));
assert!(!vardata_source.contains("TestMessage1EncodedLength"));
assert!(!flat_group_source.contains("TestMessage1EncodedLength"));
assert!(!conformance_source.contains("FlatGroupEncodedLength"));

assert!(
    vardata_source.contains("pub const fn try_compute_encoded_length("),
);
assert!(
    vardata_source.contains(
        "pub const fn try_compute_encoded_length_with_header(",
    ),
);
assert!(
    flat_group_source.contains(
        "pub const fn compute_encoded_length(",
    ),
);
assert!(
    flat_group_source.contains(
        "pub const fn compute_encoded_length_with_message_header(",
    ),
);
```

- [ ] **Step 2: Run the source-surface test and confirm it fails**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test direct_shapes_generate_helpers_without_builders -- --nocapture
```

Expected result: failure because simple dynamic messages still generate
`{Message}EncodedLength` and checked direct helpers are absent.

- [ ] **Step 3: Move direct-helper generation behind `LengthStrategy::Direct`**

`encoded_length::generate()` returns:

```rust
pub(super) struct GeneratedEncodedLength {
    pub(super) encoder_impl: proc_macro2::TokenStream,
    pub(super) standalone: proc_macro2::TokenStream,
}
```

For `Fixed`, both token streams are empty. For `Direct`, `encoder_impl`
contains direct methods and `standalone` is empty. For `Staged`,
`encoder_impl` is empty and `standalone` contains the staged builder.

Delete the old `has_nested_dynamic_tail()` and the inline direct-helper block
from `sbe/src/codegen/mod.rs` only after the new path passes.

- [ ] **Step 4: Add checked direct arithmetic and validation**

Generate group parameter types from `get_dim_num_layout()` and varData checks
from `get_vardata_info()` plus `MessageVarData::max_length`.

The generated checked body must use this arithmetic pattern:

```rust
let mut len = Self::BLOCK_LENGTH;
let entries_len = match 12_usize.checked_mul(bids_count as usize) {
    Some(value) => value,
    None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
};
len = match len.checked_add(4) {
    Some(value) => value,
    None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
};
len = match len.checked_add(entries_len) {
    Some(value) => value,
    None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
};
```

The generated `12` and `4` literals above come from the resolved group block
length and dimension composite. Do not add new constants or methods to the
group encoder interface solely for length generation.

For each varData parameter:

```rust
if description_len > 65_534 {
    return Err(sbe_rt::EncodeError::VarDataTooLong {
        field: "description",
        max_length: 65_534,
        actual: description_len,
    });
}
len = match len.checked_add(4) {
    Some(value) => value,
    None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
};
len = match len.checked_add(description_len) {
    Some(value) => value,
    None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
};
```

Use resolved literals; `65_534` and `4` above are the conformance fixture's
values, not universal constants. Do not generate `?`, `and_then`, or a closure
inside these checked helpers; they must remain valid `const fn` on Rust 1.95.

- [ ] **Step 5: Add runtime direct-helper exactness tests**

For `FlatGroup`, compute an exact header-inclusive length with:

```rust
let len = FlatGroupEncoder::try_compute_encoded_length_with_header(2, 1, 17)?;
```

Allocate exactly `len`, encode two bids, one ask, and a 17-byte description,
then assert:

```rust
assert_eq!(len, complete.encoded_length_with_header());
assert_eq!(len, complete.as_bytes().len());
```

Add a second test for `CompactMsg` using count `u8::MAX` and memo length
`u8::MAX as usize`, then verify `memo_len = u8::MAX as usize + 1` returns
`VarDataTooLong`.

Add a module-scope constant:

```rust
const FLAT_CONST_LENGTH: Result<usize, sbe_rt::EncodeError> =
    FlatGroupEncoder::try_compute_encoded_length_with_header(2, 1, 17);
```

Assert it equals the runtime invocation.

- [ ] **Step 6: Run direct-helper tests**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test direct_ -- --nocapture
cargo test -p ergo-sbe --test conformance_test conformance_length_builder_invariants -- --nocapture
```

Expected result: direct source and runtime tests pass. The conformance test may
still fail until its old flat-builder call is migrated in Task 8; record that
expected migration failure without weakening the new focused tests.

- [ ] **Step 7: Commit**

```sh
git add sbe/src/codegen/mod.rs sbe/src/codegen/encoded_length.rs sbe/tests/encoded_length_api_test.rs
git commit -m "feat: use direct lengths for simple SBE tails"
```

### Task 3: Add the checked stack-only accumulator — audited partial

**Files:**

- Modify: `sbe/src/codegen/encoded_length.rs`
- Test: `sbe/tests/encoded_length_api_test.rs`

**Interfaces:**

- Produces: one private generated `EncodedLengthAccumulator` for schema modules
  containing staged messages.
- Consumes: existing `EncodeError::EncodedLengthOverflow`.

- [ ] **Step 1: Add failing accumulator behaviour tests**

Cover:

```rust
#[test]
fn length_accumulator_scales_nested_contributions()
-> Result<(), Box<dyn std::error::Error>> {
    let mut state = EncodedLengthAccumulator::new(16);
    let message_multiplier = state.enter_group(2, 4, 12);
    let bid_multiplier = state.enter_group(3, 4, 8);
    state.add_scaled(9, state.multiplier());
    state.leave_group(bid_multiplier);
    state.leave_group(message_multiplier);
    let (body, full) = state.finish(8)?;
    assert_eq!(body, 16 + 4 + 2 * 12 + 2 * 4 + 6 * 8 + 6 * 9);
    assert_eq!(full, body + 8);
    Ok(())
}

#[test]
fn length_accumulator_preserves_first_error()
-> Result<(), Box<dyn std::error::Error>> {
    let mut state = EncodedLengthAccumulator::new(usize::MAX);
    state.add_scaled(1, 1);
    state.add_scaled(1, 1);
    assert_eq!(
        state.finish(8),
        Err(EncodeError::EncodedLengthOverflow),
    );
    Ok(())
}
```

Expose a `pub(crate) const fn multiplier(&self) -> usize` for generated code
and tests.

- [ ] **Step 2: Run the focused test and confirm it fails**

Run:

```sh
cargo test -p ergo-sbe length_accumulator -- --nocapture
```

Expected result: generated-source tests fail because
`EncodedLengthAccumulator` is absent.

- [ ] **Step 3: Emit the accumulator**

Implement the operations specified in section 3.2. Ensure `fail()` preserves
the first error and every later arithmetic call becomes a no-op after failure.

- [ ] **Step 4: Add a generated-source compile proof**

Generate a complex schema and assert the source contains exactly one
`struct EncodedLengthAccumulator`, parses through `syn`, and compiles in the
existing temporary-crate harness. Generate a fixed-only and direct-only schema
and assert both omit `EncodedLengthAccumulator`.

- [ ] **Step 5: Run runtime and generated-source tests**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test length_accumulator_ -- --nocapture
cargo test -p ergo-sbe --test encoded_length_api_test accumulator_is_generated_once -- --nocapture
cargo test -p ergo-sbe --test encoded_length_api_test simple_schemas_omit_accumulator -- --nocapture
```

Expected result: both pass.

- [ ] **Step 6: Commit**

```sh
git add sbe/src/codegen/encoded_length.rs sbe/tests/encoded_length_api_test.rs
git commit -m "feat: add checked encoded length accumulator"
```

### Task 4: Generate the concise uniform nested-group path — audited partial

**Files:**

- Modify: `sbe/src/codegen/encoded_length.rs`
- Test: `sbe/tests/encoded_length_api_test.rs`
- Test: `sbe/tests/conformance_test.rs`
- Test: `sbe/tests/l3_orderbook_test.rs`

**Interfaces:**

- Produces: `group(count)` uniform methods, recursive uniform pending stages,
  flat nested-group completion, entry varData transitions, and complete
  message length stages.
- Consumes: `EncodedLengthAccumulator`.

- [ ] **Step 1: Add failing compile-and-run tests for the desired syntax**

Compile these exact chains against `samples/l3-book/schemas/l3-book.xml`:

```rust
let fixed_nested = L3BookEncodedLength::new()
    .bids(2)
    .orders(2)?
    .asks(0)
    .symbol(7)?
    .encoded_length_with_header();

let nested_vardata = L3BookVarDataEncodedLength::new()
    .bids(2)
    .orders(2)
    .order_id(5)?
    .asks(0)
    .symbol(7)?
    .encoded_length_with_header();

assert!(fixed_nested > 0);
assert!(nested_vardata > fixed_nested);
```

Compile a three-level chain against `nested-group-schema.xml`:

```rust
let len = TopEncodedLength::new()
    .x(2)
    .y(3)
    .z(4)?
    .encoded_length_with_header();
assert!(len > TopEncoder::BLOCK_LENGTH);
```

- [ ] **Step 2: Run the focused tests and confirm method-arity failures**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test uniform_nested_ -- --nocapture
```

Expected result: generated temporary crates fail because current complex group
methods require `(count, closure)`.

- [ ] **Step 3: Generate weighted uniform stages recursively**

For each dynamic group:

1. add the group dimension scaled by the current multiplier;
2. multiply the current multiplier by the declared count;
3. add the group entry block scaled by the new multiplier;
4. return a pending concrete stage for the first entry tail;
5. restore the parent multiplier when the final entry tail completes;
6. return the next owner stage.

For a flat nested leaf, generate:

```rust
pub const fn orders(
    mut self,
    count: u16,
) -> Result<L3BookEncodedLengthAfterBids, sbe_rt::EncodeError> {
    let parent_multiplier = self.state.enter_group(
        count as usize,
        4,
        16,
    );
    self.state.leave_group(parent_multiplier);
    match self.state.check() {
        Ok(()) => Ok(L3BookEncodedLengthAfterBids {
            state: self.state,
        }),
        Err(error) => Err(error),
    }
}
```

The generated `4` and `16` are resolved literals for the sample's nested
dimension and entry block. Do not add length-only constants to encoder types.
The actual continuation type depends on remaining outer-entry fields. If
`venue` follows `orders`, return an `AfterOrders` stage rather than the
message's `AfterBids` stage.

- [ ] **Step 4: Generate entry varData transitions**

For each varData field:

1. validate the byte length;
2. add `(prefix_length + byte_length) * multiplier`;
3. call `state.check()` at the fallible boundary;
4. return the next entry stage or complete the parent entry/group.

The generated operation must avoid adding `prefix + byte_length` before
checking that addition:

```rust
self.state.add_scaled(prefix_length, multiplier);
self.state.add_scaled(byte_length, multiplier);
if let Err(error) = self.state.check() {
    return Err(error);
}
```

Uniform methods must not generate `?`, iterator adapters, or closures
internally, so they remain `const fn`.

- [ ] **Step 5: Add a const-evaluation compile proof**

Add the `uniform_length()` function and `UNIFORM_LENGTH` constant from section
3.7 to a generated temporary crate. Assert the const result equals the normal
runtime chain.

- [ ] **Step 6: Add exact-wire assertions**

For uniform L3 shapes:

1. compute the length using the new uniform builder;
2. allocate exactly that many bytes;
3. encode identical group counts and varData lengths with the unchanged
   encoder interface;
4. decode all nested entries;
5. assert builder, encoder, `as_bytes()`, and decoder header-inclusive lengths
   agree.

Cover:

- empty;
- one outer/one inner;
- two outer/two inner;
- three nesting levels;
- one and multiple entry varData fields;
- parent varData after a nested group;
- message varData.

- [ ] **Step 7: Run uniform tests**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test uniform_nested_ -- --nocapture
cargo test -p ergo-sbe --test encoded_length_api_test uniform_const_ -- --nocapture
cargo test -p ergo-sbe --test l3_orderbook_test -- --nocapture
cargo test -p ergo-sbe --test conformance_test conformance_nested_group_roundtrip -- --nocapture
```

Expected result: all focused uniform and exact-wire tests pass.

- [ ] **Step 8: Commit**

```sh
git add sbe/src/codegen/encoded_length.rs sbe/tests/encoded_length_api_test.rs sbe/tests/conformance_test.rs sbe/tests/l3_orderbook_test.rs
git commit -m "feat: simplify uniform nested length building"
```

### Task 5: Generate known ragged stages without `add()` boilerplate — audited not done

**Files:**

- Modify: `sbe/src/codegen/encoded_length.rs`
- Test: `sbe/tests/encoded_length_api_test.rs`
- Test: `sbe/tests/conformance_test.rs`
- Test: `sbe/tests/l3_orderbook_test.rs`

**Interfaces:**

- Produces: `group_ragged(count, closure)` and borrowed staged entry-tail
  methods that automatically count completed entries.
- Consumes: uniform nested stages from Task 4.

- [ ] **Step 1: Add failing outer-ragged/inner-uniform tests**

Use:

```rust
let len = L3BookVarDataEncodedLength::new()
    .bids_ragged(2, |bids| {
        bids.orders(2).order_id(5)?;
        bids.orders(1).order_id(3)?;
        Ok(())
    })?
    .asks(0)
    .symbol(7)?
    .encoded_length_with_header();
```

Encode matching entries and assert exact length.

- [ ] **Step 2: Add failing outer-uniform/inner-ragged tests**

Use:

```rust
let len = L3BookVarDataEncodedLength::new()
    .bids(2)
    .orders_ragged(2, |orders| {
        orders.order_id(5)?;
        orders.order_id(3)?;
        Ok(())
    })?
    .asks(0)
    .symbol(0)?
    .encoded_length_with_header();
```

This means every uniform bid repeats the same two-order ragged shape.

- [ ] **Step 3: Add failing ragged-count validation tests**

Known too few:

```rust
let result = L3BookVarDataEncodedLength::new()
    .bids_ragged(2, |bids| {
        bids.orders(1).order_id(5)?;
        Ok(())
    });

assert!(matches!(
    result,
    Err(sbe_rt::EncodeError::GroupCountMismatch {
        declared: 2,
        actual: 1,
    }),
));
```

Known too many:

```rust
let result = L3BookVarDataEncodedLength::new()
    .bids_ragged(1, |bids| {
        bids.orders(1).order_id(5)?;
        bids.orders(1).order_id(5)?;
        Ok(())
    });

assert!(matches!(
    result,
    Err(sbe_rt::EncodeError::GroupFull {
        declared: 1,
        attempted: 2,
    }),
));
```

- [ ] **Step 4: Run the focused tests and confirm missing-method failures**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test ragged_ -- --nocapture
```

Expected result: generated source lacks `bids_ragged` and borrowed automatic
entry stages.

- [ ] **Step 5: Implement borrowed ragged entry stages**

Generate one entry-start method for the first dynamic tail of the entry.
That method:

1. rejects a start when `written == declared_count`;
2. adds one fixed entry block scaled by the parent multiplier;
3. enters the first dynamic tail;
4. returns a borrowed consuming stage.

The final dynamic-tail method:

1. completes checked arithmetic;
2. increments `written`;
3. releases its borrow of the ragged group builder;
4. returns `sbe_rt::GroupResult`.

The known ragged group method invokes the closure, then checks
`written == declared_count`.

- [ ] **Step 6: Run ragged and exact-wire tests**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test ragged_ -- --nocapture
cargo test -p ergo-sbe --test conformance_test conformance_error_group_count_mismatch -- --nocapture
```

Expected result: all ragged tests pass and existing mismatch semantics remain.

- [ ] **Step 7: Commit**

```sh
git add sbe/src/codegen/encoded_length.rs sbe/tests/encoded_length_api_test.rs sbe/tests/conformance_test.rs sbe/tests/l3_orderbook_test.rs
git commit -m "feat: add ragged encoded length stages"
```

### Task 6: Add unknown-size and flat-entry bulk count paths — audited not done

**Files:**

- Modify: `sbe/src/codegen/encoded_length.rs`
- Test: `sbe/tests/encoded_length_api_test.rs`
- Test: `sbe/tests/conformance_test.rs`

**Interfaces:**

- Produces: `group_unknown_size(closure)` for dynamic entries and
  `entries(n)` for fixed-width unknown-size groups.
- Consumes: ragged entry completion from Task 5.

- [ ] **Step 1: Add failing dynamic unknown-size tests**

Cover outer unknown/inner uniform:

```rust
.bids_unknown_size(|bids| {
    bids.orders(2).order_id(5)?;
    bids.orders(1).order_id(3)?;
    Ok(())
})?
```

Cover outer uniform/inner unknown:

```rust
.bids(2)
.orders_unknown_size(|orders| {
    orders.order_id(5)?;
    orders.order_id(3)?;
    Ok(())
})?
```

Cover unknown/unknown:

```rust
.bids_unknown_size(|bids| {
    bids.orders_unknown_size(|orders| {
        orders.order_id(5)?;
        orders.order_id(3)?;
        Ok(())
    })?;
    Ok(())
})?
```

- [ ] **Step 2: Add failing fixed-width unknown-size bulk test**

Against a complex message that contains a flat top-level group, use:

```rust
.rate_limits_unknown_size(|entries| entries.entries(6))?
```

Assert its result equals the known-size `.rate_limits(6)?` result.

- [ ] **Step 3: Add `u8` overflow test**

Use `group-with-data-schema.xml`, whose `numInGroup` is `u8`. Complete 256
entry shapes in an unknown-size group and assert:

```rust
assert!(matches!(
    result,
    Err(sbe_rt::EncodeError::GroupCountOverflow {
        maximum: 255,
        actual: 256,
    }),
));
```

- [ ] **Step 4: Run the focused tests and confirm failures**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test unknown_size_ -- --nocapture
```

Expected result: unknown-size dynamic builders still require explicit
`add()` calls and fixed builders expose `add_n()` instead of `entries()`.

- [ ] **Step 5: Implement unknown-size automatic counting**

Reuse ragged completed-entry accounting with `declared_count: None`. After the
closure:

```rust
if builder.written > builder.maximum_count {
    return Err(sbe_rt::EncodeError::GroupCountOverflow {
        maximum: builder.maximum_count as u32,
        actual: builder.written as u32,
    });
}
```

Generate `entries(n)` only for fixed-width unknown-size length groups. It uses
checked multiplication and validates the resulting count width.

- [ ] **Step 6: Run known/unknown cross-product tests**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test known_unknown_ -- --nocapture
cargo test -p ergo-sbe --test encoded_length_api_test unknown_size_ -- --nocapture
```

Expected result: known/known, known/unknown, unknown/known, and
unknown/unknown logical equivalents all produce the same exact length.

- [ ] **Step 7: Commit**

```sh
git add sbe/src/codegen/encoded_length.rs sbe/tests/encoded_length_api_test.rs sbe/tests/conformance_test.rs
git commit -m "feat: simplify unknown size length groups"
```

### Task 7: Handle empty dynamic groups and forwarding collisions — audited partial

**Files:**

- Modify: `sbe/src/codegen/encoded_length.rs`
- Create: `sbe/tests/fixtures/schemas/encoded-length-method-collision.xml`
- Test: `sbe/tests/encoded_length_api_test.rs`

**Interfaces:**

- Produces: zero-count forwarding sugar, `finish_empty()`, pending-stage
  fallible terminal methods, and collision-safe generation.
- Consumes: uniform pending stages from Task 4.

- [ ] **Step 1: Add failing zero-forward tests**

Cover:

```rust
let empty = L3BookVarDataEncodedLength::new()
    .bids(0)
    .asks(0)
    .symbol(0)?
    .encoded_length_with_header();
```

Cover zero nested group followed by parent varData:

```rust
let nested_empty = NestedGroupEncodedLength::new()
    .bids(2)
    .orders(0)?
    .venue(4)?
    .asks(0)
    .comment(0)?
    .encoded_length_with_header();
```

Cover a message ending in a zero dynamic group:

```rust
let zero_terminal = PureFixedNestedEncodedLength::new()
    .records(0)
    .encoded_length_with_header()?;
```

- [ ] **Step 2: Add a non-zero skipped-shape error test**

Use:

```rust
let result = L3BookVarDataEncodedLength::new()
    .bids(2)
    .asks(0)
    .symbol(0);

assert!(matches!(
    result,
    Err(sbe_rt::EncodeError::GroupCountMismatch {
        declared: 2,
        actual: 0,
    }),
));
```

- [ ] **Step 3: Add a focused method-collision fixture**

The fixture must declare a dynamic outer group whose first nested group has
the same Rust accessor name as the following message-level group. The test
asserts generated source parses, omits ambiguous forwarding on the pending
stage, and supports:

```rust
let after_outer = CollisionMsgEncodedLength::new()
    .outer(0)
    .finish_empty()?;
let complete = after_outer
    .next(0)?
    .payload(0)?;
assert!(complete.encoded_length_with_header() > 0);
```

- [ ] **Step 4: Run zero/collision tests and confirm failures**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test zero_ -- --nocapture
cargo test -p ergo-sbe --test encoded_length_api_test method_collision_ -- --nocapture
```

Expected result: pending stages lack forwarding and `finish_empty()`.

- [ ] **Step 5: Generate forwarding methods conditionally**

For each pending uniform group:

1. collect the pending entry-tail method names;
2. collect the immediate continuation's public method names;
3. generate forwarding only for names absent from the pending set;
4. on forwarding, succeed silently when effective count is zero;
5. otherwise store or return `GroupCountMismatch`;
6. preserve the first error until the next fallible boundary.

Always generate `finish_empty()`.

- [ ] **Step 6: Generate fallible pending terminals**

When a pending stage can reach message completion by skipping an empty group,
generate consuming `encoded_length()` and `encoded_length_with_header()` that
call `EncodedLengthAccumulator::finish()`.

- [ ] **Step 7: Run all zero, collision, and source-parse tests**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test zero_ -- --nocapture
cargo test -p ergo-sbe --test encoded_length_api_test method_collision_ -- --nocapture
cargo test -p ergo-sbe --test schema_edge_cases_test nested_group_types_exist -- --nocapture
```

Expected result: all pass.

- [ ] **Step 8: Commit**

```sh
git add sbe/src/codegen/encoded_length.rs sbe/tests/encoded_length_api_test.rs sbe/tests/fixtures/schemas/encoded-length-method-collision.xml
git commit -m "feat: handle empty nested length groups"
```

### Task 8: Build the comprehensive exact-length conformance matrix — audited partial foundation

**Files:**

- Modify: `sbe/tests/common/mod.rs`
- Create: `sbe/tests/common/encoded_length_matrix.rs`
- Modify: `sbe/tests/conformance_test.rs`
- Modify: `sbe/tests/l3_orderbook_test.rs`
- Modify: `sbe/tests/domain_objects_test.rs`
- Modify: `sbe/tests/allocation_count_test.rs`
- Test: the files above

**Interfaces:**

- Produces: behavioural proof that direct helpers/builders, encoder, decoder,
  DTO/domain objects, and exact buffers agree.
- Consumes: all generated length interfaces from Tasks 2–7.

- [ ] **Step 1: Add the one-crate many-test runner**

Implement `compile_and_run_generated_tests()` from section 3.5. It must:

1. validate every generated test name as a non-empty snake-case Rust
   identifier;
2. reject duplicate test names before writing the crate;
3. write generated codec source once;
4. write shared support source once;
5. parse each `body` as a `proc_macro2::TokenStream` and render each case with:

```rust
let test_item = quote::quote! {
    #[test]
    fn #test_ident() -> Result<(), Box<dyn std::error::Error>> {
        #body
        Ok(())
    }
};
```

6. run `cargo test -- --test-threads=1`;
7. include stdout and stderr in a failure;
8. remove its temporary crate after success or failure.

Add helper unit tests for duplicate-name rejection and invalid identifier
rejection.

- [ ] **Step 2: Compile one generated test crate per schema family**

Use one `compile_and_run_generated_tests()` invocation for each of:

- `conformance_schema.xml`;
- `group-with-data-schema.xml`;
- `l3-orderbook-schema.xml`;
- `nested-group-schema.xml`;
- `u8-dimension-schema.xml`.

Each invocation contains many named tests but only one copy of generated codec
source. This avoids one Cargo build and package-cache lock per combination
while preserving exact failing test names.

Use `common::generate_domain()` for every matrix that asserts domain-object
length or encoding. Use `common::generate()` only for source-surface or
compile-fail tests that intentionally do not need domain objects.

- [ ] **Step 3: Generate the logical payload cases**

Add the `NestedPayloadCase` values from section 3.5 plus:

- maximum `u8` count with empty payloads;
- maximum `u8` varData payload;
- a parent-entry varData case for `group-with-data-schema.xml::TestMessage3`;
- a two-varData-per-entry case for `TestMessage2` and `TestMessage4`;
- a depth-three count case for `nested-group-schema.xml::Top`.

The renderer names tests deterministically using:

```text
length_{logical_case}_{outer_length_mode}_{nested_length_mode}
_encode_{outer_encoder_mode}_{nested_encoder_mode}_{fixed_write_mode}
```

For example:

```text
length_uniform_many_uniform_known_uniform_known
_encode_known_unknown_size_fixed_struct
```

- [ ] **Step 4: Generate every applicable shape-mode cross product**

Add named runtime cases for:

| Outer mode | Nested mode | Entry shape |
|---|---|---|
| Uniform known | Uniform known | Identical |
| Uniform known | Ragged known | Inner lengths differ |
| Uniform known | Unknown | Inner count discovered |
| Ragged known | Uniform known | Outer entries differ |
| Ragged known | Ragged known | Both levels differ |
| Ragged known | Unknown | Outer entries and inner counts differ |
| Unknown | Uniform known | Outer count discovered |
| Unknown | Ragged known | Outer count discovered, inner lengths differ |
| Unknown | Unknown | Both counts discovered |

For each logical value with multiple construction modes, assert equal computed
lengths. Track an invocation counter per mode pair and assert all nine length
mode pairs ran at least once.

- [ ] **Step 5: Cross every applicable length mode with codec modes**

For each logical case and applicable length mode pair, run all:

```text
2 outer encoder modes
× 2 nested encoder modes
× 2 fixed-write modes
= 8 encoder paths
```

For each path:

- allocate the exact computed length;
- encode using the unchanged generated encoder;
- verify completed encoder body/header lengths;
- verify `as_bytes()`;
- decode and verify all fixed fields, group counts, nested payload bytes, and
  message varData;
- construct the domain object for the same logical value;
- verify domain body/header lengths;
- encode the domain object into an exact buffer;
- require byte identity between flyweight and domain-object encoding.

This creates thousands of runtime assertions from a bounded number of
compiled helper functions.

- [ ] **Step 6: Cover tail placements**

Add cases for:

- message-level varData only;
- one flat group;
- multiple flat top-level groups;
- entry varData only;
- two entry varData fields;
- nested fixed leaf;
- nested child varData;
- nested group followed by parent-entry varData;
- message varData after all groups;
- depth-three nested fixed groups;
- multiple nested sibling groups from the Binance production schema at
  source-generation level.

- [ ] **Step 7: Cover cardinalities and byte lengths**

Use:

- zero entries;
- one entry;
- two identical entries;
- ragged `[0, 1, 3]` nested counts;
- maximum `u8` group count;
- empty varData;
- one-byte varData;
- UTF-8 multibyte content measured in bytes;
- embedded zero bytes in binary varData;
- maximum `u8` varData length.

- [ ] **Step 8: Assert the complete length invariant**

Every successful runtime case must assert:

```rust
assert_eq!(computed_body, complete.encoded_length());
assert_eq!(computed_full, complete.encoded_length_with_header());
assert_eq!(computed_full, complete.as_bytes().len());
assert_eq!(computed_full, decoder.encoded_length_with_header()?);
assert_eq!(computed_full, exact_buffer.len());
```

When domain objects are enabled, also assert:

```rust
assert_eq!(computed_body, dto.encoded_length()?);
assert_eq!(computed_full, dto.encoded_length_with_header()?);
```

- [ ] **Step 9: Add exact-buffer boundary cases**

For each representative direct and staged shape:

1. exact length succeeds;
2. one-byte-short buffer returns `BufferTooShort`;
3. one-byte-long buffer still reports only the encoded prefix length;
4. `as_bytes().len()` equals the encoded prefix, not backing capacity.

- [ ] **Step 10: Add compile-fail type-state proofs**

Use `compile_fails()` to prove:

- `asks` cannot precede completed non-empty `bids`;
- a consumed uniform stage cannot be reused;
- `encoded_length()` is unavailable before required non-empty tails complete;
- nested varData cannot precede its nested group in an entry with both;
- encoder and decoder interfaces remain unchanged.

- [ ] **Step 11: Add error cases**

Cover:

- known ragged too few;
- known ragged too many;
- unknown `u8` count overflow;
- `u8` varData overflow;
- checked direct-helper overflow through a unit-level accumulator test;
- skipped non-empty uniform shape;
- one-byte-short buffer;
- truncated nested decode.

- [ ] **Step 12: Add zero-allocation proof**

In `sbe/tests/allocation_count_test.rs`, reset the counting allocator, compute
both a uniform and ragged nested length, and assert zero allocations occurred
during each builder chain. Keep schema generation and any `Vec` allocation
outside the measured region.

- [ ] **Step 13: Add matrix cardinality assertions**

At the end of matrix rendering and execution, assert:

- all logical cases were used;
- all nine length outer/nested mode pairs were used where structurally
  applicable;
- all four encoder outer/nested mode pairs were used;
- both fixed-field write modes were used;
- every successful test checked flyweight encode, decoder, domain object,
  exact buffer, and byte identity;
- no test name was duplicated.

Print a compact summary containing generated test count and assertion-path
count. The count is evidence, not a fixed golden number; the structural
coverage assertions are the acceptance gate.

- [ ] **Step 14: Run the conformance matrix**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test -- --nocapture
cargo test -p ergo-sbe --test conformance_test -- --nocapture
cargo test -p ergo-sbe --test l3_orderbook_test -- --nocapture
cargo test -p ergo-sbe --test domain_objects_test -- --nocapture
cargo test -p ergo-sbe --test allocation_count_test -- --test-threads=1 --nocapture
```

Expected result: all pass.

- [ ] **Step 15: Commit**

```sh
git add sbe/tests/common/mod.rs sbe/tests/common/encoded_length_matrix.rs sbe/tests/conformance_test.rs sbe/tests/l3_orderbook_test.rs sbe/tests/domain_objects_test.rs sbe/tests/allocation_count_test.rs
git commit -m "test: cover encoded length interface matrix"
```

### Task 9: Migrate existing length-builder callers without touching codec calls — audited partial with regressions

**Files:**

- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/conformance_test.rs`
- Modify: `sbe/tests/domain_objects_test.rs`
- Modify: `sbe/tests/l3_orderbook_test.rs`
- Modify: `sbe/tests/ordered_decoder_stages_test.rs`
- Modify: `samples/l3-book/src/main.rs`
- Modify: `samples/l3-book/tests/l3_tests.rs`

**Interfaces:**

- Produces: all existing tests and samples using the new length interface.
- Preserves: all encoder and decoder call chains.

- [ ] **Step 1: Replace simple builder calls with direct helpers**

Examples:

```rust
let len = FlatGroupEncoder::try_compute_encoded_length_with_header(
    2,
    1,
    b"test exchange data".len(),
)?;
```

Do not change the subsequent `FlatGroupEncoder` calls.

- [ ] **Step 2: Replace uniform complex calls**

Replace:

```rust
.bids(1, |bids| {
    bids.add()?;
    bids.orders(2, |orders| {
        orders.add()?;
        orders.add()?;
        Ok(())
    })?;
    Ok(())
})?
```

with:

```rust
.bids(1)
.orders(2)?
```

when the nested entries are fixed width.

Replace nested varData with:

```rust
.bids(1)
.orders(2)
.order_id(5)?
```

when both order IDs have the same byte length.

- [ ] **Step 3: Replace ragged complex calls**

For per-level order counts `[2, 1]`, use:

```rust
.bids_ragged(2, |bids| {
    bids.orders(2)?;
    bids.orders(1)?;
    Ok(())
})?
```

For nested order IDs with different lengths, use automatic entry chains under
`bids_ragged` and/or `orders_ragged`.

- [ ] **Step 4: Preserve encoder and decoder syntax byte-for-byte where possible**

Only pre-encoding length calculations change. Any diff in generated encoder
or decoder use must be reverted unless required by an unrelated pre-existing
compile failure.

- [ ] **Step 5: Search for stale builder boilerplate**

Run:

```sh
rg -n "EncodedLength::new\\(|\\.add_n\\(|\\.add\\(\\)\\?" sbe/tests samples/l3-book -g '*.rs'
```

Expected result:

- no simple message uses `EncodedLength::new()`;
- no new length chain uses `add()` or `add_n()`;
- encoder closures still use their unchanged `add`, `add_struct`, and entry
  setters.

- [ ] **Step 6: Run migrated tests and sample**

Run:

```sh
cargo test -p ergo-sbe --all-features -- --test-threads=1
cd samples/l3-book && cargo test
cd samples/l3-book && cargo run
```

Expected result: all pass and the sample prints a computed length equal to the
actual encoded length.

- [ ] **Step 7: Commit**

```sh
git add sbe/tests/baseline_test.rs sbe/tests/conformance_test.rs sbe/tests/domain_objects_test.rs sbe/tests/l3_orderbook_test.rs sbe/tests/ordered_decoder_stages_test.rs samples/l3-book/src/main.rs samples/l3-book/tests/l3_tests.rs
git commit -m "refactor: migrate encoded length callers"
```

### Task 10: Prove production-scale schema generation — audited partial

**Files:**

- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/schema_edge_cases_test.rs`
- Test: the files above

**Interfaces:**

- Produces: source-generation proof for all representative schema families.
- Consumes: the completed encoded-length generator.

- [ ] **Step 1: Extend the existing real-world generation tests**

For `binance_spot_3_5.xml`, assert generated source:

- parses with `syn`;
- contains direct helpers for `DepthResponse`;
- omits `DepthResponseEncodedLength`;
- contains staged builders for `ExchangeInfoResponse`;
- contains uniform and ragged methods for its dynamic groups;
- has no duplicate inherent methods.

- [ ] **Step 2: Extend schema-edge generation tests**

Assert successful generation for:

- `basic-schema.xml`;
- `basic-variable-length-schema.xml`;
- `basic-group-schema.xml`;
- `group-with-data-schema.xml`;
- `nested-group-schema.xml`;
- `u8-dimension-schema.xml`;
- `example-bigendian-test-schema.xml`;
- `custom-header-type.xml`;
- `l3-orderbook-schema.xml`;
- `cme_templates_FixBinary.xml`;
- `ilinkbinary.xml`.

- [ ] **Step 3: Run production schema generation**

Run:

```sh
cargo test -p ergo-sbe --test baseline_test generate_multi_message_schema -- --nocapture
cargo test -p ergo-sbe --test baseline_test binance_spot_schema_compiles -- --nocapture
cargo test -p ergo-sbe --test baseline_test cme_fix_binary_schema_compiles -- --nocapture
cargo test -p ergo-sbe --test schema_edge_cases_test -- --nocapture
```

Expected result: all pass with no generated duplicate-method or unresolved
stage errors.

- [ ] **Step 4: Commit**

```sh
git add sbe/tests/baseline_test.rs sbe/tests/schema_edge_cases_test.rs
git commit -m "test: verify length APIs across production schemas"
```

### Task 11: Audit generated interface cleanliness and usability — audited partial

**Files:**

- Modify: `sbe/tests/encoded_length_api_test.rs`
- Create: `sbe/tests/golden/encoded_length_api.txt`
- Test: `sbe/tests/encoded_length_api_test.rs`

**Interfaces:**

- Produces: AST cleanliness checks, curated ergonomics compile tests, and a
  compact public encoded-length interface golden.
- Consumes: generated source from representative direct and staged schemas.

- [ ] **Step 1: Add curated handwritten compile tests**

Keep these as literal Rust snippets, not matrix-rendered strings:

```rust
let fixed = FixedOnlyEncoder::ENCODED_LENGTH;

let direct = FlatGroupEncoder::try_compute_encoded_length_with_header(
    2,
    1,
    7,
)?;

let uniform = L3BookVarDataEncodedLength::new()
    .bids(2)
    .orders(2)
    .order_id(5)?
    .asks(0)
    .symbol(7)?
    .encoded_length_with_header();

let ragged = L3BookVarDataEncodedLength::new()
    .bids_ragged(2, |bids| {
        bids.orders(2).order_id(5)?;
        bids.orders(1).order_id(3)?;
        Ok(())
    })?
    .asks(0)
    .symbol(7)?
    .encoded_length_with_header();

let unknown = L3BookVarDataEncodedLength::new()
    .bids_unknown_size(|bids| {
        bids.orders_unknown_size(|orders| {
            orders.order_id(5)?;
            orders.order_id(3)?;
            Ok(())
        })?;
        Ok(())
    })?
    .asks(0)
    .symbol(0)?
    .encoded_length_with_header();

assert!(fixed > 0);
assert!(direct > 0);
assert!(uniform > 0);
assert!(ragged > 0);
assert!(unknown > 0);
```

Add a separate depth-three snippet:

```rust
let depth_three = TopEncodedLength::new()
    .x(2)
    .y(3)
    .z(4)?
    .encoded_length_with_header();
assert!(depth_three > 0);
```

- [ ] **Step 2: Build an inherent-method index from the `syn` AST**

Walk generated `Item::Impl` blocks and collect:

```rust
std::collections::BTreeMap<(String, String), usize>
```

where the key is `(self_type_tokens, method_name)`. Assert every count is one.
Limit the audit to inherent impls; trait implementations may legitimately use
the same method names on a type.

- [ ] **Step 3: Assert strategy and boilerplate rules from the AST**

Verify:

- fixed and direct messages have no struct whose name ends with
  `EncodedLength`;
- staged messages have an entry-point struct and complete stage;
- no inherent impl on a type containing `EncodedLength` defines `add` or
  `add_n`;
- public user entry methods are `group`, `group_ragged`, and
  `group_unknown_size` according to group shape;
- plumbing structs carry `#[doc(hidden)]`;
- only complete or zero-pending terminal stages expose length result methods.

- [ ] **Step 4: Extract the public signature golden**

Render one line per public encoded-length item, sorted in generated source
order:

```text
pub struct CarEncodedLength
pub const fn CarEncodedLength::fuel_figures(self, count: u16) -> CarFuelFiguresUniformEncodedLength
pub fn CarEncodedLength::fuel_figures_ragged<F>(self, count: u16, f: F) -> Result<CarEncodedLengthAfterFuelFigures, sbe_rt::EncodeError>
pub const fn CarFuelFiguresUniformEncodedLength::usage_description(self, byte_len: usize) -> Result<CarEncodedLengthAfterFuelFigures, sbe_rt::EncodeError>
pub const fn CarEncodedLengthAfterFuelFigures::performance_figures(self, count: u16) -> CarPerformanceFiguresUniformEncodedLength
pub fn CarEncodedLengthAfterFuelFigures::performance_figures_ragged<F>(self, count: u16, f: F) -> Result<CarEncodedLengthAfterPerformanceFigures, sbe_rt::EncodeError>
pub const fn CarEncodedLengthComplete::encoded_length(&self) -> usize
pub const fn CarEncodedLengthComplete::encoded_length_with_header(&self) -> usize
```

The extractor uses parsed `syn` nodes and `quote::ToTokens`, then normalises
whitespace. It must not use line-based regex extraction.

- [ ] **Step 5: Add an ignored golden update test**

Follow `sbe/tests/stability_test.rs`:

```rust
#[test]
#[ignore = "run manually to regenerate the encoded-length API golden"]
fn update_encoded_length_api_golden()
-> Result<(), Box<dyn std::error::Error>> {
    let signatures = extract_encoded_length_signatures()?;
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/golden/encoded_length_api.txt",
        ),
        signatures,
    )?;
    Ok(())
}
```

- [ ] **Step 6: Add formatting idempotence**

Parse the generated source with `syn`, format it with `prettyplease`, parse and
format the result again, and assert both formatted strings are identical.

- [ ] **Step 7: Audit const qualification**

Inspect `syn::Signature::constness` and require `const fn` on every method in
section 3.7's const-capable list. Require no const qualifier on
`*_ragged` and `*_unknown_size` methods.

Compile the module-scope `DIRECT_LENGTH` and `UNIFORM_LENGTH` constants from
section 3.7. Assert their values equal runtime results.

- [ ] **Step 8: Run cleanliness tests**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test curated_ergonomics_ -- --nocapture
cargo test -p ergo-sbe --test encoded_length_api_test generated_interface_ -- --nocapture
cargo test -p ergo-sbe --test encoded_length_api_test formatting_is_idempotent -- --nocapture
cargo test -p ergo-sbe --test encoded_length_api_test generated_const_ -- --nocapture
```

Expected result: all pass.

- [ ] **Step 9: Generate and review the API golden**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test update_encoded_length_api_golden -- --ignored --nocapture
git diff -- sbe/tests/golden/encoded_length_api.txt
```

Review the short signature list for:

- concise common paths;
- explicit ragged naming;
- no exposed `add`/`add_n`;
- no duplicate methods;
- no builder on simple messages;
- no accidental public plumbing methods;
- const qualifiers exactly match the const-evaluation policy.

- [ ] **Step 10: Commit**

```sh
git add sbe/tests/encoded_length_api_test.rs sbe/tests/golden/encoded_length_api.txt
git commit -m "test: audit encoded length interface"
```

### Task 12: Regenerate the golden source and review the public interface diff — audited partial

**Files:**

- Modify by generator: `sbe/tests/golden/car_example.rs`
- Test: `sbe/tests/stability_test.rs`

**Interfaces:**

- Produces: deterministic golden output for the new complexity-based length
  interface.
- Consumes: all generator changes.

- [ ] **Step 1: Regenerate through the existing ignored test**

Run:

```sh
cargo test -p ergo-sbe --test stability_test update_golden -- --ignored --nocapture
```

Expected result: `sbe/tests/golden/car_example.rs` is rewritten.

- [ ] **Step 2: Inspect only the encoded-length portions**

Run:

```sh
git diff -- sbe/tests/golden/car_example.rs
rg -n "EncodedLength|compute_encoded_length|try_compute_encoded_length|finish_empty|_ragged" sbe/tests/golden/car_example.rs
```

Confirm:

- `Car` remains staged because `performanceFigures` contains nested
  `acceleration` and `fuelFigures` contains entry varData;
- `fuelFigures(count).usage_description(byte_len)?` is the uniform
  entry-varData path;
- `fuelFigures_ragged(count, closure)` represents differing
  `usageDescription` lengths;
- `performanceFigures(count)` is the uniform path;
- `performanceFigures_ragged(count, closure)` exists;
- no length-builder `add()` or `add_n()` remains;
- encoder and decoder sections have no interface diff caused by this work;
- generated source contains one accumulator definition.

- [ ] **Step 3: Run golden stability and allocation tests**

Run:

```sh
cargo test -p ergo-sbe --test stability_test generated_output_matches_golden -- --nocapture
cargo test -p ergo-sbe --test allocation_count_test -- --test-threads=1 --nocapture
```

Expected result: both pass.

- [ ] **Step 4: Commit**

```sh
git add sbe/tests/golden/car_example.rs
git commit -m "test: update encoded length golden output"
```

### Task 13: Update user documentation around the three strategies — audited partial; examples require correction

**Files:**

- Modify: `sbe/README.md`
- Modify: `samples/l3-book/README.md`
- Modify: `docs/design/2026-07-23-exact-length-builder-and-conformance-tests.md`

**Interfaces:**

- Produces: one documented decision tree and copy-pasteable examples.
- Consumes: final generated method names.

- [ ] **Step 1: Document the decision tree**

Add:

```text
Fixed fields only
    → Encoder::ENCODED_LENGTH

Only message varData and/or fixed-width top-level groups
    → Encoder::try_compute_encoded_length_with_header(group_count, data_len)

Any group entry containing a nested group or varData
    → MessageEncodedLength::new()
```

- [ ] **Step 2: Document uniform, ragged, and unknown-size complex examples**

Use the exact examples from section 2.3. Explicitly state:

- uniform uses `group(count)`;
- known ragged uses `group_ragged(count, closure)`;
- unknown uses `group_unknown_size(closure)`;
- zero normally forwards without a closure;
- `finish_empty()` is the explicit collision-safe form;
- encoder and decoder interfaces are unchanged.

- [ ] **Step 3: Add a supersession note to the earlier design**

The note must state that this plan supersedes only the earlier design's
"builder for all dynamic tails" interface choice. It retains the earlier
correctness, checked arithmetic, domain-length, conformance, and performance
requirements.

- [ ] **Step 4: Verify documentation code snippets**

Run:

```sh
rg -n "\\.add\\(\\)\\?|\\.add_n\\(" sbe/README.md samples/l3-book/README.md docs/design/2026-07-23-exact-length-builder-and-conformance-tests.md
```

Expected result: no length-builder examples use `add()` or `add_n()`. Encoder
examples may still use encoder `add()` because that interface is unchanged.

- [ ] **Step 5: Commit**

```sh
git add sbe/README.md samples/l3-book/README.md docs/design/2026-07-23-exact-length-builder-and-conformance-tests.md
git commit -m "docs: explain encoded length strategy selection"
```

### Task 14: Run the full correctness and performance gates — audited not done; gates currently red

**Files:**

- No source changes expected.
- If a gate fails, return to the task that owns the failing behaviour and
  repeat its red/green cycle before rerunning this task.

**Interfaces:**

- Produces: final verification evidence.
- Consumes: all implementation tasks.

- [ ] **Step 1: Check repository diff hygiene**

Run:

```sh
git status --short
git diff --check
```

Expected result: no whitespace errors and no unrelated changes. Preserve the
existing untracked nested `simple-binary-encoding` repository state.

- [ ] **Step 2: Run formatting**

Run:

```sh
cargo fmt --all --check
cd samples/l3-book && cargo fmt --check
```

Expected result: both pass.

- [ ] **Step 3: Run focused SBE tests**

Run:

```sh
cargo test -p ergo-sbe --test encoded_length_api_test -- --nocapture
cargo test -p ergo-sbe --test conformance_test -- --nocapture
cargo test -p ergo-sbe --test l3_orderbook_test -- --nocapture
cargo test -p ergo-sbe --test domain_objects_test -- --nocapture
cargo test -p ergo-sbe --test allocation_count_test -- --test-threads=1 --nocapture
cargo test -p ergo-sbe --test stability_test -- --nocapture
```

Expected result: all pass.

- [ ] **Step 4: Run the complete ergo-sbe suite**

Run:

```sh
cargo test -p ergo-sbe --all-features -- --test-threads=1
```

Expected result: all unit, integration, compile-fail, property, and golden
tests pass with no new ignored tests.

- [ ] **Step 5: Run Clippy**

Run:

```sh
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
cd samples/l3-book && cargo clippy --all-targets -- -D warnings
```

Expected result: no warnings.

- [ ] **Step 6: Run sample tests**

Run:

```sh
cd samples/l3-book && cargo test -- --test-threads=1
cd samples/exchange-example && cargo test -- --test-threads=1
cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
cd samples/cluster-rfq && cargo test
```

Expected result: all available offline sample tests pass.

- [ ] **Step 7: Run the mandatory SBE parity benchmark**

Run from the repository root:

```sh
just bench
```

Expected result:

- every maintained ergon/Aeron ratio is at or below `1.00`;
- no generated encoder or decoder hot-path regression;
- record the date, host, Rust toolchain, Criterion medians, confidence
  intervals, and gate output in the execution transcript.

If any maintained ratio exceeds `1.00`, do not accept the change. Diagnose the
generated diff and revise or revert the responsible implementation while
preserving the length-interface contract.

- [ ] **Step 8: Run the repository product gate**

Run:

```sh
just check-products
```

Expected result: formatting, Clippy, ergo-sbe tests, and cluster library tests
pass.

- [ ] **Step 9: Final diff review**

Run:

```sh
git diff --stat
git diff -- sbe/src/codegen/mod.rs sbe/src/codegen/encoded_length.rs
git status --short
```

Confirm:

- no encoder or decoder public interface change;
- no wire-layout change;
- no allocation in generated length code;
- no stale simple builders;
- no length-builder `add()`/`add_n()` boilerplate;
- no unrelated files changed.

- [ ] **Step 10: Commit any verification-only documentation correction**

Only if the verification run required a documentation correction:

```sh
git add sbe/README.md samples/l3-book/README.md
git commit -m "docs: correct encoded length examples"
```

Do not create an empty commit when no correction was required.

---

## 6. Acceptance checklist

- [ ] Fixed-only messages use existing constants and generate no builder.
- [ ] Message-varData-only messages use direct helpers and generate no builder.
- [ ] One or many fixed-width top-level groups use direct helpers and generate no builder.
- [ ] A group with entry varData generates a staged builder.
- [ ] A group with any nested group generates a staged builder.
- [ ] Uniform nested syntax needs no closure and no `add()`:
  `.bids(2).orders(2).order_id(5)?`.
- [ ] Ragged known-size syntax uses one closure and no `add()`:
  `.bids_ragged(2, |bids| { bids.orders(1).order_id(5)?; bids.orders(2).order_id(3)?; Ok(()) })?`.
- [ ] Unknown-size dynamic syntax counts completed entry chains automatically.
- [ ] Fixed-width unknown-size length groups use `entries(n)`.
- [ ] Zero-count dynamic groups normally forward to the next owner tail.
- [ ] `finish_empty()` handles zero completion explicitly and safely.
- [ ] Known ragged groups reject too few and too many completed entries.
- [ ] Unknown groups reject counts that exceed the wire count primitive.
- [ ] VarData limits use the resolved length encoding.
- [ ] Every addition and multiplication on the checked interface detects overflow.
- [ ] Custom header sizes are respected.
- [ ] Little- and big-endian schemas calculate identical structural lengths for identical shapes.
- [ ] Exact builder/helper length equals encoder, decoder, byte-slice, domain-object, and exact-buffer length.
- [ ] Exact buffers succeed and one-byte-short buffers fail.
- [ ] Runtime length calculation allocates zero times.
- [ ] Encoder and decoder interfaces are unchanged.
- [ ] Existing tests and samples pass after length-call migration.
- [ ] Generated output remains deterministic.
- [ ] Production-scale Binance, CME, iLink, cluster, and sample schemas generate valid Rust.
- [ ] Every maintained benchmark ratio is at or below `1.00`.

---

## 7. Explicitly rejected alternatives

### Keep both `bids(count)` and `bids(count, closure)`

Rust does not overload inherent methods by arity. A trait method does not solve
this because an inherent method with the same name shadows trait lookup even
when the argument count differs. Use `bids(count)` for uniform and
`bids_ragged(count, closure)` for ragged.

### Generate builders for every dynamic message

The repository inventory shows that 260 dynamic messages are fully described
by top-level counts and message varData lengths. A builder adds interface
surface without representing additional information for those messages.

### Allocate a tree of entry-shape descriptors

This violates the zero-allocation constraint and makes exact length
calculation proportional to an intermediate heap representation. Weighted
accumulation computes the same result while the fluent chain executes.

### Require `add()` or `add_n()` in the new complex interface

Uniform group counts already declare multiplicity. Ragged dynamic entry
completion can be inferred from the final entry-tail stage. Keeping explicit
adds would preserve the boilerplate this redesign is intended to remove.

### Change encoder group methods to match the new length interface

Encoder closures write distinct field values and therefore still need one
entry operation per encoded entry. This work changes length calculation only;
encoder and decoder interfaces remain stable.

### Use const-generic counts

Syntax such as `.bids::<2>()` can distinguish zero at the type level but cannot
accept runtime collection lengths, which are the common case. Runtime counts
plus zero-aware pending stages preserve normal Rust usage.

### Silently accept a skipped non-empty uniform shape

That would undercount the wire message. Skipping is valid only when the
effective count is zero; otherwise the builder returns
`GroupCountMismatch`.
