# Verify `prettyplease` over `rustfmt` subprocess

**Blocked by:** none

The codegen already uses `prettyplease::unparse` for formatting generated code
(line 219 of `codegen.rs`). No external `rustfmt` process is spawned. But there
are gaps that need verification.

## Acceptance criteria

- [x] Fix `.unwrap_or(src)` fallback at `codegen.rs:240` — silently returns
  unformatted code when `syn::parse_str` fails. Changed to `expect("generated
  code must be valid Rust syntax")`. Also fixed the same pattern in
  `generate_sbe_rt_src()` at line 318.
- [x] Verify every `generate()` code path goes through `prettyplease::unparse`
  (including `generate_multi`, `gen_schema`). All code paths call `gen_schema()`
  which has the formatting block.
- [x] `generate_sbe_rt_src()` output IS run through prettyplease — twice. First
  by its own inner formatting (line 316-318, now `expect()`), and again when its
  output is concatenated into `src` in `gen_schema()` and the whole thing is
  formatted at lines 238-240. The inner formatting is redundant but harmless.
- [x] CI `cargo fmt --all --check` should never need to reformat generated
  output — prettyplease output is already canonical. The stability golden test
  confirms generated output is unchanged.
- [x] Audit `Cargo.toml` for any rustfmt-as-library dependency (shouldn't
  exist; prettyplease replaces it). No rustfmt dependency found in workspace or
  crate Cargo.toml.

Ref: user instruction to use prettyplease instead of rustfmt subprocess.
