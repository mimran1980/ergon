# Codegen as a library

Generator-API examples showing how to call `ergo-sbe` as a library to generate and
inspect codecs programmatically (without a `build.rs`).

The other samples (`l3-book`, `sbe-feature-tour`, `exchange-example`) all use
`build.rs` to generate codecs at compile time and then exercise them
end-to-end. These examples complement them by showing the generator API
itself, which is useful for tooling, golden-file workflows, and understanding what the
generator produces.

## From schema XML to generated Rust

<div class="ergon-anim" data-anim="codegen-pipeline" data-title="From schema XML to generated Rust"></div>

*Animation, in words:* it follows a small illustrative `Quote` schema (two
fixed fields, a `legs` group and a `venue` var-data field), not the car schema
the examples use. `generate(source, config)` is `parse`, `Schema::from_ir`,
then `Generator::generate`.

1. An optional XSD shape check (`with_xsd_validation()`, off by default) runs
   on the root document's text before anything is parsed.
2. `parse` turns the XML into one flat stream of 46 Begin/End tokens, with
   each type reference inlined.
3. Still inside `parse`, `resolve_schema` fills the same tokens in place:
   `price` at offset 0, `qty` at 8, a block length of 12, a group entry length
   of 8, null/min/max defaults, and checks for duplicate template ids.
4. `Schema::from_ir` only wraps the result. The `Generator` checks the
   `GenerationConfig`, then splits the tokens into typed messages, groups and
   fields.
5. One decoder call per message emits `QuoteDecoder`, the staged `into_*`
   chain, `ordered()` and `memoized()`. The encoder call emits the type-state
   `QuoteEncoder` and `compute_encoded_length(legs_count, venue_len)`, which
   is 40 bytes for 2 legs and a 4-byte venue (48 with the header).
6. `syn` and `prettyplease` produce one in-memory module, which is where
   these examples read `set.modules()`.
7. `write_to_out_dir()` writes `$OUT_DIR/quote.rs` (all or none) and prints
   `rerun-if-changed` for the schema. `sbe_mod!(quote)` includes it, and a
   plain `include!` does the same without a normal dependency on `ergo-sbe`.

If a field names an undefined type, `parse` stops there with
`ergo_sbe::schema_parse::invalid` and the line underlined. `resolve` and the
generator never run. All of this happens at build time, so a running program
never parses a schema. The cost is a rebuild for each schema change, and an
`sbe_rt` in every module unless you use `with_shared_module` or
`with_external_sbe_rt`. The book records no timings for the pipeline.

## Examples

```sh
# From the repository root; this crate is not a workspace member.
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example flyweight
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example domain_objects
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example l3_nested
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example dump_gen
```

## What each shows

| Example | Demonstrates |
|---------|-------------|
| `flyweight` | `Generator::generate()` with default config, giving zero-copy flyweights |
| `domain_objects` | `with_domain_objects(DomainVarData::Strings)`: owned DTOs (`String` var-data) + `try_from_decoder` |
| `l3_nested` | The full type graph for 3-level nested groups (L1, L2 and L3 entry types) |
| `dump_gen` | The complete generated Rust source for inspection |

All examples parse the canonical car schema.
