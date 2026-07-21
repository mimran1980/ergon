# ErgoSBE Roadmap

## Current: Prototype release

ErgoSBE and Ergo Aeron Cluster are experimental prototype crates.
They generate correct, fast SBE codecs and connect to Aeron Cluster,
but are not production-tested.

## Near-term (within prototype)

- [x] ErgoSBE fallible generation API
- [x] Vendored cluster schemas
- [x] Concrete consuming encoder/decoder stages
- [x] Domain-object generation
- [x] Fragment assembly for controlled egress
- [ ] `no_std` support (codec core is already no_std-clean)
- [ ] Serde support on domain objects
- [ ] Proc-macro driver (`#[ergo_sbe::schema("car.xml")]`)

## Deferred

- Rust Aeron Cluster **service** (explicit non-goal)
- Live exchange WebSocket in CI (manual recipe only)
- `MaybeUninit` owned-buffer encoders (benchmark experiment)
- SIMD/prefetch (only for measured bulk bottlenecks)

## Published crates

| Crate | Status |
|-------|--------|
| `ergo-sbe` | Publishable prototype |
| `ergo-aeron-cluster` | Publishable prototype |
| `ergo-clickhouse-persist` | Unpublished lab |
| `ergo-clickhouse-persist-derive` | Unpublished lab |
