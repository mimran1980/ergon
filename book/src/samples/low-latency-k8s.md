# Low-Latency Kubernetes

[`samples/low-latency-k8s`](https://github.com/mimran1980/ergon/tree/main/samples/low-latency-k8s)
is the whole stack in one place: an Aeron ping and pong that each
busy-spin on an exclusive CPU and print their own latency, the media
driver, the node and pod settings, and a verifier that proves where every
thread ran. Its README section is the full guide:
[samples/README.md#low-latency-k8s](https://github.com/mimran1980/ergon/blob/main/samples/README.md#low-latency-k8s).

| part | what it is |
|---|---|
| `src/bin/ll-app.rs`, `src/pingpong.rs` | the app: one pinned Aeron thread (`ll-hot`), helpers on the shared CPUs, ergon-tune's K checks at start, a placement self-check, a latency report |
| `src/bin/ll-probe.rs`, `scripts/verify.sh` | the verifier: every thread's CPU over a run |
| `src/logging.rs` | `tracing` through a bounded, non-blocking queue: the hot thread never waits on stdout |
| `docker/` | the image (app, DPDK media driver, entrypoint) and its CPU-level tags |
| `k8s/base`, `k8s/components`, `k8s/overlays` | kustomize: kind (`vdev-local`), a Linux VM (`vdev-isolated`), EKS (`eks-ena-x86`, `eks-ena-arm64`) |
| `helm/` | values for the [shared-CPU plugin](../deploy/shared-cpus.md) chart |
| `node/` | node preparation: EKS user data, kind config, a VM script, the quiet-CPU unit |
| `tests/aeron_client_book.rs`, `tests/busy_loop_book.rs` | the code in [Aeron client](../transports/aeron-client.md) and [busy-spin loop](../deploy/busy-spin-loop.md) |

What has run where (macOS, Docker on arm64 and amd64, a Docker DPDK
end-to-end) and what has not (kind, a VM with `isolcpus`, EKS): the
README's
[What has run, and where](https://github.com/mimran1980/ergon/blob/main/samples/README.md#what-has-run-and-where).
No latency figure from this sample on real hardware is published.

```sh
just samples-ll          # fmt, clippy, unit, logger, Aeron client and manifest tests (host)
just samples-ll-docker   # the same plus the Linux tests, linux/arm64 and linux/amd64
just samples-ll-e2e      # driver, ping, pong and ll-probe in Docker over DPDK taps
```
