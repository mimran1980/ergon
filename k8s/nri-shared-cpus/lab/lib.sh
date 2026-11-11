# SPDX-License-Identifier: Apache-2.0
# shellcheck shell=bash
#
# The quiet node settings of the latency lab. An external AWS lab sources this
# file, so its path and the names lab_quiet_settings, lab_cpus and
# lab_cpumask_hex are a contract: keep them stable. Pure functions: no cloud
# call, no ssh, no kubectl. test/test-lib.sh checks each.

# lab_quiet_settings <housekeeping cpulist> [<exclusive cpulist>]: the node
# settings that keep kernel work off the isolated CPUs (the recorded Azure runs
# applied them as LAB_QUIET=1), one "path|value|why" line each (path under
# /proc/sys or /sys). Each names a per-CPU kernel worker or timer of
# Documentation/admin-guide/kernel-per-CPU-kthreads.rst, or one the traced
# rep caught on the spin CPU (THP and the fair server, run 20261024233129).
lab_quiet_settings() {
    local c
    if [[ -n ${2:-} ]]; then
        for c in $(lab_cpus "$2"); do
            echo "/sys/kernel/debug/sched/fair_server/cpu$c/runtime|0|the fair server's 1 s replenish timer: a nohz kick and a reschedule IPI each second, and the tick kept on (nothing real-time runs here to starve it)"
        done
    fi
    cat <<EOF
/proc/sys/vm/stat_interval|120|vmstat_update work on a CPU whose counters moved (1 s by default)
/proc/sys/kernel/watchdog_cpumask|$1|the soft-lockup watchdog's per-CPU hrtimer and its stopper-thread wakeup (nohz_full already narrows it; set explicitly)
/proc/sys/kernel/nmi_watchdog|0|the hard-lockup detector's per-CPU perf event
/proc/sys/kernel/numa_balancing|0|NUMA balancing's page scanning of the hot task's memory
/proc/sys/vm/compaction_proactiveness|0|proactive compaction migrating the hot task's pages (a fault on the next touch)
/proc/sys/kernel/timer_migration|1|unpinned timers armed on an isolated CPU move to a housekeeping one
/sys/bus/workqueue/devices/writeback/cpumask|$(lab_cpumask_hex "$1")|writeback work on the housekeeping CPUs
/sys/kernel/mm/transparent_hugepage/enabled|never|khugepaged's collapses call lru_add_drain_all, which runs a kworker on every CPU with queued pages (hugetlbfs pages are unaffected)
EOF
}

# lab_cpus <cpulist>: each CPU of it, one a line.
lab_cpus() {
    local part parts c
    IFS=, read -ra parts <<<"$1"
    for part in "${parts[@]}"; do
        for ((c = ${part%-*}; c <= ${part#*-}; c++)); do echo "$c"; done
    done
}

# lab_cpumask_hex <cpulist>: as a hex mask (CPUs below 64).
lab_cpumask_hex() {
    local m=0 part a b c parts
    IFS=, read -ra parts <<<"$1"
    for part in "${parts[@]}"; do
        a=${part%-*} b=${part#*-}
        for ((c = a; c <= b; c++)); do m=$((m | (1 << c))); done
    done
    printf '%x\n' "$m"
}
