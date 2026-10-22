#!/usr/bin/env bash
# Prints the user data for a self-launched EKS node (Amazon Linux 2023 EKS
# AMI) that hosts the rusteron-media-driver DPDK ENA pair: a gzip-compressed
# MIME multipart document with
#
#   * a shell part (cloud-init) that writes /etc/rusteron-dpdk/node.env,
#     writes provision-node.sh, bootstrap-dpdk-ena.sh, preflight.sh,
#     irq-affinity.sh, rusteron-dpdk-ena.service and
#     rusteron-irq-affinity.service to /opt/rusteron-dpdk/node from quoted
#     here-documents and runs provision-node.sh, which reboots the node once
#     before kubelet starts. Each file goes in as its header (the leading
#     comment block, its documentation) and its code: the comment lines in
#     between stay in the repo;
#   * the nodeadm NodeConfig: the cluster, and kubelet's low-latency settings.
#
# usage: userdata.sh <cluster> <apiServerEndpoint> <ca-b64> <service-cidr> <reserved-cpus> [irq-cpus] > userdata.gz
#
#   <reserved-cpus>  kubelet's reservedSystemCPUs and the kernel's housekeeping
#                    CPUs (every other CPU is isolated): whole physical cores,
#                    one CPU each with SMT off (the launch template's CpuOptions
#                    ThreadsPerCore=1), e.g. "0" on a c7i.4xlarge.
#   [irq-cpus]       IRQ destination, a subset of <reserved-cpus>. Default is
#                    <reserved-cpus> (or RUSTERON_IRQ_CPUS when that is set),
#                    so a one-CPU reservation still takes interrupts. Set it to
#                    one reserved CPU when the others are shared helpers.
#
# The output is binary: pass it as `--user-data fileb://userdata.gz`. EC2
# limits raw user data to 16384 bytes, and cloud-init and nodeadm both
# decompress gzip user data, which with the in-body comments left out is how
# the scripts fit; this fails rather than print more than the limit.
#
# Optional environment:
#   RUSTERON_DPDK_DEVICE_NUMBERS  the sender's and receiver's ENI device numbers ("1 2")
#   RUSTERON_HUGEPAGES            2 MiB hugepages reserved at boot (2048)
#
# Kubelet settings, and why:
#   * cpuManagerPolicy static with full-pcpus-only: a Guaranteed pod gets
#     whole physical cores, never one SMT twin shared with another pod.
#   * reservedSystemCPUs: the housekeeping cores, outside every exclusive
#     cpuset. Kubelet rejects it while kubeReservedCgroup or
#     systemReservedCgroup is set, and nodeadm sets both, so both are emptied
#     here: nodeadm writes this config as a drop-in that kubelet applies as a
#     JSON merge patch, so "" overrides.
#   * topologyManagerPolicy single-numa-node, scope pod: the pod's CPUs and
#     devices come from one NUMA node (memory is not a hint provider without
#     the Memory Manager).
#   * memoryManagerPolicy None (kubelet's default, stated): every c7i size
#     below metal-48xl has one NUMA node, so Static would pin nothing, and
#     kubelet refuses to start under Static unless reservedMemory equals
#     kubeReserved + systemReserved + the hard eviction threshold exactly.
#     nodeadm's memory reservations stay as it derives them.
#   * cpuCFSQuota is not set: it stays true (the default), so every pod's CPU
#     limit holds. Kubelet 1.33 and later (1.36 or later here) sets no CPU
#     quota on a container with exclusive CPUs (this static policy, a
#     Guaranteed pod, a whole-number CPU request), nor on its pod. Never set
#     it false: that removes the CPU limit of every pod on the node.
#   * The node registers labelled rusteron-dpdk/ena=true and tainted
#     rusteron-dpdk=true:NoSchedule, so only the DPDK workloads land there.
set -euo pipefail

fail() { echo "userdata FAIL: $*" >&2; exit 1; }
if (($# < 5 || $# > 6)); then
    echo "usage: $0 <cluster> <apiServerEndpoint> <ca-b64> <service-cidr> <reserved-cpus> [irq-cpus] > userdata.gz" >&2
    exit 2
fi
cluster="$1" endpoint="$2" ca="$3" cidr="$4" reserved="$5"
irq="${6:-${RUSTERON_IRQ_CPUS:-$reserved}}"
device_numbers="${RUSTERON_DPDK_DEVICE_NUMBERS:-1 2}"
hugepages="${RUSTERON_HUGEPAGES:-2048}"

[[ "$cluster" =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]] || fail "cluster name '$cluster'"
[[ "$endpoint" =~ ^https://[^[:space:]]+$ ]] || fail "API server endpoint '$endpoint': https://..."
[[ "$ca" =~ ^[A-Za-z0-9+/=]+$ ]] || fail "certificate authority: base64 data"
[[ "$cidr" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+/[0-9]+$ ]] || fail "service CIDR '$cidr'"
[[ "$reserved" =~ ^[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*$ ]] || fail "reserved CPUs '$reserved': a CPU list such as 0"
[[ "$irq" =~ ^[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*$ ]] || fail "IRQ CPUs '$irq': a CPU list such as 0"
[[ "$device_numbers" =~ ^[1-9][0-9]*\ [1-9][0-9]*$ ]] || fail "RUSTERON_DPDK_DEVICE_NUMBERS='$device_numbers': two device numbers"
[[ "$hugepages" =~ ^[1-9][0-9]*$ ]] || fail "RUSTERON_HUGEPAGES='$hugepages': a positive count"

node_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
files=(provision-node.sh bootstrap-dpdk-ena.sh preflight.sh irq-affinity.sh
    rusteron-dpdk-ena.service rusteron-irq-affinity.service)
eof=RUSTERON_NODE_FILE_EOF
for f in "${files[@]}"; do
    # Each file goes in verbatim as a quoted here-document: plain ASCII, a
    # final newline, and no line equal to the terminator.
    [[ -f "$node_dir/$f" ]] || fail "$node_dir/$f missing"
    ! LC_ALL=C grep -q $'[^\t -~]' "$node_dir/$f" || fail "$f has non-ASCII bytes"
    [[ -z "$(tail -c 1 "$node_dir/$f")" ]] || fail "$f does not end in a newline"
    ! grep -qx "$eof" "$node_dir/$f" || fail "$f has a line '$eof'"
done

# A node file as the node gets it: the leading comment block, then every line
# but a comment line. None of the files has a here-document or string with a
# line that starts with '#' (node/test/test-bootstrap.sh runs the result).
node_file() {
    awk 'head && /^#/ { print; next } { head = 0 } /^[[:space:]]*#/ { next } { print }' head=1 "$1"
}

boundary="==rusteron-dpdk-node=="
out="$(mktemp)"
trap 'rm -f "$out"' EXIT
{
    printf 'MIME-Version: 1.0\nContent-Type: multipart/mixed; boundary="%s"\n\n' "$boundary"
    printf -- '--%s\nContent-Type: text/x-shellscript; charset="us-ascii"\n\n' "$boundary"
    cat <<EOF
#!/bin/bash
set -euo pipefail
install -d -m 0755 /etc/rusteron-dpdk /opt/rusteron-dpdk/node
cat > /etc/rusteron-dpdk/node.env <<'ENV'
RUSTERON_DPDK_DRIVER=igb_uio
RUSTERON_HOUSEKEEPING_CPUS=$reserved
RUSTERON_IRQ_CPUS=$irq
RUSTERON_DPDK_DEVICE_NUMBERS="$device_numbers"
RUSTERON_HUGEPAGES=$hugepages
ENV
cd /opt/rusteron-dpdk/node
EOF
    for f in "${files[@]}"; do
        printf "cat > %s <<'%s'\n" "$f" "$eof"
        node_file "$node_dir/$f"
        printf '%s\n' "$eof"
    done
    printf 'chmod 0755 *.sh\nexec ./provision-node.sh\n\n'
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
      cpuManagerPolicyOptions:
        full-pcpus-only: "true"
      reservedSystemCPUs: "$reserved"
      kubeReservedCgroup: ""
      systemReservedCgroup: ""
      topologyManagerPolicy: single-numa-node
      topologyManagerScope: pod
      memoryManagerPolicy: None
    flags:
      - --node-labels=rusteron-dpdk/ena=true
      - --register-with-taints=rusteron-dpdk=true:NoSchedule

EOF
    printf -- '--%s--\n' "$boundary"
} | gzip -9n > "$out"
size=$(($(wc -c < "$out")))
((size <= 16384)) || fail "user data is $size bytes gzipped, over EC2's 16384-byte limit"
cat "$out"
