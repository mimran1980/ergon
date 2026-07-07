# Convert remaining codegen helpers to syn/quote

**Blocked by:** none (can run anytime — mandatory code quality)

`syn`/`quote`/`prettyplease` deps are already in `Cargo.toml`. `generate()` wraps
output through `prettyplease::unparse`.

## REGRESSION WARNING (2026-07-07)

**The audit of 2026-07-06 counted 203 `push_str` calls. Today it's 217.**
New `push_str(&format!(...))` has been added to `generate_message_decoder` (+7),
`generate_group_decoder` (+5), `generate_message_encoder` (+1), and
`generate_decoder_display` (+1). **This is a direct violation of CLAUDE.md.**

```text
CLAUDE.md: "No new push_str(&format!(...)) in codegen.rs. When modifying existing
string-based templates, convert the affected section to quote! rather than adding
more string pushers. This is non-negotiable."
```

**Every commit that touches codegen.rs MUST shrink the push_str count, never grow it.**

## Current audit (2026-07-07)

**217 `push_str` calls** across 13 functions (up from 203 on 2026-07-06).
**165 `push_str(&format!(...))` patterns**.

| Function | push_str | Δ since audit | Status |
|----------|----------|---------------|--------|
| `generate_sbe_rt_src` | 0 | 0 | ✅ Converted |
| `generate_enum` | 1 | 0 | ✅ Converted (1 output push_str only) |
| `generate_set` | 1 | 0 | ✅ Converted (1 output push_str only) |
| `generate_nullification` | 2 | 0 | ⚠️ Nearly done (1 quote! block already) |
| `generate_composite` | 20 | 0 | Pending |
| `generate_message_decoder` | 49 | **+7** | Pending (largest, growing) |
| `generate_message_encoder` | 32 | **+1** | Pending |
| `generate_group_decoder` | 40 | **+5** | Pending |
| `generate_group_encoder` | 11 | 0 | Pending |
| `generate_any_message` | 23 | 0 | ⚠️ Partial (visitor uses quote!) |
| `generate_decoder_display` | 9 | **+1** | Pending |
| `generate_message_field_meta` | 3 | 0 | Pending |
| `generate_schema_id_from_header` | 1 | 0 | Pending |
| `emit_field_consts` | 4 | 0 | Pending |
| `gen_schema` | 21 | 0 | Pending (orchestration, last to convert) |

## Acceptance criteria

- [ ] **Zero `push_str(&format!(...))` in codegen.rs** — the count hits 0
- [ ] All codegen goes through `syn`/`quote!` → `prettyplease::unparse`
- [ ] Regen stability test passes
- [x] No `rustfmt` subprocess — all formatting via `prettyplease`
- [ ] CI hook: `grep -c 'push_str(&format!' sbe/src/codegen.rs` fails CI if > 0

Ref: user request. `syn`/`quote` deps already in `Cargo.toml`.

## Verification / Unit Testing
- [x] Verify the migration by ensuring all modified files compile and pass the regeneration stability test.
