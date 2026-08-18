# Why NullVal Instead of Option

An SBE enum may declare a `nullValue` in the schema — an explicit wire sentinel
that means "not present" / "not set". When the schema doesn't specify one, SBE
defaults to the encoding type's null sentinel: the maximum value for unsigned
types (e.g. `255` for `uint8`) and the minimum value for signed types (e.g.
`-128` for `int8`).

`nullValue` / `minValue` / `maxValue` on a type or field must fit the declared
primitive width. `nullValue="256"` on `uint8` is a parse error — it would
otherwise collapse to `0` on the wire and make `Some(0)` indistinguishable from
`None`. See [Error Diagnostics](../getting-started/error-diagnostics.md).

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

On **encode**, `fixed(&FixedFields)` always writes every fixed field: `Some(v)`
writes the value and `None` writes the exact schema null wire image (including
nested optional composite members). Dirty buffer reuse is therefore safe when
you go through `fixed`. Prefer that path for whole-message encodes.

`apply_nulls()` remains on the unfixed encoder after
`wrap_and_apply_header` when you set individual optional fields piecemeal
instead of using `FixedFields`. See
[Encode and Decode](../getting-started/encode-decode.md#optional-fields-and-apply_nulls).

## Opting into `Option<T>` with `with_null_as_option`

The `NullVal` is the right default, but some codebases prefer `Option`
throughout. Use `with_null_as_option` to make generated enum accessors
return `Option<Enum>` — `NullVal` maps to `None`, all other values to
`Some(v)`. Wire bytes are identical either way.

```rust,ignore
use ergo_sbe::{ConversionSelector, GenerationConfig};

// Individual enum → Option<Enum>
let config = GenerationConfig::new("msgs")
    .with_null_as_option(ConversionSelector::named_type("EventCode"));

// Every enum in the schema → Option<Enum>
let config = GenerationConfig::new("msgs")
    .with_all_enums_as_option();
```

Generated diff (individual setter):

```rust,ignore
// Default (NullVal)                         // with_null_as_option
pub fn code(&self) -> EventCode { … }   →   pub fn code(&self) -> Option<EventCode> { … }
```

The `as_option()` method is also generated on every enum for manual use:
`event_code.as_option()` → `Option<EventCode>`.

## Null-aware accessors on BooleanType

For `BooleanType` fields, ergon emits a `_bool()` accessor alongside the
standard enum getter:

```rust,ignore
// Standard getter — returns the enum variant (raw wire discriminant).
pub fn available_wire(&self) -> BooleanType { … }

// Null-aware — rejects NullVal (returns Err); Ok(true/false) otherwise.
// Required fields → Result<bool, DecodeError>; optional → Option<bool>.
pub fn try_available_bool(&self) -> Result<bool, DecodeError> { … }
```

For enums and other types, the `NullVal` variant remains the default.
`with_null_as_option` (above) is the opt-in `Option<T>` mapping; the
wire encoding is identical either way.

