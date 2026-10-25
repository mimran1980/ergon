#![allow(missing_docs)]
//! No poll waits on the media driver, a session follows its own egress image,
//! and a NewLeaderEvent is applied as Java's client applies it.
//!
//! The client runs on a Java media driver in its own process, which the tests
//! freeze with `SIGSTOP`: an add that waited for the driver would hang until
//! it resumes, while a polled add keeps every poll short. A fake cluster on an
//! in-process driver plays the members, so each step happens on cue.

#[expect(
    dead_code,
    reason = "these tests drive a fake cluster, not the shared echo helpers"
)]
mod common;

use std::error::Error;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use ergo_aeron_cluster::cluster_codec_types::{
    AnyMessage, EventCode, NewLeaderEventEncoder, NewLeaderEventFixedFields, SessionEventEncoder,
    SessionEventFixedFields, SessionMessageHeaderDecoder,
};
use ergo_aeron_cluster::{
    AeronCluster, AsyncClusterConnect, ClusterError, ConnectStep, NullListener, SessionBuilder,
    SessionState,
};
use rusteron_client::{
    Aeron, AeronContext, AeronExclusivePublication, AeronSubscription, Handlers, cformat,
};
use serial_test::serial;

use common::{Capture, OwnDriver, launch_own_driver};

/// How long a test keeps the client's driver frozen.
const FROZEN_FOR: Duration = Duration::from_millis(1500);
/// Far below [`FROZEN_FOR`], so a poll that waited on the driver fails.
const POLL_BUDGET: Duration = Duration::from_millis(250);
const CLUSTER_SESSION_ID: i64 = 7;
const MEMBER_INGRESS: [&str; 2] = [
    "aeron:udp?endpoint=localhost:19231",
    "aeron:udp?endpoint=localhost:19232",
];
const ENDPOINTS: &str = "0=localhost:19231,1=localhost:19232";
const CLIENT_EGRESS: &str = "aeron:udp?endpoint=localhost:19233";
/// Differ from the driver defaults, so a rebuilt publication that dropped
/// them would report other limits.
const INGRESS_SETTINGS: &str = "|term-length=1m|mtu=4096";
const LIMITS: (usize, usize) = (1024 * 1024 / 8, 4096 - 32);

/// A Java media driver in its own process, so a test can freeze it without
/// freezing itself.
struct JavaDriver {
    child: Child,
    dir: String,
}

impl JavaDriver {
    fn launch(tag: &str) -> Result<Self, Box<dyn Error>> {
        let dir = std::env::temp_dir().join(format!("{tag}-{}", std::process::id()));
        let dir = dir
            .to_str()
            .ok_or("aeron dir is not valid UTF-8")?
            .to_string();
        let jar = ergo_aeron_cluster_test_harness::jar::try_find_jar("aeron-all-")?;
        let child = ergo_aeron_cluster_test_harness::jvm::try_command()?
            .arg("-cp")
            .arg(jar)
            .arg(format!("-Daeron.dir={dir}"))
            .args([
                "-Daeron.dir.delete.on.start=true",
                "-Daeron.dir.delete.on.shutdown=true",
                "io.aeron.driver.MediaDriver",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let driver = Self { child, dir };
        // Waits for the driver's CnC file and first heartbeat.
        connect(&driver.dir)?;
        Ok(driver)
    }

    /// Freeze the driver for [`FROZEN_FOR`] and run `poll` meanwhile, timing
    /// each call. Another thread resumes the driver, so a poll that waits on
    /// it ends rather than hangs.
    fn poll_frozen(
        &self,
        mut poll: impl FnMut() -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let pid = self.child.id();
        signal(pid, "-STOP")?;
        let resume = thread::spawn(move || {
            thread::sleep(FROZEN_FOR);
            signal(pid, "-CONT").map_err(|e| e.to_string())
        });
        let frozen_at = Instant::now();
        while frozen_at.elapsed() < FROZEN_FOR - Duration::from_millis(300) {
            let started = Instant::now();
            poll()?;
            assert!(
                started.elapsed() < POLL_BUDGET,
                "a poll took {:?} while the driver was frozen",
                started.elapsed()
            );
            thread::sleep(Duration::from_millis(1));
        }
        Ok(resume.join().map_err(|_| "resume thread panicked")??)
    }
}

impl Drop for JavaDriver {
    fn drop(&mut self) {
        let pid = self.child.id();
        let _ = signal(pid, "-CONT");
        // TERM runs the shutdown hook that deletes the driver directory.
        let _ = signal(pid, "-TERM");
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && matches!(self.child.try_wait(), Ok(None)) {
            thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn signal(pid: u32, signal: &str) -> Result<(), Box<dyn Error>> {
    let status = Command::new("kill")
        .arg(signal)
        .arg(pid.to_string())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("kill {signal} {pid}: {status}").into())
    }
}

fn connect(dir: &str) -> Result<Aeron, Box<dyn Error>> {
    let ctx = AeronContext::new()?;
    ctx.set_dir(&cformat!("{dir}"))?;
    let aeron = Aeron::new(&ctx)?;
    aeron.start()?;
    Ok(aeron)
}

fn wait_until(mut done: impl FnMut() -> bool, what: &str) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done() {
        if Instant::now() >= deadline {
            return Err(format!("timed out waiting for {what}").into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn offer(publication: &AeronExclusivePublication, message: &[u8]) -> Result<(), Box<dyn Error>> {
    wait_until(
        || publication.offer_raw(message, Handlers::NONE) > 0,
        "a fake member's offer",
    )
}

fn session_id(publication: &AeronExclusivePublication) -> Result<i32, Box<dyn Error>> {
    Ok(publication.get_constants()?.session_id)
}

fn session_event(
    code: EventCode,
    leader_member_id: i32,
    detail: &str,
    correlation_id: i64,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf =
        vec![0u8; SessionEventEncoder::compute_encoded_length_with_message_header(detail.len())];
    let len = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&SessionEventFixedFields {
            cluster_session_id: CLUSTER_SESSION_ID,
            correlation_id,
            leadership_term_id: 1,
            leader_member_id,
            code,
            version: Some(1),
            leader_heartbeat_timeout_ns: None,
        })
        .detail(detail.as_bytes())?
        .encoded_length_with_header();
    assert_eq!(len, buf.len());
    Ok(buf)
}

fn new_leader_event(
    leadership_term_id: i64,
    leader_member_id: i32,
    endpoints: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf =
        vec![
            0u8;
            NewLeaderEventEncoder::compute_encoded_length_with_message_header(endpoints.len())
        ];
    let len = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&NewLeaderEventFixedFields {
            leadership_term_id,
            cluster_session_id: CLUSTER_SESSION_ID,
            leader_member_id,
        })
        .ingress_endpoints(endpoints.as_bytes())?
        .encoded_length_with_header();
    assert_eq!(len, buf.len());
    Ok(buf)
}

fn builder(
    message_timeout: Duration,
    new_leader_timeout: Duration,
) -> Result<SessionBuilder, ClusterError> {
    SessionBuilder::default()
        .ingress_channel(format!("{}{INGRESS_SETTINGS}", MEMBER_INGRESS[0]))?
        .egress_channel(CLIENT_EGRESS)?
        .message_timeout(message_timeout)?
        .new_leader_timeout(new_leader_timeout)
}

fn finish(mut connecting: AsyncClusterConnect) -> Result<AeronCluster, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !connecting.is_complete() {
        connecting.poll()?;
        if Instant::now() >= deadline {
            return Err(format!("connect stuck at {:?}", connecting.step()).into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    Ok(connecting.finish()?)
}

/// Cluster members played on an in-process driver. Member `i` listens on
/// `MEMBER_INGRESS[i]`; each egress publication is a separate image on the
/// client's egress subscription, as each real member's is.
struct FakeCluster {
    members: Vec<AeronSubscription>,
    aeron: Aeron,
    _driver: OwnDriver,
}

impl FakeCluster {
    fn launch() -> Result<Self, Box<dyn Error>> {
        let driver = launch_own_driver("nonblocking-cluster")?;
        let aeron = connect(&driver.dir)?;
        let mut members = Vec::with_capacity(MEMBER_INGRESS.len());
        for channel in MEMBER_INGRESS {
            members.push(aeron.add_subscription(
                &cformat!("{channel}"),
                101,
                Handlers::NONE,
                Handlers::NONE,
                Duration::from_secs(5),
            )?);
        }
        Ok(Self {
            members,
            aeron,
            _driver: driver,
        })
    }

    /// A new egress image on the client, open once this returns.
    fn egress(&self) -> Result<AeronExclusivePublication, Box<dyn Error>> {
        let egress = self.aeron.add_exclusive_publication(
            &cformat!("{CLIENT_EGRESS}|term-length=64k"),
            102,
            Duration::from_secs(5),
        )?;
        wait_until(|| egress.is_connected(), "a fake member's egress image")?;
        Ok(egress)
    }

    /// Poll `connecting` until `member` hears its SessionConnectRequest, then
    /// answer over a new egress image. The reply echoes that request's
    /// correlation id.
    fn answer(
        &self,
        connecting: &mut AsyncClusterConnect,
        member: usize,
        reply: impl Fn(i64) -> Result<Vec<u8>, Box<dyn Error>>,
    ) -> Result<AeronExclusivePublication, Box<dyn Error>> {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut correlation = None;
        while correlation.is_none() {
            self.members[member].poll_fn(
                |data, _| {
                    if correlation.is_some() {
                        return;
                    }
                    if let Ok(AnyMessage::SessionConnectRequest(decoder)) =
                        AnyMessage::decode(data, 0)
                    {
                        correlation = Some(decoder.correlation_id());
                    }
                },
                10,
            )?;
            if correlation.is_some() {
                break;
            }
            connecting.poll()?;
            if Instant::now() >= deadline {
                return Err(format!(
                    "member {member} never heard the connect (step {:?})",
                    connecting.step()
                )
                .into());
            }
            thread::sleep(Duration::from_millis(1));
        }
        let correlation = correlation.ok_or("connect request had no correlation id")?;
        let egress = self.egress()?;
        offer(&egress, &reply(correlation)?)?;
        Ok(egress)
    }

    /// Connect through member 0, which answers OK over a new egress image.
    fn connect(
        &self,
        builder: SessionBuilder,
        client_driver: &JavaDriver,
    ) -> Result<(AeronCluster, AeronExclusivePublication), Box<dyn Error>> {
        let mut connecting = builder.connect_async(client_driver.dir.clone());
        let egress = self.answer(&mut connecting, 0, |correlation| {
            session_event(EventCode::OK, 0, "", correlation)
        })?;
        Ok((finish(connecting)?, egress))
    }

    /// Wait for `payload` from the client at `member`, returning the session
    /// header's leadership term and cluster session id.
    fn receive(&self, member: usize, payload: &[u8]) -> Result<(i64, i64), Box<dyn Error>> {
        let mut received = None;
        wait_until(
            || {
                let _ = self.members[member].poll_fn(
                    |data, _| {
                        if let Ok(header) = SessionMessageHeaderDecoder::decode(data, 0)
                            && data.get(AeronCluster::SESSION_HEADER_LENGTH..) == Some(payload)
                        {
                            received =
                                Some((header.leadership_term_id(), header.cluster_session_id()));
                        }
                    },
                    10,
                );
                received.is_some()
            },
            "a message at a fake member",
        )?;
        received.ok_or_else(|| "no message".into())
    }
}

/// The test's egress listener, with poll helpers.
struct Egress(Capture);

impl Egress {
    fn new() -> Self {
        Self(Capture::new())
    }

    fn poll(&mut self, client: &mut AeronCluster) -> Result<i32, ClusterError> {
        client.poll(&mut self.0, 10)
    }

    fn new_leader_member_id(&self) -> Option<i32> {
        self.0.new_leader_member_id
    }

    /// Poll `client` until it reaches `state`.
    fn poll_until(
        &mut self,
        client: &mut AeronCluster,
        state: SessionState,
    ) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while client.state() != state {
            self.poll(client)?;
            if Instant::now() >= deadline {
                return Err(format!("still {:?}, expected {state:?}", client.state()).into());
            }
            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    /// Offer `payload`, retrying while the publication is not connected yet.
    fn offer(&mut self, client: &mut AeronCluster, payload: &[u8]) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while let Err(error) = client.offer(payload) {
            if !error.is_retryable() || Instant::now() >= deadline {
                return Err(error.into());
            }
            self.poll(client)?;
            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }
}

/// Let messages offered by a fake member reach the client's image before the
/// test polls, so one poll reads them all.
fn let_land() {
    thread::sleep(Duration::from_millis(500));
}

/// Member 1's endpoint, which nothing subscribes to until a test says so.
const LATE_MEMBER: &str = "0=localhost:19231,1=localhost:19234";

/// Bring up the member a dropped add pointed at, and check the closed session
/// never adopts it.
fn assert_pending_add_dropped(
    cluster: &FakeCluster,
    client: &mut AeronCluster,
) -> Result<(), Box<dyn Error>> {
    let _late_member = cluster.aeron.add_subscription(
        c"aeron:udp?endpoint=localhost:19234",
        101,
        Handlers::NONE,
        Handlers::NONE,
        Duration::from_secs(5),
    )?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(1) {
        client.poll(&mut NullListener, 0)?;
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        (
            client.state(),
            client.leadership_term_id(),
            client.leader_member_id()
        ),
        (SessionState::Closed, 1, 0)
    );
    Ok(())
}

#[test]
#[serial]
fn async_connect_adds_its_transport_without_waiting() -> Result<(), Box<dyn Error>> {
    let driver = JavaDriver::launch("nonblocking-connect")?;
    let builder = SessionBuilder::default()
        .ingress_channel("aeron:udp?endpoint=localhost:19221")?
        .egress_channel("aeron:udp?endpoint=localhost:19222")?
        .message_timeout(Duration::from_secs(30))?;
    let mut connecting = builder.connect_async(driver.dir.clone());

    driver.poll_frozen(|| {
        connecting.poll()?;
        assert_eq!(
            connecting.step(),
            ConnectStep::CreateTransport,
            "a frozen driver cannot have added the transport"
        );
        Ok(())
    })?;

    let deadline = Instant::now() + Duration::from_secs(10);
    while connecting.step() == ConnectStep::CreateTransport {
        connecting.poll()?;
        if Instant::now() >= deadline {
            return Err("the transport was not added after the driver resumed".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(connecting.step(), ConnectStep::SendConnect);
    Ok(())
}

#[test]
#[serial]
fn redirect_adds_its_publication_without_waiting() -> Result<(), Box<dyn Error>> {
    let client_driver = JavaDriver::launch("nonblocking-redirect")?;
    let cluster = FakeCluster::launch()?;
    let mut connecting = builder(Duration::from_secs(10), Duration::from_secs(60))?
        .connect_async(client_driver.dir.clone());
    // Member 0 is a follower and redirects the client to member 1.
    let _follower_egress = cluster.answer(&mut connecting, 0, |correlation| {
        session_event(EventCode::REDIRECT, 1, ENDPOINTS, correlation)
    })?;
    let_land();

    client_driver.poll_frozen(|| {
        connecting.poll()?;
        Ok(())
    })?;
    assert_eq!(
        connecting.step(),
        ConnectStep::SendConnect,
        "the REDIRECT's publication waits for the driver, not the poll"
    );

    let _leader_egress = cluster.answer(&mut connecting, 1, |correlation| {
        session_event(EventCode::OK, 1, "", correlation)
    })?;
    let client = finish(connecting)?;
    assert_eq!(client.leader_member_id(), 1);
    assert_eq!(
        (
            client.ingress_max_message_length(),
            client.ingress_max_payload_length()
        ),
        LIMITS,
        "the redirected publication keeps the ingress channel settings"
    );
    Ok(())
}

fn failover_follows_the_session_egress_image_without_waiting(
    mut egress: Egress,
) -> Result<(), Box<dyn Error>> {
    let client_driver = JavaDriver::launch("nonblocking-failover")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, old_egress) = cluster.connect(
        builder(Duration::from_secs(10), Duration::from_secs(60))?,
        &client_driver,
    )?;
    let first_image = client
        .egress_image_correlation_id()
        .ok_or("no egress image bound at connect")?;
    assert_eq!(
        client.egress_image_session_id(),
        Some(session_id(&old_egress)?)
    );
    assert_eq!(
        (
            client.ingress_max_message_length(),
            client.ingress_max_payload_length()
        ),
        LIMITS
    );

    // Member 1 is elected and sends NewLeaderEvent over a second egress image
    // while member 0's image stays open, as a failed leader's image lingers.
    let new_egress = cluster.egress()?;
    offer(&new_egress, &new_leader_event(2, 1, ENDPOINTS)?)?;
    let_land();

    client_driver.poll_frozen(|| {
        egress.poll(&mut client)?;
        Ok(())
    })?;
    assert_eq!(
        egress.new_leader_member_id(),
        Some(1),
        "the NewLeaderEvent had not reached the client before its driver froze"
    );
    // The session moves to the new image at once, but keeps the old term
    // until the new leader's publication exists.
    assert_eq!(client.state(), SessionState::AwaitingNewLeader);
    assert_eq!(
        client.egress_image_session_id(),
        Some(session_id(&new_egress)?)
    );
    assert_ne!(client.egress_image_correlation_id(), Some(first_image));
    assert_eq!(client.leadership_term_id(), 1);
    assert!(matches!(
        client.offer(b"too early"),
        Err(ClusterError::NotConnected)
    ));

    // Once the driver adds the publication and it connects, the session is
    // member 1's, with the channel settings and so the limits it started with.
    egress.poll_until(&mut client, SessionState::Connected)?;
    assert_eq!(
        (client.leadership_term_id(), client.leader_member_id()),
        (2, 1)
    );
    assert_eq!(
        (
            client.ingress_max_message_length(),
            client.ingress_max_payload_length()
        ),
        LIMITS
    );
    egress.offer(&mut client, b"to member 1")?;
    assert_eq!(cluster.receive(1, b"to member 1")?, (2, CLUSTER_SESSION_ID));
    // The largest payload the limit allows goes out, fragmented.
    egress.offer(
        &mut client,
        &vec![7u8; LIMITS.0 - AeronCluster::SESSION_HEADER_LENGTH],
    )?;

    // The bound image closes while member 0's image is still open: a leader
    // loss that counting the subscription's images would miss.
    drop(new_egress);
    egress.poll_until(&mut client, SessionState::AwaitingNewLeader)?;
    assert!(
        old_egress.is_connected(),
        "member 0's egress image must still be open"
    );
    Ok(())
}

#[test]
#[serial]
fn failover_follows_the_session_egress_image_without_waiting_on_poll() -> Result<(), Box<dyn Error>>
{
    failover_follows_the_session_egress_image_without_waiting(Egress::new())
}

#[test]
#[serial]
fn closed_after_a_new_leader_in_one_poll_ends_the_session() -> Result<(), Box<dyn Error>> {
    let client_driver = JavaDriver::launch("nonblocking-closed")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, _old_egress) = cluster.connect(
        builder(Duration::from_secs(10), Duration::from_secs(60))?,
        &client_driver,
    )?;
    let new_egress = cluster.egress()?;
    offer(&new_egress, &new_leader_event(2, 1, ENDPOINTS)?)?;
    offer(&new_egress, &session_event(EventCode::CLOSED, 1, "", 0)?)?;
    let_land();

    Egress::new().poll(&mut client)?;
    assert_eq!(client.state(), SessionState::Closed);
    assert_eq!(client.leadership_term_id(), 1);
    Ok(())
}

#[test]
#[serial]
fn closed_while_a_new_leader_is_pending_ends_the_session() -> Result<(), Box<dyn Error>> {
    let client_driver = JavaDriver::launch("nonblocking-closed-pending")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, _old_egress) = cluster.connect(
        builder(Duration::from_secs(10), Duration::from_secs(60))?,
        &client_driver,
    )?;
    let new_egress = cluster.egress()?;
    // Nothing subscribes at member 1's endpoint yet, so the add stays pending.
    offer(&new_egress, &new_leader_event(2, 1, LATE_MEMBER)?)?;
    let mut egress = Egress::new();
    egress.poll_until(&mut client, SessionState::AwaitingNewLeader)?;

    offer(&new_egress, &session_event(EventCode::CLOSED, 1, "", 0)?)?;
    let deadline = Instant::now() + Duration::from_secs(3);
    while client.state() != SessionState::Closed {
        egress.poll(&mut client)?;
        if Instant::now() >= deadline {
            return Err(format!("still {:?}: the CLOSED was lost", client.state()).into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_pending_add_dropped(&cluster, &mut client)
}

#[test]
#[serial]
fn close_while_a_new_leader_is_pending_drops_its_publication() -> Result<(), Box<dyn Error>> {
    let client_driver = JavaDriver::launch("nonblocking-close-pending")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, _old_egress) = cluster.connect(
        builder(Duration::from_secs(10), Duration::from_secs(60))?,
        &client_driver,
    )?;
    let new_egress = cluster.egress()?;
    offer(&new_egress, &new_leader_event(2, 1, LATE_MEMBER)?)?;
    Egress::new().poll_until(&mut client, SessionState::AwaitingNewLeader)?;

    client.close()?;
    assert_pending_add_dropped(&cluster, &mut client)
}

#[test]
#[serial]
fn closed_beside_a_failing_new_leader_event_ends_the_session() -> Result<(), Box<dyn Error>> {
    let client_driver = JavaDriver::launch("nonblocking-closed-failing")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, _old_egress) = cluster.connect(
        builder(Duration::from_secs(10), Duration::from_secs(60))?,
        &client_driver,
    )?;
    let new_egress = cluster.egress()?;
    // Member 2 is not in the endpoint list, so this event alone would fail.
    offer(&new_egress, &new_leader_event(2, 2, ENDPOINTS)?)?;
    offer(&new_egress, &session_event(EventCode::CLOSED, 1, "", 0)?)?;
    let_land();

    Egress::new().poll(&mut client)?;
    assert_eq!(client.state(), SessionState::Closed);
    Ok(())
}

#[test]
#[serial]
fn a_new_leader_whose_image_closes_waits_for_the_next_one() -> Result<(), Box<dyn Error>> {
    let message_timeout = Duration::from_secs(4);
    let client_driver = JavaDriver::launch("nonblocking-leader-lost")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, _old_egress) = cluster.connect(
        builder(message_timeout, Duration::from_secs(20))?,
        &client_driver,
    )?;
    let first = cluster.egress()?;
    offer(&first, &new_leader_event(2, 1, LATE_MEMBER)?)?;
    let mut egress = Egress::new();
    egress.poll_until(&mut client, SessionState::AwaitingNewLeader)?;
    let event_at = Instant::now();

    // Member 1 fails before its ingress connects: its egress image closes.
    drop(first);
    wait_until(
        || {
            let _ = egress.poll(&mut client);
            !client.is_egress_connected()
        },
        "member 1's egress image to close",
    )?;
    assert!(
        event_at.elapsed() < message_timeout,
        "the image took {:?} to close, too long for this test",
        event_at.elapsed()
    );
    // Past message_timeout the session still waits, now for a new leader.
    while event_at.elapsed() < message_timeout + Duration::from_secs(1) {
        egress.poll(&mut client)?;
        assert_eq!(client.state(), SessionState::AwaitingNewLeader);
        thread::sleep(Duration::from_millis(1));
    }

    let second = cluster.egress()?;
    offer(&second, &new_leader_event(3, 0, ENDPOINTS)?)?;
    egress.poll_until(&mut client, SessionState::Connected)?;
    assert_eq!(
        (client.leadership_term_id(), client.leader_member_id()),
        (3, 0)
    );
    Ok(())
}

#[test]
#[serial]
fn the_last_new_leader_event_in_one_poll_wins() -> Result<(), Box<dyn Error>> {
    let client_driver = JavaDriver::launch("nonblocking-last-leader")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, _old_egress) = cluster.connect(
        builder(Duration::from_secs(10), Duration::from_secs(60))?,
        &client_driver,
    )?;
    let new_egress = cluster.egress()?;
    offer(&new_egress, &new_leader_event(2, 1, ENDPOINTS)?)?;
    offer(&new_egress, &new_leader_event(3, 0, ENDPOINTS)?)?;
    let_land();

    let mut egress = Egress::new();
    wait_until(
        || {
            let _ = egress.poll(&mut client);
            client.state() == SessionState::Connected && client.leadership_term_id() != 1
        },
        "the session to adopt a new term",
    )?;
    assert_eq!(
        (client.leadership_term_id(), client.leader_member_id()),
        (3, 0)
    );
    Ok(())
}

#[test]
#[serial]
fn a_new_leader_that_never_connects_times_out() -> Result<(), Box<dyn Error>> {
    let message_timeout = Duration::from_secs(3);
    let client_driver = JavaDriver::launch("nonblocking-unreachable")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, _old_egress) = cluster.connect(
        builder(message_timeout, Duration::from_secs(60))?,
        &client_driver,
    )?;
    let new_egress = cluster.egress()?;
    // Nothing subscribes at member 1's advertised endpoint.
    offer(
        &new_egress,
        &new_leader_event(2, 1, "0=localhost:19231,1=localhost:19234")?,
    )?;

    let_land();

    let mut egress = Egress::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut event_at = None;
    loop {
        match egress.poll(&mut client) {
            Err(ClusterError::Disconnected { .. }) => break,
            Err(error) => return Err(error.into()),
            Ok(_) => {}
        }
        if egress.new_leader_member_id().is_some() {
            event_at.get_or_insert_with(Instant::now);
            assert_ne!(
                client.state(),
                SessionState::Connected,
                "a session must not be Connected before its new publication connects"
            );
        }
        if Instant::now() >= deadline {
            return Err(format!("still {:?} after 20s", client.state()).into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    let waited = event_at.ok_or("no NewLeaderEvent")?.elapsed();
    assert!(
        waited >= message_timeout - Duration::from_millis(100),
        "closed after {waited:?}, before message_timeout"
    );
    assert_eq!(client.state(), SessionState::Closed);
    Ok(())
}

#[test]
#[serial]
fn a_failed_new_leader_add_is_reported_once() -> Result<(), Box<dyn Error>> {
    let message_timeout = Duration::from_secs(3);
    let client_driver = JavaDriver::launch("nonblocking-failed-add")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, _old_egress) = cluster.connect(
        builder(message_timeout, Duration::from_secs(60))?,
        &client_driver,
    )?;
    let new_egress = cluster.egress()?;
    // The driver rejects the port, so the add fails.
    offer(
        &new_egress,
        &new_leader_event(2, 1, "0=localhost:19231,1=localhost:99999")?,
    )?;
    let_land();

    let mut egress = Egress::new();
    let mut failures = 0;
    let started = Instant::now();
    // After the failure the session waits out message_timeout, as Java's
    // AWAIT_NEW_LEADER_CONNECTION does, not new_leader_timeout.
    let reason = loop {
        match egress.poll(&mut client) {
            Err(ClusterError::ReconnectFailed { .. }) => failures += 1,
            Err(ClusterError::Disconnected { reason }) => break reason,
            Err(error) => return Err(error.into()),
            Ok(_) => {
                assert_eq!(client.state(), SessionState::AwaitingNewLeader);
                assert_eq!(
                    (client.leadership_term_id(), client.leader_member_id()),
                    (1, 0)
                );
            }
        }
        if started.elapsed() > Duration::from_secs(20) {
            return Err(format!("still {:?} after 20s", client.state()).into());
        }
        thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(failures, 1, "the failed add is reported exactly once");
    assert!(
        started.elapsed() < message_timeout + Duration::from_secs(2),
        "closed after {:?}",
        started.elapsed()
    );
    assert!(reason.contains("message_timeout"), "{reason}");
    Ok(())
}

#[test]
#[serial]
fn a_new_leader_event_without_the_leader_changes_nothing() -> Result<(), Box<dyn Error>> {
    let client_driver = JavaDriver::launch("nonblocking-no-leader")?;
    let cluster = FakeCluster::launch()?;
    let (mut client, old_egress) = cluster.connect(
        builder(Duration::from_secs(10), Duration::from_secs(60))?,
        &client_driver,
    )?;
    let image = client.egress_image_correlation_id();
    let new_egress = cluster.egress()?;
    // Member 2 is not in the endpoint list.
    offer(&new_egress, &new_leader_event(2, 2, ENDPOINTS)?)?;
    let_land();

    assert!(matches!(
        Egress::new().poll(&mut client),
        Err(ClusterError::ReconnectFailed { .. })
    ));
    assert_eq!(client.state(), SessionState::Connected);
    assert_eq!(
        (client.leadership_term_id(), client.leader_member_id()),
        (1, 0)
    );
    assert_eq!(client.egress_image_correlation_id(), image);
    assert_eq!(
        client.egress_image_session_id(),
        Some(session_id(&old_egress)?)
    );
    Ok(())
}
