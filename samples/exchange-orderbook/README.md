# exchange-orderbook

Multi-exchange SBE orderbook demo: vendor schemas → normalized AppMessage →
local book → optional ClickHouse snapshot.

## Status

**Sample.** Unit/roundtrip tests always; live CH via `just samples-orderbook`.

## Depends on

- Path: `ergosbe`, `ergo-clickhouse-persist`
- Schemas under `schemas/` (`binance-spot.xml`, `bitget-spot.xml`, `normalized-app.xml`)

## Build / test

```sh
cd samples/exchange-orderbook
cargo test -- --test-threads=1
cargo test --test e2e_persist_test -- --include-ignored --test-threads=1   # needs CH

# From repo root
just test-exchange-orderbook-live
just samples-orderbook
```

## Layout

| Path | Role |
|------|------|
| `src/orderbook.rs` | Local book apply |
| `src/persist.rs` | CH snapshot path |
| `schemas/` | Exchange + normalized SBE XML |
| `build.rs` | ErgoSBE generate normalized codecs |

## Public entry points

Library re-exports `orderbook` + `persist` for e2e tests; binary drives live WS.

## Where truth lives

- Samples map: [`../README.md`](../README.md)
- Phase2 ledger: [`../../phase2-completion-goal.md`](../../phase2-completion-goal.md)

## Non-goals

- HA / cluster failover (see `cluster-ha-orderbook`)
- Full multi-symbol OMS
