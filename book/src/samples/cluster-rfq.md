# Cluster RFQ

RFQ / auction protocol codecs with cluster-backed examples. Demonstrates
e-sniping timer, auction state machine, and multi-participant message flows
over Aeron Cluster.

- `rfq_client` — sends RFQ, receives quotes, places order.
- `auction_client` — auction lifecycle with e-sniping.
- `rfq_roundtrip` — encode/decode parity for all RFQ message types.

Source: [`samples/cluster-rfq/`](https://github.com/mimran1980/ergon/tree/main/samples/cluster-rfq)

```sh
cargo build --manifest-path samples/cluster-rfq/Cargo.toml --examples
cargo run --manifest-path samples/cluster-rfq/Cargo.toml --example rfq_roundtrip
```
