//! How an application's feed names map to Aeron.
//!
//! An agent subscribes and publishes by name, `ctx.subscribe("md-binance",
//! "md")`; the application supplies a [`Directory`] that turns a name into
//! an address. The runtime knows nothing about where names come from (a
//! registry file, a service catalogue, a test's constants). A simulation
//! needs no directory: names are its feeds.

use crate::Error;

/// Where to take one feed from this node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedAddr {
    /// The feed's stream id.
    pub stream_id: i32,
    /// The channel to subscribe to it live, off the network.
    pub live: String,
    /// Where its publisher's archive records it, to catch up from after a
    /// restart or a slow spell; `None` subscribes live only.
    pub archive: Option<ArchiveAddr>,
}

/// The archive that records a feed, on its publisher's node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveAddr {
    /// A name that resolves to the publisher's node (it may move).
    pub host: String,
    /// The archive's control port there.
    pub port: u16,
    /// The publisher's control port: its recordings' channels carry it, so
    /// recordings of this feed are found by it.
    pub publisher_port: u16,
}

/// Where to publish one feed from this node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PubAddr {
    /// The publication channel.
    pub channel: String,
    /// Its stream id.
    pub stream_id: i32,
}

/// Names to addresses, as the application defines them.
pub trait Directory {
    /// Where to take `service`'s `kind` feed from the node at `host_ip`.
    ///
    /// # Errors
    ///
    /// The directory does not know the feed.
    fn feed(&self, service: &str, kind: &str, host_ip: &str) -> Result<FeedAddr, Error>;

    /// Where to publish `service`'s `kind` from the node at `host_ip`.
    ///
    /// # Errors
    ///
    /// The directory does not know the feed.
    fn publication(&self, service: &str, kind: &str, host_ip: &str) -> Result<PubAddr, Error>;
}

/// No names: feeds and outputs are opened by channel only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoDirectory;

impl Directory for NoDirectory {
    fn feed(&self, service: &str, kind: &str, _host_ip: &str) -> Result<FeedAddr, Error> {
        Err(Error::Config(format!(
            "no directory names {service}/{kind}: subscribe by channel"
        )))
    }

    fn publication(&self, service: &str, kind: &str, _host_ip: &str) -> Result<PubAddr, Error> {
        Err(Error::Config(format!(
            "no directory names {service}/{kind}: publish by channel"
        )))
    }
}
