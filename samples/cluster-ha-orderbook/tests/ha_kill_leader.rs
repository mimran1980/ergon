//! Multi-node Java kill-leader harness: the book never serves across a failover.
//!
//! Requires `test-harness` feature (Java ClusterLauncher jars). Over the live
//! echo cluster it proves:
//! 1. A pre-failover L2 snapshot serves.
//! 2. After the leader is killed, the book stops serving from the first poll
//!    that shows the session loss (the client leaves `Connected` or
//!    `NewLeader` arrives), and stays stopped on every poll until recovery.
//! 3. On the new leader, a replayed old-term frame and a delayed old-term
//!    recovery snapshot (newer sequence) are both rejected as stale epochs.
//! 4. A new-term snapshot restores serving with the reference image, never
//!    merged with the old levels.
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
use cluster_ha_orderbook::ha_book::{ApplyOutcome, RejectReason};
use cluster_ha_orderbook::market::{L2Snapshot, Level, WireDec};
use cluster_ha_orderbook::publish::{
    ClusterBookPublisher, PublishOutcome, RecordingClaimIngress, app_payload,
};
use ergo_aeron_cluster::SessionState;
use ergo_aeron_cluster::egress::EgressListener;
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use ergo_aeron_cluster_test_harness::TestCluster;
use serial_test::serial;

fn lvl(p: i64, s: i64) -> Level {
    Level {
        price: WireDec::new(p, -2),
        size: WireDec::new(s, -4),
    }
}

fn snap(epoch: i64, sequence: u64, bids: Vec<Level>, asks: Vec<Level>) -> L2Snapshot {
    L2Snapshot {
        epoch,
        sequence,
        symbol: "BTCUSDT".to_owned(),
        exchange_ts_ns: 1_000,
        bids,
        asks,
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
        .ingress_channel(cluster_ingress)?
        .egress_channel(format!("aeron:udp?endpoint=localhost:{egress_port}"))?
        .ingress_stream_id(101)
        .egress_stream_id(102)
        .message_timeout(Duration::from_secs(10))?;
    AeronCluster::connect(&builder, aeron_dir)
}

/// Feeds echoed app payloads to the follower and reports session loss on
/// `NewLeader`; [`poll_once`] reports state changes the poll loop sees.
struct BookCapture {
    follower: BookFollower,
    /// The follower was told the session is down and not yet told it is up.
    session_down: bool,
    new_leader_member_id: Option<i32>,
    outcomes: Vec<ApplyOutcome>,
    apply_errors: u64,
}

impl BookCapture {
    fn new() -> Self {
        Self {
            follower: BookFollower::new(),
            session_down: true,
            new_leader_member_id: None,
            outcomes: Vec::new(),
            apply_errors: 0,
        }
    }

    fn lose_session(&mut self) {
        self.follower.on_session_lost();
        self.session_down = true;
    }
}

impl EgressListener for BookCapture {
    fn on_message(&mut self, _csid: i64, _ts: i64, buffer: &[u8]) {
        match self.follower.on_app_payload(buffer) {
            Ok(o) => self.outcomes.push(o),
            Err(_) => self.apply_errors += 1,
        }
    }
    fn on_new_leader(&mut self, _: i64, _: i64, leader_member_id: i32, _: &str) {
        if self.new_leader_member_id.is_none() {
            self.new_leader_member_id = Some(leader_member_id);
        }
        self.lose_session();
    }
}

/// One keep-alive + poll, then tell the follower what the session state
/// shows: lost as soon as the client leaves `Connected`, up again (on the
/// client's new term) once it is back.
fn poll_once(client: &mut AeronCluster, listener: &mut BookCapture) -> Result<(), Box<dyn Error>> {
    let _ = client.send_keep_alive();
    client.poll(listener, 10)?;
    let connected = client.state() == SessionState::Connected;
    if connected && listener.session_down {
        listener.follower.on_session_up(client.leadership_term_id());
        listener.session_down = false;
    } else if !connected && !listener.session_down {
        listener.lose_session();
    }
    Ok(())
}

/// Poll until `done` holds; `done` may fail the test early.
fn poll_until(
    client: &mut AeronCluster,
    listener: &mut BookCapture,
    timeout: Duration,
    what: &str,
    mut done: impl FnMut(&AeronCluster, &BookCapture) -> Result<bool, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + timeout;
    loop {
        poll_once(client, listener)?;
        if done(client, listener)? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "{what}: not reached within {timeout:?} (state={:?}, new_leader={:?})",
                client.state(),
                listener.new_leader_member_id
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Fails once the book serves while it must not.
fn not_serving(listener: &BookCapture, phase: &str) -> Result<(), Box<dyn Error>> {
    let book = listener.follower.book();
    if book.is_serving() || book.live_image().is_some() {
        return Err(format!("book served during {phase}: {:?}", book.live_image()).into());
    }
    Ok(())
}

/// Encode one AppMessage(L2Book) payload (no SessionMessageHeader).
fn encode_payload(snapshot: &L2Snapshot) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut pubr = ClusterBookPublisher::new(RecordingClaimIngress::new(snapshot.epoch, 1));
    if pubr.publish_snapshot(snapshot, 1_100) != PublishOutcome::Published {
        return Err("encode publish failed".into());
    }
    let frame = &pubr.ingress().committed[0];
    Ok(app_payload(frame).ok_or("missing payload")?.to_vec())
}

#[test]
#[serial]
fn kill_leader_never_serves_stale_book() -> Result<(), Box<dyn Error>> {
    let mut cluster = TestCluster::three_node()?;
    let driver = launch_own_driver("ha-kill");
    let mut client = connect_own_driver(&cluster.ingress_channel, 19300, &driver.dir)?;
    assert_eq!(client.state(), SessionState::Connected);

    let initial_leader = client.leader_member_id();
    assert!((0..3).contains(&initial_leader));
    let old_term = client.leadership_term_id();

    let mut listener = BookCapture::new();
    listener.follower.on_session_up(old_term);
    listener.session_down = false;

    // --- Pre-failover snapshot ---
    let pre = snap(old_term, 1, vec![lvl(500, 1)], vec![lvl(501, 2)]);
    let pre_payload = encode_payload(&pre)?;
    client.offer(&pre_payload)?;
    poll_until(
        &mut client,
        &mut listener,
        Duration::from_secs(15),
        "pre-failover serving",
        |_, l| Ok(l.follower.book().is_serving()),
    )?;
    assert_eq!(listener.follower.book().live_image(), Some(&pre));

    // --- Kill the leader; never serve from the first observed loss ---
    cluster.kill_node(initial_leader as usize);
    let mut polls_since_loss = 0u32;
    let mut loss_seen_as = None;
    poll_until(
        &mut client,
        &mut listener,
        Duration::from_secs(60),
        "NewLeader + Connected",
        |c, l| {
            if l.follower.losses > 0 {
                loss_seen_as.get_or_insert(if l.new_leader_member_id.is_some() {
                    "NewLeader"
                } else {
                    "state left Connected"
                });
                not_serving(l, "failover")?;
                polls_since_loss += 1;
            }
            Ok(l.new_leader_member_id.is_some() && c.state() == SessionState::Connected)
        },
    )?;
    println!(
        "loss first seen as {:?}; not serving on {polls_since_loss} polls to reconnect",
        loss_seen_as
    );
    assert!(polls_since_loss > 0, "session loss was never observed");
    not_serving(&listener, "reconnect")?;

    let new_leader = listener.new_leader_member_id.ok_or("new leader")?;
    assert_ne!(new_leader, initial_leader);
    assert_eq!(client.leader_member_id(), new_leader);
    let new_term = client.leadership_term_id();
    assert!(new_term > old_term, "term {new_term} after {old_term}");
    assert_eq!(listener.follower.book().min_epoch(), new_term);

    // --- Old-term frames through the new leader: a replay and a delayed
    // recovery snapshot with a newer sequence. Both are rejected. ---
    let delayed = snap(old_term, 2, vec![lvl(499, 9)], vec![lvl(502, 9)]);
    let before = listener.outcomes.len();
    client.offer(&pre_payload)?;
    client.offer(&encode_payload(&delayed)?)?;
    poll_until(
        &mut client,
        &mut listener,
        Duration::from_secs(15),
        "old-term echoes",
        |_, l| {
            not_serving(l, "old-term echoes")?;
            Ok(l.outcomes.len() >= before + 2)
        },
    )?;
    let stale = ApplyOutcome::Rejected(RejectReason::StaleEpoch {
        min_epoch: new_term,
        got: old_term,
    });
    assert_eq!(listener.outcomes[before..before + 2], [stale, stale]);

    // --- New-term reference snapshot restores serving ---
    let reference = snap(
        new_term,
        1,
        vec![lvl(510, 3), lvl(509, 4)],
        vec![lvl(511, 5)],
    );
    client.offer(&encode_payload(&reference)?)?;
    poll_until(
        &mut client,
        &mut listener,
        Duration::from_secs(15),
        "post-failover serving",
        |_, l| Ok(l.follower.book().is_serving()),
    )?;
    // Replaced, never merged with the old-term levels (500/501).
    assert_eq!(listener.follower.book().live_image(), Some(&reference));
    assert_eq!(listener.apply_errors, 0);

    let _ = client.close();
    Ok(())
}
