# Migration notes (0.1.x constructor surface)

This page is a short pointer. The living story is the book:

- [Trust Boundary](../book/src/sbe/core-concepts/trust-boundary.md)
- [Coming from sbe-tool](../book/src/sbe/getting-started/from-sbe-tool.md)
- [SBE compatibility](SBE_COMPATIBILITY.md)

## Name map (0.1.10 → 0.1.12+)

| Era | Checked (`Result`) | Trusted (panic on short) | Unchecked (`unsafe`) |
|-----|--------------------|--------------------------|----------------------|
| 0.1.10 | unsuffixed `wrap` / `decode` returned `Result` | n/a | private cores |
| 0.1.12+ | `try_wrap` / `try_wrap_and_apply_header` / `try_decode` | bare `wrap` / `wrap_and_apply_header` / `decode` | `*_unchecked` |

If your code called `wrap` expecting `Result`, rename to `try_wrap`.

## 0.1.13 soundness

Bare constructors prove header + fixed-body extent before returning a flyweight
whose accessors use unchecked loads/stores. Undersized buffers panic (trusted)
or return `Err` (`try_*`); only `*_unchecked` may invoke UB on OOB.
