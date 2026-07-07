# Convert remaining codegen helpers to syn/quote

**Blocked by:** none (can run anytime — mandatory code quality)

`syn`/`quote`/`prettyplease` deps are already in `Cargo.toml`. `generate()` wraps
output through `prettyplease::unparse`.

## Progress (2026-07-07)

**125 `push_str(&format!(...))` calls remaining** (down from 165 — 7 functions converted to `quote!`, -40 calls).

| Function | push_str | Δ since audit | Status |
|----------|----------|---------------|--------|
| `generate_sbe_rt_src` | 0 | 0 | ✅ Converted |
| `generate_enum` | 1 | 0 | ✅ Converted (1 output push_str only) |
| `generate_set` | 1 | 0 | ✅ Converted (1 output push_str only) |
| `generate_nullification` | 2 | 0 | ✅ Converted (2026-07-07) |
| `generate_decoder_display` | 0 | **-9** | ✅ Converted (2026-07-07) |
| `generate_group_encoder` | 0 | **-11** | ✅ Converted (2026-07-07) |
| `generate_message_field_meta` | 3 | 0 | ✅ Converted (2026-07-07) |
| `generate_schema_id_from_header` | 1 | 0 | ✅ Converted (2026-07-07) |
| `emit_field_consts` | 4 | 0 | ✅ Converted (2026-07-07) |
| `generate_composite` | 0 | **-20** | ✅ Converted (2026-07-07) |
| `generate_message_decoder` | 49 | +7 | Pending (large) |
| `generate_message_encoder` | 32 | +1 | Pending |
| `generate_group_decoder` | 40 | +5 | Pending |
| `generate_any_message` | 23 | 0 | ⚠️ Partial (visitor uses quote!) |
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
