#!/usr/bin/env bash
# Prints the user data for one self-launched EKS AL2023 latency node (the
# NRI lab, eks-nri.sh): a gzip-compressed MIME multipart document with
#
#   * a shell part (cloud-init) that sets the kernel command line with grubby
#     (hugepages for AERON_DIR, nohz_full/rcu_nocbs/isolcpus on the isolated
#     CPUs, irqaffinity on the IRQ CPU, THP never), the node's sysctls
#     (rmem_max/wmem_max: Aeron's 2 MiB window; never busy_poll, which was
#     catastrophic on Graviton) and systemd's CPUAffinity on the reserved
#     CPUs, then reboots once. nodeadm-run.service (which starts kubelet) is
#     ordered after cloud-final, so kubelet's first start is already on the
#     provisioned command line.
#   * the nodeadm NodeConfig: the cluster, and kubelet's low-latency settings.
#
# usage: eks-nri-userdata.sh <cluster> <apiServerEndpoint> <ca-b64> <service-cidr> \
#          <reserved-cpus> <isolated-cpus> [hugepages] > userdata.gz
#
#   <reserved-cpus>   kubelet's reservedSystemCPUs, also the kernel's
#                     housekeeping CPUs (irqaffinity): "0,1" on a c8g.xlarge
#                     (0 = system and the shared pool, 1 = NIC IRQs).
#   <isolated-cpus>   the CPUs every latency pod's exclusive cpuset comes from:
#                     "2,3" on a c8g.xlarge. nohz_full, rcu_nocbs and
#                     isolcpus=managed_irq name these, never the reserved ones:
#                     the shared CPU must load-balance.
#   [hugepages]       2 MiB pages reserved at boot (256 = 512 MiB).
#
# The output is binary: pass it as `--user-data fileb://userdata.gz`. EC2
# limits raw user data to 16384 bytes and both cloud-init and nodeadm
# decompress gzip user data; this fails rather than print more than the limit.
#
# Kubelet settings, and why:
#   * cpuManagerPolicy static: a Guaranteed pod on whole CPUs gets exclusive
#     CPUs from the static pool (2,3 here).
#   * reservedSystemCPUs: the system/shared CPU and the NIC IRQ CPU, outside
#     every exclusive cpuset. Kubelet rejects it while kubeReservedCgroup or
#     systemReservedCgroup is set, and nodeadm sets both, so both are emptied
#     here (a JSON merge patch: "" overrides).
#   * topologyManagerPolicy single-numa-node, scope pod: the pod's CPUs come
#     from one NUMA node. No effect on a single-NUMA c8g.xlarge; kept so the
#     same node config is right on multi-NUMA types.
#   * cpuCFSQuota is NEVER set false: that removes the CPU limit of every pod
#     on the node. Kubelet 1.33+ sets no CPU quota on a container with
#     exclusive CPUs (1.36 is the floor here), so the hot container needs no
#     exception.
#   * no strict-cpu-reservation (it would empty the shared pool).
#   * Graviton has no SMT: no full-pcpus-only (it is x86-only anyway).
set -euo pipefail

fail() { echo "userdata FAIL: $*" >&2; exit 1; }
if (($# < 6 || $# > 7)); then
    echo "usage: $0 <cluster> <apiServerEndpoint> <ca-b64> <service-cidr> <reserved-cpus> <isolated-cpus> [hugepages] > userdata.gz" >&2
    exit 2
fi
cluster="$1" endpoint="$2" ca="$3" cidr="$4" reserved="$5" isolated="$6"
hugepages="${7:-256}"

[[ "$cluster" =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]] || fail "cluster name '$cluster'"
[[ "$endpoint" =~ ^https://[^[:space:]]+$ ]] || fail "API server endpoint '$endpoint': https://..."
[[ "$ca" =~ ^[A-Za-z0-9+/=]+$ ]] || fail "certificate authority: base64 data"
[[ "$cidr" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+/[0-9]+$ ]] || fail "service CIDR '$cidr'"
[[ "$reserved" =~ ^[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*$ ]] || fail "reserved CPUs '$reserved': a CPU list such as 0,1"
[[ "$isolated" =~ ^[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*$ ]] || fail "isolated CPUs '$isolated': a CPU list such as 2,3"
[[ "$hugepages" =~ ^[1-9][0-9]*$ ]] || fail "hugepages '$hugepages': a positive count"
# the two lists must not overlap
overlap=$(python3 -c '
import sys
def cpus(s):
    out = set()
    for part in s.split(","):
        lo, _, hi = part.partition("-")
        out.update(range(int(lo), int(hi or lo) + 1))
    return out
print("yes" if cpus(sys.argv[1]) & cpus(sys.argv[2]) else "no")' "$reserved" "$isolated") ||
    fail "cannot compare the CPU lists"
[[ $overlap == no ]] || fail "reserved CPUs '$reserved' and isolated CPUs '$isolated' overlap"

boundary="==ergon-nri-node=="
out="$(mktemp)"
trap 'rm -f "$out"' EXIT
{
    printf 'MIME-Version: 1.0\nContent-Type: multipart/mixed; boundary="%s"\n\n' "$boundary"
    printf -- '--%s\nContent-Type: text/x-shellscript; charset="us-ascii"\n\n' "$boundary"
    cat <<EOF
#!/bin/bash
# First boot only: the kernel command line and the node's sysctls, then one
# reboot before kubelet's first start (nodeadm-run is after cloud-final).
set -euo pipefail
if grep -q ' isolcpus=' /proc/cmdline; then
    exit 0
fi
args="hugepagesz=2M hugepages=$hugepages transparent_hugepage=never"
args="\$args nohz_full=$isolated rcu_nocbs=$isolated isolcpus=managed_irq,$isolated"
# irqaffinity names the reserved CPUs: the NIC IRQs land on the IRQ CPU there,
# never on an isolated one (Graviton kernel-UDP nodes need a spare CPU for IRQs).
args="\$args irqaffinity=$reserved"
grubby --update-kernel=ALL --args="\$args"
info="\$(grubby --info=DEFAULT)"
grep -q 'isolcpus=' <<<"\$info" || { echo "ergon-lab: grubby did not set the kernel arguments" >&2; exit 1; }
mkdir -p /etc/sysctl.d /etc/systemd/system.conf.d
cat > /etc/sysctl.d/90-ergon-lab.conf <<'SYS'
# Aeron's 2 MiB receive window. Not busy_poll/busy_read: those are host-wide
# and were catastrophic on Graviton (p50 4973 us at 1 Mpps).
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
SYS
sysctl --system >/dev/null
cat > /etc/systemd/system.conf.d/90-ergon-lab.conf <<'SYS'
[Manager]
CPUAffinity=$reserved
SYS
rm -f /var/lib/kubelet/cpu_manager_state
echo "ergon-lab: rebooting into the provisioned kernel command line"
systemctl reboot

EOF
    printf -- '--%s\nContent-Type: application/node.eks.aws\n\n' "$boundary"
    cat <<EOF
---
apiVersion: node.eks.aws/v1alpha1
kind: NodeConfig
spec:
  cluster:
    name: $cluster
    apiServerEndpoint: $endpoint
    certificateAuthority: $ca
    cidr: $cidr
  kubelet:
    config:
      cpuManagerPolicy: static
      reservedSystemCPUs: "$reserved"
      kubeReservedCgroup: ""
      systemReservedCgroup: ""
      topologyManagerPolicy: single-numa-node
      topologyManagerScope: pod
    flags:
      - --node-labels=ergon.io/shared-cpus=true
  containerd:
    config: |
      [plugins."io.containerd.nri.v1.nri".default_validator]
        enable = true

EOF
    printf -- '--%s--\n' "$boundary"
} | gzip -9n > "$out"
size=$(($(wc -c < "$out")))
((size <= 16384)) || fail "user data is $size bytes gzipped, over EC2's 16384-byte limit"
cat "$out"
