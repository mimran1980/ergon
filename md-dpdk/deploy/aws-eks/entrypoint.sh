#!/usr/bin/env bash
# Container entrypoint for the rusteron-media-driver DPDK ENA image. Reads the
# pod's cpuset (kubelet CPU Manager `static`), reduces it to one
# logical CPU per physical core (SMT siblings share execution resources and
# would wreck latency), then gives each role a core of its own, the layout the
# VM lab (benches/cloud/vm.sh) measures:
#
#   housekeeping  the Aeron conductor (not busy-spinning), the DPDK EAL main
#                 lcore, the transport's housekeeping and local-I/O threads and
#                 every other thread that is not an agent
#   sender        the Aeron sender, busy-spinning (`noop`)
#   receiver      the Aeron receiver, busy-spinning (`noop`)
#   spare         any further cores: printed and left unused by the driver (a
#                 client in the same pod, e.g. rtt, can take one)
#
# Fails before starting Aeron if fewer than three physical cores are present.
# Then execs the passed command (default: media_driver) confined to the
# housekeeping CPU; the agents pin themselves from AERON_*_CPU_AFFINITY.
# Every variable the caller already set wins.
#
# The RUSTERON_DPDK_* PCI/CIDR/gateway values are injected by the device
# plugin at allocation time; the media_driver binary itself fails fast (and
# nonzero, never falling back to the socket driver) if any are missing, so the
# entrypoint does not duplicate that validation.
#
# For the fixture tests (node/test/test-bootstrap.sh): RUSTERON_PROC_STATUS
# and RUSTERON_CPUINFO replace /proc/self/status and /proc/cpuinfo,
# RUSTERON_CPUSET_FILE (a file holding a cpuset list such as "1-4,9-12")
# replaces the cpuset lookup altogether, and RUSTERON_ENTRYPOINT_PRINT_ONLY=1
# prints the startup line and the sorted AERON_*/RUSTERON_* environment, then
# exits instead of exec'ing.
set -euo pipefail

REQUIRED_CPUS=3   # housekeeping + sender + receiver
PROC_STATUS="${RUSTERON_PROC_STATUS:-/proc/self/status}"
CPUINFO="${RUSTERON_CPUINFO:-/proc/cpuinfo}"

# --- the pod's cpuset --------------------------------------------------------
# Cpus_allowed_list is the CPU set the kernel lets this process run on: the
# container cgroup's cpuset, which is exactly what `taskset` and the agents'
# affinity calls are checked against. /sys/fs/cgroup/cpuset.cpus.effective is
# not used: containerd gives a privileged container (this pod) no cgroup
# namespace, so that path is the host's cgroup root and lists every online
# CPU, kubelet's reserved ones included.
read_cpuset() {
    if [[ -n "${RUSTERON_CPUSET_FILE:-}" ]]; then
        cat "$RUSTERON_CPUSET_FILE"
        return
    fi
    sed -n 's/^Cpus_allowed_list:[[:space:]]*//p' "$PROC_STATUS"
}

# Expand "0-3,7" into one CPU id per line, ascending.
expand_cpuset() {
    local range cpu lo hi
    local -a ranges
    IFS=',' read -r -a ranges <<<"$1"
    for range in "${ranges[@]}"; do
        if [[ "$range" == *-* ]]; then
            lo="${range%-*}"; hi="${range#*-}"
            for ((cpu = lo; cpu <= hi; cpu++)); do echo "$cpu"; done
        else
            echo "$range"
        fi
    done | sort -n
}

CPUSET="$(read_cpuset || true)"
if [[ -z "$CPUSET" ]]; then
    echo "rusteron entrypoint: cannot read the allowed CPU list (Cpus_allowed_list in $PROC_STATUS)" >&2
    exit 1
fi
mapfile -t CPUS < <(expand_cpuset "$CPUSET")

# --- SMT-sibling exclusion ---------------------------------------------------
# kubelet CPU Manager static hands guaranteed pods a cpuset that includes both
# threads of each core (full-pcpus-only). Reduce it to one logical CPU per
# physical core (the lowest sibling) before assigning roles. /proc/cpuinfo is
# used because the cpu-topology sysfs tree is often masked inside containers.
declare -A CORE_OF=()                       # logical cpu -> "socket:core"
if [[ -r "$CPUINFO" ]]; then
    cpu="" pkg=""
    while IFS= read -r line; do
        case "$line" in
            "processor"*) cpu="${line#*: }" ;;
            "physical id"*) pkg="${line#*: }" ;;
            "core id"*) CORE_OF[$cpu]="$pkg:${line#*: }" ;;
        esac
    done < "$CPUINFO"
fi

declare -A FIRST_OF_CORE=()
for cpu in "${CPUS[@]}"; do
    key="${CORE_OF[$cpu]:-$cpu}"            # no SMT info -> treat as its own core
    if [[ -z "${FIRST_OF_CORE[$key]:-}" ]] || (( cpu < FIRST_OF_CORE[$key] )); then
        FIRST_OF_CORE[$key]="$cpu"
    fi
done
mapfile -t CPUS < <(for key in "${!FIRST_OF_CORE[@]}"; do echo "${FIRST_OF_CORE[$key]}"; done | sort -n)

if ((${#CPUS[@]} < REQUIRED_CPUS)); then
    echo "rusteron entrypoint: cpuset ${CPUSET} yields ${#CPUS[@]} distinct physical core(s) (SMT siblings excluded), need >= $REQUIRED_CPUS (housekeeping + sender + receiver). Raise the DaemonSet cpu request (SMT is off on latency nodes, so one CPU is one core; on a 2-thread/core instance 6 vCPUs are 3 cores)." >&2
    exit 1
fi

HK="${CPUS[0]}"
SENDER_CPU="${CPUS[1]}"
RECEIVER_CPU="${CPUS[2]}"
SPARE="$(IFS=,; echo "${CPUS[*]:3}")"

# --- Aeron dedicated-thread driver settings ---------------------------------
# The conductor shares the housekeeping core and keeps Aeron's default
# (backoff) idle strategy; only the sender and receiver busy-spin, each on its
# own core. The wildcard ranges are "<low> <high>": Aeron parses the second
# number with strtoll, so "20000-20999" would read as -20999 and fail driver
# context creation.
export AERON_THREADING_MODE="${AERON_THREADING_MODE:-DEDICATED}"
export AERON_SENDER_IDLE_STRATEGY="${AERON_SENDER_IDLE_STRATEGY:-noop}"
export AERON_RECEIVER_IDLE_STRATEGY="${AERON_RECEIVER_IDLE_STRATEGY:-noop}"
export AERON_CONDUCTOR_CPU_AFFINITY="${AERON_CONDUCTOR_CPU_AFFINITY:-$HK}"
export AERON_SENDER_CPU_AFFINITY="${AERON_SENDER_CPU_AFFINITY:-$SENDER_CPU}"
export AERON_RECEIVER_CPU_AFFINITY="${AERON_RECEIVER_CPU_AFFINITY:-$RECEIVER_CPU}"
export AERON_SENDER_WILDCARD_PORT_RANGE="${AERON_SENDER_WILDCARD_PORT_RANGE:-20000 20999}"
export AERON_RECEIVER_WILDCARD_PORT_RANGE="${AERON_RECEIVER_WILDCARD_PORT_RANGE:-21000 21999}"
export AERON_MTU_LENGTH="${AERON_MTU_LENGTH:-1408}"

# --- DPDK selector, housekeeping and EAL lcore ------------------------------
# `-l <hk>` makes the housekeeping CPU the EAL's only lcore, so no EAL thread
# lands on an agent core; the transport's housekeeping and local-I/O threads
# are pinned there too.
export RUSTERON_MEDIA_DRIVER_TRANSPORT="${RUSTERON_MEDIA_DRIVER_TRANSPORT:-dpdk-ena}"
export RUSTERON_DPDK_HOUSEKEEPING_CPU="${RUSTERON_DPDK_HOUSEKEEPING_CPU:-$HK}"
export RUSTERON_DPDK_EAL_ARGS="${RUSTERON_DPDK_EAL_ARGS:--l $HK}"
if [[ -z "${RUSTERON_DPDK_FILE_PREFIX:-}" ]]; then
    export RUSTERON_DPDK_FILE_PREFIX="rusteron-dpdk-${HOSTNAME:-pod}"
fi

echo "rusteron entrypoint: transport=${RUSTERON_MEDIA_DRIVER_TRANSPORT} cpuset=${CPUSET} housekeeping=${HK} conductor=${AERON_CONDUCTOR_CPU_AFFINITY} sender=${AERON_SENDER_CPU_AFFINITY} receiver=${AERON_RECEIVER_CPU_AFFINITY} spare=${SPARE}"
if [[ "${RUSTERON_ENTRYPOINT_PRINT_ONLY:-0}" == 1 ]]; then
    env | grep -E '^(AERON|RUSTERON)_' | sort
    exit 0
fi
# Every thread that is not an agent (signal handler, Aeron's, the EAL's)
# starts on, and stays on, the housekeeping CPU.
exec taskset -c "$HK" "$@"
