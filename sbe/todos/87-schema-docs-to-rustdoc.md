# Schema XML descriptions → rustdoc comments

Emit `///` rustdoc comments on generated types and accessors from XML `description`
attributes, `<!-- -->` comments, and `description` child elements. This is specified
in DECISIONS.md §9 as a core helper.

## Status: Message-level decoder struct only

## Acceptance Criteria (SIMPLE cut — todo 87)

- [x] XML `description` attribute on messages → `///` doc on decoder struct
- [x] XML `description` on messages → `///` doc on encoder struct
- [ ] XML `description` on fields → `///` doc on field accessor methods
- [ ] XML `description` on enum `<validValue>` → `///` doc on enum variant constants
- [ ] XML `description` on set `<choice>` → `///` doc on set accessor methods
- [ ] XML `description` on composites → `///` doc on composite struct
- [ ] XML `<!-- -->` comments associated to nearest element → combined into `///` doc
- [ ] `semanticType` → `/// Semantic type: Price` line appended to field doc
- [ ] Schema-level `description` → module-level doc comment
- [ ] Multi-line descriptions handled correctly (wrapped at 80 chars)
- [ ] Test: `cargo doc --no-deps` produces clean output with schema descriptions
- [ ] Golden file updated

## Dependencies

- 64-schema-doc-comments (related but 64 focuses on CompatibilityMode gating; this is the core impl)

## Notes

- DECISIONS.md §9 and §4 specify this.
- `roxmltree` already preserves comment nodes.
- The IR already captures description strings — this is about emitting them in codegen.
