//! Recording config (`recording/v1`) parsing and validation.
//!
//! `recording.yaml` is the source of truth for temporary-table policy.
//! Validation is shared by every consumer (watcher CLI, producers,
//! ingester): an invalid revision is rejected whole — a valid revision can
//! never partially activate its rules.

use crate::protocol::limits;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

/// Parsed and validated recording configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordingConfig {
    /// Always `recording/v1`.
    pub api_version: String,
    /// Global defaults.
    pub defaults: Defaults,
    /// Ordered rules; exact-instance rules take precedence over wildcards.
    pub rules: Vec<Rule>,
    /// Canonical digest of the validated content.
    pub digest: u64,
}

/// Defaults block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Defaults {
    /// Must be `false` in `recording/v1`; enabling requires an explicit rule.
    pub temporary_enabled: bool,
    /// Default row retention for temporary tables.
    pub row_ttl: Duration,
    /// Default idle-table cleanup for temporary tables.
    pub idle_table_ttl: Duration,
}

/// One temporary-table rule.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    /// Exact process name.
    pub process: String,
    /// Exact instance name or `"*"`.
    pub instance: String,
    /// Exact table name; no wildcard tables exist in v1.
    pub table: String,
    /// Whether recording is enabled for matches.
    pub enabled: bool,
    /// Row retention override.
    pub row_ttl: Option<Duration>,
    /// Idle-table cleanup override.
    pub idle_table_ttl: Option<Duration>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigDoc {
    api_version: String,
    #[serde(default)]
    defaults: DefaultsDoc,
    #[serde(default)]
    rules: Vec<RuleDoc>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct DefaultsDoc {
    #[serde(default)]
    temporary_enabled: Option<bool>,
    #[serde(default)]
    row_ttl: Option<String>,
    #[serde(default)]
    idle_table_ttl: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct RuleDoc {
    process: String,
    instance: String,
    table: String,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    row_ttl: Option<String>,
    #[serde(default)]
    idle_table_ttl: Option<String>,
}

/// Config validation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    /// Human-readable, stable message for status surfaces.
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

fn err(message: impl Into<String>) -> ConfigError {
    ConfigError {
        message: message.into(),
    }
}

/// Parse a duration like `250ms`, `24h`, `7d`, `90s`, `10m`.
pub fn parse_duration(s: &str) -> Result<Duration, ConfigError> {
    let s = s.trim();
    let (num, unit) = s.split_at(
        s.find(|c: char| c.is_alphabetic())
            .ok_or_else(|| err(format!("duration missing unit: {s}")))?,
    );
    let v: u64 = num
        .parse()
        .map_err(|_| err(format!("invalid duration number: {s}")))?;
    if v == 0 {
        return Err(err(format!("non-positive duration: {s}")));
    }
    let d = match unit {
        "ms" => Duration::from_millis(v),
        "s" => Duration::from_secs(v),
        "m" => Duration::from_secs(v * 60),
        "h" => Duration::from_secs(v * 3600),
        "d" => Duration::from_secs(v * 86_400),
        _ => return Err(err(format!("unsupported duration unit: {unit}"))),
    };
    Ok(d)
}

fn fnv(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Names of known permanent tables for the consumer; a rule naming one of
/// these is rejected (permanent tables cannot be disabled or made temporary).
pub type PermanentTables<'a> = &'a [&'a str];

/// Validate `recording/v1` bytes. Structural validation only; consumers
/// additionally pass their known permanent-table names.
pub fn validate(
    bytes: &[u8],
    permanent: PermanentTables<'_>,
) -> Result<RecordingConfig, ConfigError> {
    if bytes.len() > limits::MAX_CONFIG_BYTES {
        return Err(err(format!(
            "config exceeds {} bytes",
            limits::MAX_CONFIG_BYTES
        )));
    }
    let doc: ConfigDoc = serde_yaml::from_slice(bytes).map_err(|e| err(format!("yaml: {e}")))?;
    if doc.api_version != "recording/v1" {
        return Err(err(format!("unsupported api_version: {}", doc.api_version)));
    }
    if doc.defaults.temporary_enabled == Some(true) {
        return Err(err(
            "defaults.temporary_enabled must be false; enabling a temporary \
             table requires an explicit matching enabled rule",
        ));
    }
    let row_ttl = match &doc.defaults.row_ttl {
        Some(s) => parse_duration(s)?,
        None => Duration::from_secs(24 * 3600),
    };
    let idle_table_ttl = match &doc.defaults.idle_table_ttl {
        Some(s) => parse_duration(s)?,
        None => Duration::from_secs(7 * 86_400),
    };
    if doc.rules.len() > limits::MAX_CONFIG_RULES {
        return Err(err(format!(
            "rule count exceeds {}",
            limits::MAX_CONFIG_RULES
        )));
    }
    let mut rules = Vec::with_capacity(doc.rules.len());
    let mut seen: HashMap<(String, String, String), ()> = HashMap::new();
    for (i, r) in doc.rules.into_iter().enumerate() {
        if r.process.is_empty() || r.table.is_empty() {
            return Err(err(format!("rule {i}: process/table must be exact names")));
        }
        if r.instance.is_empty() {
            return Err(err(format!(
                "rule {i}: instance must be an exact name or \"*\""
            )));
        }
        if r.table == "*" {
            return Err(err("wildcard table selectors are not supported in v1"));
        }
        if permanent.iter().any(|p| *p == r.table) {
            return Err(err(format!(
                "rule {i}: {r:?} names a permanent table; permanent tables \
                 cannot be disabled or converted by this config"
            )));
        }
        let key = (r.process.clone(), r.instance.clone(), r.table.clone());
        if seen.insert(key, ()).is_some() {
            return Err(err(format!(
                "rule {i}: duplicate selector ({}/{}/{})",
                r.process, r.instance, r.table
            )));
        }
        if r.enabled.is_none() {
            return Err(err(format!("rule {i}: missing enabled")));
        }
        let row_ttl = match &r.row_ttl {
            Some(s) => Some(parse_duration(s)?),
            None => None,
        };
        let idle_table_ttl = match &r.idle_table_ttl {
            Some(s) => Some(parse_duration(s)?),
            None => None,
        };
        rules.push(Rule {
            process: r.process,
            instance: r.instance,
            table: r.table,
            enabled: r.enabled.unwrap_or(false),
            row_ttl,
            idle_table_ttl,
        });
    }
    let digest = fnv(bytes);
    Ok(RecordingConfig {
        api_version: doc.api_version,
        defaults: Defaults {
            temporary_enabled: false,
            row_ttl,
            idle_table_ttl,
        },
        rules,
        digest,
    })
}

/// Effective decision for one (process, instance, table) triple.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effective {
    /// Whether recording is enabled.
    pub enabled: bool,
    /// Row retention.
    pub row_ttl: Duration,
    /// Idle-table cleanup.
    pub idle_table_ttl: Duration,
}

impl RecordingConfig {
    /// Resolve the effective policy for a triple.
    ///
    /// Absent from config means disabled — an invariant, not a convention.
    #[must_use]
    pub fn resolve(&self, process: &str, instance: &str, table: &str) -> Effective {
        let mut best: Option<&Rule> = None;
        for r in &self.rules {
            if r.process != process || r.table != table {
                continue;
            }
            if r.instance == instance {
                best = Some(r);
                break; // exact instance wins
            }
            if r.instance == "*" && best.is_none() {
                best = Some(r);
            }
        }
        match best {
            Some(r) if r.enabled => Effective {
                enabled: true,
                row_ttl: r.row_ttl.unwrap_or(self.defaults.row_ttl),
                idle_table_ttl: r.idle_table_ttl.unwrap_or(self.defaults.idle_table_ttl),
            },
            _ => Effective {
                enabled: false,
                row_ttl: self.defaults.row_ttl,
                idle_table_ttl: self.defaults.idle_table_ttl,
            },
        }
    }
}
