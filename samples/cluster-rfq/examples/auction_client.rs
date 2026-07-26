//! Auction cluster client — ported from `BasicAuctionClusterClient.java`.
//!
//! ```bash
//! cargo run --example auction_client --features test-harness
//! ```

use ergo_aeron_cluster::cluster_codec_types::{
    SessionConnectRequestEncoder, SessionMessageHeaderEncoder,
};
use rusteron_client::cformat;
use std::time::Duration;

// Message layout matching `BasicAuctionClusteredService`:
//   BID: correlationId(8) + customerId(8) + price(8) = 24 bytes
const CORRELATION_ID_OFFSET: usize = 0;
const CUSTOMER_ID_OFFSET: usize = 8;
const PRICE_OFFSET: usize = 16;
const BID_MESSAGE_LENGTH: usize = 24;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let customer_id: u64 = 100;
    let num_bids: u64 = 5;
    println!("=== Ergo Aeron Cluster Auction Client ===\n");
    println!("Customer ID: {customer_id}, Bids to send: {num_bids}");

    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    let dir = cformat!("{}", cluster.aeron_dir().display());
    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&dir)?;
    let a = rusteron_client::Aeron::new(&ctx)?;
    a.start()?;

    let ing = cformat!("{}", cluster.ingress_channel);
    let egr = cformat!("{}", cluster.egress_channel);
    let egress = a.add_subscription(
        &egr,
        102,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(5),
    )?;
    let ingress = a.add_publication(&ing, 101, Duration::from_secs(5))?;

    // Connect + get session ID
    {
        // TODO: MUST use ergo-sbe EncodedLength, not a magic-sized buffer (CLAUDE.md hard rule)
        let mut buf = [0u8; 512];
        let mut enc = SessionConnectRequestEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
        enc.correlation_id(1).response_stream_id(102).version(0);
        let complete = enc
            .response_channel(cluster.egress_channel.as_bytes())?
            .encoded_credentials(b"")?
            .client_info(b"")?;
        let bytes = complete.as_bytes_with_header();
        for _ in 0..20 {
            if ingress.offer_raw(bytes, rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    let (cluster_session_id, leadership_term_id) = {
        let (mut csid, mut ltid) = (-1i64, -1i64);
        for _ in 0..30 {
            egress
                .poll_fn(
                    |data, _hdr| {
                        if data.len() >= 40 && u16::from_le_bytes([data[2], data[3]]) == 2 {
                            csid = i64::from_le_bytes(data[8..16].try_into().unwrap());
                            ltid = i64::from_le_bytes(data[24..32].try_into().unwrap());
                        }
                    },
                    10,
                )
                .ok();
            if csid >= 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        (csid, ltid)
    };

    // Send bids
    let mut last_bid_price: u64 = 100;
    let mut next_correlation_id: u64 = 0;

    #[allow(clippy::explicit_counter_loop)]
    for bid_num in 0..num_bids {
        let price = last_bid_price + (bid_num + 1) * 10;
        let cid = next_correlation_id;
        next_correlation_id += 1;

        // Build bid message with SessionMessageHeader
        // TODO: MUST use ergo-sbe EncodedLength, not a magic-sized buffer (CLAUDE.md hard rule)
        let mut msg = [0u8; 512];
        let mut enc = SessionMessageHeaderEncoder::try_wrap_and_apply_header(&mut msg, 0)?;
        enc.leadership_term_id(leadership_term_id)
            .cluster_session_id(cluster_session_id)
            .timestamp(0);
        let hdr_len = enc.as_ref().len();
        // Write bid payload after header
        msg[hdr_len + CORRELATION_ID_OFFSET..hdr_len + CORRELATION_ID_OFFSET + 8]
            .copy_from_slice(&cid.to_le_bytes());
        msg[hdr_len + CUSTOMER_ID_OFFSET..hdr_len + CUSTOMER_ID_OFFSET + 8]
            .copy_from_slice(&customer_id.to_le_bytes());
        msg[hdr_len + PRICE_OFFSET..hdr_len + PRICE_OFFSET + 8]
            .copy_from_slice(&price.to_le_bytes());
        let total = hdr_len + BID_MESSAGE_LENGTH;

        println!("Bid {bid_num}: cid={cid} customer={customer_id} price={price}");

        for _ in 0..10 {
            if ingress.offer_raw(&msg[..total], rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // Receive response (EchoService echoes back the bid message)
        let mut got_response = false;
        for _ in 0..30 {
            egress
                .poll_fn(
                    |data, _hdr| {
                        if !got_response && data.len() >= 8 {
                            let t = u16::from_le_bytes([data[2], data[3]]);
                            if t == 1 && data.len() >= 32 + 24 {
                                let app = &data[32..]; // skip headers
                                let r_cid = u64::from_le_bytes(app[0..8].try_into().unwrap());
                                if r_cid == cid {
                                    let r_cust = u64::from_le_bytes(app[8..16].try_into().unwrap());
                                    let r_price =
                                        u64::from_le_bytes(app[16..24].try_into().unwrap());
                                    println!("  Echoed: cid={r_cid} cust={r_cust} price={r_price}");
                                    last_bid_price = r_price;
                                    got_response = true;
                                }
                            }
                        }
                    },
                    10,
                )
                .ok();
            if got_response {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // Keep running poll to drain any remaining messages
        let _ = egress.poll_fn(|_, _| {}, 1).ok();
    }

    println!("\n=== Done: final price = {last_bid_price} ===");

    Ok(())
}
