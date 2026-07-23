# 08 — Make normal Cluster offer allocation-free

**What to build:** Let callers publish through the convenient high-level offer API without allocating and copying a combined session-header-plus-payload buffer, while retaining the explicit zero-copy claim path.

**Blocked by:** 03 — Expand the supported Cluster facade and configuration API.

**Status:** ready-for-agent

- [ ] The normal offer path sends the session header and caller payload through scatter/gather or an equivalent allocation-free interface.
- [ ] The caller payload is not copied into a temporary combined buffer.
- [ ] Header fields remain byte-identical to the reference codec and include the current term and session identifiers.
- [ ] Every Aeron offer status is mapped to the existing typed Cluster result without becoming a generic error.
- [ ] The explicit claim path remains available and preserves commit, abort, payload bounds, and drop behavior.
- [ ] Allocation tests prove zero heap allocations for a warmed successful offer and claim.
- [ ] Equal-work benchmarks cover the normal offer and claim-shaped header path without regressing maintained ratios.
- [ ] A transport-level test proves the receiving side observes the exact application payload.
