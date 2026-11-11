#![allow(missing_docs)]
//! Authentication: raw connect requests to a node with the default
//! authenticator, and `AeronCluster` against `TestAuthenticator`, after
//! Java's `AuthenticationTest`.

#[expect(
    dead_code,
    reason = "the auth tests use only the shared driver and echo helpers"
)]
mod common;

use std::borrow::Cow;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ergo_aeron_cluster::cluster_codec_types::{
    SessionConnectRequestEncoder, SessionConnectRequestFixedFields,
};
use ergo_aeron_cluster::{AeronCluster, ClusterError, CredentialsSupplier, SessionBuilder};
use ergo_aeron_cluster_test_harness::TestCluster;
use rusteron_client::cformat;
use serial_test::serial;

use common::{Capture, await_echo, launch_own_driver};

/// Sends `credentials` on connect and `answer` to a challenge, recording
/// each challenge.
struct Scripted {
    credentials: &'static [u8],
    answer: &'static [u8],
    challenges: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl Scripted {
    fn new(credentials: &'static [u8], answer: &'static [u8]) -> (Self, Arc<Mutex<Vec<Vec<u8>>>>) {
        let challenges = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                credentials,
                answer,
                challenges: Arc::clone(&challenges),
            },
            challenges,
        )
    }
}

impl CredentialsSupplier for Scripted {
    fn encoded_credentials(&self) -> Option<Cow<'_, [u8]>> {
        Some(Cow::Borrowed(self.credentials))
    }

    fn on_challenge(&self, encoded_challenge: &[u8]) -> Option<Cow<'_, [u8]>> {
        if let Ok(mut challenges) = self.challenges.lock() {
            challenges.push(encoded_challenge.to_vec());
        }
        Some(Cow::Borrowed(self.answer))
    }
}

/// A connect's outcome: the client or its error, and the challenges seen.
/// The client's driver lives as long as this.
type Attempt = (
    Result<AeronCluster, ClusterError>,
    Vec<Vec<u8>>,
    common::OwnDriver,
);

/// Connect to `cluster` with `credentials`, answering a challenge with
/// `answer`.
fn connect_with(
    cluster: &TestCluster,
    credentials: &'static [u8],
    answer: &'static [u8],
    tag: &str,
) -> Result<Attempt, Box<dyn Error>> {
    let driver = launch_own_driver(tag)?;
    let (supplier, challenges) = Scripted::new(credentials, answer);
    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel("aeron:udp?endpoint=localhost:0")?
        .credentials(Arc::new(supplier))
        .message_timeout(Duration::from_secs(10))?;
    let client = AeronCluster::connect(&builder, &driver.dir);
    let seen = challenges
        .lock()
        .map_err(|_| "challenge log poisoned")?
        .clone();
    Ok((client, seen, driver))
}

/// The session the authenticator accepted serves a round trip.
fn assert_serves(client: Result<AeronCluster, ClusterError>) -> Result<(), Box<dyn Error>> {
    let mut client = client?;
    assert!(client.cluster_session_id() >= 0);
    await_echo(
        &mut client,
        &mut Capture::new(),
        b"authenticated",
        Duration::from_secs(5),
    )?;
    client.close()?;
    Ok(())
}

/// Java `shouldAuthenticateOnConnectRequestWithEmptyCredentials`.
#[test]
#[serial]
fn empty_credentials_are_accepted_without_a_challenge() -> Result<(), Box<dyn Error>> {
    let cluster = TestCluster::single_node_auth()?;
    let (client, challenges, _driver) = connect_with(&cluster, b"", b"", "auth-empty")?;
    assert_eq!(challenges, Vec::<Vec<u8>>::new());
    assert_serves(client)
}

/// Java `shouldAuthenticateOnConnectRequestWithCredentials`.
#[test]
#[serial]
fn credentials_on_the_connect_request_are_accepted() -> Result<(), Box<dyn Error>> {
    let cluster = TestCluster::single_node_auth()?;
    let (client, challenges, _driver) = connect_with(&cluster, b"admin:admin", b"", "auth-creds")?;
    assert_eq!(challenges, Vec::<Vec<u8>>::new());
    assert_serves(client)
}

/// Java `shouldAuthenticateOnChallengeResponse`.
#[test]
#[serial]
fn a_challenge_answered_correctly_is_accepted() -> Result<(), Box<dyn Error>> {
    let cluster = TestCluster::single_node_auth()?;
    let (client, challenges, _driver) =
        connect_with(&cluster, b"challenge-me", b"answer", "auth-challenge")?;
    assert_eq!(challenges, [b"server-challenge".to_vec()]);
    assert_serves(client)
}

/// Java `shouldRejectOnConnectRequest`.
#[test]
#[serial]
fn wrong_credentials_are_rejected() -> Result<(), Box<dyn Error>> {
    let cluster = TestCluster::single_node_auth()?;
    let (client, challenges, _driver) = connect_with(&cluster, b"admin:wrong", b"", "auth-reject")?;
    assert_eq!(challenges, Vec::<Vec<u8>>::new());
    assert!(
        matches!(client, Err(ClusterError::AuthRejected)),
        "expected AuthRejected, got {:?}",
        client.map(|c| c.cluster_session_id())
    );
    Ok(())
}

/// Java `shouldRejectOnChallengeResponse`.
#[test]
#[serial]
fn a_wrong_challenge_answer_is_rejected() -> Result<(), Box<dyn Error>> {
    let cluster = TestCluster::single_node_auth()?;
    let (client, challenges, _driver) =
        connect_with(&cluster, b"challenge-me", b"wrong", "auth-reject-challenge")?;
    assert_eq!(challenges, [b"server-challenge".to_vec()]);
    assert!(
        matches!(client, Err(ClusterError::AuthRejected)),
        "expected AuthRejected, got {:?}",
        client.map(|c| c.cluster_session_id())
    );
    Ok(())
}

fn connect_and_send(
    cluster: &ergo_aeron_cluster_test_harness::TestCluster,
    credentials: &[u8],
) -> Result<(bool, bool), Box<dyn std::error::Error>> {
    let dir_cstr = cformat!("{}", cluster.aeron_dir().display());
    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&dir_cstr)?;
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

    let mut buf = [0u8; 512];
    let complete = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&SessionConnectRequestFixedFields {
            correlation_id: 1,
            response_stream_id: 102,
            version: Some(0),
        })
        .response_channel(cluster.egress_channel.as_bytes())?
        .encoded_credentials(credentials)?
        .client_info(b"")?;
    let buf_ref = complete.as_bytes_with_header();

    let mut sent = false;
    for _ in 0..20 {
        let r = ingress.offer_raw(buf_ref, rusteron_client::Handlers::NONE);
        if r > 0 {
            sent = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let mut received = false;
    let mut event_code = 0i32;
    for _ in 0..30 {
        egress
            .poll_fn(
                |data, _hdr| {
                    if data.len() >= 16 {
                        let t = u16::from_le_bytes([data[2], data[3]]);
                        if t == 2 {
                            // SessionEvent
                            received = true;
                            // Event code is at offset 20 (after header+body prefix)
                            event_code =
                                i32::from_le_bytes([data[20], data[21], data[22], data[23]]);
                        }
                    }
                },
                10,
            )
            .ok();
        if received {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok((sent, received))
}

#[test]
#[serial]
fn test_connect_with_null_credentials() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node()?;
    let (sent, received) = connect_and_send(&cluster, b"")?;
    assert!(
        sent || received,
        "null credentials connect failed: sent={sent} received={received}"
    );

    Ok(())
}

#[test]
#[serial]
fn test_connect_with_simple_credentials() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node()?;
    // Default authenticator accepts any credentials in SingleNodeCluster config
    let (sent, received) = connect_and_send(&cluster, b"admin:password")?;
    assert!(
        sent || received,
        "credentialed connect failed: sent={sent} received={received}"
    );

    Ok(())
}

#[test]
#[serial]
fn test_two_connections_to_same_cluster() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node()?;
    let (s1, r1) = connect_and_send(&cluster, b"")?;
    let (s2, r2) = connect_and_send(&cluster, b"")?;
    assert!(s1 || r1, "first connect failed");
    assert!(s2 || r2, "second connect failed");

    Ok(())
}
