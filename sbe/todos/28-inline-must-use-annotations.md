# Add `#[inline]` and `#[must_use]` annotations to generated code

**Blocked by:** `01-scalar-wire-parity`

Two missing annotations that matter for correctness and performance:

**`#[inline]`** — Upstream Java generator annotates every small method. ErgoSBE
only annotates `tail_offset_N()`. For HFT, LLVM needs `#[inline]` on every
field accessor to elide dead field reads and `bswap` on same-endian machines.
Without it, cross-crate inlining may not happen.

**`#[must_use]`** — Every decoder accessor returns `Result<T, DecodeError>` but
none has `#[must_use]`. A caller can silently drop a decode error. Encoder
type-state wrappers (`Needs*`) should also carry `#[must_use = "..."]`.

Also: `encoded_length()` silently falls back on error via `unwrap_or` instead
of propagating. Should return `Result<usize, DecodeError>`.
**Status: DONE (Phase 2 gate close)**


## Acceptance criteria

- [x] `#[inline]` on: every decoder field accessor (checked + unchecked + raw)
- [x] `#[inline]` on: group decoder `wrap`, `is_empty`, `skip`, `as_chunks`
- [x] `#[must_use]` on: encoder setters returning `&mut Self` (scalars, arrays, composites, enums, sets)
- [x] `#[inline]` on: encoder `wrap`, `wrap_and_apply_header`, `encoded_length`
- [x] `#[must_use]` on: `Result`-returning encoder methods (wrap_and_apply_header)
- [x] `#[inline]` on: every encoder setter method (skipped — setters get `#[must_use]`; both would be noise)
- [x] `#[must_use = "encoder must be consumed to write the message"]` on encoder type-state types
- [x] `encoded_length()` on group decoders returns `Result` instead of `unwrap_or` fallback
- [x] `#[cold]` on error-construction paths (already tracked in 08, verify)
- [x] `#[inline(always)]` only on the hottest one-liners (raw accessors, unchecked variants)
- [x] Benchmark: verify no regression from added attributes (should be zero-cost or improvement)

Ref: `design/DECISIONS.md` §10 codegen rules. `simple-binary-encoding/sbe-tool/src/main/java/uk/co/real_logic/sbe/generation/rust/RustGenerator.java`.


## Verification / Unit Testing
- [x] Verify `#[inline]` annotations exist on decoder field accessors, group decoder methods, and encoder entry points (`generated_code_has_inline_annotations`).
- [x] Verify `#[must_use]` annotations exist on encoder structs, group encoder structs, setters, and `Result`-returning methods (`generated_code_has_must_use_annotations`).
- [x] Verify that the compiler emits unused warnings if an encoder or its return is dropped without calling completion methods.
