⚠️ **DEFERRED — post-v1.** Semantic type system is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# Semantic type system — from annotations to type safety

**Blocked by:** `02-composite-enum-set-wire-parity`

`semanticType` is SBE's buried treasure. Every field in an SBE schema can carry
a `semanticType` attribute that declares its business meaning:

```xml
<field name="price"      type="int64"  semanticType="Price"/>
<field name="qty"        type="int32"  semanticType="Qty"/>
<field name="timestamp"  type="uint64" semanticType="UTCTimestamp"/>
<field name="symbol"     type="string" semanticType="SecurityID"/>
```

The official Java generator stores this in a constant and ignores it. Rust can
do much more. The key insight: **semanticType tells the generator what the int
really is**, enabling type-safe APIs that prevent entire classes of bugs at
compile time.
**Status: DESIGN / ROADMAP**


## What semanticType unlocks

### 1. Strongly-typed newtypes (P0)

Instead of raw primitives, fields get `#[repr(transparent)]` newtypes:

```rust
// semanticType="Price" on int64          // semanticType="Qty" on int32
#[repr(transparent)]                      #[repr(transparent)]
pub struct Price(pub i64);               pub struct Qty(pub i32);

// Generated accessor uses the newtype
pub fn price(&self) -> Price { ... }     pub fn qty(&self) -> Qty { ... }
```

Compiler now rejects `order.set_price(qty)` — unit confusion caught at compile
time. Zero runtime cost (repr(transparent)).

### 1b. Type-level scale, currency, and units (P0.5)

For trading systems, `Price(i64)` is useful but still incomplete. The same raw
integer may represent USD with 4 decimals, JPY with 0 decimals, ticks, basis
points, lots, shares, millis, or nanos. When schema metadata or generator config
provides the information, encode it in the type:

```rust
#[repr(transparent)]
pub struct Price<const SCALE: i32, Ccy>(i64);

pub enum Usd {}
pub enum Jpy {}

pub type BidPx = Price<4, Usd>;
pub type YenPx = Price<0, Jpy>;
```

The generated aliases keep the public API readable, while the compiler rejects
mixing incompatible units. Formatting/conversion remains optional; raw access is
still a single integer load.

### 2. Built-in type registry (P0)

Known semantic types mapped to Rust types and behaviours:

| semanticType | Wire type | Rust newtype | Display | Notes |
|---|---|---|---|---|
| Price | int64 | `Price(i64)` | `$123.45` | Configurable decimal places |
| Qty | int32/uint32 | `Qty(i32)` | `1,000,000` | Always non-negative |
| UTCTimestamp | uint64 | `UTCTimestamp(u64)` | ISO 8601 | Nanos since epoch |
| LocalMktDate | uint16 | `LocalMktDate(u16)` | `2026-07-05` | Days since epoch |
| SecurityID | string | `SecurityID(String)` | as-is | ISIN, CUSIP, ticker |
| StringEnum | uint8/char | E3 newtype | variant name | Enum with unknown sentinel |
| BooleanFlag | uint8 | `BooleanFlag(bool)` | `true`/`false` | Single boolean |
| Percentage | int8/int16 | `Percentage(i16)` | `12.5%` | Basis points or percent |

### 3. Validation at the type boundary (P1)

```rust
impl Price {
    pub fn new(raw: i64) -> Result<Self, SemanticError> {
        if raw <= 0 { return Err(SemanticError::NonPositive); }
        Ok(Self(raw))
    }
}
```

The `raw()` escape hatch skips validation. Constructor validates once;
every use thereafter is zero-cost.

### 4. Cross-schema consistency checks (P1)

If schema A declares `semanticType="Price"` on int64 and schema B on int32,
the IR validation pass warns: "Price has mismatched wire type: int64 vs int32."

### 5. User-extensible registry (P2)

```rust
// build.rs
Generator::new(config)
    .with_semantic_type("NotionalValue", |f| {
        f.rust_type("NotionalValue")
         .wire_type(PrimitiveType::Int64)
         .display(|v| format!("${:.2}", v as f64 / 100.0))
         .conversion("rust_decimal::Decimal", "Decimal::new(val.0, 2)")
    })
    .generate(&schema);
```

Users register their own semantic types. The generator enforces wire-type
consistency and emits the newtype + conversions + Display.

## Acceptance criteria

- [ ] Parse `semanticType` attribute from XML into the Token IR
- [ ] IR validation: same semantic type → same primitive type across the schema
- [ ] Built-in registry: Price, Qty, UTCTimestamp, LocalMktDate, SecurityID,
      StringEnum, BooleanFlag, Percentage
- [ ] `semantic-newtypes` feature flag: off → raw primitives (current behaviour);
      on → newtype wrappers on field accessors
- [ ] Optional type-level scale/currency/unit markers for Price, Qty,
      Percentage, and timestamp-like semantic types
- [ ] Generator emits readable aliases for concrete schema fields, not only
      generic wrapper names
- [ ] Newtypes are `#[repr(transparent)]` with `raw()`, `From`/`Into`, `Debug`, `Clone`, `Copy`
- [ ] Display impl per semantic type (Price → `$123.45`, UTCTimestamp → ISO 8601)
- [ ] Validation constructor (`new() -> Result`) for types with business rules
- [ ] User-extensible registry via `with_semantic_type()` in config
- [ ] Cross-schema consistency: mismatched wire types for same semantic type → warning
- [ ] Cross-schema consistency: same semantic type with incompatible scale,
      currency, or unit emits a warning or error according to configured policy
- [ ] Field accessor rustdoc includes semantic type: `/// Price (semantic type: Price)`
- [ ] Test: Car schema with `semanticType` annotations → Price/Qty/StringEnum newtypes generated
- [ ] Test: passing a Qty to a Price parameter → compile error (type safety proven)
- [ ] Test: passing `Price<4, Usd>` where `Price<0, Jpy>` is expected fails to compile

Ref: `design/DECISIONS.md` §4 semantic newtypes. SBE XML spec `semanticType` attribute.
