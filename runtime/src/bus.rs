//! The application's Aeron client and its identity on it.
//!
//! [`Bus::connect`] once per process. One owner holds it, the runtime's
//! [`Ctx`](crate::rt::Ctx), or a simulation that records or replays through
//! it ([`SimConfig::bus`](crate::rt::sim::SimConfig::bus)): it is neither
//! `Clone` nor `Send`. The client has no thread of its own. Its conductor
//! runs when the owner's loop calls [`Bus::poll`], as the runtime's duty
//! cycle does, and at each poll of a
//! [`PersistentSubscription`](crate::subscription::PersistentSubscription),
//! whose Aeron counterpart runs it for a client with no conductor thread.
//! Every wait on Aeron drives it while it waits (a publication being added, a
//! subscriber to record persist's stream, a stalled record): a client
//! nothing drives for the client liveness timeout (10 s) is closed by the
//! driver.
//!
//! Feeds and persist's stream are publications made from the bus and owned
//! by what writes them: the runtime's outputs ([`Ctx::publish`]),
//! [`Persist`](crate::persist::Persist), or a
//! [`Publication`](crate::publication::Publication). Each is exclusive, one
//! writer and no CAS on the term tail, and shares the application's
//! [`Source`]: its id in every persist frame, and its `Source` message ahead
//! of each recording's first frame, so a recording names who published it.
//! The feeds' drops are counted in one set, [`Bus::drops`]; persist counts its
//! own. Owners close what they own: at shutdown the runtime closes its outputs
//! and persist's publication at once.
//!
//! The [`bridge`](crate::bridge)'s concurrent publication shares only the
//! identity: what it cannot publish is not counted, and the runtime leaves it
//! open. It closes with its layer, or with the client.
//!
//! [`Ctx::publish`]: crate::rt::Ctx::publish

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronBufferClaim, AeronCError, AeronContext, AeronErrorType, AeronExclusivePublication,
    AeronOfferError, IntoCString,
};

use crate::source::Source;
use crate::{Error, Settings};

/// How many times a term rotation (`AdminAction`) is retried before the
/// record is dropped. Aeron asks for an immediate retry; this keeps a stuck
/// rotation off the recording thread.
const ADMIN_ACTION_RETRIES: u32 = 8;

/// The application's Aeron client, in invoker mode: its conductor runs in the
/// owner's loop.
pub struct Bus {
    aeron: Aeron,
    /// This node's IP: see [`Settings::host_ip`].
    host_ip: String,
    /// See [`Settings::region`].
    region: String,
    /// Stamped into every persist frame's reserved value.
    source: Source,
    source_message: Vec<u8>,
    /// The feeds' drops, shared with what publishes and reports them.
    drops: DropCounts,
}

/// Why a record was not published.
///
/// [`Drops::other`] is a mis-sized encode, a failed commit, or another error
/// Aeron reports (a full log, say). A term rotation that would not finish
/// counts as back pressure.
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
    pub const fn total(self) -> u64 {
        self.not_connected + self.back_pressure + self.too_large + self.other
    }
}

impl std::ops::Add for Drops {
    type Output = Self;

    /// Both counts, reason by reason.
    fn add(self, other: Self) -> Self {
        Self {
            not_connected: self.not_connected + other.not_connected,
            back_pressure: self.back_pressure + other.back_pressure,
            too_large: self.too_large + other.too_large,
            other: self.other + other.other,
        }
    }
}

/// Which counter [`DropCounts::count`] increments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DropKind {
    NotConnected,
    BackPressure,
    TooLarge,
    Other,
    /// Its owner closed the publication: not a drop.
    Closed,
}

/// Drops by reason, one set shared by what counts them and what reports
/// them: a clone counts into the same set.
#[derive(Clone, Debug, Default)]
pub(crate) struct DropCounts(Rc<Cell<Drops>>);

impl DropCounts {
    /// One more drop of `kind`; [`DropKind::Closed`] is none.
    pub(crate) fn count(&self, kind: DropKind) {
        let mut drops = self.0.get();
        match kind {
            DropKind::NotConnected => drops.not_connected += 1,
            DropKind::BackPressure => drops.back_pressure += 1,
            DropKind::TooLarge => drops.too_large += 1,
            DropKind::Other => drops.other += 1,
            DropKind::Closed => return,
        }
        self.0.set(drops);
    }

    /// The drops so far.
    pub(crate) fn get(&self) -> Drops {
        self.0.get()
    }
}

impl Bus {
    /// Connect to the media driver (`settings.aeron_dir`) as `settings.app`,
    /// with the conductor in the caller's loop.
    ///
    /// # Errors
    ///
    /// The media driver is unreachable, or the `Source` message could not
    /// be encoded.
    pub fn connect(settings: &Settings) -> Result<Self, Error> {
        let aeron = client(settings).map_err(|e| Error::Aeron(e.to_string()))?;
        let mut source = settings.sim_start.map_or_else(
            || Source::new(&settings.host, &settings.pod, &settings.app),
            |start| {
                Source::at(
                    &settings.host,
                    &settings.pod,
                    &settings.app,
                    start.0.cast_unsigned(),
                    &settings.run,
                )
            },
        );
        if source.run.is_empty() && std::env::var("JOURNAL").is_ok_and(|v| v == "on") {
            source.run = format!("live-{}", source.id);
        } else if source.run.is_empty() {
            source.run.clone_from(&settings.run);
        }
        source.client = aeron.client_id();
        let source_message = source.message()?;
        Ok(Self {
            aeron,
            host_ip: settings.host_ip.clone(),
            region: settings.region.clone(),
            source,
            source_message,
            drops: DropCounts::default(),
        })
    }

    /// Add an exclusive publication (one writing thread, no CAS on the term
    /// tail), driving the conductor until it is added. Its owner closes it.
    pub(crate) fn add_exclusive_publication(
        &self,
        channel: &str,
        stream_id: i32,
    ) -> Result<AeronExclusivePublication, Error> {
        let adding = self
            .aeron
            .async_add_exclusive_publication(&channel.into_c_string(), stream_id)
            .map_err(|e| Error::Aeron(format!("{channel}: {e}")))?;
        await_added(&self.aeron, channel, || adding.poll())
    }

    /// One wait step of a blocking call; see [`pause`].
    pub(crate) fn pause(&self) {
        pause(&self.aeron);
    }

    /// The client conductor's duty cycle: the owner's loop calls it, between
    /// messages. Returns the work count.
    #[inline]
    #[must_use = "the work count feeds the idle strategy"]
    pub fn poll(&self) -> usize {
        poll(&self.aeron)
    }

    /// This node's IP: the address a directory gives a feed on this node.
    #[must_use]
    pub fn host_ip(&self) -> &str {
        &self.host_ip
    }

    /// Where this application runs: see [`Settings::region`].
    #[must_use]
    pub fn region(&self) -> &str {
        &self.region
    }

    pub(crate) const fn aeron(&self) -> &Aeron {
        &self.aeron
    }

    pub(crate) const fn source(&self) -> &Source {
        &self.source
    }

    pub(crate) fn source_message(&self) -> &[u8] {
        &self.source_message
    }

    /// The feeds' drop counts, for what publishes or reports them.
    pub(crate) const fn drop_counts(&self) -> &DropCounts {
        &self.drops
    }

    pub(crate) fn count(&self, kind: DropKind) {
        self.drops.count(kind);
    }

    /// Frames dropped so far on this bus's feeds, by reason: the runtime's
    /// outputs and every [`Publication`](crate::publication::Publication)
    /// made from it. [`Persist`](crate::persist::Persist) counts its own
    /// records, and the [`bridge`](crate::bridge) counts none;
    /// [`Persist::drops`] adds these to persist's own.
    ///
    /// [`Persist::drops`]: crate::persist::Persist::drops
    #[must_use]
    pub fn drops(&self) -> Drops {
        self.drops.get()
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
            .field("source", &self.source.id)
            .field("dropped", &self.dropped())
            .finish_non_exhaustive()
    }
}

/// Wait up to 10 s for an asynchronous add, driving the conductor meanwhile.
pub(crate) fn await_added<T>(
    aeron: &Aeron,
    channel: &str,
    mut poll: impl FnMut() -> Result<Option<T>, AeronCError>,
) -> Result<T, Error> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match poll() {
            Ok(Some(added)) => return Ok(added),
            Ok(None) if Instant::now() < deadline => pause(aeron),
            Ok(None) => return Err(Error::Aeron(format!("{channel}: not added within 10 s"))),
            Err(e) => return Err(Error::Aeron(format!("{channel}: {e}"))),
        }
    }
}

/// One wait step of a blocking call: the conductor's duty cycle, then the
/// core to whatever else is ready.
pub(crate) fn pause(aeron: &Aeron) {
    let _ = poll(aeron);
    std::thread::yield_now();
}

/// The client conductor's duty cycle. Returns the work count.
#[inline]
pub(crate) fn poll(aeron: &Aeron) -> usize {
    aeron
        .main_do_work()
        .map_or(0, |n| usize::try_from(n).unwrap_or(0))
}

/// Close `publications` now, so the media driver drops them at once and
/// subscribers turn to the next publisher of each within seconds, rather
/// than after this client's liveness timeout. Closing one handle closes
/// every clone of it: a later claim on any is refused as closed. One already
/// closed is passed over.
///
/// The client is in invoker mode, so each close is done before this
/// returns: the remove command is in the driver's queue and the log
/// released, with nothing left for the conductor to finish.
pub(crate) fn close(publications: impl IntoIterator<Item = AeronExclusivePublication>) {
    for publication in publications {
        if !publication.is_closed() {
            let _ = publication.close();
        }
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
    pub(crate) const fn new(claim: AeronBufferClaim) -> Self {
        Self { claim, done: false }
    }

    #[inline]
    pub(crate) fn data(&self) -> &mut [u8] {
        self.claim.data()
    }

    #[inline]
    pub(crate) fn commit(mut self) -> Result<(), AeronCError> {
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

/// Claim `len` bytes of an exclusive `publication`, stamped with
/// `reserved`: no CAS on the term tail, and no check for a close (a claim on
/// a closed publication is refused). A feed stamps its publish time, persist
/// its source id.
#[inline]
pub(crate) fn claim_exclusive(
    publication: &AeronExclusivePublication,
    len: usize,
    reserved: i64,
) -> Result<Claim, DropKind> {
    let claim = AeronBufferClaim::new_zeroed_on_stack();
    retry_admin(|| publication.try_claim(len, &claim)).map_err(|err| classify(&err))?;
    claim.frame_header_mut().reserved_value = reserved;
    Ok(Claim::new(claim))
}

#[inline]
pub(crate) fn retry_admin<T>(
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

pub(crate) fn classify(err: &AeronOfferError) -> DropKind {
    match err {
        AeronOfferError::NotConnected => DropKind::NotConnected,
        AeronOfferError::BackPressured | AeronOfferError::AdminAction => DropKind::BackPressure,
        // Aeron returns this when the claim is longer than `max_payload`.
        AeronOfferError::Error(inner) if inner.kind() == AeronErrorType::PublicationError => {
            DropKind::TooLarge
        }
        _ => DropKind::Other,
    }
}

fn client(settings: &Settings) -> Result<Aeron, AeronCError> {
    let ctx = AeronContext::new()?;
    if let Some(dir) = &settings.aeron_dir {
        ctx.set_dir(&dir.as_str().into_c_string())?;
    }
    // The driver's counters name their client by this: see `aeron_counters`.
    if !settings.app.is_empty() {
        ctx.set_client_name(&settings.app.as_str().into_c_string())?;
    }
    // No conductor thread: the owner's loop runs it (`Bus::poll`), so it
    // never contends for the loop's core.
    ctx.set_use_conductor_agent_invoker(true)?;
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

    #[test]
    fn drop_counts_are_shared_by_their_clones_and_a_close_is_not_one() {
        let counts = DropCounts::default();
        let reader = counts.clone();
        counts.count(DropKind::NotConnected);
        counts.count(DropKind::Other);
        counts.count(DropKind::Closed);
        assert_eq!(
            reader.get(),
            Drops {
                not_connected: 1,
                other: 1,
                ..Drops::default()
            }
        );
        assert_eq!(reader.get().total(), 2);
    }
}
