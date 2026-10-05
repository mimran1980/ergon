//! The ClickHouse lab's conventions, apart from the runtime: which feeds
//! exist and where (`config/streams.yaml`), and how a lab application starts.
//!
//! The runtime knows a feed only by name and asks the application's
//! [`Directory`](ergon_runtime::directory::Directory) where it is; here the
//! registry is that directory. Regions are the deployment's: a service's
//! region is in the registry, and each process reads its own from `REGION`.

pub mod streams;

use ergon_runtime::rt::Runtime;

pub use streams::{Streams, Watch, check_node_network};

/// `PERSIST_STREAMS`, else `config/streams.yaml`.
#[must_use]
pub fn streams_path() -> String {
    std::env::var("PERSIST_STREAMS").unwrap_or_else(|_| "config/streams.yaml".into())
}

/// A lab application's live runtime for `schema`: the node check,
/// `streams` as its directory, and the runtime's own environment (`REGION`,
/// `IDLE`, `HOST_IP`, persist).
///
/// # Errors
///
/// The node check failed, or the runtime could not start.
pub fn runtime(schema: &str, streams: &Streams) -> Result<Runtime, ergon_runtime::app::Error> {
    check_node_network()?;
    Runtime::from_env(schema, Box::new(streams.clone()))
}

/// The median of the stored per-window p50 of `md_to_engine_ns` for the
/// route from `venue`'s region `from_region` to `agent_region`'s engine: a
/// backtest's network delay for that route.
///
/// # Errors
///
/// The query failed, the route has no observations, or the result is not a
/// nonnegative number of nanoseconds.
#[cfg(feature = "clickhouse")]
pub fn route_delay(
    client: &ergon_runtime::clickhouse::ClickHouse,
    venue: &str,
    from_region: &str,
    agent_region: &str,
) -> Result<ergon_runtime::clock::Nanos, RouteDelayError> {
    let literal = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");
    let sql = format!(
        "SELECT toInt64(round(median(p50))) FROM `{}`.metrics_histogram \
         WHERE name = 'md_to_engine_ns' AND labels['venue'] = '{}' AND labels['from'] = '{}' \
         AND app = '{}' HAVING count() > 0 FORMAT TabSeparatedRaw",
        client.database.replace('`', "\\`"),
        literal(venue),
        literal(from_region),
        literal(&format!("engine-{agent_region}"))
    );
    let value: i64 = client.query(&sql)?.trim().parse().map_err(|e| {
        RouteDelayError::Unmeasured(format!(
            "route {from_region}->{agent_region} has no median latency: {e}"
        ))
    })?;
    if value < 0 {
        return Err(RouteDelayError::Unmeasured(format!(
            "route {from_region}->{agent_region} has a negative median latency"
        )));
    }
    Ok(ergon_runtime::clock::Nanos(value))
}

/// Why [`route_delay`] has no delay for a route.
#[cfg(feature = "clickhouse")]
#[derive(Debug)]
pub enum RouteDelayError {
    /// The query failed.
    Query(ergon_runtime::clickhouse::Error),
    /// The route has no observations, or its median is not a nonnegative
    /// number of nanoseconds.
    Unmeasured(String),
}

#[cfg(feature = "clickhouse")]
impl std::fmt::Display for RouteDelayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query(e) => write!(f, "route delay: {e}"),
            Self::Unmeasured(m) => f.write_str(m),
        }
    }
}

#[cfg(feature = "clickhouse")]
impl std::error::Error for RouteDelayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Query(e) => Some(e),
            Self::Unmeasured(_) => None,
        }
    }
}

#[cfg(feature = "clickhouse")]
impl From<ergon_runtime::clickhouse::Error> for RouteDelayError {
    fn from(e: ergon_runtime::clickhouse::Error) -> Self {
        Self::Query(e)
    }
}
