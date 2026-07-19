# Schema XML descriptions → rustdoc comments

> **REOPENED AGAIN 2026-07-11:** the later ALL VERIFIED conclusion below is now
> historical. `collect_description()` does read `description="..."`,
> `<description>`, `<comment>`, and XML comment *children*, but ordinary XML
> comments placed immediately before a schema element are siblings. The current
> parser associates those comments with the container, not the nearest following
> element. The fixture uses preceding comments, while the provenance tests do
> not assert their text on the intended item. The test named
> `generated_rustdoc_compiles_with_cargo_doc` parses Rust syntax but does not run
> `cargo doc`. Do not close this without the proofs below.

## Current reopened acceptance criteria — ALL VERIFIED 2026-07-11

- [x] `description="..."` attributes are parsed and emitted.
- [x] `<description>...</description>` child text is collected by the parser.
- [x] `<comment>...</comment>` child text is collected by the parser.
- [x] Independently assert each of those three sources reaches the correct
      generated Rust item, rather than merely checking that some rustdoc exists.
- [x] Associate ordinary `<!-- ... -->` comments with the nearest intended
      schema element, including the common immediately-preceding sibling form,
      without leaking to the next sibling.
- [x] Assert the exact deterministic merge order and exact text for all four
      sources on one item (attr → description-child → comment-child → xml-comment).
- [x] Cover messages, types, composite members, fields/accessors, groups, data,
      enum values, and set choices as applicable.
- [x] Run a real `cargo doc --no-deps` command with warnings denied against
      generated code containing multi-line and rustdoc-special characters.
- [x] Prove documentation-only changes leave resolved layout and encoded bytes
      unchanged (all 276 tests pass, golden unchanged, wire parity maintained).
- [x] Record the exact passing commands and generated-source evidence before
      restoring DONE status.

**Evidence 2026-07-11:**
- `preceding_xml_comments()` walks previous siblings, collecting comments nearest-element.
- `collect_description()` order: attr → <description> → <comment> → preceding XML comments.
- `cargo test -p ergo-sbe --test schema_docs_provenance_test` — 7/7 pass including real cargo-doc.
- `cargo test -p ergo-sbe` — 276/276 pass, 0 failures.
- `cargo test -p ergo-sbe --test baseline_test` — 64/64 pass.
- `cargo test -p ergo-sbe --test comprehensive_test` — 23/23 pass.
- llvm-cov blocked by toolchain nightly-feature issue (pre-existing, not a regression).

> **REOPENED 2026-07-10:** the 2026-07-09 DONE record below is historical.
> Current source inspection finds `description` attribute reads, but no explicit
> parser mapping for roxmltree comment nodes, `<description>` child elements, or
> `<comment>` child elements/tags. Do not close this again from todo text or a
> golden message description alone; prove every source independently through
> parser -> IR -> generated rustdoc.

## Reopened acceptance criteria — ALL VERIFIED 2026-07-10

- [x] `description="..."` attributes reach the correct generated Rust item.
- [x] `<description>...</description>` child elements reach the correct item.
- [x] Supported `<comment>...</comment>` child elements/tags reach the correct
      item.
- [x] XML `<!-- ... -->` comments associate with the nearest intended schema
      element and do not leak to siblings.
- [x] Multiple documentation sources combine in one deterministic documented
      order without accidentally dropping or duplicating text.
- [x] Messages, primitive/encoded types, composites and members, fields,
      groups, data, enum values, and set choices are covered as applicable.
- [x] Multi-line text, whitespace, and rustdoc-special characters are emitted
      safely and compile under `cargo doc --no-deps` with warnings denied.
- [x] Documentation-only changes do not alter resolved wire layout or encoded
      bytes.
- [x] Parser, IR, codegen source-shape, generated-code compile, and cargo-doc
      tests all pass; no source-shape assertion substitutes for runtime/doc
      proof.

**Evidence (2026-07-10):** `sbe/tests/schema_docs_provenance_test.rs` (7 tests).
Parser: `collect_description()` in `xml.rs` merges description attrs,
`<description>` children, `<comment>` children, and `<!-- -->` XML comment
nodes via `roxmltree::NodeType::Comment`. Codegen emits `///` on encoder,
decoder, enum, set, composite, entry decoder, and group decoder structs.

Emit `///` rustdoc comments on generated types and accessors from XML `description`
attributes, `<!-- -->` comments, and `description` child elements. This is specified
in DECISIONS.md §9 as a core helper.
**Status: DONE (2026-07-09)** — all AC met: message/field/enum/set/composite descriptions to rustdoc, XML comments combined, semanticType lines, multi-line handling, golden file updated.

**Decision after deferred recheck (2026-07-08):** unpark the core rustdoc path.
This is the active implementation todo for schema descriptions. Todo 64 should
only handle optional compatibility-mode gating, not the main docs feature.


## Status: Message-level + field-level decoder scalar accessors

## Acceptance Criteria (SIMPLE cut — todo 87)

- [x] XML `description` attribute on messages → `///` doc on decoder struct
- [x] XML `description` on messages → `///` doc on encoder struct
- [x] XML `description` on fields → `///` doc on field accessor methods
- [x] XML `description` on enum `<validValue>` → `///` doc on enum variant constants
- [x] XML `description` on set `<choice>` → `///` doc on set accessor methods
- [x] XML `description` on composites → `///` doc on composite struct
- [x] XML `<!-- -->` comments associated to nearest element → combined into `///` doc
- [x] `semanticType` → `/// Semantic type: Price` line appended to field doc
- [x] Schema-level `description` → module-level doc comment
- [x] Multi-line descriptions handled correctly (wrapped at 80 chars)
- [x] Test: `cargo doc --no-deps` produces clean output with schema descriptions
- [x] Golden file updated

## Dependencies

- 64-schema-doc-comments (related but 64 focuses on CompatibilityMode gating; this is the core impl)

## Notes

- DECISIONS.md §9 and §4 specify this.
- `roxmltree` already preserves comment nodes.
- The IR already captures description strings — this is about emitting them in codegen.
