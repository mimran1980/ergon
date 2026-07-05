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

## Acceptance criteria

- [ ] `#[inline]` on: every decoder field accessor (checked + unchecked + raw)
- [ ] `#[inline]` on: every encoder setter method
- [ ] `#[inline]` on: `wrap`, `wrap_and_apply_header`, `new` constructors
- [ ] `#[inline]` on: group `next()`, `len()`, `is_empty()` methods
- [ ] `#[must_use]` on: every `Result`-returning method
- [ ] `#[must_use = "encoder must be consumed to write the message"]` on encoder type-state types
- [ ] `encoded_length()` on group decoders returns `Result` instead of `unwrap_or` fallback
- [ ] `#[cold]` on error-construction paths (already tracked in 08, verify)
- [ ] `#[inline(always)]` only on the hottest one-liners (raw accessors, unchecked variants)
- [ ] Benchmark: verify no regression from added attributes (should be zero-cost or improvement)

Ref: `design/DECISIONS.md` §10 codegen rules. `simple-binary-encoding/sbe-tool/src/main/java/uk/co/real_logic/sbe/generation/rust/RustGenerator.java`.
