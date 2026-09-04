# Decoder Lanes

A decoder for a message with groups or variable-data exposes **four lanes**
(a fixed-block message has [exactly one](#fixed-block-messages-have-exactly-one-lane)).
They read the same wire and return the same values; they differ in how order is
enforced and what each dynamic-tail access costs. The standard group `Iterator`
remains for
compatibility and partial traversal. It is not a fifth message-decoding lane:
`Iterator::next()` must learn the next entry position before yielding a
dynamic entry, so it is not the ordered fast path.

| Lane | Entry point | Ordering | Dynamic-tail cost | `Sync` |
|------|-------------|----------|-------------------|--------|
| Random access | `try_decode` / `wrap` getters | Any order | Recalculates preceding offsets | yes |
| Memoized | `decoder.memoized()` | Any order | Walks each boundary at most once | no |
| Staged | `into_*` and `visit_entries` | Compile time | One wire-order pass | yes |
| Mutable ordered | `decoder.ordered()` | Runtime `OutOfOrder` | One wire-order pass plus order checks | yes |

Fixed fields stay random-access in every lane. Groups and variable-data must
be consumed in schema order in the staged and mutable ordered lanes.

## Fixed-block messages have exactly one lane

A message with no repeating groups and no variable-data has no dynamic tail:
every field sits at a compile-time offset inside the block, and the base
decoder reads them all in any order at constant cost. There is nothing to
memoize and nothing to order, so **`memoized()` and `ordered()` are not
generated for those messages at all** — and `AnyMessage` offers only
`into_<name>()` for them. This is not an omission you work around; the base
decoder already is the whole story, and a second name for it would only invite
the question of which one is faster.

The lanes below therefore describe messages that *do* carry groups or
variable-data.

## Choosing a lane

- **Sparse or one-off access** — random access. Smallest decoder, `Sync`, no
  cache to pay for. This is the default and the right answer surprisingly often.
- **Repeated or out-of-order access through the same decoder instance** —
  `.memoized()`.
- **Complete sequential decoding** — `.ordered()` (or the staged lane when you
  want the compiler, not the runtime, to enforce order). Fastest full-message
  path.

Group entry decoders keep a one-shot extent cache in *every* lane: the group
iterator computes an entry's end in order to advance, and the entry's last
var-data accessor reuses it rather than re-reading a length header. That is
internal and needs no configuration.

If you already know which template you want, `AnyMessage` will hand you the
lane directly — `into_car()`, `into_car_memoized()`, `into_car_ordered()` —
instead of making you take the base decoder and convert at the call site. A
fixed-block message only offers `into_<name>()`, for the reason above.

## Why four lanes at all — the sbe-tool comparison

sbe-tool's Rust generator gives you **one** decoder: a `&mut` flyweight
carrying a `limit` cursor. Every group and var-data accessor reads at the
current `limit` and then advances it. That single design has to answer three
different questions at once, and it answers them by trusting the caller:

| Question | sbe-tool's answer | Consequence |
|----------|-------------------|-------------|
| What order may I read tails in? | Whatever order you call them in | Calling `activation_code_decoder()` before iterating `fuelFigures` reads the **group's bytes as a length prefix**. No error — a wrong value, or a panic on a short slice |
| Can I go back and re-read a tail? | No — `limit` only moves forward | Re-reading means re-wrapping the message from the start |
| Can I hold the decoder and read fields later? | Only through `&mut` (or by consuming it — `fuel_figures_decoder(self)` takes the decoder and `parent()` gives it back) | No useful `&`-sharing across helpers: reading a tail mutates the cursor, so two readers cannot hold it at once |

ergon splits those three questions into separate types so each one has a
correct answer rather than a convention:

| ergon lane | Closest sbe-tool spelling | What ergon adds | What it costs |
|-----------|---------------------------|------------------|---------------|
| Random access | *(no equivalent — sbe-tool cannot re-read)* | Order-independent reads from an `&` shared, `Sync` decoder | Each dynamic-tail read re-walks from the block |
| Memoized | *(no equivalent)* | The same, but each boundary is walked at most once | One `usize` per tail, inline; not `Sync` |
| Staged | `_decoder()` + `.parent()` chain | The wrong order is a **compile error**, not wrong bytes | Stage types appear in signatures |
| Mutable ordered | `&mut` flyweight with `limit` | The wrong order is `DecodeError::OutOfOrder`, cursor unchanged and retryable | One runtime ordinal check per tail |

Two things are true in **every** ergon lane and in none of sbe-tool's:

- **A short buffer is a `DecodeError`, not a panic**, once you enter through a
  `try_*` constructor. sbe-tool's accessors index the slice directly.
- **A group's declared `numInGroup` is checked against the bytes that are
  actually there before an entry reaches your code.** For a fixed-stride group
  the whole `count × blockLength` region is proven in bounds up front; for a
  dynamic-stride group each entry's minimum extent is proven before that entry
  is handed over. Either way a truncated frame is a `DecodeError`. sbe-tool's
  `advance()` trusts the count and the read panics part-way through iteration.

The trade is real and worth stating plainly: sbe-tool's single flyweight is
less to learn. If your code always decodes complete messages in wire order and
never re-reads, the mutable ordered lane is the like-for-like port and the
other three are choices you can ignore.

## Random access

Simplest for sparse or genuinely out-of-order reads. You can ask for
`manufacturer` before walking `fuelFigures`. Every dynamic-tail getter
re-walks from the fixed block, so the decoder holds nothing but the buffer,
offset, and acting header values. Construction and fixed-field reads are
constant-time.

**Advantages**

- Any-order access; no stage types to thread through the call site
- Natural for "read two fields and stop"
- Smallest decoder, and `Sync` — shareable across threads
- Nothing is paid for a cache you might not use

**Disadvantages**

- Nothing stops you from reading tails twice or skipping a required walk
- Full-message decode of nested groups is the slowest of the lanes
- Reading the same tail twice walks it twice

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_random_access}}
```

## Memoized (`decoder.memoized()`)

Same getter names as random access, with a progressive cache of discovered
dynamic-tail ends. The first access to a tail walks forward from the cache
frontier; later accesses — in any order — reuse what was already discovered.

```rust,ignore
let decoder = CarDecoder::try_from(bytes)?;   // small, Sync, recalculates tails
let decoder = decoder.memoized();             // lazy cache, no allocation

read_header(&decoder)?;
read_groups(&decoder)?;
read_final_tail(&decoder)?;                   // reuses discovered boundaries
```

Construction is O(1) and allocates nothing. Decoded values and wire bytes are
identical to the base lane.

**Build it once and pass `&CarMemoizedDecoder` around.** Calling `.memoized()`
separately inside every function creates a *separate empty cache* each time and
re-walks everything — the opposite of what you wanted. The cache uses `Cell`,
so the wrapper is `Send` but not `Sync`: one instance per thread over shareable
immutable bytes. `into_inner()` hands the base decoder back.

**Use it when**

- You read tails out of order, or read the same tail more than once
- A view jumps to the last var-data field and then back to a group
- One decoder is read by several helpers in the same thread

**Do not use it when**

- You make a single cold pass in wire order. Each tail already begins where the
  last one ended, so there is nothing to reuse — you pay for the cache and get
  nothing back. A cold jump to the last tail is *slower* than random access,
  because it publishes every boundary it skips past.
- You need `Sync`.

`just bench-diagnostics` runs `versioned_l3_bench`, whose `vl3/lane` group
measures exactly these shapes — cold single tail, construct-plus-fixed, one
full traversal, and repeated root re-reads — in both LTO profiles.

Ordered and staged decoders are **not** memoized: they already carry their
current offset and never re-walk an earlier tail, so a cache would be pure
overhead.

## Staged (`into_*` / `visit_entries`)

Maximum safety and the expected maximum-performance sequential path.
Ownership and generated stage types make a later tail unreachable until the
current one is consumed.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_decode_stages}}
```

Each `into_*_as_str()` returns `(&'a str, NextStage<'a>)` — the `&str` borrows
from the original wire buffer, not from the consumed stage. All three strings
remain valid simultaneously while the stage chain advances.

*(This code comes from the `sbe-feature-tour` sample crate.)*

### `#[must_use]` on stages

Consuming stages (`CarDecoderAfterFuelFigures`, `…AfterManufacturer`,
`CarDecoderComplete`, …) are `#[must_use]`. Dropping a stage without
`into_*` / `finish` / `skip_remaining` **silently skips** remaining wire
tails (groups and var-data). That is easy to miss when a function returns
early — prefer advancing until `Complete` or an explicit skip.

### `finish` vs `skip_remaining`

| Method | Meaning |
|--------|---------|
| `finish()` | Advance past any **remaining entries** of the current group and hand back the next named stage (or complete). |
| `skip_remaining()` | Explicit sequential spelling of the same idea — “I am done with this group; jump to the next tail.” |

Use `skip_remaining` when you want the intent obvious in review; both move the
tail cursor in wire order.

### Ordered one-pass `visit_entries`

The group decoder keeps the next message stage until the group is fully
consumed. `remaining_entries()` / `is_empty()` are O(1) observers of the
wire-declared count (`into_*` already read `numInGroup`). `visit_entries`
walks every remaining entry once and returns the next parent stage:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_visit_entries}}
```

Dynamic-entry callbacks return the generated completion stage so the next
cursor comes from the walk, not from a pre-scan of `encoded_length()`.
Fixed-stride callbacks return `Result<(), E>`. Empty groups invoke the
callback zero times.

**Advantages**

- Wrong order is a compile error (missing method on this stage)
- One pass; no offset rescan
- Expected fastest sequential decode; maintained benches require it ≤ sbe-tool
  and ≤ iterator decode

**Disadvantages**

- Stage types appear in signatures; you cannot hold “the decoder” and pick
  tails later
- Skipping a tail still requires an explicit `finish` / `skip_remaining`
- Partial group walks use the `Iterator`, which is not the one-pass path

## Mutable ordered (`ordered()`)

The ergonomic sequential choice: one `&mut` cursor, schema-order tails, runtime
`OutOfOrder` if you jump ahead. Fixed fields stay random-access. Group methods
return a guard that borrows the parent until `visit_entries`, `finish`, or
`skip_remaining` consumes it.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_mutable_ordered}}
```

A wrong dynamic-field call returns `DecodeError::OutOfOrder { owner, expected, requested }`
and leaves the cursor unchanged, so the correct method can still be called.
Calling an already-consumed field reports the next expected name; after
completion `expected` is `"<complete>"`. `finish(self)` skips any unconsumed
suffix and returns the existing complete stage.

Group guards own a local cursor and commit the parent offset only after a
successful completion. Dropping the guard, malformed data, or a callback error
leaves the parent at the group start (retry from the beginning of that group).
`remaining_entries()` is O(1). The guard does not implement `Iterator`. Nested
group guards borrow their entry, so Rust prevents using the parent entry until
the nested guard completes. Unread suffix of a dynamic entry is skipped once
on successful callback return — you cannot omit an earlier tail and then
request a later one.

**Advantages**

- One mutable value instead of a chain of stage types
- Same one-pass walk as staged; order mistakes are `Result` errors, not
  silent rescans
- Dropped guards are retryable; operations are transactional (commit after
  success, including UTF-8 / nested-message validation)

**Disadvantages**

- Order is checked at runtime, not by the type system
- A live group guard borrows the parent, so you cannot interleave parent
  access until the guard is consumed
- Slightly more work than staged (predictable ordinal checks)

## Full-frame bytes mid-walk

| Need | API |
|------|-----|
| Full frame after finishing the walk | complete stage `as_bytes_with_header()` |
| Full frame without consuming stages | inherent `dec.as_bytes_with_header()?` (rescans tails) |
| Fixed block only (not a full frame) | `dec.get_metadata().as_fixed_region_with_header()?` |

See [Generated code](generated-code.md#metadata-limits-tailed-messages) for the
metadata `limit` vs full-frame table.
