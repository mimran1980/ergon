//! A lab client never outlives its run. In an AWS run `rtt xpong`'s echo
//! retried a back-pressured claim for ever, its ping long gone, and took the
//! lab's SIGTERM without stopping: the lab waited on it for 47 minutes. Here
//! the other end is played by this process's own Aeron client (the peer), on
//! an external driver this process runs (rusteron-media-driver, as
//! invoker.rs's mixed test runs one). Against a pong (`rtt xpong`, `load
//! xecho`) the peer sends until the client under test stops reading, which
//! only an echo stuck back-pressured does (the peer never reads the echoes);
//! against a ping it holds its subscription unread until the ping is stuck
//! sending. Then it goes, wholly or in part. The client must exit within 2 s
//! of SIGTERM in every state, and by itself once the other end is gone: its
//! own publication not connected for over a second, or (a ping, between
//! polls) its image of the other end closed.

use rusteron_client::{Aeron, AeronContext, AeronExclusivePublication, AeronSubscription, Handlers};
use rusteron_media_driver::{AeronDriver, AeronDriverContext};
use std::ffi::CString;
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

// rtt's streams, and load xecho's first stream and its echo
const PING_STREAM: i32 = 1002;
const PONG_STREAM: i32 = 1003;
const MIX_STREAM: i32 = 2001;
const MIX_ECHO_STREAM: i32 = 3001;

/// How soon after SIGTERM a client must be gone.
const AFTER_SIGTERM: Duration = Duration::from_secs(2);
/// How soon after the other end vanished a client must be gone by itself:
/// the driver here declares a publication not connected 1 s and an image
/// gone 2 s after the last word from the other end, and a client gives up a
/// publication not connected for 1 s, so a client goes within about 2 s
/// (the driver's timers, not the CPU), and 6 s is ample on a busy machine.
/// No more: an echo given up must end its pong at once, not retry the rest
/// of its poll's pings for a second each (10 s on).
const BY_ITSELF: Duration = Duration::from_secs(6);

/// A free UDP port on 127.0.0.1 (released for the client to bind).
fn endpoint() -> String {
    let port = std::net::UdpSocket::bind("127.0.0.1:0")
        .and_then(|s| s.local_addr())
        .map(|a| a.port())
        .expect("a free UDP port");
    format!("127.0.0.1:{port}")
}

fn channel(endpoint: &str) -> CString {
    CString::new(format!("aeron:udp?endpoint={endpoint}")).unwrap()
}

/// An external media driver in this process, its directory fresh, with a
/// 64 KiB term (so a 32 KiB window: about 500 unread messages back-pressure
/// a publication) and short timeouts, so the other end's going shows at once.
struct Driver {
    dir: String,
    _driver: rusteron_media_driver::EmbeddedMediaDriver,
}

impl Driver {
    fn start(tag: &str) -> Self {
        let dir = format!(
            "{}/aeron-harness-stuck-{tag}-{}",
            std::env::temp_dir().display(),
            std::process::id()
        );
        let _ = std::fs::remove_dir_all(&dir);
        let context = AeronDriverContext::new().unwrap();
        context.set_dir(&CString::new(dir.clone()).unwrap()).unwrap();
        context.set_dir_delete_on_start(true).unwrap();
        context.set_dir_delete_on_shutdown(true).unwrap();
        context.set_term_buffer_length(65536).unwrap();
        context.set_publication_connection_timeout_ns(1_000_000_000).unwrap();
        context.set_image_liveness_timeout_ns(2_000_000_000).unwrap();
        let driver = AeronDriver::launch_embedded_guard(context.clone(), false);
        Self { dir, _driver: driver }
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The other end: a publication and a subscription of this process's own
/// client of the driver, either of which it can close on its own. Dropped,
/// it vanishes (both closed, then the client).
struct Peer {
    publication: Option<AeronExclusivePublication>,
    subscription: Option<AeronSubscription>,
    _aeron: Aeron,
}

impl Peer {
    /// Publishes on `publish` and subscribes to `subscribe`, then waits up to
    /// 30 s for the client under test: its subscription connected to this
    /// publication, and its publication's image in this subscription.
    fn connect(dir: &str, publish: (&str, i32), subscribe: (&str, i32)) -> Self {
        let context = AeronContext::new().unwrap();
        context.set_dir(&CString::new(dir).unwrap()).unwrap();
        let aeron = Aeron::new(&context).unwrap();
        aeron.start().unwrap();
        let publication = aeron
            .async_add_exclusive_publication(&channel(publish.0), publish.1)
            .and_then(|p| p.poll_blocking(Duration::from_secs(5)))
            .expect("the peer's publication");
        let subscription = aeron
            .async_add_subscription(&channel(subscribe.0), subscribe.1, Handlers::NONE, Handlers::NONE)
            .and_then(|s| s.poll_blocking(Duration::from_secs(5)))
            .expect("the peer's subscription");
        let deadline = Instant::now() + Duration::from_secs(30);
        while !(publication.is_connected() && subscription.image_at_index(0).is_some()) {
            assert!(Instant::now() < deadline, "the client under test never connected");
            std::thread::sleep(Duration::from_millis(5));
        }
        Self {
            publication: Some(publication),
            subscription: Some(subscription),
            _aeron: aeron,
        }
    }

    /// Stops publishing: the client's image of it closes once the driver
    /// hears nothing more (its image liveness timeout, 2 s here).
    fn close_publication(&mut self) {
        self.publication = None;
    }

    /// Stops subscribing: the client's publication goes not connected once
    /// the driver hears no status from it (1 s here).
    fn close_subscription(&mut self) {
        self.subscription = None;
    }

    /// Offers 32-byte messages, numbered as rtt's are, until the client under
    /// test stops reading them: back-pressured 500 ms without a break (a
    /// client that reads takes them in microseconds). The messages sent.
    fn flood(&self) -> u64 {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut blocked: Option<Instant> = None;
        let mut sent = 0u64;
        loop {
            assert!(
                Instant::now() < deadline,
                "the client under test kept reading for 20 s: never back-pressured ({sent} sent)"
            );
            let mut message = [0u8; 32];
            message[8..16].copy_from_slice(&sent.to_le_bytes());
            match self.publication.as_ref().expect("publishing").offer(&message) {
                Ok(_) => {
                    sent += 1;
                    blocked = None;
                }
                Err(e) if e.is_retryable() => {
                    if blocked.get_or_insert_with(Instant::now).elapsed() >= Duration::from_millis(500) {
                        return sent;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => panic!("the peer's offer: {e:?}"),
            }
        }
    }

    /// Whether its subscription still has the client's image.
    fn sees_client(&self) -> bool {
        self.subscription
            .as_ref()
            .is_some_and(|s| s.image_at_index(0).is_some())
    }
}

/// A client process, its output collected as it runs.
struct Client {
    what: String,
    child: Child,
    stdout: std::thread::JoinHandle<String>,
    stderr: std::thread::JoinHandle<String>,
}

/// How a client ended, and how long after the moment it was timed from.
struct Ended {
    status: ExitStatus,
    after: Duration,
    stdout: String,
    stderr: String,
}

impl Client {
    /// `bin` with `args` on the external driver at `dir` (`HARNESS_DRIVER`
    /// unset, as the lab's kernel and DPDK modes run), `extra` its additions.
    fn start(bin: &str, args: &[&str], dir: &str, extra: &[(&str, &str)]) -> Self {
        let mut cmd = Command::new(bin);
        cmd.args(args).env("AERON_DIR", dir).env("LABEL", "stuck-test");
        for var in [
            "HARNESS_DRIVER",
            "PUB_CHANNEL_PARAMS",
            "SUB_CHANNEL_PARAMS",
            "LOOP_STALLS_DIR",
            "LOAD_SPIKES",
            "RTT_HISTOGRAM",
            "RTT_LOST_TIMEOUT_US",
            "RTT_SECONDS",
        ] {
            cmd.env_remove(var);
        }
        for (k, v) in extra {
            cmd.env(k, v);
        }
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawning {bin}: {e}"));
        let mut out = child.stdout.take().unwrap();
        let mut err = child.stderr.take().unwrap();
        Self {
            what: format!("{} {}", bin.rsplit('/').next().unwrap(), args.join(" ")),
            child,
            stdout: std::thread::spawn(move || {
                let mut s = String::new();
                let _ = out.read_to_string(&mut s);
                s
            }),
            stderr: std::thread::spawn(move || {
                let mut s = String::new();
                let _ = err.read_to_string(&mut s);
                s
            }),
        }
    }

    /// Whether it is still running.
    fn running(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }

    /// Waits up to `bound` from `from` for it to exit; past that, kills it
    /// (by its PID) and fails, with what it said.
    fn ended_within(mut self, from: Instant, bound: Duration, why: &str) -> Ended {
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            if from.elapsed() > bound {
                let _ = self.child.kill();
                let _ = self.child.wait();
                let stderr = self.stderr.join().unwrap_or_default();
                panic!(
                    "{} still running {:?} after {why} (bound {bound:?}): killed; stderr:\n{stderr}",
                    self.what,
                    from.elapsed()
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let after = from.elapsed();
        let stderr = self.stderr.join().unwrap_or_default();
        // the evidence a run leaves (--nocapture): how long each took to go, and why
        let said = stderr
            .lines()
            .filter(|l| l.contains(" ended: ") || l.contains("went away"))
            .collect::<Vec<_>>()
            .join(" | ");
        eprintln!("{}: ended {after:?} after {why} ({status}): {said}", self.what);
        Ended {
            status,
            after,
            stdout: self.stdout.join().unwrap_or_default(),
            stderr,
        }
    }

    /// SIGTERM, as the lab's `pkill -f '[r]tt xpong'` sends it; then it must
    /// be gone within [`AFTER_SIGTERM`].
    fn terminate(self) -> Ended {
        let pid = self.child.id().to_string();
        let sent = Instant::now();
        let killed = Command::new("kill").args(["-TERM", &pid]).status().expect("kill -TERM");
        assert!(killed.success(), "kill -TERM {pid}");
        self.ended_within(sent, AFTER_SIGTERM, "SIGTERM")
    }
}

const RTT: &str = env!("CARGO_BIN_EXE_rtt");
const LOAD: &str = env!("CARGO_BIN_EXE_load");

/// `rtt xpong` on `driver`, and a ping peer it is connected to: the peer
/// publishes pings to the pong and subscribes to its echoes.
fn pong(driver: &Driver) -> (Client, Peer) {
    let (ping, pong) = (endpoint(), endpoint());
    let client = Client::start(RTT, &["xpong", &ping, &pong, "-"], &driver.dir, &[]);
    let peer = Peer::connect(&driver.dir, (&ping, PING_STREAM), (&pong, PONG_STREAM));
    (client, peer)
}

/// The G2 hang: the pong's echo back-pressured (the ping reads none), the
/// ping vanishes, and the lab's SIGTERM comes. Gone within 2 s.
#[test]
fn a_back_pressured_pong_whose_ping_vanished_exits_within_two_seconds_of_sigterm() {
    let driver = Driver::start("pong-term-gone");
    let (mut pong, peer) = pong(&driver);
    let sent = peer.flood();
    drop(peer);
    assert!(pong.running(), "the pong ended before SIGTERM ({sent} pings sent)");
    let ended = pong.terminate();
    assert!(
        ended.status.success(),
        "the pong after SIGTERM: {:?} after {:?}\n{}",
        ended.status,
        ended.after,
        ended.stderr
    );
    assert!(ended.stderr.contains("rtt: pong ended: SIGTERM"), "{}", ended.stderr);
    assert!(ended.stdout.is_empty(), "{}", ended.stdout);
}

/// SIGTERM while the pong is back-pressured by a ping that is still there
/// (connected, reading nothing): gone within 2 s all the same.
#[test]
fn a_pong_back_pressured_by_a_live_ping_exits_within_two_seconds_of_sigterm() {
    let driver = Driver::start("pong-term-live");
    let (pong, peer) = pong(&driver);
    peer.flood();
    let ended = pong.terminate();
    drop(peer);
    assert!(ended.status.success(), "{:?}\n{}", ended.status, ended.stderr);
    assert!(ended.stderr.contains("rtt: pong ended: SIGTERM"), "{}", ended.stderr);
}

/// SIGTERM before any ping (the pong waiting for one) and while it idles,
/// connected with nothing to echo: gone within 2 s in both.
#[test]
fn a_waiting_or_idle_pong_exits_within_two_seconds_of_sigterm() {
    let driver = Driver::start("pong-term-idle");
    let waiting = Client::start(RTT, &["xpong", &endpoint(), &endpoint(), "-"], &driver.dir, &[]);
    std::thread::sleep(Duration::from_millis(500));
    waiting.terminate();
    let (pong, peer) = pong(&driver);
    std::thread::sleep(Duration::from_millis(200));
    let ended = pong.terminate();
    drop(peer);
    assert!(ended.status.success(), "{:?}\n{}", ended.status, ended.stderr);
}

/// No SIGTERM: a back-pressured pong whose ping vanished ends by itself,
/// saying why.
#[test]
fn a_back_pressured_pong_ends_by_itself_once_its_ping_is_gone() {
    let driver = Driver::start("pong-gone");
    let (mut pong, peer) = pong(&driver);
    peer.flood();
    let gone = Instant::now();
    drop(peer);
    assert!(pong.running(), "the pong ended as its ping went");
    let ended = pong.ended_within(gone, BY_ITSELF, "its ping vanished");
    assert!(ended.status.success(), "{:?}\n{}", ended.status, ended.stderr);
    assert!(
        ended.stderr.contains("rtt: pong ended: its ping's image closed")
            || ended.stderr.contains("rtt: pong ended: its publication not connected"),
        "{}",
        ended.stderr
    );
}

/// The two ways a ping goes, apart. Its subscription alone goes (it still
/// publishes, so the pong's image of it stays): the pong's publication goes
/// not connected, and the pong gives its echo up 1 s on.
#[test]
fn a_back_pressured_pong_ends_once_its_publication_has_been_not_connected_for_a_second() {
    let driver = Driver::start("pong-unread");
    let (pong, mut peer) = pong(&driver);
    peer.flood();
    let gone = Instant::now();
    peer.close_subscription();
    let ended = pong.ended_within(gone, BY_ITSELF, "its echoes' reader went");
    assert!(ended.status.success(), "{:?}\n{}", ended.status, ended.stderr);
    assert!(
        ended
            .stderr
            .contains("rtt: pong ended: its publication not connected for over 1 s, its echoes unread"),
        "{}",
        ended.stderr
    );
    assert!(
        ended.after >= Duration::from_secs(1),
        "given up before its second: {:?}",
        ended.after
    );
    drop(peer);
}

/// `load xecho`'s echo, back-pressured likewise, ends by itself once its
/// reader (xmix's subscription) is gone, its ping still publishing. (It
/// takes SIGTERM's default: it dies of it.)
#[test]
fn a_back_pressured_load_echo_ends_once_its_publication_has_been_not_connected_for_a_second() {
    let driver = Driver::start("xecho-unread");
    let (ping, pong) = (endpoint(), endpoint());
    let echo = Client::start(LOAD, &["xecho", &ping, &pong, "-", "2001"], &driver.dir, &[]);
    let mut peer = Peer::connect(&driver.dir, (&ping, MIX_STREAM), (&pong, MIX_ECHO_STREAM));
    peer.flood();
    let gone = Instant::now();
    peer.close_subscription();
    let ended = echo.ended_within(gone, BY_ITSELF, "its echoes' reader went");
    assert!(ended.status.success(), "{:?}\n{}", ended.status, ended.stderr);
    assert!(
        ended
            .stderr
            .contains("load: echo ended: an echo's publication not connected for over 1 s, its echoes unread"),
        "{}",
        ended.stderr
    );
    drop(peer);
}

/// `rtt xping` stuck back-pressured, and its pong peer: with lost round
/// trips timed out (an unreliable channel's RTT_LOST_TIMEOUT_US, here
/// 0.1 ms) against a pong that echoes nothing and reads nothing, the ping
/// keeps sending until back-pressured, then retries past its one-second
/// budget.
fn stuck_ping(driver: &Driver) -> (Client, Peer) {
    let (ping, pong) = (endpoint(), endpoint());
    let mut client = Client::start(
        RTT,
        &["xping", &ping, &pong, "1000000", "10", "-"],
        &driver.dir,
        &[("RTT_SECONDS", "1"), ("RTT_LOST_TIMEOUT_US", "100")],
    );
    let peer = Peer::connect(&driver.dir, (&pong, PONG_STREAM), (&ping, PING_STREAM));
    // ~1,000 sends fill the receiver's window and the publication's (32 KiB
    // each) at about one each 0.1 ms; its budget is 1.25 s, so a ping still
    // running at 3 s is stuck back-pressured
    std::thread::sleep(Duration::from_secs(3));
    assert!(peer.sees_client(), "the ping's publication went");
    if !client.running() {
        let ended = client.ended_within(Instant::now(), BY_ITSELF, "its budget");
        panic!(
            "the ping ended within its budget, never back-pressured: {:?}\n{}\n{}",
            ended.status, ended.stdout, ended.stderr
        );
    }
    (client, peer)
}

/// The pong vanishes: the ping ends by itself, failing, as it does when a
/// pong goes away.
#[test]
fn a_back_pressured_ping_ends_by_itself_once_its_pong_is_gone() {
    let driver = Driver::start("ping-gone");
    let (client, peer) = stuck_ping(&driver);
    let gone = Instant::now();
    drop(peer);
    let ended = client.ended_within(gone, BY_ITSELF, "its pong vanished");
    assert!(
        !ended.status.success(),
        "the ping succeeded without its pong:\n{}",
        ended.stdout
    );
    assert!(ended.stderr.contains("pong went away"), "{}", ended.stderr);
}

/// The pong stops publishing but still subscribes, unread, so the ping's
/// publication stays connected and back-pressured: the ping ends once its
/// image of the pong closes (it reads that image dry between round trips, so
/// the driver lets it go; a pong stuck inside its poll holds its own open).
#[test]
fn a_back_pressured_ping_ends_once_its_pongs_image_closes() {
    let driver = Driver::start("ping-silent");
    let (client, mut peer) = stuck_ping(&driver);
    let gone = Instant::now();
    peer.close_publication();
    let ended = client.ended_within(gone, BY_ITSELF, "its pong stopped publishing");
    assert!(!ended.status.success(), "{}", ended.stdout);
    assert!(
        ended
            .stderr
            .contains("pong went away: the other side's image closed while the ping was back-pressured"),
        "{}",
        ended.stderr
    );
    assert!(
        peer.sees_client(),
        "the ping's publication went before the pong's image closed"
    );
}
