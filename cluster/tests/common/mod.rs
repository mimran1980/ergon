//! Shared helpers for the own-driver UDP cluster integration tests.
//!
//! Each lifecycle test (failover, quorum loss, restart) connects a
//! high-level `AeronCluster` client to a Java cluster over the client's
//! OWN embedded media driver + UDP transport, then drives it through
//! `poll_egress`. These helpers factor that shared scaffolding out.

use std::error::Error;
use std::ffi::CString;
use std::time::{Duration, Instant};

use ergo_aeron_cluster::codecs::cluster_codecs::{
    admin_request_type::AdminRequestType, admin_response_code::AdminResponseCode, event_code::EventCode,
};
use ergo_aeron_cluster::egress::{EgressAdapter, EgressListener};
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};

/// The client's own embedded C media driver, held alive for the test.
/// Dropping this stops the driver.
pub struct OwnDriver {
    pub dir: String,
    _guard: rusteron_media_driver::EmbeddedMediaDriver,
}

/// Launch the client's own driver in a fresh temp dir tagged `tag`.
pub fn launch_own_driver(tag: &str) -> OwnDriver {
    let dir = std::env::temp_dir().join(format!("{tag}-{pid}", pid = std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let dir_cstr = CString::new(dir.to_str().unwrap()).unwrap();
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

/// Connect a high-level client to the cluster over own-driver UDP. The
/// egress port is the client's own UDP endpoint (the cluster sends
/// SessionEvent / echoes / NewLeaderEvent here).
pub fn connect_own_driver(
    cluster_ingress: &str,
    egress_port: u16,
    aeron_dir: &str,
) -> Result<AeronCluster, ergo_aeron_cluster::ClusterError> {
    let builder = SessionBuilder::builder()
        .ingress_channel(cluster_ingress.to_string())
        .egress_channel(format!("aeron:udp?endpoint=localhost:{egress_port}"))
        .ingress_stream_id(101)
        .egress_stream_id(102)
        .message_timeout(Duration::from_secs(10));
    AeronCluster::connect(&builder, aeron_dir)
}

/// Records echoed application payloads and flags the first `NewLeaderEvent`.
pub struct Capture {
    pub messages: Vec<Vec<u8>>,
    pub new_leader_member_id: Option<i32>,
}

impl Capture {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            new_leader_member_id: None,
        }
    }
}

impl EgressListener for Capture {
    fn on_message(&mut self, _csid: i64, _ts: i64, buffer: &[u8]) {
        self.messages.push(buffer.to_vec());
    }
    fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, _: EventCode, _: &str) {}
    fn on_new_leader(&mut self, _: i64, _: i64, leader_member_id: i32, _: &str) {
        if self.new_leader_member_id.is_none() {
            self.new_leader_member_id = Some(leader_member_id);
        }
    }
    fn on_challenge(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_admin_response(&mut self, _: i64, _: i64, _: AdminRequestType, _: AdminResponseCode, _: &str, _: &[u8]) {}
}

/// Offer `payload` and poll egress (with best-effort keep-alive) until the
/// cluster echoes it back. Errors on timeout — a real round-trip failure.
pub fn await_echo(
    client: &mut AeronCluster,
    adapter: &mut EgressAdapter<Capture>,
    payload: &[u8],
    timeout: Duration,
) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + timeout;
    client.offer(payload)?;
    loop {
        let _ = client.send_keep_alive();
        let _ = client.poll_egress(adapter, 10);
        if adapter.listener().messages.iter().any(|m| m.as_slice() == payload) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "no echo of {:?} within {:?} (got {} messages)",
                std::str::from_utf8(payload).unwrap_or("<bytes>"),
                timeout,
                adapter.listener().messages.len()
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
