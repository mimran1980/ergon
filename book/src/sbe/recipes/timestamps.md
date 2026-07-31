# Timestamp Conversions

Three wire `uint64` fields, each a different timestamp precision on the wire,
but **all mapping to `chrono::DateTime<Utc>`** in Rust. The generated
`u64`↔`DateTime` converter always uses **nanosecond** precision, so millis and
micros need their own `TryFromSbe`/`TryToSbe` impls. Distinguish them with
`FieldPath` selectors:

```xml
<!-- schema fragment — three uint64 fields, same wire type, three precisions -->
<field name="created_at"  id="1" type="uint64" semanticType="UTCTimestamp"/>
<field name="updated_at"  id="2" type="uint64" semanticType="UTCTimestampMicros"/>
<field name="received_at" id="3" type="uint64" semanticType="UTCTimestampMillis"/>
```

```text
// build.rs — register converters for all three
let config = GenerationConfig::new("msgs")
    .with_conversion(ConversionSelector::field_path("Event.created_at"))   // nanos, built-in
    .with_conversion(ConversionSelector::field_path("Event.updated_at"))   // micros, custom
    .with_conversion(ConversionSelector::field_path("Event.received_at")); // millis, custom
```

Writing `TryFromSbe<u64>` for micros would clash with the built-in nano
converter — `TryFromSbe<u64>` can only exist once. Resolve this by naming
the wire fields unique types — the idiomatic pattern when three `uint64`
columns mean three different things:

```xml
<!-- Distinguish wire types by name — all are uint64 under the hood -->
<composite name="TimestampNanos">  <type name="ts" primitiveType="uint64"/>  </composite>
<composite name="TimestampMicros"> <type name="ts" primitiveType="uint64"/>  </composite>
<composite name="TimestampMillis"> <type name="ts" primitiveType="uint64"/>  </composite>

<message name="Event" id="1">
  <field name="created_at"  id="1" type="TimestampNanos"/>
  <field name="updated_at"  id="2" type="TimestampMicros"/>
  <field name="received_at" id="3" type="TimestampMillis"/>
</message>
```

Now each wire type generates a distinct Rust newtype (`TimestampNanos`,
`TimestampMicros`, `TimestampMillis` — all `#[repr(transparent)]` wrappers
around `u64`). Implement the converters per-type, no blanket-clash:

```text
// Nanos — trivially delegates to the built-in logic
impl TryFromSbe<TimestampNanos> for chrono::DateTime<chrono::Utc> {
    type Error = &'static str;
    fn try_from_sbe(wire: TimestampNanos) -> Result<Self, Self::Error> {
        chrono::DateTime::from_timestamp(
            (wire.0 / 1_000_000_000) as i64,
            (wire.0 % 1_000_000_000) as u32,
        )
        .ok_or("timestamp out of range")
    }
}
// … TryToSbe, etc.

// Micros
impl TryFromSbe<TimestampMicros> for chrono::DateTime<chrono::Utc> {
    type Error = &'static str;
    fn try_from_sbe(wire: TimestampMicros) -> Result<Self, Self::Error> {
        chrono::DateTime::from_timestamp(
            (wire.0 / 1_000_000) as i64,
            ((wire.0 % 1_000_000) * 1_000) as u32,
        )
        .ok_or("timestamp out of range")
    }
}

// Millis
impl TryFromSbe<TimestampMillis> for chrono::DateTime<chrono::Utc> {
    type Error = &'static str;
    fn try_from_sbe(wire: TimestampMillis) -> Result<Self, Self::Error> {
        chrono::DateTime::from_timestamp(
            (wire.0 / 1_000) as i64,
            ((wire.0 % 1_000) * 1_000_000) as u32,
        )
        .ok_or("timestamp out of range")
    }
}
```

```text
// build.rs — three selectors, each naming a distinct named type
let config = GenerationConfig::new("msgs")
    .with_conversion(ConversionSelector::named_type("TimestampNanos"))
    .with_conversion(ConversionSelector::named_type("TimestampMicros"))
    .with_conversion(ConversionSelector::named_type("TimestampMillis"));
```

```text
// Encode — all use the same Rust type, with the wire precision implicit
let now = chrono::Utc::now();
enc.created_at_from(&now)?;      // → TimestampNanos  (wire: u64 nanos)
enc.updated_at_from(&now)?;      // → TimestampMicros (wire: u64 micros)
enc.received_at_from(&now)?;     // → TimestampMillis (wire: u64 millis)

// Decode — all return chrono::DateTime<Utc>, precision transparent
let created:  chrono::DateTime<chrono::Utc> = dec.created_at_as()?;
let updated:  chrono::DateTime<chrono::Utc> = dec.updated_at_as()?;
let received: chrono::DateTime<chrono::Utc> = dec.received_at_as()?;
```

The pattern generalises: **one Rust type, N wire representations** → one
single-field composite per representation, each with its own `TryFromSbe` /
`TryToSbe` impl, all distinguished by `ConversionSelector::named_type`.
