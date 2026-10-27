//! The ergon-shared-cpus lab's gap probe: a loop that spins reading the CPU's
//! counter, a histogram of every iteration, a log of every gap at or above a
//! threshold, a sampler of `/proc/interrupts` and `/proc/softirqs`, the
//! report that names what overlapped each run's max, the kernel-trace report
//! that names what ran on the probe's CPU, and the shared-CPU scaling report.

pub mod clock;
pub mod gaplog;
pub mod irq;
pub mod procfs;
pub mod report;
pub mod scale;
pub mod stats;
pub mod trace;
