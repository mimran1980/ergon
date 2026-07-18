# Encoder `wrap()` for body-only encoding

**Status: DONE (2026-07-19)** — golden `CarEncoder::wrap(buf, pos) -> Result<Self, EncodeError>`
and `wrap_and_apply_header -> Result`. Short-buffer returns `EncodeError`.
Optional nullification remains explicit `apply_nulls()`.

Verify-and-close audit 2026-07-19 against `sbe/tests/golden/car_example.rs`.
