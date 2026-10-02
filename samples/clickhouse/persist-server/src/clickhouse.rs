//! A small ClickHouse HTTP client plus the table schema sync.

use std::time::Duration;

use persist_client::TableKind;

use crate::Error;
use crate::table::{Column, Shape};

/// How many recent inserts a table remembers. Plain MergeTree deduplicates
/// nothing until this is set; a retry of one of those inserts is dropped.
pub(crate) const DEDUP_WINDOW: u64 = 1000;

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

    /// The same server and user, in `database`.
    #[must_use]
    pub fn in_database(&self, database: &str) -> Self {
        Self {
            database: database.to_string(),
            ..self.clone()
        }
    }

    /// `CREATE VIEW IF NOT EXISTS name AS SELECT * FROM table WHERE name = 'name'`
    /// in [`Self::database`]: one metric's rows under its own name. Returns
    /// the DDL. A table or view already called `name` is left as it is.
    pub(crate) fn create_metric_view(&self, name: &str, table: &str) -> Result<String, Error> {
        let ddl = format!(
            "CREATE VIEW IF NOT EXISTS {db}.{} AS SELECT * FROM {db}.{} WHERE name = '{}'",
            quote(name),
            quote(table),
            name.replace('\\', "\\\\").replace('\'', "\\'"),
            db = quote(&self.database),
        );
        self.query(&ddl)?;
        Ok(ddl)
    }

    /// `CREATE DATABASE IF NOT EXISTS` for [`Self::database`].
    pub(crate) fn create_database(&self) -> Result<(), Error> {
        let sql = format!("CREATE DATABASE IF NOT EXISTS {}", quote(&self.database));
        self.query(&sql).map(drop)
    }

    /// Run a statement and return the response body.
    pub fn query(&self, sql: &str) -> Result<String, Error> {
        self.post(sql, &[])
    }

    /// `INSERT INTO table (columns) FORMAT RowBinary` with `rows` as the body.
    pub(crate) fn insert(&self, table: &str, columns: &[&str], rows: &[u8]) -> Result<(), Error> {
        self.insert_token(table, columns, rows, "")
    }

    /// [`Self::insert`] identified by `token`. ClickHouse drops the insert
    /// when that token was already used for this table, so a retry of the
    /// same batch does not add rows. An empty token keeps the content checksum.
    pub(crate) fn insert_token(
        &self,
        table: &str,
        columns: &[&str],
        rows: &[u8],
        token: &str,
    ) -> Result<(), Error> {
        let cols: Vec<String> = columns.iter().map(|c| quote(c)).collect();
        let settings = if token.is_empty() {
            String::new()
        } else {
            format!(
                " SETTINGS insert_deduplication_token = '{}'",
                token.replace('\'', "\\'")
            )
        };
        let sql = format!(
            "INSERT INTO {}.{} ({}){settings} FORMAT RowBinary",
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
    pub(crate) fn sync(&self, table: &Shape, kind: TableKind) -> Result<Sync, Error> {
        let wanted = &table.columns;
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
        if let Some(ddl) = self.ensure_dedup(&table.name)? {
            sync.applied.push(ddl);
        }
        for Column { name, ch_type } in wanted {
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

    /// `CREATE TABLE`: MergeTree, partitioned by day, plus an `inserted_at`
    /// column that ClickHouse fills.
    #[must_use]
    pub fn create_sql(&self, table: &Shape) -> String {
        let mut cols: Vec<String> = table
            .columns
            .iter()
            .map(|c| format!("    {} {}", quote(&c.name), c.ch_type))
            .collect();
        cols.push("    inserted_at DateTime64(3, 'UTC') DEFAULT now64(3)".into());
        let order: Vec<String> = table.order_by.iter().map(|c| quote(c)).collect();
        let partition = table
            .partition
            .as_ref()
            .map(|ts| format!("\nPARTITION BY toDate({})", quote(ts)))
            .unwrap_or_default();
        format!(
            "CREATE TABLE IF NOT EXISTS {}.{} (\n{}\n)\nENGINE = MergeTree{partition}\nORDER BY ({})\nSETTINGS non_replicated_deduplication_window = {DEDUP_WINDOW}",
            quote(&self.database),
            quote(&table.name),
            cols.join(",\n"),
            order.join(", "),
        )
    }

    /// Give an existing table the dedup window [`Self::create_sql`] sets.
    fn ensure_dedup(&self, table: &str) -> Result<Option<String>, Error> {
        let sql = format!(
            "SELECT engine_full FROM system.tables WHERE database = '{}' AND name = '{}' FORMAT TabSeparatedRaw",
            self.database.replace('\'', "\\'"),
            table.replace('\'', "\\'"),
        );
        if has_dedup_window(&self.query(&sql)?) {
            return Ok(None);
        }
        let ddl = format!(
            "ALTER TABLE {}.{} MODIFY SETTING non_replicated_deduplication_window = {DEDUP_WINDOW}",
            quote(&self.database),
            quote(table)
        );
        self.query(&ddl)?;
        Ok(Some(ddl))
    }
}

fn has_dedup_window(engine_full: &str) -> bool {
    let engine = engine_full.replace(' ', "");
    let key = "non_replicated_deduplication_window=";
    let Some(at) = engine.find(key) else {
        return false;
    };
    let rest = &engine[at + key.len()..];
    let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    n == DEDUP_WINDOW.to_string()
}

/// The token of piece `i` of a batch named `token`, so a retry drops only
/// the pieces that already landed. The first piece keeps the batch's own
/// token, which is what a batch inserted whole before pieces existed used.
pub(crate) fn piece_token(token: &str, i: usize) -> std::borrow::Cow<'_, str> {
    if i == 0 {
        token.into()
    } else {
        format!("{token}#{i}").into()
    }
}

fn quote(ident: &str) -> String {
    format!("`{}`", ident.replace('`', "\\`"))
}

fn same_type(a: &str, b: &str) -> bool {
    a.replace(' ', "") == b.replace(' ', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_window_matches_only_that_setting() {
        assert!(has_dedup_window(
            "MergeTree SETTINGS non_replicated_deduplication_window = 1000"
        ));
        assert!(!has_dedup_window(
            "MergeTree SETTINGS non_replicated_deduplication_window = 10000"
        ));
        assert!(!has_dedup_window("MergeTree ORDER BY tuple()"));
    }

    #[test]
    fn each_piece_has_its_own_token_and_the_first_keeps_the_batchs() {
        assert_eq!(piece_token("3:4096", 0), "3:4096");
        assert_eq!(piece_token("3:4096", 1), "3:4096#1");
        assert_eq!(piece_token("3:4096", 12), "3:4096#12");
    }
}
