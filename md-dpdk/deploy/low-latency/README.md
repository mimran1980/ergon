# Low-latency Kubernetes deployment of md-dpdk

The pod that was here moved into a full sample:
[samples/low-latency-k8s](../../../samples/README.md#aws-k8s). That sample has the
media driver (sender and receiver on two exclusive CPUs, everything else on
the shared CPU), two Aeron apps, the ergon-shared-cpus chart values, the node
settings and a verifier that proves the thread layout. The old `pod.yaml`
had no hugepages, no Aeron directory mount and no `ergon-cpus`; the sample's
manifest test keeps a copy as the case its rules must refuse.

## crates.io

To use the driver from your own crate, pin the published versions (`0.1.37`
is the crates.io release; this workspace is `0.1.37`):

```toml
ergon-md-dpdk = "=0.1.37"
ergon-tune = "=0.1.37"
```

See [Cargo.toml.sample](Cargo.toml.sample). The crates.io crate named
`ergon` is a different project.
