# Convert remaining codegen helpers to syn/quote

**Blocked by:** none (can run anytime, cosmetic improvement)

`syn`/`quote`/`prettyplease` deps are already in `Cargo.toml`. `generate()` wraps
output through `prettyplease::unparse`. As of 2026-07-06 audit, **155**
`push_str`/`format!` calls remain across `generate_composite`,
`generate_message_decoder`, `generate_message_encoder`, `generate_group_decoder`,
`generate_group_encoder`, `generate_any_message`, `generate_decoder_display`,
`generate_message_field_meta`, and `generate_schema_id_from_header`.
Convert them to `quote!` for readability.

## Audit status (2026-07-06)

| Function | Lines | push_str/format! | quote! | Status |
|----------|-------|-----------------|--------|--------|
| `generate_sbe_rt_src` | 284-394 | 0 | 1 | Converted (not in original scope) |
| `generate_enum` | 1040-1229 | 0 | 7 | Converted |
| `generate_set` | 1231-1305 | 0 | 2 | Converted |
| `generate_nullification` | 3245-3290 | 0 | 1 | Converted (statement body uses quote!, wrapper uses push_str) |
| `generate_composite` | 1307-1601 | ~28 | 0 | **Pending** |
| `generate_message_decoder` | 1696-2509 | ~55 | 0 | **Pending** (largest remaining) |
| `generate_message_encoder` | 3292-3821 | ~38 | 0 | **Pending** |
| `generate_group_decoder` | 2591-3243 | ~40 | 0 | **Pending** |
| `generate_group_encoder` | 3823-4057 | ~18 | 0 | **Pending** |
| `generate_any_message` | 4099-4424 | ~20 | 3 | **Pending** (visitor section uses quote!, rest is format!) |
| `generate_decoder_display` | 2511-2589 | ~8 | 0 | Not tracked (nice-to-have) |
| `generate_message_field_meta` | 4578-4613 | ~4 | 0 | Not tracked (nice-to-have) |
| `generate_schema_id_from_header` | 4059-4097 | ~1 | 0 | Not tracked (nice-to-have) |

## Acceptance criteria

- [x] Convert `generate_enum` templates to `quote!` (7 template blocks)
- [x] Convert `generate_set` templates to `quote!` (4 blocks)
- [ ] Convert `generate_composite` templates to `quote!` — **~28 push_str calls**
- [ ] Convert `generate_message_decoder` templates to `quote!` — **~55 push_str calls**
- [ ] Convert `generate_message_encoder` templates to `quote!` — **~38 push_str calls**
- [ ] Convert `generate_group_decoder` and `generate_group_encoder` to `quote!` — **~40 + ~18 calls**
- [x] Convert `generate_nullification` to `quote!`
- [ ] Convert `generate_any_message` to `quote!` — **~20 push_str calls (visitor section uses quote!)**
- [x] All existing tests still pass after conversion
- [x] Generated output is semantically identical (regen-stability test catches regressions)
- [ ] (Nice-to-have) Convert `generate_decoder_display`, `generate_message_field_meta`, `generate_schema_id_from_header`

Ref: user request. `syn`/`quote` deps already in `Cargo.toml`.


## Verification / Unit Testing
- [ ] Verify the migration by ensuring all modified files compile and pass the regeneration stability test.
