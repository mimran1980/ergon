#!/usr/bin/env bash
# Runs scripts/test-dpdk-vdev.sh (every vdev scenario, over the real DPDK tap
# PMD) without root, hugetlbfs or host changes: inside a user, network and
# mount namespace of its own (`unshare -rnm`), where this unprivileged user is
# root over a private network stack. There:
#   - the tap PMD (net_tap) creates its taps through /dev/net/tun, which the
#     namespace's root may do for its own network namespace;
#   - EAL runs without hugepages (RUSTERON_VDEV_NO_HUGE=1: --no-huge -m 512
#     --no-pci), so no hugetlbfs mount or vm.nr_hugepages;
#   - a private tmpfs over /run (/var/run) holds EAL's runtime directory,
#     which the namespace's root could not create on the host's /run.
# The bridge, the taps and every address live and die with the namespace.
#
# Needs (Linux x86_64 or aarch64): unprivileged user namespaces, a readable
# and writable /dev/net/tun, iproute2, ping, libdpdk (the build's), cargo.
# Ubuntu 23.10 and later refuse unprivileged user namespaces through AppArmor
# (kernel.apparmor_restrict_unprivileged_userns=1): the run then fails and
# says so, never skips. That sysctl and loading sch_netem (loss_recovery's tc
# netem; a namespace cannot load modules) are host settings, made once by an
# administrator; the run itself stays unprivileged. In Docker: run as a
# non-root user with --security-opt seccomp=unconfined (unshare(CLONE_NEWUSER)
# is not in the default profile), --security-opt apparmor=unconfined where
# AppArmor confines containers, and --device /dev/net/tun.
#
# Idea from melin-dpdk's netns runner (BUSL-1.1); written here from scratch.
# It uses net_tap, not melin's net_af_packet: af_packet advertises no IPv4/UDP
# checksum offloads, which the transport requires of every port.
#
# Usage: scripts/test-dpdk-vdev-rootless.sh [--self-test] [test-dpdk-vdev.sh args...]
#   --self-test   only probe: the namespace, /dev/net/tun, a tap, the tmpfs
set -euo pipefail

SYSCTL=kernel.apparmor_restrict_unprivileged_userns
TUN="${RUSTERON_ROOTLESS_TUN:-/dev/net/tun}"
log() { echo "[rootless] $*" >&2; }
die() { echo "[rootless] ERROR: $*" >&2; exit 1; }

SELF_TEST=0
if [[ "${1:-}" == --self-test ]]; then
    SELF_TEST=1
    shift
fi

if [[ -z "${RUSTERON_ROOTLESS_INNER:-}" ]]; then
    # ---- outside: an unprivileged user, before the namespace ---------------
    [[ "$(id -u)" != 0 ]] ||
        die "run this unprivileged; as root use sudo scripts/test-dpdk-vdev.sh (the sudo lane)"
    if ! err="$(unshare -rnm true 2>&1)"; then
        restrict="$(cat "/proc/sys/kernel/apparmor_restrict_unprivileged_userns" 2>/dev/null || echo absent)"
        max_ns="$(cat /proc/sys/user/max_user_namespaces 2>/dev/null || echo absent)"
        die "unshare -rnm refused: ${err:-no message}. $SYSCTL=$restrict (1 refuses unprivileged user namespaces on Ubuntu 23.10 and later; an administrator sets it to 0: sudo sysctl -w $SYSCTL=0), user.max_user_namespaces=$max_ns (0 refuses them). In Docker: --security-opt seccomp=unconfined (and apparmor=unconfined where AppArmor confines containers)."
    fi
    log "user namespace: ok (unshare -rnm)"
    cd "$(dirname "$0")/.."
    if ((SELF_TEST == 0)) && [[ ! -x "${RUSTERON_HARNESS_BIN:-}" ]]; then
        # Built out here: the namespace has no network for cargo.
        log "building dpdk-harness (--features dpdk)"
        "${CARGO:-cargo}" build -p ergon-md-dpdk-harness --features dpdk
        export RUSTERON_HARNESS_BIN="${CARGO_TARGET_DIR:-target}/debug/dpdk-harness"
        [[ -x "$RUSTERON_HARNESS_BIN" ]] || die "the harness build did not produce $RUSTERON_HARNESS_BIN"
    fi
    args=()
    ((SELF_TEST)) && args+=(--self-test)
    exec unshare -rnm --propagation private env RUSTERON_ROOTLESS_INNER=1 bash "$PWD/scripts/test-dpdk-vdev-rootless.sh" "${args[@]}" "$@"
fi

# ---- inside: root of the namespace ------------------------------------------
[[ -r "$TUN" && -w "$TUN" ]] ||
    die "no usable $TUN in the namespace: net_tap needs it (modprobe tun on the host; in Docker --device /dev/net/tun)"
# EAL's runtime directory (/var/run/dpdk/<prefix> as root): a tmpfs of our own.
run_dir=/run
[[ -d "$run_dir" ]] || run_dir=/var/run
mount -t tmpfs -o mode=0755 rusteron-rootless "$run_dir" ||
    die "cannot mount a private tmpfs on $run_dir (Docker: --security-opt apparmor=unconfined)"
ip link set lo up || die "cannot bring lo up in the namespace"
if ! err="$(ip tuntap add dev rlprobe0 mode tap 2>&1)"; then
    die "net_tap is unusable here: cannot create a tap in the namespace (${err:-no message}); the af_packet fallback is not built (the transport needs checksum offloads af_packet lacks)"
fi
ip link del rlprobe0 || die "cannot delete the probe tap"
log "namespace: /dev/net/tun, a tap and a private $run_dir tmpfs: ok ($(uname -r), $(uname -m))"
if ((SELF_TEST)); then
    log "self-test ok: net_tap usable without root"
    exit 0
fi
export RUSTERON_VDEV_NO_HUGE=1
exec bash "$(dirname "$0")/test-dpdk-vdev.sh" "$@"
