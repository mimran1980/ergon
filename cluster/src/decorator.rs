//! [`IngressSessionDecorator`] puts a session message header in front of an
//! offer on a raw Aeron publication.
//!
//! It is the Java `io.aeron.cluster.client.IngressSessionDecorator`. It is not
//! thread-safe: each publishing thread keeps its own. Each offer builds the
//! header on the stack.

use rusteron_client::{AeronExclusivePublication, AeronPublication};

use crate::codecs::session::SessionMessageHeaderEncoder;
use crate::error::ClusterError;

/// Puts a client session header in front of a buffer offered to a cluster.
///
/// The leadership term, cluster session id and timestamp start at `-1`, Aeron's
/// null. [`Self::cluster_session_id`] and [`Self::leadership_term_id`] replace
/// the first two. The timestamp stays null, as in Java's `offer`.
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

    /// A decorator with every header field at Aeron's null (`-1`).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            leadership_term_id: -1,
            cluster_session_id: -1,
        }
    }

    /// Sets the cluster session id in the header.
    pub const fn cluster_session_id(&mut self, cluster_session_id: i64) -> &mut Self {
        self.cluster_session_id = cluster_session_id;
        self
    }

    /// Sets the leadership term id in the header.
    pub const fn leadership_term_id(&mut self, leadership_term_id: i64) -> &mut Self {
        self.leadership_term_id = leadership_term_id;
        self
    }

    /// Offers `payload` on a shared publication, session header first.
    ///
    /// # Errors
    ///
    /// Returns the publication failure when the offer is not accepted.
    pub fn offer(&self, publication: &AeronPublication, payload: &[u8]) -> Result<i64, ClusterError> {
        self.offer_with(payload, |parts| publication.offer_parts(parts))
    }

    /// Offers `payload` on an exclusive publication, session header first.
    ///
    /// # Errors
    ///
    /// Returns the publication failure when the offer is not accepted.
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
