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
//! - A message that spans two records joins the leftover and the new
//!   plaintext by moving the smaller of the two (counted in
//!   `span_copy_bytes`): the new record back next to the leftover, or the
//!   leftover (held bytes included, as one block) forward to just before
//!   the new plaintext.
//! - The buffer is compacted only when the free tail runs low (counted in
//!   `compact_bytes`).
//! - The send queue ([`TxQueue`]) is drained from a head offset: a partial
//!   send moves nothing; the queue is compacted when it empties or its
//!   spare capacity runs low.
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
    txq: TxQueue,
    stats: StreamStats,
    route: RouteSlot,
}

impl TlsStream {
    /// A stream whose buffer holds `plain` bytes of plaintext at the front,
    /// followed by `cipher` bytes of ciphertext.
    pub(crate) fn new(
        sock: TcpStream,
        buf: Box<[u8]>,
        plain: usize,
        cipher: usize,
        tls: RecordLayer,
        txq: impl Into<TxQueue>,
    ) -> Self {
        let txq = txq.into();
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

    /// The send queue's unsent bytes (test lever).
    pub(crate) fn queued_tx(&self) -> &[u8] {
        self.txq.pending()
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
    pub(crate) const fn tx_queue(&mut self) -> &mut TxQueue {
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
            .open_in_place(&mut self.buf[rec_start..rec_start + len], self.txq.tail())?;
        self.head += len;
        self.stats.records += 1;
        if let Opened::AppData(plen) = opened {
            self.place(rec_start + tls::HEADER_LEN, plen);
        }
        Ok(true)
    }

    /// Appends `plen` bytes of plaintext at `src` (inside the record just
    /// opened) to the window: in place when the window is empty (held bytes
    /// included: the gap joins the held span), else by moving the smaller
    /// side. The record sits behind every live byte, and
    /// everything between the window's end and the record is dead (record
    /// headers, content types and tags), so either move lands on dead bytes:
    ///
    /// - the leftover (from the hold point when holding, else the window's
    ///   start; one block, so offsets from the hold point are unchanged) is
    ///   at most `plen` bytes: it moves forward to end just before `src`;
    /// - else the new plaintext moves back to the window's end.
    #[inline]
    fn place(&mut self, src: usize, plen: usize) {
        if self.p_start == self.p_end {
            // Held bytes stay where they are: the gap up to `src` becomes
            // part of the held span.
            self.p_start = src;
            self.p_end = src + plen;
            return;
        }
        let live = self.live_start();
        let left = self.p_end - live;
        if left < plen {
            let to = src - left;
            self.buf.copy_within(live..self.p_end, to);
            self.p_start = to + (self.p_start - live);
            self.hold = to;
            self.p_end = src + plen;
            self.stats.span_copy_bytes += left as u64;
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
        self.tls.seal_build(self.txq.tail(), build)
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

/// Spare capacity below which an append first compacts the sent bytes
/// away: room for two full records and their overhead.
const TX_LOW: usize = 2 * MAX_RECORD;

/// A connection's send queue: the bytes from `head` on are unsent.
///
/// The driver sends from the front ([`TxQueue::pending`]) and marks what
/// the kernel took ([`TxQueue::sent`]): a partial send only moves `head`,
/// it moves no bytes. The sent prefix is dropped when the queue empties
/// (no copy), or by one compaction when an append finds the spare capacity
/// low ([`TxQueue::tail`]).
#[derive(Debug, Default)]
pub struct TxQueue {
    buf: Vec<u8>,
    head: usize,
    /// Bytes moved by compaction (test lever).
    moved: u64,
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
impl TxQueue {
    /// A queue over `buf`, all of it unsent.
    pub(crate) const fn new(buf: Vec<u8>) -> Self {
        Self {
            buf,
            head: 0,
            moved: 0,
        }
    }

    /// An empty queue of `cap` bytes whose every page is written once now,
    /// so the first sends take no page faults (fresh memory is mapped on
    /// first touch).
    pub(crate) fn prefaulted(cap: usize) -> Self {
        let mut buf = Vec::with_capacity(cap);
        prefault(&mut buf);
        Self::new(buf)
    }

    /// The unsent bytes.
    #[inline]
    pub(crate) fn pending(&self) -> &[u8] {
        &self.buf[self.head..]
    }

    /// Unsent bytes.
    #[inline]
    pub(crate) const fn len(&self) -> usize {
        self.buf.len() - self.head
    }

    /// Whether nothing is waiting to be sent.
    #[inline]
    pub(crate) const fn is_empty(&self) -> bool {
        self.head == self.buf.len()
    }

    /// The kernel took the first `n` unsent bytes.
    #[inline]
    pub(crate) fn sent(&mut self, n: usize) {
        self.head = (self.head + n).min(self.buf.len());
        if self.head == self.buf.len() {
            self.buf.clear();
            self.head = 0;
        }
    }

    /// Drops everything queued.
    #[inline]
    pub(crate) fn clear(&mut self) {
        self.buf.clear();
        self.head = 0;
    }

    /// The vector to append to. When sent bytes are still at the front and
    /// the spare capacity is low, they are compacted away first (one move of
    /// the unsent bytes), so the append does not reallocate around dead
    /// bytes.
    #[inline]
    pub(crate) fn tail(&mut self) -> &mut Vec<u8> {
        if self.head > 0 && self.buf.capacity() - self.buf.len() < TX_LOW {
            self.compact();
        }
        &mut self.buf
    }

    /// Appends `bytes` (tests and stand-ins).
    #[cfg(test)]
    #[inline]
    pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.tail().extend_from_slice(bytes);
    }

    #[cold]
    fn compact(&mut self) {
        self.moved += self.len() as u64;
        self.buf.drain(..self.head);
        self.head = 0;
    }

    /// Swaps the queue's vector with `other` (`io_uring`: the in-flight
    /// send buffer, no copy). Returns where the unsent bytes start in the
    /// vector now in `other`; the queue is left over the old `other`,
    /// cleared.
    #[inline]
    pub(crate) fn swap_out(&mut self, other: &mut Vec<u8>) -> usize {
        std::mem::swap(&mut self.buf, other);
        self.buf.clear();
        std::mem::take(&mut self.head)
    }

    /// Where the unsent bytes start (test lever).
    #[cfg(test)]
    pub(crate) const fn head(&self) -> usize {
        self.head
    }

    /// Bytes moved by compaction so far (test lever).
    #[cfg(test)]
    pub(crate) const fn moved(&self) -> u64 {
        self.moved
    }
}

impl From<Vec<u8>> for TxQueue {
    fn from(buf: Vec<u8>) -> Self {
        Self::new(buf)
    }
}

/// Writes every page of `v`'s spare capacity once (a non-zero byte, then
/// the length is restored): fresh pages are mapped on first touch, so this
/// moves those faults to setup, off the send path.
pub fn prefault(v: &mut Vec<u8>) {
    let len = v.len();
    v.resize(v.capacity(), 1);
    std::hint::black_box(v.as_slice());
    v.truncate(len);
}

#[cfg(test)]
mod tests {
    use super::TxQueue;

    #[test]
    fn a_partial_send_moves_no_bytes() {
        let mut q = TxQueue::new(Vec::with_capacity(1 << 20));
        q.extend_from_slice(&[7u8; 1000]);
        q.sent(300);
        assert_eq!((q.head(), q.len(), q.moved()), (300, 700, 0));
        assert_eq!(q.pending(), &[7u8; 700][..]);
        q.extend_from_slice(b"next");
        assert_eq!((q.len(), q.moved()), (704, 0), "plenty of room: no move");
        q.sent(704);
        assert_eq!((q.head(), q.len(), q.moved()), (0, 0, 0), "drained: reset");
    }

    #[test]
    fn an_append_with_low_capacity_compacts_once() {
        let cap = 3 * super::MAX_RECORD;
        let mut q = TxQueue::new(Vec::with_capacity(cap));
        let first: Vec<u8> = (0..cap - 100)
            .map(|i| u8::try_from(i % 251).unwrap_or(0))
            .collect();
        q.extend_from_slice(&first);
        q.sent(first.len() - 10);
        q.extend_from_slice(b"tail");
        assert_eq!(
            (q.head(), q.moved()),
            (0, 10),
            "only the unsent bytes moved"
        );
        let mut want = first[first.len() - 10..].to_vec();
        want.extend_from_slice(b"tail");
        assert_eq!(q.pending(), &want[..]);
    }

    #[test]
    fn swap_out_hands_over_the_unsent_offset() {
        let mut q = TxQueue::new(Vec::new());
        q.extend_from_slice(b"abcdef");
        q.sent(2);
        let mut other = Vec::with_capacity(64);
        other.extend_from_slice(b"old");
        let off = q.swap_out(&mut other);
        assert_eq!(&other[off..], b"cdef");
        assert!(q.is_empty());
        assert_eq!(q.head(), 0);
    }

    #[test]
    fn prefault_keeps_the_length_and_capacity() {
        let mut v = Vec::with_capacity(1 << 16);
        v.extend_from_slice(b"keep");
        let cap = v.capacity();
        super::prefault(&mut v);
        assert_eq!((v.as_slice(), v.capacity()), (&b"keep"[..], cap));
    }
}
