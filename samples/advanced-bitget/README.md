# advanced-bitget

IPC baseline sample: Bitget (or fixture) feed → normalize AppMessage → Aeron
**IPC** → typed books + dynamic V2 rows → ClickHouse.

## Status

**Sample.** Live-green via `just samples-orderbook` when ClickHouse is up.

## Depends on

- `ergosbe`, `ergo-clickhouse-persist`
- `rusteron-client` / media-driver **`0.2`** (same line as cluster; latest 0.2.x)
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

**Canonical nested encode:** `src/publication.rs` (AppMessage → L2Book via
`compute_encoded_length_with_message_header` + `payload_with` + `try_add`).  
Guide: [`../../sbe/docs/guide/claim-nested-encode.md`](../../sbe/docs/guide/claim-nested-encode.md).

## Failure modes

| Condition | Behaviour |
|-----------|-----------|
| ClickHouse unreachable | Live tests fail preflight — start Docker CH first |
| Persist queue full | Drop / batch policy — never block the feed unboundedly |
| WS disconnect | Reconnect path in the sample binary (see `main` / config) |
| Need leadership failover | Use [`../cluster-ha-orderbook/`](../cluster-ha-orderbook/) instead |

## When not to use

Choose **cluster-ha-orderbook** if you need NewLeader / never-stale book across
cluster leadership releases. This sample is IPC-only (no cluster term).

## Where truth lives

- Design: [`../../docs/superpowers/specs/2026-07-10-bitget-aeron-clickhouse-sample-design.md`](../../docs/superpowers/specs/2026-07-10-bitget-aeron-clickhouse-sample-design.md)
- Samples map: [`../README.md`](../README.md)

## Non-goals

- Cluster leadership / NewLeader handling → use `cluster-ha-orderbook`
- Dual-pinning a second rusteron major/minor line in this crate
