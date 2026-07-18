# Fix systematic `needed` miscalculation in BufferTooShort

**Blocked by:** none

**Status: DONE (2026-07-19)** — decoder BufferTooShort paths fixed earlier;
encoder `wrap` / `wrap_and_apply_header` return `Result<_, EncodeError>` on
short buffers (golden `CarEncoder::wrap` → `Result`). Verify-and-close audit
2026-07-19.

## Historical notes

Decoder field templates previously mis-reported `needed` as position+size;
those decoder cases were closed before this stamp. Encoder wrap is fallible
in current codegen.
