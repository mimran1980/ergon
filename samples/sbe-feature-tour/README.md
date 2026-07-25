# sbe-feature-tour

Standalone **laboratory** sample that exercises the main ergo-sbe generated
features against a checked-in schema. `publish = false`; not a product crate.

This sample is the **crates.io / docs.rs entry point** for learning ergo-sbe:
the published [ergo-sbe README](https://github.com/mimran1980/ergon/blob/main/sbe/README.md)
links here with absolute GitHub URLs (relative monorepo paths do not work on
crates.io).

## GitHub paths (stable)

| Resource | URL |
|----------|-----|
| This directory | https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour |
| Schema | https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/schemas/feature-tour.xml |
| Named demos (`demo_*`) | https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs |
| `build.rs` | https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/build.rs |
| Integration tests | https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/tests/feature_tour.rs |
| Deeper L3 sample | https://github.com/mimran1980/ergon/tree/main/samples/l3-book |

## Layout (in a checkout)

| Path | Role |
|------|------|
| `schemas/feature-tour.xml` | Self-contained multi-message schema (Heartbeat, Car, Note) |
| `build.rs` | Generates codecs + domain DTOs + bool/timestamp conversions |
| `src/lib.rs` | Named demos for each feature |
| `src/main.rs` | Prints and runs all demos |
| `OUT_DIR/feature_tour.rs` | Generated module (after `cargo build`; not checked in) |

## Run

```sh
git clone https://github.com/mimran1980/ergon.git
cd ergon
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
```

## Feature map → demo function

| Feature | Demo (in `src/lib.rs`) |
|---------|------------------------|
| Fixed message + `Encoder::ENCODED_LENGTH` | `demo_fixed_heartbeat` |
| Staged `CarEncodedLength` + exact encode | `demo_car_size_and_encode` |
| Consuming decoder stages (groups → nested → var-data) | `demo_car_decode_stages` |
| Owned DTO `CarDomain` + byte-identical re-encode | `demo_car_domain_dto` |
| Multi-template `AnyMessage` | `demo_any_message` |
| `try_from` / `try_wrap` / `verify` vs trusted wrap | `demo_try_vs_trusted` |
| Diagnostic `Display` / `Debug` | `demo_display_debug` |
| Run everything | `run_all` |

## Generation config (see `build.rs`)

- `enable_domain_objects()` → `CarDomain`, etc.
- `BooleanType` → `bool`
- `semanticType="UTCTimestamp"` → `chrono::DateTime<Utc>` on Heartbeat

## Inspect generated code

After a build in this directory:

```sh
find target -name feature_tour.rs 2>/dev/null
```

Open that file to see `ENCODED_LENGTH`, `CarEncodedLength`, encoders, decoders,
and domain structs produced for the schema.
