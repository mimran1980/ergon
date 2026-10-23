# Low-latency Kubernetes sample for md-dpdk

This is an example of the node and the pod. It is not a measured run.
EKS is not verified. No latency figure is stated here.

The pod in [pod.yaml](pod.yaml) requests `cpu: "2"` and
`ergon.io/shared-millicpus: "500"`. Two is the whole-number exclusive
request the static CPU manager pins. 500 asks for one shared CPU:
`ceil(500/1000)` is 1. The plugin accounts that CPU. It does not pin half
a core. A request of `M = 1000 × |S|` with oversubscription 1 is what
grants the whole configured helper set. This pod does not ask for that.

The hook pins threads that exist at create onto the shared CPUs. A later
thread inherits its creator's mask. A helper created by a pinned hot
thread inherits the hot mask when it is created with `pthread_create`.
That was observed for the C helper in `pin-test masks` on linuxkit, which
has no `isolcpus`. A Go 1.27 OS thread started from the hot thread stayed
on the shared CPU. JVM and Aeron were not run. The application can change
affinity.

## Public image and chart

Anyone can pull these. No image pull secret. Version 0.1.1 is not yet run
on kind.

| what | address |
|---|---|
| image | `ghcr.io/mimran1980/ergon-shared-cpus:0.1.1` |
| image digest | `sha256:69893e0c23ec6c3656053c0d6c4a06c676e33b726d56f362a30ce760908796c0` |
| chart | `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.1` |
| chart digest | `sha256:77a65305a45ea1b376ce80f9b1d5234b9061f4066a8c5baf5efdb892854fc0d8` |

The pod is labelled `ergon.io/latency: hot`, so the plugin checks it
against the node's kubelet config, `/sys` and `/proc` at create; without
the `hostContract` values it is refused. EKS AL2023 (needs chart 0.1.2,
not yet published): nodeadm writes `/etc/kubernetes/kubelet/config.json`
and the `NodeConfig` kubelet settings as the drop-in
`config.json.d/40-nodeadm.conf`, which `kubeletConfigDir` mounts and the
check merges as the kubelet does.

```sh
helm install ergon-shared-cpus oci://ghcr.io/mimran1980/charts/ergon-shared-cpus \
  --version 0.1.2 \
  --namespace ergon-system --create-namespace \
  --set-string config.sharedCpus=0 \
  --set 'config.namespaces={trading}' \
  --set hostContract.kubeletConfig.enabled=true \
  --set hostContract.kubeletConfig.path=/etc/kubernetes/kubelet/config.json \
  --set hostContract.kubeletConfigDir.enabled=true \
  --set hostContract.sys.enabled=true \
  --set hostContract.proc.enabled=true
```

A kubeadm node (kubelet file `/var/lib/kubelet/config.yaml`, no drop-in
directory) works with chart 0.1.1: `--version 0.1.1` and drop the
`kubeletConfig.path` and `kubeletConfigDir` lines. Chart 0.1.1 rejects
`kubeletConfigDir`; without it every hot create on EKS AL2023 is refused.

`config.sharedCpus` is the operator's helper list. It has to sit inside
`reservedSystemCPUs` and off the IRQ destination. Do not infer the CPU
numbers from an instance size. Label the node `ergon.io/shared-cpus=true`.

The chart writes every config key, including `seccomp.ioUring`, `ptp` and
`schedFifo`. The pod uses `repository:tag`, or `repository@digest` when
`image.digest` is set.

Apply the two admission files from `k8s/nri-latency-pods/admission/`:
`priority-class.yaml` and `validating-admission-policy.yaml`. Leave
`examples/` and the mutating policy unapplied. Admission of the Pod does
not control a later `kubectl exec`.

## crates.io

Pin the crates this repository publishes. `0.1.33` is the crates.io release.
This workspace is `0.1.34`, not yet on crates.io.

```toml
ergon-md-dpdk = "=0.1.33"
ergon-tune = "=0.1.33"
```

See [Cargo.toml.sample](Cargo.toml.sample). The crates.io name `ergon` is
a different project. Do not use it for this driver.

The media-driver image in the pod is one you build. It is not
`ghcr.io/mimran1980/ergon-shared-cpus`.

## Node

The chart does not set these. Operator-supplied, for this example: SMT
off, `cpuManagerPolicy: static`, `full-pcpus-only: true`, topology manager
`single-numa-node`, `reservedSystemCPUs` covering housekeeping, and the
boot line isolating only the exclusive CPUs
(`isolcpus=managed_irq,domain,<exclusive> nohz_full=<exclusive>
rcu_nocbs=<exclusive>`). `irqaffinity` and the IRQ watcher use a separate
IRQ destination inside housekeeping, so IRQs stay off the shared helpers
and off the busy CPUs. Leave `cpuCFSQuota` enabled. Do not set
`cpuCFSQuota: false`. Kubernetes 1.33 can drop the CFS quota for a
Guaranteed whole-CPU container when `DisableCPUQuotaWithExclusiveCPUs`
stays on. The plugin does not clear a quota.

Also: `irqbalance` off, NIC interrupts re-pinned after a reset,
`transparent_hugepage=never`, a sized 2 MiB hugetlbfs for `AERON_DIR`,
`kernel.numa_balancing=0`, and socket buffers at least as large as Aeron's
window. `SCHED_FIFO`, io_uring seccomp and PTP stay off unless you opt in
on a binary that has those grants. The chart writes those keys as
`"off"`.

The repository states a single-NUMA premise for the EKS notes. The driver
refuses two ENAs that report different NUMA nodes, and the device plugin
does not advertise that pair. This sample does not read a machine. The
book's measurements include an SMT-on receiver. That does not change the
SMT-off setting this pod assumes.

Hot creates are refused when the mounted kubelet config, the NIC IRQ
masks, or the isolated set do not match. The plugin stays connected.
While the plugin is down, kubelet reconciliation can put helpers on the
exclusive CPUs. Recovery does not show that the contract held during that
outage.

Driver env that matches the existing media-driver ConfigMap
(`deploy/aws-eks/media-driver-config.yaml`): `AERON_FILE_PAGE_SIZE=2097152`,
`AERON_TERM_BUFFER_SPARSE_FILE=false`, the Aeron directory on the sized
hugetlbfs. The pod sets those. It does not add a cpuset partition.
`aeronmd` does not move threads into child cgroups.
