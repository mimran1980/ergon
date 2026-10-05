//! A feed published from outside the runtime, on one thread.
//!
//! A test's or a benchmark's market data, say: an agent publishes through
//! [`Ctx::publish`](crate::rt::Ctx::publish) instead, which this mirrors.
//!
//! It publishes on a channel and stream, such as those the application's
//! [`Directory`](crate::directory::Directory) gives, and the archive on
//! whichever node publishes it records it.
//!
//! [`Publication::record`] is the same exact-length claim as
//! [`Ctx::send`](crate::rt::Ctx::send), on an exclusive publication: one
//! writer, no CAS on the term tail. Its frames carry their publish time in the
//! reserved value. The application's `Source` message goes ahead of the first
//! frame, and again once the stream had no subscriber, so the recording names
//! who published it. Its drops are the feeds', counted in [`Bus::drops`];
//! persist counts its own, and
//! [`Persist::drops`](crate::persist::Persist::drops) adds the two. Unlike
//! persist, it does not ask `tables.yaml`: subscribers need every message
//! whether or not it is persisted, so the ingester applies `enabled`, `apps`
//! and `until` to feed tables when it inserts them. Every message must fit
//! one UDP frame ([`Publication::max_payload`]). Its owner closes it
//! ([`Publication::close`]).

use std::cell::Cell;

use rusteron_archive::AeronExclusivePublication;

use crate::Error;
use crate::bus::{Bus, DropCounts, DropKind, claim_exclusive};
use crate::clock::Clock;

/// A publication of this application's on a feed's channel and stream:
/// neither `Send` nor `Sync`.
pub struct Publication {
    exclusive: AeronExclusivePublication,
    stream_id: i32,
    max_payload: usize,
    source_message: Vec<u8>,
    /// Its `Source` message went out since it last had no subscriber.
    sourced: Cell<bool>,
    /// The bus's feeds' drops.
    drops: DropCounts,
    /// Stamps each frame's publish time.
    clock: Clock,
}

impl Bus {
    /// Publish on `channel` and `stream_id`.
    ///
    /// # Errors
    ///
    /// The media driver did not add the publication within 10 s.
    pub fn publication(&self, channel: &str, stream_id: i32) -> Result<Publication, Error> {
        let publication = self.add_exclusive_publication(channel, stream_id)?;
        let max_payload = publication
            .max_payload_length()
            .map_err(|e| Error::Aeron(e.to_string()))?;
        Ok(Publication {
            exclusive: publication,
            stream_id,
            max_payload,
            source_message: self.source_message().to_vec(),
            sourced: Cell::new(false),
            drops: self.drop_counts().clone(),
            clock: Clock::new(),
        })
    }
}

impl Publication {
    /// Publish one message of exactly `len` bytes (header included) that
    /// `encode` writes into the claimed frame; see
    /// [`Ctx::send`](crate::rt::Ctx::send).
    ///
    /// # Errors
    ///
    /// Returns the error from `encode`. A frame Aeron does not take is counted
    /// in [`Bus::drops`](crate::bus::Bus::drops) and returns `Ok(())`; after
    /// [`Publication::close`], it is not counted.
    #[inline]
    pub fn record<E>(
        &self,
        template_id: u16,
        len: usize,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<(), E> {
        if !self.sourced.get() {
            self.send_source();
        }
        if len > self.max_payload {
            self.refused(DropKind::TooLarge);
            return Ok(());
        }
        // A feed frame carries its publish time; its source is the recording's.
        let claim = match claim_exclusive(&self.exclusive, len, self.clock.read().0) {
            Ok(claim) => claim,
            Err(kind) => {
                self.refused(kind);
                return Ok(());
            }
        };
        let slot = claim.data();
        let written = encode(slot)?;
        let honest = written == len && slot.get(2..4) == Some(&template_id.to_le_bytes()[..]);
        debug_assert!(
            honest,
            "encode wrote {written} bytes of template {}, but claimed {len} bytes of {template_id}",
            u16::from_le_bytes([slot[2], slot[3]])
        );
        if !(honest && claim.commit().is_ok()) {
            self.refused(DropKind::Other);
        }
        Ok(())
    }

    /// Count a frame that was not published, unless the publication is
    /// closed: then it is not a drop. With no subscriber, perhaps a new
    /// recording follows: the `Source` message goes again first.
    #[cold]
    fn refused(&self, kind: DropKind) {
        if self.exclusive.is_closed() {
            return;
        }
        if kind == DropKind::NotConnected {
            self.sourced.set(false);
        }
        self.drops.count(kind);
    }

    /// The `Source` message on this publication, so its recording names who
    /// recorded it on its own. Not counted as a drop when Aeron cannot take
    /// it: the next record tries again.
    #[cold]
    fn send_source(&self) {
        let message = &self.source_message;
        if let Ok(claim) = claim_exclusive(&self.exclusive, message.len(), self.clock.read().0) {
            claim.data().copy_from_slice(message);
            self.sourced.set(claim.commit().is_ok());
        }
    }

    /// Close the publication now, so the media driver drops it at once and
    /// its subscribers turn to the next publisher within seconds, rather than
    /// after this client's timeout. A record after it is not published, and
    /// not counted as a drop.
    pub fn close(&self) {
        crate::bus::close([self.exclusive.clone()]);
    }

    /// The longest message one frame holds: size chunks from it.
    #[must_use]
    pub const fn max_payload(&self) -> usize {
        self.max_payload
    }

    /// The Aeron stream this publication writes.
    #[must_use]
    pub const fn stream_id(&self) -> i32 {
        self.stream_id
    }

    /// A subscriber or the archive's spy is taking it.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.exclusive.is_connected()
    }
}
