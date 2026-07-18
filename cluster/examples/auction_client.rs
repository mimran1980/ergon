//! Auction cluster client — ported from `BasicAuctionClusterClient.java`.
//!
//! ```bash
//! cargo run --example auction_client --features test-harness
//! ```

use ergo_aeron_cluster::codecs::cluster_codecs::{
    WriteBuf,
    session_connect_request_codec::SessionConnectRequestEncoder,
    session_message_header_codec::{SBE_BLOCK_LENGTH, SessionMessageHeaderEncoder},
};
use std::ffi::CString;
use std::time::Duration;

// Message layout matching `BasicAuctionClusteredService`:
//   BID: correlationId(8) + customerId(8) + price(8) = 24 bytes
const CORRELATION_ID_OFFSET: usize = 0;
const CUSTOMER_ID_OFFSET: usize = 8;
const PRICE_OFFSET: usize = 16;
const BID_MESSAGE_LENGTH: usize = 24;

fn main() {
    let customer_id: u64 = 100;
    let num_bids: u64 = 5;
    println!("=== Ergo Aeron Cluster Auction Client ===\n");
    println!("Customer ID: {customer_id}, Bids to send: {num_bids}");

    let cluster = ergo_aeron_cluster_test_support::TestCluster::single_node();
    let dir = CString::new(cluster.aeron_dir().to_str().unwrap()).unwrap();
    let ctx = rusteron_client::AeronContext::new().unwrap();
    ctx.set_dir(&dir).unwrap();
    let a = rusteron_client::Aeron::new(&ctx).unwrap();
    a.start().unwrap();

    let ing = CString::new(&cluster.ingress_channel[..]).unwrap();
    let egr = CString::new(&cluster.egress_channel[..]).unwrap();
    let egress = a
        .add_subscription(
            &egr,
            102,
            rusteron_client::Handlers::NONE,
            rusteron_client::Handlers::NONE,
            Duration::from_secs(5),
        )
        .unwrap();
    let ingress = a.add_publication(&ing, 101, Duration::from_secs(5)).unwrap();

    // Connect + get session ID
    {
        let mut buf = vec![0u8; 512];
        let wb = WriteBuf::new(&mut buf);
        let mut enc = SessionConnectRequestEncoder::default().wrap(wb, 8);
        enc.correlation_id(1);
        enc.response_stream_id(102);
        enc.version(0);
        enc.response_channel(cluster.egress_channel.as_bytes());
        enc.encoded_credentials(b"");
        let _h = enc.header(0);
        for _ in 0..20 {
            if ingress.offer_raw(&buf, rusteron_client::Handlers::NONE) > 0 {
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

    for bid_num in 0..num_bids {
        let price = last_bid_price + (bid_num + 1) * 10;
        let cid = next_correlation_id;
        next_correlation_id += 1;

        // Build bid message with SessionMessageHeader
        let mut msg = vec![0u8; 512];
        let hdr_len: usize;
        {
            let wb = WriteBuf::new(&mut msg);
            let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            enc.leadership_term_id(leadership_term_id);
            enc.cluster_session_id(cluster_session_id);
            enc.timestamp(0);
            let _h = enc.header(0);
            hdr_len = 8 + SBE_BLOCK_LENGTH as usize;
        }
        // Write bid payload after header
        msg[hdr_len + CORRELATION_ID_OFFSET..hdr_len + CORRELATION_ID_OFFSET + 8].copy_from_slice(&cid.to_le_bytes());
        msg[hdr_len + CUSTOMER_ID_OFFSET..hdr_len + CUSTOMER_ID_OFFSET + 8].copy_from_slice(&customer_id.to_le_bytes());
        msg[hdr_len + PRICE_OFFSET..hdr_len + PRICE_OFFSET + 8].copy_from_slice(&price.to_le_bytes());
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
                                    let r_price = u64::from_le_bytes(app[16..24].try_into().unwrap());
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
}
