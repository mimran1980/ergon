# advanced-bitget (IPC sample)

Test harness: Bitget/fixture feed → normalized **AppMessage** → Aeron **IPC** →
typed + dynamic ClickHouse. Absorbs the former `exchange-orderbook` sample
(multi-schema roundtrips, local L2 book, `#[derive(Persist)]` snapshot DTO).

## Status

**Sample / exercise Ergo — not a product.** Offline tests always; live CH via
`just samples-orderbook`. Live exchange WebSocket remains manual.

## Depends on

- `ergo-sbe`, `ergo-clickhouse-persist` (+ derive)
- `rusteron-client` / media-driver **`0.2`**
- Docker ClickHouse on `127.0.0.1:8123` for live CH tests

## Build / test

```sh
# From repo root
just test-ipc                 # offline (skip live CH)
just samples-orderbook        # live CH: clickhouse_e2e + e2e_persist snapshot

cd samples/advanced-bitget
cargo test -- --test-threads=1 --skip clickhouse
```

## Layout

| Module | Role |
|--------|------|
| `publication` | Exact-length claim encode (AppMessage → L2Book/Trade) |
| `persistence` | Foreground CH consumer path |
| `orderbook` | Local L2 book (`LocalBook`) |
| `snapshot_persist` | `OrderbookSnapshot` Persist DTO |
| `decimal` / `market` | Wire decimals |
| `bitget` / `main` | Ingest + three-thread binary |

**Nested encode guide:** [`../../sbe/docs/guide/claim-nested-encode.md`](../../sbe/docs/guide/claim-nested-encode.md).

**Cluster / NewLeader:** use [`../cluster-ha-orderbook/`](../cluster-ha-orderbook/) — not this crate.

## Non-goals

- Leadership / kill-leader HA
- Production exchange OMS
