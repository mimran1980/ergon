# Verified frame proof and mode-typed decoders

**Blocked by:** `69-buffer-verify-function`, `03-group-vardata-wire-parity`, `125-schema-parser-aeron-parity`
**Severity:** HIGH
**Status: DONE (Phase 2 gate close)**


## Problem

`verify(buf) -> Result<(), VerifyError>` is useful, but it throws away the proof
that the buffer was structurally checked. Feed handlers then decode the same
buffer through the normal checked path, repeating structural work or relying on
manual convention.

Rust can carry the proof as a type. Generated verification should optionally
return a proof token that is the only safe way to construct a trusted decoder
mode.

## Design

Generate checked and verified modes:

```rust
pub enum Checked {}
pub enum Verified {}

pub struct CarDecoder<'a, Mode = Checked> {
    buf: &'a [u8],
    offset: usize,
    // mode marker is private/sealed
}

pub struct VerifiedFrame<'a, M: SbeMessage> {
    buf: &'a [u8],
    range: core::ops::Range<usize>,
    _marker: core::marker::PhantomData<M>,
}
```

The public API shape can evolve, but these invariants should not:

- normal users get `CarDecoder<'a, Checked>` from `try_from`/`wrap`
- generated `verify_frame` is the only safe constructor for `VerifiedFrame`
- `VerifiedFrame<'a, Car>` can produce `CarDecoder<'a, Verified>`
- `Verified` mode may skip repeated structural scans and some internal bounds
  checks only where the verifier has proven the full extent
- all unsafe fast paths remain inside generated code and are justified by the
  proof token

## Acceptance criteria

- [x] `CarDecoder::verify_frame(buf, off, frame_len)` returns
      `Result<VerifiedFrame<'a, Car>, VerifyError>`
- [x] `VerifiedFrame<'a, Car>::decoder()` returns a verified-mode decoder without
      rereading the message header unnecessarily
- [x] Checked-mode decoder APIs remain source-compatible for existing users
- [x] Verified-mode constructors are sealed/private except through successful
      generated verification
- [x] Verified mode never trusts a buffer whose group or var-data extent was not
      proven by the verifier
- [x] Tests cover valid, truncated, bad group-count, and bad var-data-length
      frames
- [x] Test proves checked and verified decoders return identical values for the
      same valid fixture
- [x] Compile-fail test proves user code cannot construct `VerifiedFrame`
      directly
- [x] Benchmark compares checked decode, `verify + checked decode`, and
      `verify_frame + verified decode` on market-data-shaped messages
- [x] Miette/verify errors still report field name, offset, needed length, and
      available length

Ref: `design/DECISIONS.md` §3 and trap 12; todo 69 is the baseline verifier.
