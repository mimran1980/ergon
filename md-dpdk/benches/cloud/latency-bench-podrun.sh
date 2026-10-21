#!/usr/bin/env bash
# One latency-pods lab run inside a pod (latency-bench.d/*.yaml's command).
# Two nodes, two pods, cross-host only.
#
#   latency-bench-podrun.sh ping <ping ep> <pong ep> [messages] [warmup]   (node a)
#   latency-bench-podrun.sh pong <ping ep> <pong ep>                       (node b)
#
# Runs `rtt` (ergon-md-dpdk-harness) cross-host with HARNESS_DRIVER=invoker:
# the measuring thread is also the client conductor and the media driver, one
# spinner per pod, which fits cpu: 2 on c8g.xlarge (exclusive 2,3 + shared 0).
# The measuring loop pins to the first ERGON_ISOLATED_CPUS CPU (the shared-cpus
# plugin gives the pod both isolated CPUs; this script keeps the busy-spin off
# the shared one). The second isolated CPU runs gapspin, so every max can be
# told from a host stall. Busy helpers (none here) would stay on
# ERGON_SHARED_CPUS.
#
# When the plugin set ERGON_ISOLATED_CGROUP (cpu-partition.ergon.io), the
# measuring process is first moved into that child cgroup and only then pinned
# to the exclusive CPU -- the kernel rejects a partition write if a thread
# outside the child holds an exclusive CPU, so move-then-pin is the order. The
# move is proved before any number counts, and a run that cannot show its
# measuring task inside an isolated partition exits rather than measure
# outside it. A mode that asked for a partition (LATENCY_REQUIRE_PARTITION=1)
# exits when the grant is missing; a mode that did not asks for none and says
# so in meta.txt.
#
# When the plugin set ERGON_HUGEPAGES_DIR (hugepages.ergon.io), AERON_DIR is
# $ERGON_HUGEPAGES_DIR/aeron and AERON_FILE_PAGE_SIZE is $ERGON_HUGEPAGE_SIZE,
# unless the manifest already set them.
#
# Writes to $LATENCY_OUT (an emptyDir the lab script collects):
#   rtt-line.txt        xping's one line: rtt,<label>,udp,p50,...,samples,lost
#   rtt-histogram.csv   RTT_HISTOGRAM
#   spikes.csv          LOAD_SPIKES (LOAD_SPIKE_NS=100000)
#   stalls-ping.csv / stalls-pong.csv   LOOP_STALLS_DIR (LOOP_STALL_NS=20000)
#   gapspin-a.txt / gapspin-b.txt       gapspin on the spare isolated CPU
#   vmstat.txt          pgfault/pgmajfault around the run (/proc/vmstat)
#   meta.txt            env the run saw (AERON_DIR mount type, ulimits, cgroup)
#
# No prelock, no SCHED_FIFO: measured not worth it on AWS (io/benches/BENCHMARK.md).
# Nothing here adds capabilities; rlimits ride along from the NRI plugin.
set -uo pipefail

log() { echo "[latency-bench-podrun ${role:-lib} $(date +%T)] $*" >&2; }
die() { log "ERROR: $*"; exit 1; }

# ---------------------------------------------------------------- cgroup
# Sourced by test/test-latency-bench-podrun.sh (LATENCY_PODRUN_LIB=1) against a fake
# tree: these three are the whole placement contract, and they are tested
# behaviourally rather than by grepping this file.
cgroup_root=${LATENCY_CGROUP_ROOT:-/sys/fs/cgroup}

# ERGON_ISOLATED_CGROUP is the node's path (nriplugin.partitionChild). Inside
# the pod's own cgroup namespace the child is a directory of the namespace
# root, so a path that is not a directory is retried under the cgroup root by
# name. Prints the resolved child, fails when there is none.
resolve_child() {
    local p=${ERGON_ISOLATED_CGROUP:-} alt
    [[ -n $p ]] || return 1
    if [[ -d $p ]]; then
        printf '%s\n' "$p"
        return 0
    fi
    alt=$cgroup_root/$(basename "$p")
    if [[ -d $alt ]]; then
        printf '%s\n' "$alt"
        return 0
    fi
    return 1
}

# cpu_in_mask <cpu> <taskset mask>: the mask is a comma list of numbers and
# ranges ("2", "2,3", "0-3,8-11"), the way taskset -pc prints it.
cpu_in_mask() {
    local cpu=$1 mask=$2 r
    local IFS=,
    for r in $mask; do
        if [[ $r == *-* ]]; then
            ((cpu >= ${r%%-*} && cpu <= ${r##*-})) && return 0
        elif [[ $r == "$cpu" ]]; then
            return 0
        fi
    done
    return 1
}

# enter_partition: move this process into the isolated child and require the
# child to read back as an isolated partition. Sets `child` to the path and
# prints it. With no grant at all `child` is empty and it returns 0 -- unless
# LATENCY_REQUIRE_PARTITION is set, which is how a mode that asked for
# cpu-partition.ergon.io says a missing grant is a failure rather than a
# control run.
#
# Call it in the current shell, never as `child=$(enter_partition)`. A command
# substitution runs the function in a subshell, so `die`'s `exit 1` stops only
# that subshell: the runner goes on with `child=""`, and `prove_placed` then
# skips the cgroup-membership proof and the run measures outside the
# partition.
enter_partition() {
    local part
    child=
    if [[ -z ${ERGON_ISOLATED_CGROUP:-} ]]; then
        if [[ ${LATENCY_REQUIRE_PARTITION:-} == 1 ]]; then
            die "LATENCY_REQUIRE_PARTITION=1 but ERGON_ISOLATED_CGROUP is unset: no partition grant"
        fi
        echo "partition=none child=none" >>"${out:?}/meta.txt"
        log "no cpu-partition grant: the measuring task stays in the container cgroup"
        return 0
    fi
    child=$(resolve_child) ||
        die "no isolated cgroup under ERGON_ISOLATED_CGROUP=$ERGON_ISOLATED_CGROUP (tried it and $cgroup_root/$(basename "${ERGON_ISOLATED_CGROUP}"))"
    [[ -w $child/cgroup.procs ]] || die "$child/cgroup.procs is not writable"
    echo $$ >"$child/cgroup.procs" 2>/dev/null || die "cannot move $$ into $child"
    part=$(cat "$child/cpuset.cpus.partition" 2>/dev/null || echo unknown)
    echo "partition=$part child=$child" >>"${out:?}/meta.txt"
    [[ $part == isolated ]] || die "$child/cpuset.cpus.partition is '$part', expected 'isolated'"
    printf '%s\n' "$child"
}

# prove_placed <pid> <cpu> [child]: the measuring task is in the isolated
# child and on the CPU the grant named, before any number counts. cgroup.procs
# is write-only in a threaded child (names.go), so membership is read from
# cgroup.threads. Affinity is set inside rtt after it parses its arguments, so
# it is retried briefly rather than sampled once.
prove_placed() {
    local pid=$1 cpu=$2 child=${3:-} mask= i
    if [[ -n $child ]]; then
        grep -qx "$pid" "$child/cgroup.threads" 2>/dev/null ||
            die "pid $pid is not in $child (cgroup.threads: $(tr '\n' ' ' <"$child/cgroup.threads" 2>/dev/null))"
    fi
    for i in 1 2 3 4 5 6 7 8 9 10; do
        mask=$(taskset -pc "$pid" 2>/dev/null | sed 's/.*: //') ||
            die "cannot read the affinity of pid $pid"
        if cpu_in_mask "$cpu" "$mask"; then
            log "placed: pid $pid on cpu $cpu${child:+ in $child} (affinity $mask)"
            return 0
        fi
        sleep 0.2
    done
    die "pid $pid is not on CPU $cpu (affinity $mask)"
}

if [[ ${LATENCY_PODRUN_LIB:-} == 1 ]]; then
    return 0 2>/dev/null || exit 0
fi

role=${1:?latency-bench-podrun.sh: ping|pong <ping ep> <pong ep> [messages] [warmup]}
case $role in
ping | pong) ;;
*)
    echo "latency-bench-podrun.sh: role ping|pong, got $role" >&2
    exit 2
    ;;
esac
ping_ep=${2:?ping endpoint host:port}
pong_ep=${3:?pong endpoint host:port}
messages=${4:-${RTT_MESSAGES:-300000}}
warmup=${5:-${RTT_WARMUP:-50000}}
out=${LATENCY_OUT:-/out}
mkdir -p "$out"

# The measuring CPU: the first isolated CPU the plugin named. The gap spinner
# takes the second. Never a shared CPU: the loop is the hot path.
IFS=, read -r -a isolated <<<"${ERGON_ISOLATED_CPUS:-}"
if [[ ${#isolated[@]} -lt 1 || -z ${isolated[0]} ]]; then
    die "ERGON_ISOLATED_CPUS is unset: run with the shared-cpus plugin (ergon.io/shared-millicpus)"
fi
meas_cpu=${isolated[0]}
spin_cpu=${isolated[1]:-}

# Hugepages: the plugin's sized hugetlbfs is the AERON_DIR when it is there.
if [[ -n ${ERGON_HUGEPAGES_DIR:-} ]]; then
    export AERON_DIR=${AERON_DIR:-$ERGON_HUGEPAGES_DIR/aeron}
    export AERON_FILE_PAGE_SIZE=${AERON_FILE_PAGE_SIZE:-${ERGON_HUGEPAGE_SIZE:-2097152}}
    mkdir -p "${AERON_DIR}" 2>/dev/null || die "cannot create AERON_DIR=$AERON_DIR"
fi
: "${AERON_DIR:=/aeron}"

export HARNESS_DRIVER=${HARNESS_DRIVER:-invoker}
export LOAD_SPIKE_NS=${LOAD_SPIKE_NS:-100000}
export LOOP_STALL_NS=${LOOP_STALL_NS:-20000}
export RTT_HISTOGRAM=$out/rtt-histogram.csv
export LOAD_SPIKES=$out/spikes.csv
export LOOP_STALLS_DIR=$out
export RTT_SECONDS=${RTT_SECONDS:-60}

# What the run saw, so a max can be read against the mode.
{
    echo "role=$role"
    echo "meas_cpu=$meas_cpu spin_cpu=${spin_cpu:-none}"
    echo "isolated=${ERGON_ISOLATED_CPUS:-} shared=${ERGON_SHARED_CPUS:-}"
    echo "isolated_cgroup=${ERGON_ISOLATED_CGROUP:-unset}"
    echo "AERON_DIR=$AERON_DIR file_page_size=${AERON_FILE_PAGE_SIZE:-unset}"
    echo "hugepages_dir=${ERGON_HUGEPAGES_DIR:-unset} hugepage_size=${ERGON_HUGEPAGE_SIZE:-unset}"
    findmnt -T "$AERON_DIR" -o FSTYPE,SOURCE,OPTIONS -n 2>/dev/null || true
    echo "sched=$(cut -d' ' -f2 /proc/self/stat 2>/dev/null) policy=$(chrt -p $$ 2>&1 | tr '\n' ' ')"
    echo "memlock=$(ulimit -l) nofile=$(ulimit -n) rtprio=$(ulimit -r)"
    echo "seccomp=${ERGON_SECCOMP:-unset} ptp=${ERGON_PTP_DEVICE:-unset} cpu_dma_latency=${ERGON_CPU_DMA_LATENCY:-unset}"
} >"$out/meta.txt"

vmstat_line() { awk '/^pgfault |^pgmajfault /{printf "%s %s\n", $1, $2}' /proc/vmstat; }

# Move-then-pin: into the isolated child cgroup first, then onto the CPU. The
# hook created the partition but moved no tasks; a thread outside the child
# cannot hold an exclusive CPU of a valid isolated partition. Fails closed:
# `enter_partition` is called in the current shell so a missing or unusable
# child stops this script (see the comment on the function).
child=
enter_partition >/dev/null

gapspin_bg=
cleanup() {
    [[ -n ${gapspin_bg:-} ]] && kill "$gapspin_bg" 2>/dev/null
    wait 2>/dev/null
}
trap cleanup EXIT INT TERM

if [[ -n $spin_cpu ]]; then
    # 200 us threshold: anything shorter is not a round trip's max at this scale
    gapspin "$spin_cpu" 200 ${RTT_SECONDS:-60} >"$out/gapspin-$role.txt" 2>>"$out/gapspin-$role.log" &
    gapspin_bg=$!
    log "gapspin on CPU $spin_cpu (pid $gapspin_bg)"
else
    log "no second isolated CPU: no gapspin, host stalls cannot be ruled out"
fi

before=$(vmstat_line)
log "rtt on CPU $meas_cpu, $messages messages after $warmup warmup, AERON_DIR=$AERON_DIR"
rc=0
case $role in
ping)
    rtt xping "$ping_ep" "$pong_ep" "$messages" "$warmup" "$meas_cpu" >"$out/rtt-line.txt" 2>>"$out/rtt-stderr.log" &
    app=$!
    prove_placed "$app" "$meas_cpu" "$child"
    wait "$app" || rc=$?
    ;;
pong)
    # xpong stops on SIGTERM (the lab script sends it after xping prints)
    rtt xpong "$ping_ep" "$pong_ep" "$meas_cpu" >"$out/rtt-pong.log" 2>>"$out/rtt-stderr.log" &
    app=$!
    echo "$app" >"$out/pong.pid"
    prove_placed "$app" "$meas_cpu" "$child"
    wait "$app" || rc=$?
    ;;
esac
after=$(vmstat_line)
paste <(echo "$before") <(echo "$after" | awk '{print $2}') 2>/dev/null |
    awk '{print $1, $2, $3}' >"$out/vmstat.txt" || { echo "$before" >"$out/vmstat.txt"; }

if [[ $role == ping ]]; then
    cat "$out/rtt-line.txt"
    log "done rc=$rc: $(cat "$out/rtt-line.txt" 2>/dev/null)"
else
    log "pong done rc=$rc"
fi
exit $rc
