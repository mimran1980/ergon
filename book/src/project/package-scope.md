# Package Scope

crates.io ships generator source, the crate README, and the manifest.

Not published: tests, fixtures, samples, benches, the book, Java harnesses,
and upstream reference trees. Those live on GitHub.

| Crate | crates.io |
|-------|-----------|
| `ergo-sbe` | generator |
| `ergo-aeron-cluster` | Cluster client |

Samples and `ergo-sbe-benchmarks` are `publish = false`. Cluster codecs stay
crate-private; use `AeronCluster`, not `cluster_codec_types`.

Apache-2.0.
