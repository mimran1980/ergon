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

```rust,no_run
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

```rust,no_run
// Decode
let created: chrono::DateTime<chrono::Utc> = dec.try_created_at()?;
let updated: chrono::NaiveDateTime = dec.try_updated_at()?;

// Encode
enc.try_created_at(chrono::Utc::now())?;
enc.try_updated_at(chrono::NaiveDateTime::from_timestamp_micros(1_720_000_000_000_000).unwrap())?;
```

Conversion cost: 2.8 ns (nanos → DateTime), 5.5 ns (micros → NaiveDateTime).
See the [measured benchmarks](../configuration/feature-integrations.md#measured-conversion-cost).

## Milliseconds — composite newtype

`UTCTimestampMillis` has no built-in chrono converter. Distinguish millis
fields from nanos/micros fields with a single-element composite:

```xml
<composite name="TimestampMillis">
  <type name="ts" primitiveType="uint64"/>
</composite>
<field name="received_at" id="3" type="TimestampMillis"/>
```

This generates `TimestampMillis(pub u64)` — a distinct Rust type from `u64`.
Now implement `TryFromSbe` / `TryToSbe` for it in your application crate.
The traits are emitted into the generated module as `sbe_rt::TryFromSbe` /
`sbe_rt::TryToSbe` — import them from your generated codec module, not from
`ergo_sbe::codegen` (which is crate-private):

```rust,no_run
// my_msgs is the module name passed to GenerationConfig::new("my_msgs")
use my_msgs::sbe_rt::{TryFromSbe, TryToSbe};
use my_msgs::TimestampMillis;

impl TryFromSbe<TimestampMillis> for chrono::NaiveDateTime {
    type Error = my_msgs::sbe_rt::DecodeError;
    fn try_from_sbe(wire: TimestampMillis) -> Result<Self, Self::Error> {
        let ms = wire.0; // TimestampMillis is a transparent wrapper around u64
        let secs = (ms / 1000) as i64;
        let nsecs = ((ms % 1000) * 1_000_000) as u32;
        chrono::DateTime::from_timestamp(secs, nsecs)
            .map(|dt| dt.naive_utc())
            .ok_or_else(|| my_msgs::sbe_rt::DecodeError::ValueOutOfRange {
                field: "received_at",
                actual: ms as i128,
                min: 0,
                max: i64::MAX as i128,
            })
    }
}

impl TryToSbe<TimestampMillis> for chrono::NaiveDateTime {
    fn try_to_sbe(&self) -> Result<TimestampMillis, ergo_sbe::EncodeError> {
        Ok(TimestampMillis(self.and_utc().timestamp_millis() as u64))
    }
}
```

Then register it in `build.rs`:

```rust,no_run
let config = GenerationConfig::new("msgs")
    .with_conversion(ConversionSelector::named_type("TimestampMillis"));
```

The generated accessor is `dec.received_at_as::<chrono::NaiveDateTime>()?` —
a different shape from `try_received_at()` because `with_conversion` (generic)
vs `with_domain_type` (canonical) choose different method-name patterns.
