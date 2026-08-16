# Codegen as Library

Generator-API examples — how to call `ergo-sbe` as a library to generate and
inspect codecs programmatically (without a `build.rs`).

The other samples (`l3-book`, `sbe-feature-tour`, `exchange-example`) all use
`build.rs` to generate codecs at compile time and then exercise them
end-to-end. These examples complement them by showing the **generator API**
itself — useful for tooling, golden-file workflows, and understanding what the
generator produces.

## Examples

```sh
# From the repository root — this crate is not a workspace member.
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example flyweight
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example domain_objects
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example l3_nested
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example dump_gen
```

## What each shows

| Example | Demonstrates |
|---------|-------------|
| `flyweight` | `Generator::generate()` with default config — zero-copy flyweights |
| `domain_objects` | `with_domain_objects(DomainVarData::Strings)` — owned DTOs (`String` var-data) + `From<Decoder>` |
| `l3_nested` | The full type graph for 3-level nested groups (L1→L2→L3 entry types) |
| `dump_gen` | The complete generated Rust source for inspection |

All examples parse the canonical car schema.
