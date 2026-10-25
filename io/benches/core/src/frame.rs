//! The TCP arms' framing: a u32 little-endian payload length, then the
//! payload. Raw mio TCP and ergon-io's framed TCP both write it through
//! here, so the two put identical bytes on the wire.

use crate::Error;

/// Bytes in a frame's length prefix.
pub const FRAME_HEADER: usize = 4;

/// The length prefix of a `len`-byte payload.
///
/// # Errors
/// [`Error::FrameTooLarge`] past `u32::MAX`.
#[inline]
pub fn header(len: usize) -> Result<[u8; FRAME_HEADER], Error> {
    let n = u32::try_from(len).map_err(|_| Error::FrameTooLarge {
        len: len as u64,
        max: u32::MAX as usize,
    })?;
    Ok(n.to_le_bytes())
}

/// Writes `payload`'s frame at the start of `out`; returns its length.
///
/// # Errors
/// [`Error::FrameBuffer`] when `out` cannot hold it, [`Error::FrameTooLarge`]
/// past `u32::MAX`.
#[inline]
pub fn encode(payload: &[u8], out: &mut [u8]) -> Result<usize, Error> {
    let need = FRAME_HEADER + payload.len();
    let have = out.len();
    let dst = out
        .get_mut(..need)
        .ok_or(Error::FrameBuffer { need, have })?;
    dst[..FRAME_HEADER].copy_from_slice(&header(payload.len())?);
    dst[FRAME_HEADER..].copy_from_slice(payload);
    Ok(need)
}

/// The first whole frame at the start of `buf` and the bytes it takes;
/// `None` while `buf` holds only part of one.
///
/// # Errors
/// [`Error::FrameTooLarge`] as soon as the length says more than
/// `max_frame`, before its payload arrives.
#[inline]
pub fn decode(buf: &[u8], max_frame: usize) -> Result<Option<(&[u8], usize)>, Error> {
    let Some(len) = buf.first_chunk::<FRAME_HEADER>() else {
        return Ok(None);
    };
    let len = u32::from_le_bytes(*len);
    let n = usize::try_from(len).map_err(|_| Error::FrameTooLarge {
        len: u64::from(len),
        max: max_frame,
    })?;
    if n > max_frame {
        return Err(Error::FrameTooLarge {
            len: u64::from(len),
            max: max_frame,
        });
    }
    let end = FRAME_HEADER + n;
    Ok(buf.get(FRAME_HEADER..end).map(|payload| (payload, end)))
}

/// Frames from a byte stream arriving in any split.
///
/// Bytes go in by [`push`](Self::push) (or a read into [`spare`](Self::spare) and then
/// [`filled`](Self::filled)); [`next_frame`](Self::next_frame) yields each
/// whole frame. The buffer is allocated once and holds two whole frames.
#[derive(Debug, Clone)]
pub struct Deframer {
    buf: Vec<u8>,
    start: usize,
    end: usize,
    max_frame: usize,
}

impl Deframer {
    /// For frames of at most `max_frame` payload bytes.
    ///
    /// # Errors
    /// [`Error::FrameBuffer`] when the buffer's size overflows.
    pub fn new(max_frame: usize) -> Result<Self, Error> {
        let size = max_frame
            .checked_add(FRAME_HEADER)
            .and_then(|f| f.checked_mul(2))
            .ok_or(Error::FrameBuffer {
                need: usize::MAX,
                have: 0,
            })?;
        Ok(Self {
            buf: vec![0; size],
            start: 0,
            end: 0,
            max_frame,
        })
    }

    /// Bytes held that are not yet a whole frame (or not yet taken).
    #[must_use]
    pub const fn buffered(&self) -> usize {
        self.end - self.start
    }

    fn compact(&mut self) {
        if self.start > 0 {
            self.buf.copy_within(self.start..self.end, 0);
            self.end -= self.start;
            self.start = 0;
        }
    }

    /// Room to read into: the held bytes move to the front first when that
    /// makes room.
    #[inline]
    pub fn spare(&mut self) -> &mut [u8] {
        if self.end == self.buf.len() || self.start == self.end {
            self.compact();
        }
        &mut self.buf[self.end..]
    }

    /// `n` bytes were read into [`spare`](Self::spare).
    ///
    /// # Errors
    /// [`Error::FrameBuffer`] when `n` is more than the spare room.
    #[inline]
    pub const fn filled(&mut self, n: usize) -> Result<(), Error> {
        let have = self.buf.len() - self.end;
        if n > have {
            return Err(Error::FrameBuffer { need: n, have });
        }
        self.end += n;
        Ok(())
    }

    /// Copies in as much of `data` as fits; returns how much. Take the frames
    /// out ([`next_frame`](Self::next_frame)) and push the rest: after that
    /// there is always room.
    ///
    /// # Errors
    /// None today; the signature matches [`filled`](Self::filled).
    #[inline]
    pub fn push(&mut self, data: &[u8]) -> Result<usize, Error> {
        let spare = self.spare();
        let n = spare.len().min(data.len());
        spare[..n].copy_from_slice(&data[..n]);
        self.filled(n)?;
        Ok(n)
    }

    /// The next whole frame's payload, taken; `None` until one is held.
    ///
    /// # Errors
    /// [`Error::FrameTooLarge`] for a length over the limit; the stream
    /// cannot be resynchronised after one.
    #[inline]
    pub fn next_frame(&mut self) -> Result<Option<&[u8]>, Error> {
        match decode(&self.buf[self.start..self.end], self.max_frame)? {
            None => Ok(None),
            Some((_, taken)) => {
                let from = self.start + FRAME_HEADER;
                self.start += taken;
                let frame = &self.buf[from..self.start];
                if self.start == self.end {
                    // empty: the next read starts at the front
                    self.start = 0;
                    self.end = 0;
                }
                Ok(Some(frame))
            }
        }
    }
}
