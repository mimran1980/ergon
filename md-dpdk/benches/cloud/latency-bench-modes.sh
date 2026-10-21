# latency-pod extensions benchmark: the mode set, what each mode changes, and the CSV
# mode names the results must use. One place, so the lab script, the pod runner,
# the CSV emitter and the report agree.
#
# Sourced by the lab (aws/eks-nri.sh) and by the collectors.
#
# The mode set is M0..M6 (aws/eks-nri.sh and the M0..M6 mode table). One CSV
# line per measured rep. A mode that cannot run on a node emits no CSV line;
# its reason goes to benchmarks.md "Not measured / open".
#
#   baseline=M0  hugepages=M1  partition=M2  seccomp=M3
#   devices=M4   probe=M5      all=M6
#
# latency_bench_modes: the enum, in run order.
# latency_bench_mode_ok <name>: 0 when <name> is in the set.
# latency_bench_mode_canonical <name>: the mode name (the set is the canonical set).
# latency_bench_mode_manifest <mode>: the manifest under latency-bench.d/.
# latency_bench_mode_env <mode>: extra KEY=value lines the pods export on top of
#   the baseline ones, one per line.
# latency_bench_mode_desc <mode>: one plain line for benchmarks.md.
# latency_bench_mode_implies <mode>: the pieces the mode also carries (for "all"
#   and cross-checks), one per line.

LATENCY_BENCH_MODES="baseline hugepages partition seccomp devices probe all"

latency_bench_mode_ok() {
    local m
    for m in $LATENCY_BENCH_MODES; do
        [[ $m == "$1" ]] && return 0
    done
    return 1
}

# latency_bench_mode_canonical <name>: the mode name. Fails on a name not in the
# set.
latency_bench_mode_canonical() {
    local c
    for c in $LATENCY_BENCH_MODES; do
        [[ $c == "$1" ]] && {
            echo "$c"
            return 0
        }
    done
    return 1
}

latency_bench_mode_manifest() {
    case $1 in
    baseline) echo baseline.yaml ;;
    hugepages) echo hugepages.yaml ;;
    partition) echo partition.yaml ;;
    seccomp) echo seccomp.yaml ;;
    devices) echo devices.yaml ;;
    probe) echo probe.yaml ;;
    all) echo all.yaml ;;
    *) return 1 ;;
    esac
}

# Env on top of baseline. The plugin's opt-in annotations do most of the work;
# these are the values the app reads (bench-harness.md "Env the app reads").
latency_bench_mode_env() {
    case $1 in
    hugepages | all)
        cat <<'EOF'
AERON_DIR=/dev/hugepages/aeron
AERON_FILE_PAGE_SIZE=2097152
EOF
        ;;
    esac
    return 0
}

latency_bench_mode_desc() {
    case $1 in
    baseline) echo "M0: plain emptyDir AERON_DIR, default seccomp, no extension annotations, no probe, SCHED_OTHER" ;;
    hugepages) echo "M1: sized hugetlbfs on /dev/hugepages via NRI AddMount (pagesize=2M,size=), AERON_DIR there" ;;
    partition) echo "M2: cpuset.cpus.partition=isolated on the exclusive CPUs via the createRuntime child cgroup" ;;
    seccomp) echo "M3: RuntimeDefault plus the io_uring_* allow rule (never Unconfined)" ;;
    devices) echo "M4: PHC /dev/ptp* read-only via NRI AddDevice; cpuqos only on x86 (this lab is Graviton: PHC only)" ;;
    probe) echo "M5: baseline plus a livenessProbe.exec every 1s: the harm the admission extension removes" ;;
    all) echo "M6: hugepages + partition + seccomp + devices, no probe" ;;
    *) return 1 ;;
    esac
}

latency_bench_mode_implies() {
    case $1 in
    all)
        cat <<'EOF'
hugepages
partition
seccomp
devices
EOF
        ;;
    esac
    return 0
}

# The NRI plugins every measured pod names. Required so a missing plugin fails
# the create instead of silently measuring the wrong thing.
LATENCY_BENCH_REQUIRED_PLUGINS='["ergon-shared-cpus","ergon-latency-pods-ext"]'
