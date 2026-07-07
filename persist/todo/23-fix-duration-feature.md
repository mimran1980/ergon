# Fix duration feature: std always available, chrono behind feature

**Blocked by:** none
**Severity:** MEDIUM

## Problem

The `duration` feature flag is empty (`duration = []` in Cargo.toml) and
gates the `PersistAs` impl for `std::time::Duration`. This is wrong on
two levels:

1. **`std::time::Duration` should always be available.** It's in std, no
   dependency needed. Gating it behind a feature flag adds friction for zero
   benefit.

2. **The feature name "duration" implies `chrono::Duration` support**, which
   is what users actually expect from an optional feature. `chrono` is
   already an optional dependency.

## Design

1. Remove the `#[cfg(feature = "duration")]` gate from the `std::time::Duration`
   `PersistAs` impl — it's always available.

2. Add a `PersistAs` impl for `chrono::Duration` behind the existing
   `chrono` feature (or a new `duration` feature that depends on `chrono`).

   The `chrono::Duration` maps to ClickHouse `Interval` (same as `std::time::Duration`).

3. The feature flag `duration` can either:
   a. Be removed entirely
   b. Be changed to `duration = ["dep:chrono"]` and gate the `chrono::Duration` impl

   Option (a) is simpler: both `std::time::Duration` and `chrono::Duration`
   (behind `chrono` feature) are available. No separate `duration` feature.

## Acceptance criteria

- [ ] `PersistAs` for `std::time::Duration` is always available (no feature gate)
- [ ] `PersistAs` for `chrono::Duration` is available behind `chrono` feature
- [ ] Feature flag `duration` removed from `Cargo.toml` (or repurposed as alias for chrono)
- [ ] `feature_impls.rs` updated: tests run without `duration` feature
- [ ] No regression in existing tests
