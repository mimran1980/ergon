#!/usr/bin/env bash
# ergon-md-dpdk DPDK vdev interoperability harness (plan §11.2).
#
# Runs the dpdk-harness scenarios over DPDK tap devices on a Linux bridge,
# exercising DPDK-to-DPDK, DPDK-to-default-UDP in both directions, reconnect,
# multiple endpoints, loss recovery, and restart cleanup.
#
# This requires a PRIVILEGED Linux x86_64 host:
#   - root (the script creates bridges/taps and mounts hugetlbfs)
#   - /dev/net/tun available (modprobe tun)
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

ALL_SCENARIOS="bidirectional_unicast dpdk_to_udp udp_to_dpdk reconnect multi_endpoint loss_recovery restart"

# ---------------------------------------------------------------------------
# Prerequisites
# ---------------------------------------------------------------------------

[[ "$(id -u)" -eq 0 ]] || die "must run as root (sudo scripts/test-dpdk-vdev.sh)"
[[ -e /dev/net/tun ]] || die "/dev/net/tun missing — modprobe tun"
command -v "$CARGO" >/dev/null || die "cargo not found"
command -v pkg-config >/dev/null || die "pkg-config not found"
pkg-config --exists libdpdk || die "libdpdk dev package not found (pkg-config libdpdk)"
pkg-config --exists libdpdk >/dev/null || true

setup_hugepages() {
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
}

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
    # One word: run_one passes this file through `env $(cat ...)`, which
    # splits on spaces, so the lcore list is attached (`-l3`, not `-l 3`).
    echo "RUSTERON_DPDK_EAL_ARGS=${RUSTERON_DPDK_EAL_ARGS:--l$PLAN_MAIN_LCORE}"
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
    log "launching $role ($scenario, run $run_id)"
    # shellcheck disable=SC2046
    env $(cat "$envfile") "$HARNESS_BIN" --role "$role" --scenario "$scenario" --report "$report" "$@" \
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

    wait_report "$p_report" "$p_pid" || die "$scenario primary did not report"
    wait_report "$s_report" "$s_pid" || die "$scenario secondary did not report"

    assert_report "$p_report" "$MSGS" "primary->secondary"
    assert_report "$s_report" "$MSGS" "secondary->primary"

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
# Cleanup + main
# ---------------------------------------------------------------------------

cleanup() {
    [[ -n "${CLEANUP_ON_EXIT:-}" ]] || return 0
    pkill -f 'dpdk-harness' 2>/dev/null || true
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
export LD_LIBRARY_PATH="$(find target -name 'libaeron*.so' -printf '%h\n' 2>/dev/null | sort -u | tr '\n' ':' | sed 's/:$//' || true)"

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
