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
    /// Connections are kept, and a request whose kept connection the server
    /// had closed is sent once more ([`ClickHouse::pooled`]).
    pooled: bool,
}

impl ClickHouse {
    /// Connect settings; nothing is sent until the first query.
    ///
    /// No connection is kept for the next query: a backtest's queries come
    /// seconds apart, by when the server may have closed it, and a query is
    /// not retried.
    #[must_use]
    pub fn new(url: &str, user: &str, password: &str, database: &str) -> Self {
        Self::with(url, user, password, database, false)
    }

    /// For a writer that sends many requests to one server, such as the
    /// ingester: each connection is kept for the next request, which saves a
    /// round trip per request (a cross-region insert costs two instead of
    /// one). A request whose kept connection the server had already closed
    /// fails before any answer, and is sent once more on a fresh connection,
    /// so use it only for requests that are safe to repeat: queries, and
    /// inserts that `ClickHouse` deduplicates by their token or checksum.
    #[must_use]
    pub fn pooled(url: &str, user: &str, password: &str, database: &str) -> Self {
        Self::with(url, user, password, database, true)
    }

    fn with(url: &str, user: &str, password: &str, database: &str, pooled: bool) -> Self {
        // Each step is bounded, not the whole request: a backtest streams a
        // window of frames for as long as rows keep coming, while a server
        // that stops answering still times out.
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(30))
            .timeout_write(Duration::from_secs(30));
        let agent = if pooled {
            agent
        } else {
            agent.max_idle_connections(0)
        };
        Self {
            url: url.trim_end_matches('/').to_string(),
            user: user.to_string(),
            password: password.to_string(),
            database: database.to_string(),
            agent: agent.build(),
            pooled,
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
                literal(token)
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
        let response = match self.send(sql, body) {
            // A kept connection the server has since closed fails before any
            // answer: once more, on a fresh connection.
            Err(e) if self.pooled && matches!(*e, ureq::Error::Transport(_)) => {
                self.send(sql, body)
            }
            other => other,
        };
        match response.map_err(|e| *e) {
            Ok(r) => Ok(r),
            Err(ureq::Error::Status(code, r)) => {
                let text = r.into_string().unwrap_or_default();
                Err(Error(format!("HTTP {code}: {}", text.trim())))
            }
            Err(e) => Err(Error(e.to_string())),
        }
    }

    fn send(&self, sql: &str, body: &[u8]) -> Result<ureq::Response, Box<ureq::Error>> {
        let request = self
            .agent
            .post(&self.url)
            .set("X-ClickHouse-User", &self.user)
            .set("X-ClickHouse-Key", &self.password);
        // With a body, the statement travels in the URL; otherwise it is the body.
        if body.is_empty() {
            request.send_string(sql)
        } else {
            request.query("query", sql).send_bytes(body)
        }
        .map_err(Box::new)
    }
}

/// `ident` as a quoted identifier.
pub(crate) fn quote(ident: &str) -> String {
    format!("`{}`", ident.replace('`', "\\`"))
}

/// `text` escaped for a single-quoted string literal.
pub(crate) fn literal(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\'', "\\'")
}

/// The longest `RowBinary` string [`read_string`] takes.
const MAX_STRING: usize = 16 * 1024 * 1024;

/// One `RowBinary` `String`: its LEB128 length, then its bytes.
///
/// # Errors
///
/// The stream failed or ended early, or the length is malformed or over
/// 16 MiB.
pub(crate) fn read_string(reader: &mut impl Read) -> Result<Vec<u8>, Error> {
    let failed = |e: std::io::Error| Error(e.to_string());
    let mut length = 0u64;
    for shift in (0..70).step_by(7) {
        let mut byte = [0];
        reader.read_exact(&mut byte).map_err(failed)?;
        if shift == 63 && byte[0] > 1 {
            break;
        }
        length |= u64::from(byte[0] & 127) << shift;
        if byte[0] & 128 == 0 {
            let len = usize::try_from(length).map_err(|e| Error(e.to_string()))?;
            if len > MAX_STRING {
                return Err(Error("a string longer than 16 MiB".into()));
            }
            let mut bytes = vec![0; len];
            reader.read_exact(&mut bytes).map_err(failed)?;
            return Ok(bytes);
        }
    }
    Err(Error("a string length overflows".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};

    /// Takes one request off `socket`, without answering it.
    fn request(socket: &std::net::TcpStream) -> std::io::Result<()> {
        let mut request = BufReader::new(socket);
        let mut length = 0;
        loop {
            let mut line = String::new();
            request.read_line(&mut line)?;
            if line == "\r\n" {
                break;
            }
            if let Some(n) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = n.trim().parse().map_err(std::io::Error::other)?;
            }
        }
        request.read_exact(&mut vec![0; length])
    }

    #[test]
    fn a_query_never_rides_a_connection_the_server_may_have_dropped()
    -> Result<(), Box<dyn std::error::Error>> {
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: a literal loopback address, nothing to resolve"
        )]
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let client = ClickHouse::new(
            &format!("http://{}", listener.local_addr()?),
            "",
            "",
            "test",
        );
        // The first answer keeps its connection open (keep-alive). A query
        // sent on it again is dropped unanswered, as one sent just as the
        // server's idle timeout closes the connection; a query on a
        // connection of its own is answered.
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: the blocking client under test waits on it"
        )]
        let server = std::thread::spawn(move || -> std::io::Result<()> {
            let (mut kept, _) = listener.accept()?;
            request(&kept)?;
            kept.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx")?;
            kept.set_read_timeout(Some(Duration::from_millis(500)))?;
            let mut byte = [0];
            if matches!(kept.read(&mut byte), Ok(n) if n > 0) {
                return Ok(()); // the query came on the old connection: dropped
            }
            let (mut fresh, _) = listener.accept()?;
            request(&fresh)?;
            fresh.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\ny")
        });
        let mut first = String::new();
        client.reader("SELECT 1")?.read_to_string(&mut first)?;
        let mut second = String::new();
        client.reader("SELECT 2")?.read_to_string(&mut second)?;
        assert_eq!((first.as_str(), second.as_str()), ("x", "y"));
        server.join().map_err(|_| "HTTP fixture thread failed")??;
        Ok(())
    }

    #[test]
    fn a_pooled_client_keeps_its_connection_and_resends_on_one_the_server_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: a literal loopback address, nothing to resolve"
        )]
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let client = ClickHouse::pooled(
            &format!("http://{}", listener.local_addr()?),
            "",
            "",
            "test",
        );
        // Two queries ride one kept connection. The third goes out on it just
        // as the server closes it at its idle timeout: read, never answered.
        // The client sends it once more, on a second connection.
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: the blocking client under test waits on it"
        )]
        let server = std::thread::spawn(move || -> std::io::Result<()> {
            let (mut kept, _) = listener.accept()?;
            for answer in [b"x", b"y"] {
                request(&kept)?;
                kept.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n")?;
                kept.write_all(answer)?;
            }
            request(&kept)?;
            drop(kept);
            let (mut fresh, _) = listener.accept()?;
            request(&fresh)?;
            fresh.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nz")
        });
        let (first, second) = (client.query("SELECT 1")?, client.query("SELECT 2")?);
        let third = client.query("SELECT 3")?;
        assert_eq!(
            (first.as_str(), second.as_str(), third.as_str()),
            ("x", "y", "z")
        );
        server.join().map_err(|_| "HTTP fixture thread failed")??;
        Ok(())
    }

    #[test]
    fn a_response_streams_for_as_long_as_rows_keep_coming() -> Result<(), Box<dyn std::error::Error>>
    {
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: a literal loopback address, nothing to resolve"
        )]
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let client = ClickHouse::new(
            &format!("http://{}", listener.local_addr()?),
            "",
            "",
            "test",
        );
        // Twelve chunks a second apart: longer than any one read may wait,
        // as a backtest's stream of a long window is.
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: the blocking client under test waits on it"
        )]
        let server = std::thread::spawn(move || -> std::io::Result<()> {
            let (mut socket, _) = listener.accept()?;
            request(&socket)?;
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\n")?;
            for _ in 0..12 {
                std::thread::sleep(Duration::from_secs(1));
                socket.write_all(b"x")?;
                socket.flush()?;
            }
            Ok(())
        });
        let mut body = String::new();
        client.reader("SELECT 1")?.read_to_string(&mut body)?;
        assert_eq!(body, "x".repeat(12));
        server.join().map_err(|_| "HTTP fixture thread failed")??;
        Ok(())
    }
}
