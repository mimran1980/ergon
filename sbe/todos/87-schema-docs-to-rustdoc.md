# Schema XML descriptions → rustdoc comments

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
