//! Scenario implementations for the interoperability harness (plan §11.2).
//!
//! Each scenario exercises a §11.2 virtual-device case through the full stack:
//! embedded media driver → transport (DPDK vdev or default kernel UDP) →
//! rusteron-client pub/sub. Cross-driver unicast uses manual MDC
//! (`control-mode=manual` + `add_destination`), the pattern a real
//! publisher/subscriber on separate nodes uses.
//!
//! Scenarios are symmetric (both roles send and receive) or asymmetric
//! (primary transmits, secondary receives); the `role` branch decides.

use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hdrhistogram::Histogram;
use log::{info, warn};
use rusteron_client::{Aeron, AeronExclusivePublication, AeronSubscription, Handlers, IntoCString};

/// A single `host:port` endpoint (IPv4 — this plan targets Nitro, no brackets).
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

impl Endpoint {
    /// Parse `host:port`.
    pub fn parse(s: &str) -> Self {
        let (host, port) = s.rsplit_once(':').expect("endpoint must be host:port");
        Endpoint {
            host: host.to_string(),
            port: port.parse().expect("endpoint port must be numeric"),
        }
    }

    /// `aeron:udp?endpoint=host:port` — a subscription or MDC destination URI.
    pub fn udp_uri(&self) -> String {
        format!("aeron:udp?endpoint={}:{}", self.host, self.port)
    }
}

/// Per-process configuration, read from the environment by `main`.
#[derive(Clone, Debug)]
pub struct ScenarioCfg {
    pub role: String,
    pub name: String,
    /// Local MDC control endpoint (publication URI `control-mode=manual`).
    pub pub_ctrl: Endpoint,
    /// Second control endpoint — the reconnect scenario re-opens here.
    pub pub_ctrl2: Endpoint,
    /// Local subscription endpoint(s); `[0]` is the primary.
    pub sub_endpoints: Vec<Endpoint>,
    /// Peer subscription endpoint(s) the publication sends to.
    pub destinations: Vec<Endpoint>,
    pub msgs: u64,
    pub payload: usize,
    pub stream: i32,
    pub timeout: Duration,
    /// Byte this process fills its payload body with (direction is visible).
    pub fill_byte: u8,
    /// Byte this process expects the peer's payload body to contain.
    pub expect_byte: u8,
    /// Media driver MTU; the driver fragments above this and the DPDK transport
    /// bounds every emitted datagram by it (§8).
    pub mtu: usize,
    /// `perf`-scenario wall-clock bound (`None` = the vdev scenarios' msg-count mode).
    pub duration: Option<Duration>,
    /// `perf`-scenario offered rate (`0` = saturation, no pacing).
    pub load_rps: u64,
    /// Where the receiver appends raw one-way latency samples (`None` = skip).
    pub latency_samples: Option<String>,
}

impl ScenarioCfg {
    pub fn sub(&self) -> &Endpoint {
        &self.sub_endpoints[0]
    }
    pub fn dest(&self) -> &Endpoint {
        &self.destinations[0]
    }
}

/// Outcome of a scenario run, mapped into the JSON report by `main`.
#[derive(Default, Debug)]
pub struct ScenarioResult {
    pub ok: bool,
    pub sent: u64,
    pub received: u64,
    pub bytes: u64,
    pub bad_payload: u64,
    pub duration_ms: u64,
    /// Receiver-side one-way latency histogram (plan §12); zero in count-mode runs.
    pub latency_p50_ns: u64,
    pub latency_p99_ns: u64,
    pub latency_max_ns: u64,
    /// Throughput derived from the perf run's wall clock.
    pub offered_per_sec: f64,
    pub delivered_per_sec: f64,
    /// Offer calls that returned retryable/backpressured (sender-side).
    pub backpressure_ops: u64,
    /// Unrecovered sequence gaps observed by the receiver.
    pub gaps: u64,
    pub detail: Vec<String>,
    pub error: Option<String>,
}

impl ScenarioResult {
    fn fail(msg: impl Into<String>) -> Self {
        ScenarioResult {
            ok: false,
            error: Some(msg.into()),
            ..Default::default()
        }
    }
    fn push(&mut self, msg: impl Into<String>) {
        self.detail.push(msg.into());
    }
}

/// Run the named scenario. `name` comes from `--scenario`.
pub fn run(cfg: &ScenarioCfg, aeron: &Aeron) -> ScenarioResult {
    let started = Instant::now();
    let mut res = match cfg.name.as_str() {
        // Cross-transport scenarios run the same symmetric unicast flow — only
        // the media driver transport differs (script's `scenario_bidi` wires
        // DESTINATIONS/SUB_ENDPOINTS per transport).
        "bidirectional_unicast" | "restart" | "dpdk_to_udp" | "udp_to_dpdk" | "loss_recovery" => {
            symmetric_unicast(cfg, aeron)
        }
        "reconnect" => reconnect(cfg, aeron),
        "multi_endpoint" => multi_endpoint(cfg, aeron),
        "perf" => perf(cfg, aeron),
        other => return ScenarioResult::fail(format!("unknown scenario {other:?}")),
    };
    // The sender's throughput is its own offers; the receiver's is its deliveries.
    if res.sent > 0 && res.duration_ms > 0 {
        res.offered_per_sec = res.sent as f64 / (res.duration_ms as f64 / 1000.0);
    }
    if res.received > 0 && res.duration_ms > 0 {
        res.delivered_per_sec = res.received as f64 / (res.duration_ms as f64 / 1000.0);
    }
    res.duration_ms = started.elapsed().as_millis() as u64;
    res
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_publication(
    aeron: &Aeron,
    ctrl: &Endpoint,
    stream: i32,
    timeout: Duration,
) -> Result<AeronExclusivePublication, Box<dyn Error>> {
    let uri = format!("aeron:udp?control-mode=manual|control={}:{}", ctrl.host, ctrl.port);
    let pub_ = aeron.add_exclusive_publication(&uri.clone().into_c_string(), stream, timeout)?;
    info!("[pub] channel={uri}");
    Ok(pub_)
}

fn open_subscription(
    aeron: &Aeron,
    ep: &Endpoint,
    stream: i32,
    timeout: Duration,
) -> Result<AeronSubscription, Box<dyn Error>> {
    let uri = ep.udp_uri();
    let sub = aeron.add_subscription(
        &uri.clone().into_c_string(),
        stream,
        Handlers::NONE,
        Handlers::NONE,
        timeout,
    )?;
    info!("[sub] channel={uri}");
    Ok(sub)
}

/// Add an MDC destination and wait until the publication is connected to it.
fn add_and_wait(pub_: &AeronExclusivePublication, dest: &Endpoint, timeout: Duration) -> Result<(), Box<dyn Error>> {
    let uri = dest.udp_uri();
    pub_.add_destination(&uri.clone().into_c_string(), timeout)?;
    let started = Instant::now();
    while !pub_.is_connected() && started.elapsed() < timeout {
        sleep(Duration::from_millis(10));
    }
    if !pub_.is_connected() {
        return Err(format!("publication never connected to destination {uri}").into());
    }
    info!("[pub] connected to {uri}");
    Ok(())
}

/// Offer one message, retrying transient errors until it is accepted. Returns
/// the number of retry passes taken, so callers can surface backpressure.
fn offer_retry(pub_: &AeronExclusivePublication, msg: &[u8], timeout: Duration) -> Result<u64, Box<dyn Error>> {
    let started = Instant::now();
    let mut retries = 0u64;
    loop {
        match pub_.offer(msg) {
            Ok(pos) if pos >= 0 => return Ok(retries),
            Ok(_) => { /* negative position — unreachable via the typed wrapper */ }
            Err(e) if e.is_retryable() && started.elapsed() < timeout => retries += 1,
            Err(e) => return Err(format!("offer failed: {e}").into()),
        }
        if started.elapsed() >= timeout {
            return Err("offer retry timed out".into());
        }
        sleep(Duration::from_millis(1));
    }
}

struct Recv {
    received: u64,
    bytes: u64,
    bad_payload: u64,
    /// Unrecovered sequence gaps (NAK/retransmit recovers most loss).
    gaps: u64,
    /// Repeated sequence deliveries (retransmit raced the original). Counted
    /// separately so the exact-count assertions see only unique messages.
    dups: u64,
    last_seq: Option<u64>,
    histogram: Histogram<u64>,
    samples: Option<BufWriter<File>>,
}

impl Recv {
    fn new(latency_samples: Option<&str>) -> Result<Recv, Box<dyn Error>> {
        let samples = match latency_samples {
            Some(path) => Some(BufWriter::new(File::create(path)?)),
            None => None,
        };
        // 1 ns .. 10 s, 3 significant digits — a 60s @ saturation run of 1 MiB
        // messages must not overflow the histogram.
        Ok(Recv {
            received: 0,
            bytes: 0,
            bad_payload: 0,
            gaps: 0,
            dups: 0,
            last_seq: None,
            histogram: Histogram::new_with_bounds(1, 10_000_000_000, 3)?,
            samples,
        })
    }

    fn flush_samples(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(w) = self.samples.as_mut() {
            w.flush()?;
        }
        Ok(())
    }
}

impl Default for Recv {
    fn default() -> Self {
        Self::new(None).unwrap()
    }
}

/// Drain `sub` once, validating the payload pattern, tracking sequence gaps,
/// and recording one-way latency. The sender stamps wall-clock nanoseconds at
/// [8..16] (both nodes NTP-synchronise on EKS; a stable clock skew cancels out
/// of mode-vs-mode comparisons — see the benchmark report).
fn poll_once(sub: &AeronSubscription, expect_byte: u8, recv: &mut Recv) -> Result<(), Box<dyn Error>> {
    sub.poll_fn(
        |data, _header| {
            if data.len() < 17 {
                return;
            }
            let seq = u64::from_le_bytes(data[0..8].try_into().unwrap());
            if let Some(last) = recv.last_seq {
                // Each sub in multi_endpoint owns a disjoint, monotonic range
                // (round-robin distribution), so a repeat is a retransmit that
                // raced the original — count it, don't deliver it again.
                if seq <= last {
                    recv.dups += 1;
                    return;
                }
                if seq != last + 1 {
                    recv.gaps += seq.wrapping_sub(last).saturating_sub(1);
                }
            }
            recv.last_seq = Some(seq);

            let ts = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let now = now_wall_ns();
            if now >= ts {
                let lat = now - ts;
                recv.histogram.record(lat).ok();
                if let Some(w) = recv.samples.as_mut() {
                    let _ = writeln!(w, "{lat}");
                }
            }

            recv.received += 1;
            recv.bytes += data.len() as u64;
            if data[16..].iter().any(|&b| b != expect_byte) {
                recv.bad_payload += 1;
            }
        },
        1024,
    )?;
    Ok(())
}

fn now_wall_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn build_msg(cfg: &ScenarioCfg, seq: u64) -> Vec<u8> {
    let mut m = vec![0u8; 16 + cfg.payload];
    m[16..].fill(cfg.fill_byte);
    m[0..8].copy_from_slice(&seq.to_le_bytes());
    m[8..16].copy_from_slice(&now_wall_ns().to_le_bytes());
    m
}

fn send_batch(
    pub_: &AeronExclusivePublication,
    cfg: &ScenarioCfg,
    range: std::ops::Range<u64>,
) -> Result<(), Box<dyn Error>> {
    for seq in range {
        let msg = build_msg(cfg, seq);
        offer_retry(pub_, &msg, cfg.timeout)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

/// §11.2 DPDK-to-DPDK / DPDK-to-default bidirectional unicast. Both roles
/// publish and subscribe; every message must round-trip byte-identically.
fn symmetric_unicast(cfg: &ScenarioCfg, aeron: &Aeron) -> ScenarioResult {
    let mut res = ScenarioResult::default();
    let result = (|| -> Result<(), Box<dyn Error>> {
        let pub_ = open_publication(aeron, &cfg.pub_ctrl, cfg.stream, cfg.timeout)?;
        let sub = open_subscription(aeron, cfg.sub(), cfg.stream, cfg.timeout)?;
        add_and_wait(&pub_, cfg.dest(), cfg.timeout)?;

        let mut sent: u64 = 0;
        let mut recv = Recv::default();
        let started = Instant::now();
        while (sent < cfg.msgs || recv.received < cfg.msgs) && started.elapsed() < cfg.timeout {
            if sent < cfg.msgs {
                let msg = build_msg(cfg, sent);
                offer_retry(&pub_, &msg, cfg.timeout)?;
                sent += 1;
            }
            poll_once(&sub, cfg.expect_byte, &mut recv)?;
            sleep(Duration::from_millis(1));
        }
        res.sent = sent;
        res.received = recv.received;
        res.bytes = recv.bytes;
        res.bad_payload = recv.bad_payload;
        if sent != cfg.msgs {
            return Err(format!("sent {sent} of {} messages", cfg.msgs).into());
        }
        if recv.received != cfg.msgs {
            return Err(format!("received {} of {} messages", recv.received, cfg.msgs).into());
        }
        if recv.bad_payload != 0 {
            return Err(format!("{} payload byte mismatches", recv.bad_payload).into());
        }
        res.push(format!(
            "bidirectional round-trip ok ({sent} messages, {} bytes)",
            res.bytes
        ));
        Ok(())
    })();

    match result {
        Ok(()) => res.ok = true,
        Err(e) => {
            warn!("scenario failed: {e}");
            res.error = Some(e.to_string());
        }
    }
    res
}

/// §11.2 transport reconnect: the primary closes its publication to the first
/// destination and opens a fresh one to a second destination; the secondary
/// receives both batches across its two subscriptions. The first batch must
/// not be corrupted by the reconnect, and the second must land on the new
/// transport — no caller-pointer retention in the copied remote address.
fn reconnect(cfg: &ScenarioCfg, aeron: &Aeron) -> ScenarioResult {
    let mut res = ScenarioResult::default();
    let split = cfg.msgs / 3;
    let result = if cfg.role == "primary" {
        (|| -> Result<(), Box<dyn Error>> {
            let pub1 = open_publication(aeron, &cfg.pub_ctrl, cfg.stream, cfg.timeout)?;
            add_and_wait(&pub1, cfg.dest(), cfg.timeout)?;
            send_batch(&pub1, cfg, 0..split)?;
            drop(pub1); // close the first transport; the next is a fresh create

            let pub2 = open_publication(aeron, &cfg.pub_ctrl2, cfg.stream, cfg.timeout)?;
            add_and_wait(&pub2, &cfg.destinations[1], cfg.timeout)?;
            send_batch(&pub2, cfg, split..cfg.msgs)?;
            res.sent = cfg.msgs;
            res.push(format!(
                "reconnected: batch1 {split} msgs, batch2 {} msgs",
                cfg.msgs - split
            ));
            Ok(())
        })()
    } else {
        (|| -> Result<(), Box<dyn Error>> {
            let sub0 = open_subscription(aeron, &cfg.sub_endpoints[0], cfg.stream, cfg.timeout)?;
            let sub1 = open_subscription(aeron, &cfg.sub_endpoints[1], cfg.stream, cfg.timeout)?;
            let mut recv = Recv::default();
            let started = Instant::now();
            while recv.received < cfg.msgs && started.elapsed() < cfg.timeout {
                poll_once(&sub0, cfg.expect_byte, &mut recv)?;
                poll_once(&sub1, cfg.expect_byte, &mut recv)?;
                sleep(Duration::from_millis(1));
            }
            res.received = recv.received;
            res.bytes = recv.bytes;
            res.bad_payload = recv.bad_payload;
            if recv.received != cfg.msgs {
                return Err(format!(
                    "received {} of {} messages across both transports",
                    recv.received, cfg.msgs
                )
                .into());
            }
            if recv.bad_payload != 0 {
                return Err(format!("{} payload byte mismatches", recv.bad_payload).into());
            }
            res.push(format!(
                "reconnected: received {} messages on both transports",
                recv.received
            ));
            Ok(())
        })()
    };

    match result {
        Ok(()) => res.ok = true,
        Err(e) => {
            warn!("scenario failed: {e}");
            res.error = Some(e.to_string());
        }
    }
    res
}

/// §11.2 multiple registered endpoints (and loss recovery — same code path):
/// the primary adds every destination; the secondary registers one
/// subscription per endpoint and must receive `msgs × endpoints`. Under
/// `tc netem` loss on the bridge the driver's NAK/retransmit path recovers
/// every gap, so the secondary still sees all messages.
fn multi_endpoint(cfg: &ScenarioCfg, aeron: &Aeron) -> ScenarioResult {
    let mut res = ScenarioResult::default();
    let result = if cfg.role == "primary" {
        (|| -> Result<(), Box<dyn Error>> {
            let pub_ = open_publication(aeron, &cfg.pub_ctrl, cfg.stream, cfg.timeout)?;
            for dest in &cfg.destinations {
                add_and_wait(&pub_, dest, cfg.timeout)?;
            }
            // is_connected() is a single aggregate flag that turns true as soon
            // as ANY destination connects, so add_and_wait returns before the
            // remaining destinations have completed their own connection
            // handshake. Offering immediately then silently drops the leading
            // frames to a not-yet-connected destination (the MDC tracker treats
            // a no-route send as success, never retries). Settle briefly so
            // every destination is connected before the first offer.
            sleep(Duration::from_millis(250));
            // An MDC publication casts every offer to ALL registered
            // destinations (aeron_udp_destination_tracker_send fans the iovec
            // out to each), so offering `msgs` total hands every endpoint the
            // full `msgs` range and the secondary (which subscribes to every
            // endpoint) expects `msgs * endpoints`. Pace the offers like
            // symmetric_unicast: the vdev bridge/tap path drops frames when a
            // sender bursts, and only the retransmitted data frames survive.
            let total = cfg.msgs;
            let started = Instant::now();
            let mut sent = 0u64;
            while sent < total && started.elapsed() < cfg.timeout {
                let msg = build_msg(cfg, sent);
                let retries = offer_retry(&pub_, &msg, cfg.timeout)?;
                res.backpressure_ops += retries;
                sent += 1;
                sleep(Duration::from_millis(1));
            }
            res.sent = sent;
            if sent != total {
                return Err(format!(
                    "sent {sent} of {total} messages across {} destinations",
                    cfg.destinations.len()
                )
                .into());
            }
            res.push(format!(
                "published {total} messages to {n} destinations",
                n = cfg.destinations.len()
            ));
            Ok(())
        })()
    } else {
        (|| -> Result<(), Box<dyn Error>> {
            let mut subs = Vec::new();
            for ep in &cfg.sub_endpoints {
                subs.push(open_subscription(aeron, ep, cfg.stream, cfg.timeout)?);
            }
            let expect = cfg.msgs * cfg.sub_endpoints.len() as u64;
            // MDC-cast hands every endpoint the same full range, but each sub's
            // stream is its own monotonic copy, so track them separately: a
            // shared `last_seq` would conflate three streams that each advance
            // by ~1 per poll round and mis-count gaps.
            let mut recvs: Vec<Recv> = (0..subs.len()).map(|_| Recv::default()).collect();
            let started = Instant::now();
            loop {
                let done: u64 = recvs.iter().map(|r| r.received).sum();
                if done >= expect || started.elapsed() >= cfg.timeout {
                    break;
                }
                for (sub, recv) in subs.iter().zip(recvs.iter_mut()) {
                    poll_once(sub, cfg.expect_byte, recv)?;
                }
                sleep(Duration::from_millis(1));
            }
            res.received = recvs.iter().map(|r| r.received).sum();
            res.bytes = recvs.iter().map(|r| r.bytes).sum();
            res.bad_payload = recvs.iter().map(|r| r.bad_payload).sum();
            let dups: u64 = recvs.iter().map(|r| r.dups).sum();
            if res.received != expect {
                // gaps=0 with a short count ⇒ prefix loss (frames offered before
                // the destination connected); gaps>0 ⇒ interleaved/ongoing loss.
                let per_sub: Vec<(u64, u64, u64)> = recvs.iter().map(|r| (r.received, r.dups, r.gaps)).collect();
                return Err(format!(
                    "received {} of {expect} messages across {} endpoints (per-sub received/dups/gaps: {per_sub:?})",
                    res.received,
                    subs.len()
                )
                .into());
            }
            if res.bad_payload != 0 {
                return Err(format!("{} payload byte mismatches", res.bad_payload).into());
            }
            res.push(format!(
                "received {} messages across {} endpoints{}",
                res.received,
                subs.len(),
                if dups != 0 {
                    format!(" ({dups} duplicate retransmits deduped)")
                } else {
                    String::new()
                }
            ));
            Ok(())
        })()
    };

    match result {
        Ok(()) => res.ok = true,
        Err(e) => {
            warn!("scenario failed: {e}");
            res.error = Some(e.to_string());
        }
    }
    res
}

/// §12 benchmark run: asymmetric (primary offers, secondary receives) for a
/// fixed wall-clock duration. `load_rps > 0` paces offers to that rate (the
/// common/stress offered loads the acceptance script computes); `0` saturates.
/// The receiver records one-way latency into its histogram and gap counters,
/// and dumps raw samples to `latency_samples` when configured.
fn perf(cfg: &ScenarioCfg, aeron: &Aeron) -> ScenarioResult {
    let mut res = ScenarioResult::default();
    let duration = cfg.duration.unwrap_or_else(|| Duration::from_secs(60));
    let result = if cfg.role == "primary" {
        (|| -> Result<(), Box<dyn Error>> {
            let pub_ = open_publication(aeron, &cfg.pub_ctrl, cfg.stream, cfg.timeout)?;
            add_and_wait(&pub_, cfg.dest(), cfg.timeout)?;
            let started = Instant::now();
            let interval_ns = if cfg.load_rps > 0 {
                // load_rps > 0 here, so this never hits the div-by-zero branch;
                // checked_div silences the lint and 0 (=saturate) is the safe fallback.
                1_000_000_000u64.checked_div(cfg.load_rps).unwrap_or(0)
            } else {
                0
            };
            let mut sent: u64 = 0;
            let mut backpressure: u64 = 0;
            let mut next_slot = 0u64;
            while started.elapsed() < duration {
                if interval_ns > 0 {
                    // Coarse pacing: hold each offer to its slot (ponytail: fine
                    // for hitting a target offered rate; the HDR latency is the
                    // measurement that matters).
                    let elapsed_ns = started.elapsed().as_nanos() as u64;
                    if elapsed_ns < next_slot {
                        sleep(Duration::from_nanos(next_slot - elapsed_ns));
                        continue;
                    }
                    next_slot += interval_ns;
                }
                let msg = build_msg(cfg, sent);
                match pub_.offer(&msg) {
                    Ok(pos) if pos >= 0 => sent += 1,
                    Ok(_) => { /* negative position — unreachable via the typed wrapper */ }
                    Err(e) if e.is_retryable() => {
                        backpressure += 1;
                        sleep(Duration::from_micros(50));
                    }
                    Err(e) => return Err(format!("offer failed: {e}").into()),
                }
            }
            res.sent = sent;
            res.backpressure_ops = backpressure;
            res.push(format!(
                "offered {sent} messages in {}s ({} msg/s, {backpressure} backpressure ops)",
                duration.as_secs(),
                sent as f64 / duration.as_secs_f64()
            ));
            Ok(())
        })()
    } else {
        (|| -> Result<(), Box<dyn Error>> {
            let sub = open_subscription(aeron, cfg.sub(), cfg.stream, cfg.timeout)?;
            let mut recv = Recv::new(cfg.latency_samples.as_deref())?;
            let started = Instant::now();
            // Drain for duration + a short tail so in-flight batches land.
            let drain = duration + Duration::from_secs(5);
            let mut silence = 0u32;
            while started.elapsed() < drain {
                let before = recv.received;
                poll_once(&sub, cfg.expect_byte, &mut recv)?;
                if recv.received == before {
                    silence += 1;
                    if silence > 200 {
                        break; // 200ms without a message — the run is over
                    }
                } else {
                    silence = 0;
                }
                sleep(Duration::from_millis(1));
            }
            recv.flush_samples()?;
            res.received = recv.received;
            res.bytes = recv.bytes;
            res.bad_payload = recv.bad_payload;
            res.gaps = recv.gaps;
            res.latency_p50_ns = recv.histogram.value_at_quantile(0.5);
            res.latency_p99_ns = recv.histogram.value_at_quantile(0.99);
            res.latency_max_ns = recv.histogram.max();
            if recv.bad_payload != 0 {
                return Err(format!("{} payload byte mismatches", recv.bad_payload).into());
            }
            if recv.received == 0 {
                return Err("no messages received".into());
            }
            res.push(format!(
                "received {} messages ({} msg/s, {} gaps, p50={}ns p99={}ns max={}ns)",
                recv.received,
                recv.received as f64 / duration.as_secs_f64(),
                res.gaps,
                res.latency_p50_ns,
                res.latency_p99_ns,
                res.latency_max_ns
            ));
            Ok(())
        })()
    };

    match result {
        Ok(()) => res.ok = true,
        Err(e) => {
            warn!("scenario failed: {e}");
            res.error = Some(e.to_string());
        }
    }
    res
}
