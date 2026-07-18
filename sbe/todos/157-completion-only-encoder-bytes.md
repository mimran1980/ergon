# Completion-only encoder byte views

**Blocked by:** `03-group-vardata-wire-parity`
**Severity:** HIGH
**Status: DONE (implemented per remediation plan Task 4; verified 2026-07-18)** — infallible `as_bytes(&self) -> &'a [u8]` / `encoded_length() -> usize` exist only on complete encoder stages; partial/decoder stages return `Result<usize, DecodeError>` / `Result<&'a [u8], _>` (explicitly partial). Confirmed in the generated Car codec (complete stages at e.g. `as_bytes` returning `&'a [u8]`; incomplete at `encoded_length() -> Result<...>`).

## Problem

The canonical design allows complete-message `as_bytes()`,
`as_bytes_with_header()`, `encoded_length()`, and `AsRef<[u8]>` only on a
terminal complete encoder stage. Current code generation still emits an
initial/incomplete method named `as_bytes()` for partial scalar inspection.
That name can make an incomplete message look publishable.

The 2026-07-11 ledger statement that the encoder API was fully compliant did
not detect this remaining generated method and is superseded for this point.

## Acceptance criteria

- [ ] Add a source-shape test that fails while an incomplete encoder stage
      exposes `as_bytes`, `as_bytes_with_header`, `encoded_length`, or
      `AsRef<[u8]>`.
- [ ] Remove the partial view if no maintained measured workflow needs it.
- [ ] If partial inspection is demonstrably required, name it
      `written_prefix()` or `partial_bytes()` and document that it is not a
      complete SBE message.
- [ ] Keep complete header-inclusive views only on terminal complete stages.
- [ ] Compile-fail calling every complete-message view on incomplete outer and
      nested encoders.
- [ ] Preserve exact official-SBE bytes and generated-code stability.
- [ ] Prove no allocation and no regression against the previous ErgoSBE and
      comparable Aeron success paths using the canonical five-run gate.
- [ ] Update guides and ledgers only after generated source and tests prove the
      final interface.
