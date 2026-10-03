//! An exclusive publication shared by handle but claimed by one thread.
#![allow(unsafe_code)]

use std::thread::ThreadId;

use rusteron_archive::AeronExclusivePublication;

thread_local! {
    static THREAD: ThreadId = std::thread::current().id();
}

/// The calling thread's id, cached: one thread-local read.
#[inline]
pub fn current() -> ThreadId {
    THREAD.with(|id| *id)
}

/// An exclusive publication that only `owner` may claim on.
pub struct OwnedPublication {
    publication: AeronExclusivePublication,
    owner: ThreadId,
}

// SAFETY: the publication is reachable only through `claimable`, which returns
// it on the owner thread alone, so claims and commits never race. Other
// threads may call `is_connected`, which reads the C publication's atomic
// connection flag and mutates nothing.
unsafe impl Sync for OwnedPublication {}

impl OwnedPublication {
    /// Owned by the calling thread.
    pub fn new(publication: AeronExclusivePublication) -> Self {
        Self {
            publication,
            owner: current(),
        }
    }

    /// The calling thread owns it.
    #[inline]
    pub fn is_owner(&self) -> bool {
        current() == self.owner
    }

    /// The publication, on the owner thread only.
    #[inline]
    pub fn claimable(&self) -> Option<&AeronExclusivePublication> {
        self.is_owner().then_some(&self.publication)
    }

    /// A subscriber or the archive is taking it.
    pub fn is_connected(&self) -> bool {
        self.publication.is_connected()
    }
}
