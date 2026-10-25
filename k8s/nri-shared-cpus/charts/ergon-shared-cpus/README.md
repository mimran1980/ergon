# ergon-shared-cpus chart

A Guaranteed Kubernetes pod gets its exclusive CPUs plus the node's shared
housekeeping CPUs, and every new thread starts on the shared ones, so the
busy-spin loop keeps its core. One DaemonSet: a containerd NRI plugin and a
kubelet device plugin.

Not yet run on kind. Not verified on EKS. Latency not measured.

## Install

The chart does not set the boot line, the kubelet or containerd. Each node
needs:

- Kubernetes >= 1.33, `cpuManagerPolicy: static`;
- `reservedSystemCPUs` = the IRQ CPU(s) + the shared CPUs;
- kernel >= 6.2 with cgroup v2;
- containerd >= 2.2 with the NRI default validator on;
- the label `ergon.io/shared-cpus=true`;
- the kubelet config at `/var/lib/kubelet/config.yaml`, or set
  `hostContract.kubeletConfig.path`. The plugin pod does not start
  without it. A kubelet started with `--config-dir` also needs
  `hostContract.kubeletConfigDir` (EKS AL2023, below).

A pod labelled `ergon.io/latency: hot` is also checked at create: every
exclusive CPU in `isolcpus`, no shared CPU isolated or in `nohz_full`,
`cpuCFSQuota` not false, and NIC IRQs on `reservedSystemCPUs` minus
`sharedCpus`. A miss refuses that create. The check reads the kubelet file
merged with its drop-in directory when `kubeletConfigDir` is set.

```sh
helm install ergon-shared-cpus oci://ghcr.io/mimran1980/charts/ergon-shared-cpus \
  --version 0.1.2 \
  --namespace ergon-system --create-namespace \
  --set-string config.sharedCpus=0 \
  --set 'config.namespaces={trading}' \
  --set hostContract.kubeletConfig.enabled=true \
  --set hostContract.sys.enabled=true \
  --set hostContract.proc.enabled=true
```

**EKS AL2023** (not verified on EKS). nodeadm writes `/etc/kubernetes/kubelet/config.json` and the
`NodeConfig` kubelet settings as the drop-in
`/etc/kubernetes/kubelet/config.json.d/40-nodeadm.conf`, and runs the
kubelet with `--config-dir` on that directory:

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

Without `kubeletConfigDir` every hot create on EKS AL2023 is refused.

## Key values

| key | default | effect |
|---|---|---|
| `config.sharedCpus` | `"4"` | **required.** Inside `reservedSystemCPUs`, not an IRQ CPU |
| `config.namespaces` | `[trading]` | allowlist; empty refuses every opt-in |
| `config.oversubscription` | `1` | capacity = `len(sharedCpus) × 1000 ×` this |
| `image.digest` | empty | `sha256:…` renders `repository@digest` |
| `layoutRevision` | empty | a ConfigMap edit does not roll the DaemonSet: drain, then set a new revision |
| `admission.create` | `false` | `true` installs the latency-pod admission policy and PriorityClasses |
| `hostContract.{kubeletConfig,sys,proc}.enabled` | `true` | mount the kubelet file, `/sys` and `/proc` read-only for the hot-pod check |
| `hostContract.kubeletConfig.path` | `/var/lib/kubelet/config.yaml` | the kubelet config file on the node |
| `hostContract.kubeletConfigDir.enabled` | `false` | mount the kubelet's `--config-dir` read-only (a `Directory`: a missing path keeps the pod from starting) and merge its `*.conf` drop-ins as the kubelet does |
| `hostContract.kubeletConfigDir.path` | `/etc/kubernetes/kubelet/config.json.d` | the drop-in directory on the node (the EKS AL2023 path) |

Every value: `values.yaml` (`helm show values`). Node setup, pod shape,
admission, failure modes and limits:
[k8s/nri-shared-cpus](https://github.com/mimran1980/ergon/tree/main/k8s/nri-shared-cpus).

Apache-2.0.
