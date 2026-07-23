# 06 — Propagate keep-alive failures and enforce session isolation

**What to build:** Make polling report session-health failures and prevent events belonging to another Cluster session from reaching listeners or mutating the connected client.

**Blocked by:** 05 — Surface malformed egress and controlled-polling failures.

**Status:** ready-for-agent

- [ ] A due keep-alive failure is returned by both regular and controlled polling rather than silently discarded.
- [ ] The last-successful keep-alive time advances only after a successful publication.
- [ ] Publication status remains distinguishable as backpressure, administrative action, closure, disconnection, or other typed failure.
- [ ] Application messages for a different session are ignored without invoking the listener.
- [ ] Every lifecycle, challenge, administrative, and leader event carrying a session identifier is checked before dispatch or state mutation.
- [ ] Foreign-session leader events cannot trigger reconnect behavior.
- [ ] Focused tests cover successful keep-alive, each failure class, timer behavior, and foreign-session events.
- [ ] Java-backed polling still maintains a healthy session across the configured keep-alive interval.
