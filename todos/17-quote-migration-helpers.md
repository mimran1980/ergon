# Convert remaining codegen helpers to syn/quote

**Blocked by:** none (can run anytime, cosmetic improvement)

`syn`/`quote`/`prettyplease` deps are already in `Cargo.toml`. `generate()` wraps
output through `prettyplease::unparse`. ~135 `push_str`/`format!` calls remain
across `generate_enum`, `generate_set`, `generate_composite`,
`generate_message_decoder`, `generate_message_encoder`, `generate_group_decoder`,
`generate_group_encoder`, `generate_nullification`, and `generate_any_message`.
Convert them to `quote!` for readability.

## Acceptance criteria

- [x] Convert `generate_enum` templates to `quote!` (7 template blocks)
- [x] Convert `generate_set` templates to `quote!` (4 blocks)
- [ ] Convert `generate_composite` templates to `quote!` (field accessors, encoder)
- [ ] Convert `generate_message_decoder` templates to `quote!`
- [ ] Convert `generate_message_encoder` templates to `quote!`
- [ ] Convert `generate_group_decoder` and `generate_group_encoder` to `quote!`
- [x] Convert `generate_nullification` to `quote!`
- [ ] Convert `generate_any_message` to `quote!`
- [x] All existing tests still pass after conversion
- [x] Generated output is semantically identical (regen-stability test catches regressions)

Ref: user request. `syn`/`quote` deps already in `Cargo.toml`.
