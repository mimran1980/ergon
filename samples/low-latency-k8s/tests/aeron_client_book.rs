//! The Aeron client calls `book/src/transports/aeron-client.md` includes,
//! compiled and run here against an embedded media driver: connect to a
//! driver by its directory, add a publication and a subscription, wait for
//! both to connect, send with `offer` and with `try_claim`, poll with a
//! handler, then close in order.
//!
//! In production the driver is another process (a sidecar or a node
//! `DaemonSet`) and the directory comes from `AERON_DIR`; here
//! `EmbeddedDriver` stands in for it. Any OS.

use std::ffi::CString;
use std::time::{Duration, Instant};

use rusteron_client::{
    Aeron, AeronContext, AeronOfferError, AeronPublication, AeronSubscription, Handlers,
};
use rusteron_media_driver::testing::{EmbeddedDriver, find_unused_udp_port};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

// ANCHOR: connect
/// Connects to the media driver whose directory is `aeron_dir` (the value
/// of `AERON_DIR`; with no `set_dir` the client reads that variable itself).
/// Blocks until the driver answers: call it before the loop, never in it.
fn connect(aeron_dir: &str) -> R<Aeron> {
    let context = AeronContext::new()?;
    context.set_dir(&CString::new(aeron_dir)?)?;
    // Errors raised on the client conductor thread, not returned by a call.
    context.set_error_handler(Some(|code: i32, msg: &str| {
        eprintln!("aeron error {code}: {msg}");
    }))?;
    let aeron = Aeron::new(&context)?;
    // Starts the client conductor thread (`aeron-client`). It inherits this
    // thread's CPU mask: start it before the loop thread pins itself.
    aeron.start()?;
    Ok(aeron)
}
// ANCHOR_END: connect

// ANCHOR: add
/// Adds a publication and a subscription. Each `async_add_*` asks the
/// driver; `poll_blocking` waits for its answer. Set-up only: in a running
/// loop, call the poller's non-blocking `poll()` each turn instead.
fn add(aeron: &Aeron, channel: &str, stream: i32) -> R<(AeronPublication, AeronSubscription)> {
    let channel = CString::new(channel)?;
    let publication = aeron
        .async_add_publication(&channel, stream)?
        .poll_blocking(Duration::from_secs(5))?;
    let subscription = aeron
        .async_add_subscription(&channel, stream, Handlers::NONE, Handlers::NONE)?
        .poll_blocking(Duration::from_secs(5))?;
    Ok((publication, subscription))
}
// ANCHOR_END: add

// ANCHOR: wait_connected
/// Waits until a subscriber's image reaches the publication (`offer` fails
/// with `NotConnected` until then) and the subscription has an image.
fn wait_connected(
    publication: &AeronPublication,
    subscription: &AeronSubscription,
    timeout: Duration,
) -> R {
    let deadline = Instant::now() + timeout;
    while !(publication.is_connected() && subscription.is_connected()) {
        if Instant::now() > deadline {
            return Err("no peer within the timeout".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
// ANCHOR_END: wait_connected

// ANCHOR: offer
/// What one send attempt did. A busy loop never waits for a retryable
/// error: it keeps the message and tries again on its next turn.
enum Sent {
    /// In the log at this position.
    Done(i64),
    /// Back-pressure, an admin action, or no subscriber yet: try again.
    Retry(AeronOfferError),
}

/// `offer` copies `msg` into the publication's log.
fn offer(publication: &AeronPublication, msg: &[u8]) -> R<Sent> {
    match publication.offer(msg) {
        Ok(position) => Ok(Sent::Done(position)),
        Err(e) if e.is_retryable() => Ok(Sent::Retry(e)),
        // Closed, or past the log's maximum position: the publication is gone.
        Err(e) => Err(e.into()),
    }
}
// ANCHOR_END: offer

// ANCHOR: try_claim
/// `try_claim_owned` reserves `len` bytes in the log; write the message
/// there and commit, with no copy. A claim dropped without `commit` is
/// aborted. Commit or drop it before the publication closes.
fn send_claimed(publication: &AeronPublication, seq: u64) -> R<Sent> {
    match publication.try_claim_owned(8) {
        Ok(mut claim) => {
            claim.data().copy_from_slice(&seq.to_le_bytes());
            Ok(Sent::Done(claim.commit()?))
        }
        Err(e) if e.is_retryable() => Ok(Sent::Retry(e)),
        Err(e) => Err(e.into()),
    }
}
// ANCHOR_END: try_claim

// ANCHOR: poll
/// Up to this many fragments a poll: bounds one turn of the loop.
const FRAGMENT_LIMIT: usize = 10;

/// Delivers up to `FRAGMENT_LIMIT` fragments to the closure and returns how
/// many. A fragment is a slice of the log, valid during the call only. A
/// message larger than the MTU arrives in fragments: wrap the handler in
/// an `AeronFragmentAssembler`, or keep messages within one MTU.
fn poll(subscription: &AeronSubscription, got: &mut Vec<u64>) -> R<i32> {
    Ok(subscription.poll_fn(
        |fragment, _header| {
            if let Some(seq) = fragment.get(..8).and_then(|b| b.try_into().ok()) {
                got.push(u64::from_le_bytes(seq));
            }
        },
        FRAGMENT_LIMIT,
    )?)
}
// ANCHOR_END: poll

// ANCHOR: close
/// Closes in order: claims and image handles first (here none are held),
/// then the publication and the subscription, then the client. The media
/// driver outlives its clients.
fn close(aeron: Aeron, publication: AeronPublication, subscription: AeronSubscription) -> R {
    publication.close()?;
    subscription.close()?;
    // Deferred until every handle from this client is gone.
    aeron.close()?;
    Ok(())
}
// ANCHOR_END: close

/// The whole recipe on `channel`: three messages by `offer`, three by
/// `try_claim`, each received once and in order.
fn round_trip(driver: &EmbeddedDriver, channel: &str, stream: i32) -> R {
    let aeron = connect(driver.dir())?;
    let (publication, subscription) = add(&aeron, channel, stream)?;
    wait_connected(&publication, &subscription, Duration::from_secs(10))?;
    let mut sent = 0_u64;
    let mut got = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while got.len() < 6 {
        if Instant::now() > deadline {
            return Err(format!("received {got:?} of 6").into());
        }
        if sent < 6 {
            let attempt = if sent < 3 {
                offer(&publication, &sent.to_le_bytes())?
            } else {
                send_claimed(&publication, sent)?
            };
            match attempt {
                Sent::Done(position) => {
                    assert!(position > 0);
                    sent += 1;
                }
                Sent::Retry(AeronOfferError::Closed) => return Err("closed".into()),
                Sent::Retry(_) => {}
            }
        }
        poll(&subscription, &mut got)?;
    }
    assert_eq!(got, (0..6).collect::<Vec<_>>());
    close(aeron, publication, subscription)
}

#[test]
fn ipc_offer_claim_poll_and_close() -> R {
    let driver = EmbeddedDriver::launch()?;
    round_trip(&driver, "aeron:ipc", 2001)
}

#[test]
fn udp_offer_claim_poll_and_close() -> R {
    let driver = EmbeddedDriver::launch()?;
    let port = find_unused_udp_port(40_200).ok_or("no free UDP port")?;
    round_trip(
        &driver,
        &format!("aeron:udp?endpoint=127.0.0.1:{port}"),
        2002,
    )
}

#[test]
fn offer_before_a_subscriber_is_retryable() -> R {
    let driver = EmbeddedDriver::launch()?;
    let aeron = connect(driver.dir())?;
    let publication = aeron
        .async_add_publication(c"aeron:ipc", 2003)?
        .poll_blocking(Duration::from_secs(5))?;
    match offer(&publication, b"early")? {
        Sent::Retry(e) => assert!(matches!(e, AeronOfferError::NotConnected), "{e:?}"),
        Sent::Done(_) => return Err("offered with no subscriber".into()),
    }
    publication.close()?;
    aeron.close()?;
    Ok(())
}
