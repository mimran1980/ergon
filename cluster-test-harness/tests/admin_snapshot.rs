#![allow(missing_docs)]
//! Admin request/response integration test for
//! `AeronCluster::send_admin_request_to_take_snapshot` against a live Java
//! cluster and verifies the reply arrives on `on_admin_response`.
//!
//! ```sh
//! cargo test -p ergo-aeron-cluster-test-harness \
//!   --test admin_snapshot -- --test-threads=1
//! ```

use ergo_aeron_cluster::cluster_codec_types::{AdminRequestType, AdminResponseCode, EventCode};
use ergo_aeron_cluster::egress::EgressListener;
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use rusteron_client::cformat;
use serial_test::serial;
use std::time::{Duration, Instant};

/// Records the first admin response (correlation id + response code), and
/// its request type and message.
#[derive(Default)]
struct Rec {
    admin_cid: Option<i64>,
    admin_code: Option<AdminResponseCode>,
    admin_request_type: Option<AdminRequestType>,
    admin_message: String,
}

impl EgressListener for Rec {
    fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, _: EventCode, _: &str) {}
    fn on_new_leader(&mut self, _: i64, _: i64, _: i32, _: &str) {}
    fn on_challenge(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_admin_response(
        &mut self,
        _csid: i64,
        correlation_id: i64,
        request_type: AdminRequestType,
        response_code: AdminResponseCode,
        msg: &str,
        _payload: &[u8],
    ) {
        if self.admin_cid.is_none() {
            self.admin_cid = Some(correlation_id);
            self.admin_code = Some(response_code);
            self.admin_request_type = Some(request_type);
            msg.clone_into(&mut self.admin_message);
        }
    }
}

#[test]
#[serial]
fn test_admin_snapshot_request_gets_response() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node()?;
    let dir = cformat!("{}", cluster.aeron_dir().display());

    let builder = SessionBuilder::default()
        .ingress_channel(&cluster.ingress_channel)?
        .egress_channel(&cluster.egress_channel)?;

    let mut listener = Rec::default();
    let mut client = AeronCluster::connect(&builder, &dir.to_string_lossy())?;

    // Request a snapshot; the Java cluster replies on the egress listener.
    let correlation_id = client.leadership_term_id() + 1000;
    client.send_admin_request_to_take_snapshot(correlation_id)?;

    // Poll egress until the admin response arrives (or timeout).
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let _ = client.send_keep_alive();
        client.poll(&mut listener, 10)?;
        if listener.admin_cid.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    let got_cid = listener.admin_cid;
    let got_code = listener.admin_code;
    // The round-trip is the parity point: the cluster MUST reply on
    // on_admin_response with the matching correlation id. The response CODE
    // depends on cluster auth: the default single-node test cluster denies
    // snapshot admin requests as UNAUTHORISEDACCESS (no elevated credentials),
    // which still proves the request was received, processed, and answered.
    assert_eq!(
        got_cid,
        Some(correlation_id),
        "admin response correlation id must match"
    );
    assert!(
        got_code.is_some(),
        "admin response must carry a response code, got None"
    );

    client.close()?;
    Ok(())
}

/// A client of `cluster` on its own media driver.
fn connect(
    cluster: &ergo_aeron_cluster_test_harness::TestCluster,
) -> Result<AeronCluster, Box<dyn std::error::Error>> {
    let builder = SessionBuilder::default()
        .ingress_channel(&cluster.ingress_channel)?
        .egress_channel(&cluster.egress_channel)?;
    Ok(AeronCluster::connect(
        &builder,
        &cluster.aeron_dir().to_string_lossy(),
    )?)
}

/// Polls `client` until the first admin response arrives.
fn await_admin_response(client: &mut AeronCluster) -> Result<Rec, Box<dyn std::error::Error>> {
    let mut listener = Rec::default();
    let deadline = Instant::now() + Duration::from_secs(15);
    while listener.admin_cid.is_none() {
        if Instant::now() >= deadline {
            return Err("no admin response".into());
        }
        client.keep_alive_if_due()?;
        client.poll(&mut listener, 10)?;
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(listener)
}

/// Java's default authorisation service refuses a snapshot request
/// (`ClusterTest.shouldRejectTakeSnapshotRequestWithAnAuthorisationError`).
#[test]
#[serial]
fn an_unauthorised_snapshot_request_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node()?;
    let mut client = connect(&cluster)?;
    client.send_admin_request_to_take_snapshot(41)?;
    let response = await_admin_response(&mut client)?;
    assert_eq!(response.admin_cid, Some(41));
    assert_eq!(
        response.admin_request_type,
        Some(AdminRequestType::SNAPSHOT)
    );
    assert_eq!(
        response.admin_code,
        Some(AdminResponseCode::UNAUTHORISEDACCESS)
    );
    assert_eq!(
        response.admin_message,
        "Execution of the SNAPSHOT request was not authorised"
    );
    client.close()?;
    Ok(())
}

/// An authorised snapshot request is answered OK
/// (`ClusterTest.shouldTakeASnapshotAfterReceivingAdminRequestOfTypeSnapshot`).
#[test]
#[serial]
fn an_authorised_snapshot_request_is_answered_ok() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node_admin_allowed()?;
    let mut client = connect(&cluster)?;
    client.send_admin_request_to_take_snapshot(42)?;
    let response = await_admin_response(&mut client)?;
    assert_eq!(response.admin_cid, Some(42));
    assert_eq!(response.admin_code, Some(AdminResponseCode::OK));
    assert_eq!(response.admin_message, "");
    client.close()?;
    Ok(())
}

/// An admin request of an unknown type gets an ERROR response, which the
/// client delivers with the type it does not know
/// (`ClusterTest.shouldRejectAnInvalidAdminRequest`). The client sends only
/// snapshot requests, so the test offers this one on its own publication.
#[test]
#[serial]
fn an_invalid_admin_request_is_answered_with_an_error() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_aeron_cluster::cluster_codec_types::{AdminRequestEncoder, AdminRequestFixedFields};
    use rusteron_client::{Aeron, AeronContext, Handlers};

    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node_admin_allowed()?;
    let mut client = connect(&cluster)?;
    let ctx = AeronContext::new()?;
    ctx.set_dir(&cformat!("{}", cluster.aeron_dir().display()))?;
    let aeron = Aeron::new(&ctx)?;
    aeron.start()?;
    let ingress = aeron.add_exclusive_publication(
        &cformat!("{}", cluster.ingress_channel),
        101,
        Duration::from_secs(5),
    )?;
    let mut request = vec![0u8; AdminRequestEncoder::compute_encoded_length_with_message_header(0)];
    let _ = AdminRequestEncoder::wrap_and_apply_header(&mut request, 0)
        .fixed(&AdminRequestFixedFields {
            leadership_term_id: client.leadership_term_id(),
            cluster_session_id: client.cluster_session_id(),
            correlation_id: 43,
            request_type: AdminRequestType::NullVal,
        })
        .payload(&[])?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while ingress.offer_raw(&request, Handlers::NONE) < 0 {
        if Instant::now() >= deadline {
            return Err("the request was never offered".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let response = await_admin_response(&mut client)?;
    assert_eq!(response.admin_cid, Some(43));
    assert_eq!(response.admin_request_type, Some(AdminRequestType::NullVal));
    assert_eq!(response.admin_code, Some(AdminResponseCode::ERROR));
    assert_eq!(response.admin_message, "Unknown request type: NULL_VAL");
    client.close()?;
    Ok(())
}
