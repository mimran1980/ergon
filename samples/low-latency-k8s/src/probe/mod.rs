//! `ll-probe`'s pieces: `/proc` parsers, the expected placement of every
//! thread, the PASS/FAIL verdicts and the `ll-verify` output format.
//!
//! Everything here is a pure function over text, so it is tested against
//! fixtures on any system; the binary (`src/bin/ll-probe.rs`) reads the live
//! files.

pub mod expect;
pub mod format;
pub mod interrupts;
pub mod proc;
pub mod verdict;
pub mod watch;
