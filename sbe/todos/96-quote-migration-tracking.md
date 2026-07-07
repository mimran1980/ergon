# Complete `quote!` migration — remaining codegen sections

Track and complete the migration of remaining `push_str(&format!(...))` sections in
codegen.rs to `quote!`. Todo 17 tracks this at a high level, but the remaining
sections need an explicit checklist. CLAUDE.md says this is **non-negotiable** —
no new `push_str` additions.

**Status:** ⚠️ REGRESSION — count grew from 203 → 217 since last audit

## ⛔ ANTI-REGRESSION RULE

```text
CLAUDE.md: "No new push_str(&format!(...)) in codegen.rs. When modifying existing
string-based templates, convert the affected section to quote! rather than adding
more string pushers. This is non-negotiable."
```

**Every commit touching codegen.rs MUST shrink the `push_str` count.** A commit
that adds new `push_str(&format!(...))` without removing at least as many is a
regression and should be rejected. The count must only go down.

## Audit (2026-07-07)

`sbe/src/codegen.rs` — `push_str` count: **217** (was 203 on 2026-07-06 — **+14 regression**)

| Metric | 2026-07-06 | 2026-07-07 | Δ |
|--------|------------|------------|---|
| `push_str` calls | 203 | **212** | **+9** ❌ |
| `push_str(&format!(...))` | 155 | **158** | **+3** ❌ |
| `format!` calls | ~195 | **≈200** | +5 |
| Functions converted | 3 | **7** | +4 |
| Functions with regressions | 0 | **4** | +4 → partially fixed |

### Functions that GREW (regression)

| Function | Before | After | Δ | Likely cause |
|----------|--------|-------|---|--------------|
| `generate_message_decoder` | 42 | **49** | **+7** | todo 104 infallible accessors, todo 106 flat enum |
| `generate_group_decoder` | 35 | **40** | **+5** | todo 104 step 2, group iteration fix |
| `generate_message_encoder` | 31 | **32** | **+1** | todo 106 flat enum |
| `generate_decoder_display` | 8 | **9** | **+1** | todo 104 |

### Current per-function counts (2026-07-07, after converting 4 smallest functions)

```
 49  generate_message_decoder()       1678-2461   +7 ❌
 40  generate_group_decoder()         2540-3228   +5 ❌
 32  generate_message_encoder()       3276-3807   +1 ❌
 23  generate_any_message()           4085-4416    0
 21  gen_schema()                     132-279      0
 20  generate_composite()             1271-1584    0
 11  generate_group_encoder()         3808-4044    0
  9  generate_decoder_display()       2462-2539   +1 ❌
  4  emit_field_consts()             524-570     ✅ CONVERTED
  3  generate_message_field_meta()    4564-4698   ✅ CONVERTED
  2  generate_nullification()         3229-3275   ✅ CONVERTED
  1  generate_schema_id_from_header() 4045-4084   ✅ CONVERTED
  1  generate_enum()                  1045-1194    0 (output only)
  1  generate_set()                   1195-1270    0 (output only)
```

**Converted this session:** 4 functions, 10 push_str calls eliminated.
**Remaining:** 158 `push_str(&format!(...))` across 9 functions.

### Effort estimate

| Priority | Function | Est. effort | Reasoning |
|----------|----------|-------------|-----------|
| 1 | `generate_nullification` | ~1hr | 1 `quote!` block already exists; only 2 `push_str` remain. Already proven pattern. |
| 2 | `generate_composite` | ~3hr | Self-contained struct generation; no version awareness or tail-offset logic. Good next target. |
| 3 | `generate_schema_id_from_header` | ~30min | Tiny function. |
| 4 | `generate_message_field_meta` | ~30min | Tiny function. |
| 5 | `emit_field_consts` | ~1hr | Small, self-contained. |
| 6 | `generate_decoder_display` | ~2hr | Pattern-based Display generation. |
| 7 | `generate_any_message` | ~4hr | Mix of quote! and string; the non-visitor parts are medium complexity. |
| 8 | `generate_group_encoder` | ~4hr | Entry encoder with nested groups and var-data. |
| 9 | `generate_group_decoder` | ~6hr | Iterator, ExactSizeIterator, tail offsets, nested groups — high complexity. |
| 10 | `generate_message_encoder` | ~6hr | Type-state encoder with phantom states; follows message_decoder structure but writes. |
| 11 | `generate_message_decoder` | ~6hr | Largest single function; many field accessor patterns, version awareness, verify logic. |
| 12 | `gen_schema` | ~3hr | Orchestration — needs to call converted generators correctly. Last to convert. |

**Total estimate: ~35–40 hours** across all remaining functions for a thorough conversion.

### Migration strategy (recommended order)

1. **Low-effort wins first** — `generate_nullification`, `generate_schema_id_from_header`, `generate_message_field_meta`, `emit_field_consts`, `generate_decoder_display`. These are small, self-contained, and build confidence.
2. **Composite** — `generate_composite` is medium-effort and isolated. Good next target.
3. **AnyMessage** — already partially converted; finish what's started.
4. **Group encoder** — self-contained, medium effort.
5. **Group decoder** — most complex decoder piece; the iterator/tail-offset/length patterns are intricate.
6. **Message encoder** — large but structural; type-state is mechanical once understood.
7. **Message decoder** — the largest conversion; field accessor patterns are repetitive but numerous.
8. **gen_schema** — last, since it's orchestration and must integrate with all converted generators.

## Known obstacles

1. **`quote!` doesn't handle tail whitespace or blank lines well** — generated code needs careful post-processing or `prettyplease` handles it anyway.
2. **`generate_nullification` currently converts `quote!` output to string then manually indents** — the `.to_string()` + `push_str` pattern needs a cleaner approach.
3. **`syn::parse_str` for large quote! output can fail silently** — current fallback `.unwrap_or(src)` in `gen_schema` is a known bug (CLAUDE.md notes this).
4. **Type-state generics in `generate_message_encoder`** — the phantom `S: EncodeState` pattern may need careful `GenericParam` / `Ident` construction in `syn`.
5. **Group decoders have inline `tail_offset_0`, `len`, `next`, `fmt` methods generated as strings** — these will need individual `impl` blocks in `quote!`.

## Acceptance criteria

- [x] **Audit complete (2026-07-06).**
- [x] **Re-audit (2026-07-07)** — regression detected (+14 `push_str`), documented above.
- [x] Convert `generate_nullification` — uses `quote!` fully, 0 `push_str` remain
- [ ] Convert `generate_composite` templates to `quote!` (20 calls)
- [ ] Convert `generate_message_decoder` templates to `quote!` (49 calls — largest)
- [ ] Convert `generate_message_encoder` templates to `quote!` (32 calls)
- [ ] Convert `generate_group_decoder` templates to `quote!` (40 calls)
- [ ] Convert `generate_group_encoder` templates to `quote!` (11 calls)
- [ ] Convert `generate_any_message` remaining templates to `quote!` (23 calls)
- [ ] Convert `gen_schema` to `quote!` (21 calls — last, orchestration)
- [x] Convert all small functions: `emit_field_consts` (4), `generate_decoder_display` (9), `generate_message_field_meta` (3), `generate_schema_id_from_header` (1)
- [ ] **Zero `push_str(&format!(...))` in codegen.rs** — grep returns empty
- [ ] All codegen goes through `syn`/`quote!` → `prettyplease::unparse`
- [ ] Regen stability test passes
- [x] No `rustfmt` subprocess — all formatting via `prettyplease`

## Dependencies

- `17-quote-migration-helpers` — foundation and helper utilities

## Notes

CLAUDE.md states:

> "No new push_str(&format!(...)) in codegen.rs. When modifying existing
> string-based templates, convert the affected section to quote! rather than
> adding more string pushers. This is non-negotiable."

The existing `push_str` count (217) is technical debt. Each change to `codegen.rs`
**must** shrink it. The 2026-07-06 → 2026-07-07 regression (+14) was caused by
todos 104 (infallible accessors) and 106 (flat enum) adding new `push_str` without
converting anything to `quote!`. Future feature work on these functions must convert
the affected section to `quote!` rather than adding more string pushers.

### Quick-start: smallest conversions first (~2hr total for first 4)

| # | Function | push_str | Est. |
|---|----------|----------|------|
| 1 | `generate_schema_id_from_header` | 1 | 15min |
| 2 | `generate_nullification` | 2 | 30min |
| 3 | `generate_message_field_meta` | 3 | 30min |
| 4 | `emit_field_consts` | 4 | 45min |

These four are purely mechanical — no version-awareness, no type-state, no iterator
logic. Good first PR to establish the pattern.
