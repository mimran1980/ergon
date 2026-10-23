#!/usr/bin/env bash
# ergon-md-dpdk DPDK vdev interoperability harness (plan §11.2).
#
# Runs the dpdk-harness scenarios over DPDK tap devices on a Linux bridge,
# exercising DPDK-to-DPDK, DPDK-to-default-UDP in both directions, reconnect,
# multiple endpoints, loss recovery, restart cleanup, ICMP echo, a peer whose
# MAC moves mid-stream, a gateway that never answers ARP, the largest MTU
# the port takes, and the lab's raw floor client (floor: the transport's
# bindings with no Aeron driver or client) round tripping. The new scenarios assert on the reports' counters and
# logs only.
#
# This requires a Linux x86_64 or aarch64 host and:
#   - root (the script creates bridges/taps and mounts hugetlbfs), or the
#     root of a user namespace: scripts/test-dpdk-vdev-rootless.sh runs this
#     unprivileged, with RUSTERON_VDEV_NO_HUGE=1 (EAL --no-huge, no hugetlbfs)
#   - /dev/net/tun available (modprobe tun), ping (iputils), tc (iproute2)
#   - libdpdk dev packages (matching the DPDK version the build links)
#   - a Rust toolchain (cargo)
#
# The DPDK tap PMD assigns the host interface names dynamically (dtapN), so
# this script discovers the interfaces each process creates rather than
# assuming a fixed name.
#
# Usage:
#   sudo scripts/test-dpdk-vdev.sh [--scenario NAME] [--msgs N]

set -euo pipefail
# Surface silent `set -e` exits: a bare non-zero command aborts the script and
# otherwise only the EXIT-trap cleanup would print.
trap 'echo "[vdev] FAILED: ${BASH_COMMAND} (exit $?)" >&2' ERR
cd "$(dirname "$0")/.."

BRIDGE=rusteron-vdev0
BRIDGE_IP=10.9.0.5          # the default-UDP side binds here
BRIDGE_MAC=02:00:00:00:00:05
DPDK_SUBNET=10.9.0.0/24
GW=10.9.0.254
HUGEDIR=/dev/hugepages
REPORT_DIR="${RUSTERON_REPORT_DIR:-/tmp/rusteron-vdev-reports}"
WORK_DIR="${RUSTERON_WORK_DIR:-/tmp/rusteron-vdev}"
CARGO="${CARGO:-cargo}"

SCENARIO=""
MSGS=1000
CLEANUP_ON_EXIT=1

log()  { echo "[vdev] $*" >&2; }
die()  { echo "[vdev] ERROR: $*" >&2; cleanup; exit 1; }

usage() {
    sed -n '2,20p' "$0"
    echo "  --scenario NAME   run only NAME (default: all)"
    echo "  --msgs N          messages per run (default: 1000)"
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --scenario) SCENARIO="$2"; shift 2 ;;
        --msgs)     MSGS="$2"; shift 2 ;;
        -h|--help)  usage ;;
        *) die "unknown argument $1" ;;
    esac
done

ALL_SCENARIOS="bidirectional_unicast dpdk_to_udp udp_to_dpdk reconnect multi_endpoint loss_recovery restart icmp_ping peer_mac_move silent_gateway largest_mtu raw_floor"
# RUSTERON_VDEV_NO_HUGE=1: EAL without hugepages (--no-huge, this much memory
# in MiB, no PCI scan), for an unprivileged run (test-dpdk-vdev-rootless.sh).
NO_HUGE="${RUSTERON_VDEV_NO_HUGE:-0}"
NO_HUGE_MB="${RUSTERON_VDEV_NO_HUGE_MB:-512}"
# The largest Aeron MTU largest_mtu sends: the transport's own ceiling, 8960
# (a 9001-byte jumbo L3 MTU less IPv4 and UDP), which the tap PMD takes; one
# step up (+32) must be refused at start.
LARGEST_MTU="${RUSTERON_VDEV_LARGEST_MTU:-8960}"

# ---------------------------------------------------------------------------
# Prerequisites
# ---------------------------------------------------------------------------

[[ "$(id -u)" -eq 0 ]] || die "must run as root (sudo scripts/test-dpdk-vdev.sh), or unprivileged through scripts/test-dpdk-vdev-rootless.sh"
[[ -e /dev/net/tun ]] || die "/dev/net/tun missing — modprobe tun"
command -v ping >/dev/null || die "ping not found (iputils-ping): icmp_ping needs it"
[[ -x "${RUSTERON_HARNESS_BIN:-}" ]] || command -v "$CARGO" >/dev/null || die "cargo not found"
command -v pkg-config >/dev/null || die "pkg-config not found"
pkg-config --exists libdpdk || die "libdpdk dev package not found (pkg-config libdpdk)"
pkg-config --exists libdpdk >/dev/null || true

setup_hugepages() {
    if ((NO_HUGE)); then
        log "no hugepages: EAL --no-huge -m $NO_HUGE_MB (RUSTERON_VDEV_NO_HUGE=1)"
        return
    fi
    if ! mountpoint -q "$HUGEDIR"; then
        mkdir -p "$HUGEDIR"
        mount -t hugetlbfs hugetlbfs "$HUGEDIR" || die "failed to mount hugetlbfs at $HUGEDIR"
    fi
    local pages; pages=$(sysctl -n vm.nr_hugepages 2>/dev/null || echo 0)
    if (( pages < 256 )); then
        log "raising vm.nr_hugepages 0 -> 256"
        sysctl -w vm.nr_hugepages=256 >/dev/null
    fi
}

build_harness() {
    if [[ -x "${RUSTERON_HARNESS_BIN:-}" ]]; then
        HARNESS_BIN="$RUSTERON_HARNESS_BIN"
        log "using prebuilt harness $HARNESS_BIN"
        return
    fi
    log "building dpdk-harness (--features dpdk)"
    "$CARGO" build -p ergon-md-dpdk-harness --features dpdk
    HARNESS_BIN="${CARGO_TARGET_DIR:-target}/debug/dpdk-harness"
    [[ -x "$HARNESS_BIN" ]] || die "harness build did not produce $HARNESS_BIN"
}

# ---------------------------------------------------------------------------
# Bridge + tap discovery
# ---------------------------------------------------------------------------

snapshot_taps() {
    # `grep` exits 1 when no tap exists yet; under `set -e`/`pipefail` that
    # aborts the script on the very first (pre-launch) snapshot. Force 0.
    ip -o link show 2>/dev/null | awk -F': ' '{print $2}' | grep -E '^(dtap|tap)[0-9]+$' | sort -u || true
}

# new_taps <snapshot-file>: interfaces in snapshot-file's "before" list that
# have appeared since (write current list into $TAPS for the caller).
discover_taps() {
    local before=$1 pid=$2 want=$3 timeout=${4:-25} after
    local deadline=$((SECONDS + timeout))
    while (( SECONDS < deadline )); do
        if ! kill -0 "$pid" 2>/dev/null; then
            return 1   # process died before creating its taps
        fi
        after=$(snapshot_taps)
        # `before` is a string of tap names, not a file path — sort it via
        # here-string, or `sort` treats it as a filename and errors.
        TAPS=$(comm -13 <(sort <<<"$before") <(echo "$after"))
        local n; n=$(echo "$TAPS" | grep -c . || true)
        if (( n >= want )); then
            echo "$TAPS" | head -n "$want"
            return 0
        fi
        sleep 0.5
    done
    return 1
}

bridge_up() {
    if ! ip link show "$BRIDGE" >/dev/null 2>&1; then
        ip link add "$BRIDGE" type bridge
        # A fixed MAC: a bridge without one takes its lowest port's, so its
        # MAC would change as taps join and leave.
        ip link set "$BRIDGE" address "$BRIDGE_MAC"
        ip addr add "$BRIDGE_IP/24" dev "$BRIDGE"
        ip link set "$BRIDGE" up
        # Allow the bridge itself to answer ARP for $BRIDGE_IP and L3-terminate.
        echo 0 > /proc/sys/net/bridge/bridge-nf-call-iptables 2>/dev/null || true
    fi
    log "bridge $BRIDGE up at $BRIDGE_IP/24"
}

# attach_tap <iface> [ip]: add to the bridge so unicast to the DPDK port MAC
# crosses it. The DPDK tap PMD copies its (random) port MAC onto the kernel
# iface (rte_eth_tap.c SIOCSIFHWADDR at tap_create), and a Linux bridge records
# each member port's MAC as a LOCAL FDB entry and absorbs unicast to it —
# delivers it to the host stack instead of forwarding it back to the tap. So
# the DPDK ARP replies and UDP frames (unicast to that MAC) never arrive. Re-point
# the kernel iface at a distinct MAC (derived from the role IP) and enable
# promiscuous: the bridge then forwards/floods unicast to the DPDK MAC and the
# tap accepts it, making the vdev fabric deliver unicast like a real ENA switch.
# No IP is assigned — the DPDK processes own their IPs in userspace, and a host
# address here would let the kernel ARP-respond for them and race the DPDK replies.
attach_tap() {
    local iface=$1 ip=$2
    ip link set "$iface" up
    if [[ -n "$ip" ]]; then
        ip link set "$iface" address "$(printf '02:00:00:00:00:%02x' "${ip##*.}")"
    fi
    ip link set "$iface" promisc on
    ip link set "$iface" master "$BRIDGE"
    log "tap $iface -> bridge${ip:+ @ $ip} promisc, distinct MAC"
    # Both of a role's taps are bridged once its receiver's is (sender first):
    # let it start sending (start_file says why).
    case "$ip" in
        10.9.0.3) touch "$(start_file primary)" ;;
        10.9.0.4) touch "$(start_file secondary)" ;;
    esac
}

# start_file <role>: a DPDK role brings its ports up, then waits for this file
# before it opens a publication (RUSTERON_HARNESS_START_FILE). Until its taps
# are bridged, the kernel side of a tap is an ordinary interface of this
# network namespace, and the kernel answers ARP there for any local address
# (the bridge's 10.9.0.5) with the tap's own MAC, which attach_tap then
# changes: the DPDK port would send to that dead MAC until its 30 s ARP
# refresh (dpdk_to_udp took 31 s instead of 2).
start_file() { echo "$WORK_DIR/start-$1"; }

# ---------------------------------------------------------------------------
# Environment for a harness process
# ---------------------------------------------------------------------------

# The CPUs of one CPU's core: its SMT siblings, itself included.
core_of() {
    local list
    list=$(cat "/sys/devices/system/cpu/cpu$1/topology/thread_siblings_list" 2>/dev/null || echo "$1")
    expand_cpus "$list"
}

# "0-2,5" -> "0 1 2 5"
expand_cpus() {
    local part a b out=()
    IFS=, read -ra parts <<<"$1"
    for part in "${parts[@]}"; do
        a=${part%-*}; b=${part#*-}
        for ((c = a; c <= b; c++)); do out+=("$c"); done
    done
    echo "${out[@]}"
}

# The transport refuses DEDICATED when the EAL's main lcore (its interrupt and
# alarm threads) shares a core with the busy-spinning sender or receiver, SMT
# siblings included. Plan from this host's topology: the main lcore on the
# first allowed CPU, the sender and the receiver on the last two CPUs off its
# core (siblings of each other on a 2-core runner, which is allowed).
# RUSTERON_HARNESS_{SENDER,RECEIVER}_CPU and RUSTERON_DPDK_EAL_ARGS override.
plan_cpus() {
    local allowed main off=() c
    allowed=$(expand_cpus "$(awk '/^Cpus_allowed_list:/ {print $2}' /proc/self/status)")
    read -ra cpus <<<"$allowed"
    main=${cpus[0]}
    local main_core
    main_core=" $(core_of "$main") "
    for c in "${cpus[@]}"; do
        [[ "$main_core" == *" $c "* ]] || off+=("$c")
    done
    if (( ${#off[@]} < 2 )); then
        die "need two CPUs off the EAL main lcore's core (CPU $main, core $main_core) for the sender and the receiver; allowed: $allowed"
    fi
    PLAN_MAIN_LCORE=$main
    PLAN_SENDER_CPU=${off[${#off[@]}-1]}
    PLAN_RECEIVER_CPU=${off[${#off[@]}-2]}
    log "CPU plan: EAL main lcore $PLAN_MAIN_LCORE, sender $PLAN_SENDER_CPU, receiver $PLAN_RECEIVER_CPU (allowed $allowed)"
}

dpdk_env() {
    local role=$1 suffix s_ip r_ip
    suffix=rusteron-vdev-p
    [[ "$role" == secondary ]] && suffix=rusteron-vdev-s
    # Sender/receiver tap IPs per role (see the attach_tap calls in
    # scenario_bidi): primary 10.9.0.1/.3, secondary 10.9.0.2/.4.
    if [[ "$role" == secondary ]]; then
        s_ip=10.9.0.2; r_ip=10.9.0.4
    else
        s_ip=10.9.0.1; r_ip=10.9.0.3
    fi
    echo "RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena"
    echo "RUSTERON_DPDK_FILE_PREFIX=$suffix"
    echo "RUSTERON_DPDK_TEST_VDEV=1"
    # The kernel-UDP peer is the bridge, a kernel interface on this host:
    # reach it over the DPDK tap (the wire) so these scenarios test DPDK
    # interop, not the transport's host-local kernel path.
    echo "RUSTERON_DPDK_HOST_LOCAL=loopback"
    echo "RUSTERON_DPDK_SENDER_PCI=net_tap0"
    echo "RUSTERON_DPDK_SENDER_IPV4_CIDR=$s_ip/24"
    echo "RUSTERON_DPDK_SENDER_GATEWAY=$GW"
    echo "RUSTERON_DPDK_RECEIVER_PCI=net_tap1"
    echo "RUSTERON_DPDK_RECEIVER_IPV4_CIDR=$r_ip/24"
    echo "RUSTERON_DPDK_RECEIVER_GATEWAY=$GW"
    echo "RUSTERON_DPDK_HUGE_DIR=$HUGEDIR"
    echo "RUSTERON_DPDK_RX_DESCRIPTORS=1024"
    echo "RUSTERON_DPDK_TX_DESCRIPTORS=1024"
    # 8192, not the 65536 default: PA-mode hugepages fragment after the first
    # pool, forcing the second into a memzone split — harmless with a short
    # pool name, but a smaller footprint keeps both pools in a single memzone.
    # 8192 covers the 1000-msg scenarios with huge headroom (matches the
    # test-fixture value).
    echo "RUSTERON_DPDK_MBUFS_PER_PORT=8192"
    echo "RUSTERON_DPDK_MEMPOOL_CACHE=256"
    echo "RUSTERON_DPDK_BURST_SIZE=32"
    echo "RUSTERON_DPDK_MAX_AERON_MTU=1408"
    # One assignment a line: run_one passes each line to env whole, so a value
    # may hold spaces (the EAL arguments do).
    local eal="${RUSTERON_DPDK_EAL_ARGS:--l$PLAN_MAIN_LCORE}"
    ((NO_HUGE)) && eal+=" --no-huge -m $NO_HUGE_MB --no-pci"
    echo "RUSTERON_DPDK_EAL_ARGS=$eal"
}

harness_common_env() {
    echo "RUSTERON_HARNESS_MSGS=$MSGS"
    echo "RUSTERON_HARNESS_PAYLOAD=130"
    echo "RUSTERON_HARNESS_STREAM=32931"
    echo "RUSTERON_HARNESS_TIMEOUT_SECS=40"
    echo "RUSTERON_HARNESS_MTU=1408"
    echo "RUSTERON_HARNESS_SENDER_CPU=${RUSTERON_HARNESS_SENDER_CPU:-$PLAN_SENDER_CPU}"
    echo "RUSTERON_HARNESS_RECEIVER_CPU=${RUSTERON_HARNESS_RECEIVER_CPU:-$PLAN_RECEIVER_CPU}"
}

# run_one <role> <scenario> <run-id> <envfile> [extra args...]
run_one() {
    local role=$1 scenario=$2 run_id=$3 envfile=$4
    shift 4
    local report="$REPORT_DIR/$scenario-$role-$run_id.json"
    # A report left by an earlier run would pass for this one's.
    rm -f "$report"
    log "launching $role ($scenario, run $run_id)"
    local vars
    mapfile -t vars < "$envfile"
    rm -f "$(start_file "$role")"
    if grep -qx 'RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena' "$envfile"; then
        vars+=("RUSTERON_HARNESS_START_FILE=$(start_file "$role")")
    fi
    env "${vars[@]}" "$HARNESS_BIN" --role "$role" --scenario "$scenario" --report "$report" "$@" \
        >"$WORK_DIR/$scenario-$role-$run_id.log" 2>&1 &
    echo "$report $!"
}

wait_report() {
    local report=$1 pid=$2
    local deadline=$((SECONDS + 45))
    while (( SECONDS < deadline )); do
        if [[ -s "$report" ]]; then
            return 0
        fi
        if ! kill -0 "$pid" 2>/dev/null; then
            log "process for $report exited before writing a report"
            return 1
        fi
        sleep 0.5
    done
    return 1
}

report_field() {
    local report=$1 field=$2
    grep -o "\"$field\": [^,}]*" "$report" | head -1 | cut -d' ' -f2
}

assert_report() {
    local report=$1 expect_received=$2 label=$3
    local ok; ok=$(report_field "$report" ok)
    local sent; sent=$(report_field "$report" sent)
    local received; received=$(report_field "$report" received)
    local duration; duration=$(report_field "$report" duration_ms)
    local backpressure; backpressure=$(report_field "$report" backpressure_ops)
    log "assert $label: ok=$ok sent=$sent received=$received duration_ms=$duration backpressure_ops=$backpressure"
    [[ "$ok" == true ]] || die "$label: report not ok"
    [[ "$received" == "$expect_received" ]] || die "$label: received $received != $expect_received"
    assert_clock "$report" "$label"
    echo "$sent"
}

# assert_clock <report> <label>: an embedded DPDK transport reads the CPU
# counter at the rate the real EAL gives it (rte_get_tsc_hz) or, where that is
# more than 100 ppm off the rate the transport measures at start, at the
# measured rate. On aarch64 it is the generic timer's own (CNTFRQ_EL0, as the
# harness reads it). A fallback to CLOCK_MONOTONIC fails the run and prints the
# transport's reason: a host whose kernel clock source or CPU rules the counter
# out says so there; otherwise the real EAL path is broken.
assert_clock() {
    local report=$1 label=$2
    [[ "$(report_field "$report" transport)" == '"dpdk"' ]] || return 0
    local clock; clock=$(report_field "$report" clock)
    local hz; hz=$(report_field "$report" clock_hz)
    local cntfrq; cntfrq=$(report_field "$report" cntfrq_hz)
    local rate; rate=$(report_field "$report" clock_rate)
    local eal_hz; eal_hz=$(report_field "$report" clock_eal_hz)
    local measured_hz; measured_hz=$(report_field "$report" clock_measured_hz)
    local reason; reason=$(grep -o '"clock_reason": "[^"]*"' "$report" | head -1 | cut -d'"' -f4)
    log "assert $label: clock=$clock clock_hz=$hz rate=$rate eal_hz=$eal_hz measured_hz=$measured_hz cntfrq_hz=$cntfrq ${reason:+reason=$reason}"
    [[ "$clock" == '"counter"' ]] || die "$label: the transport reads CLOCK_MONOTONIC, not the counter: $reason"
    (( hz > 0 )) || die "$label: the counter's rate is $hz"
    if [[ "$(uname -m)" == aarch64 ]]; then
        [[ "$hz" == "$cntfrq" ]] || die "$label: the counter's rate $hz Hz is not CNTFRQ_EL0's $cntfrq Hz"
    fi
}

# ---------------------------------------------------------------------------
# Scenario runners
# ---------------------------------------------------------------------------

# scenario_bidi <scenario> <primary-transport> <secondary-transport>
#   transport: dpdk | udp
scenario_bidi() {
    local scenario=$1 ptx=$2 stx=$3
    local penv=$WORK_DIR/$scenario-p.env senv=$WORK_DIR/$scenario-s.env
    : > "$penv"; : > "$senv"

    harness_common_env >> "$penv"; harness_common_env >> "$senv"

    # DPDK roles own their IPs in userspace (10.9.0.1/.3 primary, 10.9.0.2/.4
    # secondary); a UDP role terminates L3 on the bridge itself ($BRIDGE_IP), so
    # each role's DESTINATIONS must target the peer's actual receiver:
    #   dpdk->dpdk : 10.9.0.4:40102 / 10.9.0.3:40102
    #   dpdk->udp  : primary -> $BRIDGE_IP:40102, secondary -> 10.9.0.3:40102
    #   udp->dpdk  : primary -> 10.9.0.4:40102, secondary -> $BRIDGE_IP:40103
    if [[ "$ptx" == dpdk ]]; then
        dpdk_env primary >> "$penv"
        echo "RUSTERON_HARNESS_PUB_CTRL=10.9.0.1:40101" >> "$penv"
        echo "RUSTERON_HARNESS_SUB_ENDPOINTS=10.9.0.3:40102" >> "$penv"
        if [[ "$stx" == dpdk ]]; then
            echo "RUSTERON_HARNESS_DESTINATIONS=10.9.0.4:40102" >> "$penv"
        else
            echo "RUSTERON_HARNESS_DESTINATIONS=$BRIDGE_IP:40102" >> "$penv"
        fi
    else
        echo "RUSTERON_HARNESS_PUB_CTRL=$BRIDGE_IP:40101" >> "$penv"
        echo "RUSTERON_HARNESS_SUB_ENDPOINTS=$BRIDGE_IP:40103" >> "$penv"
        if [[ "$stx" == dpdk ]]; then
            echo "RUSTERON_HARNESS_DESTINATIONS=10.9.0.4:40102" >> "$penv"
        else
            echo "RUSTERON_HARNESS_DESTINATIONS=$BRIDGE_IP:40102" >> "$penv"
        fi
    fi
    if [[ "$stx" == dpdk ]]; then
        dpdk_env secondary >> "$senv"
        echo "RUSTERON_HARNESS_PUB_CTRL=10.9.0.2:40101" >> "$senv"
        echo "RUSTERON_HARNESS_SUB_ENDPOINTS=10.9.0.4:40102" >> "$senv"
        if [[ "$ptx" == dpdk ]]; then
            echo "RUSTERON_HARNESS_DESTINATIONS=10.9.0.3:40102" >> "$senv"
        else
            echo "RUSTERON_HARNESS_DESTINATIONS=$BRIDGE_IP:40103" >> "$senv"
        fi
    else
        echo "RUSTERON_HARNESS_PUB_CTRL=$BRIDGE_IP:40101" >> "$senv"
        echo "RUSTERON_HARNESS_SUB_ENDPOINTS=$BRIDGE_IP:40102" >> "$senv"
        if [[ "$ptx" == dpdk ]]; then
            echo "RUSTERON_HARNESS_DESTINATIONS=10.9.0.3:40102" >> "$senv"
        else
            echo "RUSTERON_HARNESS_DESTINATIONS=$BRIDGE_IP:40103" >> "$senv"
        fi
    fi

    # A scenario built on this flow adds its own lines, last (the later wins).
    [[ -z "${BIDI_PENV_EXTRA:-}" ]] || echo "$BIDI_PENV_EXTRA" >> "$penv"
    [[ -z "${BIDI_SENV_EXTRA:-}" ]] || echo "$BIDI_SENV_EXTRA" >> "$senv"

    # Launch primary; discover and bridge its taps (sender first, receiver
    # second). A UDP role creates no DPDK taps — it binds the bridge itself.
    local before_p=""
    [[ "$ptx" == dpdk ]] && before_p=$(snapshot_taps)
    local p_spec; p_spec=$(run_one primary "$scenario" 1 "$penv")
    local p_report=${p_spec%% *} p_pid=${p_spec##* }
    local p_taps=""
    if [[ "$ptx" == dpdk ]]; then
        p_taps=$(discover_taps "$before_p" "$p_pid" 2) || die "primary ($ptx) tap discovery failed"
        p_taps=( $p_taps )
        attach_tap "${p_taps[0]}" 10.9.0.1
        attach_tap "${p_taps[1]}" 10.9.0.3
    fi

    # Launch secondary; discover and bridge its taps.
    local before_s=""
    [[ "$stx" == dpdk ]] && before_s=$(snapshot_taps)
    local s_spec; s_spec=$(run_one secondary "$scenario" 1 "$senv")
    local s_report=${s_spec%% *} s_pid=${s_spec##* }
    local s_taps=""
    if [[ "$stx" == dpdk ]]; then
        s_taps=$(discover_taps "$before_s" "$s_pid" 2) || die "secondary ($stx) tap discovery failed"
        s_taps=( $s_taps )
        attach_tap "${s_taps[0]}" 10.9.0.2
        attach_tap "${s_taps[1]}" 10.9.0.4
    fi

    # Secondary's receiver tap gets the loss injection for the loss scenario.
    if [[ "$scenario" == loss_recovery ]]; then
        local rxtap
        if [[ "$stx" == dpdk ]]; then rxtap="${s_taps[1]}"; else rxtap=""; fi
        if [[ -n "$rxtap" ]]; then
            tc qdisc add dev "$rxtap" root netem loss 3% 25%
            log "netem 3% loss on $rxtap (receiver)"
        else
            tc qdisc add dev "$BRIDGE" root netem loss 3% 25%
            log "netem 3% loss on bridge (UDP receiver)"
        fi
    fi

    # A scenario built on this flow acts mid-run (the peer MAC move).
    if [[ -n "${BIDI_HOOK:-}" ]]; then
        "$BIDI_HOOK"
    fi

    wait_report "$p_report" "$p_pid" || die "$scenario primary did not report"
    wait_report "$s_report" "$s_pid" || die "$scenario secondary did not report"

    assert_report "$p_report" "$MSGS" "primary->secondary"
    assert_report "$s_report" "$MSGS" "secondary->primary"
    BIDI_P_REPORT=$p_report BIDI_S_REPORT=$s_report

    kill "$p_pid" 2>/dev/null || true
    kill "$s_pid" 2>/dev/null || true
    wait "$p_pid" 2>/dev/null || true
    wait "$s_pid" 2>/dev/null || true

    # A DPDK tap netdev can outlive the process that created it for a beat.
    # Any that linger would be snapshotted as the next run's "before" set and
    # shadow the new process's identically-named taps (discover_taps diffs by
    # name) — the restart scenario hit exactly this. Drop what's left so the
    # next scenario starts with a clean tap namespace.
    local linger; linger=$(snapshot_taps)
    for t in $linger; do
        ip link del "$t" 2>/dev/null || true
    done
    log "scenario $scenario PASSED"
}

scenario_reconnect() {
    local scenario=reconnect
    local penv=$WORK_DIR/$scenario-p.env senv=$WORK_DIR/$scenario-s.env
    : > "$penv"; : > "$senv"
    harness_common_env >> "$penv"; harness_common_env >> "$senv"
    dpdk_env primary >> "$penv"; dpdk_env secondary >> "$senv"

    # Primary reconnects from 10.9.0.4:40102 to 10.9.0.4:40104.
    echo "RUSTERON_HARNESS_PUB_CTRL=10.9.0.1:40101" >> "$penv"
    echo "RUSTERON_HARNESS_PUB_CTRL2=10.9.0.1:40111" >> "$penv"
    echo "RUSTERON_HARNESS_DESTINATIONS=10.9.0.4:40102,10.9.0.4:40104" >> "$penv"
    # Secondary registers both endpoints.
    echo "RUSTERON_HARNESS_SUB_ENDPOINTS=10.9.0.4:40102,10.9.0.4:40104" >> "$senv"
    echo "RUSTERON_HARNESS_PUB_CTRL=10.9.0.2:40101" >> "$senv"
    echo "RUSTERON_HARNESS_DESTINATIONS=10.9.0.3:40102" >> "$senv"

    local before_p; before_p=$(snapshot_taps)
    local p_spec; p_spec=$(run_one primary "$scenario" 1 "$penv")
    local p_report=${p_spec%% *} p_pid=${p_spec##* }
    local p_taps; p_taps=$(discover_taps "$before_p" "$p_pid" 2) || die "primary tap discovery failed"
    p_taps=( $p_taps )
    attach_tap "${p_taps[0]}" 10.9.0.1
    attach_tap "${p_taps[1]}" 10.9.0.3

    local before_s; before_s=$(snapshot_taps)
    local s_spec; s_spec=$(run_one secondary "$scenario" 1 "$senv")
    local s_report=${s_spec%% *} s_pid=${s_spec##* }
    local s_taps; s_taps=$(discover_taps "$before_s" "$s_pid" 2) || die "secondary tap discovery failed"
    s_taps=( $s_taps )
    attach_tap "${s_taps[0]}" 10.9.0.2
    attach_tap "${s_taps[1]}" 10.9.0.4

    wait_report "$p_report" "$p_pid" || die "reconnect primary did not report"
    wait_report "$s_report" "$s_pid" || die "reconnect secondary did not report"

    assert_report "$p_report" 0 "reconnect primary (sender)"
    assert_report "$s_report" "$MSGS" "reconnect secondary (received both batches)"

    kill "$p_pid" 2>/dev/null || true
    kill "$s_pid" 2>/dev/null || true
    wait "$p_pid" 2>/dev/null || true
    wait "$s_pid" 2>/dev/null || true

    # A DPDK tap netdev can outlive the process that created it for a beat.
    # Any that linger would be snapshotted as the next run's "before" set and
    # shadow the new process's identically-named taps (discover_taps diffs by
    # name) — the restart scenario hit exactly this. Drop what's left so the
    # next scenario starts with a clean tap namespace.
    local linger; linger=$(snapshot_taps)
    for t in $linger; do
        ip link del "$t" 2>/dev/null || true
    done
    log "scenario $scenario PASSED"
}

scenario_multi_endpoint() {
    local scenario=multi_endpoint
    local penv=$WORK_DIR/$scenario-p.env senv=$WORK_DIR/$scenario-s.env
    : > "$penv"; : > "$senv"
    harness_common_env >> "$penv"; harness_common_env >> "$senv"
    dpdk_env primary >> "$penv"; dpdk_env secondary >> "$senv"

    # Primary publishes to all three secondary endpoints.
    echo "RUSTERON_HARNESS_PUB_CTRL=10.9.0.1:40101" >> "$penv"
    echo "RUSTERON_HARNESS_DESTINATIONS=10.9.0.4:40102,10.9.0.4:40112,10.9.0.4:40122" >> "$penv"
    echo "RUSTERON_HARNESS_SUB_ENDPOINTS=10.9.0.4:40102,10.9.0.4:40112,10.9.0.4:40122" >> "$senv"
    echo "RUSTERON_HARNESS_PUB_CTRL=10.9.0.2:40101" >> "$senv"
    echo "RUSTERON_HARNESS_DESTINATIONS=10.9.0.3:40102" >> "$senv"
    # MDC-cast fans each offer to all 3 destinations = 3x the frame load of the
    # single-stream scenarios; drop the count so the vdev link converges within
    # the timeout instead of burying the tail under the retransmit storm.
    echo "RUSTERON_HARNESS_MSGS=300" >> "$penv"
    echo "RUSTERON_HARNESS_MSGS=300" >> "$senv"

    local before_p; before_p=$(snapshot_taps)
    local p_spec; p_spec=$(run_one primary "$scenario" 1 "$penv")
    local p_report=${p_spec%% *} p_pid=${p_spec##* }
    local p_taps; p_taps=$(discover_taps "$before_p" "$p_pid" 2) || die "primary tap discovery failed"
    p_taps=( $p_taps )
    attach_tap "${p_taps[0]}" 10.9.0.1
    attach_tap "${p_taps[1]}" 10.9.0.3

    local before_s; before_s=$(snapshot_taps)
    local s_spec; s_spec=$(run_one secondary "$scenario" 1 "$senv")
    local s_report=${s_spec%% *} s_pid=${s_spec##* }
    local s_taps; s_taps=$(discover_taps "$before_s" "$s_pid" 2) || die "secondary tap discovery failed"
    s_taps=( $s_taps )
    attach_tap "${s_taps[0]}" 10.9.0.2
    attach_tap "${s_taps[1]}" 10.9.0.4

    wait_report "$p_report" "$p_pid" || die "multi_endpoint primary did not report"
    wait_report "$s_report" "$s_pid" || die "multi_endpoint secondary did not report"

    # Secondary expects msgs * 3 endpoints (msgs overridden to 300 above).
    local expect=$((300 * 3))
    assert_report "$p_report" 0 "multi_endpoint primary (sender)"
    assert_report "$s_report" "$expect" "multi_endpoint secondary (3 endpoints)"

    kill "$p_pid" 2>/dev/null || true
    kill "$s_pid" 2>/dev/null || true
    wait "$p_pid" 2>/dev/null || true
    wait "$s_pid" 2>/dev/null || true

    # A DPDK tap netdev can outlive the process that created it for a beat.
    # Any that linger would be snapshotted as the next run's "before" set and
    # shadow the new process's identically-named taps (discover_taps diffs by
    # name) — the restart scenario hit exactly this. Drop what's left so the
    # next scenario starts with a clean tap namespace.
    local linger; linger=$(snapshot_taps)
    for t in $linger; do
        ip link del "$t" 2>/dev/null || true
    done
    log "scenario $scenario PASSED"
}

scenario_restart() {
    local scenario=restart
    log "scenario $scenario (run 1)"
    scenario_bidi bidirectional_unicast dpdk dpdk

    # Restart: kill the primary and verify it frees its taps and file-prefix
    # lock, then relaunch and run again.
    log "scenario $scenario: restarting primary"
    local pids; pids=$(pgrep -f "dpdk-harness --role primary" || true)
    if [[ -n "$pids" ]]; then
        kill $pids 2>/dev/null || true
        sleep 2
        pids=$(pgrep -f "dpdk-harness --role primary" || true)
        [[ -z "$pids" ]] || die "primary did not exit on SIGTERM"
    fi
    # Any stale tap devices from the killed primary must be gone.
    local stale; stale=$(snapshot_taps)
    [[ -z "$stale" ]] || log "note: $stale tap devices remain after kill"

    log "scenario $scenario (run 2)"
    scenario_bidi bidirectional_unicast dpdk dpdk
    log "scenario $scenario PASSED"
}

# ---------------------------------------------------------------------------
# Scenarios that assert on the transport's counters and logs
# ---------------------------------------------------------------------------

# alive <pid>: running, not a zombie. A harness started inside run_one's
# subshell is reparented, and where nothing reaps it (a container whose init
# is not a shell) it lingers as a zombie, which kill -0 still finds.
alive() {
    [[ -r /proc/$1/stat ]] && [[ "$(awk '{print $3}' "/proc/$1/stat" 2>/dev/null)" != Z ]]
}

# report_counter <report> <role.dir.name>: the DPDK counter's value at the end
# of that process's run (the report's dpdk_counters), 0 when absent.
report_counter() {
    grep -o '"dpdk_counters": "[^"]*"' "$1" | cut -d'"' -f4 | tr ';' '\n' |
        awk -F= -v k="$2" '$1 == k {v = $2} END {print v + 0}'
}

# wait_file <path> <pid> <what>: until the file exists (the harness's
# <role>.ready), or the process dies, or 45 s pass.
wait_file() {
    local deadline=$((SECONDS + 45))
    until [[ -e "$1" ]]; do
        kill -0 "$2" 2>/dev/null || die "$3: the process exited before $1"
        (( SECONDS < deadline )) || die "$3: no $1 within 45 s"
        sleep 0.2
    done
}

# launch_dpdk_primary <scenario> <envfile>: start the DPDK primary and bridge
# its two taps (sender 10.9.0.1, receiver 10.9.0.3); sets L_REPORT and L_PID.
launch_dpdk_primary() {
    local before spec taps
    before=$(snapshot_taps)
    spec=$(run_one primary "$1" 1 "$2")
    L_REPORT=${spec%% *} L_PID=${spec##* }
    taps=$(discover_taps "$before" "$L_PID" 2) || die "$1: primary tap discovery failed"
    taps=( $taps )
    attach_tap "${taps[0]}" 10.9.0.1
    attach_tap "${taps[1]}" 10.9.0.3
}

finish_primary() {
    kill "$L_PID" 2>/dev/null || true
    wait "$L_PID" 2>/dev/null || true
    for t in $(snapshot_taps); do
        ip link del "$t" 2>/dev/null || true
    done
}

# ICMP echo: the bridge's kernel stack pings both DPDK ports (their own IPs,
# answered by the transport, never the kernel). Asserted on ping's own count
# and on each port's icmp-echo counter, which must equal the pings sent.
scenario_icmp_ping() {
    local scenario=icmp_ping hold=$WORK_DIR/hold-icmp_ping count=5
    local penv=$WORK_DIR/$scenario-p.env
    rm -rf "$hold"; mkdir -p "$hold"
    { harness_common_env; dpdk_env primary
      echo "RUSTERON_HARNESS_PUB_CTRL=10.9.0.1:40101"
      echo "RUSTERON_HARNESS_SUB_ENDPOINTS=10.9.0.3:40102"
      echo "RUSTERON_HARNESS_HOLD_DIR=$hold"; } > "$penv"
    launch_dpdk_primary "$scenario" "$penv"
    wait_file "$hold/primary.ready" "$L_PID" "$scenario"
    local ip role
    for ip in 10.9.0.1 10.9.0.3; do
        ping -n -c "$count" -i 0.2 -W 2 "$ip" > "$WORK_DIR/$scenario-$ip.log" 2>&1 || true
        log "ping $ip: $(grep -E 'packets transmitted' "$WORK_DIR/$scenario-$ip.log" || echo 'no summary')"
        grep -q "$count packets transmitted, $count received" "$WORK_DIR/$scenario-$ip.log" ||
            die "$scenario: $ip did not answer every echo ($(tail -2 "$WORK_DIR/$scenario-$ip.log" | tr '\n' ' '))"
    done
    touch "$hold/primary.release"
    wait_report "$L_REPORT" "$L_PID" || die "$scenario primary did not report"
    [[ "$(report_field "$L_REPORT" ok)" == true ]] || die "$scenario: report not ok"
    for role in sender receiver; do
        local echoes; echoes=$(report_counter "$L_REPORT" "$role.rx.icmp-echo")
        log "assert $scenario: $role icmp-echo=$echoes (want $count)"
        [[ "$echoes" == "$count" ]] || die "$scenario: $role icmp-echo $echoes, want $count"
    done
    finish_primary
    log "scenario $scenario PASSED"
}

# The peer's MAC moves mid-stream (the DPDK-to-UDP flow; the kernel bridge is
# the UDP peer): the primary holds after half its messages, the bridge takes
# a new MAC, and every message must still arrive within 20 s, long before the
# ARP entry's 30 s background refresh. Only the transport noticing the
# peer's new source MAC (and re-resolving it) gets there in time.
move_bridge_mac() {
    wait_file "$MAC_HOLD/primary.ready" "$(pgrep -f "dpdk-harness --role primary" | head -1)" peer_mac_move
    local before; before=$(ip -o link show "$BRIDGE" | grep -o 'link/ether [0-9a-f:]*' | cut -d' ' -f2)
    ip link set "$BRIDGE" address 02:00:00:00:00:fe
    log "peer_mac_move: bridge $BRIDGE MAC $before -> 02:00:00:00:00:fe mid-stream"
    touch "$MAC_HOLD/primary.release"
}

scenario_peer_mac_move() {
    MAC_HOLD=$WORK_DIR/hold-peer_mac_move
    rm -rf "$MAC_HOLD"; mkdir -p "$MAC_HOLD"
    BIDI_PENV_EXTRA="RUSTERON_HARNESS_HOLD_DIR=$MAC_HOLD
RUSTERON_HARNESS_PAUSE_AT=$((MSGS / 2))
RUSTERON_HARNESS_TIMEOUT_SECS=20"
    BIDI_SENV_EXTRA="RUSTERON_HARNESS_TIMEOUT_SECS=20"
    BIDI_HOOK=move_bridge_mac
    scenario_bidi peer_mac_move dpdk udp
    BIDI_PENV_EXTRA="" BIDI_SENV_EXTRA="" BIDI_HOOK=""
    grep -q "held after $((MSGS / 2)) messages" "$BIDI_P_REPORT" || die "peer_mac_move: the primary never held mid-stream"
    ip link set "$BRIDGE" address "$BRIDGE_MAC"
}

# A destination off the subnet behind a gateway nobody answers: the transport
# must count the unresolved next hop (arp-miss) and report the gateway, not
# the destination, as not answering ARP; the publication stays unconnected.
scenario_silent_gateway() {
    local scenario=silent_gateway penv=$WORK_DIR/silent_gateway-p.env
    { harness_common_env; dpdk_env primary
      echo "RUSTERON_HARNESS_PUB_CTRL=10.9.0.1:40101"
      echo "RUSTERON_HARNESS_SUB_ENDPOINTS=10.9.0.3:40102"
      echo "RUSTERON_HARNESS_DESTINATIONS=10.9.7.7:40102"
      echo "RUSTERON_HARNESS_HOLD_SECS=6"; } > "$penv"
    launch_dpdk_primary "$scenario" "$penv"
    wait_report "$L_REPORT" "$L_PID" || die "$scenario primary did not report"
    [[ "$(report_field "$L_REPORT" ok)" == true ]] || die "$scenario: report not ok ($(grep -o '"error": .*' "$L_REPORT"))"
    local misses errors
    misses=$(report_counter "$L_REPORT" sender.tx.arp-miss)
    errors=$(grep -o '"driver_errors": "[^"]*"' "$L_REPORT" | cut -d'"' -f4)
    log "assert $scenario: sender arp-miss=$misses; driver errors: ${errors:-none}"
    (( misses > 0 )) || die "$scenario: no arp-miss counted for the silent gateway"
    [[ "$errors" == *"next hop $GW does not answer ARP"* ]] ||
        die "$scenario: the driver's error log does not name the gateway $GW as not answering ARP"
    finish_primary
    log "scenario $scenario PASSED"
}

# The largest Aeron MTU (LARGEST_MTU), with every data frame full (a message
# of MTU - 32 bytes, Aeron's data header the rest): all of them delivered both
# ways, none dropped as multi-segment or refused as a peer MTU too large. Then
# one step up (+32) must be refused at start, with the reason.
scenario_largest_mtu() {
    local mtu=$LARGEST_MTU
    BIDI_PENV_EXTRA="RUSTERON_DPDK_MAX_AERON_MTU=$mtu
RUSTERON_HARNESS_MTU=$mtu
RUSTERON_HARNESS_PAYLOAD=$((mtu - 32 - 16))"
    BIDI_SENV_EXTRA="$BIDI_PENV_EXTRA"
    scenario_bidi largest_mtu dpdk dpdk
    BIDI_PENV_EXTRA="" BIDI_SENV_EXTRA=""
    local r key v
    for r in "$BIDI_P_REPORT" "$BIDI_S_REPORT"; do
        for key in sender.rx.multi-segment receiver.rx.multi-segment sender.rx.peer-mtu-too-large receiver.rx.peer-mtu-too-large; do
            v=$(report_counter "$r" "$key")
            [[ "$v" == 0 ]] || die "largest_mtu: $key=$v in $r"
        done
        v=$(report_counter "$r" receiver.rx.pkts)
        (( v >= MSGS )) || die "largest_mtu: receiver rx pkts $v < $MSGS in $r"
    done
    log "assert largest_mtu: $MSGS full $mtu-byte frames each way, no multi-segment or peer-MTU drops"
    # One step up: refused at start (the port's limit), never a silent truncation.
    local penv=$WORK_DIR/largest_mtu-over-p.env up=$((mtu + 32)) spec pid deadline
    { harness_common_env; dpdk_env primary
      echo "RUSTERON_DPDK_MAX_AERON_MTU=$up"
      echo "RUSTERON_HARNESS_MTU=$up"; } > "$penv"
    spec=$(run_one primary largest_mtu-over 1 "$penv")
    pid=${spec##* }
    deadline=$((SECONDS + 45))
    while alive "$pid"; do
        (( SECONDS < deadline )) || { kill "$pid"; die "largest_mtu: Aeron MTU $up started (no refusal within 45 s)"; }
        sleep 0.2
    done
    # A refused start writes no report (the harness exits 1 before any scenario).
    local logf=$WORK_DIR/largest_mtu-over-primary-1.log
    [[ ! -s "${spec%% *}" ]] || die "largest_mtu: Aeron MTU $up ran a scenario ($(grep -o '"ok": [a-z]*' "${spec%% *}"))"
    grep -q "transport configuration failed" "$logf" || die "largest_mtu: Aeron MTU $up did not fail its transport ($(tail -3 "$logf" | tr '\n' ' '))"
    grep -qE "L3 MTU|max_aeron_mtu" "$logf" || die "largest_mtu: Aeron MTU $up failed without naming the MTU ($(tail -3 "$logf" | tr '\n' ' '))"
    log "assert largest_mtu: Aeron MTU $up refused at start: $(grep -oE '[^:]*(L3 MTU|max_aeron_mtu)[^"]*' "$logf" | head -1)"
    for t in $(snapshot_taps); do
        ip link del "$t" 2>/dev/null || true
    done
    log "scenario largest_mtu PASSED"
}

# The lab's raw floor (md-dpdk/harness/src/bin/floor.rs, its dpdk-raw mode):
# ergon-md-dpdk's bindings driven directly, no Aeron driver or client. The
# secondary pongs (10.9.0.4 in, 10.9.0.2 out), the primary pings (10.9.0.3
# in, 10.9.0.1 out), each on one thread pinned off the EAL main lcore's core.
# Asserted on what the lab reads: the ping's one rtt line (every round trip
# measured, none lost), its histogram, its spikes (LOAD_SPIKE_NS=1: every
# round trip one), both loop-stall files, and the pong ending on SIGTERM.
scenario_raw_floor() {
    local scenario=raw_floor out=$WORK_DIR/raw_floor
    local floor=${RUSTERON_FLOOR_BIN:-$(dirname "$HARNESS_BIN")/floor}
    [[ -x $floor ]] || die "$scenario: no floor client at $floor (cargo build -p ergon-md-dpdk-harness --features dpdk builds it; RUSTERON_FLOOR_BIN names another)"
    rm -rf "$out"; mkdir -p "$out"
    local penv=$WORK_DIR/$scenario-p.env senv=$WORK_DIR/$scenario-s.env
    { dpdk_env primary
      echo "LABEL=$scenario"
      echo "RTT_SECONDS=30"
      # the tap PMD loses nothing: a stall of the emulated host is not a loss
      echo "RTT_LOST_TIMEOUT_US=10000000"
      echo "RTT_HISTOGRAM=$out/histogram.csv"
      echo "LOAD_SPIKES=$out/spikes.csv"
      echo "LOAD_SPIKE_NS=1"
      echo "LOOP_STALLS_DIR=$out"
      echo "RUSTERON_HARNESS_START_FILE=$(start_file primary)"; } > "$penv"
    { dpdk_env secondary
      echo "LOOP_STALLS_DIR=$out"
      echo "RUSTERON_HARNESS_START_FILE=$(start_file secondary)"; } > "$senv"
    local ping_ep=10.9.0.4:40130 pong_ep=10.9.0.3:40131 vars
    rm -f "$(start_file primary)" "$(start_file secondary)"

    local before; before=$(snapshot_taps)
    mapfile -t vars < "$senv"
    env "${vars[@]}" "$floor" xpong "$ping_ep" "$pong_ep" "$PLAN_RECEIVER_CPU" \
        >"$WORK_DIR/$scenario-pong.out" 2>"$WORK_DIR/$scenario-pong.log" &
    local pong_pid=$!
    local taps; taps=$(discover_taps "$before" "$pong_pid" 2) || die "$scenario: the pong's taps did not appear"
    taps=( $taps )
    attach_tap "${taps[0]}" 10.9.0.2
    attach_tap "${taps[1]}" 10.9.0.4

    before=$(snapshot_taps)
    mapfile -t vars < "$penv"
    env "${vars[@]}" "$floor" xping "$ping_ep" "$pong_ep" "$MSGS" 100 "$PLAN_SENDER_CPU" \
        >"$WORK_DIR/$scenario-ping.out" 2>"$WORK_DIR/$scenario-ping.log" &
    local ping_pid=$!
    taps=$(discover_taps "$before" "$ping_pid" 2) || die "$scenario: the ping's taps did not appear"
    taps=( $taps )
    attach_tap "${taps[0]}" 10.9.0.1
    attach_tap "${taps[1]}" 10.9.0.3

    local deadline=$((SECONDS + 120)) rc=0
    while kill -0 "$ping_pid" 2>/dev/null && ((SECONDS < deadline)); do sleep 0.5; done
    kill -0 "$ping_pid" 2>/dev/null && die "$scenario: the ping still running after 120 s"
    wait "$ping_pid" || rc=$?
    kill -TERM "$pong_pid" 2>/dev/null || true
    local prc=0
    wait "$pong_pid" || prc=$?
    ((rc == 0)) || die "$scenario: the ping failed ($rc)"
    ((prc == 0)) || die "$scenario: the pong failed ($prc)"
    grep -q "^floor: ergon-md-dpdk's packet path, no Aeron driver or client" "$WORK_DIR/$scenario-ping.log" ||
        die "$scenario: the ping did not run on the DPDK wire"
    grep -q '^floor: pong ended: SIGTERM$' "$WORK_DIR/$scenario-pong.log" || die "$scenario: the pong did not say it ended on SIGTERM"

    # rtt,<label>,udp,p50,p90,p99,p99.9,p99.99,max,mean,samples,lost: one line
    local line fields
    line=$(cat "$WORK_DIR/$scenario-ping.out")
    log "assert $scenario: $line"
    [[ $(wc -l <"$WORK_DIR/$scenario-ping.out") -eq 1 && $line == "rtt,$scenario,udp,"* ]] ||
        die "$scenario: not one rtt line on stdout: $line"
    IFS=, read -ra fields <<<"$line"
    ((${#fields[@]} == 12)) || die "$scenario: ${#fields[@]} fields, not 12: $line"
    [[ ${fields[10]} == "$MSGS" && ${fields[11]} == 0 ]] ||
        die "$scenario: ${fields[10]} round trips and ${fields[11]} lost, want $MSGS and 0"
    ((fields[3] > 0 && fields[3] <= fields[8])) || die "$scenario: p50 ${fields[3]} and max ${fields[8]}"
    local counted
    counted=$(awk -F, 'NR > 1 {n += $2} END {print n + 0}' "$out/histogram.csv")
    [[ $(head -1 "$out/histogram.csv") == value_ns,count && $counted == "$MSGS" ]] ||
        die "$scenario: the histogram holds $counted round trips, want $MSGS"
    grep -qx "# spikes_total=$MSGS" "$out/spikes.csv" || die "$scenario: spikes_total is not $MSGS"
    [[ $(grep -v '^#' "$out/spikes.csv" | head -1) == stream,due_ns,latency_ns,ping ]] ||
        die "$scenario: the spikes file's header"
    [[ $(grep -c '^1002,' "$out/spikes.csv") == "$MSGS" ]] || die "$scenario: not $MSGS spike rows"
    local f
    for f in stalls-ping.csv stalls-pong.csv; do
        grep -q '^# start_realtime_ns=' "$out/$f" && grep -qx 'end_ns,gap_ns' "$out/$f" ||
            die "$scenario: $f missing or malformed"
    done

    local linger; linger=$(snapshot_taps)
    for t in $linger; do
        ip link del "$t" 2>/dev/null || true
    done
    log "scenario $scenario PASSED"
}

# ---------------------------------------------------------------------------
# Cleanup + main
# ---------------------------------------------------------------------------

cleanup() {
    [[ -n "${CLEANUP_ON_EXIT:-}" ]] || return 0
    pkill -f 'dpdk-harness' 2>/dev/null || true
    pkill -x floor 2>/dev/null || true
    sleep 1
    ip link del "$BRIDGE" 2>/dev/null || true
    for t in $(snapshot_taps); do
        ip link del "$t" 2>/dev/null || true
    done
    log "cleaned up bridge and taps"
}

# Dump the redirected harness output on failure: the harness's own stderr goes
# to $WORK_DIR/*.log, which CI otherwise never surfaces.
dump_harness_logs() {
    for f in "$WORK_DIR"/*.log; do
        [[ -e "$f" ]] || continue
        log "--- $f ---"
        tail -n 120 "$f" >&2
    done
}

trap 'rc=$?; [[ $rc -ne 0 ]] && dump_harness_logs; cleanup' EXIT

mkdir -p "$REPORT_DIR" "$WORK_DIR"

setup_hugepages
build_harness

# The harness links libaeron*.so dynamically from the build-script OUT_DIR
# (a non-standard loader path), so a bare `target/debug/dpdk-harness` cannot
# find them at runtime. Export the dirs so the launched harness resolves them.
export LD_LIBRARY_PATH="$(find "${CARGO_TARGET_DIR:-target}" -name 'libaeron*.so' -printf '%h\n' 2>/dev/null | sort -u | tr '\n' ':' | sed 's/:$//' || true)"

plan_cpus
bridge_up

run_one_scenario() {
    case "$1" in
        bidirectional_unicast) scenario_bidi "$1" dpdk dpdk ;;
        dpdk_to_udp)          scenario_bidi "$1" dpdk udp ;;
        udp_to_dpdk)          scenario_bidi "$1" udp dpdk ;;
        reconnect)            scenario_reconnect ;;
        multi_endpoint)       scenario_multi_endpoint ;;
        loss_recovery)        scenario_bidi "$1" dpdk dpdk ;;
        restart)              scenario_restart ;;
        icmp_ping)            scenario_icmp_ping ;;
        peer_mac_move)        scenario_peer_mac_move ;;
        silent_gateway)       scenario_silent_gateway ;;
        largest_mtu)          scenario_largest_mtu ;;
        raw_floor)            scenario_raw_floor ;;
        *) die "unknown scenario $1" ;;
    esac
}

if [[ -n "$SCENARIO" ]]; then
    run_one_scenario "$SCENARIO"
else
    for s in $ALL_SCENARIOS; do
        log "=== scenario: $s ==="
        run_one_scenario "$s"
    done
fi

CLEANUP_ON_EXIT=0
cleanup
log "ALL VDEV SCENARIOS PASSED"
