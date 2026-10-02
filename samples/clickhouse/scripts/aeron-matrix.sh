#!/usr/bin/env bash
# Aeron round-trip latency on one node across driver threading modes,
# transports, loads and idle strategies, under the CPU policy the node is in
# now. Each run appends one JSON line (aeron-bench's, plus the run's settings).
#
#   scripts/aeron-matrix.sh run OUT.jsonl POLICY     the matrix, labelled POLICY
#   scripts/aeron-matrix.sh exclusive on|off         confine everything else to
#                                                    CPUs 0-1 (cgroup cpusets, as
#                                                    Kubernetes' static CPU manager)
#   scripts/aeron-matrix.sh isolate on|off           kernel isolcpus/nohz_full on
#                                                    CPUs 2-3; reboots the node
#
# POLICY decides pinning: `shared` pins nothing; any other pins pong to CPU 2,
# ping to CPU 3 and a standalone driver to CPUs 0-1. Needs target/release/
# aeron-bench and aeron-driver (cargo build --release -p aeron-bench -p aeron-driver).
# Runs: COUNT (default 300000) measured round trips after WARMUP (50000).
# Archive runs need java and target/aeron-all-*.jar (`just _aeron-jar`); the
# archive records to ARCHIVE_DIR (default ~/aeron-matrix-archive), on disk.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
bench=$root/target/release/aeron-bench
driver_bin=$root/target/release/aeron-driver
count=${COUNT:-300000}
warmup=${WARMUP:-50000}
hot=2,3
rest=0-1

pin() { if [[ $policy == shared ]]; then "${@:2}"; else taskset -c "$1" "${@:2}"; fi; }

# Bench processes run in a scope allowed every CPU, so `exclusive` (which
# confines the user and system slices to 0-1) does not confine them too.
scope() { sudo systemd-run --quiet --scope --uid="$(id -u)" -p AllowedCPUs=0-3 --setenv=AERON_DIR="$dir" "$@"; }

# A Java archive on the run's driver, threading mode $1, pinned with the
# driver. It records to disk, as the lab's does.
start_archive() {
    local jar
    jar=$(ls "$root"/target/aeron-all-*.jar | head -1)
    rm -rf "${ARCHIVE_DIR:-$HOME/aeron-matrix-archive}"
    scope bash -c "$(declare -f pin); policy=$policy; pin $rest java -Xms64m -Xmx256m -XX:+UseSerialGC \
        --add-opens java.base/jdk.internal.misc=ALL-UNNAMED -cp $jar \
        -Daeron.dir=$dir -Daeron.archive.dir=${ARCHIVE_DIR:-$HOME/aeron-matrix-archive} \
        -Daeron.archive.threading.mode=$(tr a-z A-Z <<<"$1") \
        -Daeron.archive.control.channel='aeron:udp?endpoint=localhost:18010' \
        -Daeron.archive.replication.channel='aeron:udp?endpoint=localhost:0' \
        io.aeron.archive.Archive" >/dev/null 2>&1 &
    sleep 6
}

run_one() {
    local mode=$1 transport=$2 rate=$3 idle=$4 size=$5 archive_mode=${6:-}
    local ping=aeron:ipc pong=aeron:ipc driver_pid= pong_pid= record=no
    if [[ $transport == udp ]]; then
        ping='aeron:udp?endpoint=127.0.0.1:24001'
        pong='aeron:udp?endpoint=127.0.0.1:24002'
    fi
    dir=/dev/shm/aeron-matrix
    rm -rf "$dir"
    local pong_driver=external
    if [[ $mode == invoker ]]; then
        pong_driver=invoker
    else
        AERON_THREADING_MODE=$(tr a-z- A-Z_ <<<"$mode") AERON_DIR_DELETE_ON_START=true \
            AERON_SENDER_IDLE_STRATEGY=spin AERON_RECEIVER_IDLE_STRATEGY=spin \
            AERON_SHAREDNETWORK_IDLE_STRATEGY=spin AERON_SHARED_IDLE_STRATEGY=spin \
            AERON_CONDUCTOR_IDLE_STRATEGY=sleep-ns \
            scope env AERON_DIR="$dir" bash -c "$(declare -f pin); policy=$policy; pin $rest $driver_bin" >/dev/null 2>&1 &
        driver_pid=$!
        for _ in $(seq 100); do [[ -e $dir/cnc.dat ]] && break; sleep 0.1; done
    fi
    if [[ -n $archive_mode ]]; then
        record=yes
        start_archive "$archive_mode"
    fi
    scope bash -c "$(declare -f pin); policy=$policy; pin 2 $bench pong --driver $pong_driver --idle $idle --ping '$ping' --pong '$pong'" 2>/dev/null &
    pong_pid=$!
    sleep 2
    local result
    result=$(scope bash -c "$(declare -f pin); policy=$policy; pin 3 $bench ping --idle $idle --rate $rate --size $size \
        --count $count --warmup $warmup --ping '$ping' --pong '$pong' --record $record --label x" 2>/dev/null) || result=
    sudo pkill -f "aeron-bench pong" || true
    local replayed=
    if [[ $record == yes && -n $result ]]; then
        replayed=$(scope bash -c "$(declare -f pin); policy=$policy; pin 3 $bench replay --idle spin" 2>/dev/null) || replayed=
    fi
    sudo pkill -f io.aeron.archive.Archive || true
    [[ -n $driver_pid ]] && { sudo pkill -f "$driver_bin" || true; }
    wait 2>/dev/null || true
    if [[ -z $result ]]; then
        echo "FAILED $policy $mode $transport rate=$rate idle=$idle size=$size" >&2
        return
    fi
    printf '%s\n' "$result" | python3 -c "
import json,sys
r=json.loads(sys.stdin.read())
rep=json.loads('''$replayed''' or 'null')
if rep: r.update(replay_first_ns=rep['first_ns'], replay_msgs_per_s=rep['msgs_per_s'], replay_mb_per_s=rep['mb_per_s'])
r.update(archive='${archive_mode:-none}', policy='$policy', mode='$mode', transport='$transport', host='$(hostname)', cpu='$(lscpu | sed -n 's/Model name: *//p')', load1=float(open('/proc/loadavg').read().split()[0]))
r.pop('label', None); r.pop('driver', None)
print(json.dumps(r))" >> "$out"
    tail -1 "$out" | python3 -c "import json,sys; r=json.load(sys.stdin); print(f\"{r['policy']:9s} {r['archive']:9s} {r['mode']:14s} {r['transport']:3s} rate={r['rate']:>7} idle={r['idle']:7s} size={r['size']:4} p50={r['p50_ns']:>7} p99={r['p99_ns']:>8} p99.9={r['p999_ns']:>9} max={r['max_ns']:>10}\")"
}

run() {
    out=$1 policy=$2
    local mode transport rate idle
    for mode in dedicated shared-network shared invoker; do
        for transport in ipc udp; do
            for rate in 0 100000 500000; do
                run_one "$mode" "$transport" "$rate" spin 64
            done
        done
    done
    # Idle strategies and a larger message, on the pinned DEDICATED and INVOKER paths.
    for mode in dedicated invoker; do
        for idle in backoff sleep; do run_one "$mode" ipc 0 "$idle" 64; done
        run_one "$mode" ipc 100000 spin 512
    done
    # The archive recording the ping stream, then replaying it.
    local archive_mode
    for archive_mode in dedicated shared; do
        for mode in dedicated invoker; do
            for rate in 0 100000 500000; do
                run_one "$mode" ipc "$rate" spin 64 "$archive_mode"
            done
        done
    done
}

exclusive() {
    local cpus=0-3
    [[ $1 == on ]] && cpus=$rest
    for slice in system.slice user.slice init.scope kubepods.slice; do
        sudo systemctl set-property --runtime "$slice" AllowedCPUs="$cpus" 2>/dev/null || true
    done
    # k3s may put pods outside a systemd slice: confine its cgroup directly.
    [[ -d /sys/fs/cgroup/kubepods ]] && echo "$cpus" | sudo tee /sys/fs/cgroup/kubepods/cpuset.cpus >/dev/null || true
    grep -h Cpus_allowed_list /proc/1/status
}

isolate() {
    local args="isolcpus=$hot nohz_full=$hot rcu_nocbs=$hot irqaffinity=$rest"
    if [[ $1 == on ]]; then
        sudo sed -i "s|^GRUB_CMDLINE_LINUX_DEFAULT=\"\\(.*\\)\"|GRUB_CMDLINE_LINUX_DEFAULT=\"\\1 $args\"|" /etc/default/grub
    else
        sudo sed -i "s| $args||" /etc/default/grub
    fi
    grep ^GRUB_CMDLINE_LINUX_DEFAULT /etc/default/grub
    sudo update-grub >/dev/null 2>&1
    sudo systemctl reboot
}

case "${1:-}" in
    run) run "$2" "$3" ;;
    exclusive | isolate) "$1" "$2" ;;
    *) echo "usage: $0 run OUT POLICY | exclusive on|off | isolate on|off" >&2; exit 2 ;;
esac
