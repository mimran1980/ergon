//! Cluster try_claim publisher for AppMessage(L2Book).
//!
//! Matches [`ergo_aeron_cluster::AeronCluster::try_claim`]: the ingress writes
//! SessionMessageHeader into the first 32 bytes; the fill closure only encodes
//! the application payload.

use ergo_aeron_cluster::codecs::session::SessionMessageHeaderEncoder;
use ergo_aeron_cluster::{AeronCluster, ClusterError};

use crate::market::Level;
use crate::normalized_app::{AppMessageEncoder, Decimal, L2BookEncoder, Source, sbe_rt};

const APP_NAME: &str = "cluster-ha-orderbook";
/// Header-inclusive SessionMessageHeader length (prefer generated const).
pub const MSG_HDR_TOTAL: usize = SessionMessageHeaderEncoder::ENCODED_LENGTH;
pub const SESSION_MSG_HDR_TEMPLATE_ID: u16 = SessionMessageHeaderEncoder::TEMPLATE_ID;

/// Outcome of one try_claim publish attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    Published,
    Dropped,
    EncodeFailed,
}

/// Ingress that can claim a payload region after SessionMessageHeader.
/// Production: [`AeronCluster`]; tests: [`RecordingClaimIngress`].
pub trait ClaimIngress {
    /// Claim `payload_len` application bytes (header written by implementor).
    /// `fill` receives only the payload slice.
    fn try_claim_app<F>(&mut self, payload_len: usize, fill: F) -> PublishOutcome
    where
        F: FnOnce(&mut [u8]) -> Result<(), sbe_rt::EncodeError>;
}

/// Offline / test ingress: records full frames (header + payload).
#[derive(Default)]
pub struct RecordingClaimIngress {
    pub committed: Vec<Vec<u8>>,
    pub claim_attempts: usize,
    pub fail_next: bool,
    pub leadership_term_id: i64,
    pub cluster_session_id: i64,
}

impl RecordingClaimIngress {
    pub fn new(leadership_term_id: i64, cluster_session_id: i64) -> Self {
        Self {
            leadership_term_id,
            cluster_session_id,
            ..Self::default()
        }
    }
}

impl ClaimIngress for RecordingClaimIngress {
    fn try_claim_app<F>(&mut self, payload_len: usize, fill: F) -> PublishOutcome
    where
        F: FnOnce(&mut [u8]) -> Result<(), sbe_rt::EncodeError>,
    {
        self.claim_attempts += 1;
        if self.fail_next {
            self.fail_next = false;
            return PublishOutcome::Dropped;
        }
        let total = MSG_HDR_TOTAL + payload_len;
        let mut buf = vec![0u8; total];
        // Real ErgoSBE SessionMessageHeader (same encoder as AeronCluster::try_claim).
        {
            let mut enc = match SessionMessageHeaderEncoder::wrap_and_apply_header(
                &mut buf[..MSG_HDR_TOTAL],
                0,
            ) {
                Ok(e) => e,
                Err(_) => return PublishOutcome::EncodeFailed,
            };
            let _ = enc
                .leadership_term_id(self.leadership_term_id)
                .cluster_session_id(self.cluster_session_id)
                .timestamp(0);
        }
        match fill(&mut buf[MSG_HDR_TOTAL..]) {
            Ok(()) => {
                self.committed.push(buf);
                PublishOutcome::Published
            }
            Err(_) => PublishOutcome::EncodeFailed,
        }
    }
}

/// Production adapter: real [`AeronCluster::try_claim`].
pub struct AeronClusterIngress<'a> {
    pub cluster: &'a mut AeronCluster,
}

impl ClaimIngress for AeronClusterIngress<'_> {
    fn try_claim_app<F>(&mut self, payload_len: usize, fill: F) -> PublishOutcome
    where
        F: FnOnce(&mut [u8]) -> Result<(), sbe_rt::EncodeError>,
    {
        let mut claim = match self.cluster.try_claim(payload_len) {
            Ok(c) => c,
            Err(ClusterError::Publication { .. }) | Err(ClusterError::NotConnected) => {
                return PublishOutcome::Dropped;
            }
            Err(_) => return PublishOutcome::Dropped,
        };
        match fill(claim.payload_mut()) {
            Ok(()) => match claim.commit() {
                Ok(_) => PublishOutcome::Published,
                Err(_) => PublishOutcome::Dropped,
            },
            Err(_) => {
                let _ = claim.abort();
                PublishOutcome::EncodeFailed
            }
        }
    }
}

/// Publishes L2 snapshots via try_claim-shaped ingress.
pub struct ClusterBookPublisher<I: ClaimIngress> {
    ingress: I,
}

impl<I: ClaimIngress> ClusterBookPublisher<I> {
    pub fn new(ingress: I) -> Self {
        Self { ingress }
    }

    pub fn into_ingress(self) -> I {
        self.ingress
    }

    pub fn ingress(&self) -> &I {
        &self.ingress
    }

    /// Publish one L2 book: SessionMessageHeader (via claim) + AppMessage(L2Book).
    pub fn publish_l2_snapshot(
        &mut self,
        symbol: &str,
        sequence: u64,
        exchange_ts_ns: u64,
        receive_ts_ns: u64,
        bids: &[Level],
        asks: &[Level],
    ) -> PublishOutcome {
        let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
            bids.len(),
            asks.len(),
            symbol.len(),
        );
        let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(
            APP_NAME.len(),
            inner_len,
        );
        self.ingress.try_claim_app(outer_len, |app_buf| {
            let mut app = AppMessageEncoder::wrap_and_apply_header(app_buf, 0)?;
            let _ = app.sent_ts(receive_ts_ns);
            let after = app.app_name(APP_NAME.as_bytes())?;
            // Nested SBE: exact inner header-inclusive length + encode into var-data.
            let _ =
                after.payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                    let mut enc = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
                    let _ = enc
                        .source(Source::Bitget)
                        .exchange_timestamp(exchange_ts_ns)
                        .receive_timestamp(receive_ts_ns)
                        .sequence(sequence);
                    let after = enc.try_bids(bids.len() as u16, |g| {
                        for l in bids {
                            g.try_add(|e| -> Result<(), sbe_rt::EncodeError> {
                                let _ = e
                                    .price_wire(Decimal::new(l.price.mantissa, l.price.exponent))
                                    .size_wire(Decimal::new(l.size.mantissa, l.size.exponent));
                                Ok(())
                            })?;
                        }
                        Ok::<(), sbe_rt::EncodeError>(())
                    })?;
                    let after = after.try_asks(asks.len() as u16, |g| {
                        for l in asks {
                            g.try_add(|e| -> Result<(), sbe_rt::EncodeError> {
                                let _ = e
                                    .price_wire(Decimal::new(l.price.mantissa, l.price.exponent))
                                    .size_wire(Decimal::new(l.size.mantissa, l.size.exponent));
                                Ok(())
                            })?;
                        }
                        Ok::<(), sbe_rt::EncodeError>(())
                    })?;
                    let _complete = after.symbol(symbol.as_bytes())?;
                    Ok(())
                })?;
            Ok(())
        })
    }
}

#[must_use]
pub fn session_header_template_id(frame: &[u8]) -> Option<u16> {
    if frame.len() < 8 {
        return None;
    }
    Some(u16::from_le_bytes([frame[2], frame[3]]))
}

/// Application payload after a full SessionMessageHeader (generated framing helper).
#[must_use]
pub fn app_payload(frame: &[u8]) -> Option<&[u8]> {
    SessionMessageHeaderEncoder::after_this_message(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::WireDec;

    fn lvl(p: i64, s: i64) -> Level {
        Level {
            price: WireDec::new(p, -2),
            size: WireDec::new(s, -4),
        }
    }

    #[test]
    fn try_claim_publish_writes_session_header_and_app_payload()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut pubr = ClusterBookPublisher::new(RecordingClaimIngress::new(7, 42));
        let o =
            pubr.publish_l2_snapshot("BTCUSDT", 1, 1_000, 1_100, &[lvl(100, 1)], &[lvl(101, 2)]);
        assert_eq!(o, PublishOutcome::Published);
        let frames = &pubr.ingress().committed;
        assert_eq!(frames.len(), 1);
        let frame = &frames[0];
        assert_eq!(
            session_header_template_id(frame),
            Some(SESSION_MSG_HDR_TEMPLATE_ID)
        );
        let term = i64::from_le_bytes(frame[8..16].try_into()?);
        assert_eq!(term, 7);
        let csid = i64::from_le_bytes(frame[16..24].try_into()?);
        assert_eq!(csid, 42);
        let payload = app_payload(frame).expect("payload");
        assert!(payload.len() > 8);
        assert_eq!(u16::from_le_bytes([payload[2], payload[3]]), 1);
        Ok(())
    }
}
