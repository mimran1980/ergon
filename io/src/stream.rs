//! The receive side every protocol shares: one buffer, TLS records decrypted
//! in place, and a plaintext parse window.
//!
//! Buffer layout (`head <= tail`):
//!
//! ```text
//! [ .. | leftover plaintext p_start..p_end | gap | ciphertext head..tail | free tail.. ]
//! ```
//!
//! - The kernel writes into the free tail: the only copy.
//! - `open_next` decrypts the record at `head` in place. With an empty
//!   window, the record's plaintext becomes the window: no copy.
//! - A message that spans two records moves the new plaintext next to the
//!   leftover (counted in `span_copy_bytes`).
//! - The buffer is compacted only when the free tail runs low (counted in
//!   `compact_bytes`).
//! - [`TlsStream::hold`] keeps parsed plaintext alive (a fragmented message
//!   delivered as slices): the bytes from the hold point on are neither
//!   released nor overwritten until [`TlsStream::release`]. While held, a
//!   record that opens onto an empty window still becomes the window in place;
//!   compaction moves the held bytes with the window, so offsets from the
//!   hold point stay valid.

use std::net::TcpStream;
use std::os::fd::{AsRawFd, RawFd};

use crate::endpoint::sealed::RouteSlot;
use crate::error::{Error, Result};
use crate::route::RouteCheck;
use crate::tls::{self, MAX_RECORD, Opened, RecordLayer};

/// Receive-side counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StreamStats {
    /// Bytes received from the kernel.
    pub bytes_in: u64,
    /// TLS records opened.
    pub records: u64,
    /// Plaintext bytes moved because a message spanned two records.
    pub span_copy_bytes: u64,
    /// Bytes moved by buffer compaction.
    pub compact_bytes: u64,
}

/// A connected TLS stream with its receive buffer and send queue.
pub struct TlsStream {
    sock: TcpStream,
    buf: Box<[u8]>,
    head: usize,
    tail: usize,
    p_start: usize,
    p_end: usize,
    /// Start of the held plaintext ([`TlsStream::hold`]); meaningful only
    /// while `holding`. `hold <= p_start`.
    hold: usize,
    holding: bool,
    tls: RecordLayer,
    txq: Vec<u8>,
    stats: StreamStats,
    route: RouteSlot,
}

impl TlsStream {
    /// A stream whose buffer holds `plain` bytes of plaintext at the front,
    /// followed by `cipher` bytes of ciphertext.
    pub(crate) const fn new(
        sock: TcpStream,
        buf: Box<[u8]>,
        plain: usize,
        cipher: usize,
        tls: RecordLayer,
        txq: Vec<u8>,
    ) -> Self {
        Self {
            sock,
            buf,
            head: plain,
            tail: plain + cipher,
            p_start: 0,
            p_end: plain,
            hold: 0,
            holding: false,
            tls,
            txq,
            stats: StreamStats {
                bytes_in: 0,
                records: 0,
                span_copy_bytes: 0,
                compact_bytes: 0,
            },
            route: RouteSlot::Ready(RouteCheck::NotChecked),
        }
    }

    /// The receive buffer's size.
    pub(crate) const fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// The send queue (test lever).
    pub(crate) fn queued_tx(&self) -> &[u8] {
        &self.txq
    }

    pub(crate) fn fd(&self) -> RawFd {
        self.sock.as_raw_fd()
    }

    pub(crate) const fn stats(&self) -> &StreamStats {
        &self.stats
    }

    /// What the route check found, or [`RouteCheck::Pending`] until
    /// ergon-io's warning thread has made it.
    pub(crate) fn route(&self) -> &RouteCheck {
        self.route.get()
    }

    pub(crate) const fn route_state(&self) -> &RouteSlot {
        &self.route
    }

    pub(crate) fn set_route(&mut self, route: RouteSlot) {
        self.route = route;
    }

    #[inline]
    pub(crate) fn rx_space(&mut self) -> &mut [u8] {
        &mut self.buf[self.tail..]
    }

    #[inline]
    pub(crate) const fn tx_queue(&mut self) -> &mut Vec<u8> {
        &mut self.txq
    }

    pub(crate) const fn has_buffered_input(&self) -> bool {
        self.head < self.tail || self.p_start < self.p_end
    }

    /// Bytes not yet consumed or released (diagnostics).
    pub(crate) const fn buffered(&self) -> usize {
        self.tail - self.head + self.p_end - self.live_start()
    }

    /// The first plaintext byte still needed: the hold point, or the window.
    #[inline]
    const fn live_start(&self) -> usize {
        if self.holding {
            self.hold
        } else {
            self.p_start
        }
    }

    /// Accounts for `n` bytes the kernel wrote into [`TlsStream::rx_space`].
    #[inline]
    pub(crate) const fn received(&mut self, n: usize) {
        self.tail += n;
        self.stats.bytes_in += n as u64;
    }

    /// Decrypts the next complete record in place and appends its application
    /// data to the window. Returns `false` when no complete record is buffered.
    ///
    /// # Errors
    /// A malformed record, an AEAD failure or an alert.
    #[inline]
    pub(crate) fn open_next(&mut self) -> Result<bool> {
        let Some(len) = RecordLayer::complete_record_len(&self.buf[self.head..self.tail])? else {
            return Ok(false);
        };
        let rec_start = self.head;
        let opened = self
            .tls
            .open_in_place(&mut self.buf[rec_start..rec_start + len], &mut self.txq)?;
        self.head += len;
        self.stats.records += 1;
        if let Opened::AppData(plen) = opened {
            self.place(rec_start + tls::HEADER_LEN, plen);
        }
        Ok(true)
    }

    /// Appends `plen` bytes of plaintext at `src` (inside the record just
    /// opened) to the window: in place when the window is empty, else moved
    /// next to the leftover. Held bytes lie before `p_start` and are never
    /// touched: the record sits behind every byte of the window.
    #[inline]
    fn place(&mut self, src: usize, plen: usize) {
        if self.p_start == self.p_end {
            self.p_start = src;
            self.p_end = src + plen;
        } else {
            self.buf.copy_within(src..src + plen, self.p_end);
            self.p_end += plen;
            self.stats.span_copy_bytes += plen as u64;
        }
    }

    /// The plaintext not yet consumed.
    #[inline]
    pub(crate) fn window(&self) -> &[u8] {
        &self.buf[self.p_start..self.p_end]
    }

    /// Marks `n` window bytes parsed. Outside a hold they are released at
    /// once; inside one they stay until [`TlsStream::release`].
    #[inline]
    pub(crate) const fn consume(&mut self, n: usize) {
        self.p_start += n;
        if self.p_start >= self.p_end && !self.holding {
            self.p_start = 0;
            self.p_end = 0;
        }
    }

    /// Holds the plaintext from the window's start on: [`TlsStream::consume`]
    /// then only parses past it, and nothing from here is released, moved
    /// out of order or overwritten until [`TlsStream::release`]. Offsets
    /// from this point ([`TlsStream::held_len`], [`TlsStream::held`]) survive
    /// compaction.
    #[inline]
    pub(crate) const fn hold(&mut self) {
        self.hold = self.p_start;
        self.holding = true;
    }

    /// Bytes from the hold point to the window's start (record gaps of
    /// records adopted in place included).
    #[inline]
    pub(crate) const fn held_len(&self) -> usize {
        self.p_start - self.hold
    }

    /// The held bytes, from the hold point to the window's start.
    #[inline]
    pub(crate) fn held(&self) -> &[u8] {
        &self.buf[self.hold..self.p_start]
    }

    /// Ends the hold: the held bytes are free.
    #[inline]
    pub(crate) const fn release(&mut self) {
        self.holding = false;
        if self.p_start >= self.p_end {
            self.p_start = 0;
            self.p_end = 0;
        }
    }

    /// Reclaims space at the end of a read: resets an empty buffer, or moves the
    /// live bytes (held ones included) to the front when the free tail is short.
    pub(crate) fn compact(&mut self) {
        let pending_plain = self.holding || self.p_start != self.p_end;
        if self.head == self.tail && !pending_plain {
            self.head = 0;
            self.tail = 0;
            return;
        }
        if self.buf.len() - self.tail >= 2 * MAX_RECORD {
            return;
        }
        if pending_plain {
            // Plaintext to the front, then the ciphertext right behind it: the
            // dead record headers and tags between them are dropped. Held
            // bytes move as one block with the window, so offsets from the
            // hold point are unchanged.
            let live = self.live_start();
            let plain = self.p_end - live;
            let cipher = self.tail - self.head;
            self.buf.copy_within(live..self.p_end, 0);
            self.buf.copy_within(self.head..self.tail, plain);
            self.p_start -= live;
            self.hold = 0;
            self.p_end = plain;
            self.head = plain;
            self.tail = plain + cipher;
            self.stats.compact_bytes += (plain + cipher) as u64;
        } else {
            let moved = self.tail - self.head;
            self.buf.copy_within(self.head..self.tail, 0);
            self.head = 0;
            self.tail = moved;
            self.stats.compact_bytes += moved as u64;
        }
    }

    /// Builds one record at the end of the send queue: `build` appends the
    /// plaintext, which is encrypted in place.
    ///
    /// # Errors
    /// `build` fails or produces more than one record of plaintext.
    #[inline]
    pub(crate) fn seal_build(
        &mut self,
        build: impl FnOnce(&mut Vec<u8>) -> Result<()>,
    ) -> Result<()> {
        self.tls.seal_build(&mut self.txq, build)
    }

    /// Bench and test lever: puts `plain` into the window as if a record had
    /// just been opened onto an empty buffer.
    ///
    /// # Errors
    /// Buffered bytes remain, or `plain` does not fit with room for one record.
    pub(crate) fn preload_window(&mut self, plain: &[u8]) -> Result<()> {
        if self.buffered() != 0 || self.holding {
            return Err(Error::Config("preload needs an empty buffer".into()));
        }
        if plain.len() + MAX_RECORD > self.buf.len() {
            return Err(Error::Config("preload larger than the buffer".into()));
        }
        self.buf[..plain.len()].copy_from_slice(plain);
        self.p_start = 0;
        self.p_end = plain.len();
        self.head = plain.len();
        self.tail = plain.len();
        Ok(())
    }

    /// Bench and test lever: appends one unencrypted stand-in record of
    /// `plain` to the ciphertext, laid out as a real one (5-byte header,
    /// plaintext, content type and a 16-byte tag), for
    /// [`TlsStream::open_next_plain`].
    ///
    /// # Errors
    /// `plain` is over 16 KiB or does not fit the free tail.
    pub(crate) fn stage_plain_record(&mut self, plain: &[u8]) -> Result<()> {
        let body = plain.len() + 1 + tls::TAG_LEN;
        let Ok(len16) = u16::try_from(body) else {
            return Err(Error::Config("stand-in record over 16 KiB".into()));
        };
        if plain.len() > 1 << 14 {
            return Err(Error::Config("stand-in record over 16 KiB".into()));
        }
        let total = tls::HEADER_LEN + body;
        if self.buf.len() - self.tail < total {
            return Err(Error::Config("stand-in record does not fit".into()));
        }
        let at = self.tail;
        let [hi, lo] = len16.to_be_bytes();
        self.buf[at..at + tls::HEADER_LEN].copy_from_slice(&[tls::CT_APPDATA, 3, 3, hi, lo]);
        let p = at + tls::HEADER_LEN;
        self.buf[p..p + plain.len()].copy_from_slice(plain);
        self.buf[p + plain.len()..at + total].fill(0);
        self.tail += total;
        Ok(())
    }

    /// [`TlsStream::open_next`] for stand-in records: the same placement,
    /// without the AEAD.
    ///
    /// # Errors
    /// A malformed record header.
    pub(crate) fn open_next_plain(&mut self) -> Result<bool> {
        let Some(len) = RecordLayer::complete_record_len(&self.buf[self.head..self.tail])? else {
            return Ok(false);
        };
        let Some(plen) = len.checked_sub(tls::HEADER_LEN + 1 + tls::TAG_LEN) else {
            return Err(Error::Record("stand-in record too short"));
        };
        let rec_start = self.head;
        self.head += len;
        self.stats.records += 1;
        self.place(rec_start + tls::HEADER_LEN, plen);
        Ok(true)
    }
}
