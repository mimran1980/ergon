//! Streaming historical feed frames from `ClickHouse` into the simulation merge.

use crate::{Error, clickhouse::ClickHouse, clock::Nanos, rt::FeedId};
use std::io::{BufReader, Read};

/// Backtest connection and raw frame table.
#[derive(Clone, Debug)]
pub struct ClickHouseConfig {
    /// HTTP client and database.
    pub client: ClickHouse,
    /// Table created by the ingester's Frame schema (normally `frame`).
    pub table: String,
}

type Row = (Nanos, i64, i64, Vec<u8>);

struct Head {
    reader: BufReader<Box<dyn Read + Send + Sync>>,
    current: Option<Row>,
}

/// A streamed head per subscribed service/kind, bounded by the requested window.
pub struct ClickHouseSource {
    config: ClickHouseConfig,
    from: Nanos,
    to: Nanos,
    names: Vec<String>,
    heads: Vec<Head>,
    /// Each head's delivery feed ids.
    pub feeds: Vec<Vec<FeedId>>,
}

impl ClickHouseSource {
    /// Prepare an input source; queries are opened when subscriptions bind.
    #[must_use]
    pub const fn new(config: ClickHouseConfig, from: Nanos, to: Nanos) -> Self {
        Self {
            config,
            from,
            to,
            names: Vec::new(),
            heads: Vec::new(),
            feeds: Vec::new(),
        }
    }

    /// Number of independent feed heads.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.heads.len()
    }

    /// Whether no feed has bound yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.heads.is_empty()
    }

    /// Add newly subscribed feeds, starting at `now` for mid-run subscriptions.
    ///
    /// # Errors
    /// The query failed or its `RowBinary` response is malformed.
    pub fn bind(&mut self, names: &[String], now: Nanos) -> Result<(), Error> {
        for (id, name) in names.iter().enumerate() {
            if name.is_empty() {
                continue;
            }
            let feed = FeedId(u32::try_from(id).map_err(|e| Error::Config(e.to_string()))?);
            if let Some(i) = self.names.iter().position(|n| n == name) {
                if !self.feeds[i].contains(&feed) {
                    self.feeds[i].push(feed);
                }
                continue;
            }
            let (service, kind) = name
                .split_once('/')
                .ok_or_else(|| Error::Config(format!("feed needs service/kind: {name}")))?;
            let quote = |s: &str| format!("`{}`", s.replace('`', "\\`"));
            let literal = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");
            let sql = format!(
                "SELECT ts, recording_id, position, message FROM {}.{} WHERE service = '{}' AND kind = '{}' AND ts BETWEEN fromUnixTimestamp64Nano({}) AND fromUnixTimestamp64Nano({}) ORDER BY ts, recording_id, position FORMAT RowBinary",
                quote(&self.config.client.database),
                quote(&self.config.table),
                literal(service),
                literal(kind),
                self.from.max(now).0,
                self.to.0
            );
            let reader = self
                .config
                .client
                .reader(&sql)
                .map_err(|e| Error::Config(format!("ClickHouse source: {e}")))?;
            let mut head = Head {
                reader: BufReader::new(reader),
                current: None,
            };
            head.current = read_row(&mut head.reader)?;
            self.names.push(name.clone());
            self.heads.push(head);
            self.feeds.push(vec![feed]);
        }
        Ok(())
    }

    /// Current row of this feed.
    #[must_use]
    pub fn head(&self, i: usize) -> Option<(Nanos, i64, i64, &[u8])> {
        self.heads
            .get(i)?
            .current
            .as_ref()
            .map(|(ts, r, p, b)| (*ts, *r, *p, b.as_slice()))
    }

    /// Consume a row and read the next one, keeping memory bounded to one frame per feed.
    ///
    /// # Errors
    /// The stream is truncated or malformed.
    pub fn advance(&mut self, i: usize) -> Result<(), Error> {
        let head = &mut self.heads[i];
        head.current = read_row(&mut head.reader)?;
        Ok(())
    }
}

fn read_row(reader: &mut impl Read) -> Result<Option<Row>, Error> {
    let bad = |e: std::io::Error| Error::Config(format!("ClickHouse RowBinary: {e}"));
    let mut fixed = [0; 24];
    let mut first = [0; 1];
    if reader.read(&mut first).map_err(bad)? == 0 {
        return Ok(None);
    }
    fixed[0] = first[0];
    reader.read_exact(&mut fixed[1..]).map_err(bad)?;
    let mut length = 0u64;
    for shift in (0..70).step_by(7) {
        let mut byte = [0; 1];
        reader.read_exact(&mut byte).map_err(bad)?;
        if shift == 63 && byte[0] > 1 {
            return Err(Error::Config("RowBinary length overflow".into()));
        }
        length |= u64::from(byte[0] & 127) << shift;
        if byte[0] & 128 == 0 {
            let len = usize::try_from(length).map_err(|e| Error::Config(e.to_string()))?;
            if len > 16 * 1024 * 1024 {
                return Err(Error::Config("RowBinary frame exceeds 16 MiB".into()));
            }
            let mut frame = vec![0; len];
            reader.read_exact(&mut frame).map_err(bad)?;
            let value = |at| i64::from_le_bytes(fixed[at..at + 8].try_into().unwrap_or([0; 8]));
            return Ok(Some((Nanos(value(0)), value(8), value(16), frame)));
        }
    }
    Err(Error::Config("RowBinary length overflow".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_bound_feed_streams_rows_and_eof_without_loading_the_window()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::io::{BufRead, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let serve = std::thread::spawn(move || -> Result<(), std::io::Error> {
            let (mut socket, _) = listener.accept()?;
            let mut request = BufReader::new(&mut socket);
            let mut length = 0;
            loop {
                let mut line = String::new();
                request.read_line(&mut line)?;
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value
                        .trim()
                        .parse::<usize>()
                        .map_err(std::io::Error::other)?;
                }
            }
            let mut request_body = vec![0; length];
            request.read_exact(&mut request_body)?;
            drop(request);
            let mut body = Vec::new();
            for (time, recording, position, message) in
                [(123i64, 5i64, 10i64, b'a'), (124, 5, 11, b'b')]
            {
                for value in [time, recording, position] {
                    body.extend_from_slice(&value.to_le_bytes());
                }
                body.extend_from_slice(&[1, message]);
            }
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            socket.write_all(&body)
        });
        let config = ClickHouseConfig {
            client: ClickHouse::new(&format!("http://{address}"), "", "", "test"),
            table: "frame".into(),
        };
        let mut source = ClickHouseSource::new(config, Nanos(100), Nanos(200));
        source.bind(&["md-test/md".into()], Nanos(100))?;
        assert_eq!(source.head(0), Some((Nanos(123), 5, 10, &b"a"[..])));
        source.advance(0)?;
        assert_eq!(source.head(0), Some((Nanos(124), 5, 11, &b"b"[..])));
        source.advance(0)?;
        assert_eq!(source.head(0), None);
        serve.join().map_err(|_| "HTTP fixture thread failed")??;
        Ok(())
    }

    #[test]
    fn rowbinary_preserves_order_keys_and_rejects_truncation() -> Result<(), Error> {
        let mut row = Vec::new();
        for value in [123i64, 42, 4096] {
            row.extend_from_slice(&value.to_le_bytes());
        }
        row.extend_from_slice(&[3, 7, 8, 9]);
        assert_eq!(
            read_row(&mut row.as_slice())?,
            Some((Nanos(123), 42, 4096, vec![7, 8, 9]))
        );
        assert!(read_row(&mut &row[..27]).is_err());
        assert_eq!(read_row(&mut &[][..])?, None);
        Ok(())
    }
}
