# LZ4 compression + TLS/SSL support

**Blocked by:** none
**Severity:** MEDIUM

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

## Acceptance criteria

- [ ] `ClickhouseSinkBuilder::compression()` accepts `None` / `Lz4`
- [ ] LZ4 compression enabled on `clickhouse::Client` when configured
- [ ] TLS enabled automatically for `https://` ClickHouse URLs
- [ ] `tls_skip_verify()` for dev environments with self-signed certs
- [ ] `tls_ca_cert()` for custom CA bundles
- [ ] Integration test with LZ4 verifies compression roundtrip (needs Docker ClickHouse, same pattern as existing `#[ignore]` integration tests)
- [x] No regression in existing tests
