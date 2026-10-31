# Troubleshooting

**First, check the node.** Run [`ergon-tune`](../tune/overview.md#check-a-node-first-the-ergon-tune-command)
on it, or in a pod that mounts the node's `/`, and fix every `WARN`:

```sh
ergon-tune --hot 2-7 --nic ens6               # on the node; --tcp, --busy-poll as the app does
ergon-tune --root /host --hot 2-7 --require   # in a pod with the node's / at /host
```

Each finding prints its fix. Latency symptoms the host causes (loop gaps,
timer ticks, interrupts, page faults, quota) map to a check in
[ergon-tune: symptom, check, fix](../tune/overview.md#symptom-check-fix).
The rest:

## Aeron

| symptom | check | fix |
|---|---|---|
| `Aeron::new` or `start` fails: no driver | is the driver running; does `$AERON_DIR/cnc.dat` exist in the app's container | the same `AERON_DIR` in both containers, on a volume both mount; retry with a deadline while the driver starts |
| `offer` returns `NotConnected` and stays there | `publication.is_connected()`; the subscriber's channel and stream id | the same channel and stream id on both sides; on UDP, `endpoint=` is the receiver's address and its UDP port is open between the hosts |
| `offer` returns `BackPressured` for long | does the subscriber poll every turn | poll it every turn; a larger `term-length` on the channel only buys time |
| the driver does not start with `AERON_DIR` on hugetlbfs | T23 | `AERON_FILE_PAGE_SIZE=2097152` and `AERON_TERM_BUFFER_SPARSE_FILE=false` |
| Aeron's CPU affinity settings have no effect | T22 | the Aeron 1.54 names: `AERON_THREAD_AFFINITY_USE_CPUSET_OFFSETS`, `AERON_THREAD_AFFINITY_FAIL_ON_VALIDATION_ERRORS` |
| the Aeron directory is on a disk | T4 | `/dev/shm`, a memory `emptyDir`, or a sized 2 MiB hugetlbfs |
| a crash after closing | was a handle used after its publication, subscription or client closed | close in order: claims and images, then publications and subscriptions, then the client |

## The loop thread

| symptom | check | fix |
|---|---|---|
| a helper thread runs on the loop's CPU | `ps -L -o tid,psr,comm -p <pid>`, `taskset -pc <tid>` | create every thread before the loop pins itself ([start-up order](busy-spin-loop.md#start-up-order)) |
| T1, T8 and T2 report nothing for the loop | was the loop's CPU in the `HotSet` | pin the thread first, then name its CPU in the `HotSet` |
| T19 warnings: the loop takes page faults | `FaultWatch` | `mlockall` (`CAP_IPC_LOCK`) and write every buffer once before the loop; T3, T5, T14, T17 on the node |
| millisecond stalls; `nr_throttled` grows | T9, `ThrottleWatch` | a Guaranteed pod, whole CPUs, the static CPU manager ([checklist](kubernetes.md#latency-checklist)) |
| stalls when the log is busy | does the loop format or write log lines itself | a bounded non-blocking queue to a logging thread |

## Kubernetes

| symptom | check | fix |
|---|---|---|
| every pod labelled `ergon.io/latency: hot` is refused | the plugin's log; `curl <plugin-pod-ip>:9466/readyz` | the kubelet config mounted (`hostContract.kubeletConfig`); on EKS AL2023 also `hostContract.kubeletConfigDir.enabled=true` ([shared CPUs](shared-cpus.md)) |
| a hot pod is refused after the node ran fine (`reason node-contract`) | the plugin's log names the drift: IRQs, the reserved set, a quota, isolation | fix the node; on EKS a NIC attached after boot puts its IRQs on the shared CPU: move them to the IRQ CPU |
| the container exits with `SIGILL` at start | the image's CPU tag against the node's CPU | build for a baseline CPU (`x86-64-v2`, `armv8.2-a`), or select nodes by instance family for a CPU-specific image ([build for the CPU](hosts.md#build-for-the-cpu)) |
| a K check warns (K1–K7) | the finding says pod spec or node | as the finding says ([what it checks](../tune/overview.md#what-it-checks)) |
| a DPDK process starts without its NIC | T24 | the NIC device plugin (not shipped here) must inject the three `RUSTERON_DPDK_SENDER_*` keys |
| latency good after boot, worse later (AWS) | T2, N1 | an ENA reset undid them: re-apply from a link-event hook ([interrupts and the NIC](hosts.md#interrupts-and-the-nic)) |
