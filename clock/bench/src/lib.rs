//! Benchmarks of ergon-clock against quanta, minstant, std and the
//! kernel's clocks, shared by the `clockbench` binary and the Criterion
//! bench (`benches/read.rs`): one definition of every row ([`rows`]), the
//! measurements ([`measure`]), the system calls they need ([`sys`]) and
//! quanta's clock as the comparison reads it ([`quanta`]); and
//! `clockbench --discipline`, the discipline of the counter to the wall
//! clock run live ([`discipline`]).

pub mod discipline;
pub mod measure;
pub mod quanta;
pub mod rows;
pub mod sys;

/// Held by a test that counts page faults and by a test that makes them
/// (a live run's threads and buffers), so neither runs beside the other:
/// off Linux the count is the whole process's (`sys::minor_faults`), so a
/// live run in parallel shows as faults in a timed pass it never touched.
#[cfg(test)]
pub(crate) fn faults_alone() -> std::sync::MutexGuard<'static, ()> {
    static FAULTS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    FAULTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
