# Composite Layout & Endianness

On a little-endian host, can a composite be a `#[repr(C)]` / `#[repr(C, packed)]`
overlay so field access is a free load?

**Almost — but not via `repr(C)` transmute.** ergo-sbe does something safer that
is still a plain load on LE:

| Approach | What ergo-sbe does | Why not the other thing |
|----------|--------------------|-------------------------|
| **Wire image** | `#[repr(transparent)] pub struct Engine(pub [u8; 10])` — the value *is* the on-wire bytes | `#[repr(C)]` native fields insert **alignment padding**; SBE is packed and may be unaligned |
| **Accessors** | `from_le_bytes` / `to_le_bytes` at schema offsets | Native loads break **big-endian** schemas and unaligned safety |
| **Flyweight** | `EngineDecoder { buf, pos }` reads in place | Default decode path |
| **Eager value** | `engine_value()` copies the `N`-byte image once | Still not field-by-field re-pack; `.0` is the wire block |
| **Encode** | Writer copies `engine.0` into the frame | Same image the decoder reads back |

On LE hosts, `from_le_bytes` lowers to a load. The generator also emits

```rust,ignore
const _: () = assert!(core::mem::size_of::<Engine>() == 10);
```

so the Rust type size is locked to the wire size.

`*FixedFields` is an **application** struct used to fill the fixed block in one
call. It is **not** a zero-copy overlay; `.fixed(&…)` writes each field with
endian conversion.

**Single-field access is already one load.** `layout_access_bench` compares
flyweight, preheld `[u8; N]`, and `#[repr(C, packed)]` on a mid-block field:
they measure equivalently. Materialising the whole composite first is the
expensive path. `zerocopy` `FromBytes`/`IntoBytes` was evaluated and was not
faster than the flyweight `mov`.

| You need… | Use |
|-----------|-----|
| One or a few fields on the hot path | **Flyweight** |
| Whole composite as an owned wire blob | **Value** `Engine([u8; N])` / `*_value()` — pay `N` once |
| Hand-rolled `repr(C, packed)` for speed | **Skip it** — same cost, extra UB/layout risk |

We do **not** generate packed field structs: packing, unaligned references,
big-endian schemas, enums/sets/nested composites. The transparent wire image
is the portable form that already optimizes to the packed load on LE.

On encode, `encode_style_bench` shows `.fixed(&CarFixedFields{…})` and
individual setters measure equivalently after inlining. Prefer `.fixed` for
completeness, not speed. Building a BE image on an LE host costs a bswap per
multi-byte field; once the image exists, write cost matches LE.

[`composite_layout_test`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/composite_layout_test.rs) ·
[`layout_access_bench`](https://github.com/mimran1980/ergon/blob/main/sbe/benchmarks/benches/layout_access_bench.rs) ·
[`encode_style_bench`](https://github.com/mimran1980/ergon/blob/main/sbe/benchmarks/benches/encode_style_bench.rs).
Read live Criterion output; do not quote numbers from this page.
