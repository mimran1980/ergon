# Decoder Lanes

A decoder with groups or var-data has **two jobs**: read some fields in any
order, or walk the whole message once in schema order. A group decoder from a
random-access getter implements `Iterator` for partial traversal; it is not a
third message-decoding lane.

Encoding is one state machine: each stage **is** the cursor, so the next legal
field is the only method on that type. See
[Encode and Decode](../getting-started/encode-decode.md) and
[Method Chaining](../getting-started/method-chaining.md). Decoding is the
choice, because a decoder can be asked to read in an order the encoder never
had.

| Lane | Entry | Order | Dynamic-tail cost | `Sync` |
|------|-------|-------|-------------------|--------|
| Random access | `try_decode` / `wrap` getters | Any | Recalculates preceding offsets | yes |
| Staged | `into_*` / `skip_*` | Compile time | One forward traversal | yes |
| Ordered | `decoder.ordered()` | Compile time | Delegates to staged | yes |
| Memoized | `decoder.memoized()` | Any | Cache of discovered boundaries | no (`Send`) |

Fixed fields stay random-access in all of them. Ordered also offers a `fixed`
callback (fixed-fields-only view, not the full decoder). A message with no
groups and no var-data has **one** lane: the base decoder. `memoized()` and
`ordered()` are not generated; `AnyMessage` offers only `into_<name>()`.

| If you want… | Use |
|---|---|
| Some fields, any order, share `&Decoder` | Random access (default) |
| Whole message in wire order | Staged `into_*` / `skip_*` |
| Whole message, one callback per tail | Ordered (`decoder.ordered()`) |
| Same decoder, many helpers, mixed tail order | Memoized — benchmark it; not the default |

Worked nested example:
[L3 order book](../../samples/l3-book.md#decoding-random-access-or-one-staged-chain).
`AnyMessage` can land in a lane directly (`into_car()`, `into_car_memoized()`).

sbe-tool has **one** `&mut` flyweight with a `limit` cursor. Wrong order reads
the next group's bytes as a length prefix; re-read means re-wrap; no useful
`&`-sharing. Ergon splits those jobs. Full mapping:
[Coming from sbe-tool](../getting-started/from-sbe-tool.md).

In every ergon lane, `try_*` construction makes a short buffer a `DecodeError`
(not a panic), and a group's `numInGroup` is checked against the bytes that
are actually there before an entry reaches you.

## Random access

Any-order getters. The decoder holds the buffer, offset, and acting header
values — nothing else. Each dynamic-tail getter re-walks from the fixed block.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_random_access}}
```

## Memoized (`decoder.memoized()`)

Same getter names. First access to a tail walks forward from the cache
frontier; later accesses reuse it. Construction is O(1) and allocates nothing.
Decoded values match the base lane, including schema max-length rejection:
warming the cache with `encoded_length()` does not accept a payload the base
getter would refuse.

```rust,ignore
let decoder = CarDecoder::try_from(bytes)?;   // small, Sync, recalculates tails
let decoder = decoder.memoized();             // lazy cache, no allocation

read_header(&decoder)?;
read_groups(&decoder)?;
read_final_tail(&decoder)?;                   // reuses discovered boundaries
```

Build it **once** and pass `&CarMemoizedDecoder`. Calling `.memoized()` inside
every function creates a separate empty cache. `Cell` interior mutability:
`Send` but not `Sync`. `into_inner()` returns the base decoder.

The base lane's `tail_offset_k` re-walks every preceding tail on **every**
getter — quadratic even in schema order. Memoized makes that sweep linear.
Staged already carries the cursor, so it needs no cache.

Use it when several helpers read the same decoder, or when you jump to a late
tail and come back. Skip it for one tail and stop, a full wire-order walk
(use staged), or if you need `Sync`.

The cache covers **message-level** tails only. Nested groups inside an entry
use an ordinary entry decoder. `just bench-diagnostics` → `versioned_l3_bench`.

## Staged (`into_*` / `skip_*`)

Sequential decode, the encoder dual. Each `into_*` consumes the stage. Wrong
order is a missing method.

| Group entries | `into_<group>` | Why |
|---|---|---|
| Own groups or var-data | visit closure returning the entry's completion | No stride — the completion *is* the next offset |
| Fixed-stride (no tails) | iterator | Next entry is `offset + acting block length` |

`Iterator` is implemented for `&mut Iter` only — `for entry in iter` does not
compile, so a loop cannot drop the rest of the message. Also
`ExactSizeIterator` and `FusedIterator`.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_decode_stages}}
```

Each `into_*_as_str()` returns `(&'a str, NextStage<'a>)` — the `&str` borrows
the original buffer. `try_<data>(|bytes| ...)` returns the next stage without
a tuple binding. See [Method chaining](../getting-started/method-chaining.md#decoding).

Start with `try_decode` / `try_from`. Do not `verify` first then walk — that
is two structural passes. Inside a dynamic-entry callback, consume nested
tails with `into_*` / `try_*` / `skip_*` / `ordered()` and return that
completion. Random-access getters or a root `manufacturer_len()` before
visiting preceding groups rescan.

Consuming stages and group iterators are `#[must_use]`. Dropping a stage
without `into_*` / `skip_*` / `finish()` abandons remaining tails.

A fixed-stride iterator need not be drained: `finish()`, or the next tail's
`into_*` / `skip_*` on the iterator, skips unread entries as
`count × block_length`. Dynamic-stride `skip_*` still walks each unread entry.

### Asking before walking

| Accessor | Answers | Where |
|----------|---------|-------|
| `<group>_count()` | `Result<usize, DecodeError>` | base, `.memoized()`, stage before the group, nested entries |
| `<field>_len()` | var-data byte length, same `Result` | same four places |
| `iter.remaining_entries()` | unread entries (`usize`) | the group iterator |

Absent versioned tails are `0`. If a primary getter already uses that name,
the convenience accessor is omitted on that owner. Non-consuming is not
constant-time on random-access owners — lookahead may scan preceding tails.
Group **byte** length is not available from the count when entries have tails.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_visit_entries}}
```

Prefer staged when you skip tails, hold stages across functions, or want
`for entry in &mut iter`. A closure error consumes the stage; there is no
continuation to retry.

## Ordered (`decoder.ordered()`)

One callback per tail, same spelling at message and nested-entry level, plus
`EntryInfo` per entry. Generated on message decoders with tails and on group
entries that carry tails. An entry whose existing accessor is named `ordered`
keeps that accessor and does not get this lane.

`ordered()` is `sbe_rt::Ordered<S>` over the staged stage — no second cursor.
A nested `ordered()` walk **is** the completion the parent callback owes.
`.done()` only unwraps the staged complete. The message chain ends on
`encoded_length_with_header()` / `as_bytes_with_header()`, like encode.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_ordered_lane}}
```

| Tail | Callback |
|------|----------|
| Fixed block | `fixed(\|&{Name}DecoderFixedView\|)`; `try_fixed` for a custom `E` |
| Group, entries with tails | `group(\|entry, EntryInfo\|)` → entry completion, or `Ordered<completion>` from nested `ordered()` |
| Group, fixed-stride | `group(\|entry, EntryInfo\|)` → `()` |
| Var-data | `field(\|&[u8]\|)`, and `field_as_str(\|&str\|)` when the schema declares text encoding |

`EntryInfo` is `index`, wire-declared `count`, acting `block_length`, plus
`is_first()` / `is_last()` / `remaining()`. No entry scan. Group **byte**
length is absent: entries with tails only settle it by traversing.

Message-level `fixed` is optional. Skip it, or call it and lose the view.
The view has no group/var-data accessors. A first tail named `fixed` or
`tryFixed` suppresses the callback — read those fields before `ordered()`.
Entry-level wrappers have no `fixed`: the parent already holds the entry.

Empty / version-absent groups invoke the callback zero times;
`<group>_count()` on the ordered stage still reports the declared count.

Bytes and text callbacks borrow `'a` from the original buffer. A callback
error consumes the stage. Prefer ordered when you always decode whole
messages and want one shape. Prefer staged to skip, hold stages, or iterate.

## Full-frame bytes mid-walk

| Need | API |
|------|-----|
| Full frame after the walk | complete stage `as_bytes_with_header()` |
| Full frame without consuming stages | inherent `dec.as_bytes_with_header()?` (rescans tails) |
| Fixed block only | `dec.get_metadata().as_fixed_region_with_header()?` |

See [Generated code](generated-code.md#metadata-limits-tailed-messages).
