//! A private ClickHouse database plus a `tables.yaml` in a temp directory.
//! Shared by the integration tests.

#![allow(dead_code)] // each test binary uses a different part of it

use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

use persist_server::{ClickHouse, Report, Writer};

pub fn test_url() -> String {
    std::env::var("CLICKHOUSE_TEST_URL").unwrap_or_else(|_| "http://localhost:18123".into())
}

/// A private database plus a `tables.yaml` in a temp directory.
pub struct Lab {
    pub ch: ClickHouse,
    pub config: PathBuf,
    /// This test's temp directory.
    pub dir: PathBuf,
}

impl Lab {
    pub fn new(test: &str, tables_yaml: &str) -> Result<Self, Box<dyn Error>> {
        let url = test_url();
        let db = format!("persist_test_{test}");
        let ch = ClickHouse::new(&url, "lab", "lab", &db);
        ch.query(&format!("DROP DATABASE IF EXISTS {db}"))
            .map_err(|e| format!("ClickHouse at {url} is required (run `just test`): {e}"))?;
        let dir = std::env::temp_dir().join(format!("persist-test-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let lab = Self {
            ch,
            config: dir.join("tables.yaml"),
            dir,
        };
        lab.write_config(tables_yaml)?;
        Ok(lab)
    }

    pub fn write_config(&self, text: &str) -> std::io::Result<()> {
        std::fs::write(&self.config, text)
    }

    /// A writer for this lab that re-checks its tables on every tick.
    pub fn writer(&self, schema: &str) -> Result<Writer, Box<dyn Error>> {
        Ok(Writer::new(
            schema,
            self.ch.clone(),
            &self.config,
            Duration::ZERO,
        )?)
    }

    pub fn query(&self, sql: &str) -> Result<String, Box<dyn Error>> {
        Ok(self
            .ch
            .query(&sql.replace("DB", &self.ch.database))?
            .trim_end()
            .to_string())
    }
}

pub fn clean(report: &Report) -> Result<(), String> {
    if report.errors.is_empty() {
        Ok(())
    } else {
        Err(format!("unexpected errors: {:?}", report.errors))
    }
}
