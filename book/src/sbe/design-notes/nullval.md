# Why NullVal Instead of Option

An SBE enum may declare a `nullValue` in the schema — an explicit wire sentinel
that means "not present" / "not set". When the schema doesn't specify one, SBE
defaults to the encoding type's null sentinel: the maximum value for unsigned
types (e.g. `255` for `uint8`) and the minimum value for signed types (e.g.
`-128` for `int8`).

An early design tried wrapping every enum field in `Option<EventCode>` at the
field site:

```text
// Option approach — REJECTED
pub fn event_code(&self) -> Option<EventCode> { … }
pub fn set_event_code(&mut self, val: Option<EventCode>) { … }
```

This was rejected for three reasons:

1. **API complexity.** Using `Option<EventCode>` at every access point forces
   every consumer to `.unwrap()` or match, even when the field is known to be
   populated. The `NullVal` approach gives you a plain `EventCode` type — if you
   care about null, check `code == EventCode::NullVal`; if you don't, just use
   it. (The wire encoding itself would be compatible either way: `None` maps to
   the null sentinel, `Some(v)` maps to `v`. The issue is ergonomics, not wire
   format.)

2. **Generated code complexity.** Every field site that uses `Option<EventCode>`
   needs value↔Option mapping in both accessor directions, inflating the
   generated code for no wire-format gain.

3. **Schema intent.** The schema declares a null sentinel as part of the enum's
   own value domain, not as a separate presence flag. A `NullVal` variant
   reflects that intent directly in the Rust type.

The chosen design adds a `NullVal` variant to every generated enum. It is the
same size as any other variant, wire-compatible with sbe-tool, and bears no
runtime cost:

```rust,ignore
// ergo-sbe generated (conceptual)
pub enum EventCode {
    NullVal = 255,  // or schema-declared nullValue
    Ok = 200,
    Error = 400,
    Timeout = 408,
}
```

For an `Optional` field (schema `presence="optional"`), the generated accessor
returns `Option<EventCode>` — but the null check compares against the `NullVal`
discriminant on the wire, never allocates, and is transparent to the caller.
