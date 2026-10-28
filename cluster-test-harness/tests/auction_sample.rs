#![allow(missing_docs, dead_code)]
//! Client for the official `BasicAuctionClusteredService`.
//!
//! Bid layout matches the sample: little-endian `correlationId`, `customerId`,
//! `price` (24 bytes). The egress response appends a success byte (25 bytes).
//! A bid wins only when its price is strictly greater than the current best,
//! which starts at 0. Connect uses the node's explicit ingress URI. The Java
//! sample's `ingressChannel("aeron:udp")` plus an endpoint map, and
//! `egressChannel("...endpoint=localhost:0")`, are a separate client gap.

mod common;

use std::time::{Duration, Instant};

use ergo_aeron_cluster::{AeronCluster, ClusterError, SessionBuilder};
use serial_test::serial;

use common::Capture;

const BID_LENGTH: usize = 24;
const RESPONSE_LENGTH: usize = 25;

fn connect(
    cluster: &ergo_aeron_cluster_test_harness::TestCluster,
) -> Result<AeronCluster, Box<dyn std::error::Error>> {
    let dir = cluster
        .aeron_dir()
        .to_str()
        .ok_or("aeron dir is not valid UTF-8")?;
    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel(cluster.egress_channel.clone())?
        .message_timeout(Duration::from_secs(5))?;
    AeronCluster::connect(&builder, dir).map_err(Into::into)
}

fn bid_bytes(correlation_id: i64, customer_id: i64, price: i64) -> [u8; BID_LENGTH] {
    let mut buf = [0u8; BID_LENGTH];
    buf[0..8].copy_from_slice(&correlation_id.to_le_bytes());
    buf[8..16].copy_from_slice(&customer_id.to_le_bytes());
    buf[16..24].copy_from_slice(&price.to_le_bytes());
    buf
}

fn offer_until(
    client: &mut AeronCluster,
    payload: &[u8],
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + timeout;
    loop {
        match client.offer(payload) {
            Ok(_) => return Ok(()),
            Err(ClusterError::Publication { failure, .. }) if failure.is_retryable() => {}
            Err(e) => return Err(e.into()),
        }
        if Instant::now() >= deadline {
            return Err("offer stayed back-pressured".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn parse_response(buf: &[u8]) -> Option<(i64, i64, i64, u8)> {
    if buf.len() < RESPONSE_LENGTH {
        return None;
    }
    Some((
        i64::from_le_bytes(buf[0..8].try_into().ok()?),
        i64::from_le_bytes(buf[8..16].try_into().ok()?),
        i64::from_le_bytes(buf[16..24].try_into().ok()?),
        buf[24],
    ))
}

/// Offer one bid and return `(winning_customer, best_price, succeeded)`.
fn bid(
    client: &mut AeronCluster,
    listener: &mut Capture,
    correlation_id: i64,
    customer_id: i64,
    price: i64,
) -> Result<(i64, i64, bool), Box<dyn std::error::Error>> {
    let before = listener.messages.len();
    offer_until(
        client,
        &bid_bytes(correlation_id, customer_id, price),
        Duration::from_secs(2),
    )?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let _ = client.send_keep_alive();
        let _ = client.poll(listener, 10);
        if let Some(msg) = listener.messages[before..].iter().find_map(|m| {
            let parsed = parse_response(m)?;
            (parsed.0 == correlation_id).then_some(parsed)
        }) {
            return Ok((msg.1, msg.2, msg.3 == 1));
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "no auction response for correlation {correlation_id} (got {} messages)",
                listener.messages.len().saturating_sub(before)
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[serial]
fn auction_higher_bid_wins_and_lower_bid_loses() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node_auction()?;
    let mut client = connect(&cluster)?;
    let mut listener = Capture::new();

    let (winner, price, won) = bid(&mut client, &mut listener, 1, 7, 10)?;
    assert_eq!((winner, price, won), (7, 10, true));

    let (winner, price, won) = bid(&mut client, &mut listener, 2, 8, 10)?;
    assert_eq!((winner, price, won), (7, 10, false));

    let (winner, price, won) = bid(&mut client, &mut listener, 3, 9, 11)?;
    assert_eq!((winner, price, won), (9, 11, true));

    client.close()?;
    Ok(())
}
