//! The media driver's own statistics, sampled from its CnC file every few
//! seconds (as `AeronStat`, `ErrorStat` and `LossStat` print them) into
//! three tables:
//!
//! | table | row |
//! |---|---|
//! | `aeron_counters` | every counter: its value and what it changed by since the last sample |
//! | `aeron_errors` | each distinct error the driver logged, when it is first seen or seen again |
//! | `aeron_loss` | each stream's data loss, when it grows |
//!
//! A counter's label is also taken apart, so counters join on columns
//! instead of text: `session_id`, `stream_id` and `channel` from a stream
//! counter's key (`pub-pos`, `pub-lmt`, `sub-pos`, `snd-pos`, `rcv-hwm`,
//! …), `recording_id` from the archive's `rec-pos`, and `client_name` from
//! the heartbeat counter of the client that owns it. For example, how far
//! each subscriber is behind its publisher:
//!
//! ```sql
//! SELECT p.ts, p.stream_id, p.session_id, s.client_name, p.value - s.value AS behind
//! FROM aeron_counters p JOIN aeron_counters s USING (ts, session_id, stream_id)
//! WHERE p.type = 'pub-pos' AND s.type = 'sub-pos'
//! ```
//!
//! These are read from shared memory, not from the archive: while the
//! ingester is down nothing is sampled.

use std::collections::HashMap;

use persist_client::TableKind;
use rusteron_archive::AeronCnc;

use crate::clickhouse::ClickHouse;
use crate::metrics::{Row, column};
use crate::table::{Shape, write_string};

pub(crate) const COUNTERS: &str = "aeron_counters";
pub(crate) const ERRORS: &str = "aeron_errors";
pub(crate) const LOSS: &str = "aeron_loss";

/// Counter types whose key is a stream's: registration id at 0, session id
/// at 8, stream id at 12, channel length at 16 and the channel at 20
/// (`io.aeron.driver.status.StreamCounter`).
const STREAM_TYPES: [i32; 11] = [1, 2, 3, 4, 5, 9, 10, 12, 13, 19, 20];
const HEARTBEAT_TYPE: i32 = 11;
/// `rec-pos`: recording id at 0, session id at 8.
const RECORDING_POSITION_TYPE: i32 = 100;

fn origin_shape(name: &str, columns: Vec<crate::table::Column>, order_by: &[&str]) -> Shape {
    let mut columns = columns;
    columns.extend(crate::origin_columns());
    Shape {
        name: name.into(),
        columns,
        order_by: order_by.iter().map(|c| (*c).into()).collect(),
        partition: Some("ts".into()),
    }
}

fn counters_shape() -> Shape {
    origin_shape(
        COUNTERS,
        vec![
            column("ts", "DateTime64(9, 'UTC')"),
            column("counter_id", "Int32"),
            column("type_id", "Int32"),
            column("type", "LowCardinality(String)"),
            column("registration_id", "Int64"),
            column("owner_id", "Int64"),
            column("reference_id", "Int64"),
            column("client_name", "LowCardinality(String)"),
            column("session_id", "Nullable(Int32)"),
            column("stream_id", "Nullable(Int32)"),
            column("channel", "LowCardinality(String)"),
            column("recording_id", "Nullable(Int64)"),
            column("label", "String"),
            column("value", "Int64"),
            column("delta", "Nullable(Int64)"),
        ],
        &["type_id", "counter_id", "ts"],
    )
}

fn errors_shape() -> Shape {
    origin_shape(
        ERRORS,
        vec![
            column("ts", "DateTime64(9, 'UTC')"),
            column("first_observed", "DateTime64(3, 'UTC')"),
            column("last_observed", "DateTime64(3, 'UTC')"),
            column("count", "Int32"),
            column("error", "String"),
        ],
        &["ts"],
    )
}

fn loss_shape() -> Shape {
    origin_shape(
        LOSS,
        vec![
            column("ts", "DateTime64(9, 'UTC')"),
            column("first_observed", "DateTime64(3, 'UTC')"),
            column("last_observed", "DateTime64(3, 'UTC')"),
            column("observations", "Int64"),
            column("bytes_lost", "Int64"),
            column("session_id", "Int32"),
            column("stream_id", "Int32"),
            column("channel", "LowCardinality(String)"),
            column("source", "String"),
        ],
        &["ts"],
    )
}

/// One counter as read.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Counter {
    pub(crate) id: i32,
    pub(crate) type_id: i32,
    pub(crate) registration_id: i64,
    pub(crate) owner_id: i64,
    pub(crate) reference_id: i64,
    pub(crate) label: String,
    pub(crate) value: i64,
    pub(crate) key: Vec<u8>,
}

/// What a counter's key and label say, in columns.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Parsed {
    pub(crate) kind: String,
    pub(crate) session_id: Option<i32>,
    pub(crate) stream_id: Option<i32>,
    pub(crate) channel: String,
    pub(crate) recording_id: Option<i64>,
    /// A heartbeat counter's client name.
    pub(crate) client_name: Option<String>,
}

fn i32_at(key: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(key.get(at..at + 4)?.try_into().ok()?))
}

fn i64_at(key: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(key.get(at..at + 8)?.try_into().ok()?))
}

/// Take a counter's key and label apart.
pub(crate) fn parse(c: &Counter) -> Parsed {
    // "pub-pos (exclusive): 283 …" is a `pub-pos`; a system counter's label
    // is its description.
    let kind = match c.type_id {
        0 => "system".to_owned(),
        _ => {
            let head = c.label.split(':').next().unwrap_or_default();
            head.split(" (").next().unwrap_or(head).trim().to_owned()
        }
    };
    let mut parsed = Parsed {
        kind,
        ..Parsed::default()
    };
    if STREAM_TYPES.contains(&c.type_id) {
        parsed.session_id = i32_at(&c.key, 8);
        parsed.stream_id = i32_at(&c.key, 12);
        // The key holds up to 92 bytes of channel; the label all of it.
        let label_channel = c.label.split_whitespace().find(|w| w.starts_with("aeron:"));
        parsed.channel = match label_channel {
            Some(ch) => ch.to_owned(),
            None => i32_at(&c.key, 16)
                .and_then(|len| c.key.get(20..20 + usize::try_from(len).ok()?))
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default(),
        };
    }
    if c.type_id == RECORDING_POSITION_TYPE {
        // "rec-pos: <recording> <session> <stream> <channel> - archiveId=N"
        parsed.recording_id = i64_at(&c.key, 0);
        parsed.session_id = i32_at(&c.key, 8);
        let mut words = c.label.split_whitespace().skip(3);
        parsed.stream_id = words.next().and_then(|w| w.parse().ok());
        parsed.channel = words.next().unwrap_or_default().to_owned();
    }
    if c.type_id == HEARTBEAT_TYPE {
        // "client-heartbeat: id=277 name=recorder version=…"
        parsed.client_name = c
            .label
            .split_whitespace()
            .find_map(|w| w.strip_prefix("name="))
            .map(str::to_owned);
    }
    parsed
}

/// Samples one driver's CnC file into the three tables.
pub(crate) struct AeronStats {
    cnc: AeronCnc,
    /// The ingester's own `host`, `pod` and `app`, RowBinary.
    origin: Vec<u8>,
    /// Each counter's value at the last sample, by id and registration.
    last: HashMap<(i32, i64), i64>,
    /// The latest error observation already written (epoch ms).
    errors_since: i64,
    /// Each loss entry's observations already written.
    losses: HashMap<(i32, i32, String, String), i64>,
    /// Per table: the columns written, once it exists.
    synced: [Option<Vec<bool>>; 3],
}

impl AeronStats {
    pub(crate) fn open(aeron_dir: &str) -> Result<Self, crate::Error> {
        use rusteron_archive::IntoCString;
        let cnc = AeronCnc::open(&aeron_dir.into_c_string())
            .map_err(|e| crate::Error::Aeron(format!("{aeron_dir}/cnc.dat: {e}")))?;
        let mut origin = Vec::new();
        let pod = std::env::var("POD_NAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_default();
        for name in [persist_client::source::host_name(), pod, "ingester".into()] {
            write_string(name.as_bytes(), &mut origin);
        }
        Ok(Self {
            cnc,
            origin,
            last: HashMap::new(),
            errors_since: 0,
            losses: HashMap::new(),
            synced: [None, None, None],
        })
    }

    /// Every counter, as read now.
    pub(crate) fn counters(&self) -> Vec<Counter> {
        let reader = self.cnc.counters_reader();
        let mut out = Vec::new();
        reader.foreach_counter_fn(|value, id, type_id, key, label| {
            out.push(Counter {
                id,
                type_id,
                registration_id: reader.counter_registration_id(id).unwrap_or(-1),
                owner_id: reader.counter_owner_id(id).unwrap_or(-1),
                reference_id: reader.counter_reference_id(id).unwrap_or(-1),
                label: label.to_owned(),
                value,
                key: key.to_vec(),
            });
        });
        out
    }

    /// Sample now (UNIX ns) and insert. Failures are returned for the tick's
    /// report; this sample is then lost, and the next one tries again.
    /// `clients` names Aeron clients by id, for those whose heartbeat does
    /// not (the C client's).
    pub(crate) fn sample(
        &mut self,
        ch: &ClickHouse,
        now_ns: i64,
        clients: &HashMap<i64, String>,
    ) -> Vec<String> {
        let tables = [
            (counters_shape(), self.counter_rows(now_ns, clients)),
            (errors_shape(), self.error_rows(now_ns)),
            (loss_shape(), self.loss_rows(now_ns)),
        ];
        let mut errors = Vec::new();
        for (i, (shape, (rows, count))) in tables.into_iter().enumerate() {
            // Created at the first sample, rows or not, so queries of a
            // healthy driver's errors and losses work.
            if count == 0 && self.synced[i].is_some() {
                continue;
            }
            if let Err(e) = self.insert(ch, i, &shape, rows) {
                errors.push(format!("{}: {e}; this sample is not recorded", shape.name));
                self.synced[i] = None;
            }
        }
        errors
    }

    fn insert(
        &mut self,
        ch: &ClickHouse,
        i: usize,
        shape: &Shape,
        rows: Vec<u8>,
    ) -> Result<(), crate::Error> {
        if self.synced[i].is_none() {
            ch.create_database()?;
            let sync = ch.sync(shape, TableKind::Static)?;
            for p in &sync.problems {
                log::error!("{}: {p}", shape.name);
            }
            self.synced[i] = Some(sync.include);
        }
        let include = self.synced[i].as_deref().unwrap_or_default();
        if include.iter().any(|i| !i) {
            // ponytail: a static table someone altered; rows are built for
            // every column, so write nothing until it matches again.
            return Err(crate::Error::ClickHouse(
                "the table does not match; see the logged fix".into(),
            ));
        }
        if rows.is_empty() {
            return Ok(());
        }
        let columns: Vec<&str> = shape.columns.iter().map(|c| c.name.as_str()).collect();
        ch.insert(&shape.name, &columns, &rows)
    }

    /// `(RowBinary, rows)` of every counter.
    fn counter_rows(&mut self, now_ns: i64, clients: &HashMap<i64, String>) -> (Vec<u8>, usize) {
        let counters = self.counters();
        // A Java client names itself in its heartbeat; the rest are named
        // by the applications' `Source` messages.
        let mut names = clients.clone();
        names.extend(
            counters
                .iter()
                .filter(|c| c.type_id == HEARTBEAT_TYPE)
                .filter_map(|c| Some((i64_at(&c.key, 0)?, parse(c).client_name?)))
                .filter(|(_, name)| !name.is_empty()),
        );
        let all = vec![true; counters_shape().columns.len()];
        let mut out = Vec::new();
        let mut last = HashMap::with_capacity(counters.len());
        for c in &counters {
            let p = parse(c);
            // No delta where it does not fit: a persistent subscription's
            // join difference, say, is i64::MIN until it first joins.
            let delta = self
                .last
                .get(&(c.id, c.registration_id))
                .and_then(|before| c.value.checked_sub(*before));
            last.insert((c.id, c.registration_id), c.value);
            let client = names.get(&c.owner_id).map_or("", String::as_str);
            Row::new(&all, &mut out)
                .u64(now_ns as u64)
                .put(|o| o.extend_from_slice(&c.id.to_le_bytes()))
                .put(|o| o.extend_from_slice(&c.type_id.to_le_bytes()))
                .str(&p.kind)
                .put(|o| o.extend_from_slice(&c.registration_id.to_le_bytes()))
                .put(|o| o.extend_from_slice(&c.owner_id.to_le_bytes()))
                .put(|o| o.extend_from_slice(&c.reference_id.to_le_bytes()))
                .str(client)
                .put(|o| nullable(o, p.session_id.map(i32::to_le_bytes)))
                .put(|o| nullable(o, p.stream_id.map(i32::to_le_bytes)))
                .str(&p.channel)
                .put(|o| nullable(o, p.recording_id.map(i64::to_le_bytes)))
                .str(&c.label)
                .put(|o| o.extend_from_slice(&c.value.to_le_bytes()))
                .put(|o| nullable(o, delta.map(i64::to_le_bytes)))
                .end(&self.origin);
        }
        self.last = last;
        (out, counters.len())
    }

    /// Errors first seen, or seen again, since the last sample.
    fn error_rows(&mut self, now_ns: i64) -> (Vec<u8>, usize) {
        let all = vec![true; errors_shape().columns.len()];
        let (mut out, mut rows, mut latest) = (Vec::new(), 0, self.errors_since);
        let origin = &self.origin;
        self.cnc.error_log_read_fn(
            |count, first, last, text| {
                if last <= self.errors_since {
                    return;
                }
                latest = latest.max(last);
                rows += 1;
                Row::new(&all, &mut out)
                    .u64(now_ns as u64)
                    .put(|o| o.extend_from_slice(&first.to_le_bytes()))
                    .put(|o| o.extend_from_slice(&last.to_le_bytes()))
                    .put(|o| o.extend_from_slice(&count.to_le_bytes()))
                    .str(text)
                    .end(origin);
            },
            self.errors_since,
        );
        self.errors_since = latest;
        (out, rows)
    }

    /// Loss entries whose observations grew since the last sample.
    fn loss_rows(&mut self, now_ns: i64) -> (Vec<u8>, usize) {
        let all = vec![true; loss_shape().columns.len()];
        let (mut out, mut rows) = (Vec::new(), 0);
        let (origin, seen) = (&self.origin, &mut self.losses);
        let _ = self.cnc.loss_reporter_read_fn(
            |observations, bytes, first, last, session, stream, channel, source| {
                let key = (session, stream, channel.to_owned(), source.to_owned());
                if seen.get(&key) == Some(&observations) {
                    return;
                }
                seen.insert(key, observations);
                rows += 1;
                Row::new(&all, &mut out)
                    .u64(now_ns as u64)
                    .put(|o| o.extend_from_slice(&first.to_le_bytes()))
                    .put(|o| o.extend_from_slice(&last.to_le_bytes()))
                    .put(|o| o.extend_from_slice(&observations.to_le_bytes()))
                    .put(|o| o.extend_from_slice(&bytes.to_le_bytes()))
                    .put(|o| o.extend_from_slice(&session.to_le_bytes()))
                    .put(|o| o.extend_from_slice(&stream.to_le_bytes()))
                    .str(channel)
                    .str(source)
                    .end(origin);
            },
        );
        (out, rows)
    }
}

fn nullable<const N: usize>(out: &mut Vec<u8>, value: Option<[u8; N]>) {
    match value {
        Some(bytes) => {
            out.push(0);
            out.extend_from_slice(&bytes);
        }
        None => out.push(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream_key(registration: i64, session: i32, stream: i32, channel: &str) -> Vec<u8> {
        let mut key = registration.to_le_bytes().to_vec();
        key.extend_from_slice(&session.to_le_bytes());
        key.extend_from_slice(&stream.to_le_bytes());
        key.extend_from_slice(&(channel.len() as i32).to_le_bytes());
        key.extend_from_slice(channel.as_bytes());
        key.resize(112, 0);
        key
    }

    #[test]
    fn labels_and_keys_become_columns() {
        // As read from a 1.52.2 driver.
        let pub_pos = Counter {
            type_id: 12,
            label: "pub-pos (exclusive): 283 1506164233 20 aeron:ipc?mtu=1408|term-length=64k"
                .into(),
            key: stream_key(283, 1_506_164_233, 20, "aeron:ipc?mtu=1408|term-length=64k"),
            ..Counter::default()
        };
        assert_eq!(
            parse(&pub_pos),
            Parsed {
                kind: "pub-pos".into(),
                session_id: Some(1_506_164_233),
                stream_id: Some(20),
                channel: "aeron:ipc?mtu=1408|term-length=64k".into(),
                ..Parsed::default()
            }
        );
        let sub_pos = Counter {
            type_id: 4,
            label: "sub-pos: 21 1506164233 10 aeron:ipc?term-length=64k @0".into(),
            key: stream_key(21, 1_506_164_233, 10, "aeron:ipc?term-length=64k"),
            ..Counter::default()
        };
        assert_eq!(parse(&sub_pos).kind, "sub-pos");
        assert_eq!(parse(&sub_pos).channel, "aeron:ipc?term-length=64k");

        let mut rec_key = 255i64.to_le_bytes().to_vec();
        rec_key.extend_from_slice(&1_506_164_234i32.to_le_bytes());
        let rec_pos = Counter {
            type_id: 100,
            label: "rec-pos: 255 1506164234 7642 aeron:ipc - archiveId=1".into(),
            key: rec_key,
            ..Counter::default()
        };
        assert_eq!(
            parse(&rec_pos),
            Parsed {
                kind: "rec-pos".into(),
                session_id: Some(1_506_164_234),
                stream_id: Some(7642),
                channel: "aeron:ipc".into(),
                recording_id: Some(255),
                client_name: None,
            }
        );
        let heartbeat = Counter {
            type_id: 11,
            label: "client-heartbeat: id=2 name=archive archiveId=1 version=1.52.2".into(),
            ..Counter::default()
        };
        assert_eq!(parse(&heartbeat).client_name.as_deref(), Some("archive"));
        let system = Counter {
            type_id: 0,
            label: "Errors: version=1.52.2 commit=5b62f21d91".into(),
            ..Counter::default()
        };
        assert_eq!(parse(&system).kind, "system");
    }
}
