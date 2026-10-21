# Package Scope

crates.io ships each crate's source, its README, and the manifest.

Not published: tests, fixtures, samples, benches, the book, Java harnesses,
and upstream reference trees. Those live on GitHub.

| Crate | crates.io |
|-------|-----------|
| `ergo-sbe` | generator |
| `ergo-aeron-cluster` | Cluster client |
| `ergon-tune` | host and NIC checks |
| `ergon-io` | busy-spin network I/O (WebSocket, HTTP/1.1, TLS 1.3) |
| `ergon-md-dpdk` | DPDK transport for the media driver, with its native C |

Samples and `ergo-sbe-benchmarks` are `publish = false`. Cluster codecs are
`#[doc(hidden)] pub` in `cluster_codec_types`, re-exported for integration
tests and benches only — not a stable consumer API. Use `AeronCluster` for
normal usage.

Apache-2.0.
