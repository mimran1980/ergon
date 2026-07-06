# Complete `quote!` migration — remaining codegen sections

Track and complete the migration of remaining `push_str(&format!(...))` sections in
codegen.rs to `quote!`. Todo 17 tracks this at a high level, but the remaining
sections need an explicit checklist. CLAUDE.md says this is **non-negotiable** —
no new `push_str` additions.

**Status:** Not started

## Acceptance criteria

- [ ] Audit codegen.rs for all remaining `push_str` / `format!` / string-based template sections
- [ ] Convert `generate_composite` templates to `quote!`
- [ ] Convert `generate_message_decoder` templates to `quote!`
- [ ] Convert `generate_message_encoder` templates to `quote!`
- [ ] Convert `generate_group_decoder` templates to `quote!`
- [ ] Convert `generate_group_encoder` templates to `quote!`
- [ ] Convert `generate_any_message` templates to `quote!`
- [ ] Zero remaining `push_str(&format!(` calls in codegen.rs
- [ ] All codegen goes through `syn`/`quote!` → `prettyplease::unparse`
- [ ] Regen stability test passes
- [ ] No `rustfmt` subprocess — all formatting via `prettyplease`

## Dependencies

- `17-quote-migration-helpers` — foundation and helper utilities

## Notes

CLAUDE.md states:

> "No new push_str(&format!(...)) in codegen.rs. When modifying existing
> string-based templates, convert the affected section to quote! rather than
> adding more string pushers. This is non-negotiable."

The existing ~3000 lines of string-based templates are technical debt. Each
change should shrink that debt.
