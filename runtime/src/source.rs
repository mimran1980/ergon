//! Who recorded a row.
//!
//! Every frame an application publishes carries its source id in the Aeron
//! frame's reserved value, which costs no bytes and survives the archive's
//! record and replay. A `Source` message names the id once, and the ingester
//! writes the names into every row as the `host`, `pod` and `app` columns.

use crate::event::codec;

/// Template id of the `Source` message.
pub const SOURCE_TEMPLATE_ID: u16 = codec::SourceEncoder::TEMPLATE_ID;

/// An application's identity: one per [`Bus`](crate::bus::Bus).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// Stamped into every frame's reserved value.
    pub id: u64,
    /// UNIX ns when the application connected.
    pub started: u64,
    /// Its Aeron client id (0 before it connects).
    pub client: i64,
    /// The machine: in Kubernetes, the node.
    pub host: String,
    /// The pod, or the process name outside Kubernetes.
    pub pod: String,
    /// The application's name.
    pub app: String,
}

impl Source {
    /// The source of this process: its id hashes the names, the process id
    /// and the start time, so each run of each application has its own.
    #[must_use]
    pub fn new(host: &str, pod: &str, app: &str) -> Self {
        let started = crate::event::now_ns();
        let mut key = Vec::new();
        for part in [host, pod, app] {
            key.extend_from_slice(part.as_bytes());
            key.push(0);
        }
        key.extend_from_slice(&std::process::id().to_le_bytes());
        key.extend_from_slice(&started.to_le_bytes());
        Self {
            // 0 means "no source": never hand it out.
            id: crate::event::fnv64(&key).max(1),
            started,
            client: 0,
            host: host.to_owned(),
            pod: pod.to_owned(),
            app: app.to_owned(),
        }
    }

    /// This source as a `Source` message, header included.
    ///
    /// # Errors
    ///
    /// The codec rejected a name.
    pub fn message(&self) -> Result<Vec<u8>, crate::event::EncodeError> {
        let len = codec::SourceEncoder::compute_length_with_header(
            self.host.len(),
            self.pod.len(),
            self.app.len(),
        );
        crate::event::owned_frame(len, |message| {
            Ok(codec::SourceEncoder::wrap_and_apply_header(message, 0)
                .fixed(&codec::SourceFixedFields {
                    source: self.id,
                    started: self.started,
                    client: self.client,
                })
                .host(self.host.as_bytes())
                .and_then(|m| m.pod(self.pod.as_bytes()))
                .and_then(|m| m.app(self.app.as_bytes()))?
                .encoded_length_with_header())
        })
    }

    /// A source from its `Source` message (header included); `None` when
    /// the message is malformed.
    #[must_use]
    pub fn decode(message: &[u8]) -> Option<Self> {
        let d = codec::SourceDecoder::decode(message, 0).ok()?;
        Some(Self {
            id: d.source(),
            started: d.started(),
            client: d.client(),
            host: d.host_as_str().ok()?.to_owned(),
            pod: d.pod_as_str().ok()?.to_owned(),
            app: d.app_as_str().ok()?.to_owned(),
        })
    }
}

/// This machine's name: `NODE_NAME` (a Kubernetes node, from the downward
/// API), else the kernel's host name, else `hostname`.
#[must_use]
pub fn host_name() -> String {
    std::env::var("NODE_NAME")
        .ok()
        .or_else(|| std::fs::read_to_string("/proc/sys/kernel/hostname").ok())
        .or_else(|| {
            let out = std::process::Command::new("hostname").output().ok()?;
            String::from_utf8(out.stdout).ok()
        })
        .map(|h| h.trim().to_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_round_trips_through_its_message() -> Result<(), Box<dyn std::error::Error>> {
        let mut source = Source::new("node-1", "recorder-binance-7d9f", "binance");
        source.client = 42;
        let decoded = Source::decode(&source.message()?).ok_or("undecodable")?;
        assert_eq!(decoded, source);
        assert_ne!(source.id, 0);
        assert_ne!(
            Source::new("node-1", "recorder-binance-7d9f", "bybit").id,
            source.id
        );
        Ok(())
    }
}
