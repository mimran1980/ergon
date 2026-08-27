# Timestamp Conversions

SBE represents timestamps as `uint64` wire fields with a `semanticType` attribute
(`UTCTimestamp` = nanoseconds, `UTCTimestampMicros` = microseconds,
`UTCTimestampMillis` = milliseconds). The `chrono` feature (see
[Feature Integrations](../configuration/feature-integrations.md)) converts
these to `chrono::DateTime<Utc>` and `chrono::NaiveDateTime` with one line
of config.

## Nanoseconds and microseconds — built-in

Enable the `chrono` feature in your `Cargo.toml`:

```toml
[dependencies]
ergo-sbe = { version = "0.1", features = ["chrono"] }
chrono = "0.4"
```

Then register the converters by `semanticType` in `build.rs`:

```rust,ignore
use ergo_sbe::{ConversionSelector, GenerationConfig};

let config = GenerationConfig::new("msgs")
    .with_domain_type(
        ConversionSelector::semantic_type("UTCTimestamp"),
        "chrono::DateTime<chrono::Utc>",
    )
    .with_domain_type(
        ConversionSelector::semantic_type("UTCTimestampMicros"),
        "chrono::NaiveDateTime",
    );
```

Schema:

```xml
<field name="created_at"  id="1" type="uint64" semanticType="UTCTimestamp"/>
<field name="updated_at"  id="2" type="uint64" semanticType="UTCTimestampMicros"/>
```

Generated API — `try_created_at()` returns `DateTime<Utc>`, `try_updated_at()`
returns `NaiveDateTime`:

```rust,ignore
// Decode
let created: chrono::DateTime<chrono::Utc> = dec.try_created_at()?;
let updated: chrono::NaiveDateTime = dec.try_updated_at()?;

// Encode
enc.try_created_at(chrono::Utc::now())?;
enc.try_updated_at(
    chrono::DateTime::from_timestamp_micros(1_720_000_000_000_000)
        .ok_or("micros out of range")?
        .naive_utc(),
)?;
```

Conversion cost: 2.8 ns (nanos → DateTime), 5.5 ns (micros → NaiveDateTime).
See the [measured benchmarks](../configuration/feature-integrations.md#measured-conversion-cost).

## One selector, many fields

`ConversionSelector::semantic_type(..)` matches **every** field in the
schema carrying that `semanticType` — not just one. You register the
conversion **once**, not once per field. A schema with three separate
`UTCTimestamp` timestamps needs no more config than one with a single field:

```xml
<field name="createdAt"  id="1" type="uint64" semanticType="UTCTimestamp"/>
<field name="updatedAt"  id="2" type="uint64" semanticType="UTCTimestamp"/>
<field name="expiresAt"  id="3" type="uint64" semanticType="UTCTimestamp"/>
```

```rust,ignore
// Same one-time call as the two-field example above — no per-field repeats.
let config = GenerationConfig::new("msgs")
    .with_domain_type(
        ConversionSelector::semantic_type("UTCTimestamp"),
        "chrono::DateTime<chrono::Utc>",
    );
```

All three fields get their own concrete accessor, generated from that one call:

```rust,ignore
enc.try_created_at(chrono::Utc::now())?;
enc.try_updated_at(chrono::Utc::now())?;
enc.try_expires_at(chrono::Utc::now())?;

let created: chrono::DateTime<chrono::Utc> = dec.try_created_at()?;
let updated: chrono::DateTime<chrono::Utc> = dec.try_updated_at()?;
let expires: chrono::DateTime<chrono::Utc> = dec.try_expires_at()?;
```

This is why `UTCTimestamp` and `UTCTimestampMicros` need **separate**
`with_domain_type` calls in the two-field example above — they're different
`semanticType` strings, so they're different selectors — but adding a fourth
`UTCTimestamp` field to the *same* schema needs no config change at all.
[`ConversionSelector::field_path`](https://docs.rs/ergo-sbe/latest/ergo_sbe/enum.ConversionSelector.html)
is the escape hatch when one specific field needs to differ from its
semantic-type siblings.

## Mixed precisions, one app type — `DomainImpl::Manual`

The built-in converters give nanos → `DateTime<Utc>` and micros →
`NaiveDateTime` — two different **app types**, because that's what the
built-in impls happen to produce. Real schemas often need the opposite: three
fields at three different wire precisions, all normalized to the **same**
`DateTime<Utc>` so downstream app code never branches on precision. None of
the three is the bare `uint64` + `semanticType="UTCTimestamp"` shape the
built-in converter matches, so none gets an auto-generated impl — that's
exactly the case [`DomainImpl::Manual`](../configuration/conversion-vs-domain.md#option-b-manual-impl--concrete-signatures-your-own-conversion-logic)
is for: concrete `try_*` signatures from ergo-sbe, conversion body from you.

Distinguish the three precisions with single-element composites (a distinct
Rust type per precision — `TimestampMillis(u64)` is not `TimestampNanos(u64)`
even though the wire shape is identical):

```xml
<composite name="TimestampMillis"><type name="ts" primitiveType="uint64"/></composite>
<composite name="TimestampMicros"><type name="ts" primitiveType="uint64"/></composite>
<composite name="TimestampNanos"><type name="ts" primitiveType="uint64"/></composite>

<field name="createdAt" id="1" type="TimestampNanos"/>
<field name="updatedAt" id="2" type="TimestampMicros"/>
<field name="deletedAt" id="3" type="TimestampMillis"/>
```

One `with_manual_domain_type(selector, path)` call per composite — same
target type, `rust_decimal`-style — in `build.rs`:

```rust,ignore
use ergo_sbe::{ConversionSelector, GenerationConfig};

let config = GenerationConfig::new("msgs")
    .with_manual_domain_type(
        ConversionSelector::named_type("TimestampNanos"),
        "chrono::DateTime<chrono::Utc>",
    )
    .with_manual_domain_type(
        ConversionSelector::named_type("TimestampMicros"),
        "chrono::DateTime<chrono::Utc>",
    )
    .with_manual_domain_type(
        ConversionSelector::named_type("TimestampMillis"),
        "chrono::DateTime<chrono::Utc>",
    );
```

Each composite is a genuinely new named type, so there is no built-in
template to offer — write the three impls yourself, one per precision
(the traits live in the generated module as `sbe_rt::TryFromSbe` /
`sbe_rt::TryToSbe`; import from there, not from `ergo_sbe::codegen`, which
is crate-private):

```rust,ignore
// my_msgs is the module name passed to GenerationConfig::new("my_msgs")
use my_msgs::sbe_rt::{TryFromSbe, TryToSbe};
use my_msgs::{TimestampMillis, TimestampMicros, TimestampNanos};

impl TryFromSbe<TimestampNanos> for chrono::DateTime<chrono::Utc> {
    type Error = &'static str;
    fn try_from_sbe(wire: TimestampNanos) -> Result<Self, Self::Error> {
        let ns = wire.0; // single-element composite is a transparent wrapper
        chrono::DateTime::from_timestamp((ns / 1_000_000_000) as i64, (ns % 1_000_000_000) as u32)
            .ok_or("timestamp out of range")
    }
}
impl TryToSbe<TimestampNanos> for chrono::DateTime<chrono::Utc> {
    type Error = &'static str;
    fn try_to_sbe(&self) -> Result<TimestampNanos, Self::Error> {
        Ok(TimestampNanos(self.timestamp_nanos_opt().ok_or("overflow")? as u64))
    }
}

impl TryFromSbe<TimestampMicros> for chrono::DateTime<chrono::Utc> {
    type Error = &'static str;
    fn try_from_sbe(wire: TimestampMicros) -> Result<Self, Self::Error> {
        let us = wire.0;
        chrono::DateTime::from_timestamp((us / 1_000_000) as i64, ((us % 1_000_000) * 1_000) as u32)
            .ok_or("timestamp out of range")
    }
}
impl TryToSbe<TimestampMicros> for chrono::DateTime<chrono::Utc> {
    type Error = &'static str;
    fn try_to_sbe(&self) -> Result<TimestampMicros, Self::Error> {
        Ok(TimestampMicros(self.timestamp_micros() as u64))
    }
}

impl TryFromSbe<TimestampMillis> for chrono::DateTime<chrono::Utc> {
    type Error = &'static str;
    fn try_from_sbe(wire: TimestampMillis) -> Result<Self, Self::Error> {
        let ms = wire.0;
        chrono::DateTime::from_timestamp((ms / 1000) as i64, ((ms % 1000) * 1_000_000) as u32)
            .ok_or("timestamp out of range")
    }
}
impl TryToSbe<TimestampMillis> for chrono::DateTime<chrono::Utc> {
    type Error = &'static str;
    fn try_to_sbe(&self) -> Result<TimestampMillis, Self::Error> {
        Ok(TimestampMillis(self.timestamp_millis() as u64))
    }
}
```

All three fields now return the exact same app type despite three different
wire precisions — the caller never has to know or care which precision a
given field was wire-encoded at:

```rust,ignore
let created: chrono::DateTime<chrono::Utc> = dec.try_created_at()?;
let updated: chrono::DateTime<chrono::Utc> = dec.try_updated_at()?;
let deleted: chrono::DateTime<chrono::Utc> = dec.try_deleted_at()?;

enc.try_created_at(chrono::Utc::now())?;
enc.try_updated_at(chrono::Utc::now())?;
enc.try_deleted_at(chrono::Utc::now())?;
```

If you forget one of the three impls, the compile error names it directly —
`` `chrono::DateTime<Utc>` has no `TryFromSbe<TimestampMillis>` impl `` —
instead of the default trait-bound message.
