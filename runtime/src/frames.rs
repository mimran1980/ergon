//! A frame log: named streams of `(ts, frame)` records, in order.
//!
//! The simulation driver reads one as an input ([`crate::sim::Sim::add_log`]),
//! and writes one as its captured output, so a recorded run is replayed with
//! no Aeron and no `ClickHouse`, and two runs compare byte for byte.
//!
//! ```text
//! "ERGNLOG1"
//! u32 streams, then per stream: u16 name length, name ("service/kind")
//! records: u32 stream, u32 length, i64 ts, frame[length]   (little-endian)
//! ```

use crate::Error;
use crate::clock::Nanos;

/// The codec generated from `schema/frames.xml`: the `Frame` table's rows.
#[allow(
    unsafe_code,
    missing_docs,
    warnings,
    unused,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]
#[rustfmt::skip]
pub mod codec {
    include!(concat!(env!("OUT_DIR"), "/frames.rs"));
}

/// The `Frame` schema: an ingester loads it beside the applications' to
/// keep raw feed frames as rows of the `frame` table.
pub const SCHEMA: &str = include_str!("../schema/frames.xml");

/// One feed frame and where the archive held it: a `frame` table row.
#[derive(Clone, Copy, Debug)]
pub struct FrameRow<'a> {
    /// The frame's publish stamp.
    pub ts: Nanos,
    /// Its recording, and the position after it there.
    pub recording: i64,
    /// The position after the frame in its recording.
    pub position: i64,
    /// The publication's session and stream.
    pub session: i32,
    /// The publication's stream.
    pub stream: i32,
    /// The feed's service (`md-binance`).
    pub service: &'a str,
    /// The feed's kind (`md`).
    pub kind: &'a str,
    /// The frame as published, its SBE header first.
    pub message: &'a [u8],
}

impl FrameRow<'_> {
    /// This row as a `Frame` message, written into `out` (cleared first).
    ///
    /// # Errors
    ///
    /// A name or the message is longer than its length field holds.
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), codec::sbe_rt::EncodeError> {
        use codec::{FrameEncoder, FrameFixedFields};
        let id = |at: usize| {
            self.message
                .get(at..at + 2)
                .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
        };
        let len = FrameEncoder::compute_length_with_header(
            self.service.len(),
            self.kind.len(),
            self.message.len(),
        );
        out.clear();
        out.resize(len, 0);
        let written = FrameEncoder::wrap_and_apply_header(out, 0)
            .fixed(&FrameFixedFields {
                ts: self.ts.0,
                recording_id: self.recording,
                position: self.position,
                session_id: self.session,
                stream_id: self.stream,
                schema_id: id(4),
                template_id: id(2),
                version: id(6),
            })
            .service(self.service.as_bytes())?
            .kind(self.kind.as_bytes())?
            .message(self.message)?
            .encoded_length_with_header();
        out.truncate(written);
        Ok(())
    }
}

const MAGIC: &[u8; 8] = b"ERGNLOG1";
/// Bytes before a record's frame.
const RECORD: usize = 4 + 4 + 8;

/// A frame log being written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameLog {
    names: Vec<String>,
    records: Vec<u8>,
}

/// One record of a log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record<'a> {
    /// Index into the log's stream names.
    pub stream: u32,
    /// Its event time.
    pub ts: Nanos,
    /// The frame, as published.
    pub frame: &'a [u8],
    /// Its offset in the record section: a position for ordering ties.
    pub offset: usize,
}

impl FrameLog {
    /// An empty log.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            names: Vec::new(),
            records: Vec::new(),
        }
    }

    /// The index of stream `name`, added if new.
    pub fn stream(&mut self, name: &str) -> u32 {
        let index = self
            .names
            .iter()
            .position(|n| n == name)
            .unwrap_or_else(|| {
                self.names.push(name.to_owned());
                self.names.len() - 1
            });
        u32::try_from(index).unwrap_or(u32::MAX)
    }

    /// Append a record whose frame is `frame`.
    pub fn push(&mut self, stream: u32, ts: Nanos, frame: &[u8]) {
        self.append(stream, ts, frame.len(), |slot| {
            slot.copy_from_slice(frame);
            Ok::<_, std::convert::Infallible>(frame.len())
        })
        .unwrap_or_else(|never| match never {});
    }

    /// Append a record of `len` bytes that `encode` writes in place. When
    /// `encode` fails or writes another length, nothing is appended.
    ///
    /// # Errors
    ///
    /// The error from `encode`.
    pub fn append<E>(
        &mut self,
        stream: u32,
        ts: Nanos,
        len: usize,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<bool, E> {
        let start = self.records.len();
        let len32 = u32::try_from(len).unwrap_or(u32::MAX);
        self.records.extend_from_slice(&stream.to_le_bytes());
        self.records.extend_from_slice(&len32.to_le_bytes());
        self.records.extend_from_slice(&ts.0.to_le_bytes());
        self.records.resize(start + RECORD + len, 0);
        match encode(&mut self.records[start + RECORD..]) {
            Ok(written) if written == len => Ok(true),
            Ok(_) => {
                self.records.truncate(start);
                Ok(false)
            }
            Err(e) => {
                self.records.truncate(start);
                Err(e)
            }
        }
    }

    /// The stream names, by index.
    #[must_use]
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// No records yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The whole log, header included.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let names: usize = self.names.iter().map(|n| 2 + n.len()).sum();
        let mut out = Vec::with_capacity(MAGIC.len() + 4 + names + self.records.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(
            &u32::try_from(self.names.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        for name in &self.names {
            out.extend_from_slice(&u16::try_from(name.len()).unwrap_or(u16::MAX).to_le_bytes());
            out.extend_from_slice(name.as_bytes());
        }
        out.extend_from_slice(&self.records);
        out
    }

    /// The records, in order.
    #[must_use]
    pub fn records(&self) -> Records<'_> {
        Records {
            bytes: &self.records,
            at: 0,
        }
    }
}

/// A parsed log: its stream names and its record section.
#[derive(Clone, Debug)]
pub struct Parsed<'a> {
    /// Stream names, by index.
    pub names: Vec<String>,
    /// The records, borrowed.
    pub records: Records<'a>,
}

/// Parse a log written by [`FrameLog::to_bytes`].
///
/// # Errors
///
/// [`Error::Config`] when it is not a frame log or is cut short.
pub fn parse(bytes: &[u8]) -> Result<Parsed<'_>, Error> {
    let bad = |what: &str| Error::Config(format!("frame log: {what}"));
    let rest = bytes
        .strip_prefix(MAGIC)
        .ok_or_else(|| bad("no ERGNLOG1 header"))?;
    let (count, mut rest) = take_u32(rest).ok_or_else(|| bad("no stream count"))?;
    let mut names = Vec::new();
    for _ in 0..count {
        let (len, tail) = rest
            .split_first_chunk::<2>()
            .map(|(l, t)| (usize::from(u16::from_le_bytes(*l)), t))
            .ok_or_else(|| bad("a stream name is cut short"))?;
        let name = tail
            .get(..len)
            .ok_or_else(|| bad("a stream name is cut short"))?;
        names.push(String::from_utf8_lossy(name).into_owned());
        rest = &tail[len..];
    }
    Ok(Parsed {
        names,
        records: Records { bytes: rest, at: 0 },
    })
}

fn take_u32(bytes: &[u8]) -> Option<(u32, &[u8])> {
    bytes
        .split_first_chunk::<4>()
        .map(|(n, rest)| (u32::from_le_bytes(*n), rest))
}

/// The records of a log, in order. A record cut short ends it.
#[derive(Clone, Debug)]
pub struct Records<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Records<'a> {
    /// The records of a record section, from byte `at`.
    #[must_use]
    pub const fn at(bytes: &'a [u8], at: usize) -> Self {
        Self { bytes, at }
    }

    /// Bytes of the section not yet read.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.at)
    }
}

impl Record<'_> {
    /// Where the next record starts.
    #[must_use]
    pub const fn end(&self) -> usize {
        self.offset + RECORD + self.frame.len()
    }
}

impl<'a> Iterator for Records<'a> {
    type Item = Record<'a>;

    fn next(&mut self) -> Option<Record<'a>> {
        let head = self.bytes.get(self.at..self.at + RECORD)?;
        let stream = u32::from_le_bytes(head[0..4].try_into().ok()?);
        let len = usize::try_from(u32::from_le_bytes(head[4..8].try_into().ok()?)).ok()?;
        let ts = i64::from_le_bytes(head[8..16].try_into().ok()?);
        let start = self.at + RECORD;
        let frame = self.bytes.get(start..start + len)?;
        let offset = self.at;
        self.at = start + len;
        Some(Record {
            stream,
            ts: Nanos(ts),
            frame,
            offset,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_round_trips_and_a_failed_encode_leaves_nothing() -> Result<(), Error> {
        let mut log = FrameLog::new();
        let md = log.stream("md-x/md");
        let fills = log.stream("exch/exec");
        assert_eq!(log.stream("md-x/md"), md);
        log.push(md, Nanos(10), b"abc");
        assert_eq!(
            log.append(fills, Nanos(11), 4, |_| Err::<usize, &str>("no")),
            Err("no")
        );
        assert_eq!(
            log.append(fills, Nanos(12), 4, |b| {
                b.copy_from_slice(b"wxyz");
                Ok::<_, ()>(3)
            }),
            Ok(false),
            "another length is refused"
        );
        log.push(fills, Nanos(13), b"defg");
        let bytes = log.to_bytes();
        let parsed = parse(&bytes)?;
        assert_eq!(parsed.names, ["md-x/md", "exch/exec"]);
        let records: Vec<_> = parsed
            .records
            .map(|r| (r.stream, r.ts.0, r.frame))
            .collect();
        assert_eq!(records, [(0, 10, &b"abc"[..]), (1, 13, &b"defg"[..])]);
        assert!(parse(b"nope").is_err());
        Ok(())
    }

    #[test]
    fn a_frame_row_round_trips_through_its_codec() -> Result<(), Box<dyn std::error::Error>> {
        let message = [24, 0, 7, 0, 9, 0, 2, 0, 0xAB, 0xCD];
        let row = FrameRow {
            ts: Nanos(1_700_000_000_000_000_123),
            recording: 42,
            position: 4_096,
            session: -5,
            stream: 2011,
            service: "md-binance",
            kind: "md",
            message: &message,
        };
        let mut out = Vec::new();
        row.encode(&mut out)?;
        let d = codec::FrameDecoder::decode(&out, 0)?;
        assert_eq!(
            (d.ts(), d.recording_id(), d.position()),
            (row.ts.0, 42, 4_096)
        );
        assert_eq!((d.session_id(), d.stream_id()), (-5, 2011));
        assert_eq!((d.template_id(), d.schema_id(), d.version()), (7, 9, 2));
        assert_eq!(d.service_as_str()?, "md-binance");
        assert_eq!(d.kind_as_str()?, "md");
        assert_eq!(d.message()?, &message[..]);
        Ok(())
    }
}
