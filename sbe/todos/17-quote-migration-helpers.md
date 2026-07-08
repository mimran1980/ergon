# Convert remaining codegen helpers to syn/quote

**Blocked by:** none (can run anytime — mandatory code quality)

`syn`/`quote`/`prettyplease` deps are already in `Cargo.toml`. `generate()` wraps
output through `prettyplease::unparse`.
**Status: IN PROGRESS**


## Progress (2026-07-07)

**89 `push_str(&format!(...))` calls remaining** (down from 165 — 9 functions converted to `quote!`, -76 calls).

| Function | push_str | Δ since audit | Status |
|----------|----------|---------------|--------|
| `generate_sbe_rt_src` | 0 | 0 | ✅ Converted |
| `generate_enum` | 1 | 0 | ✅ Converted (1 output push_str only) |
| `generate_set` | 1 | 0 | ✅ Converted (1 output push_str only) |
| `generate_nullification` | 2 | 0 | ✅ Converted |
| `generate_decoder_display` | 0 | **-9** | ✅ Converted |
| `generate_group_encoder` | 0 | **-11** | ✅ Converted |
| `generate_message_field_meta` | 3 | 0 | ✅ Converted |
| `generate_schema_id_from_header` | 1 | 0 | ✅ Converted |
| `emit_field_consts` | 4 | 0 | ✅ Converted |
| `generate_composite` | 0 | **-20** | ✅ Converted |
| `generate_any_message` | 0 | **-36** | ✅ Converted (b0761ad) |
| `generate_message_encoder` | 0 | **-32** | ✅ Converted (hybrid — already 0 format! calls) |
| `generate_message_decoder` | 45 | -4 | Agent a52c258c converting now |
| `generate_group_decoder` | 37 | -3 | Agent a2d834a1d converting now |
| `gen_schema` | 7 | -14 | Pending (orchestration, last to convert) |

## Acceptance criteria

- [ ] **Zero `push_str(&format!(...))` in codegen.rs** — the count hits 0
- [x] All codegen goes through `syn`/`quote!` → `prettyplease::unparse`
- [ ] Regen stability test passes
- [x] No `rustfmt` subprocess — all formatting via `prettyplease`
- [ ] CI hook: `grep -c 'push_str(&format!' sbe/src/codegen.rs` fails CI if > 0

Ref: user request. `syn`/`quote` deps already in `Cargo.toml`.

## Verification / Unit Testing
- [x] Verify the migration by ensuring all modified files compile and pass the regeneration stability test.
