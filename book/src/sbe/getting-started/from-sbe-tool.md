# Coming from sbe-tool

Side-by-side mapping for teams migrating from the official Simple Binary
Encoding Rust generator (`sbe-tool`). Within the [published SBE profile]
(../design-notes/feature-matrix.md) and
[`docs/SBE_COMPATIBILITY.md`](https://github.com/mimran1980/ergon/blob/main/docs/SBE_COMPATIBILITY.md),
ergo-sbe aims for official-SBE wire fidelity with sbe-tool; the **API shape**
is intentionally different. Do not read this as unqualified “binary
compatible with every SBE feature.”

## The #1 trap: wrap offset

| | sbe-tool Rust | ergo-sbe |
|---|---------------|----------|
| `wrap` argument | **Body** offset (often `8` for a frame at 0) | **Message start** (often `0`) |
| Where fields live | `body_offset + field_offset` | `message_offset + HEADER_LENGTH + field_offset` |

```text
// Frame at buf[0..]:
// sbe-tool:  enc.wrap(buf, 8)              // body starts at 8
// ergo-sbe:  Enc::wrap(buf, 0)             // message starts at 0
//            Enc::wrap_and_apply_header(buf, 0)
```

Passing sbe-tool’s `8` into ergo-sbe for a frame at zero **mis-aligns every
field**. Generated rustdoc on `wrap` / `wrap_and_apply_header` / `decode`
repeats this callout.

## Header modes (fair comparison table)

Use the **same** logical work on both sides when comparing or porting:

| Mode | ergo-sbe | sbe-tool |
|------|----------|----------|
| **Body only** | `wrap(buf, 0)` + setters — no apply-header | `wrap(buf, 8)` + setters — no `.header(0)` |
| **Header + body** | `wrap_and_apply_header(buf, 0)` + setters | `wrap(buf, 8)` then `header(0).parent()` **then** setters |
| **Header only** | `wrap_and_apply_header` alone | `wrap(buf, 8).header(0)` alone |

- ergon `wrap` = message start; sbe-tool `wrap` = body offset.
- sbe-tool `encoded_length()` is **body only**. ergon
  `encoded_length_with_header()` **includes** the header. Never invent
  `8 + encoded_length()` to “prove” a header was written if `.header(0)` was
  not called on the sbe-tool arm.

Full fairness rules for benchmarks: [Benchmarks methodology](../benchmarks/methodology.md).

## Groups: `.parent()` hopscotch vs closures

| sbe-tool | ergo-sbe |
|----------|----------|
| Open group flyweight, fill entries, `.parent()` back | `enc.bids(n, \|bids\| { bids.add(\|e\| { … })?; Ok(()) })?` |
| Nested groups fight the borrow checker | Nested closures end; chain continues in wire order |

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:encode_sample_car}}
```

Compile-time order: you cannot call `asks` before `bids` — the stage type
has no `asks` method. See
[Wire order via named stages](../core-concepts/wire-order-stages.md).

## Length APIs

| Concept | sbe-tool | ergo-sbe |
|---------|----------|----------|
| Body length after encode | `encoded_length()` | body region only via stage `encoded_length` where exposed |
| Header + body | compute yourself (`8 + …`) | `encoded_length_with_header()` / `as_bytes_with_header()` |
| Pre-size buffer | often oversize scratch | **Exact:** `Encoder::compute_length()` / staged `*EncodedLength` builder; stack `[0u8; N]` when `N` is const |

Do **not** default to `vec![0u8; 4096]` or `Vec::with_capacity(MAX)` then
truncate. See [Buffer sizing](../core-concepts/buffer-sizing.md) and
[Exact sizing](../feature-tour/exact-sizing.md).

## Decode entry

Both ecosystems typically wrap decoders at the **body** for direct field
access after the header is known. ergon’s entry points take **message start**
(not sbe-tool’s body offset):

| Need | ergo-sbe |
|------|----------|
| Untrusted / network | `try_decode` / `try_wrap` / `try_from` → `Result` (all failures) |
| Known-good buffer | bare `wrap` → panic if short; bare `decode` → **hybrid** (panic if short, `Err` on wrong template/schema) |
| Proven-tight HFT | `unsafe wrap_unchecked`; `decode_unchecked` = unchecked extent + checked identity |

See [Trust Boundary](../core-concepts/trust-boundary.md).

## Version handling

Decoders are version-aware: tail offsets use the **wire** acting block
length, not only the compiled block length. Optional / `sinceVersion` fields
follow schema presence rules. Prefer explicit `try_*` entry points when
reading mixed-version streams.

## What has no direct ergon equivalent (and why)

| sbe-tool habit | ergo-sbe |
|----------------|----------|
| `.parent()` ownership hop | Closures + consuming stage returns |
| Generic `Encoder<State>` spelling | Named stage structs + `H: HeaderState` only for header mode ([type-state note](../design-notes/type-state.md)) |
| `encoded_length()` as full-frame size | Use `*_with_header` when you need the frame |
| Always-on meta / Display noise | Opt-out size knobs: `with_display_debug(false)`, `with_meta_attributes(false)`, `with_dispatch(false)` |

## Trust boundary

`try_*` returns `Result` on short buffers (and identity mismatches). Bare
`wrap` panics if short. Bare `decode` is a **hybrid**: panics if short, but
still returns `Err` on wrong template/schema. `decode_unchecked` is unchecked
extent + checked identity. After a safe constructor succeeds, fixed-field
accessors are branch-free. Full detail:
[Trust Boundary](../core-concepts/trust-boundary.md).

## Further reading

- [Type-state is zero-cost](../design-notes/type-state.md)
- [API freeze decisions](../design-notes/api-freeze.md)
- [Feature matrix](../design-notes/feature-matrix.md)
- Sample: [sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
