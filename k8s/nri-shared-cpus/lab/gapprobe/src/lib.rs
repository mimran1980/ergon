//! The ergon-shared-cpus lab's gap probe: a loop that spins reading the CPU's
//! counter, a histogram of every iteration, a log of every gap at or above a
//! threshold, a sampler of `/proc/interrupts` and `/proc/softirqs`, and the
//! report that names what overlapped each run's max.

pub mod clock;
pub mod irq;
pub mod report;
pub mod stats;
