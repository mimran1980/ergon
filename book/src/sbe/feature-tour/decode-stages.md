# Decoder Lanes

A decoder for a message with groups or variable-data has **two jobs**
(a fixed-block message has [exactly one](#fixed-block-messages-have-exactly-one-lane)):
read some fields in any order, or walk the whole message once in schema
order. Those jobs share the wire and the values; they differ in how order is
enforced and what each dynamic-tail access costs. The standard group
`Iterator` remains for random-access / partial traversal. It is not a
third message-decoding lane: `Iterator::next()` must learn the next entry
position before yielding a dynamic entry, so it is not the sequential fast
path.

## Encoding: one state machine

Encoding is the simple half of this page. It genuinely is a state machine —
there is a cursor, and it does track your position in the message — the
difference from a typical state machine is *where* that cursor lives. Each
encoder stage is a distinct type that **is** the current position: fixed
block written, then each group, then each var-data field, in schema order.
Writing a field consumes that stage and returns the next one, so the cursor
advances by becoming a new type rather than by mutating a field inside it.
The compiler enforces the order simply by only giving the current stage's
type a method for the next legal field. Skip a required tail or write two
fields out of order and the code does not compile, full stop — the state
machine is real, it is just resolved at compile time instead of carried at
runtime.

The cost of that safety is effectively zero: the stage types are concrete
monomorphic structs with no data of their own beyond the buffer and offset
already needed regardless, and the stage transitions disappear entirely under
optimisation. See [Encode and Decode](../getting-started/encode-decode.md) for
the `fixed()` / `raw_fixed()` mechanics and [Method
Chaining](../getting-started/method-chaining.md) for why one chained
expression is the idiom. Decoding is where the real choice lives, because a
decoder can be asked to read in an order the encoder never had to think
about — which is what the rest of this page is about.

| Lane | Entry point | Ordering | Dynamic-tail cost | `Sync` |
|------|-------------|----------|-------------------|--------|
| Random access | `try_decode` / `wrap` getters | Any order | Recalculates preceding offsets | yes |
| Staged | `into_*(|entry|)` / `skip_*` | Compile time | One wire-order pass | yes |

Fixed fields stay random-access in both. Groups and variable-data must be
consumed in schema order on the staged lane. `.memoized()` is still
generated for repeated out-of-order tail reads — it is not a third way to
start a sequential walk.

## Fixed-block messages have exactly one lane

A message with no repeating groups and no variable-data has no dynamic tail:
every field sits at a compile-time offset inside the block, and the base
decoder reads them all in any order at constant cost. There is nothing to
memoize, so **`memoized()` is not generated for those messages at all** —
and `AnyMessage` offers only `into_<name>()` for them. This is not an
omission you work around; the base decoder already is the whole story, and a
second name for it would only invite the question of which one is faster.

The lanes below therefore describe messages that *do* carry groups or
variable-data.

## Choosing a lane

**Random access (the default) is what most code should reach for first.**
It carries no cursor at all — no internal mutable state — so it is just a
`&`-shared reference you can pass around freely: into a function, across a
thread, held by several callers at once. You can read `manufacturer`, then a
group three fields later, then jump back, in any order, and it always returns
the correct value because it re-derives every offset from the start on each
call rather than trusting a remembered position. Unlike sbe-tool, wrong order
is not a silent correctness bug here — there is no "wrong order" to have. The
one real cost is that a deeply nested read (nested groups, var-data inside
entries, that kind of shape) recomputes preceding offsets each time, which is
measurably slower than a lane that remembers where it is. In practice that gap
is small enough that "default, plus benchmark if you're unsure" is the right
starting posture, not a premature switch to something else.

**The staged lane (`into_*(|entry|)` / `skip_*`) is sequential decode, the
same idea as encode.** Each `into_*` consumes the current stage and returns
the next one, so the type system — not a runtime check, and not `&mut` —
makes reading out of order a compile error. Write it as one chain, the way
the encoder is one chain: groups take a visit closure, `skip_*` jumps a
tail you do not need, and you bind only var-data payloads (or the terminal
complete). That ownership transfer is what buys back the performance random
access gives up: each tail is walked exactly once. See
[Staged](#staged-into_--skip_) below — it is the sample crate, not a sketch.

**Memoized (`decoder.memoized()`) is the slowest lane and exists for one
specific shape of problem: the same decoder instance gets handed to several
functions, and each function reads tails in a different order.** Random
access would recompute the same offsets over and over across those calls;
memoized remembers each tail boundary the first time anything reaches it, so
later reads — from any of those functions, in any order — are free. Outside
that shape it is close to pure overhead: reading one tail and stopping never
earns back the cache's bookkeeping, and if you are already reading in wire
order, the staged lane beats it without carrying a cache at all. Do not
reach for this lane by default — benchmark the concrete access pattern
first; see [Memoized](#memoized-decodermemoized) below for the
`versioned_l3_bench` numbers.

**The short version:**

| If you want… | Use |
|---|---|
| Some fields, any order, share `&Decoder` | Random access (the default) |
| The whole message in wire order | Staged `into_*(|entry|)` — one chain, compile-time order |
| The same decoder passed to many functions that each read tails in a different order | Memoized (`decoder.memoized()`) — benchmark it, don't reach for it by default |

For a worked example that puts random-access and staged side by side over one
genuinely nested schema (two repeating groups, each entry carrying its own
nested `orders` group, plus trailing var-data), see the
[L3 order book sample](../../samples/l3-book.md#decoding-random-access-or-one-staged-chain).

Group entry decoders keep a one-shot extent cache in *every* lane: the group
iterator computes an entry's end in order to advance, and the entry's last
var-data accessor reuses it rather than re-reading a length header. That is
internal and needs no configuration.

If you already know which template you want, `AnyMessage` will hand you the
lane directly — `into_car()`, `into_car_memoized()` — instead of making you
take the base decoder and convert at the call site. A fixed-block message
only offers `into_<name>()`, for the reason above.

## Why not one `&mut` cursor — the sbe-tool comparison

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
| Staged | `_decoder()` + `.parent()` chain | The wrong order is a **compile error**, not wrong bytes; one chain, no `&mut` | Stage types appear in signatures if you bind them — don't |

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
less to learn. If your code always decodes complete messages in wire order
and never re-reads, the staged `into_*(|entry|)` chain is the port — it is
the encoder dual, and it does not need to be mutable.

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

### What the cache actually saves

The base lane's `tail_offset_k` is defined in terms of `tail_offset_{k-1}`: it
re-walks every preceding tail on **every** getter, and remembers nothing
between calls. Reading tails `0..n` costs `1 + 2 + … + n` walks — quadratic in
the number of tails — even when you read them in schema order. The memoized
lane makes the same sweep linear, because a boundary already discovered is
never walked again.

That matters because it is easy to assume the opposite. Reading in wire order
does not make the base lane cheap; only a lane that *carries* its cursor —
the staged `into_*(|entry|)` chain — gets that for free.

**Use it when**

- You read more than one or two dynamic tails through the same decoder, in any
  order, including schema order
- You read the same tail more than once
- A view jumps to the last var-data field and then back to a group
- One decoder is read by several helpers in the same thread

**Do not use it when**

- You read one dynamic tail and stop. There is no second access to amortise
  the cache against, and reaching a late tail publishes every boundary it
  passes — so a single cold jump is *slower* than the base lane.
- You are decoding the whole message in wire order anyway. Use the staged
  lane: it carries the cursor without a cache and is faster still.
- You need `Sync`.

`just bench-diagnostics` runs `versioned_l3_bench`, whose `vl3/lane` group
measures exactly these shapes — cold single tail, construct-plus-fixed, one
full traversal, and repeated root re-reads — in both LTO profiles. Measure
there rather than assuming; the crossover depends on your tail count.

### Root tails only

The cache covers the message's own groups and var-data. Reaching *into* a
group — a nested group inside an entry, or an entry's var-data — goes through
an ordinary entry decoder, which rediscovers its own preceding tails as
before. Entry decoders do keep a one-shot extent cache (see above), but there
is no per-entry boundary cache and no measured demand for one.

Ordered and staged decoders are **not** memoized either: they already carry
their current offset and never re-walk an earlier tail, so a cache would be
pure overhead.

## Staged (`into_*` / `skip_*`)

Maximum safety and the expected maximum-performance sequential path — and
the encoder dual: one chain, bind values not stages. Ownership and
generated stage types make a later tail unreachable until the current one
is consumed. Do not write `let after_bids = dec.into_bids(...)?`; continue
the chain.

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
`into_*` / `skip_*` **silently skips** remaining wire tails (groups and
var-data). That is easy to miss when a function returns early — prefer
advancing until `Complete` or an explicit skip.

### One-pass `into_*(|entry|)`

`into_fuel_figures(|entry| …)` consumes the current stage, visits every
entry of that group, and returns the next parent stage. There is no
separate group object to hold: count lives on the wire, empty groups
invoke the callback zero times, and skipping is `skip_fuel_figures()`.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_visit_entries}}
```

Dynamic-entry callbacks return the generated completion stage so the next
cursor comes from the walk, not from a pre-scan of `encoded_length()`.
Fixed-stride callbacks return `Result<(), E>`.

**Advantages**

- Wrong order is a compile error (missing method on this stage)
- One pass; no offset rescan
- Expected fastest sequential decode; maintained benches require it ≤ sbe-tool
  and ≤ iterator decode

**Disadvantages**

- Stage types appear in signatures; you cannot hold “the decoder” and pick
  tails later
- Skipping a tail requires an explicit `skip_*`
- Partial group walks use the random-access `Iterator`, which is not the
  one-pass path

## Full-frame bytes mid-walk

| Need | API |
|------|-----|
| Full frame after finishing the walk | complete stage `as_bytes_with_header()` |
| Full frame without consuming stages | inherent `dec.as_bytes_with_header()?` (rescans tails) |
| Fixed block only (not a full frame) | `dec.get_metadata().as_fixed_region_with_header()?` |

See [Generated code](generated-code.md#metadata-limits-tailed-messages) for the
metadata `limit` vs full-frame table.
