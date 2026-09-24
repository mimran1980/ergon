//! A small ClickHouse HTTP client plus the table schema sync.

use std::time::Duration;

use crate::table::{Column, Table};
use crate::{Error, TableKind};

/// Where and as whom to connect.
#[derive(Clone, Debug)]
pub struct ClickHouse {
    url: String,
    user: String,
    password: String,
    /// Database every table lives in.
    pub database: String,
    agent: ureq::Agent,
}

/// Column `inserted_at DEFAULT now64(3)`: added to every table persistence
/// creates; never written by the recorder.
pub(crate) const INSERTED_AT: &str = "inserted_at";

/// Outcome of comparing a table with the schema.
#[derive(Debug, Default)]
pub(crate) struct Sync {
    /// Per schema column: `true` when it is written on insert.
    pub include: Vec<bool>,
    /// DDL persistence ran (CREATE / ALTER ADD COLUMN).
    pub applied: Vec<String>,
    /// Columns that are not written, each with the SQL that would fix it.
    pub problems: Vec<String>,
}

impl ClickHouse {
    /// Connect settings; nothing is sent until the first query.
    #[must_use]
    pub fn new(url: &str, user: &str, password: &str, database: &str) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            user: user.to_string(),
            password: password.to_string(),
            database: database.to_string(),
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(10))
                .build(),
        }
    }

    /// Run a statement and return the response body.
    pub fn query(&self, sql: &str) -> Result<String, Error> {
        self.post(sql, &[])
    }

    /// `INSERT INTO table (columns) FORMAT RowBinary` with `rows` as the body.
    pub fn insert(&self, table: &str, columns: &[&str], rows: &[u8]) -> Result<(), Error> {
        let cols: Vec<String> = columns.iter().map(|c| quote(c)).collect();
        let sql = format!(
            "INSERT INTO {}.{} ({}) FORMAT RowBinary",
            quote(&self.database),
            quote(table),
            cols.join(", ")
        );
        self.post(&sql, rows).map(drop)
    }

    fn post(&self, sql: &str, body: &[u8]) -> Result<String, Error> {
        let request = self
            .agent
            .post(&self.url)
            .set("X-ClickHouse-User", &self.user)
            .set("X-ClickHouse-Key", &self.password);
        // With a body, the statement travels in the URL; otherwise it is the body.
        let response = if body.is_empty() {
            request.send_string(sql)
        } else {
            request.query("query", sql).send_bytes(body)
        };
        match response {
            Ok(r) => r
                .into_string()
                .map_err(|e| Error::ClickHouse(e.to_string())),
            Err(ureq::Error::Status(code, r)) => {
                let text = r.into_string().unwrap_or_default();
                Err(Error::ClickHouse(format!("HTTP {code}: {}", text.trim())))
            }
            Err(e) => Err(Error::ClickHouse(e.to_string())),
        }
    }

    /// `(name, type)` of every column, or `None` when the table does not exist.
    fn describe(&self, table: &str) -> Result<Option<Vec<(String, String)>>, Error> {
        let sql = format!(
            "SELECT name, type FROM system.columns WHERE database = '{}' AND table = '{}' ORDER BY position FORMAT TabSeparatedRaw",
            self.database.replace('\'', "\\'"),
            table.replace('\'', "\\'"),
        );
        let text = self.query(&sql)?;
        let cols: Vec<(String, String)> = text
            .lines()
            .filter_map(|l| l.split_once('\t'))
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect();
        Ok(if cols.is_empty() { None } else { Some(cols) })
    }

    /// Make `table` usable and work out which columns can be written.
    ///
    /// * missing table: `CREATE` (both kinds).
    /// * missing column: dynamic -> `ALTER ADD COLUMN`; static -> not written,
    ///   reported with the `ALTER` to run.
    /// * column with a different type: not written, reported with the
    ///   `ALTER … MODIFY COLUMN` to run (both kinds; changing a type can
    ///   rewrite data, so persistence never does it by itself).
    /// * column no longer in the schema: left alone; new rows get its default.
    pub(crate) fn sync(&self, table: &Table, kind: TableKind) -> Result<Sync, Error> {
        let wanted = table.columns();
        let Some(existing) = self.describe(&table.name)? else {
            let ddl = self.create_sql(table);
            self.query(&ddl)?;
            return Ok(Sync {
                include: vec![true; wanted.len()],
                applied: vec![ddl],
                problems: Vec::new(),
            });
        };
        let mut sync = Sync::default();
        for Column { name, ch_type } in &wanted {
            let target = format!("{}.{}", quote(&self.database), quote(&table.name));
            match existing.iter().find(|(n, _)| n == name) {
                Some((_, have)) if same_type(have, ch_type) => sync.include.push(true),
                Some((_, have)) => {
                    sync.include.push(false);
                    sync.problems.push(format!(
                        "column {name} is {have} but the schema says {ch_type}; not writing it. \
                         Fix: ALTER TABLE {target} MODIFY COLUMN {} {ch_type}",
                        quote(name)
                    ));
                }
                None => {
                    let ddl = format!(
                        "ALTER TABLE {target} ADD COLUMN IF NOT EXISTS {} {ch_type}",
                        quote(name)
                    );
                    if kind == TableKind::Dynamic {
                        self.query(&ddl)?;
                        sync.applied.push(ddl);
                        sync.include.push(true);
                    } else {
                        sync.include.push(false);
                        sync.problems.push(format!(
                            "static table is missing column {name}; not writing it. Fix: {ddl}"
                        ));
                    }
                }
            }
        }
        Ok(sync)
    }

    /// `CREATE TABLE` for a schema table: MergeTree, partitioned by day of the
    /// first timestamp, ordered by the var-data columns then that timestamp.
    #[must_use]
    pub fn create_sql(&self, table: &Table) -> String {
        let mut cols: Vec<String> = table
            .columns()
            .iter()
            .map(|c| format!("    {} {}", quote(&c.name), c.ch_type))
            .collect();
        cols.push(format!(
            "    {INSERTED_AT} DateTime64(3, 'UTC') DEFAULT now64(3)"
        ));
        let order: Vec<String> = table.order_by().iter().map(|c| quote(c)).collect();
        let partition = table
            .first_timestamp()
            .map(|ts| format!("\nPARTITION BY toDate({})", quote(&ts)))
            .unwrap_or_default();
        format!(
            "CREATE TABLE IF NOT EXISTS {}.{} (\n{}\n)\nENGINE = MergeTree{partition}\nORDER BY ({})",
            quote(&self.database),
            quote(&table.name),
            cols.join(",\n"),
            order.join(", "),
        )
    }
}

fn quote(ident: &str) -> String {
    format!("`{}`", ident.replace('`', "\\`"))
}

fn same_type(a: &str, b: &str) -> bool {
    a.replace(' ', "") == b.replace(' ', "")
}
