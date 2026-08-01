# Flyweight vs Whole-Struct

You can work **field-by-field** (classic flyweight) **or** fill / materialise a
**whole struct**. Use the style that matches how much of the message you touch.

| Style | Best when | Cost | Schema evolution |
|-------|-----------|------|------------------|
| **Flyweight (per-field)** | You only **read** one or a few fields; hot path | Zero-copy; no heap | New fields are optional at call sites (you simply don’t read them) |
| **`*FixedFields` + `.fixed(...)`** | You always write the **entire fixed block** | One struct write, still flyweight buffer | Adding a **required fixed field** to the schema → **compile error** until you set it in the struct |
| **`*Domain` DTO** (`.with_domain_objects(DomainVarData::…)`) | Whole message as owned data; enum picks `String` vs `Vec<u8>` var-data | **Allocates — never use on the hot path.** Easier app code for tooling, logging, offline processing | Same idea: regenerating after a schema change forces you to fill new struct fields |

#### Decode — individual fields (flyweight)

```rust,no_run
  // Per-field zero-copy access — no DTO allocation.
  let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
  let len = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?
      .fixed(&HeartbeatFixedFields { seq: 7 })
      .encoded_length_with_header();
  let dec = HeartbeatDecoder::try_from(&buf[..len])?;
  let seq = dec.seq();
  assert_eq!(seq, 7);
```

#### Encode — whole fixed block as a struct

When you always populate every fixed field, a struct is clearer **and** schema
additions break at **compile time**:

```rust,no_run
  // Fill every field at once — missing field → compile error.
  let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
  let len = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?
      .fixed(&HeartbeatFixedFields { seq: 7 })
      .encoded_length_with_header();
// If the schema later adds a field to the fixed block, this stops compiling
// until you add it to the struct literal — you cannot silently omit it.
```

#### Decode — flyweight (prefer for single-field reads)

```rust,no_run
{{#include ../../../examples/flyweight-access.rs:flyweight_access}}
```
*(From `sbe-feature-tour` — per-field zero-copy access, tested in CI.)*

#### Decode — whole message as a DTO

> **Do not use on the latency-sensitive path.** DTO decode allocates `Vec`/`String`
> and copies every field. For the hot path, use the flyweight decoder instead.

When you always need (almost) everything, or want to pass a value across threads
/ into non-SBE code:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_domain_dto}}
```

**Rule of thumb:** one field on the hot path → **flyweight**. Always fill or
always consume the whole message → **`FixedFields` / `Domain`** for clarity and
compile-time breakage on schema growth. More on DTOs in
[Recipes — Domain DTOs](../recipes/domain-dtos.md).
