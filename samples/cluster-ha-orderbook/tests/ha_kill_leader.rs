//! Multi-node Java kill-leader harness: never-stale book across NewLeader.
//!
//! Requires `test-harness` feature (Java ClusterLauncher jars). Proves:
//! 1. Pre-failover L2 snapshot serves via echo path
//! 2. Kill elected leader → NewLeader → follower marks not serving
//! 3. Post-failover snapshot restores serving with reference equality
//!
//! ```sh
//! cd samples/cluster-ha-orderbook && \
//!   cargo test --features test-harness --test ha_kill_leader -- --test-threads=1 --nocapture
//! ```

#![cfg(feature = "test-harness")]

use rusteron_client::cformat;
use std::error::Error;
use std::time::{Duration, Instant};

use cluster_ha_orderbook::follower::BookFollower;
use cluster_ha_orderbook::market::{Level, WireDec};
use cluster_ha_orderbook::publish::{
    ClusterBookPublisher, PublishOutcome, RecordingClaimIngress, app_payload,
};
use ergo_aeron_cluster::SessionState;
use ergo_aeron_cluster::cluster_codec_types::{AdminRequestType, AdminResponseCode, EventCode};
use ergo_aeron_cluster::egress::{EgressAdapter, EgressListener};
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use serial_test::serial;

fn lvl(p: i64, s: i64) -> Level {
    Level {
        price: WireDec::new(p, -2),
        size: WireDec::new(s, -4),
    }
}

/// Own embedded driver for the client (must outlive the client).
struct OwnDriver {
    dir: String,
    _guard: rusteron_media_driver::EmbeddedMediaDriver,
}

fn launch_own_driver(tag: &str) -> OwnDriver {
    let dir = std::env::temp_dir().join(format!("{tag}-{pid}", pid = std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let dir_cstr = cformat!("{}", dir.display());
    let dc = rusteron_media_driver::AeronDriverContext::new().unwrap();
    dc.set_dir(&dir_cstr).unwrap();
    dc.set_dir_delete_on_shutdown(true).unwrap();
    dc.set_dir_delete_on_start(true).unwrap();
    let _guard = rusteron_media_driver::AeronDriver::launch_embedded_guard(dc, false);
    OwnDriver {
        dir: dir.to_str().unwrap().to_string(),
        _guard,
    }
}

fn connect_own_driver(
    cluster_ingress: &str,
    egress_port: u16,
    aeron_dir: &str,
) -> Result<AeronCluster, ergo_aeron_cluster::ClusterError> {
    let builder = SessionBuilder::default()
        .ingress_channel(cluster_ingress.to_string())
        .egress_channel(format!("aeron:udp?endpoint=localhost:{egress_port}"))
        .ingress_stream_id(101)
        .egress_stream_id(102)
        .message_timeout(Duration::from_secs(10));
    AeronCluster::connect(&builder, aeron_dir)
}

/// Captures app echoes and NewLeader; drives BookFollower on each message.
struct BookCapture {
    follower: BookFollower,
    new_leader_member_id: Option<i32>,
    last_term: i64,
    apply_errors: u64,
}

impl BookCapture {
    fn new() -> Self {
        Self {
            follower: BookFollower::new(),
            new_leader_member_id: None,
            last_term: 0,
            apply_errors: 0,
        }
    }
}

impl EgressListener for BookCapture {
    fn on_message(&mut self, _csid: i64, _ts: i64, buffer: &[u8]) {
        let term = self.last_term;
        match self.follower.on_app_payload(term, buffer) {
            Ok(_) => {}
            Err(_) => self.apply_errors += 1,
        }
    }
    fn on_session_event(
        &mut self,
        _: i64,
        _: i64,
        leadership_term_id: i64,
        _: i32,
        _: EventCode,
        _: &str,
    ) {
        if leadership_term_id > 0 {
            self.last_term = leadership_term_id;
        }
    }
    fn on_new_leader(&mut self, _: i64, leadership_term_id: i64, leader_member_id: i32, _: &str) {
        self.last_term = leadership_term_id;
        if self.new_leader_member_id.is_none() {
            self.new_leader_member_id = Some(leader_member_id);
        }
        self.follower.on_leadership_release();
    }
    fn on_challenge(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_admin_response(
        &mut self,
        _: i64,
        _: i64,
        _: AdminRequestType,
        _: AdminResponseCode,
        _: &str,
        _: &[u8],
    ) {
    }
}

/// Encode one L2 snapshot AppMessage payload (no SessionMessageHeader).
fn encode_l2_payload(
    term: i64,
    symbol: &str,
    seq: u64,
    bids: &[Level],
    asks: &[Level],
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut pubr = ClusterBookPublisher::new(RecordingClaimIngress::new(term, 1));
    let o = pubr.publish_l2_snapshot(symbol, seq, 1_000, 1_100, bids, asks);
    if o != PublishOutcome::Published {
        return Err("encode publish failed".into());
    }
    let frame = &pubr.ingress().committed[0];
    let payload = app_payload(frame).ok_or("missing payload")?.to_vec();
    Ok(payload)
}

fn await_serving(
    client: &mut AeronCluster,
    adapter: &mut EgressAdapter<BookCapture>,
    timeout: Duration,
) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + timeout;
    loop {
        let _ = client.send_keep_alive();
        let _ = client.poll_egress(adapter, 10);
        if adapter.listener().follower.book().is_serving() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("book not serving within timeout".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[serial]
fn kill_leader_never_serves_stale_book() -> Result<(), Box<dyn Error>> {
    let mut cluster = ergo_aeron_cluster::TestCluster::three_node();
    let driver = launch_own_driver("ha-kill");
    let mut client = connect_own_driver(&cluster.ingress_channel, 19300, &driver.dir)?;
    assert_eq!(client.state(), SessionState::Connected);

    let initial_leader = client.leader_member_id();
    assert!((0..3).contains(&initial_leader));

    let mut adapter = EgressAdapter::new(BookCapture::new());
    // Seed term from client after connect.
    adapter.listener_mut().last_term = client.leadership_term_id();

    // --- Pre-failover snapshot ---
    let bids1 = vec![lvl(500, 1)];
    let asks1 = vec![lvl(501, 2)];
    let payload1 = encode_l2_payload(client.leadership_term_id(), "BTCUSDT", 1, &bids1, &asks1)?;
    client.offer(&payload1)?;
    await_serving(&mut client, &mut adapter, Duration::from_secs(15))?;
    assert!(adapter.listener().follower.book().is_serving());
    let live = adapter
        .listener()
        .follower
        .book()
        .live_image()
        .expect("live pre-failover");
    assert_eq!(live.symbol, "BTCUSDT");
    assert_eq!(live.bids[0].price.mantissa, 500);

    // --- Kill actual leader ---
    cluster.kill_node(initial_leader as usize);

    // Poll until NewLeader + Connected
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let _ = client.send_keep_alive();
        client.poll_egress(&mut adapter, 10)?;
        if adapter.listener().new_leader_member_id.is_some()
            && client.state() == SessionState::Connected
        {
            break;
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "no NewLeader+Connected within 60s (state={:?}, new_leader={:?})",
                client.state(),
                adapter.listener().new_leader_member_id
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Never-stale: release must have frozen serving.
    assert!(
        !adapter.listener().follower.book().is_serving(),
        "book must not serve across leadership release"
    );
    assert!(
        adapter.listener().follower.book().live_image().is_none(),
        "live_image must be None while not serving"
    );

    let new_leader = adapter.listener().new_leader_member_id.expect("new leader");
    assert_ne!(new_leader, initial_leader);
    assert_eq!(client.leader_member_id(), new_leader);

    // --- Post-failover reference snapshot ---
    let ref_bids = vec![lvl(510, 3), lvl(509, 4)];
    let ref_asks = vec![lvl(511, 5)];
    let payload2 = encode_l2_payload(
        client.leadership_term_id(),
        "BTCUSDT",
        1,
        &ref_bids,
        &ref_asks,
    )?;
    client.offer(&payload2)?;
    await_serving(&mut client, &mut adapter, Duration::from_secs(15))?;

    let live = adapter
        .listener()
        .follower
        .book()
        .live_image()
        .expect("live post-failover");
    assert_eq!(live.bids.len(), 2);
    assert_eq!(live.asks.len(), 1);
    assert_eq!(live.bids[0].price.mantissa, 510);
    assert_eq!(live.asks[0].price.mantissa, 511);
    // Must not have merged old-term levels (500/501).
    assert!(
        live.bids.iter().all(|b| b.price.mantissa != 500),
        "stale bid 500 must not appear after resync"
    );

    let _ = client.close();
    Ok(())
}
