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

use std::net::TcpStream;
use std::os::fd::{AsRawFd, RawFd};

use crate::endpoint::sealed::RouteSlot;
use crate::error::Result;
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

    /// Bytes not yet consumed (diagnostics).
    pub(crate) const fn buffered(&self) -> usize {
        self.tail - self.head + self.p_end - self.p_start
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
            let src = rec_start + tls::HEADER_LEN;
            if self.p_start == self.p_end {
                self.p_start = src;
                self.p_end = src + plen;
            } else {
                self.buf.copy_within(src..src + plen, self.p_end);
                self.p_end += plen;
                self.stats.span_copy_bytes += plen as u64;
            }
        }
        Ok(true)
    }

    /// The plaintext not yet consumed.
    #[inline]
    pub(crate) fn window(&self) -> &[u8] {
        &self.buf[self.p_start..self.p_end]
    }

    /// Marks `n` window bytes consumed.
    #[inline]
    pub(crate) const fn consume(&mut self, n: usize) {
        self.p_start += n;
        if self.p_start >= self.p_end {
            self.p_start = 0;
            self.p_end = 0;
        }
    }

    /// Reclaims space at the end of a read: resets an empty buffer, or moves the
    /// live bytes to the front when the free tail is short.
    pub(crate) fn compact(&mut self) {
        let pending_plain = self.p_start != self.p_end;
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
            // dead record headers and tags between them are dropped.
            let plain = self.p_end - self.p_start;
            let cipher = self.tail - self.head;
            self.buf.copy_within(self.p_start..self.p_end, 0);
            self.buf.copy_within(self.head..self.tail, plain);
            self.p_start = 0;
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
}
