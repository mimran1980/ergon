//! [`IngressSessionDecorator`] — prepend a session message header to an offer
//! on a raw Aeron publication.
//!
//! Java `io.aeron.cluster.client.IngressSessionDecorator`. Not thread-safe:
//! each publishing thread keeps its own instance. The header is built on the
//! stack for each offer.

use rusteron_client::{AeronExclusivePublication, AeronPublication};

use crate::codecs::session::SessionMessageHeaderEncoder;
use crate::error::ClusterError;

/// Applies a client session header in front of a buffer offered to a cluster.
///
/// Leadership term, cluster session id, and timestamp start at `-1` (Aeron's
/// null). [`Self::cluster_session_id`] and [`Self::leadership_term_id`] replace
/// the first two; the timestamp stays null, as Java's `offer` does.
#[derive(Debug, Clone, Copy)]
pub struct IngressSessionDecorator {
    leadership_term_id: i64,
    cluster_session_id: i64,
}

impl Default for IngressSessionDecorator {
    fn default() -> Self {
        Self::new()
    }
}

impl IngressSessionDecorator {
    /// Bytes prepended to every offer: the SBE message header plus the
    /// session message block.
    pub const HEADER_LENGTH: usize = SessionMessageHeaderEncoder::ENCODED_LENGTH;

    /// Header fields set to Aeron's null (`-1`).
    #[must_use]
    pub fn new() -> Self {
        Self {
            leadership_term_id: -1,
            cluster_session_id: -1,
        }
    }

    /// Set the cluster session id carried in the header.
    pub fn cluster_session_id(&mut self, cluster_session_id: i64) -> &mut Self {
        self.cluster_session_id = cluster_session_id;
        self
    }

    /// Set the leadership term id carried in the header.
    pub fn leadership_term_id(&mut self, leadership_term_id: i64) -> &mut Self {
        self.leadership_term_id = leadership_term_id;
        self
    }

    /// Offer `payload` on a shared publication, session header first.
    ///
    /// # Errors
    ///
    /// Returns the publication failure when the offer does not land.
    pub fn offer(&self, publication: &AeronPublication, payload: &[u8]) -> Result<i64, ClusterError> {
        self.offer_with(payload, |parts| publication.offer_parts(parts))
    }

    /// Offer `payload` on an exclusive publication, session header first.
    ///
    /// # Errors
    ///
    /// Returns the publication failure when the offer does not land.
    pub fn offer_exclusive(
        &self,
        publication: &AeronExclusivePublication,
        payload: &[u8],
    ) -> Result<i64, ClusterError> {
        self.offer_with(payload, |parts| publication.offer_parts(parts))
    }

    fn offer_with(
        &self,
        payload: &[u8],
        offer: impl FnOnce(&[&[u8]]) -> Result<i64, rusteron_client::AeronOfferError>,
    ) -> Result<i64, ClusterError> {
        let mut header = [0u8; Self::HEADER_LENGTH];
        SessionMessageHeaderEncoder::wrap_and_apply_header(&mut header, 0)
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id)
            .timestamp(-1);
        offer(&[&header, payload]).map_err(|error| ClusterError::from_offer_error("ingress_session", error))
    }
}
