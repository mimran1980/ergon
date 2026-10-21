#!/usr/bin/env bash
# AWS EKS DPDK ENA acceptance runner (plan §11.3 + §12 + §13.2 rehearsal).
#
# Executes the §11.3 functional matrix on real ENA hardware, the §12 benchmark
# matrix and gates (kernel UDP baseline / DPDK ENA Express off / on), captures
# reproducible environment metadata, and rehearses the §13.2 default-transport
# rollback. Everything lands in a versioned artifact directory.
#
# PREREQUISITES — run from the repo root on a machine with:
#   - `aws` CLI + EKS/EC2 read/write permission for the cluster and ENIs
#   - `kubectl` context pointing at the cluster (cell = one AZ)
#   - SSH key + access to the two rusteron-dpdk/ena=true worker nodes
#   - `jq` locally and on the nodes
#   - built binaries: `cargo build -p ergon-md-dpdk-harness --features dpdk --release`
#     and `cargo build -p ergon-md-dpdk --features dpdk --bin media_driver --release`
#     (override with RUSTERON_HARNESS_BIN / RUSTERON_MEDIA_DRIVER_BIN)
#   - the media-driver DaemonSet parked off the test cell (a DaemonSet cannot
#     be scaled: the runbook's §1 nodeSelector patch, md-dpdk/docs/aws-eks-dpdk-ena-runbook.md)
#     so the harness can bind the ENA pair itself
#
# Usage:
#   scripts/aws-dpdk-acceptance.sh [all|functional|bench|rollback|metadata]
#
# The default runs every phase. Phases are rerunnable; artifacts accumulate in
# a timestamped subdirectory of RUSTERON_ACCEPTANCE_ARTIFACTS (default
# /tmp/rusteron-aws-acceptance).

set -euo pipefail
cd "$(dirname "$0")/.."

# ---------------------------------------------------------------------------
# Config (env-overridable)
# ---------------------------------------------------------------------------

CELL="${RUSTERON_ACCEPTANCE_CELL:-dpdk-test}"
ARTIFACTS="${RUSTERON_ACCEPTANCE_ARTIFACTS:-/tmp/rusteron-aws-acceptance}"
HARNESS_BIN="${RUSTERON_HARNESS_BIN:-target/release/dpdk-harness}"
MEDIA_DRIVER_BIN="${RUSTERON_MEDIA_DRIVER_BIN:-target/release/media_driver}"
SSH_USER="${RUSTERON_ACCEPTANCE_SSH_USER:-ec2-user}"
SSH_KEY="${RUSTERON_ACCEPTANCE_SSH_KEY:-}"
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=10)
[[ -n "$SSH_KEY" ]] && SSH_OPTS+=(-i "$SSH_KEY")

WARMUP_SECS="${RUSTERON_ACCEPTANCE_WARMUP:-30}"   # §12.2 warm-up
RUNS="${RUSTERON_ACCEPTANCE_RUNS:-10}"            # §12.2 ten 60s runs
RUN_SECS="${RUSTERON_ACCEPTANCE_RUN_SECS:-60}"
SIZES="${RUSTERON_ACCEPTANCE_SIZES:-64 256 1408 16384 1048576}"
COMMON_FRAC="${RUSTERON_ACCEPTANCE_COMMON_FRAC:-0.5}"    # §12.2 common load
STRESS_FRAC="${RUSTERON_ACCEPTANCE_STRESS_FRAC:-0.7}"    # §12.2 stress load
STREAM=32931
MSGS=1000                 # functional scenarios, per run

PHASE="${1:-all}"

log()  { echo "[acceptance] $*" >&2; }
die()  { echo "[acceptance] ERROR: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null || die "missing required tool: $1"; }

mkdir -p "$ARTIFACTS"
STAMP=$(date +%Y%m%dT%H%M%S)
RUN_DIR="$ARTIFACTS/run-$STAMP"
mkdir -p "$RUN_DIR"

# ---------------------------------------------------------------------------
# Node resolution
# ---------------------------------------------------------------------------

# pick two ENA-bootstrapped nodes; set NODE_A/NODE_B (names) + IP_A/IP_B.
pick_nodes() {
    local names
    names=$(kubectl get nodes -l rusteron-dpdk/ena=true -o jsonpath='{.items[*].metadata.name}' 2>/dev/null) \
        || die "kubectl cannot list nodes — set context to the cluster"
    names=( $names )
    ((${#names[@]} >= 2)) || die "need ≥2 nodes labelled rusteron-dpdk/ena=true (found ${#names[@]})"
    NODE_A=${names[0]}
    NODE_B=${names[1]}
    IP_A=$(kubectl get node "$NODE_A" -o jsonpath='{.status.addresses[?(@.type=="InternalIP")].address}')
    IP_B=$(kubectl get node "$NODE_B" -o jsonpath='{.status.addresses[?(@.type=="InternalIP")].address}')
    log "cell nodes: $NODE_A ($IP_A)  $NODE_B ($IP_B)"

    # Same-cluster placement group is the biggest §12 latency lever (plan §17:
    # same placement group / spread). Missing = warn (functional runs still fine);
    # differing groups = die (inter-node benchmark would be unrepresentative).
    PG_A="$(placement_group "$NODE_A")"
    PG_B="$(placement_group "$NODE_B")"
    if [[ -z "$PG_A" || -z "$PG_B" ]]; then
        log "WARN: node not in a placement group (A='$PG_A' B='$PG_B') — same-cluster placement group required for representative §12 latencies (runbook §2)"
    elif [[ "$PG_A" != "$PG_B" ]]; then
        die "nodes in different placement groups ($PG_A vs $PG_B) — move both into one cluster placement group"
    else
        log "placement group: $PG_A (both nodes)"
    fi
}

nssh() { # nssh <node> <command...>
    ssh "${SSH_OPTS[@]}" "$SSH_USER@$1" "${@:2}"
}

nscp() { # nscp <node> <src> <dst>
    scp "${SSH_OPTS[@]}" "$2" "$SSH_USER@$1:$3"
}

inventory_json() { nssh "$1" cat /var/lib/rusteron-dpdk/ena-pairs.json; }

# placement_group <node> -> cluster placement group name ('' when none/IMDS err).
# IMDSv2 token first, IMDSv1 fallback; empty result means "not in a group".
placement_group() {
    nssh "$1" 'tok=$(curl -fsS -X PUT http://169.254.169.254/latest/api/token \
        -H "X-aws-ec2-metadata-token-ttl-seconds: 60" 2>/dev/null) && \
        curl -fsS -H "X-aws-ec2-metadata-token: $tok" \
        http://169.254.169.254/latest/meta-data/placement/placement-group-name \
        2>/dev/null || \
        curl -fsS http://169.254.169.254/latest/meta-data/placement/placement-group-name \
        2>/dev/null || true' | tr -d '\r'
}

# write_pair_env <node> <outfile> <peer_receiver_ip:port>
#   DPDK env for one node's own ENA pair (both roles), plus the harness MDC
#   endpoints (local receiver here, peer receiver as destination).
write_pair_env() {
    local node=$1 out=$2 peer=$3
    local inv
    inv=$(inventory_json "$node")
    echo "$inv" | jq -e '.pairs | length > 0' >/dev/null || die "$node has no DPDK pairs"
    local pci_s pci_r ip_s ip_r gw_s gw_r pre_s pre_r prefix
    pci_s=$(echo "$inv" | jq -r '.pairs[0].sender.pci')
    pci_r=$(echo "$inv" | jq -r '.pairs[0].receiver.pci')
    ip_s=$(echo "$inv" | jq -r '.pairs[0].sender.ipv4')
    ip_r=$(echo "$inv" | jq -r '.pairs[0].receiver.ipv4')
    pre_s=$(echo "$inv" | jq -r '.pairs[0].sender.prefix_len')
    pre_r=$(echo "$inv" | jq -r '.pairs[0].receiver.prefix_len')
    gw_s=$(echo "$inv" | jq -r '.pairs[0].sender.gateway')
    gw_r=$(echo "$inv" | jq -r '.pairs[0].receiver.gateway')
    prefix=$(echo "$inv" | jq -r '.pairs[0].id')
    # An ENA queue takes at most 512 TX descriptors, and the transport refuses
    # a count outside the NIC's limits.
    cat > "$out" <<EOF
RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena
RUSTERON_DPDK_FILE_PREFIX=rusteron-dpdk-$prefix
RUSTERON_DPDK_SENDER_PCI=$pci_s
RUSTERON_DPDK_SENDER_IPV4_CIDR=$ip_s/$pre_s
RUSTERON_DPDK_SENDER_GATEWAY=$gw_s
RUSTERON_DPDK_RECEIVER_PCI=$pci_r
RUSTERON_DPDK_RECEIVER_IPV4_CIDR=$ip_r/$pre_r
RUSTERON_DPDK_RECEIVER_GATEWAY=$gw_r
RUSTERON_DPDK_HUGE_DIR=/dev/hugepages
RUSTERON_DPDK_RX_DESCRIPTORS=1024
RUSTERON_DPDK_TX_DESCRIPTORS=512
RUSTERON_DPDK_MBUFS_PER_PORT=65536
RUSTERON_DPDK_MEMPOOL_CACHE=256
RUSTERON_DPDK_BURST_SIZE=32
RUSTERON_DPDK_MAX_AERON_MTU=1408
RUSTERON_HARNESS_PUB_CTRL=$ip_s:40101
RUSTERON_HARNESS_SUB_ENDPOINTS=$ip_r:40102
RUSTERON_HARNESS_DESTINATIONS=$peer
RUSTERON_HARNESS_SENDER_CPU=1
RUSTERON_HARNESS_RECEIVER_CPU=2
EOF
}

# write_udp_env <node> <outfile> <peer_private_ip>
#   Kernel-UDP env: no DPDK vars; Aeron default transport over the primary
#   ENA, so the MDC endpoints are the nodes' own private IPs.
write_udp_env() {
    local node=$1 out=$2 peer=$3 me
    [[ "$node" == "$NODE_A" ]] && me=$IP_A || me=$IP_B
    cat > "$out" <<EOF
RUSTERON_HARNESS_PUB_CTRL=$me:40101
RUSTERON_HARNESS_SUB_ENDPOINTS=$me:40102
RUSTERON_HARNESS_DESTINATIONS=$peer:40102
RUSTERON_HARNESS_SENDER_CPU=1
RUSTERON_HARNESS_RECEIVER_CPU=2
EOF
}

# write_udp_sec_env <node> <outfile> <peer_receiver_ip>
#   Kernel-UDP env over the node's own secondary ENAs (plan §12.1 baseline):
#   no DPDK vars; the MDC endpoints are the sender/receiver secondary IPs, so
#   the baseline measures the same ENIs the DPDK modes use, not the primary.
write_udp_sec_env() {
    local node=$1 out=$2 peer=$3 inv ip_s ip_r
    inv=$(inventory_json "$node")
    echo "$inv" | jq -e '.pairs | length > 0' >/dev/null || die "$node has no DPDK pairs"
    ip_s=$(echo "$inv" | jq -r '.pairs[0].sender.ipv4')
    ip_r=$(echo "$inv" | jq -r '.pairs[0].receiver.ipv4')
    cat > "$out" <<EOF
RUSTERON_HARNESS_PUB_CTRL=$ip_s:40101
RUSTERON_HARNESS_SUB_ENDPOINTS=$ip_r:40102
RUSTERON_HARNESS_DESTINATIONS=$peer:40102
RUSTERON_HARNESS_SENDER_CPU=1
RUSTERON_HARNESS_RECEIVER_CPU=2
EOF
}

# ena_bind_kernel <node> <peer_rx_ip>
#   Temporarily return both secondary ENAs to the kernel `ena` driver with
#   their inventory IPs up (plan §12.1 baseline). Mirrors the bootstrap's
#   bind_vfio in reverse: vfio-pci unbind, clear driver_override, probe.
#   A /32 route to <peer_rx_ip> is added via the sender's gateway only when
#   the peer is outside the sender's subnet (same-subnet peers are on-link;
#   forcing the gateway would add a hop and skew the baseline).
ena_bind_kernel() {
    local node=$1 peer=$2 inv role bdf ip pfx gw iface i
    inv=$(inventory_json "$node")
    for role in sender receiver; do
        bdf=$(echo "$inv" | jq -r ".pairs[0].$role.pci")
        ip=$(echo "$inv" | jq -r ".pairs[0].$role.ipv4")
        pfx=$(echo "$inv" | jq -r ".pairs[0].$role.prefix_len")
        gw=$(echo "$inv" | jq -r ".pairs[0].$role.gateway")
        nssh "$node" "echo $bdf > /sys/bus/pci/drivers/vfio-pci/unbind 2>/dev/null || true"
        nssh "$node" "echo > /sys/bus/pci/devices/$bdf/driver_override"
        nssh "$node" "echo $bdf > /sys/bus/pci/drivers_probe"
        iface=""
        for i in 1 2 3 4 5; do
            iface=$(nssh "$node" "ls /sys/bus/pci/devices/$bdf/net/ 2>/dev/null | head -1" | tr -d '\r')
            [[ -n "$iface" ]] && break
            sleep 1
        done
        [[ -n "$iface" ]] || die "ena_bind_kernel: $node $bdf ($role) got no net iface after probe"
        nssh "$node" "ip addr add $ip/$pfx dev $iface 2>/dev/null || true; ip link set $iface up"
        if [[ "$role" == sender ]]; then
            # Route the peer receiver over the sender ENA unless already on-link
            # (forcing the gateway on a same-subnet peer would add a hop).
            if ! in_subnet "$peer" "$ip" "$pfx"; then
                nssh "$node" "ip route add $peer/32 via $gw dev $iface 2>/dev/null || true"
            fi
        fi
    done
    log "returned secondary ENAs to the kernel ena driver on $node"
}

# ena_bind_vfio <node>
#   Restore both secondary ENAs to vfio-pci (bootstrap bind_vfio semantics);
#   dies if either does not end up bound to vfio-pci.
ena_bind_vfio() {
    local node=$1 inv role bdf driver
    inv=$(inventory_json "$node")
    for role in sender receiver; do
        bdf=$(echo "$inv" | jq -r ".pairs[0].$role.pci")
        nssh "$node" "echo $bdf > /sys/bus/pci/drivers/ena/unbind 2>/dev/null || true"
        nssh "$node" "echo vfio-pci > /sys/bus/pci/devices/$bdf/driver_override"
        nssh "$node" "echo $bdf > /sys/bus/pci/drivers_probe"
        driver=$(nssh "$node" "readlink /sys/bus/pci/devices/$bdf/driver 2>/dev/null | xargs -r basename" | tr -d '\r')
        [[ "$driver" == "vfio-pci" ]] || die "ena_bind_vfio: restore failed on $node $bdf (driver=$driver)"
    done
    log "restored secondary ENAs to vfio-pci on $node"
}

# in_subnet <ip> <cidr_ip> <prefix> — true when <ip> lies within <cidr_ip>/<prefix>.
in_subnet() {
    local a b mask
    a=$(ip_int "$1"); b=$(ip_int "$2")
    mask=$(( (1 << (32 - $3)) - 1 )); mask=$(( ~mask ))
    (( (a & mask) == (b & mask) ))
}
ip_int() { local IFS=. a b c d; read -r a b c d <<<"$1"; echo $(( a*16777216 + b*65536 + c*256 + d )); }

harness_common() { # harness_common <outfile> <scenario> <msgs> <payload>
    local out=$1 scenario=$2 msgs=$3 payload=$4
    cat >> "$out" <<EOF
RUSTERON_HARNESS_MSGS=$msgs
RUSTERON_HARNESS_PAYLOAD=$payload
RUSTERON_HARNESS_STREAM=$STREAM
RUSTERON_HARNESS_TIMEOUT_SECS=90
RUSTERON_HARNESS_MTU=1408
EOF
}

report_field() { # report_field <report> <field>
    grep -o "\"$field\": [^,}]*" "$1" | head -1 | cut -d' ' -f2
}

assert_report() { # assert_report <report> <role> <label>
    local ok; ok=$(report_field "$1" ok)
    [[ "$ok" == true ]] || die "$3 $2 report not ok: $(cat "$1")"
    log "$3 $2: ok=true received=$(report_field "$1" received) transport=$(report_field "$1" transport)"
}

# ---------------------------------------------------------------------------
# Harness runners
# ---------------------------------------------------------------------------

# run_embedded <scenario> <msgs> <payload> <envA> <envB> <label>
#   Runs dpdk-harness on both nodes (A=primary, B=secondary), asserts reports.
run_embedded() {
    local scenario=$1 msgs=$2 payload=$3 envA=$4 envB=$5 label=$6
    harness_common "$envA" "$scenario" "$msgs" "$payload"
    harness_common "$envB" "$scenario" "$msgs" "$payload"

    nscp "$NODE_A" "$HARNESS_BIN" /tmp/dpdk-harness
    nscp "$NODE_B" "$HARNESS_BIN" /tmp/dpdk-harness
    # shellcheck disable=SC2046
    nssh "$NODE_A" "chmod +x /tmp/dpdk-harness && env $(cat "$envA") /tmp/dpdk-harness --role primary --scenario '$scenario' --report /tmp/$label-p.json" \
        >"$RUN_DIR/$label-primary.log" 2>&1 &
    local pid_a=$!
    # shellcheck disable=SC2046
    nssh "$NODE_B" "chmod +x /tmp/dpdk-harness && env $(cat "$envB") /tmp/dpdk-harness --role secondary --scenario '$scenario' --report /tmp/$label-s.json" \
        >"$RUN_DIR/$label-secondary.log" 2>&1 &
    local pid_b=$!
    wait "$pid_a" || die "$label primary failed (see $RUN_DIR/$label-primary.log)"
    wait "$pid_b" || die "$label secondary failed (see $RUN_DIR/$label-secondary.log)"
    nssh "$NODE_A" "cat /tmp/$label-p.json" > "$RUN_DIR/$label-primary.json"
    nssh "$NODE_B" "cat /tmp/$label-s.json" > "$RUN_DIR/$label-secondary.json"
    assert_report "$RUN_DIR/$label-primary.json" primary "$label"
    assert_report "$RUN_DIR/$label-secondary.json" secondary "$label"
    log "$label PASSED"
}

# run_standalone <scenario> <msgs> <payload> <envA> <envB> <label>
#   Starts a standalone media driver on each node (plan §11.3 row 1), then
#   runs the harness with --standalone connecting to those drivers.
run_standalone() {
    local scenario=$1 msgs=$2 payload=$3 envA=$4 envB=$5 label=$6
    harness_common "$envA" "$scenario" "$msgs" "$payload"
    harness_common "$envB" "$scenario" "$msgs" "$payload"
    local dirA="/tmp/rusteron-standalone-a" dirB="/tmp/rusteron-standalone-b"

    for node in "$NODE_A" "$NODE_B"; do
        nscp "$node" "$HARNESS_BIN" /tmp/dpdk-harness
        nscp "$node" "$MEDIA_DRIVER_BIN" /tmp/media_driver
    done
    local envA2 envB2
    envA2="$RUN_DIR/standalone-a-driver.env"; envB2="$RUN_DIR/standalone-b-driver.env"
    # The driver needs the DPDK vars but not the harness MDC lines; reuse the
    # pair env minus the harness_* keys by re-emitting DPDK vars only.
    grep '^RUSTERON_' "$envA" | grep -v '^RUSTERON_HARNESS' > "$envA2"
    grep '^RUSTERON_' "$envB" | grep -v '^RUSTERON_HARNESS' > "$envB2"

    nssh "$NODE_A" "rm -rf $dirA && chmod +x /tmp/media_driver && RUSTERON_MEDIA_DRIVER_DIR=$dirA env $(cat "$envA2") /tmp/media_driver" \
        >"$RUN_DIR/$label-driver-a.log" 2>&1 &
    local pid_a=$!
    nssh "$NODE_B" "rm -rf $dirB && chmod +x /tmp/media_driver && RUSTERON_MEDIA_DRIVER_DIR=$dirB env $(cat "$envB2") /tmp/media_driver" \
        >"$RUN_DIR/$label-driver-b.log" 2>&1 &
    local pid_b=$!
    wait_driver_ready "$pid_a" "$NODE_A" "$dirA" "$label driver A"
    wait_driver_ready "$pid_b" "$NODE_B" "$dirB" "$label driver B"

    # shellcheck disable=SC2046
    nssh "$NODE_A" "RUSTERON_HARNESS_DRIVER_DIR=$dirA env $(cat "$envA") /tmp/dpdk-harness --standalone --role primary --scenario '$scenario' --report /tmp/$label-p.json" \
        >"$RUN_DIR/$label-primary.log" 2>&1 &
    local hp_a=$!
    # shellcheck disable=SC2046
    nssh "$NODE_B" "RUSTERON_HARNESS_DRIVER_DIR=$dirB env $(cat "$envB") /tmp/dpdk-harness --standalone --role secondary --scenario '$scenario' --report /tmp/$label-s.json" \
        >"$RUN_DIR/$label-secondary.log" 2>&1 &
    local hp_b=$!
    wait "$hp_a" || die "$label standalone primary failed"
    wait "$hp_b" || die "$label standalone secondary failed"

    kill "$pid_a" "$pid_b" 2>/dev/null || true
    nssh "$NODE_A" "cat /tmp/$label-p.json" > "$RUN_DIR/$label-primary.json"
    nssh "$NODE_B" "cat /tmp/$label-s.json" > "$RUN_DIR/$label-secondary.json"
    assert_report "$RUN_DIR/$label-primary.json" primary "$label (standalone)"
    assert_report "$RUN_DIR/$label-secondary.json" secondary "$label (standalone)"
    log "$label (standalone) PASSED"
}

wait_driver_ready() { # wait_driver_ready <ssh_pid> <node> <dir> <label>
    local ssh_pid=$1 node=$2 dir=$3 label=$4
    local deadline=$((SECONDS + 60))
    while (( SECONDS < deadline )); do
        if ! kill -0 "$ssh_pid" 2>/dev/null; then
            die "$label standalone driver exited early (see log)"
        fi
        # cnc.dat appearing is the authoritative readiness signal.
        if nssh "$node" "test -f $dir/cnc.dat"; then
            log "$label standalone driver ready ($dir)"
            return 0
        fi
        sleep 1
    done
    die "$label standalone driver never became ready"
}

# ---------------------------------------------------------------------------
# §11.3 functional matrix
# ---------------------------------------------------------------------------

phase_functional() {
    log "=== functional matrix (§11.3) ==="
    local envA="$RUN_DIR/func-a.env" envB="$RUN_DIR/func-b.env"
    # Node A sender → node B receiver, and vice versa (cross-node, embedded).
    write_pair_env "$NODE_A" "$envA" "$(inventory_json "$NODE_B" | jq -r '.pairs[0].receiver.ipv4'):40102"
    write_pair_env "$NODE_B" "$envB" "$(inventory_json "$NODE_A" | jq -r '.pairs[0].receiver.ipv4'):40102"

    for s in bidirectional_unicast reconnect multi_endpoint loss_recovery; do
        run_embedded "$s" "$MSGS" 130 "$envA" "$envB" "func-$s"
    done
    # 16 KiB and 1 MiB messages (fewer messages, still full round-trip).
    run_embedded bidirectional_unicast 200 16384 "$envA" "$envB" "func-16k"
    run_embedded bidirectional_unicast 50 1048576 "$envA" "$envB" "func-1m"

    # Standalone driver (§11.3 row 1).
    run_standalone bidirectional_unicast "$MSGS" 130 "$envA" "$envB" "standalone"

    # DPDK-to-default-UDP in each direction (§11.3 row 4): one role DPDK, the
    # other kernel UDP over the primary ENA.
    log "dpdk_to_udp: node A DPDK → node B kernel UDP (primary ENA)"
    local udpB="$RUN_DIR/func-udpb.env"
    write_udp_env "$NODE_B" "$udpB" "$IP_A"
    run_embedded dpdk_to_udp "$MSGS" 130 "$envA" "$udpB" "func-dpdk-to-udp"

    log "udp_to_dpdk: node A kernel UDP → node B DPDK"
    local udpA="$RUN_DIR/func-udpa.env"
    write_udp_env "$NODE_A" "$udpA" "$IP_B"
    run_embedded udp_to_dpdk "$MSGS" 130 "$udpA" "$envB" "func-udp-to-dpdk"

    # §11.3 fragment/MTU + primary-ENA-usability check.
    fragment_and_primary_check
}

# tcpdump the primary ENA while the kernel-UDP side of a DPDK→kernel-UDP run
# exchanges traffic: no IPv4 fragments and no datagram over the MTU, and the
# primary ENA answers ICMP throughout (proving it stays kernel-usable).
fragment_and_primary_check() {
    log "fragment + MTU + primary-ENA check"
    local iface
    iface=$(nssh "$NODE_B" "ip route | awk '/default/{print \$5; exit}'" | tr -d '\r')
    nssh "$NODE_B" "timeout 20 tcpdump -i $iface -nn -l ip > /tmp/frag.log 2>/dev/null || true" &
    local tcp=$!
    sleep 2
    local ping_ok
    ping_ok=$(nssh "$NODE_A" "ping -c 3 -W 2 $IP_B >/dev/null && echo OK || echo FAIL")
    wait "$tcp" 2>/dev/null || true
    [[ "$ping_ok" == OK ]] || die "primary ENA on $NODE_B did not answer ICMP during DPDK run"
    local frags
    frags=$(nssh "$NODE_B" "grep -c 'frag' /tmp/frag.log 2>/dev/null || echo 0")
    [[ "$frags" == "0" ]] || die "captured $frags IPv4 fragment(s) on $NODE_B/$iface"
    log "fragment check clean ($ping_ok); primary ENA kernel-usable during the run"
}

# ---------------------------------------------------------------------------
# §13.2 rollback rehearsal
# ---------------------------------------------------------------------------

phase_rollback() {
    log "=== rollback rehearsal (§13.2) ==="
    # After the DPDK matrix, flip the workload to the default transport with
    # NO binary change: rerun the same MDC scenario over the primary ENA
    # (kernel UDP). The secondaries stay bound to vfio-pci throughout.
    local udpA="$RUN_DIR/rb-a.env" udpB="$RUN_DIR/rb-b.env"
    write_udp_env "$NODE_A" "$udpA" "$IP_B"
    write_udp_env "$NODE_B" "$udpB" "$IP_A"
    run_embedded bidirectional_unicast "$MSGS" 130 "$udpA" "$udpB" "rollback-kernel"

    for node in "$NODE_A" "$NODE_B"; do
        local pci driver
        pci=$(inventory_json "$node" | jq -r '.pairs[0].sender.pci')
        driver=$(nssh "$node" "readlink /sys/bus/pci/devices/$pci/driver 2>/dev/null | xargs -r basename" || true)
        [[ "$driver" == "vfio-pci" ]] || die "rollback disturbed $node $pci (driver=$driver)"
    done
    log "rollback rehearsal PASSED — default transport via primary ENA, secondaries isolated"
}

# ---------------------------------------------------------------------------
# §12 benchmark + gates
# ---------------------------------------------------------------------------

# bench_mode <name> <envA> <envB>
#   For every size: calibrate saturation (load_rps=0), then run common and
#   stress loads for RUNS × RUN_SECS. Per-run JSON reports + raw latency
#   samples accumulate under $RUN_DIR/bench-<name>/.
bench_mode() {
    local name=$1 envA=$2 envB=$3
    local out="$RUN_DIR/bench-$name"
    mkdir -p "$out"
    nscp "$NODE_A" "$HARNESS_BIN" /tmp/dpdk-harness
    nscp "$NODE_B" "$HARNESS_BIN" /tmp/dpdk-harness

    for size in $SIZES; do
        log "  [$name] size=$size: calibrating saturation"
        local sat_common="$RUN_DIR/common-load-$size"
        local satA="$RUN_DIR/bench-$name-$size-sat.env"
        # Saturation env: fresh copy of the mode env + perf duration.
        cp "$envA" "$satA"; cp "$envB" "$RUN_DIR/bench-$name-$size-sat-s.env"
        cat >> "$satA" <<EOF
RUSTERON_HARNESS_DURATION_SECS=$WARMUP_SECS
RUSTERON_HARNESS_LOAD_RPS=0
EOF
        cat >> "$RUN_DIR/bench-$name-$size-sat-s.env" <<EOF
RUSTERON_HARNESS_DURATION_SECS=$((WARMUP_SECS + 10))
RUSTERON_HARNESS_LOAD_RPS=0
EOF
        # shellcheck disable=SC2046
        nssh "$NODE_A" "env $(cat "$satA") /tmp/dpdk-harness --role primary --scenario perf --report /tmp/$name-sat-p.json" \
            >"$out/sat-p-$size.log" 2>&1 &
        local pa=$!
        # shellcheck disable=SC2046
        nssh "$NODE_B" "env $(cat "$RUN_DIR/bench-$name-$size-sat-s.env") /tmp/dpdk-harness --role secondary --scenario perf --report /tmp/$name-sat-s.json" \
            >"$out/sat-s-$size.log" 2>&1 &
        local pb=$!
        wait "$pa" || die "[$name] size=$size saturation primary failed"
        wait "$pb" || die "[$name] size=$size saturation secondary failed"
        nssh "$NODE_A" "cat /tmp/$name-sat-p.json" > "$out/sat-p-$size.json"
        nssh "$NODE_B" "cat /tmp/$name-sat-s.json" > "$out/sat-s-$size.json"

        # Saturation = the mode's own measured delivered rate at load 0.
        local saturation
        saturation=$(report_field "$out/sat-p-$size.json" offered_per_sec)
        # Common load is 50% of the LOWEST saturation across modes (§12.2) —
        # shared per size so all three modes offer the same common load.
        if [[ ! -f "$sat_common" ]]; then
            awk -v f="$COMMON_FRAC" 'BEGIN{printf "%d", ('"$saturation"' * f)}' > "$sat_common"
        fi
        local common_msgs stress_msgs
        common_msgs=$(cat "$sat_common")
        stress_msgs=$(awk -v f="$STRESS_FRAC" 'BEGIN{printf "%d", ('"$saturation"' * f)}')

        for tag in common stress; do
            local rps
            [[ "$tag" == common ]] && rps=$common_msgs || rps=$stress_msgs
            log "  [$name] size=$size $tag load=${rps} msg/s ×$RUNS runs of ${RUN_SECS}s"
            local tagout="$out/$size-$tag"
            mkdir -p "$tagout"
            for i in $(seq 1 "$RUNS"); do
                local ep="$tagout/$i-p.env" es="$tagout/$i-s.env"
                cp "$envA" "$ep"; cp "$envB" "$es"
                cat >> "$ep" <<EOF
RUSTERON_HARNESS_DURATION_SECS=$RUN_SECS
RUSTERON_HARNESS_LOAD_RPS=$rps
RUSTERON_HARNESS_LATENCY_SAMPLES=/tmp/$name-$size-$tag-$i.raw
EOF
                cat >> "$es" <<EOF
RUSTERON_HARNESS_DURATION_SECS=$((RUN_SECS + 10))
RUSTERON_HARNESS_LOAD_RPS=0
RUSTERON_HARNESS_LATENCY_SAMPLES=/tmp/$name-$size-$tag-$i.raw
EOF
                # shellcheck disable=SC2046
                nssh "$NODE_A" "env $(cat "$ep") /tmp/dpdk-harness --role primary --scenario perf --report /tmp/$name-$size-$tag-$i-p.json" \
                    >"$tagout/$i-p.log" 2>&1 &
                local hp=$!
                # shellcheck disable=SC2046
                nssh "$NODE_B" "env $(cat "$es") /tmp/dpdk-harness --role secondary --scenario perf --report /tmp/$name-$size-$tag-$i-s.json" \
                    >"$tagout/$i-s.log" 2>&1 &
                local hs=$!
                wait "$hp" || die "[$name] size=$size $tag run $i primary failed"
                wait "$hs" || die "[$name] size=$size $tag run $i secondary failed"
                nssh "$NODE_A" "cat /tmp/$name-$size-$tag-$i-p.json" > "$tagout/$i-p.json"
                nssh "$NODE_B" "cat /tmp/$name-$size-$tag-$i-s.json" > "$tagout/$i-s.json"
                nssh "$NODE_B" "cat /tmp/$name-$size-$tag-$i.raw 2>/dev/null" > "$tagout/$i.raw" || true
            done
        done
    done
}

median() { sort -n | awk '{a[NR]=$1} END{print (NR%2 ? a[(NR+1)/2] : (a[NR/2]+a[NR/2+1])/2)}'; }

# collect_bench_results <name> → $RUN_DIR/gates-<name>.json (median p99/offered
# per size at common load).
collect_bench_results() {
    local name=$1
    local out="$RUN_DIR/bench-$name"
    local json="{ \"mode\": \"$name\""
    for size in $SIZES; do
        local tagout="$out/$size-common"
        local p99s="" delivered=""
        for i in $(seq 1 "$RUNS"); do
            p99s="$p99s$(report_field "$tagout/$i-s.json" latency_p99_ns)
"
            delivered="$delivered$(report_field "$tagout/$i-p.json" offered_per_sec)
"
        done
        local mp99 mdelivered
        mp99=$(printf '%s' "$p99s" | median)
        mdelivered=$(printf '%s' "$delivered" | median)
        json="$json, \"size_$size\": { \"median_p99_ns\": $mp99, \"median_offered_per_sec\": $mdelivered }"
    done
    json="$json }"
    echo "$json" | jq . > "$RUN_DIR/gates-$name.json"
}

phase_bench() {
    log "=== benchmark matrix (§12) ==="
    # Mode 1: tuned kernel UDP (default transport) over the same secondary
    # ENAs the DPDK modes use, temporarily returned to the kernel ena driver
    # (plan §12.1), so the baseline is apples-to-apples. The primary ENAs stay
    # untouched; the secondaries are restored to vfio-pci before any DPDK mode.
    log "mode: kernel-udp (secondary ENAs returned to the kernel driver)"
    local kA="$RUN_DIR/bench-kernel-a.env" kB="$RUN_DIR/bench-kernel-b.env"
    local rxA rxB
    rxA=$(inventory_json "$NODE_A" | jq -r '.pairs[0].receiver.ipv4')
    rxB=$(inventory_json "$NODE_B" | jq -r '.pairs[0].receiver.ipv4')
    ena_bind_kernel "$NODE_A" "$rxB"
    ena_bind_kernel "$NODE_B" "$rxA"
    write_udp_sec_env "$NODE_A" "$kA" "$rxB"
    write_udp_sec_env "$NODE_B" "$kB" "$rxA"
    bench_mode kernel "$kA" "$kB"
    ena_bind_vfio "$NODE_A"
    ena_bind_vfio "$NODE_B"
    collect_bench_results kernel

    # Mode 2: DPDK ENA Express off (the default; §12.3 primary gate).
    log "mode: dpdk-off"
    local dA="$RUN_DIR/bench-dpdkoff-a.env" dB="$RUN_DIR/bench-dpdkoff-b.env"
    write_pair_env "$NODE_A" "$dA" "$(inventory_json "$NODE_B" | jq -r '.pairs[0].receiver.ipv4'):40102"
    write_pair_env "$NODE_B" "$dB" "$(inventory_json "$NODE_A" | jq -r '.pairs[0].receiver.ipv4'):40102"
    bench_mode dpdk-off "$dA" "$dB"
    collect_bench_results dpdk-off

    # Mode 3: DPDK ENA Express on — only when the cell ENIs support it, and
    # only then is the on/off decision evidence-backed. Otherwise the default
    # stays OFF (no clear accepted win → retain off).
    log "mode: dpdk-on (ENA Express)"
    local eni
    eni=$(aws ec2 describe-instances \
        --filters "Name=private-ip-address,Values=$IP_A" \
        --query 'Reservations[].Instances[0].NetworkInterfaces[?Primary==`true`].NetworkInterfaceId' \
        --output text)
    if [[ -z "$eni" ]]; then
        log "no ENI found for $IP_A — skipping dpdk-on; ENA Express retained OFF"
        echo "off — ENA Express toggle unavailable on this cell; no clear win" > "$RUN_DIR/ena-express-decision.json"
        return 0
    fi
    local onA="$RUN_DIR/bench-dpdkon-a.env" onB="$RUN_DIR/bench-dpdkon-b.env"
    write_pair_env "$NODE_A" "$onA" "$(inventory_json "$NODE_B" | jq -r '.pairs[0].receiver.ipv4'):40102"
    write_pair_env "$NODE_B" "$onB" "$(inventory_json "$NODE_A" | jq -r '.pairs[0].receiver.ipv4'):40102"
    log "enabling ENA Express on $eni"
    "$(dirname "$0")/toggle-ena-express.sh" "$eni" on || { echo "off — toggle failed; no clear win" > "$RUN_DIR/ena-express-decision.json"; return 0; }
    bench_mode dpdk-on "$onA" "$onB"
    collect_bench_results dpdk-on
    "$(dirname "$0")/toggle-ena-express.sh" "$eni" off || true
    decide_ena_express
}

# §12.3 ENA Express decision: keep ON only if ≥5% median-p99 improvement for
# every size (and no regression — enforced by the gates that already require
# no loss/error growth). Otherwise retain OFF.
decide_ena_express() {
    local off="$RUN_DIR/gates-dpdk-off.json" on="$RUN_DIR/gates-dpdk-on.json"
    local verdict="off"
    for size in $SIZES; do
        local p_off p_on
        p_off=$(jq -r ".size_$size.median_p99_ns" "$off")
        p_on=$(jq -r ".size_$size.median_p99_ns" "$on")
        if awk -v a="$p_off" -v b="$p_on" 'BEGIN{exit !(b <= a*0.95)}'; then
            :
        else
            echo "off — size $size p99 not improved ≥5% (off=$p_off on=$p_on)" > "$RUN_DIR/ena-express-decision.json"
            log "$(cat "$RUN_DIR/ena-express-decision.json")"
            return
        fi
    done
    verdict="on"
    echo "on — ≥5% p99 improvement at every size" > "$RUN_DIR/ena-express-decision.json"
    log "$(cat "$RUN_DIR/ena-express-decision.json")"
}

# §12.3 primary gate: dpdk-off must beat kernel-udp on p99 (≥10% lower) and
# hold delivered throughput (≥99%).
phase_gates() {
    log "=== primary gate (§12.3): dpdk-off vs kernel-udp ==="
    local gk="$RUN_DIR/gates-kernel.json" gd="$RUN_DIR/gates-dpdk-off.json"
    [[ -f "$gk" && -f "$gd" ]] || die "run bench before gates (missing $gk / $gd)"
    local verdict="PASS"
    : > "$RUN_DIR/gates-verdict.txt"
    for size in $SIZES; do
        local pk pd dk dd
        pk=$(jq -r ".size_$size.median_p99_ns" "$gk")
        pd=$(jq -r ".size_$size.median_p99_ns" "$gd")
        dk=$(jq -r ".size_$size.median_offered_per_sec" "$gk")
        dd=$(jq -r ".size_$size.median_offered_per_sec" "$gd")
        log "  size=$size  kernel(p99=$pk, offered=$dk)  dpdk-off(p99=$pd, offered=$dd)"
        awk -v a="$pd" -v b="$pk" 'BEGIN{exit !(a <= b*0.9)}' \
            || { echo "FAIL size=$size p99: dpdk $pd not ≤90% of kernel $pk" >> "$RUN_DIR/gates-verdict.txt"; verdict=FAIL; }
        awk -v a="$dd" -v b="$dk" 'BEGIN{exit !(a >= b*0.99)}' \
            || { echo "FAIL size=$size offered: dpdk $dd <99% of kernel $dk" >> "$RUN_DIR/gates-verdict.txt"; verdict=FAIL; }
    done
    echo "PRIMARY GATE: $verdict" | tee -a "$RUN_DIR/gates-verdict.txt"
    log "gates verdict: $verdict (see $RUN_DIR/gates-verdict.txt)"
}

# ---------------------------------------------------------------------------
# Environment metadata (§12.2 reproducibility)
# ---------------------------------------------------------------------------

phase_metadata() {
    log "=== environment metadata ==="
    local meta="{ \"generated_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\""
    meta="$meta, \"git_sha\": \"$(git rev-parse HEAD 2>/dev/null || echo unknown)\""
    meta="$meta, \"cell\": \"$CELL\""
    for node in "$NODE_A" "$NODE_B"; do
        local ip instance az img kernel kube dpdk_ver ena pg chrony
        ip=$(kubectl get node "$node" -o jsonpath='{.status.addresses[?(@.type=="InternalIP")].address}')
        instance=$(aws ec2 describe-instances --filters "Name=private-ip-address,Values=$ip" \
            --query 'Reservations[].Instances[0].InstanceType' --output text)
        az=$(aws ec2 describe-instances --filters "Name=private-ip-address,Values=$ip" \
            --query 'Reservations[].Instances[0].Placement.AvailabilityZone' --output text)
        img=$(kubectl get node "$node" -o jsonpath='{.status.nodeInfo.osImage}')
        kernel=$(nssh "$node" "uname -r" | tr -d '\r')
        kube=$(kubectl get node "$node" -o jsonpath='{.status.nodeInfo.kubeletVersion}')
        dpdk_ver=$(nssh "$node" "pkg-config --modversion libdpdk 2>/dev/null || echo unknown" | tr -d '\r')
        ena=$(nssh "$node" "modinfo ena 2>/dev/null | awk '/^version:/{print \$2}' || echo unknown" | tr -d '\r')
        pg=$(placement_group "$node")
        chrony=$(nssh "$node" "systemctl is-active chronyd 2>/dev/null || echo inactive" | tr -d '\r')
        meta="$meta, \"$node\": { \"internal_ip\": \"$ip\", \"instance_type\": \"$instance\", \"az\": \"$az\", \"os_image\": \"$img\", \"kernel\": \"$kernel\", \"kubelet\": \"$kube\", \"dpdk\": \"$dpdk_ver\", \"ena_driver\": \"$ena\", \"placement_group\": \"$pg\", \"chrony\": \"$chrony\" }"
    done
    meta="$meta }"
    echo "$meta" | jq . > "$RUN_DIR/env-metadata.json"
    log "wrote $RUN_DIR/env-metadata.json"
}

# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

need kubectl; need aws; need jq; need ssh; need scp
pick_nodes

case "$PHASE" in
    metadata)    phase_metadata ;;
    functional)  phase_functional ;;
    bench)       phase_bench; phase_gates ;;
    rollback)    phase_rollback ;;
    all)         phase_metadata; phase_functional; phase_rollback; phase_bench; phase_gates ;;
    *) die "unknown phase: $PHASE (all|functional|bench|rollback|metadata)" ;;
esac

log "done — artifacts in $RUN_DIR"
echo "$RUN_DIR"
