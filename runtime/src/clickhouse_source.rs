//! Historical feed frames from `ClickHouse`, into the simulation merge.
//!
//! Each feed is read a page at a time, each query to its end at once: one
//! long stream per feed, merged by time, would leave a feed's stream unread
//! while the others catch up, and the server drops a stream it cannot write
//! to for `http_send_timeout`.

use crate::clickhouse::{ClickHouse, literal, quote, read_string};
use crate::{Error, clock::Nanos, rt::FeedId};
use std::collections::VecDeque;
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

/// Rows of one feed a query fetches: a few megabytes.
const PAGE: usize = 16_384;

struct Head {
    service: String,
    kind: String,
    /// Where the feed's rows start: the window, or a mid-run subscription.
    from: Nanos,
    page: VecDeque<Row>,
    /// The order key of the last row taken: the next page starts after it.
    after: Option<(Nanos, i64, i64)>,
    /// The last page was short: the window holds nothing more.
    done: bool,
}

/// A head per subscribed service/kind, bounded by the requested window and
/// fetched a page at a time.
pub struct ClickHouseSource {
    config: ClickHouseConfig,
    from: Nanos,
    to: Nanos,
    page: usize,
    names: Vec<String>,
    heads: Vec<Head>,
    /// Each head's delivery feed ids.
    pub feeds: Vec<Vec<FeedId>>,
}

impl ClickHouseSource {
    /// Prepare an input source; queries are made when subscriptions bind.
    #[must_use]
    pub const fn new(config: ClickHouseConfig, from: Nanos, to: Nanos) -> Self {
        Self {
            config,
            from,
            to,
            page: PAGE,
            names: Vec::new(),
            heads: Vec::new(),
            feeds: Vec::new(),
        }
    }

    /// Pages of `page` rows: a test's.
    #[cfg(test)]
    const fn paged(mut self, page: usize) -> Self {
        self.page = page;
        self
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
    /// Each new feed is a query, with `around` run before and after it: a
    /// caller that must keep a client alive while it waits (a simulation's
    /// Aeron conductor) runs it there, between every two queries.
    ///
    /// # Errors
    /// The query failed or its `RowBinary` response is malformed.
    pub fn bind(
        &mut self,
        names: &[String],
        now: Nanos,
        mut around: impl FnMut(),
    ) -> Result<(), Error> {
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
            let mut head = Head {
                service: service.to_owned(),
                kind: kind.to_owned(),
                from: self.from.max(now),
                page: VecDeque::new(),
                after: None,
                done: false,
            };
            around();
            fetch(&self.config, self.to, self.page, &mut head)?;
            around();
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
            .page
            .front()
            .map(|(ts, r, p, b)| (*ts, *r, *p, b.as_slice()))
    }

    /// [`ClickHouseSource::advance`] on head `i` fetches the next page: it
    /// holds its page's last row, and the window has more.
    pub(crate) fn fetches(&self, i: usize) -> bool {
        self.heads
            .get(i)
            .is_some_and(|head| head.page.len() <= 1 && !head.done)
    }

    /// Consume a row; past the page, fetch the next one.
    ///
    /// # Errors
    /// The query failed or its `RowBinary` response is malformed.
    pub fn advance(&mut self, i: usize) -> Result<(), Error> {
        let head = &mut self.heads[i];
        if let Some((ts, recording, position, _)) = head.page.pop_front() {
            head.after = Some((ts, recording, position));
        }
        if head.page.is_empty() && !head.done {
            fetch(&self.config, self.to, self.page, head)?;
        }
        Ok(())
    }
}

/// The next page of `head`'s feed, up to `to`, after its last row.
fn fetch(config: &ClickHouseConfig, to: Nanos, page: usize, head: &mut Head) -> Result<(), Error> {
    let after = head.after.map_or_else(String::new, |(ts, recording, position)| {
            format!(
                " AND (ts, recording_id, position) > (fromUnixTimestamp64Nano({}), {recording}, {position})",
                ts.0
            )
        });
    let sql = format!(
        "SELECT ts, recording_id, position, message FROM {}.{} WHERE service = '{}' AND kind = '{}' AND ts BETWEEN fromUnixTimestamp64Nano({}) AND fromUnixTimestamp64Nano({}){after} ORDER BY ts, recording_id, position LIMIT {} FORMAT RowBinary",
        quote(&config.client.database),
        quote(&config.table),
        literal(&head.service),
        literal(&head.kind),
        head.from.0,
        to.0,
        page
    );
    let mut reader = BufReader::new(
        config
            .client
            .reader(&sql)
            .map_err(|e| Error::Config(format!("ClickHouse source: {e}")))?,
    );
    let before = head.page.len();
    while let Some(row) = read_row(&mut reader)? {
        head.page.push_back(row);
    }
    head.done = head.page.len() - before < page;
    Ok(())
}

fn read_row(reader: &mut impl Read) -> Result<Option<Row>, Error> {
    let bad = |e: &dyn std::fmt::Display| Error::Config(format!("ClickHouse RowBinary: {e}"));
    let mut fixed = [0; 24];
    if reader.read(&mut fixed[..1]).map_err(|e| bad(&e))? == 0 {
        return Ok(None);
    }
    reader.read_exact(&mut fixed[1..]).map_err(|e| bad(&e))?;
    let frame = read_string(reader).map_err(|e| bad(&e))?;
    let value = |at| i64::from_le_bytes(fixed[at..at + 8].try_into().unwrap_or([0; 8]));
    Ok(Some((Nanos(value(0)), value(8), value(16), frame)))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Answers one query: `rows` as `RowBinary`, handing back the query.
    fn answer(
        listener: &std::net::TcpListener,
        rows: &[(i64, i64, i64, u8)],
    ) -> Result<String, std::io::Error> {
        use std::io::{BufRead, Write};
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
        let mut query = vec![0; length];
        request.read_exact(&mut query)?;
        drop(request);
        let mut body = Vec::new();
        for &(time, recording, position, message) in rows {
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
        socket.write_all(&body)?;
        Ok(String::from_utf8_lossy(&query).into_owned())
    }

    #[test]
    fn a_bound_feed_is_read_a_page_at_a_time() -> Result<(), Box<dyn std::error::Error>> {
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: a literal loopback address, nothing to resolve"
        )]
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: the blocking client under test waits on it"
        )]
        let serve = std::thread::spawn(move || -> Result<Vec<String>, std::io::Error> {
            Ok(vec![
                answer(&listener, &[(123, 5, 10, b'a'), (124, 5, 11, b'b')])?,
                answer(&listener, &[(124, 5, 12, b'c')])?,
            ])
        });
        let config = ClickHouseConfig {
            client: ClickHouse::new(&format!("http://{address}"), "", "", "test"),
            table: "frame".into(),
        };
        let mut source = ClickHouseSource::new(config, Nanos(100), Nanos(200)).paged(2);
        source.bind(&["md-test/md".into()], Nanos(100), || {})?;
        let mut rows = Vec::new();
        while let Some((ts, recording, position, message)) = source.head(0) {
            rows.push((ts.0, recording, position, message[0]));
            source.advance(0)?;
        }
        assert_eq!(
            rows,
            [(123, 5, 10, b'a'), (124, 5, 11, b'b'), (124, 5, 12, b'c')]
        );
        let queries = serve.join().map_err(|_| "HTTP fixture thread failed")??;
        assert!(
            queries[0].contains("LIMIT 2") && !queries[0].contains("recording_id, position) >")
        );
        assert!(
            queries[1]
                .contains("(ts, recording_id, position) > (fromUnixTimestamp64Nano(124), 5, 11)"),
            "the second page starts after the first's last row: {}",
            queries[1]
        );
        Ok(())
    }

    #[test]
    fn a_bind_runs_its_hook_around_each_query() -> Result<(), Box<dyn std::error::Error>> {
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: a literal loopback address, nothing to resolve"
        )]
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        #[expect(
            clippy::disallowed_methods,
            reason = "fake ClickHouse peer: the blocking client under test waits on it"
        )]
        let serve = std::thread::spawn(move || -> Result<Vec<String>, std::io::Error> {
            Ok(vec![
                answer(&listener, &[(123, 5, 10, b'a')])?,
                answer(&listener, &[(124, 6, 10, b'b')])?,
            ])
        });
        let config = ClickHouseConfig {
            client: ClickHouse::new(&format!("http://{address}"), "", "", "test"),
            table: "frame".into(),
        };
        let mut source = ClickHouseSource::new(config, Nanos(100), Nanos(200));
        let mut runs = 0;
        // Two new feeds, a loopback one and a repeat: two queries.
        let names = [
            "md-a/md".into(),
            String::new(),
            "md-b/md".into(),
            "md-a/md".into(),
        ];
        source.bind(&names, Nanos(100), || runs += 1)?;
        let queries = serve.join().map_err(|_| "HTTP fixture thread failed")??;
        assert_eq!(queries.len(), 2, "{queries:?}");
        assert_eq!(runs, 4, "before and after each query, not around the bind");
        assert_eq!(source.feeds, [vec![FeedId(0), FeedId(3)], vec![FeedId(2)]]);
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
