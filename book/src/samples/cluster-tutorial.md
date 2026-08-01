# Cluster Tutorial

End-to-end walkthrough: launch a test cluster, connect a session, offer
messages, poll events, send keep-alives, and close. The best starting point
for the ergo-aeron-cluster client lifecycle.

Source: [`samples/cluster-tutorial/src/main.rs`](https://github.com/mimran1980/ergon/blob/main/samples/cluster-tutorial/src/main.rs)

See also: [Cluster Client → Overview](../cluster/overview.md)

```sh
just build-aeron-jars
cargo run --manifest-path samples/cluster-tutorial/Cargo.toml
```

Requires Java 17+ and built Aeron artifacts.
