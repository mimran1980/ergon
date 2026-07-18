# advanced-bitget

IPC baseline sample: Bitget (or fixture) feed → normalize AppMessage → Aeron
**IPC** → typed books + dynamic V2 rows → ClickHouse.

## Status

**Sample.** Live-green via `just samples-orderbook` when ClickHouse is up.

## Depends on

- `ergosbe`, `ergo-clickhouse-persist`
- `rusteron-client` / media-driver **=0.2.1** (not 0.2.4)
- Docker ClickHouse on `127.0.0.1:8123` for live CH tests

## Build / test

```sh
# From repo root
just samples-orderbook          # exchange-orderbook + advanced-bitget live path
just test-ipc                   # IPC tests, skip CH
just test-clickhouse-live       # CH exact rows (ignored tests included)

cd samples/advanced-bitget
cargo test -- --test-threads=1 --skip clickhouse
```

## Layout

Typical three-thread ownership: ingest / media-driver / persist. Dynamic schema
registration once; rows via `DynamicRecorder` + `PersistSender` flush.

## Failure modes

- CH unreachable → live tests fail preflight (start Docker CH first)
- Backpressure → batch drop policy on persist path (never block feed unboundedly)

## Where truth lives

- Design: [`../../docs/superpowers/specs/2026-07-10-bitget-aeron-clickhouse-sample-design.md`](../../docs/superpowers/specs/2026-07-10-bitget-aeron-clickhouse-sample-design.md)
- Samples map: [`../README.md`](../README.md)

## Non-goals

- Cluster leadership / NewLeader handling → use `cluster-ha-orderbook`
- Bumping this sample to rusteron 0.2.4 without a measured pin decision
