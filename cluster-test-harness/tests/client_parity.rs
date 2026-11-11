#![allow(missing_docs, dead_code)]
//! Behaviour of Java's `AeronCluster` client, checked against a Java cluster
//! or, where each protocol step must happen on cue, a fake one: connect
//! shapes (an ephemeral egress port, `aeron:udp` plus an ingress endpoint
//! map) and the session protocol.

mod common;
mod fake;

use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};

use ergo_aeron_cluster::cluster_codec_types::EventCode;
use ergo_aeron_cluster::egress::EgressListener;
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use serial_test::serial;

use common::{Capture, await_echo, launch_own_driver};
use fake::{CLIENT_EGRESS, FakeCluster, MEMBER_INGRESS, finish, offer, session_event};

/// A session builder for the fake cluster's member 0.
fn fake_builder(
    message_timeout: Duration,
) -> Result<SessionBuilder, ergo_aeron_cluster::ClusterError> {
    SessionBuilder::default()
        .ingress_channel(MEMBER_INGRESS[0])?
        .egress_channel(CLIENT_EGRESS)?
        .message_timeout(message_timeout)
}

/// `frame` relabelled as schema 999, keeping its template id.
fn foreign_schema(mut frame: Vec<u8>) -> Vec<u8> {
    frame[4..6].copy_from_slice(&999u16.to_le_bytes());
    frame
}

/// Records the schema and template id of every extension frame.
#[derive(Default)]
struct Extensions(Vec<(u16, u16)>);

impl EgressListener for Extensions {
    fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_extension_message(&mut self, _: u16, template_id: u16, schema_id: u16, _: u16, _: &[u8]) {
        self.0.push((schema_id, template_id));
    }
}

/// Java's `EgressPoller` ignores a frame of another schema during connect,
/// and its `EgressAdapter` hands one to the extension listener afterwards.
#[test]
#[serial]
fn another_schema_on_egress_is_an_extension_frame() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-schema-client")?;
    let cluster = FakeCluster::launch()?;
    let mut connecting =
        fake_builder(Duration::from_secs(10))?.connect_async(client_driver.dir.as_str());
    let correlation = fake::await_connect_request(&cluster, &mut connecting, 0)?;
    // The member answers with another schema's frame first: connect skips it.
    let egress = cluster.egress()?;
    offer(
        &egress,
        &foreign_schema(session_event(EventCode::OK, 0, "", correlation)?),
    )?;
    offer(&egress, &session_event(EventCode::OK, 0, "", correlation)?)?;
    let mut client = finish(connecting)?;
    assert_eq!(client.cluster_session_id(), fake::CLUSTER_SESSION_ID);

    offer(
        &egress,
        &foreign_schema(session_event(EventCode::OK, 0, "", 2)?),
    )?;
    let mut listener = Extensions::default();
    let deadline = Instant::now() + Duration::from_secs(10);
    while listener.0.is_empty() {
        client.poll(&mut listener, 10)?;
        if Instant::now() >= deadline {
            return Err("the extension frame never arrived".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(listener.0, [(999, 2)]);
    Ok(())
}

/// Longer than the earlier client's re-offer interval at this message timeout
/// (a quarter of it), several times over.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
const HOLD_ANSWER: Duration = Duration::from_millis(2500);

/// Run `poll` until one of `members` hears a `SessionConnectRequest`.
fn await_request(
    cluster: &FakeCluster,
    members: &[usize],
    mut poll: impl FnMut() -> Result<(), Box<dyn Error>>,
) -> Result<(usize, i64), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        for &member in members {
            if let Some(&correlation) = cluster.connect_requests(member)?.first() {
                return Ok((member, correlation));
            }
        }
        poll()?;
        if Instant::now() >= deadline {
            return Err("no member heard a connect request".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
}

/// The connect requests `members` hear while `poll` runs for [`HOLD_ANSWER`].
fn requests_while_unanswered(
    cluster: &FakeCluster,
    members: &[usize],
    mut poll: impl FnMut() -> Result<(), Box<dyn Error>>,
) -> Result<Vec<(usize, i64)>, Box<dyn Error>> {
    let mut heard = Vec::new();
    let started = Instant::now();
    while started.elapsed() < HOLD_ANSWER {
        for &member in members {
            heard.extend(
                cluster
                    .connect_requests(member)?
                    .into_iter()
                    .map(|c| (member, c)),
            );
        }
        poll()?;
        thread::sleep(Duration::from_millis(1));
    }
    Ok(heard)
}

/// Answer the connect request `correlation` from `member` with OK.
fn answer_ok(
    cluster: &FakeCluster,
    member: usize,
    correlation: i64,
) -> Result<rusteron_client::AeronExclusivePublication, Box<dyn Error>> {
    let egress = cluster.egress()?;
    let leader = i32::try_from(member)?;
    offer(
        &egress,
        &session_event(EventCode::OK, leader, "", correlation)?,
    )?;
    Ok(egress)
}

/// Each `SessionConnectRequest` opens a new session on the cluster
/// (`SessionManager.onSessionConnect`), so Java's client sends it once and
/// waits; it re-sends only after a challenge or a redirect. A slow answer
/// must not draw a second request.
#[test]
#[serial]
fn async_connect_sends_one_connect_request() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-once-async")?;
    let cluster = FakeCluster::launch()?;
    let mut connecting = fake_builder(CONNECT_TIMEOUT)?.connect_async(client_driver.dir.as_str());
    let (member, correlation) = await_request(&cluster, &[0], || {
        connecting.poll()?;
        Ok(())
    })?;
    let again = requests_while_unanswered(&cluster, &[0], || {
        connecting.poll()?;
        Ok(())
    })?;
    assert_eq!(again, [], "connect re-sent its request");
    let _egress = answer_ok(&cluster, member, correlation)?;
    let client = finish(connecting)?;
    assert_eq!(client.cluster_session_id(), fake::CLUSTER_SESSION_ID);
    Ok(())
}

/// The blocking connect, on a thread, with the builder `make` returns (a
/// builder is not `Send`).
fn connect_on_thread(
    make: impl FnOnce() -> Result<SessionBuilder, ergo_aeron_cluster::ClusterError> + Send + 'static,
    dir: &str,
) -> thread::JoinHandle<Result<i64, String>> {
    let dir = dir.to_owned();
    thread::spawn(move || {
        make()
            .and_then(|builder| AeronCluster::connect(&builder, &dir))
            .map(|client| client.cluster_session_id())
            .map_err(|error| error.to_string())
    })
}

#[test]
#[serial]
fn blocking_connect_sends_one_connect_request() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-once-blocking")?;
    let cluster = FakeCluster::launch()?;
    let connecting = connect_on_thread(|| fake_builder(CONNECT_TIMEOUT), &client_driver.dir);
    let (member, correlation) = await_request(&cluster, &[0], || Ok(()))?;
    let again = requests_while_unanswered(&cluster, &[0], || Ok(()))?;
    assert_eq!(again, [], "connect re-sent its request");
    let _egress = answer_ok(&cluster, member, correlation)?;
    let session = connecting.join().map_err(|_| "connect thread panicked")??;
    assert_eq!(session, fake::CLUSTER_SESSION_ID);
    Ok(())
}

#[test]
#[serial]
fn fan_out_connect_sends_one_connect_request() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-once-fanout")?;
    let cluster = FakeCluster::launch()?;
    let builder = || {
        SessionBuilder::default()
            .ingress_channel("aeron:udp")?
            .ingress_endpoints(fake::ENDPOINTS)?
            .egress_channel(CLIENT_EGRESS)?
            .message_timeout(CONNECT_TIMEOUT)
    };
    let connecting = connect_on_thread(builder, &client_driver.dir);
    let (member, correlation) = await_request(&cluster, &[0, 1], || Ok(()))?;
    let again = requests_while_unanswered(&cluster, &[0, 1], || Ok(()))?;
    assert_eq!(again, [], "connect re-sent its request");
    let _egress = answer_ok(&cluster, member, correlation)?;
    let session = connecting.join().map_err(|_| "connect thread panicked")??;
    assert_eq!(session, fake::CLUSTER_SESSION_ID);
    Ok(())
}

/// Java's `AsyncConnect` throws `ClusterException(detail)` on a
/// `SessionEvent(ERROR)` for its request, rather than wait out the message
/// timeout.
#[test]
#[serial]
fn a_connect_error_fails_the_connect_at_once() -> Result<(), Box<dyn Error>> {
    let message_timeout = Duration::from_secs(10);
    let client_driver = launch_own_driver("parity-error")?;
    let cluster = FakeCluster::launch()?;
    let mut connecting = fake_builder(message_timeout)?.connect_async(client_driver.dir.as_str());
    let _egress = cluster.answer(&mut connecting, 0, |correlation| {
        session_event(EventCode::ERROR, 0, "concurrent session limit", correlation)
    })?;
    let answered = Instant::now();
    let error = loop {
        match connecting.poll() {
            Err(error) => break error,
            Ok(_) if answered.elapsed() > message_timeout => {
                return Err("connect outlived its message timeout".into());
            }
            Ok(_) => thread::sleep(Duration::from_millis(1)),
        }
    };
    assert!(
        answered.elapsed() < Duration::from_secs(2),
        "the ERROR took {:?} to fail the connect",
        answered.elapsed()
    );
    let ergo_aeron_cluster::ClusterError::ConnectFailed { reason } = error else {
        return Err(format!("expected ConnectFailed, got {error:?}").into());
    };
    assert!(reason.contains("concurrent session limit"), "{reason}");
    Ok(())
}

/// A Java cluster at its session limit (`aeron.cluster.max.sessions`)
/// answers a second client with `SessionEvent(ERROR)`.
#[test]
#[serial]
fn the_session_limit_rejects_a_second_client_at_once() -> Result<(), Box<dyn Error>> {
    let mut options = std::ffi::OsString::from("@");
    options.push(ergo_aeron_cluster_test_harness::jvm::test_options());
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node_with_jvm_args(&[
        options,
        "-Daeron.cluster.max.sessions=1".into(),
    ])?;
    let driver = launch_own_driver("parity-session-limit")?;
    let builder = |egress: &str| {
        SessionBuilder::default()
            .ingress_channel(cluster.ingress_channel.clone())?
            .egress_channel(egress)?
            .message_timeout(Duration::from_secs(10))
    };
    let mut first =
        AeronCluster::connect(&builder("aeron:udp?endpoint=localhost:0")?, &driver.dir)?;

    let started = Instant::now();
    let second = AeronCluster::connect(&builder("aeron:udp?endpoint=localhost:0")?, &driver.dir);
    let Err(ergo_aeron_cluster::ClusterError::ConnectFailed { reason }) = second else {
        return Err(format!(
            "expected the second client to be rejected, got {:?}",
            second.map(|client| client.cluster_session_id())
        )
        .into());
    };
    assert!(reason.contains("concurrent session limit"), "{reason}");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "rejected after {:?}",
        started.elapsed()
    );

    let mut listener = Capture::new();
    await_echo(&mut first, &mut listener, b"first", Duration::from_secs(5))?;
    first.close()?;
    Ok(())
}

/// Java's `AeronCluster.close()` sends one `SessionCloseRequest` with the
/// session's term and id, and is closed when it returns.
#[test]
#[serial]
fn close_sends_one_session_close_request() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-close")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, _egress) =
        cluster.connect(fake_builder(Duration::from_secs(10))?, &client_driver.dir)?;
    client.close()?;
    assert_eq!(client.state(), ergo_aeron_cluster::SessionState::Closed);
    assert_eq!(
        cluster.close_requests(0, Duration::from_secs(1))?,
        [(1, fake::CLUSTER_SESSION_ID)]
    );
    assert!(matches!(
        client.close(),
        Err(ergo_aeron_cluster::ClusterError::SessionClosed)
    ));
    drop(client);
    assert_eq!(cluster.close_requests(0, Duration::from_millis(500))?, []);
    Ok(())
}

/// Java sends the close request only while connected: a session that lost
/// its leader has no one to tell.
#[test]
#[serial]
fn close_after_losing_the_leader_sends_no_close_request() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-close-lost")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, egress) =
        cluster.connect(fake_builder(Duration::from_secs(10))?, &client_driver.dir)?;
    // The leader's egress image closes; its ingress subscription stays up.
    drop(egress);
    let deadline = Instant::now() + Duration::from_secs(20);
    while client.state() != ergo_aeron_cluster::SessionState::AwaitingNewLeader {
        client.poll(&mut ergo_aeron_cluster::NullListener, 10)?;
        if Instant::now() >= deadline {
            return Err("the session never noticed its leader's egress closing".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(client.is_ingress_connected());
    client.close()?;
    assert_eq!(client.state(), ergo_aeron_cluster::SessionState::Closed);
    assert_eq!(cluster.close_requests(0, Duration::from_secs(1))?, []);
    Ok(())
}

/// One Java node whose sessions time out after `aeron.cluster.session.timeout`.
fn node_with_session_timeout(
    timeout: &str,
) -> Result<ergo_aeron_cluster_test_harness::TestCluster, ergo_aeron_cluster::ClusterError> {
    let mut options = std::ffi::OsString::from("@");
    options.push(ergo_aeron_cluster_test_harness::jvm::test_options());
    ergo_aeron_cluster_test_harness::TestCluster::single_node_with_jvm_args(&[
        options,
        format!("-Daeron.cluster.session.timeout={timeout}").into(),
    ])
}

/// Records the session events this session receives.
#[derive(Default)]
struct SessionEvents(Vec<EventCode>);

impl EgressListener for SessionEvents {
    fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, code: EventCode, _: &str) {
        self.0.push(code);
    }
}

/// Java `ClusterTest.shouldCloseClientOnTimeout`: without keep-alives the
/// cluster times the session out and tells the client, which closes.
#[test]
#[serial]
fn the_cluster_closes_a_session_that_sends_no_keep_alives() -> Result<(), Box<dyn Error>> {
    let cluster = node_with_session_timeout("2s")?;
    let driver = launch_own_driver("parity-session-timeout")?;
    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel("aeron:udp?endpoint=localhost:0")?
        .message_timeout(Duration::from_secs(10))?;
    let mut client = AeronCluster::connect(&builder, &driver.dir)?;
    let connected = Instant::now();
    let mut events = SessionEvents::default();
    while client.state() != ergo_aeron_cluster::SessionState::Closed {
        client.poll(&mut events, 10)?;
        if connected.elapsed() > Duration::from_secs(15) {
            return Err(format!("still {:?} after 15s", client.state()).into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(
        connected.elapsed() >= Duration::from_secs(2),
        "closed after {:?}, before the session timeout",
        connected.elapsed()
    );
    assert_eq!(events.0, [EventCode::CLOSED]);
    Ok(())
}

/// Keep-alives at [`AeronCluster::keep_alive_if_due`]'s interval hold the
/// session open well past the cluster's session timeout.
#[test]
#[serial]
fn keep_alives_hold_a_session_open_past_the_session_timeout() -> Result<(), Box<dyn Error>> {
    let cluster = node_with_session_timeout("2s")?;
    let driver = launch_own_driver("parity-keep-alive")?;
    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel("aeron:udp?endpoint=localhost:0")?
        .message_timeout(Duration::from_secs(2))?;
    let mut client = AeronCluster::connect(&builder, &driver.dir)?;
    let mut events = SessionEvents::default();
    let connected = Instant::now();
    while connected.elapsed() < Duration::from_secs(6) {
        client.keep_alive_if_due()?;
        client.poll(&mut events, 10)?;
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(client.state(), ergo_aeron_cluster::SessionState::Connected);
    assert_eq!(events.0, []);
    let mut listener = Capture::new();
    await_echo(&mut client, &mut listener, b"alive", Duration::from_secs(5))?;
    client.close()?;
    Ok(())
}

#[test]
#[serial]
fn ephemeral_egress_port_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node()?;
    let driver = launch_own_driver("ephemeral-egress")?;
    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel("aeron:udp?endpoint=localhost:0")?
        .message_timeout(Duration::from_secs(10))?;
    let mut client = AeronCluster::connect(&builder, &driver.dir)?;
    let mut listener = Capture::new();
    await_echo(
        &mut client,
        &mut listener,
        b"port-0",
        Duration::from_secs(5),
    )?;
    client.close()?;
    Ok(())
}

#[test]
#[serial]
fn three_node_ingress_fanout_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::three_node()?;
    let driver = launch_own_driver("fanout-egress")?;
    let base = cluster.base_port();
    let endpoints = format!(
        "0=localhost:{},1=localhost:{},2=localhost:{}",
        base + 2,
        base + 102,
        base + 202
    );
    let builder = SessionBuilder::default()
        .ingress_channel("aeron:udp")?
        .ingress_endpoints(endpoints)?
        .egress_channel("aeron:udp?endpoint=localhost:0")?
        .message_timeout(Duration::from_secs(20))?;
    let mut client = AeronCluster::connect(&builder, &driver.dir)?;
    let mut listener = Capture::new();
    await_echo(
        &mut client,
        &mut listener,
        b"fanout",
        Duration::from_secs(10),
    )?;
    client.close()?;
    Ok(())
}
