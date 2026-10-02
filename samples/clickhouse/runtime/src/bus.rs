//! The application's Aeron client and its identity on it.
//!
//! [`Bus::connect`] once per process. [`Bus::publication`],
//! [`Bus::subscription`] and [`Bus::persistent`] open feeds on it, and
//! [`Persist`](crate::persist::Persist) records through it. Every publication
//! made from one bus shares:
//!
//! * the application's [`Source`]: its id stamped into every frame, and its
//!   `Source` message, so each recording names who published it;
//! * the drop counters ([`Bus::drops`], the `persist_dropped` metric);
//! * [`Bus::shutdown`], which closes all of them at once.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronBufferClaim, AeronContext, AeronErrorType, AeronOfferError, AeronPublication,
    IntoCString,
};

use crate::source::Source;
use crate::{Error, Settings};

/// How many times a term rotation (`AdminAction`) is retried before the
/// record is dropped. Aeron asks for an immediate retry; this keeps a stuck
/// rotation off the recording thread.
const ADMIN_ACTION_RETRIES: u32 = 8;

/// The application's Aeron client. Cheap to clone.
#[derive(Clone)]
pub struct Bus {
    inner: Arc<Inner>,
}

struct Inner {
    aeron: Aeron,
    /// Stamped into every frame's reserved value.
    source: Source,
    source_message: Vec<u8>,
    /// Bumped every 5 s (by [`Persist`](crate::persist::Persist)'s config
    /// watcher): each publication sends its `Source` message again.
    heartbeat: AtomicU64,
    /// [`Bus::shutdown`] has begun: nothing more is published.
    closed: AtomicBool,
    /// Every publication made, for [`Bus::shutdown`] to close.
    publications: Mutex<Vec<AeronPublication>>,
    not_connected: AtomicU64,
    back_pressure: AtomicU64,
    too_large: AtomicU64,
    other: AtomicU64,
}

/// Why a record was not published. [`Drops::other`] is a mis-sized encode, a
/// failed commit, a second thread, or a term rotation that would not finish.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drops {
    /// No subscriber was recording the stream.
    pub not_connected: u64,
    /// The term buffer was full.
    pub back_pressure: u64,
    /// Longer than one `try_claim` can hold.
    pub too_large: u64,
    /// Anything else that was not published.
    pub other: u64,
}

impl Drops {
    /// Every dropped record.
    #[must_use]
    pub fn total(self) -> u64 {
        self.not_connected + self.back_pressure + self.too_large + self.other
    }
}

/// Which counter [`Bus::count`] increments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DropKind {
    NotConnected,
    BackPressure,
    TooLarge,
    Other,
    /// After [`Bus::shutdown`]: not a drop.
    Closed,
}

impl Bus {
    /// Connect to the media driver (`settings.aeron_dir`) as `settings.app`.
    ///
    /// # Errors
    ///
    /// The media driver is unreachable, or the `Source` message could not
    /// be encoded.
    pub fn connect(settings: &Settings) -> Result<Self, Error> {
        let aeron = client(settings).map_err(|e| Error::Aeron(e.to_string()))?;
        let mut source = Source::new(&settings.host, &settings.pod, &settings.app);
        source.client = aeron.client_id();
        let source_message = source.message()?;
        Ok(Self {
            inner: Arc::new(Inner {
                aeron,
                source,
                source_message,
                heartbeat: AtomicU64::new(0),
                closed: AtomicBool::new(false),
                publications: Mutex::new(Vec::new()),
                not_connected: AtomicU64::new(0),
                back_pressure: AtomicU64::new(0),
                too_large: AtomicU64::new(0),
                other: AtomicU64::new(0),
            }),
        })
    }

    /// Add a publication, closed by [`Bus::shutdown`].
    pub(crate) fn add_publication(
        &self,
        channel: &str,
        stream_id: i32,
    ) -> Result<AeronPublication, Error> {
        let publication = self
            .inner
            .aeron
            .async_add_publication(&channel.into_c_string(), stream_id)
            .and_then(|p| p.poll_blocking(Duration::from_secs(10)))
            .map_err(|e| Error::Aeron(format!("{channel}: {e}")))?;
        self.inner
            .publications
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(publication.clone());
        Ok(publication)
    }

    pub(crate) fn aeron(&self) -> &Aeron {
        &self.inner.aeron
    }

    pub(crate) fn source(&self) -> &Source {
        &self.inner.source
    }

    pub(crate) fn source_message(&self) -> &[u8] {
        &self.inner.source_message
    }

    /// The current heartbeat: a publication that last sent its `Source`
    /// message at another sends it again.
    #[inline]
    pub(crate) fn heartbeat(&self) -> u64 {
        self.inner.heartbeat.load(Ordering::Relaxed)
    }

    pub(crate) fn beat(&self) {
        self.inner.heartbeat.fetch_add(1, Ordering::Relaxed);
    }

    /// Close every publication now, so the media driver drops them at once
    /// and subscribers turn to the next publisher of each feed within
    /// seconds, rather than after this client's liveness timeout. Call it on
    /// SIGTERM, before exiting. Records made from now on are not published.
    ///
    /// It returns once the client has handed each close to the driver (at
    /// most a second): a close is asynchronous, and one lost to an exit
    /// leaves the publication open until the client times out. A new
    /// process on the same node would then join that publication, in the
    /// old session, and subscribers would never see it restart.
    pub fn shutdown(&self) {
        let inner = &self.inner;
        if inner.closed.swap(true, Ordering::Relaxed) {
            return;
        }
        // Let records already claiming finish: each takes well under a
        // microsecond.
        std::thread::sleep(Duration::from_millis(50));
        let publications = std::mem::take(
            &mut *inner
                .publications
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        let done = Arc::new(AtomicU64::new(0));
        let handler = {
            let done = Arc::clone(&done);
            rusteron_archive::Handler::new(move || {
                done.fetch_add(1, Ordering::Relaxed);
            })
        };
        let closing = publications
            .into_iter()
            .map(|p| p.close_with_handler(Some(&handler)))
            .filter(Result::is_ok)
            .count() as u64;
        let deadline = Instant::now() + Duration::from_secs(1);
        while done.load(Ordering::Relaxed) < closing && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        if done.load(Ordering::Relaxed) < closing {
            log::warn!("shutdown: the driver took more than a second to close the publications");
            // The client still holds it and may call it yet: never free it.
            std::mem::forget(handler);
        }
    }

    /// Claim `len` bytes of `publication`, stamped with this application's
    /// source id.
    #[inline]
    pub(crate) fn try_claim(
        &self,
        publication: &AeronPublication,
        len: usize,
    ) -> Result<Claim, DropKind> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(DropKind::Closed);
        }
        let claim = AeronBufferClaim::new_zeroed_on_stack();
        retry_admin(|| publication.try_claim(len, &claim)).map_err(|err| classify(&err))?;
        claim.frame_header_mut().reserved_value = self.inner.source.id as i64;
        Ok(Claim { claim, done: false })
    }

    pub(crate) fn count(&self, kind: DropKind) {
        let inner = &self.inner;
        let counter = match kind {
            DropKind::NotConnected => &inner.not_connected,
            DropKind::BackPressure => &inner.back_pressure,
            DropKind::TooLarge => &inner.too_large,
            DropKind::Other => &inner.other,
            DropKind::Closed => return,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    #[cold]
    pub(crate) fn drop_one(&self) {
        self.count(DropKind::Other);
    }

    /// Records dropped so far on every publication of this bus, by reason.
    /// Also logged, once a second while the total grows.
    #[must_use]
    pub fn drops(&self) -> Drops {
        let inner = &self.inner;
        Drops {
            not_connected: inner.not_connected.load(Ordering::Relaxed),
            back_pressure: inner.back_pressure.load(Ordering::Relaxed),
            too_large: inner.too_large.load(Ordering::Relaxed),
            other: inner.other.load(Ordering::Relaxed),
        }
    }

    /// [`Drops::total`] of [`Bus::drops`].
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.drops().total()
    }
}

impl std::fmt::Debug for Bus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bus")
            .field("source", &self.inner.source.id)
            .field("dropped", &self.dropped())
            .finish_non_exhaustive()
    }
}

/// A claimed slot of the term buffer: aborted when dropped uncommitted, so
/// an `encode` that fails releases it at once.
pub(crate) struct Claim {
    claim: AeronBufferClaim,
    done: bool,
}

impl Claim {
    #[inline]
    pub(crate) fn data(&mut self) -> &mut [u8] {
        self.claim.data()
    }

    #[inline]
    pub(crate) fn commit(mut self) -> Result<(), rusteron_archive::AeronCError> {
        self.done = true;
        self.claim.commit().map(drop)
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.claim.abort();
        }
    }
}

#[inline]
fn retry_admin<T>(
    mut once: impl FnMut() -> Result<T, AeronOfferError>,
) -> Result<T, AeronOfferError> {
    let mut left = ADMIN_ACTION_RETRIES;
    loop {
        match once() {
            Err(AeronOfferError::AdminAction) if left > 0 => {
                left -= 1;
                std::hint::spin_loop();
            }
            result => return result,
        }
    }
}

fn classify(err: &AeronOfferError) -> DropKind {
    match err {
        AeronOfferError::NotConnected => DropKind::NotConnected,
        AeronOfferError::BackPressured => DropKind::BackPressure,
        // Aeron returns this when the claim is longer than `max_payload`.
        AeronOfferError::Error(inner) if inner.kind() == AeronErrorType::PublicationError => {
            DropKind::TooLarge
        }
        _ => DropKind::Other,
    }
}

fn client(settings: &Settings) -> Result<Aeron, rusteron_archive::AeronCError> {
    let ctx = AeronContext::new()?;
    if let Some(dir) = &settings.aeron_dir {
        ctx.set_dir(&dir.as_str().into_c_string())?;
    }
    // The driver's counters name their client by this: see `aeron_counters`.
    if !settings.app.is_empty() {
        ctx.set_client_name(&settings.app.as_str().into_c_string())?;
    }
    let aeron = Aeron::new(&ctx)?;
    aeron.start()?;
    Ok(aeron)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_action_is_retried_a_handful_of_times() {
        let mut calls = 0;
        let err: Result<(), _> = retry_admin(|| {
            calls += 1;
            Err(AeronOfferError::AdminAction)
        });
        assert!(matches!(err, Err(AeronOfferError::AdminAction)));
        assert_eq!(calls, ADMIN_ACTION_RETRIES + 1);

        calls = 0;
        let ok = retry_admin(|| {
            calls += 1;
            if calls < 3 {
                Err(AeronOfferError::AdminAction)
            } else {
                Ok(7)
            }
        });
        assert_eq!(ok, Ok(7));
        assert_eq!(calls, 3);

        let err = retry_admin(|| Err::<(), _>(AeronOfferError::BackPressured));
        assert!(matches!(err, Err(AeronOfferError::BackPressured)));
        assert_eq!(
            classify(&AeronOfferError::NotConnected),
            DropKind::NotConnected
        );
        assert_eq!(
            classify(&AeronOfferError::BackPressured),
            DropKind::BackPressure
        );
        assert_eq!(
            classify(&AeronOfferError::Error(
                AeronErrorType::PublicationError.into()
            )),
            DropKind::TooLarge
        );
        assert_eq!(classify(&AeronOfferError::Closed), DropKind::Other);
    }
}
