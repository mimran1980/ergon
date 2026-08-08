# Composite Layout & Endianness

A common question: *on a little-endian host, can a composite just be a
`#[repr(C)]` / `#[repr(C, packed)]` struct overlaid on the buffer so field
access is a free load?*

**Almost — but not via `repr(C)` transmute.** ergo-sbe does something safer that
is still effectively free on LE hosts:

| Approach | What ergo-sbe does | Why not the other thing |
|----------|--------------------|-------------------------|
| **Wire image** | `#[repr(transparent)] pub struct Engine(pub [u8; 10])` — the value *is* the on-wire bytes | `#[repr(C)]` native fields would insert **alignment padding**; SBE is packed and may have unaligned fields |
| **Accessors** | `u16::from_le_bytes` / `to_le_bytes` at schema offsets | Native loads without endian conversion break **big-endian** schemas and unaligned safety |
| **Flyweight** | `EngineDecoder { buf, pos }` reads in place — **zero copy** | Default decode path for composites |
| **Eager value** | `engine_value()` copies the `N`-byte image once | Still not field-by-field re-pack; `.0` is the wire block |
| **Encode** | Writer copies `engine.0` bulk into the frame | Same image the decoder reads back |

On little-endian hosts, `from_le_bytes` lowers to a plain load (aligned or
unaligned as needed) — so member access is “super fast” **without** casting the
buffer to a padded Rust struct. The generator also emits

```rust,ignore
const _: () = assert!(core::mem::size_of::<Engine>() == 10);
```

so the Rust type size is locked to the wire size at compile time.

`*FixedFields` (e.g. `CarFixedFields`) is a different beast: an **application**
struct with typed fields used to fill the fixed block in one call. It is **not**
a zero-copy overlay of the message buffer; `.fixed(&…)` writes each field with
endian conversion into the flyweight buffer.

#### Conclusion — why not `repr(C, packed)`?

**Single-field access is already one load.** Head-to-head Criterion arms on a
**256-byte** composite (mid-block field `f15`), field-only, no alloc on the timed
path (`layout_access_bench`):

| Arm | What is timed | Median (order of) |
|-----|----------------|-------------------|
| **Flyweight** | `dec.block().f15()` | ~0.4 ns |
| **Wire-image value (preheld)** | `BigBlock([u8; 256]).f15()` | ~0.4 ns |
| **`#[repr(C, packed)]` overlay** | unaligned load of `f15` | ~0.4 ns |
| **Copy then field** | `block_value()` (256 B) then `.f15()` | ~24 ns (~60×) |

So:

1. **Flyweight ≈ preheld wire-image ≈ packed** for one field — all one load on LE.
2. **`repr(C, packed)` does not unlock free access** beyond what
   `[u8; N]` + `from_le_bytes` already gives. Hand-rolling packed overlays is
   extra UB/layout risk for no speed win.
3. **The expensive mistake** is materialising a large composite just to touch one
   field. Prefer flyweight when you only need a few members; use
   `*_value()` when you need the whole wire blob (or pass it around) and pay
   the `N`-byte copy once.
4. We still **do not** generate `repr(C)` / packed field structs: packing +
   unaligned references, big-endian schemas, enums/sets/nested composites. The
   transparent wire image is the portable form that already optimizes to the
   packed load on LE.

#### What about the `zerocopy` crate?

We evaluated using the [`zerocopy`](https://docs.rs/zerocopy) crate to derive
`FromBytes`/`IntoBytes` on generated message structs for zero-copy buffer
overlay. It was not faster.

The flyweight decoder already hits the same `mov` instructions without the
extra dependency.

| You need… | Use |
|-----------|-----|
| One or a few fields on the hot path | **Flyweight** — no composite copy |
| Whole composite as an owned wire blob | **Value** `Engine([u8; N])` / `*_value()` — pay `N` once |
| Hand-rolled `repr(C, packed)` for speed | **Skip it** — same cost as wire-image field access |

Layout contracts:
[`composite_layout_test`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/composite_layout_test.rs).
Decode microbench:
[`layout_access_bench`](https://github.com/mimran1980/ergon/blob/main/sbe/benchmarks/benches/layout_access_bench.rs).

#### Encode — FixedFields vs setters, composite write, LE vs BE

Confirmed by
[`encode_style_bench`](https://github.com/mimran1980/ergon/blob/main/sbe/benchmarks/benches/encode_style_bench.rs)
(Apple M4, LE host; values prebuilt / seeded so LLVM cannot delete the work):

| Comparison | Result |
|------------|--------|
| **`.fixed(&CarFixedFields{…})` vs all setters** | **~equal** (~2.6 ns both) — `.fixed` is the same setter sequence after inlining |
| **Composite `Engine::new` + write vs preheld `engine(e)`** | **~equal** when the rest of the fixed block is also written (10-byte image is noise next to the other stores) |
| **256 B block build+write LE vs BE** | BE ~**5%** slower on LE host (`to_be_bytes` / bswap on 32×`u64`) — 26.1 ns LE vs 27.5 ns BE |
| **Preheld wire image memcpy LE vs BE** | **~equal** (~77 ns) — endian already in `.0`; only bulk copy remains |

So on encode:

1. Prefer **`.fixed`** for clarity / schema completeness — not for speed.
2. Prefer a **prebuilt composite wire image** on the hot path when you can; for small `N` the win is tiny next to other field stores.
3. **LE body on LE host** is free endian; **BE body** costs a bswap per multi-byte field when *building* the image. Once the image exists, write cost matches LE.
