//! Shared blocking `ClickHouse` HTTP transport. Queries can stream `RowBinary`.

use std::io::Read;
use std::time::Duration;

/// Transport or server error.
#[derive(Debug)]
pub struct Error(String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

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

    /// Run a statement and return the response body.
    ///
    /// # Errors
    ///
    /// The HTTP request failed, or `ClickHouse` returned an error body.
    pub fn query(&self, sql: &str) -> Result<String, Error> {
        self.post(sql, &[])
    }

    /// `INSERT INTO table (columns) FORMAT RowBinary` with `rows` as the body.
    ///
    /// # Errors
    /// The transport or server refused the insert.
    pub fn insert(&self, table: &str, columns: &[&str], rows: &[u8]) -> Result<(), Error> {
        self.insert_token(table, columns, rows, "")
    }

    /// [`Self::insert`] identified by `token`. `ClickHouse` drops the insert
    /// when that token was already used for this table, so a retry of the
    /// same batch does not add rows. An empty token keeps the content checksum.
    ///
    /// # Errors
    /// The transport or server refused the insert.
    pub fn insert_token(
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

    /// Median of the stored per-window p50 latency summaries for a route.
    /// This is a cold startup query; persisted summaries do not retain pooled samples.
    ///
    /// # Errors
    /// The query failed, no route observations exist, or its result is invalid.
    pub fn route_delay(
        &self,
        venue: &str,
        from_region: &str,
        agent_region: &str,
    ) -> Result<crate::clock::Nanos, Error> {
        let literal = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");
        let sql = format!(
            "SELECT toInt64(round(median(p50))) FROM {}.metrics_histogram WHERE name = 'md_to_engine_ns' AND labels['venue'] = '{}' AND labels['from'] = '{}' AND app = '{}' HAVING count() > 0 FORMAT TabSeparatedRaw",
            quote(&self.database),
            literal(venue),
            literal(from_region),
            literal(&format!("engine-{agent_region}"))
        );
        let text = self.query(&sql)?;
        let value = text
            .trim()
            .parse::<i64>()
            .map_err(|e| Error(format!("route has no valid median latency: {e}")))?;
        if value < 0 {
            return Err(Error("route latency must be nonnegative".into()));
        }
        Ok(crate::clock::Nanos(value))
    }

    /// Run SQL and return a streaming binary response.
    ///
    /// # Errors
    /// The transport or server refused the query.
    pub fn reader(&self, sql: &str) -> Result<Box<dyn Read + Send + Sync>, Error> {
        self.response(sql, &[]).map(ureq::Response::into_reader)
    }

    fn post(&self, sql: &str, body: &[u8]) -> Result<String, Error> {
        self.response(sql, body)?
            .into_string()
            .map_err(|e| Error(e.to_string()))
    }

    fn response(&self, sql: &str, body: &[u8]) -> Result<ureq::Response, Error> {
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
            Ok(r) => Ok(r),
            Err(ureq::Error::Status(code, r)) => {
                let text = r.into_string().unwrap_or_default();
                Err(Error(format!("HTTP {code}: {}", text.trim())))
            }
            Err(e) => Err(Error(e.to_string())),
        }
    }
}

fn quote(ident: &str) -> String {
    format!("`{}`", ident.replace('`', "\\`"))
}
