//! Shared helpers for `ergon` benchmarks.

/// Baseline binary fixture — Java-generated Car message (schema v0, template 1).
pub const BASELINE: &[u8] = include_bytes!("fixtures/car_example_baseline_data.sbe");
