//! A fake cluster on an in-process driver: the tests play each member, so
//! every step of the session protocol happens on cue.
//!
//! Include it beside `common` (`mod common; mod fake;`).

use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};

use ergo_aeron_cluster::cluster_codec_types::{
    AnyMessage, EventCode, NewLeaderEventEncoder, NewLeaderEventFixedFields, SessionEventEncoder,
    SessionEventFixedFields, SessionMessageHeaderDecoder,
};
use ergo_aeron_cluster::{AeronCluster, AsyncClusterConnect, SessionBuilder};
use rusteron_client::{
    Aeron, AeronContext, AeronExclusivePublication, AeronSubscription, Handlers, cformat,
};

use crate::common::{OwnDriver, launch_own_driver};

pub const CLUSTER_SESSION_ID: i64 = 7;
pub const MEMBER_INGRESS: [&str; 2] = [
    "aeron:udp?endpoint=localhost:19231",
    "aeron:udp?endpoint=localhost:19232",
];
pub const ENDPOINTS: &str = "0=localhost:19231,1=localhost:19232";
pub const CLIENT_EGRESS: &str = "aeron:udp?endpoint=localhost:19233";

/// A started Aeron client on the driver in `dir`.
pub fn connect(dir: &str) -> Result<Aeron, Box<dyn Error>> {
    let ctx = AeronContext::new()?;
    ctx.set_dir(&cformat!("{dir}"))?;
    let aeron = Aeron::new(&ctx)?;
    aeron.start()?;
    Ok(aeron)
}

pub fn wait_until(mut done: impl FnMut() -> bool, what: &str) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done() {
        if Instant::now() >= deadline {
            return Err(format!("timed out waiting for {what}").into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

pub fn offer(
    publication: &AeronExclusivePublication,
    message: &[u8],
) -> Result<(), Box<dyn Error>> {
    wait_until(
        || publication.offer_raw(message, Handlers::NONE) > 0,
        "a fake member's offer",
    )
}

pub fn session_id(publication: &AeronExclusivePublication) -> Result<i32, Box<dyn Error>> {
    Ok(publication.get_constants()?.session_id)
}

pub fn session_event(
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

pub fn new_leader_event(
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

pub fn finish(mut connecting: AsyncClusterConnect) -> Result<AeronCluster, Box<dyn Error>> {
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

/// Poll `connecting` until `member` hears a `SessionConnectRequest`, and
/// return its correlation id.
pub fn await_connect_request(
    cluster: &FakeCluster,
    connecting: &mut AsyncClusterConnect,
    member: usize,
) -> Result<i64, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(correlation) = cluster.connect_requests(member)?.first() {
            return Ok(*correlation);
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
}

/// Cluster members played on an in-process driver. Member `i` listens on
/// `MEMBER_INGRESS[i]`; each egress publication is a separate image on the
/// client's egress subscription, as each real member's is.
pub struct FakeCluster {
    pub members: Vec<AeronSubscription>,
    pub aeron: Aeron,
    _driver: OwnDriver,
}

impl FakeCluster {
    pub fn launch() -> Result<Self, Box<dyn Error>> {
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
    pub fn egress(&self) -> Result<AeronExclusivePublication, Box<dyn Error>> {
        let egress = self.aeron.add_exclusive_publication(
            &cformat!("{CLIENT_EGRESS}|term-length=64k"),
            102,
            Duration::from_secs(5),
        )?;
        wait_until(|| egress.is_connected(), "a fake member's egress image")?;
        Ok(egress)
    }

    /// The correlation ids of the `SessionConnectRequest`s `member` has
    /// heard since the last call.
    pub fn connect_requests(&self, member: usize) -> Result<Vec<i64>, Box<dyn Error>> {
        let mut correlations = Vec::new();
        self.members[member].poll_fn(
            |data, _| {
                if let Ok(AnyMessage::SessionConnectRequest(decoder)) = AnyMessage::decode(data, 0)
                {
                    correlations.push(decoder.correlation_id());
                }
            },
            10,
        )?;
        Ok(correlations)
    }

    /// The `(leadership_term_id, cluster_session_id)` of each
    /// `SessionCloseRequest` `member` hears within `within`.
    pub fn close_requests(
        &self,
        member: usize,
        within: Duration,
    ) -> Result<Vec<(i64, i64)>, Box<dyn Error>> {
        let mut requests = Vec::new();
        let started = Instant::now();
        while started.elapsed() < within {
            self.members[member].poll_fn(
                |data, _| {
                    if let Ok(AnyMessage::SessionCloseRequest(decoder)) =
                        AnyMessage::decode(data, 0)
                    {
                        requests.push((decoder.leadership_term_id(), decoder.cluster_session_id()));
                    }
                },
                10,
            )?;
            thread::sleep(Duration::from_millis(1));
        }
        Ok(requests)
    }

    /// Poll `connecting` until `member` hears its `SessionConnectRequest`, then
    /// answer over a new egress image. The reply echoes that request's
    /// correlation id.
    pub fn answer(
        &self,
        connecting: &mut AsyncClusterConnect,
        member: usize,
        reply: impl Fn(i64) -> Result<Vec<u8>, Box<dyn Error>>,
    ) -> Result<AeronExclusivePublication, Box<dyn Error>> {
        let correlation = await_connect_request(self, connecting, member)?;
        let egress = self.egress()?;
        offer(&egress, &reply(correlation)?)?;
        Ok(egress)
    }

    /// Connect a client on the driver in `client_dir` through member 0, which
    /// answers OK over a new egress image.
    pub fn connect(
        &self,
        builder: SessionBuilder,
        client_dir: &str,
    ) -> Result<(AeronCluster, AeronExclusivePublication), Box<dyn Error>> {
        let mut connecting = builder.connect_async(client_dir.to_owned());
        let egress = self.answer(&mut connecting, 0, |correlation| {
            session_event(EventCode::OK, 0, "", correlation)
        })?;
        Ok((finish(connecting)?, egress))
    }

    /// Wait for `payload` from the client at `member`, returning the session
    /// header's leadership term and cluster session id.
    pub fn receive(&self, member: usize, payload: &[u8]) -> Result<(i64, i64), Box<dyn Error>> {
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
