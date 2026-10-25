//! The 64-byte message every arm sends, the same bytes on every transport.
//!
//! | bytes | field | |
//! |---|---|---|
//! | 0..8 | `seq` | u64 LE, per stream from 0 |
//! | 8..16 | `send_tsc` | u64 LE, raw counter when the send was made |
//! | 16..24 | `intended_tsc` | u64 LE, raw counter when the schedule owed it |
//! | 24..28 | `stream` | u32 LE |
//! | 28..32 | `kind` | u32 LE, [`Kind`] |
//! | 32..64 | zero | |
//!
//! Kinds start at 1, so an all-zero buffer is refused rather than read as a
//! ping.

use crate::Error;

/// Bytes in a message.
pub const MESSAGE_LEN: usize = 64;

/// What a message is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Kind {
    /// A round trip's outbound leg: the far end echoes it.
    Ping = 1,
    /// One way only (same host, same counter): never echoed.
    Oneway = 2,
    /// A ping coming back, its stamps untouched.
    Echo = 3,
}

impl Kind {
    const fn from_u32(v: u32) -> Result<Self, Error> {
        match v {
            1 => Ok(Self::Ping),
            2 => Ok(Self::Oneway),
            3 => Ok(Self::Echo),
            other => Err(Error::MessageKind(other)),
        }
    }
}

/// One message's fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Message {
    /// Sequence number within its stream, from 0.
    pub seq: u64,
    /// The raw counter when the send was made.
    pub send_tsc: u64,
    /// The raw counter when the schedule owed the send.
    pub intended_tsc: u64,
    /// The stream it belongs to.
    pub stream: u32,
    /// What it is for.
    pub kind: Kind,
}

const fn u64_at(b: &[u8; MESSAGE_LEN], at: usize) -> u64 {
    u64::from_le_bytes([
        b[at],
        b[at + 1],
        b[at + 2],
        b[at + 3],
        b[at + 4],
        b[at + 5],
        b[at + 6],
        b[at + 7],
    ])
}

const fn u32_at(b: &[u8; MESSAGE_LEN], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

impl Message {
    /// Its 64 bytes.
    #[must_use]
    #[inline]
    pub fn to_bytes(&self) -> [u8; MESSAGE_LEN] {
        let mut b = [0u8; MESSAGE_LEN];
        b[0..8].copy_from_slice(&self.seq.to_le_bytes());
        b[8..16].copy_from_slice(&self.send_tsc.to_le_bytes());
        b[16..24].copy_from_slice(&self.intended_tsc.to_le_bytes());
        b[24..28].copy_from_slice(&self.stream.to_le_bytes());
        b[28..32].copy_from_slice(&(self.kind as u32).to_le_bytes());
        b
    }

    /// Writes its 64 bytes at the start of `out`, leaving the rest alone;
    /// returns 64.
    ///
    /// # Errors
    /// [`Error::MessageLength`] when `out` is shorter than 64 bytes.
    #[inline]
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, Error> {
        let len = out.len();
        let dst = out
            .get_mut(..MESSAGE_LEN)
            .ok_or(Error::MessageLength(len))?;
        dst.copy_from_slice(&self.to_bytes());
        Ok(MESSAGE_LEN)
    }

    /// Reads a message from exactly 64 bytes.
    ///
    /// # Errors
    /// [`Error::MessageLength`] for any other length, [`Error::MessageKind`]
    /// for an unknown kind, [`Error::MessageTail`] when bytes 32..64 are not
    /// zero.
    #[inline]
    pub fn decode(buf: &[u8]) -> Result<Self, Error> {
        let b: &[u8; MESSAGE_LEN] = buf
            .try_into()
            .map_err(|_| Error::MessageLength(buf.len()))?;
        let kind = Kind::from_u32(u32_at(b, 28))?;
        if b[32..].iter().any(|&x| x != 0) {
            return Err(Error::MessageTail);
        }
        Ok(Self {
            seq: u64_at(b, 0),
            send_tsc: u64_at(b, 8),
            intended_tsc: u64_at(b, 16),
            stream: u32_at(b, 24),
            kind,
        })
    }
}

/// Turns the ping in `buf` into its echo in place (the kind, nothing else)
/// and returns the echo.
///
/// # Errors
/// What [`Message::decode`] refuses, and [`Error::NotAPing`] for a message
/// that is not a ping; `buf` is then left as it was.
#[inline]
pub fn echo_in_place(buf: &mut [u8]) -> Result<Message, Error> {
    let m = Message::decode(buf)?;
    if m.kind != Kind::Ping {
        return Err(Error::NotAPing(m.kind));
    }
    buf[28..32].copy_from_slice(&(Kind::Echo as u32).to_le_bytes());
    Ok(Message {
        kind: Kind::Echo,
        ..m
    })
}

/// The payload's layout with every field zero and kind ping: what a run
/// manifest hashes, so arms sending the same message format hash alike.
#[must_use]
pub fn template() -> [u8; MESSAGE_LEN] {
    Message {
        seq: 0,
        send_tsc: 0,
        intended_tsc: 0,
        stream: 0,
        kind: Kind::Ping,
    }
    .to_bytes()
}
