# Wire Order via Named Stages

### Why not “Java-style” parent hopping?

In sbe-tool you often juggle flyweights and call something like `.parent()` to
hand ownership back up the tree. In Rust that fight becomes **borrow-checker
pain**: move a group encoder in, get stuck returning the parent, lose the
thread of the code.

ergo-sbe leans on **scoped closures** and **chaining** so nested schemas stay
readable and you rarely pass encoder ownership field-to-field by hand:

```text
// Nested shape mirrors the schema — no .parent() hopscotch.
enc.fixed(&fields)
    .bids(n, |bids| {
        bids.add(|level| {
            level.price(p).size(s);
            level.orders(m, |ords| {
                ords.add(|o| { o.order_id(id); Ok(()) })?;
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    })?
    .ask_var_data(bytes)?;
```

Wire parity is exercised three ways: official Java `.sbe` fixtures, **live
dual-encode** suites that require ergo-sbe and sbe-tool Rust bytes to be
identical (`sbe_tool_wire_parity_test` for deep Car matrices;
`sbe_tool_multi_schema_wire_parity_test` across example/unit schemas with
checked-in sbe-tool reference crates under `sbe/tests/sbe_tool_reference/`),
and a maintained benchmark gate versus sbe-tool-generated codecs (see
[Benchmarks](../benchmarks.md)).

> **Early release (0.x).** This is the first published line of the crate. The
> **experimental banner stays** until the project has been battle-tested in
> enough real production environments — not merely until unit tests pass.
>
> Binary compatibility is covered by a large automated suite (golden bytes,
> schema edge cases, parity benches). That is necessary, not sufficient, for
> removing this warning.
>
> **If you use `ergo-sbe` in production**, please say so (GitHub issue or
> discussion). Hearing from heavy production users is how this banner goes
> away. Until then, expect possible API and generated-surface churn on the
> `0.x` series, and pin versions deliberately.
>
> **What we most want reports on** (open an issue titled e.g.
> `production-use: <your domain>`):
>
> 1. Live multi-schema / multi-template streams (not only unit fixtures)
> 2. Domain DTOs (`enable_domain_objects`) in a real app path — especially
>    `DomainVarData::LossyStrings` re-encode behaviour
> 3. Exact buffer sizing + Aeron/IPC **try_claim** (no oversize scratch buffers)
> 4. Nested/ragged books or similar twin groups (bids/asks order safety)
> 5. Schema evolution (`sinceVersion`) under mixed acting versions

### Wire order via **named stage structs**

SBE is a **positional** wire format: groups and var-data appear in a fixed
schema order with no per-field tags on the wire. That matters a lot in
financial markets, where it is common to have **two nearly identical repeating
groups** back-to-back — e.g. **bids then asks** (same entry layout, different
meaning). If you encode or decode them in the wrong order, the bytes still look
like a valid message: prices and sizes land in the opposite book side. You only
discover the disaster at **runtime** (wrong trades, inverted books, silent
corruption). Compile-time order exists so that mistake becomes a **type error**
while you still have the schema in front of you, not a production incident.

Order is enforced with the **same idea** as the classic type-state pattern
(`Encoder<State>` / `PhantomData`), but **not** that implementation.

**Implementation note:** an early design **did** use generic type-state stages.
On some encode paths that was about **~17% slower** than comparable free-order
flyweights. Profiling pointed at LLVM failing to optimise through the
type-parameter stage chain the way it does for plain monomorphic code. The API
was switched to **named stage structs** — same compile-time “you can only call
the next legal method” behaviour, without the generic tax on the hot path.

Generated code emits **separate types** for each stage, same fields, different
methods:

```text
// Approximate generated shape — not Encoder<AfterBids>:
pub struct BookEncoder<'a> { /* buf, pos, … */ }
pub struct BookAfterBids<'a> { /* same layout */ }
pub struct BookAfterAsks<'a> { /* same layout */ }
// …

impl BookEncoder<'a> {
    pub fn bids(self, …) -> Result<BookAfterBids<'a>, …> { … }
    // no asks() here — bids first on the wire
}
impl BookAfterBids<'a> {
    pub fn asks(self, …) -> Result<BookAfterAsks<'a>, …> { … }
    // no bids() here — already done
}
```

So after fixed fields you may only call the **next** group/var-data in schema
order. Calling `asks` before `bids` is a **type error** (`BookEncoder` has no
`asks` method). Decoders use the same idea: consuming stages
(`BookDecoder` → `BookDecoderAfterBids` → …).

Group bodies use **`|g| { g.add(|e| { … }) }`** so the outer encoder is not left
half-borrowed while you fill nested levels — the closure ends, then chaining
continues. That is intentional **API ergonomics for Rust** (avoids `.parent()`
style ownership hand-offs that fight the borrow checker on deep books).
