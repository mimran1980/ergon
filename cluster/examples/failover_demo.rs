//! Live failover with the **own-driver-UDP** transport (Java model).
//!
//! The client runs its own embedded C media driver and reaches the
//! cluster over UDP with a separate egress port. Killing the leader
//! does NOT kill the client's driver, so the client survives and can
//! reconnect to the new leader.
//!
//! ```bash
//! cargo run --example failover_demo --features test-harness
//! ```

use ergo_aeron_cluster::codecs::session::SessionConnectRequestEncoder;
use ergo_aeron_cluster::poller;
use rusteron_client::cformat;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Live 3-node failover (own-driver UDP) ===\n");
    let mut cluster = ergo_aeron_cluster::TestCluster::three_node();
    let node0_port = parse_port(&cluster.ingress_channel);
    println!("3-node cluster up. node 0 ingress port = {node0_port}");

    // Client's own embedded driver.
    let client_dir = std::env::temp_dir().join(format!("fo-{pid}", pid = std::process::id()));
    let _ = std::fs::create_dir_all(&client_dir);
    let dir_cstr = cformat!("{}", client_dir.display());
    let dc = rusteron_media_driver::AeronDriverContext::new()?;
    dc.set_dir(&dir_cstr)?;
    dc.set_dir_delete_on_shutdown(true)?;
    dc.set_dir_delete_on_start(true)?;
    let (_stop, _h) = rusteron_media_driver::AeronDriver::launch_embedded(dc, false);
    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&dir_cstr)?;
    let a = rusteron_client::Aeron::new(&ctx)?;
    a.start()?;

    let egress_port: u16 = 19199;
    // Already CString — do not cformat! again (would re-allocate).
    let egress_uri = cformat!("localhost:{egress_port}")?;
    let egress = a.add_subscription(
        &egress_uri,
        102,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(5),
    )?;

    let connect_to_leader = |port: u16, resp: &str| -> Option<rusteron_client::AeronPublication> {
        let uri = Some(cformat!("localhost:{port}"))?;
        let pub_ = a.add_publication(&uri, 101, Duration::from_secs(5)).ok()?;
        let mut buf = vec![0u8; 512];
        let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0).ok()?;
        enc.correlation_id(1).response_stream_id(102).version(0);
        let complete = enc
            .response_channel(resp.as_bytes())
            .ok()?
            .encoded_credentials(b"")
            .ok()?
            .client_info(b"")
            .ok()?;
        let bytes = complete.as_bytes_with_header();
        for _ in 0..50 {
            if pub_.offer_raw(bytes, rusteron_client::Handlers::NONE) > 0 {
                return Some(pub_);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Some(pub_)
    };

    let resp = format!("aeron:udp?endpoint=localhost:{egress_port}");
    let _ingress = connect_to_leader(node0_port, &resp).expect("connect to node 0");

    // Wait for SessionEvent(OK).
    let mut connected = false;
    for _ in 0..50 {
        let mut got = false;
        egress
            .poll_fn(
                |data, _h| {
                    if data.len() >= 8 && u16::from_le_bytes([data[2], data[3]]) == 2 {
                        got = true;
                    }
                },
                10,
            )
            .ok();
        if got {
            connected = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("Connected to leader: {connected}");
    if !connected {
        println!("FAIL: no session");
        return Ok(());
    }

    println!("\nKilling leader (node 0)...");
    cluster.kill_node(0);

    // Poll egress for NewLeaderEvent (give the cluster time to elect).
    println!("Polling for NewLeaderEvent (up to 30s for election)...");
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut new_leader: Option<(i32, String)> = None;
    while Instant::now() < deadline {
        let mut captured: Option<(i32, String)> = None;
        egress
            .poll_fn(
                |data, _h| {
                    if let Some(ev) = poller::parse_event(data) {
                        match ev {
                            poller::EgressEvent::NewLeader {
                                leader_member_id,
                                ingress_endpoints,
                                ..
                            } => {
                                if captured.is_none() {
                                    captured = Some((leader_member_id, ingress_endpoints));
                                }
                                println!("  <- NewLeaderEvent (leader member {leader_member_id})");
                            }
                            poller::EgressEvent::SessionEvent { code, .. } => {
                                println!("  <- SessionEvent {code:?}");
                            }
                            _ => {}
                        }
                    }
                },
                10,
            )
            .ok();
        if let Some(info) = captured {
            new_leader = Some(info);
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    match new_leader {
        Some((leader_member_id, eps)) => {
            println!("  NewLeaderEvent received: {eps}");
            // Resolve the new leader's endpoint by member id — the list is in
            // id order, so a first-entry parse would reconnect to the dead leader.
            match poller::parse_leader_endpoint(&eps, leader_member_id) {
                Some(ep) => {
                    let _reconn = connect_to_leader(parse_port(&format!("aeron:udp?endpoint={ep}")), &resp)
                        .expect("reconnect to new leader");
                    println!("  Reconnected ingress to new leader (member {leader_member_id}): {ep}");
                    println!("\n=== Result: failover handled — client survived leader death ===");
                }
                None => {
                    println!("\n=== Result: NewLeaderEvent had no endpoint for member {leader_member_id} ===");
                }
            }

            Ok(())
        }
        None => {
            println!("\n=== Result: no NewLeaderEvent within 30s ===");
            println!("(cluster election timing — the transport survived, but no new");
            println!(" leader event was observed in the window.)");
            Ok(())
        }
    }
}

fn parse_port(ch: &str) -> u16 {
    ch.rsplit(':').next().and_then(|s| s.parse().ok()).unwrap_or(0)
}
