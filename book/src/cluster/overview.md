# Cluster client

`ergo-aeron-cluster` is an experimental Rust client for Aeron Cluster. It uses
rusteron for transport and ergo-sbe-generated codecs for Aeron's Cluster
protocol.

> Hobby project. Do not use it as a production substitute for official Aeron
> Cluster client support.

## Scope

{{#include ../../../cluster/README.md:scope}}

## Session lifecycle

<div class="ergon-anim" data-anim="cluster-session-lifecycle" data-title="Cluster client session: connect, keep alive, fail over, close"></div>

Text alternative. The client has one ingress publication to the leader and one
egress subscription. Until the handshake succeeds it is `Closed`.

1. **Connect.** `connect()` offers one `SessionConnectRequest`, which lands
   once, because each request opens a session. If the cluster sends a
   `Challenge`, the client answers with a `ChallengeResponse` under a new
   correlation id. A `SessionEvent(OK)` that echoes the current id gives
   `Connected`. `message_timeout` bounds the handshake.
2. **Keep alive.** `poll` never sends keep-alives. Call `keep_alive_if_due`
   from the same loop. While `Connected`, it sends a 24-byte
   `SessionKeepAlive` every `message_timeout / 4`, clamped to 50–1000 ms.
3. **Leader lost.** When the bound egress image closes, or an offer is refused
   with `NotConnected` or `Closed`, the state becomes `AwaitingNewLeader`.
   `new_leader_timeout` bounds the wait.
4. **New leader.** A `NewLeaderEvent` on egress carries the new term, the
   leader member id and every member's ingress endpoint. The client picks the
   leader's entry, adds an ingress publication there and restarts the clock,
   now bounded by `message_timeout`. Once the publication connects, the client
   adopts the new term and leader, and the state returns to `Connected`.
5. **Close.** `SessionEvent(CLOSED)` gives `PendingClose`, and the same
   `poll` then moves it to `Closed`. An offer refused with
   `MaxPositionExceeded` gives `PendingClose` until the next `poll`.
   `close()` sends a `SessionCloseRequest` while `Connected` and goes
   straight to `Closed`.

Each message carries a 32-byte session header. Failover and close never block
`poll`, but offers fail with `NotConnected` from leader loss until the new
publication connects. The session closes if no `NewLeaderEvent` comes within
`new_leader_timeout`, or its publication does not connect within
`message_timeout`. This book records no failover timings. Endpoints and
field values in the animation are illustrative.

## Features and harness

{{#include ../../../cluster/README.md:harness}}

## Limitations

Limitations, Aeron and rusteron versions, schema identities, failure modes and
what CI runs against a live cluster: [Compatibility](./compatibility.md).

Apache-2.0.
