//! The feed registry, `config/streams.yaml`: every publishing service's
//! UDP port and streams, so feeds never clash on a node, and the channels
//! publishers, subscribers and archives use.
//!
//! A publisher binds its multi-destination-cast (MDC) control socket on
//! its own node; subscribers reach it by its Kubernetes name, which a
//! headless Service keeps pointing at whichever node it runs on (the pod
//! uses the node's network, so its IP is the node's). The media driver
//! resolves that name again when the publisher moves.
//!
//! ```yaml
//! domain: lab.svc.cluster.local
//! services:
//!   md-binance: { port: 40501, region: an1, streams: { md: 2011, tob: 2012 } }
//! kinds:
//!   md:  { reliable: true }
//!   tob: { reliable: false }
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::Error;

/// A publishing service: one pod, at one node's address at a time.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    /// Its MDC control port, unique across the registry.
    pub port: u16,
    /// Where it runs (`topology.kubernetes.io/region`).
    pub region: String,
    /// Its streams by kind, each id unique across the registry.
    pub streams: BTreeMap<String, i32>,
}

/// How a kind of stream is delivered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kind {
    /// Subscribers NAK and get every message (`md`); otherwise they take
    /// what arrives (`tob`: `reliable=false|tether=false|group=false`).
    pub reliable: bool,
    /// The node's archive records it (default `true`).
    #[serde(default = "yes")]
    pub archive: bool,
}

const fn yes() -> bool {
    true
}

/// `config/streams.yaml`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Streams {
    /// Appended to a service's name to make its DNS name.
    #[serde(default = "domain")]
    pub domain: String,
    pub services: BTreeMap<String, Service>,
    pub kinds: BTreeMap<String, Kind>,
    /// Every node's archive listens for control requests on this port
    /// (default 8010), so a service's name and it reach the archive that
    /// records the service.
    #[serde(default = "archive_port")]
    pub archive_port: u16,
}

const fn archive_port() -> u16 {
    8010
}

fn domain() -> String {
    "lab.svc.cluster.local".into()
}

/// A feed's term buffers: 1 MiB (three of them, on every node that
/// publishes or subscribes it: shared memory), with one message per
/// 1408-byte frame. A feed here moves a few KiB a second.
const PARAMS: &str = "term-length=1m|mtu=1408";

impl Streams {
    /// Parse and check the registry: ports and stream ids unique, every
    /// stream of a known kind.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let streams: Self =
            serde_yaml::from_str(text).map_err(|e| Error::Config(format!("streams.yaml: {e}")))?;
        let (mut ports, mut ids) = (BTreeSet::new(), BTreeSet::new());
        for (name, s) in &streams.services {
            if !ports.insert(s.port) {
                return Err(Error::Config(format!(
                    "streams.yaml: {name}: port {} is already another service's",
                    s.port
                )));
            }
            for (kind, id) in &s.streams {
                if !streams.kinds.contains_key(kind) {
                    return Err(Error::Config(format!(
                        "streams.yaml: {name}: no kind {kind}"
                    )));
                }
                if !ids.insert(*id) {
                    return Err(Error::Config(format!(
                        "streams.yaml: {name}: stream {id} is already another stream's"
                    )));
                }
            }
        }
        Ok(streams)
    }

    /// Read `path`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
        Self::parse(&text)
    }

    fn service(&self, name: &str) -> Result<&Service, Error> {
        self.services
            .get(name)
            .ok_or_else(|| Error::Config(format!("streams.yaml has no service {name}")))
    }

    /// The stream id of `service`'s `kind`.
    pub fn stream(&self, service: &str, kind: &str) -> Result<i32, Error> {
        self.service(service)?
            .streams
            .get(kind)
            .copied()
            .ok_or_else(|| Error::Config(format!("streams.yaml: {service} has no {kind} stream")))
    }

    /// `service`'s DNS name.
    #[must_use]
    pub fn host(&self, service: &str) -> String {
        format!("{service}.{}", self.domain)
    }

    /// The channel `service` publishes on from the node at `host_ip`.
    /// `fc=max`: the fastest subscriber sets the pace, so the publisher
    /// never waits for a slow one (which catches up from the archive).
    /// `ssc=true`: the archive's spy counts as connected, so a feed with no
    /// subscriber is still published and recorded.
    pub fn publication(&self, service: &str, host_ip: &str) -> Result<String, Error> {
        let port = self.service(service)?.port;
        Ok(format!(
            "aeron:udp?control={host_ip}:{port}|control-mode=dynamic|fc=max|ssc=true|{PARAMS}"
        ))
    }

    /// The channel to subscribe to `service`'s `kind` from the node at
    /// `host_ip`, by the service's name.
    pub fn subscription(&self, service: &str, kind: &str, host_ip: &str) -> Result<String, Error> {
        let port = self.service(service)?.port;
        let reliable = self
            .kinds
            .get(kind)
            .ok_or_else(|| Error::Config(format!("streams.yaml: no kind {kind}")))?
            .reliable;
        Ok(subscription_channel(
            &self.host(service),
            port,
            host_ip,
            reliable,
        ))
    }

    /// The channel the archive on the node at `host_ip` records
    /// `service`'s streams on: a spy on the publication, in shared memory.
    pub fn spy(&self, service: &str, host_ip: &str) -> Result<String, Error> {
        let port = self.service(service)?.port;
        Ok(format!(
            "aeron-spy:aeron:udp?control={host_ip}:{port}|control-mode=dynamic"
        ))
    }

    /// `service`'s control port.
    pub fn port(&self, service: &str) -> Result<u16, Error> {
        Ok(self.service(service)?.port)
    }

    /// Every archived stream: `(service, kind, stream id)`.
    pub fn archived(&self) -> impl Iterator<Item = (&str, &str, i32)> {
        self.services.iter().flat_map(move |(name, s)| {
            s.streams
                .iter()
                .filter(|(kind, _)| self.kinds.get(*kind).is_some_and(|k| k.archive))
                .map(move |(kind, id)| (name.as_str(), kind.as_str(), *id))
        })
    }
}

/// Follows `streams.yaml`: [`Watch::changed`] reads it at most once per
/// [`Watch::EVERY`] and hands over each new version, parsed and checked. A
/// version that does not parse is logged once and skipped, keeping the last
/// good one. In Kubernetes the file is a ConfigMap, which the kubelet swaps
/// in place when it changes.
pub struct Watch {
    path: PathBuf,
    /// The text last read, good or bad.
    last: String,
    next: Instant,
}

impl Watch {
    /// How often the file is read.
    pub const EVERY: Duration = Duration::from_secs(1);

    /// Watch `path`; its version now is the caller's.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            last: std::fs::read_to_string(&path).unwrap_or_default(),
            path,
            next: Instant::now() + Self::EVERY,
        }
    }

    /// The new version, if the file changed and parses. Until the next
    /// read is due, one clock read.
    pub fn changed(&mut self) -> Option<Streams> {
        let now = Instant::now();
        if now < self.next {
            return None;
        }
        self.next = now + Self::EVERY;
        self.read()
    }

    fn read(&mut self) -> Option<Streams> {
        // Missing for a moment while the kubelet swaps the ConfigMap.
        let text = std::fs::read_to_string(&self.path).ok()?;
        if text == self.last {
            return None;
        }
        let parsed = Streams::parse(&text);
        self.last = text;
        match parsed {
            Ok(streams) => {
                log::info!("{}: changed", self.path.display());
                Some(streams)
            }
            Err(e) => {
                log::error!("{e}; keeping the previous version");
                None
            }
        }
    }
}

/// Check that this publisher can be found by its name. Its headless
/// Service resolves to its pod's IP, while its feeds live in the node's
/// media driver, at the node's IP: the two are the same only for a pod on
/// the node's network (`hostNetwork: true`). Without it the name would point
/// subscribers at a pod address where no driver listens, and nothing would
/// say so. `POD_IP` and `HOST_IP` come from the downward API; outside
/// Kubernetes (either unset) there is nothing to check.
pub fn check_node_network() -> Result<(), Error> {
    node_network(
        std::env::var("POD_IP").ok().as_deref(),
        std::env::var("HOST_IP").ok().as_deref(),
    )
}

fn node_network(pod_ip: Option<&str>, host_ip: Option<&str>) -> Result<(), Error> {
    match (pod_ip, host_ip) {
        (Some(pod), Some(host)) if pod != host => Err(Error::Config(format!(
            "pod IP {pod} is not the node's {host}: a publisher must run on the node's network \
             (hostNetwork: true), or its name resolves to where no media driver listens"
        ))),
        _ => Ok(()),
    }
}

/// A subscription to the publisher whose control socket is `host:port`,
/// from the node at `host_ip`. Best effort adds
/// `reliable=false|tether=false|group=false`: no NAKs, and a slow
/// subscriber neither holds the publisher back nor is held itself.
#[must_use]
pub fn subscription_channel(host: &str, port: u16, host_ip: &str, reliable: bool) -> String {
    let best_effort = if reliable {
        ""
    } else {
        "|reliable=false|tether=false|group=false"
    };
    format!(
        "aeron:udp?endpoint={host_ip}:0|control={host}:{port}|control-mode=dynamic{best_effort}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const REGISTRY: &str = "
services:
  md-binance: { port: 40501, region: an1, streams: { md: 2011, tob: 2012 } }
  engine-an1: { port: 40600, region: an1, streams: { signals: 3001 } }
kinds:
  md: { reliable: true }
  tob: { reliable: false }
  signals: { reliable: true, archive: false }
";

    #[test]
    fn channels_come_from_the_registry() -> TestResult {
        let s = Streams::parse(REGISTRY)?;
        assert_eq!(s.stream("md-binance", "tob")?, 2012);
        assert_eq!(
            s.publication("md-binance", "172.18.0.2")?,
            "aeron:udp?control=172.18.0.2:40501|control-mode=dynamic|fc=max|ssc=true|term-length=1m|mtu=1408"
        );
        assert_eq!(
            s.subscription("md-binance", "md", "172.18.0.5")?,
            "aeron:udp?endpoint=172.18.0.5:0|control=md-binance.lab.svc.cluster.local:40501|control-mode=dynamic"
        );
        assert_eq!(
            s.subscription("md-binance", "tob", "172.18.0.5")?,
            "aeron:udp?endpoint=172.18.0.5:0|control=md-binance.lab.svc.cluster.local:40501|control-mode=dynamic|reliable=false|tether=false|group=false"
        );
        assert_eq!(
            s.spy("md-binance", "172.18.0.2")?,
            "aeron-spy:aeron:udp?control=172.18.0.2:40501|control-mode=dynamic"
        );
        let archived: Vec<_> = s.archived().collect();
        assert_eq!(
            archived,
            [("md-binance", "md", 2011), ("md-binance", "tob", 2012)]
        );
        Ok(())
    }

    #[test]
    fn the_lab_registry_is_valid() -> TestResult {
        let s = Streams::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../config/streams.yaml"
        ))?;
        for region in ["an1", "as1", "ew2"] {
            for service in [format!("engine-{region}"), format!("exch-sim-{region}")] {
                assert_eq!(s.services[&service].region, region, "{service}");
            }
        }
        assert!(s.archived().count() > 10);
        Ok(())
    }

    #[test]
    fn a_watch_hands_over_each_good_version_and_skips_bad_ones() -> TestResult {
        let dir = std::env::temp_dir().join(format!("streams-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("streams.yaml");
        std::fs::write(&path, REGISTRY)?;
        let mut watch = Watch::new(&path);
        assert!(watch.changed().is_none(), "not due yet");
        assert!(watch.read().is_none(), "unchanged");
        std::fs::write(&path, "services: [not a map\n")?;
        assert!(watch.read().is_none(), "a bad version is skipped");
        let next = REGISTRY.replace(
            "  engine-an1:",
            "  md-okx: { port: 40504, region: as1, streams: { md: 2041 } }\n  engine-an1:",
        );
        std::fs::write(&path, &next)?;
        let changed = watch.read().ok_or("the new version")?;
        assert_eq!(changed.stream("md-okx", "md")?, 2041);
        assert!(watch.read().is_none(), "handed over once");
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn a_publisher_must_be_on_its_nodes_network() {
        assert!(node_network(Some("172.18.0.2"), Some("172.18.0.2")).is_ok());
        assert!(node_network(None, None).is_ok(), "outside Kubernetes");
        assert!(
            node_network(Some("10.244.1.7"), Some("172.18.0.2")).is_err(),
            "an overlay pod IP: its Service would not reach the driver"
        );
    }

    #[test]
    fn clashes_and_unknown_kinds_are_refused() {
        for bad in [
            // two services on one port
            "services:\n  a: { port: 1, region: an1, streams: {} }\n  b: { port: 1, region: an1, streams: {} }\nkinds: {}\n",
            // one stream id twice
            "services:\n  a: { port: 1, region: an1, streams: { md: 5 } }\n  b: { port: 2, region: an1, streams: { md: 5 } }\nkinds:\n  md: { reliable: true }\n",
            // a stream of no kind
            "services:\n  a: { port: 1, region: an1, streams: { md: 5 } }\nkinds: {}\n",
        ] {
            assert!(Streams::parse(bad).is_err(), "accepted {bad}");
        }
    }
}
