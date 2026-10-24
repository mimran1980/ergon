# SPDX-License-Identifier: Apache-2.0
# shellcheck shell=bash
#
# Sourced by azure-vm.sh (on this machine) and vm.sh (on the lab VM): the
# CPU layout, the boot line, the plan's estimate, and the kind config and
# pod manifests. Pure functions: no az, no ssh, no kubectl. test/test-lab.sh
# checks each one.

# lab_size_vcpus <size>: the vCPU count of an Azure Arm (Cobalt) size, from
# its name (Standard_D8ps_v6 -> 8). Fails for anything else: x86 sizes have
# no `p` after the vCPU count.
lab_size_vcpus() {
    [[ $1 =~ ^Standard_[A-Z]+([0-9]+)[a-z]*p[a-z]*_v[0-9]+$ ]] || return 1
    echo "${BASH_REMATCH[1]}"
}

# lab_layout <vcpus> [shared 1|2]: "irq shared exclusive reserved" for a VM
# of that many vCPUs (Cobalt has no SMT: one CPU a core), with one shared
# CPU (e2e and bench: the default) or two (scale). Both layouts share one
# boot line and one kind node: CPU 0 takes every NIC IRQ and VMBus channel;
# CPUs 1-2 are housekeeping (not isolated, so the scheduler balances helpers
# across them) and the plugin hands sharedCpus (1, or 1-2) to helpers; CPUs
# 3 to the last are isolated and the kubelet hands them out as exclusive.
# reservedSystemCPUs is 0-2 in both, so only the chart's sharedCpus changes
# between them; with one shared CPU, CPU 2 is housekeeping no pod's helpers
# get. Needs 7 vCPUs or more: IRQ, two shared, four exclusive (the
# contention case's three pods take one, one and two).
lab_layout() {
    local n=$1 k=${2:-1}
    [[ $n =~ ^[0-9]+$ ]] && ((n >= 7)) || return 1
    case $k in
    1) echo "0 1 3-$((n - 1)) 0-2" ;;
    2) echo "0 1-2 3-$((n - 1)) 0-2" ;;
    *) return 1 ;;
    esac
}

# lab_boot_args <exclusive> <irq CPU>: the kernel command line the exclusive
# CPUs need for a hot pod to pass the plugin's check (isolcpus) and stay
# quiet (no tick, no RCU callbacks, no unmanaged IRQ by default).
lab_boot_args() {
    echo "isolcpus=managed_irq,domain,$1 nohz_full=$1 rcu_nocbs=$1 irqaffinity=$2"
}

# lab_price_per_hour <size>: approximate USD an hour, Linux pay-as-you-go
# list price (eastus, late 2026), for the plan only; LAB_PRICE_PER_HOUR
# overrides. Unknown sizes print "?".
lab_price_per_hour() {
    case $1 in
    Standard_D4ps_v6) echo 0.165 ;;
    Standard_D8ps_v6) echo 0.330 ;;
    Standard_D16ps_v6) echo 0.660 ;;
    *) echo "?" ;;
    esac
}

# lab_scale_seconds <window>: the scale step: three one-pod cases (two
# bursts and idle: each its 5 s delay, a spin of the window plus 10 s, 60 s
# of start and teardown), contention (three pods started one after the
# other: 120 s of start and teardown), accounting (two pods, scheduled or
# not: 120 s) and two chart reinstalls (60 s each).
lab_scale_seconds() {
    local w=$1
    echo $((3 * (5 + w + 10 + 60) + (5 + w + 10 + 120) + 120 + 120))
}

# lab_estimate_minutes <reps> <seconds> <delay> [arms 1] [trace 1] [scale 1]
# [scale window 30]: wall time of a run: 35 minutes of create, boot, reboot,
# image builds, kind and e2e; the scale step; per rep each arm, its spin
# plus the delay plus 90 s of chart toggle, pod start and teardown; the
# trace rep, one more spin plus 150 s (tracefs setup, the trace's copy and
# its report); then 10 minutes of fetch and delete.
lab_estimate_minutes() {
    local reps=$1 seconds=$2 delay=$3 arms=${4:-1} trace=${5:-1} scale=${6:-1} window=${7:-30} s
    s=$((reps * arms * (seconds + delay + 90) + trace * (seconds + delay + 150)))
    ((scale)) && s=$((s + $(lab_scale_seconds "$window")))
    echo $((35 + (s + 59) / 60 + 10))
}

# lab_cost <price/h> <minutes>: USD for that long, plus the public IP and
# the 64 GB OS disk (about 0.02 USD an hour together), two decimals; "?"
# for an unknown price.
lab_cost() {
    [[ $1 == "?" ]] && { echo "?"; return; }
    awk -v p="$1" -v m="$2" 'BEGIN { printf "%.2f\n", (p + 0.02) * m / 60 }'
}

# lab_first_cpu <cpulist>: its lowest CPU (2-7 -> 2; 3,5 -> 3).
lab_first_cpu() {
    local l=${1%%,*}
    echo "${l%%-*}"
}

# lab_kind_config <reserved>: the kind cluster: one node, the static CPU
# manager with that reservedSystemCPUs, NRI on, the node label the
# DaemonSet selects, the VM's own /sys at /vm-sys (a bind keeps the VM's
# sysfs, so class/net lists the VM's NICs, not the node's veth) and the
# results directory at /results.
lab_kind_config() {
    cat <<EOF
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
name: ergon-lab
nodes:
  - role: control-plane
    image: kindest/node:v1.35.1@sha256:05d7bcdefbda08b4e038f644c4df690cdac3fba8b06f8289f30e10026720a1ab
    extraMounts:
      - hostPath: /sys
        containerPath: /vm-sys
        readOnly: true
      - hostPath: /var/lib/ergon-lab/results
        containerPath: /results
    kubeadmConfigPatches:
      - |
        kind: InitConfiguration
        nodeRegistration:
          kubeletExtraArgs:
            node-labels: "ergon.io/shared-cpus=true"
      - |
        kind: KubeletConfiguration
        apiVersion: kubelet.config.k8s.io/v1beta1
        cpuManagerPolicy: static
        reservedSystemCPUs: "$1"
        cpuManagerReconcilePeriod: "2s"
containerdConfigPatches:
  - |
    [plugins."io.containerd.nri.v1.nri".default_validator]
      enable = true
  - |
    [plugins."io.containerd.nri.v1.nri"]
      disable = false
EOF
}

# lab_hot_vars <shared millicpus>: sets labels, annotations, extra, shared,
# policy and resize (the caller's locals) to what the admission policy
# requires of a hot pod: the hot label, the plugin token, the priority
# class, that many ergon.io/shared-millicpus, OnFailure and resizePolicy.
lab_hot_vars() {
    labels="app: gapprobe
    ergon.io/latency: \"hot\""
    annotations="  annotations:
    required-plugins.noderesource.dev: '[\"ergon-shared-cpus\"]'"
    extra="  priorityClassName: ergon-hot"
    shared="
          ergon.io/shared-millicpus: \"$1\""
    policy=OnFailure
    resize="      resizePolicy:
        - resourceName: cpu
          restartPolicy: RestartContainer"
}

# lab_pod <arm> <name> <rep> <seconds> <delay> <udp rate> <stress args>
# [helpers process|thread]: the probe pod. Every arm: Guaranteed, cpu "2"
# (two exclusive CPUs from the static CPU manager), the same command:
# stress-ng and the probe's socket reader and sender as helpers (`gapprobe
# spin --helpers`: a child process by default, or threads of the probe),
# the spin on --cpu auto. plugin-on, and trace (the bench's traced rep, the
# same pod): the hot label, the plugin token, the shared CPU
# (ergon.io/shared-millicpus 1000), a priority class and resizePolicy, as
# the admission policy requires; the spin's CPU is the first of
# ERGON_ISOLATED_CPUS and the helpers start on the shared CPU. plugin-off:
# none of that; the spin's CPU is the lowest of the two and the helpers run
# on both.
lab_pod() {
    local arm=$1 name=$2 rep=$3 seconds=$4 delay=$5 rate=$6 stress=$7 helpers=${8:-process}
    local labels="app: gapprobe" annotations="" extra="" shared="" policy="Never" resize=""
    [[ $helpers == process || $helpers == thread ]] || return 1
    case $arm in
    plugin-on | trace) lab_hot_vars 1000 ;;
    plugin-off) ;;
    *) return 1 ;;
    esac
    cat <<EOF
apiVersion: v1
kind: Pod
metadata:
  name: $name
  namespace: trading
  labels:
    $labels
${annotations:+$annotations
}spec:
${extra:+$extra
}  restartPolicy: $policy
  nodeSelector:
    ergon.io/shared-cpus: "true"
  volumes:
    - name: results
      hostPath:
        path: /results
        type: Directory
  containers:
    - name: probe
      image: ergon-gapprobe:lab
      imagePullPolicy: Never
      command: ["sh", "-c"]
      args:
        - |
          stress-ng $stress -q &
          s=\$!
          gapprobe spin --label $name --arm $arm --rep $rep --seconds $seconds --delay-seconds $delay --udp-rate $rate --helpers $helpers --cpu auto --out /results
          rc=\$?
          kill \$s
          wait \$s
          exit \$rc
      volumeMounts:
        - name: results
          mountPath: /results
      resources:
        requests:
          cpu: "2"
          memory: 512Mi$shared
        limits:
          cpu: "2"
          memory: 512Mi$shared
${resize}
EOF
}

# lab_scale_pod <name> <case> <cpu> <millicpus> <threads> <duty> <seconds>
# <delay>: a scale case's hot pod: <cpu> exclusive CPUs and <millicpus>
# ergon.io/shared-millicpus (Guaranteed, labelled and annotated as the
# admission policy requires), running `gapprobe burn` (<threads> helper
# threads, each busy <duty> % of every 10 ms) on the shared CPUs it starts
# on, and the spin on its first exclusive CPU for <seconds> after <delay>;
# results under /results/scale/<case>.
lab_scale_pod() {
    local name=$1 case=$2 cpu=$3 milli=$4 threads=$5 duty=$6 seconds=$7 delay=$8
    local labels annotations extra shared policy resize
    [[ $cpu =~ ^[1-9][0-9]*$ && $milli =~ ^[0-9]+$ && $threads =~ ^[0-9]+$ && $duty =~ ^[0-9]+$ ]] || return 1
    [[ $seconds =~ ^[0-9]+$ && $delay =~ ^[0-9]+$ && $case =~ ^[a-z0-9-]+$ ]] || return 1
    lab_hot_vars "$milli"
    cat <<EOF
apiVersion: v1
kind: Pod
metadata:
  name: $name
  namespace: trading
  labels:
    $labels
$annotations
spec:
$extra
  restartPolicy: $policy
  nodeSelector:
    ergon.io/shared-cpus: "true"
  volumes:
    - name: results
      hostPath:
        path: /results
        type: Directory
  containers:
    - name: probe
      image: ergon-gapprobe:lab
      imagePullPolicy: Never
      command: ["sh", "-c"]
      args:
        - |
          mkdir -p /results/scale/$case
          gapprobe burn --threads $threads --duty $duty --seconds $((delay + seconds + 5)) &
          b=\$!
          gapprobe spin --label $name --arm $case --rep 1 --seconds $seconds --delay-seconds $delay --udp-rate 0 --cpu auto --out /results/scale/$case
          rc=\$?
          kill \$b 2>/dev/null
          wait \$b
          exit \$rc
      volumeMounts:
        - name: results
          mountPath: /results
      resources:
        requests:
          cpu: "$cpu"
          memory: 256Mi$shared
        limits:
          cpu: "$cpu"
          memory: 256Mi$shared
$resize
EOF
}

# lab_placement_check <arm> <hot CPU> <shared CPUs>: reads "tid psr allowed
# comm" lines (every thread of the probe pod) and prints one ok/FAIL line
# for each. The spin thread (gp-hot) runs on the hot CPU and may run nowhere
# else, in every arm. plugin-on and trace: every other thread may run only
# on the shared CPUs and is on one. plugin-off: no check on the others
# (they run on the pod's exclusive CPUs: the arm's point), only recorded.
# Fails when any line fails, or no gp-hot thread or no helper is listed.
lab_placement_check() {
    awk -v arm="$1" -v hot="$2" -v shared="$3" '
        function inlist(cpu, list,   n, i, p, r) {
            n = split(list, p, ",")
            for (i = 1; i <= n; i++) {
                if (split(p[i], r, "-") == 2) {
                    if (cpu + 0 >= r[1] + 0 && cpu + 0 <= r[2] + 0) return 1
                } else if (cpu + 0 == p[i] + 0) return 1
            }
            return 0
        }
        NF >= 4 {
            tid = $1; psr = $2; allowed = $3
            comm = $4; for (i = 5; i <= NF; i++) comm = comm " " $i
            if (comm == "gp-hot") {
                hots++
                if (psr == hot && allowed == hot) printf "ok:   %s %s on CPU %s, allowed %s\n", comm, tid, psr, allowed
                else { printf "FAIL: %s %s on CPU %s, allowed %s: want CPU %s alone\n", comm, tid, psr, allowed, hot; bad++ }
                next
            }
            helpers++
            if (arm == "plugin-off") { printf "seen: %s %s on CPU %s, allowed %s\n", comm, tid, psr, allowed; next }
            if (allowed == shared && inlist(psr, shared)) printf "ok:   %s %s on CPU %s, allowed %s\n", comm, tid, psr, allowed
            else { printf "FAIL: %s %s on CPU %s, allowed %s: want the shared CPUs %s\n", comm, tid, psr, allowed, shared; bad++ }
        }
        END {
            if (!hots) { print "FAIL: no gp-hot thread"; bad++ }
            if (!helpers) { print "FAIL: no helper thread"; bad++ }
            exit bad ? 1 : 0
        }'
}

# lab_trace_events <cpu>: the kernel trace events the bench's traced rep
# enables, one "group:event;filter;stack" line each (a filter holds "|"): what runs on <cpu>
# (switches, interrupts, IPIs received, cross-calls run, timers, ticks, work
# items) and what is sent or queued to it from any CPU (wakeups, IPIs,
# cross-calls, work), the latter with the sender's kernel stack. Filters
# use the event's own fields, or CPU (the CPU it was recorded on). An
# event this kernel lacks is skipped and listed; irq_vectors is x86's.
lab_trace_events() {
    local n=$1
    cat <<EOF
sched:sched_switch;CPU == $n;
sched:sched_waking;target_cpu == $n;stack
sched:sched_wakeup;target_cpu == $n;stack
sched:sched_wakeup_new;target_cpu == $n;stack
sched:sched_migrate_task;orig_cpu == $n || dest_cpu == $n;stack
ipi:ipi_send_cpu;cpu == $n;stack
ipi:ipi_send_cpumask;cpumask & CPUS{$n};stack
ipi:ipi_entry;CPU == $n;
csd:csd_queue_cpu;cpu == $n;stack
csd:csd_function_entry;CPU == $n;
irq:irq_handler_entry;CPU == $n;
irq:softirq_entry;CPU == $n;
timer:hrtimer_expire_entry;CPU == $n;
timer:timer_expire_entry;CPU == $n;
timer:tick_stop;CPU == $n;
workqueue:workqueue_queue_work;cpu == $n;stack
workqueue:workqueue_execute_start;CPU == $n;
irq_vectors:local_timer_entry;CPU == $n;
irq_vectors:reschedule_entry;CPU == $n;
irq_vectors:call_function_entry;CPU == $n;
irq_vectors:call_function_single_entry;CPU == $n;
EOF
}

# lab_quiet_settings <housekeeping cpulist>: the node settings the lab
# applies (LAB_QUIET=1) so that no kernel work is sent to the isolated
# CPUs, one "path|value|why" line each (path under /proc/sys or /sys).
# Each names a per-CPU kernel worker or timer of
# Documentation/admin-guide/kernel-per-CPU-kthreads.rst; the traced rep
# shows which of them were still needed.
lab_quiet_settings() {
    cat <<EOF
/proc/sys/vm/stat_interval|120|vmstat_update work on a CPU whose counters moved (1 s by default)
/proc/sys/kernel/watchdog_cpumask|$1|the soft-lockup watchdog's per-CPU hrtimer and its stopper-thread wakeup (nohz_full already narrows it; set explicitly)
/proc/sys/kernel/nmi_watchdog|0|the hard-lockup detector's per-CPU perf event
/proc/sys/kernel/numa_balancing|0|NUMA balancing's page scanning of the hot task's memory
/proc/sys/vm/compaction_proactiveness|0|proactive compaction migrating the hot task's pages (a fault on the next touch)
/proc/sys/kernel/timer_migration|1|unpinned timers armed on an isolated CPU move to a housekeeping one
/sys/bus/workqueue/devices/writeback/cpumask|$(lab_cpumask_hex "$1")|writeback work on the housekeeping CPUs
EOF
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
