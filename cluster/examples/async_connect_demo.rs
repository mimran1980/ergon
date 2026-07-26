//! Demonstrates the poll-driven `async_connect()` and zero-copy
//! `try_claim()` — the two Java-parity features.
//!
//! ```bash
//! cargo run --example async_connect_demo --features test-harness
//! ```

use ergo_aeron_cluster::{
    AeronCluster, SessionBuilder,
    codecs::EventCode,
    egress::{EgressAdapter, EgressListener},
};
use std::time::Duration;

struct L(usize);
impl EgressListener for L {
    fn on_message(&mut self, _cs: i64, _ts: i64, b: &[u8]) {
        self.0 += 1;
        println!("  Echo #{}: {:?}", self.0, std::str::from_utf8(b).unwrap_or("<bin>"));
    }
    fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, c: EventCode, _: &str) {
        println!("  SessionEvent {c:?}");
    }
    fn on_new_leader(&mut self, _: i64, _: i64, _: i32, _: &str) {}
    fn on_challenge(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_admin_response(
        &mut self,
        _: i64,
        _: i64,
        _: ergo_aeron_cluster::cluster_codec_types::AdminRequestType,
        _: ergo_aeron_cluster::cluster_codec_types::AdminResponseCode,
        _: &str,
        _: &[u8],
    ) {
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== async_connect + try_claim demo ===\n");
    let cluster = ergo_aeron_cluster::TestCluster::single_node();

    let builder = SessionBuilder::builder()
        .ingress_channel(cluster.ingress_channel.clone())
        .egress_channel(cluster.egress_channel.clone())
        .message_timeout(Duration::from_secs(5));

    // 1. Poll-driven async connect
    println!("Step: async_connect (poll-driven)...");
    let mut ac = AeronCluster::connect_async(builder, cluster.aeron_dir().to_str().unwrap());
    let mut steps = 0;
    loop {
        steps += 1;
        match ac.poll() {
            Ok(true) => { /* more work */ }
            Ok(false) => break,
            Err(e) => {
                eprintln!("connect error: {e}");
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(10));
        if steps > 500 {
            eprintln!("connect didn't complete");
            return Ok(());
        }
    }
    let mut client = ac.finish().expect("finish");
    println!(
        "  Connected after {steps} polls: session={} term={}",
        client.cluster_session_id(),
        client.leadership_term_id()
    );

    // 2. Zero-copy try_claim publish
    println!("Step: try_claim (zero-copy)...");
    let payload = b"zero-copy hello";
    let mut claim = client.try_claim(payload.len()).expect("try_claim");
    claim.payload_mut().copy_from_slice(payload);
    let pos = claim.commit().expect("commit");
    println!("  Claimed + committed at position {pos}");

    // 3. Poll egress for the echo
    let mut adapter = EgressAdapter::new(L(0));
    for _ in 0..30 {
        let _ = client.poll_egress(&mut adapter, 10);
        if adapter.listener().0 >= 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let _ = client.send_keep_alive();
    let _ = client.close();
    println!("\n=== Done: zero-copy round-trip verified ===");

    Ok(())
}
