//! A feed this application publishes: a UDP publication other applications
//! subscribe to (see [`crate::streams`]), such as market data, recorded by
//! the archive on whichever node publishes it.
//!
//! [`Publication::record`] is the same exact-length claim as
//! [`Persist::record`](crate::persist::Persist::record), counted in the same
//! [`Bus::drops`]. Its frames carry their publish time in the reserved value;
//! the recording's `Source` message says who published it. Unlike
//! it, it does not ask `tables.yaml`: subscribers need every message whether
//! or not it is persisted, so the ingester applies `enabled`, `apps` and
//! `until` to feed tables when it inserts them. Every message must fit one
//! UDP frame ([`Publication::max_payload`]).

use std::sync::atomic::{AtomicU64, Ordering};

use rusteron_archive::AeronPublication;

use crate::Error;
use crate::bus::{Bus, DropKind};
use crate::streams::Streams;

/// A publication of this application's on a feed's channel and stream.
pub struct Publication {
    bus: Bus,
    aeron: AeronPublication,
    stream_id: i32,
    max_payload: usize,
    /// The heartbeat this feed last sent the `Source` message at.
    heartbeat: AtomicU64,
}

impl Bus {
    /// Publish `service`'s `kind` stream (see
    /// [`crate::streams::Streams::publication`]) from this node.
    ///
    /// # Errors
    ///
    /// `service` or `kind` is not in the registry, or the media driver did
    /// not add the publication within 10 s.
    pub fn publish(
        &self,
        streams: &Streams,
        service: &str,
        kind: &str,
    ) -> Result<Publication, Error> {
        self.publication(
            &streams.publication(service, self.host_ip())?,
            streams.stream(service, kind)?,
        )
    }

    /// Publish on `channel` (see [`crate::streams::Streams::publication`])
    /// and `stream_id`.
    ///
    /// # Errors
    ///
    /// The media driver did not add the publication within 10 s.
    pub fn publication(&self, channel: &str, stream_id: i32) -> Result<Publication, Error> {
        let aeron = self.add_publication(channel, stream_id)?;
        let max_payload = aeron
            .max_payload_length()
            .map_err(|e| Error::Aeron(e.to_string()))?;
        Ok(Publication {
            bus: self.clone(),
            aeron,
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
    ///
    /// # Errors
    ///
    /// Returns the error from `encode`. A frame Aeron does not take is counted
    /// in [`Bus::drops`](crate::bus::Bus::drops) and returns `Ok(())`.
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
        // A feed frame carries its publish time; its source is the recording's.
        let claim = match bus.try_claim_at(&self.aeron, len, crate::clock::epoch_now().0) {
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
            .try_claim_at(&self.aeron, message.len(), crate::clock::epoch_now().0)
            .is_ok_and(|claim| {
                claim.data().copy_from_slice(message);
                claim.commit().is_ok()
            });
        if sent {
            self.heartbeat.store(beat, Ordering::Relaxed);
        }
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
        self.aeron.is_connected()
    }
}
