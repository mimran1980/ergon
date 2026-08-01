# SessionBuilder

`SessionBuilder` is the supported configuration entry point:

```text
use std::sync::Arc;
use std::time::Duration;
use ergo_aeron_cluster::{SessionBuilder, StaticCredentials};

fn main() -> Result<(), ergo_aeron_cluster::ClusterError> {
    let session = SessionBuilder::default()
        .ingress_channel("aeron:udp?endpoint=localhost:9010")
        .egress_channel("aeron:udp?endpoint=localhost:9020")
        .credentials(Arc::new(StaticCredentials::from_utf8("user:pass")))
        .message_timeout(Duration::from_secs(5));

    session.validate()?;
    let mut client = session.connect("/path/to/aeron-dir")?;
    client.offer(b"application payload")?;
    client.close()
}
```

Connection remains poll-driven internally; the crate does not require Tokio or
another async runtime. Use `connect_async` when the application owns the poll
loop.
