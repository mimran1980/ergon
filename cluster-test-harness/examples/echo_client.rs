//! Echo client using the `AeronCluster` library API.
//!
//! ```bash
//! cargo run -p ergo-aeron-cluster-test-harness --example echo_client
//! ```

use ergo_aeron_cluster::{
    AeronCluster, NullCredentialsSupplier, SessionBuilder,
    cluster_codec_types::EventCode,
    egress::{EgressAdapter, EgressListener},
};
use std::time::Duration;

struct EchoListener {
    count: usize,
}

impl EgressListener for EchoListener {
    fn on_message(&mut self, _csid: i64, _ts: i64, buf: &[u8]) {
        self.count += 1;
        println!(
            "  Echo #{}: {:?}",
            self.count,
            std::str::from_utf8(buf).unwrap_or("<binary>")
        );
    }
    fn on_session_event(
        &mut self,
        _cid: i64,
        _sid: i64,
        _tid: i64,
        _mid: i32,
        code: EventCode,
        _d: &str,
    ) {
        println!("  SessionEvent: {code:?}");
    }
    fn on_new_leader(&mut self, _sid: i64, _tid: i64, _mid: i32, _eps: &str) {}
    fn on_challenge(&mut self, _cid: i64, _sid: i64, _c: &[u8]) {}
    fn on_admin_response(
        &mut self,
        _sid: i64,
        _cid: i64,
        _rt: ergo_aeron_cluster::cluster_codec_types::AdminRequestType,
        _rc: ergo_aeron_cluster::cluster_codec_types::AdminResponseCode,
        _m: &str,
        _p: &[u8],
    ) {
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Ergo Aeron Cluster Echo Client (library API) ===\n");

    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node();

    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel(cluster.egress_channel.clone())?
        .message_timeout(Duration::from_secs(5))?;

    let aeron_dir = cluster
        .aeron_dir()
        .to_str()
        .ok_or("Aeron directory is not valid UTF-8")?;
    let mut client = AeronCluster::connect(&builder, aeron_dir)?;
    println!(
        "Connected: session={} term={}",
        client.cluster_session_id(),
        client.leadership_term_id()
    );

    let mut adapter = EgressAdapter::new(EchoListener { count: 0 });

    for n in 0..3 {
        let msg = format!("Hello #{n}");
        // retry offer until accepted (publication may need a moment)
        let mut sent = false;
        for _ in 0..20 {
            if client.offer(msg.as_bytes()).is_ok() {
                sent = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if !sent {
            eprintln!("  failed to offer message {n}");
        }

        // poll for the echo
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            let _ = client.poll_egress(&mut adapter, 10);
            if adapter.listener().count > n as usize {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    println!("\nSending keep-alive...");
    let _ = client.send_keep_alive();
    println!("Closing session...");
    let _ = client.close();
    println!("\n=== Done ===");

    Ok(())
}

// suppress unused-import warning for NullCredentialsSupplier when not used
#[allow(dead_code)]
fn _unused() -> NullCredentialsSupplier {
    NullCredentialsSupplier
}
