//! A feed as the lab's directory names one, for the ingester's tests: its
//! publication, its archive's spy, and how a subscriber reaches it.

use ergon_runtime::directory::{ArchiveAddr, FeedAddr};
use ergon_runtime_server::RecordedFeed;

/// One feed of one publishing service, on this machine.
pub struct TestFeed {
    pub service: String,
    pub port: u16,
    pub stream_id: i32,
}

impl TestFeed {
    pub fn new(service: &str, port: u16, stream_id: i32) -> Self {
        Self {
            service: service.into(),
            port,
            stream_id,
        }
    }

    /// The publication this node's application opens.
    pub fn publication(&self) -> String {
        format!(
            "aeron:udp?control=127.0.0.1:{}|control-mode=dynamic|fc=max|ssc=true|term-length=1m|mtu=1408",
            self.port
        )
    }

    /// What the ingester records it by.
    pub fn recorded(&self) -> RecordedFeed {
        RecordedFeed {
            service: self.service.clone(),
            kind: "md".into(),
            stream_id: self.stream_id,
            spy: format!(
                "aeron-spy:aeron:udp?control=127.0.0.1:{}|control-mode=dynamic",
                self.port
            ),
        }
    }

    /// Where a subscriber on this machine takes it, live, from `host`.
    pub fn live(&self, host: &str) -> String {
        format!(
            "aeron:udp?endpoint=127.0.0.1:0|control={host}:{}|control-mode=dynamic",
            self.port
        )
    }

    /// Its address through the archive at `localhost:archive_port`.
    pub fn addr(&self, archive_port: u16) -> FeedAddr {
        FeedAddr {
            stream_id: self.stream_id,
            live: self.live("localhost"),
            archive: Some(ArchiveAddr {
                host: "localhost".into(),
                port: archive_port,
                publisher_port: self.port,
            }),
        }
    }
}
