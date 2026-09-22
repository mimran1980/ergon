//! Config (`recording/v1`) validation and effective-policy tests:
//! default-off, exact-instance precedence, removal, invalid input, and
//! rollback semantics.

use ergo_clickhouse_persist::config::{Effective, parse_duration, validate};
use std::time::Duration;

const PERMANENT: &[&str] = &["trades", "raw_exchange_messages"];

fn base() -> String {
    r#"
api_version: recording/v1
defaults:
  temporary_enabled: false
  row_ttl: 24h
  idle_table_ttl: 7d
rules:
  - process: market-recorder
    instance: "*"
    table: book_debug
    enabled: false
    row_ttl: 24h
    idle_table_ttl: 7d
"#
    .to_string()
}

#[test]
fn default_off_is_the_invariant() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = validate(base().as_bytes(), PERMANENT)?;
    let e = cfg.resolve("market-recorder", "binance-0", "book_debug");
    assert_eq!(
        e,
        Effective {
            enabled: false,
            row_ttl: Duration::from_secs(86_400),
            idle_table_ttl: Duration::from_secs(7 * 86_400)
        }
    );
    // A table absent from config is disabled without evaluating anything.
    let e = cfg.resolve("market-recorder", "binance-0", "never_listened");
    assert!(!e.enabled);
    Ok(())
}

#[test]
fn exact_instance_precedes_wildcard() -> Result<(), Box<dyn std::error::Error>> {
    let mut y = base();
    y.push_str(
        "  - process: market-recorder\n    instance: binance-0\n    table: book_debug\n    enabled: true\n    row_ttl: 1h\n",
    );
    let cfg = validate(y.as_bytes(), PERMANENT)?;
    let exact = cfg.resolve("market-recorder", "binance-0", "book_debug");
    assert!(exact.enabled);
    assert_eq!(exact.row_ttl, Duration::from_secs(3600));
    let other = cfg.resolve("market-recorder", "bybit-0", "book_debug");
    assert!(
        !other.enabled,
        "wildcard rule is disabled; exact rule only enables binance-0"
    );
    Ok(())
}

#[test]
fn enabling_requires_an_explicit_rule() -> Result<(), Box<dyn std::error::Error>> {
    let y = r#"
api_version: recording/v1
defaults:
  temporary_enabled: true
rules: []
"#;
    assert!(validate(y.as_bytes(), PERMANENT).is_err());
    Ok(())
}

#[test]
fn permanent_table_rules_are_rejected() {
    let y = r#"
api_version: recording/v1
rules:
  - process: market-recorder
    instance: "*"
    table: trades
    enabled: false
"#;
    assert!(validate(y.as_bytes(), PERMANENT).is_err());
}

#[test]
fn invalid_edits_are_rejected_whole() {
    // invalid duration
    let y = r#"
api_version: recording/v1
rules:
  - process: m
    instance: "*"
    table: book_debug
    enabled: true
    row_ttl: 0h
"#;
    assert!(validate(y.as_bytes(), PERMANENT).is_err());
    // duplicate selector
    let y = r#"
api_version: recording/v1
rules:
  - process: m
    instance: "*"
    table: book_debug
    enabled: true
  - process: m
    instance: "*"
    table: book_debug
    enabled: false
"#;
    assert!(validate(y.as_bytes(), PERMANENT).is_err());
    // wildcard table
    let y = r#"
api_version: recording/v1
rules:
  - process: m
    instance: "*"
    table: "*"
    enabled: true
"#;
    assert!(validate(y.as_bytes(), PERMANENT).is_err());
    // unsupported field
    let y = r#"
api_version: recording/v1
rules:
  - process: m
    instance: "*"
    table: book_debug
    enabled: true
    mystery: 1
"#;
    assert!(validate(y.as_bytes(), PERMANENT).is_err());
    // wrong api version
    let y = "api_version: recording/v2\nrules: []\n";
    assert!(validate(y.as_bytes(), PERMANENT).is_err());
    // garbage
    assert!(validate(b"\x00\x01 not yaml", PERMANENT).is_err());
}

#[test]
fn removal_restores_default_disabled() -> Result<(), Box<dyn std::error::Error>> {
    let enabled = r#"
api_version: recording/v1
rules:
  - process: m
    instance: "*"
    table: book_debug
    enabled: true
"#;
    let cfg = validate(enabled.as_bytes(), PERMANENT)?;
    assert!(cfg.resolve("m", "i", "book_debug").enabled);
    // rule removed -> back to disabled (the disabled default above is what
    // an editor writes after removal; the file without the rule at all is
    // the same for that table).
    let removed = r#"
api_version: recording/v1
rules: []
"#;
    let cfg = validate(removed.as_bytes(), PERMANENT)?;
    assert!(!cfg.resolve("m", "i", "book_debug").enabled);
    Ok(())
}

#[test]
fn future_tables_are_allowed_but_permanent_names_are_not() -> Result<(), Box<dyn std::error::Error>>
{
    let y = r#"
api_version: recording/v1
rules:
  - process: m
    instance: "*"
    table: pipeline_debug_not_yet_registered
    enabled: true
"#;
    let cfg = validate(y.as_bytes(), PERMANENT)?;
    assert!(
        cfg.resolve("m", "i", "pipeline_debug_not_yet_registered")
            .enabled
    );
    Ok(())
}

#[test]
fn digest_changes_with_content() -> Result<(), Box<dyn std::error::Error>> {
    let a = validate(base().as_bytes(), PERMANENT)?;
    let mut edited = base();
    edited = edited.replace("row_ttl: 24h", "row_ttl: 1h");
    let b = validate(edited.as_bytes(), PERMANENT)?;
    assert_ne!(a.digest, b.digest);
    // unchanged bytes -> same digest (no API writes)
    let c = validate(base().as_bytes(), PERMANENT)?;
    assert_eq!(a.digest, c.digest);
    Ok(())
}

#[test]
fn durations_parse() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(parse_duration("250ms")?, Duration::from_millis(250));
    assert_eq!(parse_duration("90s")?, Duration::from_secs(90));
    assert_eq!(parse_duration("10m")?, Duration::from_secs(600));
    assert_eq!(parse_duration("24h")?, Duration::from_secs(86_400));
    assert_eq!(parse_duration("7d")?, Duration::from_secs(7 * 86_400));
    assert!(parse_duration("0s").is_err());
    assert!(parse_duration("-5s").is_err());
    assert!(parse_duration("5x").is_err());
    Ok(())
}
