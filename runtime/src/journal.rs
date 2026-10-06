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
/// Historical journal and frame lookup through `ClickHouse`.
#[cfg(feature = "clickhouse")]
pub mod clickhouse {
    use super::{Error, Input, InputEvent, Journal, Origin, codec};
    use crate::clickhouse::{ClickHouse, literal, quote, read_string};
    use std::collections::btree_map::Entry;
    use std::collections::{BTreeMap, BTreeSet};
    use std::io::Read;

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
            quote(&client.database),
            quote(table),
            literal(run)
        );
        let mut reader = client.reader(&sql).map_err(bad)?;
        let mut inputs = Vec::new();
        // The persisted fixed fields use the same little-endian layout as
        // SBE: each row is an Input message's body, after its header.
        let header = codec::MessageHeader::new(
            u16::try_from(codec::InputSchema::BLOCK_LENGTH).map_err(bad)?,
            codec::InputSchema::TEMPLATE_ID,
            codec::InputSchema::SCHEMA_ID,
            codec::InputSchema::SCHEMA_VERSION,
        );
        let mut bytes = [0; Input::LENGTH];
        bytes[..codec::InputSchema::HEADER_LENGTH].copy_from_slice(&header.0);
        let body = codec::InputSchema::HEADER_LENGTH;
        while reader.read(&mut bytes[body..=body]).map_err(bad)? != 0 {
            reader.read_exact(&mut bytes[body + 1..]).map_err(bad)?;
            inputs.push(Input::decode(&bytes)?);
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

    /// The publication a journalled message came from: a recording the
    /// archive kept, or, for a subscription straight off the network, the
    /// publication's session. A recording keeps its session too: after a
    /// publisher moves, two nodes' archives can each number a recording of
    /// the feed alike, and only the session tells them apart. A persistent
    /// subscription journals the recording's session for replayed messages
    /// as well as live ones, the session the ingester stores with each frame.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Publication {
        Recording(i64, i32),
        Session(i32),
    }

    /// Where `input`'s frame is: its publication, stream and position.
    const fn locator(input: &Input) -> Option<(Publication, i32, i64)> {
        let InputEvent::Message {
            recording,
            position,
            session,
            stream,
            ..
        } = input.event
        else {
            return None;
        };
        let publication = if recording < 0 {
            Publication::Session(session)
        } else {
            Publication::Recording(recording, session)
        };
        Some((publication, stream, position))
    }

    /// The raw frames a stretch of a journal names, by feed and locator.
    #[derive(Debug, Default)]
    pub struct Frames {
        frames: BTreeMap<(String, Publication, i32, i64), Vec<u8>>,
    }

    impl Frames {
        /// The frame message `input` on `feed_name` (`service/kind`) was, taken
        /// out: each is delivered once.
        pub fn take(&mut self, feed_name: &str, input: &Input) -> Option<Vec<u8>> {
            let (publication, stream, position) = locator(input)?;
            self.frames
                .remove(&(feed_name.to_owned(), publication, stream, position))
        }
    }

    /// The frames the messages of `inputs` name, with one query per feed and
    /// publication rather than one per message.
    ///
    /// Each query takes the publication's range of positions: a replay in
    /// another region would otherwise wait a round trip for every message.
    /// `feed_names` names each feed (`service/kind`) by its id; a message on
    /// a feed not named yet is left out. `around` runs before and after each
    /// query: a caller that must keep a client alive while it waits (a
    /// simulation's Aeron conductor) runs it there.
    ///
    /// # Errors
    /// A failed query, malformed `RowBinary`, or two different frames at one
    /// locator.
    pub fn fetch(
        client: &ClickHouse,
        frame_table: &str,
        inputs: &[Input],
        feed_names: &[String],
        mut around: impl FnMut(),
    ) -> Result<Frames, Error> {
        let mut ranges: BTreeMap<(&str, Publication, i32), (i64, i64)> = BTreeMap::new();
        let (mut named, mut replayed) = (BTreeSet::new(), Vec::new());
        for input in inputs {
            let InputEvent::Message { feed, .. } = input.event else {
                continue;
            };
            let (Some(name), Some((publication, stream, position))) =
                (feed_names.get(feed.0 as usize), locator(input))
            else {
                continue;
            };
            named.insert((name.as_str(), publication, stream, position));
            if let InputEvent::Message { delivery, .. } = input.event
                && matches!(delivery.origin, Origin::Replay)
            {
                replayed.push((name.as_str(), publication, stream, position));
            }
            let range = ranges
                .entry((name.as_str(), publication, stream))
                .or_insert((position, position));
            range.0 = range.0.min(position);
            range.1 = range.1.max(position);
        }
        let mut frames = Frames::default();
        for ((name, publication, stream), (low, high)) in ranges {
            let (service, kind) = name
                .split_once('/')
                .ok_or_else(|| bad("feed locator needs service/kind"))?;
            let which = match publication {
                Publication::Recording(recording, session) => {
                    format!("recording_id = {recording} AND session_id = {session}")
                }
                Publication::Session(session) => format!("session_id = {session}"),
            };
            let sql = format!(
                "SELECT position, message FROM {}.{} WHERE service = '{}' AND kind = '{}' AND {which} AND stream_id = {stream} AND position BETWEEN {low} AND {high} FORMAT RowBinary",
                quote(&client.database),
                quote(frame_table),
                literal(service),
                literal(kind)
            );
            around();
            let mut reader = std::io::BufReader::new(client.reader(&sql).map_err(bad)?);
            while let Some((position, message)) = read_frame(&mut reader)? {
                // Only the positions the journal names: a frame between them
                // is no message of it, and may be another archive's.
                if !named.contains(&(name, publication, stream, position)) {
                    continue;
                }
                match frames
                    .frames
                    .entry((name.to_owned(), publication, stream, position))
                {
                    Entry::Vacant(slot) => {
                        slot.insert(message);
                    }
                    Entry::Occupied(slot) if *slot.get() == message => {}
                    Entry::Occupied(_) => return Err(bad("frame locator is ambiguous")),
                }
            }
            around();
        }
        legacy_replays(client, frame_table, &replayed, &mut frames, &mut around)?;
        Ok(frames)
    }

    /// The frames of `replayed` messages the session lookup did not find, by
    /// their recording alone. A journal recorded before persistent
    /// subscriptions journalled the recording's session holds a replay
    /// image's for each replayed message, which no frame row carries; such
    /// journals were looked up this way. Two frames at one position is an
    /// error, as it was then.
    fn legacy_replays(
        client: &ClickHouse,
        frame_table: &str,
        replayed: &[(&str, Publication, i32, i64)],
        frames: &mut Frames,
        around: &mut impl FnMut(),
    ) -> Result<(), Error> {
        let mut wanted: BTreeMap<(&str, i64, i32), BTreeMap<i64, Publication>> = BTreeMap::new();
        for &(name, publication, stream, position) in replayed {
            let Publication::Recording(recording, _) = publication else {
                continue;
            };
            if !frames
                .frames
                .contains_key(&(name.to_owned(), publication, stream, position))
            {
                wanted
                    .entry((name, recording, stream))
                    .or_default()
                    .insert(position, publication);
            }
        }
        for ((name, recording, stream), positions) in wanted {
            let (service, kind) = name
                .split_once('/')
                .ok_or_else(|| bad("feed locator needs service/kind"))?;
            let low = positions.keys().next().copied().unwrap_or(0);
            let high = positions.keys().next_back().copied().unwrap_or(0);
            let sql = format!(
                "SELECT position, message FROM {}.{} WHERE service = '{}' AND kind = '{}' AND recording_id = {recording} AND stream_id = {stream} AND position BETWEEN {low} AND {high} FORMAT RowBinary",
                quote(&client.database),
                quote(frame_table),
                literal(service),
                literal(kind)
            );
            around();
            let mut reader = std::io::BufReader::new(client.reader(&sql).map_err(bad)?);
            while let Some((position, message)) = read_frame(&mut reader)? {
                let Some(&publication) = positions.get(&position) else {
                    continue;
                };
                match frames
                    .frames
                    .entry((name.to_owned(), publication, stream, position))
                {
                    Entry::Vacant(slot) => {
                        slot.insert(message);
                    }
                    Entry::Occupied(slot) if *slot.get() == message => {}
                    Entry::Occupied(_) => return Err(bad("frame locator is ambiguous")),
                }
            }
            around();
        }
        Ok(())
    }

    /// One `position, message` row, or `None` at the end.
    fn read_frame(reader: &mut impl Read) -> Result<Option<(i64, Vec<u8>)>, Error> {
        let mut position = [0; 8];
        if reader.read(&mut position[..1]).map_err(bad)? == 0 {
            return Ok(None);
        }
        reader.read_exact(&mut position[1..]).map_err(bad)?;
        let message = read_string(reader).map_err(bad)?;
        Ok(Some((i64::from_le_bytes(position), message)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::clock::Nanos;
        /// The fixture's thread: the queries it was sent.
        #[expect(
            clippy::disallowed_types,
            reason = "fake ClickHouse peer: the blocking client under test waits on it"
        )]
        type Served = std::thread::JoinHandle<Result<Vec<String>, std::io::Error>>;

        /// A `ClickHouse` that answers each of `answers` once, by a fragment
        /// of its query; it hands back the queries it was sent.
        fn serve(
            answers: Vec<(&'static str, Vec<u8>)>,
        ) -> Result<(ClickHouse, Served), std::io::Error> {
            use std::io::{BufRead, BufReader, Write};
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
            #[expect(
                clippy::disallowed_methods,
                reason = "fake ClickHouse peer: the blocking client under test waits on it"
            )]
            let server = std::thread::spawn(move || {
                let mut queries = Vec::new();
                for _ in 0..answers.len() {
                    let (mut socket, _) = listener.accept()?;
                    let mut reader = BufReader::new(socket.try_clone()?);
                    let mut length = 0;
                    loop {
                        let mut line = String::new();
                        reader.read_line(&mut line)?;
                        if line == "\r\n" {
                            break;
                        }
                        if let Some(value) =
                            line.to_ascii_lowercase().strip_prefix("content-length:")
                        {
                            length = value.trim().parse().map_err(std::io::Error::other)?;
                        }
                    }
                    let mut body = vec![0; length];
                    reader.read_exact(&mut body)?;
                    let query = String::from_utf8_lossy(&body).into_owned();
                    let rows = answers
                        .iter()
                        .find(|(fragment, _)| query.contains(fragment))
                        .map_or_else(Vec::new, |(_, rows)| rows.clone());
                    write!(
                        socket,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        rows.len()
                    )?;
                    socket.write_all(&rows)?;
                    queries.push(query);
                }
                Ok(queries)
            });
            Ok((client, server))
        }

        /// One `position, message` row in `RowBinary`.
        fn row(position: i64, message: &[u8]) -> Vec<u8> {
            let mut row = position.to_le_bytes().to_vec();
            row.push(u8::try_from(message.len()).unwrap_or(u8::MAX));
            row.extend_from_slice(message);
            row
        }

        fn message(feed: u32, recording: i64, session: i32, stream: i32, position: i64) -> Input {
            use crate::rt::FeedId;
            use crate::subscription::{Delivery, Origin};
            Input {
                sequence: 2,
                wall_offset: 0,
                next_id: 10,
                ts: Nanos(100),
                event: InputEvent::Message {
                    feed: FeedId(feed),
                    recording,
                    position,
                    session,
                    stream,
                    delivery: Delivery {
                        first: false,
                        origin: Origin::Live,
                    },
                },
            }
        }

        #[test]
        fn frames_are_fetched_per_publication_not_per_message()
        -> Result<(), Box<dyn std::error::Error>> {
            let names = ["md-test/md".to_owned(), "md-test/tob".to_owned()];
            let inputs = [
                message(0, 42, 9, 2011, 128),
                message(1, -1, 9, 2012, 64),
                message(0, 42, 9, 2011, 256),
                // A feed not subscribed yet: left for a later fetch.
                message(5, 7, 9, 2013, 32),
            ];
            let archived = [row(128, b"a"), row(192, b"unasked"), row(256, b"b")].concat();
            let (client, server) = serve(vec![
                ("recording_id = 42", archived),
                ("session_id = 9", row(64, b"t")),
            ])?;
            let mut runs = 0;
            let mut frames = fetch(&client, "frame", &inputs, &names, || runs += 1)?;
            let queries = server.join().map_err(|_| "HTTP fixture thread failed")??;
            assert_eq!(
                queries.len(),
                2,
                "one query per feed and publication: {queries:?}"
            );
            assert_eq!(runs, 4, "before and after each query, not around the fetch");
            assert!(queries.iter().any(|q| q.contains("service = 'md-test' AND kind = 'md' AND recording_id = 42 AND session_id = 9 AND stream_id = 2011 AND position BETWEEN 128 AND 256")), "{queries:?}");
            assert!(queries.iter().any(|q| q.contains("kind = 'tob' AND session_id = 9 AND stream_id = 2012 AND position BETWEEN 64 AND 64")), "{queries:?}");
            assert_eq!(
                frames.take("md-test/md", &inputs[0]).as_deref(),
                Some(&b"a"[..])
            );
            assert_eq!(
                frames.take("md-test/md", &inputs[2]).as_deref(),
                Some(&b"b"[..])
            );
            assert_eq!(
                frames.take("md-test/tob", &inputs[1]).as_deref(),
                Some(&b"t"[..])
            );
            assert!(
                frames.take("md-test/md", &inputs[0]).is_none(),
                "delivered once"
            );
            assert!(
                frames.take("md-test/md", &inputs[3]).is_none(),
                "not fetched"
            );
            Ok(())
        }

        #[test]
        fn live_frames_of_two_archives_recordings_numbered_alike_are_told_apart_by_session()
        -> Result<(), Box<dyn std::error::Error>> {
            // A publisher that moved nodes: each node's archive numbered its
            // recording 42, and both reached position 128.
            let names = ["md-test/md".to_owned()];
            let inputs = [message(0, 42, 9, 2011, 128), message(0, 42, 11, 2011, 128)];
            let (client, server) = serve(vec![
                ("session_id = 9", row(128, b"before the move")),
                ("session_id = 11", row(128, b"after the move")),
            ])?;
            let mut frames = fetch(&client, "frame", &inputs, &names, || {})?;
            // Before joining the fixture: a lookup that forgets the session
            // asks once, and the fixture would wait for its second query.
            assert_eq!(
                frames.take("md-test/md", &inputs[0]).as_deref(),
                Some(&b"before the move"[..])
            );
            assert_eq!(
                frames.take("md-test/md", &inputs[1]).as_deref(),
                Some(&b"after the move"[..])
            );
            let queries = server.join().map_err(|_| "HTTP fixture thread failed")??;
            assert_eq!(queries.len(), 2, "one query per session: {queries:?}");
            Ok(())
        }

        #[test]
        fn a_replayed_frame_is_found_by_its_recordings_session()
        -> Result<(), Box<dyn std::error::Error>> {
            use crate::subscription::Origin;
            let names = ["md-test/md".to_owned()];
            let mut input = message(0, 42, 9, 2011, 128);
            if let InputEvent::Message { delivery, .. } = &mut input.event {
                delivery.origin = Origin::Replay;
            }
            let (client, server) = serve(vec![(
                "recording_id = 42 AND session_id = 9",
                row(128, b"replayed"),
            )])?;
            let mut frames = fetch(&client, "frame", &[input], &names, || {})?;
            assert_eq!(
                frames.take("md-test/md", &input).as_deref(),
                Some(&b"replayed"[..])
            );
            let queries = server.join().map_err(|_| "HTTP fixture thread failed")??;
            // The recording's session, as the persistent subscription journals it.
            assert!(
                queries
                    .iter()
                    .any(|q| q.contains("recording_id = 42 AND session_id = 9")),
                "{queries:?}"
            );
            Ok(())
        }

        /// A replayed message of a journal recorded before persistent
        /// subscriptions journalled the recording's session.
        fn old_replay(position: i64) -> Input {
            use crate::subscription::Origin;
            // The replay image's session, which no frame row carries.
            let mut input = message(0, 42, -5, 2011, position);
            if let InputEvent::Message { delivery, .. } = &mut input.event {
                delivery.origin = Origin::Replay;
            }
            input
        }

        #[test]
        fn an_older_journals_replayed_frame_is_found_by_its_recording_alone()
        -> Result<(), Box<dyn std::error::Error>> {
            let names = ["md-test/md".to_owned()];
            let input = old_replay(128);
            let (client, server) = serve(vec![
                ("session_id = -5", Vec::new()),
                ("recording_id = 42 AND stream_id", row(128, b"replayed")),
            ])?;
            let mut frames = fetch(&client, "frame", &[input], &names, || {})?;
            assert_eq!(
                frames.take("md-test/md", &input).as_deref(),
                Some(&b"replayed"[..])
            );
            server.join().map_err(|_| "HTTP fixture thread failed")??;
            Ok(())
        }

        #[test]
        fn an_older_journals_replayed_frame_is_refused_when_two_archives_have_one()
        -> Result<(), Box<dyn std::error::Error>> {
            let names = ["md-test/md".to_owned()];
            let (client, server) = serve(vec![
                ("session_id = -5", Vec::new()),
                (
                    "recording_id = 42 AND stream_id",
                    [row(128, b"one archive"), row(128, b"the other")].concat(),
                ),
            ])?;
            assert!(fetch(&client, "frame", &[old_replay(128)], &names, || {}).is_err());
            server.join().map_err(|_| "HTTP fixture thread failed")??;
            Ok(())
        }

        #[test]
        fn frames_at_positions_the_journal_does_not_name_are_ignored()
        -> Result<(), Box<dyn std::error::Error>> {
            let names = ["md-test/md".to_owned()];
            let inputs = [message(0, 42, 9, 2011, 128), message(0, 42, 9, 2011, 512)];
            // Two different frames at 256, which no message names.
            let rows = [
                row(128, b"a"),
                row(256, b"x"),
                row(256, b"y"),
                row(512, b"b"),
            ]
            .concat();
            let (client, server) = serve(vec![("recording_id = 42", rows)])?;
            let mut frames = fetch(&client, "frame", &inputs, &names, || {})?;
            server.join().map_err(|_| "HTTP fixture thread failed")??;
            assert_eq!(
                frames.take("md-test/md", &inputs[0]).as_deref(),
                Some(&b"a"[..])
            );
            assert_eq!(
                frames.take("md-test/md", &inputs[1]).as_deref(),
                Some(&b"b"[..])
            );
            Ok(())
        }

        #[test]
        fn two_frames_at_one_locator_are_an_error() -> Result<(), Box<dyn std::error::Error>> {
            let names = ["md-test/md".to_owned()];
            let inputs = [message(0, 42, 9, 2011, 128)];
            let (client, server) = serve(vec![(
                "recording_id = 42",
                [row(128, b"a"), row(128, b"z")].concat(),
            )])?;
            assert!(fetch(&client, "frame", &inputs, &names, || {}).is_err());
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
            rows.iter()
                .map(Vec::as_slice)
                .map(Input::decode)
                .collect::<Result<Vec<_>, _>>()?,
            inputs
        );
        assert!(Input::decode(&rows[0][..10]).is_err());
        Ok(())
    }
}
