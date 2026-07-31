# Display / Debug

Diagnostic only — **not** a stable wire or log schema. Do **not** treat either
format as a protocol or long-term log contract.

**`Display` currently equals `Debug`** for generated decoders (`{car}` and
`{car:?}` print the same text). Prefer `Debug` in logs if you want that intent
to stay obvious when/if the two diverge later.

Real output from the feature-tour Car (`demo_car_size_and_encode` →
`CarDecoder`):

```text
let car = CarDecoder::try_from(buf.as_slice())?;
println!("{car}");
println!("{car:?}"); // same text as Display today
```

```text
CarDecoder { serialNumber: 1234, modelYear: 2013, available: true, code: A, fuelFigures: ["{ speed: 30, mpg: 35.9, usageDescription: Urban }", "{ speed: 60, mpg: 25.0, usageDescription: Highway }"], performanceFigures: ["{ octaneRating: 95, acceleration: [{ mph: 30, seconds: 4.0 }, { mph: 60, seconds: 7.5 }] }"], manufacturer: "Honda", model: "Civic VTi", activationCode: "abcdef" }
```

Truncated / incomplete buffers omit missing tails rather than panicking.
See [demo_display_debug](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs).
