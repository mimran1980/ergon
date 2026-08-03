# Trust Boundary

Every SBE buffer crossing a process boundary must be validated. ergo-sbe
provides a **three-tier** constructor API, ordered from safest to fastest:

| Tier | Prefix | Behaviour on bad buffer | Use case |
|------|--------|------------------------|----------|
| **Checked** | `try_{wrap,decode,…}` | Returns `Result::Err` | Untrusted input, process boundaries |
| **Trusted** | bare name (`wrap`, `decode`, …) | **Panics** (slice index), safe | Known-good buffers, benchmarks |
| **Unchecked** | `unsafe fn *_unchecked` | **UB** (raw pointer ops) | Proven-tight HFT loops |

**The trusted tier (bare name) uses safe Rust primitives.** `wrap_and_apply_header`
writes the header with `copy_from_slice`; `decode` reads the header with
`read_bytes`. Both are safe — a mis-sized buffer causes a **panic**, not
undefined behaviour. LLVM elides the bounds check for fixed-size stack buffers,
so the runtime cost is zero in practice.

The `unsafe fn *_unchecked` variants exist for the extreme case where the panic
machinery is measurable on the critical path and the caller has proven the
buffer layout independently. They use the same raw-pointer operations that the
trusted tier replaced.

## Encoder entry points

| Entry | Return | Behaviour |
|-------|--------|-----------|
| `Encoder::try_wrap(buf, offset)` | `Result<Encoder, EncodeError>` | Validates capacity, returns `Err` |
| `Encoder::wrap(buf, offset)` | `Encoder` (body-only) | Panics on OOB setter access |
| `Encoder::try_wrap_and_apply_header(buf, pos)` | `Result<Encoder, EncodeError>` | Validates capacity + writes header, returns `Err` |
| `Encoder::wrap_and_apply_header(buf, pos)` | `Encoder` (header written) | Panics on OOB header write |
| `unsafe fn Encoder::wrap_unchecked(buf, offset)` | `Encoder` (body-only) | UB on OOB — raw pointer setters |
| `unsafe fn Encoder::wrap_and_apply_header_unchecked(buf, pos)` | `Encoder` (header written) | UB on OOB — `copy_nonoverlapping` header |

## Decoder entry points

| Entry | Return | Behaviour |
|-------|--------|-----------|
| `Decoder::try_wrap(buf, offset, bl, ver)` | `Result<Decoder, DecodeError>` | Validates body extent, returns `Err` |
| `Decoder::try_decode(buf, pos)` | `Result<Decoder, DecodeError>` | Header + template/schema + version-aware fixed extent |
| `Decoder::wrap(buf, offset, bl, ver)` | `Decoder` | Panics on OOB accessor access |
| `Decoder::decode(buf, pos)` | `Result<Decoder, DecodeError>` | Validates header identity, panics on OOB header read |
| `unsafe fn Decoder::wrap_unchecked(buf, offset, bl, ver)` | `Decoder` | UB on OOB — raw pointer accessors |
| `unsafe fn Decoder::decode_unchecked(buf, pos)` | `Result<Decoder, DecodeError>` | Validates header identity, UB on OOB header read (`read_bytes_unchecked`) |

## Migration from 0.1.10

| 0.1.10 | 0.1.12 |
|--------|--------|
| `wrap(buf, pos)` returning `Result` | `try_wrap(buf, pos)` — same semantics |
| `wrap_and_apply_header(buf, pos)` returning `Result` | `try_wrap_and_apply_header(buf, pos)` — same semantics |
| `try_decode(buf, pos)` | `try_decode(buf, pos)` — unchanged |
| `wrap(buf, pos)` infallible (was `unsafe`) | `wrap(buf, pos)` infallible (now safe, panics on OOB) |
| N/A | `unsafe fn wrap_unchecked(buf, pos)` — original raw-pointer behaviour |

**The short name `wrap` / `wrap_and_apply_header` no longer returns `Result`.**
If your code relied on `wrap` returning `Result`, change it to `try_wrap`. If
your code used `wrap` for performance (skipping validation), it continues to
work — and is now safe because the inner operations panic on OOB instead of
invoking UB.

See [Trust boundaries (feature tour)](../feature-tour/trust-boundaries.md) for
worked examples.
