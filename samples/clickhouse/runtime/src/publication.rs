//! A feed this application publishes: a UDP publication other applications
//! subscribe to (see [`crate::streams`]), such as market data, recorded by
//! the archive on whichever node publishes it.
//!
//! [`Publication::record`] is the same exact-length claim as
//! [`Persist::record`](crate::persist::Persist::record), stamped with the
//! application's source id and counted in the same [`Bus::drops`]. Unlike
//! it, it does not ask `tables.yaml`: subscribers need every message whether
//! or not it is persisted, so the ingester applies `enabled`, `apps` and
//! `until` to feed tables when it inserts them. Every message must fit one
//! UDP frame ([`Publication::max_payload`]).

use std::sync::atomic::{AtomicU64, Ordering};

use rusteron_archive::AeronPublication;

use crate::Error;
use crate::bus::{Bus, DropKind};

/// A publication of this application's on a feed's channel and stream.
pub struct Publication {
    bus: Bus,
    publication: AeronPublication,
    stream_id: i32,
    max_payload: usize,
    /// The heartbeat this feed last sent the `Source` message at.
    heartbeat: AtomicU64,
}

impl Bus {
    /// Publish on `channel` (see [`crate::streams::Streams::publication`])
    /// and `stream_id`.
    ///
    /// # Errors
    ///
    /// The media driver did not add the publication within 10 s.
    pub fn publication(&self, channel: &str, stream_id: i32) -> Result<Publication, Error> {
        let publication = self.add_publication(channel, stream_id)?;
        let max_payload = publication
            .max_payload_length()
            .map_err(|e| Error::Aeron(e.to_string()))?;
        Ok(Publication {
            bus: self.clone(),
            publication,
            stream_id,
            max_payload,
            heartbeat: AtomicU64::new(u64::MAX),
        })
    }
}

impl Publication {
    /// Publish one message of exactly `len` bytes (header included) that
    /// `encode` writes into the claimed frame; see
    /// [`Persist::record`](crate::persist::Persist::record).
    #[inline]
    pub fn record<E>(
        &self,
        template_id: u16,
        len: usize,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<(), E> {
        let bus = &self.bus;
        let beat = bus.heartbeat();
        if beat != self.heartbeat.load(Ordering::Relaxed) {
            self.send_source(beat);
        }
        if len > self.max_payload {
            bus.count(DropKind::TooLarge);
            return Ok(());
        }
        let mut claim = match bus.try_claim(&self.publication, len) {
            Ok(claim) => claim,
            Err(kind) => {
                bus.count(kind);
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
            bus.drop_one();
        }
        Ok(())
    }

    /// The `Source` message on this publication, so its recording names who
    /// recorded it on its own; repeated every 5 s. Not counted as a drop
    /// when Aeron cannot take it: the next record tries again.
    #[cold]
    fn send_source(&self, beat: u64) {
        let message = self.bus.source_message();
        let sent = self
            .bus
            .try_claim(&self.publication, message.len())
            .is_ok_and(|mut claim| {
                claim.data().copy_from_slice(message);
                claim.commit().is_ok()
            });
        if sent {
            self.heartbeat.store(beat, Ordering::Relaxed);
        }
    }

    /// The longest message one frame holds: size chunks from it.
    #[must_use]
    pub fn max_payload(&self) -> usize {
        self.max_payload
    }

    #[must_use]
    pub fn stream_id(&self) -> i32 {
        self.stream_id
    }

    /// A subscriber or the archive's spy is taking it.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.publication.is_connected()
    }
}
