//! The core's one error type.

use crate::message::Kind;
use std::fmt;

/// Everything the core refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A message buffer that is not exactly [`MESSAGE_LEN`](crate::message::MESSAGE_LEN)
    /// bytes (decode), or too short to hold one (encode).
    MessageLength(usize),
    /// A message whose kind is none of [`Kind`]'s.
    MessageKind(u32),
    /// A message whose bytes 32..64 are not all zero.
    MessageTail,
    /// An echo asked of a message that is not a ping.
    NotAPing(Kind),
    /// A schedule that cannot run: why.
    Schedule(&'static str),
    /// A slot reported sent out of turn: slots go strictly in order.
    SlotOutOfTurn {
        /// The slot the pacer is on.
        expected: u64,
        /// The slot reported.
        got: u64,
    },
    /// A loss window that is not a power of two of at least 64.
    Window(usize),
    /// A stream index past the tracker's streams.
    Stream {
        /// The stream given.
        stream: u32,
        /// How many streams the tracker has.
        streams: u32,
    },
    /// No streams, or a per-stream list whose length is not the stream count.
    Streams {
        /// How many streams the tracker has.
        streams: u32,
        /// How many were given.
        got: usize,
    },
    /// A frame length over the frame limit.
    FrameTooLarge {
        /// The length on the wire.
        len: u64,
        /// The limit.
        max: usize,
    },
    /// A buffer too small for what it must hold.
    FrameBuffer {
        /// Bytes needed.
        need: usize,
        /// Bytes there are.
        have: usize,
    },
    /// A row field empty or holding a comma or line break: the row would not
    /// parse back.
    Field(String),
    /// The histogram could not be built.
    Histogram(String),
    /// The cross-CPU check's peer thread stopped answering, or panicked.
    CrossCpu(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MessageLength(n) => write!(f, "message buffer of {n} bytes, not 64"),
            Self::MessageKind(k) => write!(
                f,
                "message kind {k} is not ping (1), oneway (2) or echo (3)"
            ),
            Self::MessageTail => write!(f, "message bytes 32..64 are not zero"),
            Self::NotAPing(k) => write!(f, "only a ping is echoed, not {k:?}"),
            Self::Schedule(why) => write!(f, "schedule: {why}"),
            Self::SlotOutOfTurn { expected, got } => {
                write!(f, "slot {got} sent while slot {expected} is next")
            }
            Self::Window(w) => write!(f, "loss window {w}: a power of two of at least 64"),
            Self::Stream { stream, streams } => write!(f, "stream {stream} of {streams}"),
            Self::Streams { streams, got } => write!(f, "{got} values for {streams} streams"),
            Self::FrameTooLarge { len, max } => {
                write!(f, "frame of {len} bytes over the {max}-byte limit")
            }
            Self::FrameBuffer { need, have } => write!(f, "buffer of {have} bytes, {need} needed"),
            Self::Field(v) => write!(f, "row field {v:?} is empty or holds a comma or line break"),
            Self::Histogram(e) => write!(f, "histogram: {e}"),
            Self::CrossCpu(why) => write!(f, "cross-CPU check: {why}"),
        }
    }
}

impl std::error::Error for Error {}
