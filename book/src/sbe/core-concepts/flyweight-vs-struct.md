# Flyweight vs Whole-Struct

Use the style that matches how much of the message you touch.

| Style | Best when | Cost | Schema evolution |
|-------|-----------|------|------------------|
| **Flyweight (per-field)** | You only **read** one or a few fields; hot path | Zero-copy; no heap | New fields are optional at call sites (you simply don’t read them) |
| **`*FixedFields` + `.fixed(...)`** | You always write the **entire fixed block** | One struct write, still flyweight buffer | Adding a **required fixed field** to the schema → **compile error** until you set it in the struct |
| **`*Domain` DTO** (`.with_domain_objects(DomainVarData::…)`) | Whole message as owned data; enum picks `String` vs `Vec<u8>` var-data | **Allocates — never use on the hot path.** Easier app code for tooling, logging, offline processing | Same idea: regenerating after a schema change forces you to fill new struct fields |
| **`*Source` trait** (`.with_source_traits(true)`) | Encode your **own** type, groups and var-data included, without a DTO | No allocation: groups are iterators, var-data is borrowed | A new field is a new trait method: every implementation stops compiling until it provides it |

#### Encode — whole fixed block as a struct

When you always populate every fixed field, a struct is clearer **and** schema
additions break at **compile time**:

```rust,no_run
{{#include ../../../examples/heartbeat-encode.rs:staged_chaining}}
```
*(From `book/examples/heartbeat-encode.rs` — compiled against the feature-tour codec.)*

If the schema later adds a required field to the fixed block, this stops
compiling until you add it to the struct literal — you cannot silently omit it.

#### Decode — individual fields (flyweight)

```rust,no_run
{{#include ../../../examples/flyweight-access.rs:flyweight_access}}
```
*(From `book/examples/flyweight-access.rs` — compiled against the feature-tour codec.)*

#### Decode — whole message as a DTO

> **Do not use on the latency-sensitive path.** DTO decode allocates `Vec`/`String`
> and copies every field. For the hot path, use the flyweight decoder instead.

When you always need (almost) everything, or want to pass a value across threads
/ into non-SBE code:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_domain_dto}}
```

#### Encode — your own type through a `Source` trait

`*FixedFields` covers the fixed block only. With `.with_source_traits(true)`
each message gets a `{Message}Source` trait — one getter per field in wire
order, `impl ExactSizeIterator<Item = impl …EntrySource>` per repeating group,
`&[u8]` per var-data — and the encoder gains `encode_from(&src)` and
`encoded_length_with_header_from(&src)`. Implement the trait on the type your
application already has; `encode_from` writes it through the same setters and
type-state stages a hand-written encode uses, and returns the terminal stage:

```rust,no_run
{{#include ../../../examples/claim-send.rs:source_trait}}
```
*(From `book/examples/claim-send.rs` — compiled against the feature-tour codec.)*

A fixed getter returns what the field's `*FixedFields` entry holds
(`Option<T>` for an optional field, whose `None` writes the null value and
whose `Some` of the null value, NaN for a float, is refused as
`SomeNullValue`), or its domain type when one is configured. A group iterator whose `len()` does
not match what it yields is `GroupCountMismatch` or `GroupFull`, as with a
hand-written group.

**Rule of thumb:** one field on the hot path → **flyweight**. Always fill or
always consume the whole message → **`FixedFields` / `Domain`** for clarity and
compile-time breakage on schema growth. More on DTOs in
[Recipes — Domain DTOs](../recipes/domain-dtos.md).
