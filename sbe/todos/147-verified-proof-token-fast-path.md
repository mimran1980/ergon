# Verified proof token fast path

**Blocked by:** 69-buffer-verify-function, 145-group-entry-wire-blocklength-bug
**Severity:** HIGH
**Status: DESIGN / ROADMAP**


## Problem

`verify(buf) -> Result<(), VerifyError>` proves structural validity and then
throws the proof away. Feed handlers that validate frames still decode through
checked paths that may repeat extent work.

Stable Rust can carry the proof in a sealed token so valid frames get a faster,
auditable path without exposing unsound constructors.

## Design target

- Generated verification returns `VerifiedFrame<'a, M>` for a specific message
  type or schema dispatch result.
- `VerifiedFrame` is sealed: user code cannot construct or forge it directly.
- A verified frame can produce `Decoder<'a, Verified>`.
- Verified decoders may skip only checks covered by the proof.
- Checked decoders remain the default source-compatible path.

## Acceptance criteria

- [ ] Valid fixed-only, group, and var-data messages can produce verified
      frames.
- [ ] Truncated headers, bad schema IDs, bad group extents, and bad var-data
      lengths fail before a verified decoder exists.
- [ ] Checked and verified decoders return identical values for valid fixtures.
- [ ] Compile-fail test proves users cannot construct `VerifiedFrame` or
      `Verified` mode directly.
- [ ] Benchmark compares checked decode, `verify + checked decode`, and
      `verify_frame + verified decode`.
- [ ] Public docs describe exactly which checks verified mode is allowed to
      skip.
