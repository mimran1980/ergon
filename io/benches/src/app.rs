//! Applications driven by the I/O loops: recording Binance or lab messages, the
//! ping-pong round-trip client, and the capture writer.

use ergon_io::{ConnId, Driver, Error, Handler, Result, RxMeta, WsConn, WsHandler, ws};

use crate::clock::realtime_ns;
use crate::json::{self, Kind};
use crate::record::{Record, read_i64};
use crate::runner::App;

fn conn_u8(id: ConnId) -> u8 {
    u8::try_from(id.index()).unwrap_or(u8::MAX)
}

/// Lab frame prefix: `seq u64 | send_ns i64 | capture_ns i64`.
pub const LAB_PREFIX: usize = 24;

/// What the recorder expects in each message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Binance combined-stream JSON.
    Binance,
    /// Lab replay frames.
    Lab,
}

/// Records one [`Record`] per message into a preallocated vector.
pub struct Recorder {
    variant: u8,
    mode: Mode,
    streams: Vec<(Vec<u8>, Kind)>,
    /// Records captured so far.
    pub records: Vec<Record>,
    /// Messages not recorded because the vector was full.
    pub dropped: u64,
    /// Messages that could not be parsed.
    pub unparsed: u64,
    closed: Option<Error>,
}

impl Recorder {
    /// A recorder with room for `capacity` records (allocated up front).
    #[must_use]
    pub fn new(variant: u8, mode: Mode, streams: &[String], capacity: usize) -> Self {
        Self {
            variant,
            mode,
            streams: streams
                .iter()
                .map(|s| (s.as_bytes().to_vec(), Kind::of(s).unwrap_or(Kind::Trade)))
                .collect(),
            records: Vec::with_capacity(capacity),
            dropped: 0,
            unparsed: 0,
            closed: None,
        }
    }

    fn push(&mut self, r: Record) {
        if self.records.len() < self.records.capacity() {
            self.records.push(r);
        } else {
            self.dropped += 1;
        }
    }

    /// The work of the run, to be shown beside its percentiles. A receive-only
    /// run sees arrivals rather than sends: `censored` is what arrived and
    /// produced no sample, and the send-side counts are the sender's.
    #[must_use]
    pub fn work(&self) -> crate::stats::Work {
        crate::stats::Work::received(self.records.len() as u64, self.dropped + self.unparsed)
    }
}

impl Handler for Recorder {
    fn on_closed(&mut self, _id: ConnId, error: Error) {
        self.closed.get_or_insert(error);
    }
}

impl WsHandler for Recorder {
    fn on_message(&mut self, conn: ConnId, _opcode: u8, payload: &[u8], meta: &RxMeta, pos: u16) {
        let mut r = Record {
            variant: self.variant,
            conn: conn_u8(conn),
            pos,
            hw: u8::from(meta.rx_hw_ns != 0),
            rx_sw_ns: meta.rx_sw_ns,
            rx_hw_ns: meta.rx_hw_ns,
            read_seq: meta.read_seq,
            ..Record::default()
        };
        match self.mode {
            Mode::Binance => {
                let parsed = json::combined(payload).and_then(|(name, data)| {
                    let idx = self
                        .streams
                        .iter()
                        .position(|(s, _)| s.as_slice() == name)?;
                    let f = json::fields(data, self.streams[idx].1)?;
                    Some((idx, f))
                });
                let Some((idx, f)) = parsed else {
                    self.unparsed += 1;
                    return;
                };
                r.stream = idx as u16;
                r.id = f.id;
                r.src_ns = json::binance_ts_to_ns(f.event_time);
                r.src2_ns = if f.trade_time > 0 {
                    json::binance_ts_to_ns(f.trade_time)
                } else {
                    0
                };
            }
            Mode::Lab => {
                if payload.len() < LAB_PREFIX {
                    self.unparsed += 1;
                    return;
                }
                r.id = read_i64(payload, 0);
                r.src_ns = read_i64(payload, 8);
                r.src2_ns = read_i64(payload, 16);
            }
        }
        r.user_ns = realtime_ns();
        self.push(r);
    }
}

impl App for Recorder {
    fn take_closed(&mut self) -> Option<Error> {
        self.closed.take()
    }
}

/// Ping-pong client: one message outstanding, sent at a fixed rate (or back to
/// back when `interval_ns == 0`); each echo becomes a record whose
/// `user_ns - src_ns` is the round trip. With an order connection set
/// ([`PingPong::with_order`]) the loop's other connections are market-data
/// feeds: their messages are decoded (sequence and send time read) and
/// counted, as an application would, and only the order connection's echoes
/// are round trips.
pub struct PingPong {
    variant: u8,
    interval_ns: i64,
    next_send: i64,
    seq: i64,
    outstanding: bool,
    /// When the outstanding message was due on the paced schedule (0 when
    /// unpaced).
    due_ns: i64,
    /// The message to send, allocated once at construction so the send path
    /// does not allocate and the size sent is exactly the size configured. A
    /// fixed stack buffer used to be silently clamped to 1024 bytes while the
    /// run metadata still reported the requested size, so a `--payload 16000`
    /// native run sent 1024 and a mio run sent 16000.
    payload: Vec<u8>,
    /// Round-trip records: `src_ns` is when the message was sent, `src2_ns`
    /// when it was due on the schedule (0 when unpaced). A stall delays the
    /// messages due during it, and the schedule never slips, so the time from
    /// `src2_ns` includes that wait (no coordinated omission).
    pub records: Vec<Record>,
    /// Round trips not recorded because the vector was full.
    pub dropped: u64,
    /// Sends that went out after their slot on the paced schedule.
    late: u64,
    /// When the first message was due on the schedule (for
    /// [`PingPong::work`]; 0 before the first send).
    first_due: i64,
    /// The order connection (None: the first connection, and no feeds).
    order: Option<ConnId>,
    /// Feed messages decoded.
    pub feed_messages: u64,
    /// The last feed message's sequence number and send time (the decode).
    pub feed_last: (i64, i64),
    closed: Option<Error>,
}

impl PingPong {
    /// A client sending `payload_len`-byte messages every `interval_ns`.
    #[must_use]
    pub fn new(variant: u8, interval_ns: i64, payload_len: usize, capacity: usize) -> Self {
        Self {
            variant,
            interval_ns,
            next_send: 0,
            seq: 0,
            outstanding: false,
            due_ns: 0,
            payload: vec![0u8; payload_len.max(16)],
            records: Vec::with_capacity(capacity),
            dropped: 0,
            late: 0,
            first_due: 0,
            order: None,
            feed_messages: 0,
            feed_last: (0, 0),
            closed: None,
        }
    }

    /// Round trips on `order`; every other connection is a feed.
    #[must_use]
    pub const fn with_order(mut self, order: ConnId) -> Self {
        self.order = Some(order);
        self
    }

    /// The work of the run, to be shown beside its percentiles. `end_real_ns`
    /// is when the run ended (wall clock), so the schedule's slots can be
    /// counted. Unpaced there is no schedule beyond the replies, so
    /// `scheduled` is the sends made.
    #[must_use]
    pub fn work(&self, end_real_ns: i64) -> crate::stats::Work {
        crate::stats::Work::paced(
            end_real_ns,
            self.first_due,
            self.interval_ns,
            self.seq as u64,
            self.records.len() as u64,
            self.late,
        )
    }
}

impl Handler for PingPong {
    fn on_closed(&mut self, _id: ConnId, error: Error) {
        self.closed.get_or_insert(error);
    }
}

impl WsHandler for PingPong {
    fn on_message(&mut self, conn: ConnId, _opcode: u8, payload: &[u8], meta: &RxMeta, pos: u16) {
        let now = realtime_ns();
        if payload.len() < 16 {
            return;
        }
        if self.order.is_some_and(|o| o != conn) {
            // A feed message: decoded, not timed.
            self.feed_last = (read_i64(payload, 0), read_i64(payload, 8));
            self.feed_messages += 1;
            return;
        }
        if self.records.len() < self.records.capacity() {
            self.records.push(Record {
                variant: self.variant,
                conn: conn_u8(conn),
                pos,
                id: read_i64(payload, 0),
                src_ns: read_i64(payload, 8),
                src2_ns: self.due_ns,
                rx_sw_ns: meta.rx_sw_ns,
                rx_hw_ns: meta.rx_hw_ns,
                user_ns: now,
                read_seq: meta.read_seq,
                ..Record::default()
            });
        } else {
            self.dropped += 1;
        }
        self.outstanding = false;
    }
}

impl App for PingPong {
    fn needs_idle(&self) -> bool {
        true
    }

    fn take_closed(&mut self) -> Option<Error> {
        self.closed.take()
    }

    fn on_idle(&mut self, driver: &mut Driver<WsConn>) -> Result<()> {
        if self.outstanding {
            return Ok(());
        }
        let now = realtime_ns();
        if now < self.next_send {
            return Ok(());
        }
        self.seq += 1;
        self.payload[..8].copy_from_slice(&self.seq.to_le_bytes());
        let t = realtime_ns();
        self.payload[8..16].copy_from_slice(&t.to_le_bytes());
        let Some(conn) = self
            .order
            .or_else(|| driver.iter().next().map(|(id, _)| id))
        else {
            return Err(Error::Closed);
        };
        // Late when the next slot is already due; see [`Work::is_late`].
        if self.next_send != 0 && crate::stats::Work::is_late(now, self.next_send, self.interval_ns)
        {
            self.late += 1;
        }
        driver
            .get_mut(conn)
            .ok_or(Error::Closed)?
            .send_frame(ws::OP_BINARY, &self.payload)?;
        self.outstanding = true;
        if self.interval_ns == 0 {
            self.due_ns = 0;
        } else {
            // A fixed schedule: after a stall the late messages go back to
            // back until it catches up.
            self.due_ns = if self.next_send == 0 {
                self.first_due = now;
                now
            } else {
                self.next_send
            };
            self.next_send = self.due_ns + self.interval_ns;
        }
        Ok(())
    }
}

/// Copies every message with its receive time into memory (for later replay).
pub struct Capture {
    /// `capture_ns i64 | len u32 | payload` entries.
    pub out: Vec<u8>,
    /// Messages captured.
    pub count: u64,
    /// The first connection error.
    pub closed: Option<Error>,
}

impl Handler for Capture {
    fn on_closed(&mut self, _id: ConnId, error: Error) {
        self.closed.get_or_insert(error);
    }
}

impl WsHandler for Capture {
    fn on_message(&mut self, _conn: ConnId, _opcode: u8, payload: &[u8], meta: &RxMeta, _pos: u16) {
        let t = if meta.rx_sw_ns != 0 {
            meta.rx_sw_ns
        } else {
            realtime_ns()
        };
        self.out.extend_from_slice(&t.to_le_bytes());
        self.out
            .extend_from_slice(&(payload.len() as u32).to_le_bytes());
        self.out.extend_from_slice(payload);
        self.count += 1;
    }
}

impl App for Capture {
    fn take_closed(&mut self) -> Option<Error> {
        self.closed.take()
    }
}
