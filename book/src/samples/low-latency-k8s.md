<!-- Generated page: edits here are overwritten by the next export. sha256:521d9f520ce66d7d62b942e67b638b65c647a8237d082e142940ee5abd052f20 -->
# Low-latency Kubernetes

This sample
is the whole stack in one place: an Aeron ping and pong that each
busy-spin on an exclusive CPU and print their own latency, the media
driver, the node and pod settings, and a verifier that proves where every
thread ran. Its README section is the full guide.

| part | what it is |
|---|---|
| `src/bin/ll-app.rs`, `src/pingpong.rs` | the app: one pinned Aeron thread (`ll-hot`), helpers on the shared CPUs, ergon-tune's K checks at start, a placement self-check, a latency report |
| `src/bin/ll-probe.rs`, `verify.sh` | the verifier: every thread's CPU over a run |
| `src/logging.rs` | `tracing` through a bounded, non-blocking queue: the hot thread never waits on stdout |
| `docker/` | the image (app, DPDK media driver, entrypoint) and its CPU-level tags |
| `k8s/base`, `k8s/components`, `k8s/overlays` | kustomize: kind (`vdev-local`), a Linux VM (`vdev-isolated`), EKS (`eks-ena-x86`, `eks-ena-arm64`) |
| `helm/` | values for the [shared-CPU plugin](../deploy/shared-cpus.md) chart |
| `node/` | node preparation: EKS user data, kind config, a VM script, the quiet-CPU unit |
| `tests/aeron_client_book.rs`, `tests/busy_loop_book.rs` | the code in [Aeron client](../transports/aeron-client.md) and a busy-spin loop that polls ergon-io and Aeron together |

What has run where (macOS, Docker on arm64 and amd64, a Docker DPDK
end-to-end) and what has not (kind, a VM with `isolcpus`, EKS): the
README's "What has run, and where".
No latency figure from this sample on real hardware is published.

Its checks: fmt, clippy, unit, logger, Aeron client and manifest tests on
the host; the same plus the Linux tests in Docker (linux/arm64 and
linux/amd64); and an end-to-end run of the driver, ping, pong and ll-probe
in Docker over DPDK taps.
