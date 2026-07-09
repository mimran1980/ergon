# LZ4 compression + TLS/SSL support

**Blocked by:** none
**Severity:** MEDIUM
**Status: DONE (2026-07-09)** — all AC met: compression and TLS builder plumbing in place, Docker-backed compression roundtrip verified (2026-07-09). Unit tests cover builder configuration.

**Decision after deferred recheck (2026-07-08):** do not keep this blanket
deferred. Builder support is already checked off. Only Docker-backed
compression roundtrip remains, and it should be tracked as an environment-gated
integration test.

## Problem

### Compression

ClickHouse HTTP and native protocols support LZ4 compression. The
`clickhouse::Client` supports it via `.with_compression(clickhouse::Compression::Lz4)`.
Enabling compression dramatically reduces network bandwidth for inserts —
typically 3-10× smaller wire payload.

Currently the crate creates a bare `clickhouse::Client` with no compression.

### TLS/SSL

Connecting to managed ClickHouse (ClickHouse Cloud, Altinity, Tinybird) requires
TLS. The `clickhouse::Client` supports HTTPS URLs, but the builder has no TLS
configuration options (certificate bundles, custom roots, skip-verify for dev).

## Design

### Compression

Add to `ClickhouseSinkBuilder`:

```rust
pub enum PersistCompression {
    None,
    Lz4,
}

pub fn compression(mut self, c: PersistCompression) -> Self { ... }
```

Default: `Lz4`. It's almost always a win and has negligible CPU cost.

### TLS

Add to `ClickhouseSinkBuilder`:

```rust
pub fn tls_skip_verify(mut self) -> Self { ... }     // dev only
pub fn tls_ca_cert(mut self, path: &str) -> Self { ... }
```

These configure the underlying `clickhouse::Client` (or reqwest client if using
HTTP interface).

For the URL-based configuration, simply using `https://` in the URL should
enable TLS automatically (which `clickhouse::Client` already supports).
The builder options are for custom certificate configurations.

## Current verification status (2026-07-08)

Builder/unit coverage exists, but Docker-backed compression roundtrip has not
been verified in the default workspace run.

## Acceptance criteria

- [x] `ClickhouseSinkBuilder::compression()` accepts `None` / `Lz4` (sink.rs:135)
- [x] LZ4 compression enabled on `clickhouse::Client` when configured (sink.rs:69-76)
- [x] TLS via `https://` URLs (clickhouse::Client handles this automatically)
- [x] `tls_skip_verify()` for dev (sink.rs:136)
- [x] `tls_ca_cert()` for custom CA bundles (sink.rs:137)
- [x] Integration test: compression roundtrip — verified via `CLICKHOUSE_URL=http://localhost:8123 cargo test -- --ignored` (2026-07-09). LZ4 is a `clickhouse::Client` flag; our builder plumbing is tested in unit tests.
