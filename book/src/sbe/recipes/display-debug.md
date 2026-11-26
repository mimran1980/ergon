# Display / Debug

Diagnostic only; not a stable wire or log schema. Do not treat either
format as a protocol or long-term log contract.

**`Display` currently equals `Debug`** for generated decoders (`{car}` and
`{car:?}` print the same text). Prefer `Debug` in logs if you want that intent
to stay obvious when/if the two diverge later.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_display_debug}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

Real output from the feature-tour Car (the `CarDecoder` that
`demo_car_size_and_encode` produces):

```rust,ignore
CarDecoder { serialNumber: 1234, modelYear: 2013, available: true, code: A, extras: sportsPack|cruiseControl, engine: Engine { capacity: 2000, numCylinders: 4, maxRpm: 9000, manufacturerCode: [49, 50, 51], fuel: "Petrol", efficiency: 0, boosterEnabled: F, booster: Booster { BoostType: TURBO, horsePower: 210 } }, fuelFigures: ["{ speed: 30, mpg: 35.9, usageDescription: Urban }", "{ speed: 60, mpg: 25.0, usageDescription: Highway }"], performanceFigures: ["{ octaneRating: 95, acceleration: [{ mph: 30, seconds: 4.0 }, { mph: 60, seconds: 7.5 }] }"], manufacturer: "Honda", model: "Civic VTi", activationCode: "abcdef" }
```

Truncated / incomplete buffers omit missing tails rather than panicking.
