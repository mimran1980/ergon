//! AWS `ClockBound` 2.0's shared memory segment: the daemon's bound on
//! `CLOCK_REALTIME`'s error, read from its file (no `mmap`).
//!
//! The layout is `clock-bound-shm` 2.0.3's (`shm_header.rs`, `lib.rs`), at
//! `/var/run/clockbound/shm0` (`clock-bound-client` 2.0.3's
//! `CLOCKBOUND_SHM_DEFAULT_PATH`), native-endian, on 64-bit Linux:
//!
//! - header, 16 bytes: magic `[0x414D_5A4E, 0x4342_0200]` (two `u32`s),
//!   segment size (`u32`), version (`u16`, 2), generation (`u16`: 0 until
//!   written, odd while the writer is mid-update);
//! - `ClockErrorBound` (`repr(C)`), 64 bytes: `as_of` and `void_after`
//!   (`timespec`s of `CLOCK_MONOTONIC_COARSE`), `bound_nsec` (`i64`),
//!   `disruption_marker` (`u64`), `max_drift_ppb` (`u32`), `clock_status`
//!   (`i32`: 0 unknown, 1 synchronised, 2 free-running, 3 disrupted), a
//!   `bool` and padding.
//!
//! The bound now is `bound_nsec + max_drift_ppb * (now - as_of)`, and its
//! status follows the reader's rules (`ClockErrorBound::compute_bound_at`):
//! a synchronised or free-running status stands for 5 s after `as_of`, is
//! free-running until `void_after`, and unknown after. `ClockBound` 3.0
//! (which disciplines the clock itself) is another layout, not read here.
//!
//! Reading allocates nothing: the segment is read into a buffer on the
//! stack, and every error is a plain value.

use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

/// Where `ClockBound` 2.0 writes its segment.
pub const DEFAULT_PATH: &str = "/var/run/clockbound/shm0";
/// The segment's magic number.
const MAGIC: [u32; 2] = [0x414D_5A4E, 0x4342_0200];
/// The layout version read.
const VERSION: u16 = 2;
/// The header and the bound.
const SEGMENT_LEN: usize = 80;
/// How long after `as_of` the daemon's status stands (`CLOCKBOUND_RESTART_GRACE_PERIOD`).
const GRACE_NS: u64 = 5_000_000_000;
/// `ClockErrorBound::compute_bound_at`'s allowance for `as_of` read a
/// little after `now` (`CLOCK_MONOTONIC_COARSE`'s resolution).
const CAUSALITY_BLUR_NS: u64 = 1_000;

/// `ClockBound`'s clock status (`ClockStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockBoundStatus {
    /// Unknown: the bound is not to be trusted.
    Unknown,
    /// Kept accurate by the synchronisation daemon.
    Synchronized,
    /// Free-running since the daemon's last update: the bound still holds.
    FreeRunning,
    /// The clock was disrupted (a VM migrated, say): no bound.
    Disrupted,
}

impl ClockBoundStatus {
    /// Its name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Synchronized => "synchronized",
            Self::FreeRunning => "free-running",
            Self::Disrupted => "disrupted",
        }
    }

    /// Whether `ClockBound` says its bound can be trusted.
    #[must_use]
    pub const fn trusted(self) -> bool {
        matches!(self, Self::Synchronized | Self::FreeRunning)
    }
}

/// `ClockBound`'s bound, as of a read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockBoundReading {
    /// The bound on `CLOCK_REALTIME`'s error now, ns.
    pub bound_ns: u64,
    /// Its status now (after the grace period and `void_after`).
    pub status: ClockBoundStatus,
    /// How long ago the daemon wrote it, ns.
    pub age_ns: u64,
    /// The drift rate it assumes between updates, ppb.
    pub max_drift_ppb: u32,
}

/// Why `ClockBound`'s segment could not be used. A plain value: making one
/// allocates nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockBoundError {
    /// The file could not be opened or read.
    Io(io::ErrorKind),
    /// The file holds fewer bytes than a segment (this many).
    Short(usize),
    /// Two reads never agreed on a settled copy, in four tries.
    NeverSettled,
    /// The segment did not parse: what was wrong.
    Malformed(&'static str),
    /// No `CLOCK_MONOTONIC_COARSE` reading to judge it against: a tree
    /// standing for a host, with no
    /// [`clockbound_now_ns`](crate::TimeSyncProbe::clockbound_now_ns) given.
    NoClock,
}

impl fmt::Display for ClockBoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{DEFAULT_PATH}: ")?;
        match self {
            Self::Io(kind) => write!(f, "{kind}"),
            Self::Short(n) => write!(f, "{n} bytes, shorter than a ClockBound 2.0 segment"),
            Self::NeverSettled => f.write_str("never settled in four reads"),
            Self::Malformed(why) => f.write_str(why),
            Self::NoClock => f.write_str(
                "no CLOCK_MONOTONIC_COARSE reading to judge it against (a tree standing for a \
                 host: set TimeSyncProbe::clockbound_now_ns)",
            ),
        }
    }
}

impl std::error::Error for ClockBoundError {}

/// A native-endian integer of `N` bytes at `at` (the caller checked the
/// length).
fn ne<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    let mut b = [0_u8; N];
    if let Some(s) = bytes.get(at..at + N) {
        b.copy_from_slice(s);
    }
    b
}

/// A `timespec` at `at`, ns (negative or out of range: `None`).
fn timespec_ns(bytes: &[u8], at: usize) -> Option<u64> {
    let secs = u64::try_from(i64::from_ne_bytes(ne(bytes, at))).ok()?;
    let nanos = u64::try_from(i64::from_ne_bytes(ne(bytes, at + 8))).ok()?;
    if nanos >= 1_000_000_000 {
        return None;
    }
    secs.checked_mul(1_000_000_000)?.checked_add(nanos)
}

/// The generation in a segment's header (0 where it is too short).
fn generation(segment: &[u8]) -> u16 {
    if segment.len() < 16 {
        return 0;
    }
    u16::from_ne_bytes(ne(segment, 14))
}

/// The bound in `segment` (one consistent copy of the file), at
/// `now_coarse_ns` of `CLOCK_MONOTONIC_COARSE`.
///
/// # Errors
/// What was wrong: too short, not `ClockBound`'s magic, another version, not
/// yet written or mid-update (generation 0 or odd), a segment size, time,
/// status or drift rate out of range, or `as_of` after now.
pub fn parse(segment: &[u8], now_coarse_ns: u64) -> Result<ClockBoundReading, &'static str> {
    if segment.len() < SEGMENT_LEN {
        return Err("shorter than a ClockBound 2.0 segment");
    }
    let magic = [
        u32::from_ne_bytes(ne(segment, 0)),
        u32::from_ne_bytes(ne(segment, 4)),
    ];
    if magic != MAGIC {
        return Err("not ClockBound's magic number");
    }
    let segsize = u32::from_ne_bytes(ne(segment, 8));
    if usize::try_from(segsize).map_or(true, |s| s < SEGMENT_LEN) {
        return Err("segment size too small");
    }
    if u16::from_ne_bytes(ne(segment, 12)) != VERSION {
        return Err("not layout version 2");
    }
    let generation = generation(segment);
    if generation == 0 {
        return Err("not yet written (generation 0)");
    }
    if generation % 2 == 1 {
        return Err("mid-update (odd generation)");
    }
    let as_of = timespec_ns(segment, 16).ok_or("as_of out of range")?;
    let void_after = timespec_ns(segment, 32).ok_or("void_after out of range")?;
    let bound = u64::try_from(i64::from_ne_bytes(ne(segment, 48))).map_err(|_| "negative bound")?;
    let max_drift_ppb = u32::from_ne_bytes(ne(segment, 64));
    if max_drift_ppb >= 1_000_000_000 {
        return Err("drift rate of 100% or more");
    }
    let status = match i32::from_ne_bytes(ne(segment, 68)) {
        0 => ClockBoundStatus::Unknown,
        1 => ClockBoundStatus::Synchronized,
        2 => ClockBoundStatus::FreeRunning,
        3 => ClockBoundStatus::Disrupted,
        _ => return Err("unknown clock status"),
    };
    let age_ns = if now_coarse_ns >= as_of {
        now_coarse_ns - as_of
    } else if as_of - now_coarse_ns <= CAUSALITY_BLUR_NS {
        0
    } else {
        return Err("as_of after now");
    };
    let status = match status {
        ClockBoundStatus::Synchronized | ClockBoundStatus::FreeRunning => {
            if age_ns < GRACE_NS {
                status
            } else if now_coarse_ns < void_after {
                ClockBoundStatus::FreeRunning
            } else {
                ClockBoundStatus::Unknown
            }
        }
        other => other,
    };
    let growth = u64::try_from(u128::from(age_ns) * u128::from(max_drift_ppb) / 1_000_000_000)
        .unwrap_or(u64::MAX);
    Ok(ClockBoundReading {
        bound_ns: bound.saturating_add(growth),
        status,
        age_ns,
        max_drift_ppb,
    })
}

/// The segment at `path`: opened once (so a daemon started or restarted
/// since the last read is seen), then read whole from its start into a
/// buffer on the stack until two reads agree on an even generation (the
/// writer updates it in place, about once a second), at most four tries.
/// `Ok(None)` where there is no such file. Allocates nothing for a path
/// under a few hundred bytes (std converts it on the stack).
///
/// # Errors
/// The file could not be read, is short, or never held a settled copy.
pub fn read(path: &Path) -> Result<Option<[u8; SEGMENT_LEN]>, ClockBoundError> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ClockBoundError::Io(e.kind())),
    };
    let read_once = || -> Result<[u8; SEGMENT_LEN], ClockBoundError> {
        let mut buf = [0_u8; SEGMENT_LEN];
        let filled = crate::quota::read_from_start(&file, &mut buf)
            .map_err(|e| ClockBoundError::Io(e.kind()))?;
        if filled < SEGMENT_LEN {
            return Err(ClockBoundError::Short(filled));
        }
        Ok(buf)
    };
    for _ in 0..4 {
        let first = read_once()?;
        let second = read_once()?;
        if first == second && generation(&first).is_multiple_of(2) {
            return Ok(Some(first));
        }
    }
    Err(ClockBoundError::NeverSettled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(
        generation: u16,
        as_of: u64,
        void_after: u64,
        bound: i64,
        drift: u32,
        status: i32,
    ) -> [u8; SEGMENT_LEN] {
        let mut s = [0_u8; SEGMENT_LEN];
        s[0..4].copy_from_slice(&MAGIC[0].to_ne_bytes());
        s[4..8].copy_from_slice(&MAGIC[1].to_ne_bytes());
        s[8..12].copy_from_slice(&80_u32.to_ne_bytes());
        s[12..14].copy_from_slice(&2_u16.to_ne_bytes());
        s[14..16].copy_from_slice(&generation.to_ne_bytes());
        let ts = |s: &mut [u8; SEGMENT_LEN], at: usize, ns: u64| {
            s[at..at + 8]
                .copy_from_slice(&i64::try_from(ns / 1_000_000_000).unwrap_or(0).to_ne_bytes());
            s[at + 8..at + 16]
                .copy_from_slice(&i64::try_from(ns % 1_000_000_000).unwrap_or(0).to_ne_bytes());
        };
        ts(&mut s, 16, as_of);
        ts(&mut s, 32, void_after);
        s[48..56].copy_from_slice(&bound.to_ne_bytes());
        s[64..68].copy_from_slice(&drift.to_ne_bytes());
        s[68..72].copy_from_slice(&status.to_ne_bytes());
        s
    }

    const S: u64 = 1_000_000_000;

    /// The bound grows by the drift rate from `as_of` to now: 50 ppm over
    /// 2 s adds 100 us to a 30 us bound.
    #[test]
    fn the_bound_grows_by_the_drift_rate_since_it_was_written() -> Result<(), &'static str> {
        let s = segment(2, 100 * S, 110 * S, 30_000, 50_000, 1);
        let r = parse(&s, 102 * S)?;
        assert_eq!(r.bound_ns, 130_000);
        assert_eq!(r.age_ns, 2 * S);
        assert_eq!(r.status, ClockBoundStatus::Synchronized);
        Ok(())
    }

    /// A synchronised status stands 5 s, is free-running until `void_after`,
    /// and unknown after; disrupted and unknown stay so.
    #[test]
    fn the_status_ages_as_clockbounds_reader_ages_it() -> Result<(), &'static str> {
        let s = segment(4, 100 * S, 110 * S, 0, 0, 1);
        assert_eq!(parse(&s, 104 * S)?.status, ClockBoundStatus::Synchronized);
        assert_eq!(parse(&s, 106 * S)?.status, ClockBoundStatus::FreeRunning);
        assert_eq!(parse(&s, 111 * S)?.status, ClockBoundStatus::Unknown);
        assert_eq!(
            parse(&segment(4, 100 * S, 110 * S, 0, 0, 3), 101 * S)?.status,
            ClockBoundStatus::Disrupted
        );
        Ok(())
    }

    /// Every malformed segment is refused with its fault.
    #[test]
    fn malformed_segments_are_refused() {
        let good = segment(2, 100 * S, 110 * S, 30_000, 50_000, 1);
        assert!(parse(&good, 100 * S).is_ok());
        let with = |at: usize, v: u8| {
            let mut s = good;
            s[at] = v;
            s
        };
        assert_eq!(
            parse(&good[..79], 100 * S),
            Err("shorter than a ClockBound 2.0 segment")
        );
        assert_eq!(
            parse(&with(0, 0), 100 * S),
            Err("not ClockBound's magic number")
        );
        assert_eq!(parse(&with(12, 3), 100 * S), Err("not layout version 2"));
        assert_eq!(
            parse(&segment(0, 100 * S, 110 * S, 0, 0, 1), 100 * S),
            Err("not yet written (generation 0)")
        );
        assert_eq!(
            parse(&segment(3, 100 * S, 110 * S, 0, 0, 1), 100 * S),
            Err("mid-update (odd generation)")
        );
        assert_eq!(
            parse(&segment(2, 100 * S, 110 * S, -1, 0, 1), 100 * S),
            Err("negative bound")
        );
        assert_eq!(
            parse(&segment(2, 100 * S, 110 * S, 0, 1_000_000_000, 1), 100 * S),
            Err("drift rate of 100% or more")
        );
        assert_eq!(
            parse(&segment(2, 100 * S, 110 * S, 0, 0, 9), 100 * S),
            Err("unknown clock status")
        );
        assert_eq!(parse(&good, 99 * S), Err("as_of after now"));
        let mut small = good;
        small[8..12].copy_from_slice(&16_u32.to_ne_bytes());
        assert_eq!(parse(&small, 100 * S), Err("segment size too small"));
        let mut nanos = good;
        nanos[24..32].copy_from_slice(&1_000_000_000_i64.to_ne_bytes());
        assert_eq!(parse(&nanos, 100 * S), Err("as_of out of range"));
    }

    /// The file: none is `None`, a settled one is its bytes, a short one
    /// says how short.
    #[test]
    fn the_segment_is_read_from_its_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir =
            std::env::temp_dir().join(format!("ergon-tune-clockbound-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("shm0");
        let _ = std::fs::remove_file(&path);
        assert_eq!(read(&path), Ok(None));
        let s = segment(2, 100 * S, 110 * S, 30_000, 50_000, 1);
        std::fs::write(&path, s)?;
        assert_eq!(read(&path), Ok(Some(s)));
        std::fs::write(&path, &s[..40])?;
        assert_eq!(read(&path), Err(ClockBoundError::Short(40)));
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
