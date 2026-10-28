#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The media driver pod's entrypoint, run by ergon-cpus, which has already
# turned the specs into CPU numbers:
#
#   ergon-cpus --env AERON_SENDER_CPU_AFFINITY=isolated:0 \
#              --env AERON_RECEIVER_CPU_AFFINITY=isolated:1 \
#              --env RUSTERON_DPDK_HOUSEKEEPING_CPU=shared:0 \
#              -- driver-entrypoint.sh media_driver
#
# Fails closed (exit 1, one line saying why) when:
#   - ERGON_ISOLATED_CPUS or ERGON_SHARED_CPUS is unset (the plugin did not run);
#   - there are fewer than 2 isolated CPUs;
#   - the sender or receiver affinity is unset, not an isolated CPU, or both
#     are on one core (thread_siblings_list);
#   - this process's Cpus_allowed_list is not the shared CPUs (the plugin's
#     hook did not pin it);
#   - the base page size is not 4096 (only 4 KiB-page kernels are qualified).
# It never reads /sys/fs/cgroup: a privileged container has no cgroup
# namespace, so that is the host's root.
#
# Then it sets the driver's defaults, keeping any value already set
# (DEDICATED threading, busy-spinning sender and receiver, the wildcard
# port ranges, MTU 1408, the dpdk-ena transport, the EAL's one lcore on the
# housekeeping CPU), leaves the conductor's and the native resource agent's
# affinity unset (they start on the shared CPUs) and execs the command.
#
# LL_VDEV_BRIDGE=1 (the vdev overlays): runs the command in the background,
# waits for the tap PMD's dtap0 and dtap1, bridges them on br0
# (10.9.0.254/24) as scripts/test-dpdk-vdev.sh does, forwards TERM and INT,
# and waits on the driver.
#
# Fixtures (scripts/test-entrypoint.sh): LL_PROC_STATUS, LL_SYS_CPU (a
# /sys/devices/system/cpu tree) and LL_PAGESIZE replace the live reads;
# RUSTERON_ENTRYPOINT_PRINT_ONLY=1 prints the plan and the sorted
# AERON_*/RUSTERON_* environment, then exits 0.
set -euo pipefail

PROC_STATUS="${LL_PROC_STATUS:-/proc/self/status}"
SYS_CPU="${LL_SYS_CPU:-/sys/devices/system/cpu}"

die() { echo "ll-driver: $*" >&2; exit 1; }

# "0-2,5" -> "0 1 2 5"
expand() {
  local part out=() a b c
  IFS=, read -ra parts <<<"$1"
  for part in "${parts[@]}"; do
    [[ -z "$part" ]] && continue
    a=${part%-*}; b=${part#*-}
    for ((c = a; c <= b; c++)); do out+=("$c"); done
  done
  echo "${out[@]}"
}

[[ -n "${ERGON_ISOLATED_CPUS:-}" ]] || die "ERGON_ISOLATED_CPUS is unset: the ergon-shared-cpus plugin did not run (request ergon.io/shared-millicpus in an allowlisted namespace)"
[[ -n "${ERGON_SHARED_CPUS:-}" ]] || die "ERGON_SHARED_CPUS is unset: the ergon-shared-cpus plugin did not run"
read -ra ISOLATED <<<"$(expand "$ERGON_ISOLATED_CPUS")"
(( ${#ISOLATED[@]} >= 2 )) || die "ERGON_ISOLATED_CPUS=$ERGON_ISOLATED_CPUS: need 2 isolated CPUs (sender, receiver); request cpu: \"2\""

S=${AERON_SENDER_CPU_AFFINITY:-}
R=${AERON_RECEIVER_CPU_AFFINITY:-}
[[ "$S" =~ ^[0-9]+$ && "$R" =~ ^[0-9]+$ ]] || die "AERON_SENDER_CPU_AFFINITY=$S AERON_RECEIVER_CPU_AFFINITY=$R: run under ergon-cpus --env AERON_SENDER_CPU_AFFINITY=isolated:0 --env AERON_RECEIVER_CPU_AFFINITY=isolated:1"
for cpu in "$S" "$R"; do
  [[ " ${ISOLATED[*]} " == *" $cpu "* ]] || die "CPU $cpu is not in ERGON_ISOLATED_CPUS=$ERGON_ISOLATED_CPUS"
done
[[ "$S" != "$R" ]] || die "the sender and the receiver are both on CPU $S"
siblings=$(cat "$SYS_CPU/cpu$S/topology/thread_siblings_list" 2>/dev/null || echo "$S")
[[ " $(expand "$siblings") " != *" $R "* ]] || die "the sender (CPU $S) and the receiver (CPU $R) share a core ($siblings): turn SMT off or use full-pcpus-only with two cores"

allowed=$(sed -n 's/^Cpus_allowed_list:[[:space:]]*//p' "$PROC_STATUS")
[[ "$(expand "$allowed")" == "$(expand "$ERGON_SHARED_CPUS")" ]] || die "this process may run on $allowed, not the shared CPUs $ERGON_SHARED_CPUS: the plugin's hook did not pin it (defaultAffinity must be shared)"

pagesize=${LL_PAGESIZE:-$(getconf PAGESIZE)}
[[ "$pagesize" == 4096 ]] || die "base page size $pagesize: only 4 KiB-page kernels are qualified (md-dpdk/deploy/aws-eks/README.md)"

HK=${RUSTERON_DPDK_HOUSEKEEPING_CPU:-${ERGON_SHARED_CPUS%%[,-]*}}
export RUSTERON_DPDK_HOUSEKEEPING_CPU="$HK"
export AERON_THREADING_MODE="${AERON_THREADING_MODE:-DEDICATED}"
export AERON_SENDER_IDLE_STRATEGY="${AERON_SENDER_IDLE_STRATEGY:-noop}"
export AERON_RECEIVER_IDLE_STRATEGY="${AERON_RECEIVER_IDLE_STRATEGY:-noop}"
# "<low> <high>": Aeron reads the second number with strtoll, so a dash
# would make it negative.
export AERON_SENDER_WILDCARD_PORT_RANGE="${AERON_SENDER_WILDCARD_PORT_RANGE:-20000 20999}"
export AERON_RECEIVER_WILDCARD_PORT_RANGE="${AERON_RECEIVER_WILDCARD_PORT_RANGE:-21000 21999}"
export AERON_MTU_LENGTH="${AERON_MTU_LENGTH:-1408}"
export RUSTERON_MEDIA_DRIVER_TRANSPORT="${RUSTERON_MEDIA_DRIVER_TRANSPORT:-dpdk-ena}"
export RUSTERON_DPDK_EAL_ARGS="${RUSTERON_DPDK_EAL_ARGS:--l $HK${LL_EAL_EXTRA:+ $LL_EAL_EXTRA}}"
export RUSTERON_DPDK_FILE_PREFIX="${RUSTERON_DPDK_FILE_PREFIX:-ll-${HOSTNAME:-driver}}"
[[ -z "${AERON_CONDUCTOR_CPU_AFFINITY:-}" ]] || die "AERON_CONDUCTOR_CPU_AFFINITY is set: leave it unset, the conductor belongs on the shared CPUs"
[[ "${AERON_THREAD_AFFINITY_USE_CPUSET_OFFSETS:-false}" == false ]] || die "AERON_THREAD_AFFINITY_USE_CPUSET_OFFSETS must stay false"

echo "ll-driver: isolated=$ERGON_ISOLATED_CPUS shared=$ERGON_SHARED_CPUS sender=$S receiver=$R housekeeping=$HK allowed=$allowed pagesize=$pagesize transport=$RUSTERON_MEDIA_DRIVER_TRANSPORT vdev_bridge=${LL_VDEV_BRIDGE:-0}"
if [[ "${RUSTERON_ENTRYPOINT_PRINT_ONLY:-0}" == 1 ]]; then
  env | grep -E '^(AERON|RUSTERON)_' | sort
  exit 0
fi
(( $# > 0 )) || die "no command: driver-entrypoint.sh media_driver"

if [[ "${LL_VDEV_BRIDGE:-0}" != 1 ]]; then
  exec "$@"
fi

# ---- vdev: the driver's two tap ports on one Linux bridge ------------------
"$@" &
driver=$!
trap 'kill -TERM "$driver" 2>/dev/null' TERM INT
for _ in $(seq 1 120); do
  ip link show dtap0 >/dev/null 2>&1 && ip link show dtap1 >/dev/null 2>&1 && break
  kill -0 "$driver" 2>/dev/null || { wait "$driver"; die "the driver exited before its taps appeared"; }
  sleep 0.25
done
ip link show dtap1 >/dev/null 2>&1 || die "dtap0/dtap1 did not appear within 30 s"
if ! ip link show br0 >/dev/null 2>&1; then
  ip link add br0 type bridge
  ip link set br0 address 02:00:00:00:00:fe
  ip addr add 10.9.0.254/24 dev br0
  ip link set br0 up
fi
# A Linux bridge absorbs unicast to a member port's own MAC, and the tap PMD
# copies its port MAC onto the kernel side: give each kernel side another
# MAC and turn promiscuous mode on, so frames to the DPDK port cross the
# bridge (scripts/test-dpdk-vdev.sh, attach_tap).
for tap in dtap0 dtap1; do
  ip link set "$tap" address "02:00:00:00:01:0${tap#dtap}"
  ip link set "$tap" promisc on
  ip link set "$tap" master br0
  ip link set "$tap" up
done
echo "ll-driver: vdev bridge br0 10.9.0.254/24 with dtap0 dtap1"
set +e
wait "$driver"
rc=$?
# a signal interrupts the first wait: wait for the driver's own exit
kill -0 "$driver" 2>/dev/null && { wait "$driver"; rc=$?; }
exit "$rc"
