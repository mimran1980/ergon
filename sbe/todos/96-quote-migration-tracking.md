# Complete `quote!` migration — remaining codegen sections

Track and complete the migration of remaining `push_str(&format!(...))` sections in
codegen.rs to `quote!`. Todo 17 tracks this at a high level, but the remaining
sections need an explicit checklist. CLAUDE.md says this is **non-negotiable** —
no new `push_str` additions.

**Status:** In progress — audit complete

## Audit findings (2026-07-06)

`sbe/src/codegen.rs` — 4610 lines, 60 top-level functions

### Raw counts

| Metric | Count |
|--------|-------|
| `push_str` calls | **203** (across 19 functions) |
| `quote!` invocations | **11** (across 6 functions) |
| `format!` calls | **~195** (nearly all paired with `push_str`) |
| Lines of string-based templates | **~3000** (as stated in CLAUDE.md, confirmed) |
| Functions fully converted | **3** |
| Functions partially converted | **5** |
| Functions still 100% string-based | **11** |

### Fully converted to `quote!`

| Function | Lines | Notes |
|----------|-------|-------|
| `generate_sbe_rt_src` | 282–392 | Pure `quote!` throughout. No `push_str` at all. |
| `generate_enum` | 1038–1171 | `quote!` for all generated tokens; `push_str` only for appending `prettyplease::unparse` output to `src`. |
| `generate_set` | 1173–1247 | Same pattern as `generate_enum`. |

### Partially converted (mixed `quote!` and `push_str`)

| Function | Lines | `quote!` blocks | `push_str` calls | Notes |
|----------|-------|-----------------|------------------|-------|
| `generate_nullification` | 3172–3217 | 1 | 2 | Uses `quote!` for byte-nullification statements, converts via `.to_string()`, then manually indents and appends via `push_str`. Close to done. |
| `generate_any_message` | 4018–4343 | 3 | 23 | `quote!` for the `MessageVisitor` trait and its dispatch method; string-based for `FrameCursor`, `AnyMessage` enum, `DecodedFrame`, `FramingPolicy`, and all their impls. |
| `generate_message_encoder` | 3219–3740 | 0 | 31 | Calls `generate_nullification` (which uses `quote!`); the encoder body itself is entirely string-based. |

### Not yet converted (100% `push_str` / `format!`)

| Function | Lines | `push_str` calls | Complexity |
|----------|-------|-----------------|------------|
| `generate_message_decoder` | 1638–2443 | **42** | **HIGH** — decoder struct, field getters for all types, tail-offset helpers, group accessors, var-data accessors, `verify()`, `Display` impl, `TryFrom`, `SbeMessage` impl, `AsRef` impl. The largest single function. |
| `generate_group_decoder` | 2525–3170 | **35** | **HIGH** — group decoder struct, entry decoder, `Iterator::next`, `ExactSizeIterator::len`, `tail_offset_0()`, tail-offset and var-data accessors for entry fields, nested-group support. |
| `generate_message_encoder` (body) | 3219–3740 | 31 | **HIGH** — encoder struct, `wrap()`, field setters (all types), tail-state methods, var-data methods, `Complete` state, `Sealed` / `SbeMessage` impls, `AsRef` impl. Type-state phantom pattern. |
| `gen_schema` | 132–279 | 21 | **MEDIUM** — top-level orchestration: creates `String` buffer, memoizes module paths, calls all sub-generators, writes `use` statements, applies `prettyplease::unparse`. |
| `generate_composite` | 1249–1543 | 20 | **MEDIUM** — composite type struct, getters for all member types (Primitive, Enum, Set, Composite), constructor `new(...)`. |
| `generate_any_message` (rest) | 4018–4343 | 20 | **MEDIUM** — `FrameCursor` impl (decode, decode_frame, encoded_length_with_header, as_bytes, encode), `AnyMessage` enum dispatch. |
| `generate_group_encoder` | 3742–3976 | 11 | **MEDIUM** — entry encoder with setters, nested groups, var-data. |
| `generate_decoder_display` | 2445–2523 | 8 | **LOW** — `Display` impl for message decoder. |
| `emit_field_consts` | 517–562 | 4 | **LOW** — field constant emission. |
| `generate_message_field_meta` | 4498–4533 | 3 | **LOW** — `MESSAGE_FIELD_META` constant emission. |
| `generate_schema_id_from_header` | 3978–4016 | 1 | **LOW** — `schema_id()` / `schema_version()` methods. |

### Function-to-`push_str` detailed mapping

```
 42  generate_message_decoder()       1638-2443   HIGH
 35  generate_group_decoder()          2525-3170   HIGH
 31  generate_message_encoder()        3219-3740   HIGH
 21  gen_schema()                      132-279     MEDIUM
 23  generate_any_message()            4018-4343   MEDIUM
 20  generate_composite()              1249-1543   MEDIUM
 11  generate_group_encoder()          3742-3976   MEDIUM
  8  generate_decoder_display()        2445-2523   LOW
  4  emit_field_consts()              517-562     LOW
  3  generate_message_field_meta()     4498-4533   LOW
  2  generate_nullification()          3172-3217   LOW (uses quote!)
  1  generate_schema_id_from_header()  3978-4016   LOW
  2  generate_enum()                   1038-1171   DONE (output only)
  2  generate_set()                    1173-1247   DONE (output only)
```

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

- [x] **Audit complete.**
- [ ] Convert `generate_nullification` — uses `quote!` partially, 2 `push_str` remain
- [ ] Convert `generate_composite` templates to `quote!`
- [ ] Convert `generate_message_decoder` templates to `quote!`
- [ ] Convert `generate_message_encoder` templates to `quote!`
- [ ] Convert `generate_group_decoder` templates to `quote!`
- [ ] Convert `generate_group_encoder` templates to `quote!`
- [ ] Convert `generate_any_message` remaining templates to `quote!`
- [ ] Zero remaining `push_str(- [x] Zero remaining `push_str(&format!(` calls in codegen.rsformat!(` calls in codegen.rs
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

The existing ~3000 lines of string-based templates are technical debt. Each
change should shrink that debt.
