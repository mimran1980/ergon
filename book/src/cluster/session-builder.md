# SessionBuilder

`SessionBuilder` is the supported configuration entry point:

```rust,ignore
use std::sync::Arc;
use std::time::Duration;
use ergo_aeron_cluster::{SessionBuilder, StaticCredentials};

fn main() -> Result<(), ergo_aeron_cluster::ClusterError> {
    let session = SessionBuilder::default()
        .ingress_channel("aeron:udp?endpoint=localhost:9010")?
        .egress_channel("aeron:udp?endpoint=localhost:9020")?
        .credentials(Arc::new(StaticCredentials::from_utf8("user:pass")))
        .message_timeout(Duration::from_secs(5))?;

    session.validate()?;
    let mut client = session.connect("/path/to/aeron-dir")?;
    client.offer(b"application payload")?;
    client.close()
}
```

Connection remains poll-driven internally; the crate does not require Tokio or
another async runtime. `AsyncClusterConnect` is a poll-driven Aeron state
machine, not a Rust `Future`. When the application owns the poll loop, the
shortest supported path is `SessionBuilder` → `connect_async` → `default_idle`
→ `poll_connect_until_done` → `finish`:

```rust,ignore
use ergo_aeron_cluster::{default_idle, poll_connect_until_done, SessionBuilder};

fn connect(aeron_dir: &str) -> Result<ergo_aeron_cluster::AeronCluster, ergo_aeron_cluster::ClusterError> {
    let builder = SessionBuilder::default()
        .ingress_channel("aeron:udp?endpoint=localhost:9010")?
        .egress_channel("aeron:udp?endpoint=localhost:9020")?;
    let mut connecting = builder.connect_async(aeron_dir);
    let mut idle = default_idle();
    poll_connect_until_done(&mut connecting, &mut idle)?;
    connecting.finish()
}
```

Drive `poll` / `step` yourself when you need to inspect connect steps or plug
in a custom idle strategy.

## Multi-member ingress: `ingress_endpoints`

For first-connect against a cluster whose current leader isn't known ahead of
time, use `ingress_endpoints` instead of (or alongside) `ingress_channel`:

```rust,ignore
let session = SessionBuilder::default()
    .ingress_endpoints("0=host-a:9010,1=host-b:9010,2=host-c:9010")?
    .egress_channel("aeron:udp?endpoint=localhost:9020")?;
```

The grammar is `member_id=host:port` pairs separated by commas — no `aeron:`
prefix, member IDs need not be sorted or contiguous. The setter parses and
validates eagerly: a missing `=`, non-numeric member ID, empty endpoint,
empty map, or duplicate member ID fails immediately at the call site rather
than at `validate()` or the first `poll()`.
