# Verify `prettyplease` over `rustfmt` subprocess

**Blocked by:** none

The codegen already uses `prettyplease::unparse` for formatting generated code
(line 219 of `codegen.rs`). No external `rustfmt` process is spawned. But there
are gaps that need verification.

## Acceptance criteria

- [ ] Fix `.unwrap_or(src)` fallback at `codegen.rs:220` — silently returns
  unformatted code when `syn::parse_str` fails. Should be `expect("generated
  code must be valid Rust syntax")` or propagate the error.
- [ ] Verify every `generate()` code path goes through `prettyplease::unparse`
  (including `generate_multi`, `gen_schema`).
- [ ] `generate_sbe_rt_src()` (line 225) is NOT run through prettyplease —
  fix or document why it's exempt (short, hand-maintained, no benefit).
- [ ] CI `cargo fmt --all --check` should never need to reformat generated
  output — prettyplease output is already canonical.
- [ ] Audit `Cargo.toml` for any rustfmt-as-library dependency (shouldn't
  exist; prettyplease replaces it).

Ref: user instruction to use prettyplease instead of rustfmt subprocess.
