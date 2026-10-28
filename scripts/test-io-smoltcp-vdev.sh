#!/usr/bin/env bash
# Runs ergon-io's smoltcp-spin vdev tests (io/tests-dpdk/smoltcp_vdev.rs)
# without root, hugetlbfs or host changes, in a user, network and mount
# namespace of its own (`unshare -rnm`, as scripts/test-dpdk-vdev-rootless.sh
# does for ergon-md-dpdk):
#   - a veth pair v0/v1: the DPDK side takes v0 through the net_af_packet
#     PMD (qdisc_bypass=0, so tc netem on v0 applies to its frames); the
#     kernel side has 10.99.0.1/24 on v1, with its checksum and segmentation
#     offloads off (an AF_PACKET reader would otherwise see partial
#     checksums and 64 KiB GSO frames) and an RTO floor of 10 ms on the
#     route, as the kernel arms of the benchmarks use;
#   - the peer (examples/smoltcp_peer: framed TCP, WebSocket over TLS and
#     UDP echo on the kernel's stack) is started by each test;
#   - EAL runs with --no-huge --no-pci, a private tmpfs over /run.
#
# Needs (Linux x86_64 or aarch64): unprivileged user namespaces, iproute2
# (ip, tc), ethtool, libdpdk >= 23.11 with its af_packet PMD, cargo, and
# sch_netem loaded on the host (a namespace cannot load modules). In Docker:
# a non-root user, --security-opt seccomp=unconfined and
# apparmor=unconfined (io/native/dpdk/test/verify-smoltcp.sh does this).
#
# Usage: scripts/test-io-smoltcp-vdev.sh [test filter...]
#   ERGON_SMOLTCP_TEST_BIN / ERGON_SMOLTCP_PEER_BIN: prebuilt binaries
#   (otherwise cargo builds them first, outside the namespace).
set -euo pipefail

log() { echo "[smoltcp-vdev] $*" >&2; }
die() { echo "[smoltcp-vdev] ERROR: $*" >&2; exit 1; }

if [[ -z "${ERGON_SMOLTCP_INNER:-}" ]]; then
    # ---- outside: an unprivileged user, before the namespace -------------
    [[ "$(id -u)" != 0 ]] || die "run this unprivileged (it makes its own user namespace)"
    if ! err="$(unshare -rnm true 2>&1)"; then
        die "unshare -rnm refused: ${err:-no message} (Ubuntu 23.10+: sysctl kernel.apparmor_restrict_unprivileged_userns=0; Docker: --security-opt seccomp=unconfined --security-opt apparmor=unconfined)"
    fi
    cd "$(dirname "$0")/.."
    if [[ ! -x "${ERGON_SMOLTCP_TEST_BIN:-}" || ! -x "${ERGON_SMOLTCP_PEER_BIN:-}" ]]; then
        # Built out here: the namespace has no network for cargo.
        log "building the vdev test and its peer (--features smoltcp-dpdk)"
        built="$("${CARGO:-cargo}" build -p ergon-io --features smoltcp-dpdk --test smoltcp_vdev \
            --example smoltcp_peer --message-format=json-render-diagnostics)"
        exe() { sed -n "s/.*\"executable\":\"\([^\"]*$1[^\"]*\)\".*/\1/p" <<<"$built" | tail -1; }
        export ERGON_SMOLTCP_TEST_BIN="$(exe smoltcp_vdev)"
        export ERGON_SMOLTCP_PEER_BIN="$(exe smoltcp_peer)"
    fi
    [[ -x "$ERGON_SMOLTCP_TEST_BIN" ]] || die "no test binary: ${ERGON_SMOLTCP_TEST_BIN:-unset}"
    [[ -x "$ERGON_SMOLTCP_PEER_BIN" ]] || die "no peer binary: ${ERGON_SMOLTCP_PEER_BIN:-unset}"
    exec unshare -rnm --propagation private env ERGON_SMOLTCP_INNER=1 \
        ERGON_SMOLTCP_TEST_BIN="$(realpath "$ERGON_SMOLTCP_TEST_BIN")" \
        ERGON_SMOLTCP_PEER_BIN="$(realpath "$ERGON_SMOLTCP_PEER_BIN")" \
        bash "$PWD/scripts/test-io-smoltcp-vdev.sh" "$@"
fi

# ---- inside: root of the namespace ------------------------------------------
for tool in ip tc ethtool; do
    command -v "$tool" >/dev/null || die "$tool is missing (iproute2, ethtool)"
done
run_dir=/run
[[ -d "$run_dir" ]] || run_dir=/var/run
mount -t tmpfs -o mode=0755 ergon-smoltcp "$run_dir" || die "cannot mount a private tmpfs on $run_dir"
ip link set lo up
ip link add v0 type veth peer name v1
for dev in v0 v1; do
    # No IPv6 link-local address, so no DAD or router solicitations.
    ip link set "$dev" addrgenmode none
    # Whole, checksummed frames of at most the MTU in both directions.
    ethtool -K "$dev" tx off rx off tso off gso off gro off >/dev/null
    ip link set "$dev" up
done
ip addr add 10.99.0.1/24 dev v1
ip route replace 10.99.0.0/24 dev v1 src 10.99.0.1 rto_min 10ms
# netem must be usable here (sch_netem loaded on the host).
tc qdisc add dev v1 root netem delay 0ms || die "tc netem is unusable (modprobe sch_netem on the host)"
tc qdisc del dev v1 root
peer_mac="$(ip -br link show dev v1 | awk '{print $3}')"
log "namespace: v0 (DPDK, net_af_packet) <-> v1 (kernel, 10.99.0.1, $peer_mac); $(uname -r) $(uname -m)"

export ERGON_SMOLTCP_STACK="port=net_af_packet0,ip=10.99.0.2/24,eal=--no-huge -m 256 --no-pci --iova-mode=va --vdev=net_af_packet0,iface=v0,qdisc_bypass=0"
export ERGON_SMOLTCP_PEER=10.99.0.1
export ERGON_SMOLTCP_PEER_MAC="$peer_mac"
export ERGON_SMOLTCP_NETEM="v0 v1"
# A host check reads its CPU and pod settings; it does not fail the run.
export ERGON_HOST_CHECKS="${ERGON_HOST_CHECKS:-on}"
exec "$ERGON_SMOLTCP_TEST_BIN" --test-threads=1 --nocapture "$@"
