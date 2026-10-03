//! Opt-in live dispatch journal: references to messages and actual timer firings.
use crate::Error;
use crate::clock::Nanos;
use crate::rt::FeedId;
use crate::subscription::{Delivery, Origin};

/// Generated Input wire codec.
#[allow(unsafe_code, missing_docs, warnings, unused, clippy::all, clippy::pedantic, clippy::nursery, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[rustfmt::skip]
pub mod codec { include!(concat!(env!("OUT_DIR"), "/input.rs")); }
/// Schema automatically loaded by the ingester.
pub const SCHEMA: &str = include_str!("../schema/input.xml");
/// One actual dispatch in a live session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    /// Dispatch sequence within the live process.
    pub sequence: u64,
    /// Cached wall offset observed by the callback.
    pub wall_offset: i64,
    /// Deterministic-id counter immediately before the callback.
    pub next_id: u64,
    /// Agent-visible receive or timer-poll time.
    pub ts: Nanos,
    /// Message reference or actual timer firing.
    pub event: InputEvent,
}
/// Dispatch metadata, sufficient to preserve delivery and timer behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    /// Context immediately before `Agent::start`, needed to reconstruct startup timers.
    Start,
    /// Context immediately before the final `Agent::stop` callback.
    Stop,
    /// All timers were prepared before the first callback in this live poll.
    TimerBatchStart {
        /// Number of following `PendingTimer` preparation rows.
        count: u64,
    },
    /// Timer prepared by the live poll, even if a prior callback later suppresses it.
    PendingTimer {
        /// Caller-assigned correlation token.
        token: u64,
        /// Original deadline.
        deadline: Nanos,
        /// Whole periods skipped by the live poll.
        missed: u64,
    },
    /// A frame dispatched from a subscribed feed.
    Message {
        /// Runtime feed index.
        feed: FeedId,
        /// Archive recording; -1 for a raw network subscription.
        recording: i64,
        /// Position immediately after the frame.
        position: i64,
        /// Publication session, also usable to resolve raw subscriptions.
        session: i32,
        /// Publication stream.
        stream: i32,
        /// Exactly what the agent saw live.
        delivery: Delivery,
    },
    /// An actual timer dispatch; replay must suppress computed firings.
    Timer {
        /// Caller-assigned correlation token.
        token: u64,
        /// Deadline of the timer which fired.
        deadline: Nanos,
        /// Periods skipped in the live poll.
        missed: u64,
    },
}
/// Inputs in dispatch order, including ties. Never sort this list by timestamp.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Journal {
    /// Dispatches in the order the agent observed them.
    pub inputs: Vec<Input>,
}
impl Input {
    /// Fixed wire size including SBE header.
    pub const LENGTH: usize =
        codec::InputEncoder::HEADER_LENGTH + codec::InputEncoder::BLOCK_LENGTH;
    /// Encode directly into a claimed Persist frame.
    pub fn encode(self, out: &mut [u8]) -> usize {
        use codec::{InputEncoder, InputFixedFields};
        let mut f = InputFixedFields {
            sequence: self.sequence,
            wall_offset: self.wall_offset,
            next_id: self.next_id,
            ts: self.ts.0,
            feed: 0,
            recording_id: -1,
            position: 0,
            session_id: 0,
            stream_id: 0,
            token: 0,
            deadline: 0,
            missed: 0,
            kind: 0,
            first: 0,
            origin: 0,
        };
        match self.event {
            InputEvent::Start => {
                f.kind = 2;
            }
            InputEvent::Stop => {
                f.kind = 5;
            }
            InputEvent::TimerBatchStart { count } => {
                f.kind = 3;
                f.missed = count;
            }
            InputEvent::PendingTimer {
                token,
                deadline,
                missed,
            } => {
                f.kind = 4;
                f.token = token;
                f.deadline = deadline.0;
                f.missed = missed;
            }
            InputEvent::Message {
                feed,
                recording,
                position,
                session,
                stream,
                delivery,
            } => {
                f.feed = feed.0;
                f.recording_id = recording;
                f.position = position;
                f.session_id = session;
                f.stream_id = stream;
                f.first = u8::from(delivery.first);
                f.origin = u8::from(delivery.origin == Origin::Replay);
            }
            InputEvent::Timer {
                token,
                deadline,
                missed,
            } => {
                f.kind = 1;
                f.token = token;
                f.deadline = deadline.0;
                f.missed = missed;
            }
        }
        InputEncoder::wrap_and_apply_header(out, 0)
            .fixed(&f)
            .encoded_length_with_header()
    }
    /// Decode a persisted Input row.
    ///
    /// # Errors
    /// Invalid wire header, truncated row, or unknown dispatch kind.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let bad = |why| Error::Config(format!("input journal: {why}"));
        let d = codec::InputDecoder::try_decode(bytes, 0).map_err(|e| bad(e.to_string()))?;
        let event = match d.kind() {
            0 if d.first() <= 1 && d.origin() <= 1 => InputEvent::Message {
                feed: FeedId(d.feed()),
                recording: d.recording_id(),
                position: d.position(),
                session: d.session_id(),
                stream: d.stream_id(),
                delivery: Delivery {
                    first: d.first() == 1,
                    origin: if d.origin() == 0 {
                        Origin::Live
                    } else {
                        Origin::Replay
                    },
                },
            },
            2 => InputEvent::Start,
            5 => InputEvent::Stop,
            3 => InputEvent::TimerBatchStart { count: d.missed() },
            4 => InputEvent::PendingTimer {
                token: d.token(),
                deadline: Nanos(d.deadline()),
                missed: d.missed(),
            },
            1 => InputEvent::Timer {
                token: d.token(),
                deadline: Nanos(d.deadline()),
                missed: d.missed(),
            },
            _ => return Err(bad("invalid dispatch kind or delivery flags".to_owned())),
        };
        Ok(Self {
            sequence: d.sequence(),
            wall_offset: d.wall_offset(),
            next_id: d.next_id(),
            ts: Nanos(d.ts()),
            event,
        })
    }
}
impl Journal {
    /// Load Input SBE rows without changing their dispatch order.
    ///
    /// # Errors
    /// Any malformed Input row rejects the journal.
    pub fn decode<'a>(rows: impl IntoIterator<Item = &'a [u8]>) -> Result<Self, Error> {
        Ok(Self {
            inputs: rows
                .into_iter()
                .map(Input::decode)
                .collect::<Result<_, _>>()?,
        })
    }
}

/// Historical journal and frame lookup through `ClickHouse`.
#[cfg(feature = "clickhouse")]
pub mod clickhouse {
    use super::{Error, Input, InputEvent, Journal};
    use crate::clickhouse::ClickHouse;
    use std::io::Read;

    fn identifier(s: &str) -> String {
        format!("`{}`", s.replace('`', "\\`"))
    }
    fn literal(s: &str) -> String {
        s.replace('\\', "\\\\").replace('\'', "\\'")
    }
    fn bad(e: impl std::fmt::Display) -> Error {
        Error::Config(format!("ClickHouse journal: {e}"))
    }

    /// Load a recorded run in dispatch sequence order.
    ///
    /// # Errors
    /// Query failure, truncated rows, or invalid Input data.
    pub fn load(client: &ClickHouse, table: &str, run: &str) -> Result<Journal, Error> {
        let sql = format!(
            "SELECT ts, feed, recording_id, position, session_id, stream_id, token, deadline, missed, kind, first, origin, sequence, wall_offset, next_id FROM {}.{} WHERE run = '{}' ORDER BY sequence FORMAT RowBinary",
            identifier(&client.database),
            identifier(table),
            literal(run)
        );
        let mut reader = client.reader(&sql).map_err(bad)?;
        let mut inputs = Vec::new();
        loop {
            let mut bytes = vec![0; Input::LENGTH];
            // The persisted fixed fields use the same little-endian layout as SBE.
            bytes[..8].copy_from_slice(&[87, 0, 1, 0, 252, 255, 1, 0]);
            if reader.read(&mut bytes[8..9]).map_err(bad)? == 0 {
                break;
            }
            reader.read_exact(&mut bytes[9..]).map_err(bad)?;
            let input = Input::decode(&bytes)?;
            if inputs.last().is_some_and(|previous: &Input| {
                previous.sequence.checked_add(1) != Some(input.sequence)
            }) {
                return Err(bad("dispatch sequence is missing, duplicated or regressed"));
            }
            inputs.push(input);
        }
        complete(inputs)
    }

    fn complete(inputs: Vec<Input>) -> Result<Journal, Error> {
        if !inputs
            .first()
            .is_some_and(|i| i.sequence == 1 && matches!(i.event, InputEvent::Start))
        {
            return Err(bad("complete journal must start with Start at sequence 1"));
        }
        if inputs
            .windows(2)
            .any(|rows| rows[0].sequence.checked_add(1) != Some(rows[1].sequence))
        {
            return Err(bad("dispatch sequence is missing, duplicated or regressed"));
        }
        if !inputs
            .last()
            .is_some_and(|i| matches!(i.event, InputEvent::Stop))
        {
            return Err(bad("complete journal has no terminal Stop checkpoint"));
        }
        Ok(Journal { inputs })
    }

    /// Resolve a message reference to a unique raw frame.
    ///
    /// # Errors
    /// Missing or ambiguous frame, failed query, or malformed `RowBinary`.
    pub fn resolve(
        client: &ClickHouse,
        frame_table: &str,
        input: &Input,
        feed_name: &str,
    ) -> Result<Vec<u8>, Error> {
        let InputEvent::Message {
            recording,
            position,
            session,
            stream,
            delivery,
            ..
        } = input.event
        else {
            return Err(bad("a timer has no frame"));
        };
        let (service, kind) = feed_name
            .split_once('/')
            .ok_or_else(|| bad("feed locator needs service/kind"))?;
        let publication = if recording >= 0 {
            let session_clause = if delivery.origin == crate::subscription::Origin::Live {
                format!(" AND session_id = {session}")
            } else {
                String::new()
            };
            format!(
                "recording_id = {recording} AND position = {position} AND stream_id = {stream}{session_clause}"
            )
        } else {
            format!("session_id = {session} AND stream_id = {stream} AND position = {position}")
        };
        let locator = format!(
            "service = '{}' AND kind = '{}' AND {publication}",
            literal(service),
            literal(kind)
        );
        let sql = format!(
            "SELECT message FROM {}.{} WHERE {locator} LIMIT 2 FORMAT RowBinary",
            identifier(&client.database),
            identifier(frame_table)
        );
        let mut reader = client.reader(&sql).map_err(bad)?;
        let mut length = 0u64;
        for shift in (0..70).step_by(7) {
            let mut byte = [0];
            reader.read_exact(&mut byte).map_err(bad)?;
            if shift == 63 && byte[0] > 1 {
                return Err(bad("frame length overflow"));
            }
            length |= u64::from(byte[0] & 127) << shift;
            if byte[0] & 128 == 0 {
                let length = usize::try_from(length).map_err(bad)?;
                if length > 16 * 1024 * 1024 {
                    return Err(bad("frame exceeds 16 MiB"));
                }
                let mut bytes = vec![0; length];
                reader.read_exact(&mut bytes).map_err(bad)?;
                let mut extra = [0];
                if reader.read(&mut extra).map_err(bad)? != 0 {
                    return Err(bad("frame locator is ambiguous"));
                }
                return Ok(bytes);
            }
        }
        Err(bad("frame length overflow"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::clock::Nanos;
        #[test]
        fn frame_lookup_qualifies_archive_ids_with_feed_and_live_session()
        -> Result<(), Box<dyn std::error::Error>> {
            use crate::rt::FeedId;
            use crate::subscription::{Delivery, Origin};
            use std::io::{BufRead, BufReader, Write};
            let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
            let address = listener.local_addr()?;
            let server = std::thread::spawn(move || -> Result<(), std::io::Error> {
                let (mut socket, _) = listener.accept()?;
                let mut reader = BufReader::new(socket.try_clone()?);
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line)?;
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().map_err(std::io::Error::other)?;
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body)?;
                let query = String::from_utf8_lossy(&body);
                for expected in [
                    "service = 'md-test'",
                    "kind = 'md'",
                    "recording_id = 42",
                    "session_id = 9",
                    "stream_id = 2011",
                ] {
                    if !query.contains(expected) {
                        return Err(std::io::Error::other(format!(
                            "missing {expected} in {query}"
                        )));
                    }
                }
                socket.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n\x01a",
                )
            });
            let client = ClickHouse::new(&format!("http://{address}"), "", "", "test");
            let input = Input {
                sequence: 2,
                wall_offset: 0,
                next_id: 10,
                ts: Nanos(100),
                event: InputEvent::Message {
                    feed: FeedId(0),
                    recording: 42,
                    position: 128,
                    session: 9,
                    stream: 2011,
                    delivery: Delivery {
                        first: false,
                        origin: Origin::Live,
                    },
                },
            };
            assert_eq!(resolve(&client, "frame", &input, "md-test/md")?, b"a");
            server.join().map_err(|_| "HTTP fixture thread failed")??;
            Ok(())
        }

        #[test]
        fn complete_sessions_reject_lost_rows_and_partial_runs() -> Result<(), Error> {
            let start = Input {
                sequence: 1,
                wall_offset: 0,
                next_id: 10,
                ts: Nanos(100),
                event: InputEvent::Start,
            };
            let stop = Input {
                sequence: 2,
                event: InputEvent::Stop,
                ..start
            };
            assert_eq!(complete(vec![start, stop])?.inputs.len(), 2);
            assert!(complete(vec![start]).is_err());
            assert!(
                complete(vec![
                    start,
                    Input {
                        sequence: 3,
                        ..stop
                    }
                ])
                .is_err()
            );
            assert!(complete(vec![stop]).is_err());
            assert!(complete(Vec::new()).is_err());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn message_and_late_timer_round_trip_in_dispatch_order() -> Result<(), Error> {
        let inputs = [
            Input {
                sequence: 0,
                wall_offset: 20,
                next_id: 333,
                ts: Nanos(90),
                event: InputEvent::Start,
            },
            Input {
                sequence: 1,
                wall_offset: 20,
                next_id: 333,
                ts: Nanos(100),
                event: InputEvent::Message {
                    feed: FeedId(3),
                    recording: 42,
                    position: 4096,
                    session: -5,
                    stream: 2011,
                    delivery: Delivery {
                        first: true,
                        origin: Origin::Replay,
                    },
                },
            },
            Input {
                sequence: 2,
                wall_offset: 20,
                next_id: 334,
                ts: Nanos(100),
                event: InputEvent::Timer {
                    token: 17,
                    deadline: Nanos(20),
                    missed: 7,
                },
            },
        ];
        let rows: Vec<_> = inputs
            .iter()
            .map(|i| {
                let mut b = vec![0; Input::LENGTH];
                assert_eq!(i.encode(&mut b), Input::LENGTH);
                b
            })
            .collect();
        assert_eq!(
            Journal::decode(rows.iter().map(Vec::as_slice))?.inputs,
            inputs
        );
        assert!(Input::decode(&rows[0][..10]).is_err());
        Ok(())
    }
}
