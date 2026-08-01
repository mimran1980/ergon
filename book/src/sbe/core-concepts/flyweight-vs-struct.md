# Flyweight vs Whole-Struct

You can work **field-by-field** (classic flyweight) **or** fill / materialise a
**whole struct**. Use the style that matches how much of the message you touch.

| Style | Best when | Cost | Schema evolution |
|-------|-----------|------|------------------|
| **Flyweight (per-field)** | You only **read** one or a few fields; hot path | Zero-copy; no heap | New fields are optional at call sites (you simply don’t read them) |
| **`*FixedFields` + `.fixed(...)`** | You always write the **entire fixed block** | One struct write, still flyweight buffer | Adding a **required fixed field** to the schema → **compile error** until you set it in the struct |
| **`*Domain` DTO** (`.enable_domain_objects(DomainVarData::…)`) | Whole message as owned data; enum picks `String` vs `Vec<u8>` var-data | **Allocates — never use on the hot path.** Easier app code for tooling, logging, offline processing | Same idea: regenerating after a schema change forces you to fill new struct fields |

#### Decode — individual fields (flyweight)

```text
// Only read what you need; no owned DTO or whole-message materialisation.
let car = CarDecoder::try_from(buf)?;
let serial_number = car.serial_number();
let model_year = car.model_year();
```

#### Encode — whole fixed block as a struct

When you always populate every fixed field, a struct is clearer **and** schema
additions break at **compile time**:

```text
// Generated (simplified):
// pub struct CarFixedFields {
//     pub serial_number: u64,
//     pub model_year: u16,
//     pub available: BooleanType,
//     pub code: Model,
//     pub some_numbers: [u32; 4],
//     pub vehicle_code: [u8; 6],
//     pub extras: OptionalExtras,
//     pub engine: Engine,
// }

let len = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .fixed(&CarFixedFields {
        serial_number: 1234,
        model_year: 2013,
        available: true.into(),
        code: Model::A,
        some_numbers: [10, 20, 30, 40],
        vehicle_code: *b"ABCDEF",
        extras,
        engine,
    })
    .fuel_figures(0, |_| Ok(()))?
    .performance_figures(0, |_| Ok(()))?
    .manufacturer(b"Honda")?
    .model(b"Civic")?
    .activation_code(b"active")?
    .encoded_length_with_header()
        .expect("header present");
let frame = &buf[..len];

// If the schema later adds `paint_code` to the fixed block, this stops compiling
// until you add `paint_code: …` to the struct literal — you cannot silently omit it.
```

#### Decode — flyweight (prefer for single-field reads)

```text
let car = CarDecoder::try_from(buf)?;
// Only touch what you need — no allocation, no materialising the rest of the car.
let year = car.model_year();
```

#### Decode — whole message as a DTO

> **Do not use on the latency-sensitive path.** DTO decode allocates `Vec`/`String`
> and copies every field. For the hot path, use the flyweight decoder instead.

When you always need (almost) everything, or want to pass a value across threads
/ into non-SBE code:

```text
// build.rs: .enable_domain_objects(DomainVarData::LossyStrings)
let dto = CarDomain::try_from_decoder(CarDecoder::try_from(buf)?)?;
// dto is a plain Rust struct: Vecs for groups/strings, owned fields.
process_order(&dto);
let n = dto.encode(&mut out)?; // round-trip back to wire when needed
```

**Rule of thumb:** one field on the hot path → **flyweight**. Always fill or
always consume the whole message → **`FixedFields` / `Domain`** for clarity and
compile-time breakage on schema growth. More on DTOs in
[Recipes — Domain DTOs](../recipes/domain-dtos.md).
