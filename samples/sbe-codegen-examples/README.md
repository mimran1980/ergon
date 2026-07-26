# sbe-codegen-examples

Generator-API examples — how to call `ergo-sbe` as a library to generate and
inspect codecs programmatically (without a `build.rs`).

The other samples (`l3-book`, `sbe-feature-tour`, `exchange-example`) all use
`build.rs` to generate codecs at compile time and then exercise them
end-to-end. These examples complement them by showing the **generator API**
itself — useful for tooling, golden-file workflows, and understanding what the
generator produces.

## Examples

```sh
cargo run --example flyweight       # flyweight codec generation + API surface
cargo run --example domain_objects  # owned domain-object generation (DTOs)
cargo run --example l3_nested       # 3-level nested-group API surface
cargo run --example dump_gen        # dump full generated source to stdout
```

## What each shows

| Example | Demonstrates |
|---------|-------------|
| `flyweight` | `Generator::generate()` with default config — zero-copy flyweights |
| `domain_objects` | `enable_domain_objects(true)` — owned DTOs (`String` var-data) + `From<Decoder>` |
| `l3_nested` | The full type graph for 3-level nested groups (L1→L2→L3 entry types) |
| `dump_gen` | The complete generated Rust source for inspection |

All examples parse the canonical car schema from
`sbe/tests/fixtures/schemas/example-schema.xml`.
