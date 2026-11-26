#![allow(missing_docs, dead_code)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
use ergo_aeron_cluster::egress::{ControlledAction, ControlledEgressListener, EgressListener};
use ergo_aeron_cluster::{AeronCluster, ClusterError, SessionBuilder, SessionState};
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

/// A `SessionEvent(OK)` whose detail spans several MTU-sized fragments.
fn answer_ok_fragmented(
    cluster: &FakeCluster,
    member: usize,
    correlation: i64,
) -> Result<rusteron_client::AeronExclusivePublication, Box<dyn Error>> {
    let egress = cluster.egress()?;
    let detail = "x".repeat(3 * egress.get_constants()?.max_payload_length());
    offer(
        &egress,
        &session_event(EventCode::OK, i32::try_from(member)?, &detail, correlation)?,
    )?;
    Ok(egress)
}

/// Java's `EgressPoller` reassembles fragments during connect, so an event
/// larger than one MTU (a long REDIRECT endpoint list or detail) still
/// completes the handshake.
#[test]
#[serial]
fn async_connect_reassembles_a_fragmented_event() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-assemble-async")?;
    let cluster = FakeCluster::launch()?;
    let mut connecting = fake_builder(CONNECT_TIMEOUT)?.connect_async(client_driver.dir.as_str());
    let (member, correlation) = await_request(&cluster, &[0], || {
        connecting.poll()?;
        Ok(())
    })?;
    let _egress = answer_ok_fragmented(&cluster, member, correlation)?;
    let client = finish(connecting)?;
    assert_eq!(client.cluster_session_id(), fake::CLUSTER_SESSION_ID);
    Ok(())
}

#[test]
#[serial]
fn blocking_connect_reassembles_a_fragmented_event() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-assemble-blocking")?;
    let cluster = FakeCluster::launch()?;
    let connecting = connect_on_thread(|| fake_builder(CONNECT_TIMEOUT), &client_driver.dir);
    let (member, correlation) = await_request(&cluster, &[0], || Ok(()))?;
    let _egress = answer_ok_fragmented(&cluster, member, correlation)?;
    let session = connecting.join().map_err(|_| "connect thread panicked")??;
    assert_eq!(session, fake::CLUSTER_SESSION_ID);
    Ok(())
}

#[test]
#[serial]
fn fan_out_connect_reassembles_a_fragmented_event() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-assemble-fanout")?;
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
    let _egress = answer_ok_fragmented(&cluster, member, correlation)?;
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

/// The publication-limit (type 1) and subscriber-position (type 4) counters
/// on `aeron`'s driver: one per publication and subscription it holds.
fn transport_counters(aeron: &rusteron_client::Aeron) -> usize {
    let mut count = 0;
    aeron
        .counters_reader()
        .foreach_counter_fn(|_, _, type_id, _, _| {
            if type_id == 1 || type_id == 4 {
                count += 1;
            }
        });
    count
}

/// Waits until `aeron`'s driver holds no publication or subscription.
fn await_transport_released(
    aeron: &rusteron_client::Aeron,
    what: &str,
) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while transport_counters(aeron) > 0 {
        if Instant::now() >= deadline {
            return Err(format!(
                "{what}: {} transport counters left",
                transport_counters(aeron)
            )
            .into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

/// Java's `AsyncConnect.close()` removes the egress subscription and every
/// member's ingress publication, whether still being added or added
/// (`AeronClusterAsyncConnectTest.shouldClose*`). Dropping an
/// `AsyncClusterConnect` on a shared Aeron client must release them too.
#[test]
#[serial]
fn dropping_an_async_connect_releases_its_transport() -> Result<(), Box<dyn Error>> {
    let client_driver = launch_own_driver("parity-async-drop")?;
    let cluster = FakeCluster::launch()?;
    let aeron = fake::connect(&client_driver.dir)?;
    let builder = || -> Result<SessionBuilder, ergo_aeron_cluster::ClusterError> {
        Ok(SessionBuilder::default()
            .ingress_channel("aeron:udp")?
            .ingress_endpoints(fake::ENDPOINTS)?
            .egress_channel(CLIENT_EGRESS)?
            .message_timeout(CONNECT_TIMEOUT)?
            .aeron(aeron.clone()))
    };

    // Dropped while the adds are in flight.
    let mut connecting = builder()?.connect_async(client_driver.dir.as_str());
    connecting.poll()?;
    drop(connecting);
    // Long enough for the driver to finish an add the drop failed to cancel.
    thread::sleep(Duration::from_secs(1));
    await_transport_released(&aeron, "dropped while adding")?;

    // Dropped once every member's publication and the egress are up.
    let mut connecting = builder()?.connect_async(client_driver.dir.as_str());
    let _ = await_request(&cluster, &[0, 1], || {
        connecting.poll()?;
        Ok(())
    })?;
    // The egress subscription and at least the member that heard the request.
    assert!(
        transport_counters(&aeron) >= 2,
        "{}",
        transport_counters(&aeron)
    );
    drop(connecting);
    await_transport_released(&aeron, "dropped while awaiting the answer")?;
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

/// Java's `trackIngressPublicationResult` closes the client once its ingress
/// publication reaches its maximum position: the offer reports it, and the
/// next poll leaves the session closed. The ingress starts 4 KiB short of the
/// maximum position (Aeron's `MaxPositionPublicationTest` channel settings).
#[test]
#[serial]
fn reaching_the_ingress_max_position_closes_the_session() -> Result<(), Box<dyn Error>> {
    const TERM_LENGTH: i64 = 64 * 1024;
    const INITIAL_TERM_ID: i32 = -777;
    let start = TERM_LENGTH * (i64::from(i32::MAX) + 1) - 4096;
    let term_id = INITIAL_TERM_ID.wrapping_add(i32::try_from(start / TERM_LENGTH)?);
    let term_offset = start % TERM_LENGTH;
    let ingress = format!(
        "{}|term-length=64k|init-term-id={INITIAL_TERM_ID}|term-id={term_id}|term-offset={term_offset}",
        MEMBER_INGRESS[0]
    );
    let client_driver = launch_own_driver("parity-max-position")?;
    let cluster = FakeCluster::launch()?;
    let builder = SessionBuilder::default()
        .ingress_channel(ingress)?
        .egress_channel(CLIENT_EGRESS)?
        .message_timeout(Duration::from_secs(10))?;
    let (mut client, _egress) = cluster.connect(builder, &client_driver.dir)?;

    let payload = [7u8; 1000];
    let deadline = Instant::now() + Duration::from_secs(10);
    let error = loop {
        match client.offer(&payload) {
            Err(error) if !error.is_retryable() => break error,
            _ if Instant::now() >= deadline => return Err("the ingress never filled".into()),
            _ => thread::sleep(Duration::from_millis(1)),
        }
    };
    assert!(
        matches!(
            error,
            ClusterError::Publication {
                failure: ergo_aeron_cluster::PublicationFailure::MaxPositionExceeded,
                ..
            }
        ),
        "{error:?}"
    );
    client.poll(&mut ergo_aeron_cluster::NullListener, 0)?;
    assert_eq!(client.state(), SessionState::Closed);
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
    // Timed from before connect: the cluster starts the session timer when it
    // opens the session, which is before connect() returns.
    let connected = Instant::now();
    let mut client = AeronCluster::connect(&builder, &driver.dir)?;
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

#[test]
#[serial]
fn controlled_poll_egress_actions_and_panic_on_java_cluster() -> Result<(), Box<dyn Error>> {
    struct ScriptedControlled {
        actions: std::collections::VecDeque<ControlledAction>,
        panic_on_next: bool,
        seen: Vec<(i64, i64, Vec<u8>)>,
    }

    impl ControlledEgressListener for ScriptedControlled {
        // The panic is the behaviour under test: the poll must catch it.
        #[allow(clippy::panic)]
        fn on_message(
            &mut self,
            cluster_session_id: i64,
            timestamp: i64,
            buffer: &[u8],
        ) -> ControlledAction {
            if self.panic_on_next {
                self.panic_on_next = false;
                panic!("controlled listener boom");
            }
            self.seen
                .push((cluster_session_id, timestamp, buffer.to_vec()));
            self.actions
                .pop_front()
                .unwrap_or(ControlledAction::Continue)
        }
    }

    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node()?;
    let driver = launch_own_driver("controlled-poll-java")?;
    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel("aeron:udp?endpoint=localhost:0")?
        .message_timeout(Duration::from_secs(10))?;
    let mut client = AeronCluster::connect(&builder, &driver.dir)?;
    let pos_initial = client
        .egress_position()
        .ok_or("missing bound egress image")?;

    // 1. Abort leaves the fragment unconsumed at the same position; Commit on the next
    //    poll redelivers the identical fragment and advances the position once.
    client.offer(b"abort-then-commit")?;
    let mut listener = ScriptedControlled {
        actions: std::collections::VecDeque::from([
            ControlledAction::Abort,
            ControlledAction::Commit,
        ]),
        panic_on_next: false,
        seen: Vec::new(),
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while listener.seen.is_empty() {
        let count = client.controlled_poll_egress(&mut listener, 10)?;
        assert_eq!(count, 0, "Abort must report 0 consumed fragments");
        if Instant::now() >= deadline {
            return Err("timed out waiting for first controlled delivery".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    let pos_after_abort = client
        .egress_position()
        .ok_or("missing bound egress image")?;
    assert_eq!(
        pos_after_abort, pos_initial,
        "Abort must not advance egress subscriber position"
    );
    assert_eq!(listener.seen.len(), 1);
    assert_eq!(listener.seen[0].2, b"abort-then-commit");

    let count = client.controlled_poll_egress(&mut listener, 10)?;
    assert_eq!(count, 1, "Commit must consume the redelivered fragment");
    assert_eq!(listener.seen.len(), 2);
    assert_eq!(
        listener.seen[0], listener.seen[1],
        "Abort must redeliver the identical fragment (session_id, timestamp, payload)"
    );
    let pos_after_commit = client
        .egress_position()
        .ok_or("missing bound egress image")?;
    assert!(
        pos_after_commit > pos_after_abort,
        "Commit must advance egress position ({pos_after_commit} <= {pos_after_abort})"
    );
    assert_eq!(
        client.controlled_poll_egress(&mut listener, 10)?,
        0,
        "Commit must advance once without redelivering"
    );

    // 2. Break stops after one fragment even when more are queued; Continue consumes the rest.
    client.offer(b"break-first")?;
    client.offer(b"continue-second")?;
    // Wait until the first fragment is in the egress buffer by peeking with Abort, then
    // give the second echo a brief window to land behind it in the same term buffer.
    listener.seen.clear();
    listener.actions = std::collections::VecDeque::from([ControlledAction::Abort]);
    let deadline = Instant::now() + Duration::from_secs(5);
    while listener.seen.is_empty() {
        client.controlled_poll_egress(&mut listener, 10)?;
        if Instant::now() >= deadline {
            return Err("timed out waiting for break-first".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    thread::sleep(Duration::from_millis(50));

    listener.seen.clear();
    listener.actions =
        std::collections::VecDeque::from([ControlledAction::Break, ControlledAction::Continue]);
    let broken = client.controlled_poll_egress(&mut listener, 10)?;
    assert_eq!(broken, 1, "Break must stop after consuming 1 fragment");
    assert_eq!(listener.seen.len(), 1);
    assert_eq!(listener.seen[0].2, b"break-first");

    let deadline = Instant::now() + Duration::from_secs(5);
    while listener.seen.len() < 2 {
        client.controlled_poll_egress(&mut listener, 10)?;
        if Instant::now() >= deadline {
            return Err("timed out waiting for continue-second".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(listener.seen[1].2, b"continue-second");

    // 3. A panicking listener returns ClusterError::ListenerPanicked and leaves the client usable.
    client.offer(b"trigger-panic")?;
    listener.seen.clear();
    listener.panic_on_next = true;
    let deadline = Instant::now() + Duration::from_secs(5);
    let panic_err = loop {
        match client.controlled_poll_egress(&mut listener, 10) {
            Err(err) => break err,
            Ok(0) => {
                if Instant::now() >= deadline {
                    return Err("timed out waiting for panicking fragment".into());
                }
                thread::sleep(Duration::from_millis(1));
            }
            Ok(n) => return Err(format!("expected panic error, got Ok({n})").into()),
        }
    };
    assert!(
        matches!(
            panic_err,
            ClusterError::ListenerPanicked {
                context: "controlled egress dispatch"
            }
        ),
        "expected ListenerPanicked, got {panic_err:?}"
    );
    assert_eq!(client.state(), SessionState::Connected);

    client.offer(b"usable-after-panic")?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while listener.seen.is_empty() {
        client.controlled_poll_egress(&mut listener, 10)?;
        if Instant::now() >= deadline {
            return Err("timed out waiting for usable-after-panic".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(listener.seen[0].2, b"usable-after-panic");

    client.close()?;
    Ok(())
}

#[test]
#[serial]
fn controlled_poll_egress_delivers_lifecycle_and_extension_events() -> Result<(), Box<dyn Error>> {
    use ergo_aeron_cluster::cluster_codec_types::{
        AdminRequestType, AdminResponseCode, AdminResponseEncoder, AdminResponseFixedFields,
    };

    #[derive(Default)]
    struct LifecycleControlled {
        events: Vec<(EventCode, String)>,
        leaders: Vec<(i64, i32, String)>,
        admins: Vec<(i64, AdminRequestType, AdminResponseCode, String, Vec<u8>)>,
        extensions: Vec<(u16, u16)>,
    }

    impl ControlledEgressListener for LifecycleControlled {
        fn on_message(&mut self, _: i64, _: i64, _: &[u8]) -> ControlledAction {
            ControlledAction::Continue
        }
        fn on_session_event(
            &mut self,
            _: i64,
            _: i64,
            _: i64,
            _: i32,
            code: EventCode,
            detail: &str,
        ) {
            self.events.push((code, detail.to_owned()));
        }
        fn on_new_leader(
            &mut self,
            _: i64,
            leadership_term_id: i64,
            leader_member_id: i32,
            ingress_endpoints: &str,
        ) {
            self.leaders.push((
                leadership_term_id,
                leader_member_id,
                ingress_endpoints.to_owned(),
            ));
        }
        fn on_admin_response(
            &mut self,
            _: i64,
            correlation_id: i64,
            request_type: AdminRequestType,
            response_code: AdminResponseCode,
            message: &str,
            payload: &[u8],
        ) {
            self.admins.push((
                correlation_id,
                request_type,
                response_code,
                message.to_owned(),
                payload.to_vec(),
            ));
        }
        fn on_extension_message(
            &mut self,
            _: u16,
            template_id: u16,
            schema_id: u16,
            _: u16,
            _: &[u8],
        ) -> ControlledAction {
            self.extensions.push((schema_id, template_id));
            ControlledAction::Continue
        }
    }

    let client_driver = launch_own_driver("controlled-lifecycle")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, egress) =
        cluster.connect(fake_builder(Duration::from_secs(10))?, &client_driver.dir)?;

    let mut admin_buf =
        vec![0u8; AdminResponseEncoder::compute_encoded_length_with_message_header(4, 2)];
    let _ = AdminResponseEncoder::wrap_and_apply_header(&mut admin_buf, 0)
        .fixed(&AdminResponseFixedFields {
            cluster_session_id: fake::CLUSTER_SESSION_ID,
            correlation_id: 77,
            request_type: AdminRequestType::SNAPSHOT,
            response_code: AdminResponseCode::OK,
        })
        .message(b"done")?
        .payload(&[0xaa, 0xbb])?;

    offer(&egress, &session_event(EventCode::OK, 0, "notice", 10)?)?;
    offer(&egress, &admin_buf)?;
    offer(
        &egress,
        &foreign_schema(session_event(EventCode::OK, 0, "", 11)?),
    )?;
    offer(&egress, &fake::new_leader_event(2, 1, fake::ENDPOINTS)?)?;

    let mut listener = LifecycleControlled::default();
    let deadline = Instant::now() + Duration::from_secs(10);
    while listener.leaders.is_empty()
        || listener.events.is_empty()
        || listener.admins.is_empty()
        || listener.extensions.is_empty()
    {
        client.controlled_poll_egress(&mut listener, 10)?;
        if Instant::now() >= deadline {
            return Err("timed out waiting for controlled lifecycle events".into());
        }
        thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(listener.events, vec![(EventCode::OK, "notice".to_owned())]);
    assert_eq!(
        listener.admins,
        vec![(
            77,
            AdminRequestType::SNAPSHOT,
            AdminResponseCode::OK,
            "done".to_owned(),
            vec![0xaa, 0xbb]
        )]
    );
    assert_eq!(listener.extensions, vec![(999, 2)]);
    assert_eq!(listener.leaders, vec![(2, 1, fake::ENDPOINTS.to_owned())]);

    offer(&egress, &session_event(EventCode::CLOSED, 1, "closed", 12)?)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while client.state() != SessionState::Closed {
        client.controlled_poll_egress(&mut listener, 10)?;
        if Instant::now() >= deadline {
            return Err("timed out waiting for SessionEvent(CLOSED)".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        listener.events.last(),
        Some(&(EventCode::CLOSED, "closed".to_owned()))
    );
    Ok(())
}
