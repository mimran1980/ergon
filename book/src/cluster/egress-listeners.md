# Egress Listeners

Implement `EgressListener` and pass it through `EgressAdapter` to
`AeronCluster::poll_egress`. Use `ControlledEgressListener` and
`ControlledEgressAdapter` when callbacks must return Aeron controlled-poll
actions.

Protocol errors, listener panics, keep-alive failures, publication failures, and
reconnect failures are returned as `ClusterError`. Application payloads,
credentials, challenges, and binary response data remain byte slices. Text
fields declared by the protocol are validated before being exposed as `&str`.

The high-level client, configuration, listener, state, error, offer, and claim
types are the consumer-facing surface. The generated protocol codecs are also
reachable, for advanced direct encode/decode, through the `cluster_codec_types`
module — but that module is `#[doc(hidden)]` and **not** a stable API: it exists
for repository tests and low-level experimentation, and its shape may change
without a semver bump. Normal applications should use `AeronCluster`.
