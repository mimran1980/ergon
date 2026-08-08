# Why NullVal Instead of Option

An SBE enum may declare a `nullValue` in the schema — an explicit wire sentinel
that means "not present" / "not set". When the schema doesn't specify one, SBE
defaults to the encoding type's null sentinel: the maximum value for unsigned
types (e.g. `255` for `uint8`) and the minimum value for signed types (e.g.
`-128` for `int8`).

An early design tried wrapping every enum field in `Option<EventCode>` at the
field site:

```rust,ignore
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

On **encode**, wrap does not auto-nullify optionals. Call `apply_nulls()` after
`wrap_and_apply_header` when any optional may be left unset — otherwise stale
buffer bytes ship as if they were intentional values. See
[Encode and Decode](../getting-started/encode-decode.md#optional-fields-and-apply_nulls).

## Opting into `Option<T>` (configurable)

The `NullVal` default is the right choice for most schemas — zero-cost, matches
the schema's declared value domain, and keeps generated code lean. But when a
codebase already uses `Option` heavily, or when every access site already checks
for null, the boilerplate of `if code == EventCode::NullVal` can outweigh the
simplicity.

Ergon supports **opt-in `Option<T>` mapping** per selector — the wire format
stays identical (`NullVal` discriminant → `None`, any other value → `Some(v)`),
but the generated accessors use `Option<EventCode>` (for enums) and
`Option<bool>` (for `BooleanType`):

```rust,ignore
use ergo_sbe::{ConversionSelector, GenerationConfig};

// Enum fields matching this selector → Option<EventCode>
let config = GenerationConfig::new("msgs")
    .with_null_as_option(ConversionSelector::named_type("EventCode"));

// All boolean fields → Option<bool>
let config = GenerationConfig::new("msgs")
    .with_null_as_option(ConversionSelector::named_type("BooleanType"));

// Every enum in the schema
let config = GenerationConfig::new("msgs")
    .with_null_as_option(ConversionSelector::all_enums());
```

Generated diff (enum):

```rust,ignore
// Default (NullVal)                    // with_null_as_option
pub fn code(&self) -> EventCode { … }   →   pub fn code(&self) -> Option<EventCode> { … }
pub fn set_code(&mut self, v: EventCode) →   pub fn set_code(&mut self, v: Option<EventCode>)
```

Generated diff (bool):

```rust,ignore
pub fn available(&self) -> BooleanType { … }   →   pub fn available(&self) -> Option<bool> { … }
```

The wire encoding is byte-identical either way — `None` writes the `NullVal`
discriminant, `Some(v)` writes `v`. The choice is pure API preference.

