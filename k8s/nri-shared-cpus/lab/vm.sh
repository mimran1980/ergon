#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The lab VM's steps; azure-vm.sh runs each over SSH, in this order:
#
#   vm.sh bootstrap   docker, kind v0.33.0, kubectl, helm; the isolation boot
#                     line (lib.sh lab_boot_args) from the next boot
#   (reboot)
#   vm.sh tune        checks the boot line took; every movable IRQ, every
#                     VMBus channel and unbound workqueues on the IRQ CPU;
#                     with QUIET=1 the node settings of lib.sh
#                     lab_quiet_settings; fails unless every NIC interrupt is
#                     there
#   vm.sh cluster     builds ergon-shared-cpus:dev and ergon-gapprobe:lab from
#                     the uploaded checkout, creates the kind cluster
#                     (lab_kind_config), loads both images, applies the
#                     admission policy and namespace trading
#   vm.sh e2e         installs the chart from the checkout; a hot probe pod is
#                     admitted and starts, its spin thread runs on its
#                     isolated CPU alone and every helper on the shared CPU
#                     alone (ps -L and Cpus_allowed_list, three snapshots);
#                     an exec-probe hot pod is denied
#   vm.sh scale       the chart reinstalled with SCALE_SHARED_CPUS (two
#                     shared CPUs); accounting (allocatable millicpus, a pod
#                     over it Pending, one at it scheduled), then cases
#                     a-burst-100m, b-burst-1001m, c-idle, d-contention (hot
#                     pods with busy or idle helper threads: per-CPU busy
#                     time, each container's cgroup usage, weight, quota and
#                     throttling, its shared CPUs, where each thread ran,
#                     the spin's worst gap); results/scale.md
#   vm.sh bench       REPS reps of each arm in ARMS (plugin-on: chart
#                     installed, hot pod; plugin-off: chart uninstalled,
#                     plain static CPU manager pod), alternating the order;
#                     with TRACE=1 one more plugin-on rep (label trace-1)
#                     under kernel ftrace filtered to its spin CPU:
#                     results/trace-cpu<N>.md. Each run: interrupts
#                     snapshot, pod, the sampler on the IRQ CPU for the
#                     spin's CPU, a placement snapshot mid-run, logs,
#                     snapshot again
#   vm.sh report      results/report.md: host facts, then gapprobe report
#
# Settings come from ~/lab/lab.env (azure-vm.sh writes it): IRQ_CPU,
# SHARED_CPUS, SCALE_SHARED_CPUS, EXCLUSIVE_CPUS, RESERVED_CPUS, REPS,
# SECONDS_PER_RUN, DELAY_SECONDS, UDP_RATE, STRESS, ARMS, HELPERS, TRACE,
# QUIET, SCALE_SECONDS, SCALE_THREADS. Results go to
# /var/lib/ergon-lab/results. LAB_SYS_ROOT, LAB_PROC_ROOT, LAB_CGROUP_ROOT,
# LAB_TRACEFS and LAB_GAPPROBE (default /sys, /proc, /sys/fs/cgroup,
# /sys/kernel/tracing, ~/bin/gapprobe) let test/test-lab.sh run the checks
# on fake trees; VM_LIB_ONLY=1 sources the functions only.
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
. "$here/lib.sh"
LAB_SYS_ROOT=${LAB_SYS_ROOT:-/sys}
LAB_PROC_ROOT=${LAB_PROC_ROOT:-/proc}
LAB_CGROUP_ROOT=${LAB_CGROUP_ROOT:-/sys/fs/cgroup}
LAB_TRACEFS=${LAB_TRACEFS:-/sys/kernel/tracing}
LAB_GAPPROBE=${LAB_GAPPROBE:-$HOME/bin/gapprobe}
nri=$(cd "$here/.." && pwd)
admission=$(cd "$nri/.." && pwd)/nri-latency-pods/admission
results=/var/lib/ergon-lab/results
KIND_VERSION=v0.33.0
KUBECTL_VERSION=v1.35.1
HELM_VERSION=v3.17.4

say() { echo "[$(date -u +%T)] $*"; }
fail() { say "FAIL: $*"; exit 1; }

# nic_check <irq CPU>: one line per NIC interrupt (MSI or legacy IRQ: its
# effective CPUs) and per VMBus channel (its cpu) of every NIC under
# class/net with a device directory, ok when on the IRQ CPU alone; returns
# 1 when any is elsewhere or unreadable, or no NIC interrupt is found.
nic_check() {
    local cpu=$1 dev name irq eff ch c bad=0 seen=0
    for dev in "$LAB_SYS_ROOT"/class/net/*/device; do
        [[ -d $dev ]] || continue
        name=${dev%/device}
        name=${name##*/}
        for irq in "$dev"/msi_irqs/*; do
            [[ -e $irq ]] || continue
            irq=${irq##*/}
            seen=$((seen + 1))
            eff=$(cat "$LAB_PROC_ROOT/irq/$irq/effective_affinity_list" 2>/dev/null) ||
                eff=$(cat "$LAB_PROC_ROOT/irq/$irq/smp_affinity_list" 2>/dev/null) || eff=
            if [[ $eff == "$cpu" ]]; then echo "ok:   $name irq $irq on CPU $eff"; else
                echo "FAIL: $name irq $irq on CPU ${eff:-?}, want $cpu"
                bad=1
            fi
        done
        for ch in "$dev"/channels/*; do
            [[ -d $ch ]] || continue
            seen=$((seen + 1))
            c=$(cat "$ch/cpu" 2>/dev/null) || c=
            if [[ $c == "$cpu" ]]; then echo "ok:   $name vmbus channel ${ch##*/} on CPU $c"; else
                echo "FAIL: $name vmbus channel ${ch##*/} on CPU ${c:-?}, want $cpu"
                bad=1
            fi
        done
    done
    ((seen)) || { echo "FAIL: no NIC interrupt or VMBus channel under $LAB_SYS_ROOT/class/net"; return 1; }
    return $bad
}

# pin_interrupts <irq CPU> <housekeeping CPUs>: every IRQ that lets it and
# new IRQs' default on the IRQ CPU, every VMBus channel there (the host
# signals a channel on its cpu), unbound workqueues on the housekeeping CPUs.
# A refusal (a managed or per-CPU IRQ) is the kernel's to keep.
pin_interrupts() {
    local cpu=$1 hk=$2 f
    for f in "$LAB_PROC_ROOT"/irq/[0-9]*/smp_affinity_list; do
        echo "$cpu" | sudo tee "$f" >/dev/null 2>&1 || true
    done
    printf '%x\n' $((1 << cpu)) | sudo tee "$LAB_PROC_ROOT/irq/default_smp_affinity" >/dev/null 2>&1 || true
    for f in "$LAB_SYS_ROOT"/bus/vmbus/devices/*/channels/*/cpu; do
        [[ -e $f ]] || continue
        echo "$cpu" | sudo tee "$f" >/dev/null 2>&1 || echo "note: $f refused CPU $cpu (stays $(cat "$f" 2>/dev/null))"
    done
    if [[ -e $LAB_SYS_ROOT/devices/virtual/workqueue/cpumask ]]; then
        cpumask "$hk" | sudo tee "$LAB_SYS_ROOT/devices/virtual/workqueue/cpumask" >/dev/null 2>&1 ||
            echo "note: unbound workqueue cpumask refused $hk"
    fi
}

# quiet_node <housekeeping CPUs> [<exclusive CPUs>]: every setting of lib.sh
# lab_quiet_settings that this kernel has, written and printed "path: before
# -> after (why)"; one it lacks is printed "skip: path (not on this kernel)".
# Looked at as root: debugfs is 700 root.
quiet_node() {
    local path value why f before
    while IFS='|' read -r path value why; do
        case $path in
        /proc/*) f=$LAB_PROC_ROOT/${path#/proc/} ;;
        /sys/*) f=$LAB_SYS_ROOT/${path#/sys/} ;;
        *) continue ;;
        esac
        if ! sudo test -e "$f"; then
            echo "skip: $path (not on this kernel)"
            continue
        fi
        before=$(sudo cat "$f" 2>/dev/null)
        if echo "$value" | sudo tee "$f" >/dev/null 2>&1; then
            echo "${path#/proc/sys/}: $before -> $(sudo cat "$f" 2>/dev/null) ($why)"
        else
            echo "refused: ${path#/proc/sys/} stays $before, wanted $value ($why)"
        fi
    done < <(lab_quiet_settings "$1" "${2:-}")
}

# cpumask <cpulist>: as a hex mask (CPUs below 64).
cpumask() { lab_cpumask_hex "$1"; }

# tset <tracefs file> <value>: writes it; fails when the kernel refuses.
tset() { printf '%s\n' "$2" | sudo tee "$LAB_TRACEFS/$1" >/dev/null 2>&1; }
# tdir <tracefs dir>: whether it exists (tracefs is 700 root: ask as root).
tdir() { sudo test -d "$LAB_TRACEFS/$1"; }

# trace_start <cpu> <prefix>: tracefs ready for the traced rep: the buffer
# cleared, the mono clock, 16 MiB a CPU, every event of lab_trace_events
# this kernel has, filtered (and with a stacktrace trigger where listed);
# writes <prefix>.events (enabled, with its filter), <prefix>.missing (not
# on this kernel, or a filter or trigger the kernel refused) and
# <prefix>.offset (CLOCK_REALTIME - CLOCK_MONOTONIC, ns, from gapprobe
# clocks) and <prefix>.cpu, then turns tracing on. Every event of interest is filtered in the
# kernel, so the buffers hold only what concerns that CPU.
trace_start() {
    local cpu=$1 p=$2 ev filter stack rel rt mono
    tdir events || sudo mount -t tracefs nodev "$LAB_TRACEFS" >/dev/null 2>&1
    tdir events || { echo "no tracefs at $LAB_TRACEFS"; return 1; }
    tset tracing_on 0 || { echo "tracefs: cannot write tracing_on"; return 1; }
    tset current_tracer nop
    tset set_event ""
    tset trace ""
    tset trace_clock mono || { echo "tracefs: no mono trace clock"; return 1; }
    tset buffer_size_kb 16384 || echo "tracefs: buffer_size_kb 16384 refused"
    : >"$p.events"
    : >"$p.missing"
    echo "$cpu" >"$p.cpu"
    while IFS=';' read -r ev filter stack; do
        rel=events/${ev%%:*}/${ev#*:}
        if ! tdir "$rel"; then
            echo "$ev" >>"$p.missing"
            continue
        fi
        tset "$rel/filter" "$filter" || { echo "$ev (filter refused: $filter)" >>"$p.missing"; continue; }
        if [[ -n $stack ]] && ! tset "$rel/trigger" "stacktrace if $filter"; then
            echo "$ev (stack trigger refused)" >>"$p.missing"
            stack=
        fi
        tset "$rel/enable" 1 || { echo "$ev (enable refused)" >>"$p.missing"; continue; }
        [[ -n $stack ]] && stack=", with the sender's stack"
        echo "$ev if $filter$stack" >>"$p.events"
    done < <(lab_trace_events "$cpu")
    read -r rt mono < <("$LAB_GAPPROBE" clocks) && [[ -n $rt && -n $mono ]] ||
        { echo "gapprobe clocks failed"; return 1; }
    echo $((rt - mono)) >"$p.offset"
    tset tracing_on 1 || { echo "tracefs: cannot turn tracing on"; return 1; }
}

# trace_stop <prefix>: tracing off; the clock offset again to
# <prefix>.offset.end; the buffer's text to <prefix>.trace.txt,
# each CPU's overrun count to <prefix>.stats; every event of the list
# disabled, its filter cleared and its trigger removed; buffer and clock
# back to the kernel's defaults.
trace_stop() {
    local p=$1 ev filter stack rel c rt mono
    tset tracing_on 0
    # the offset again: NTP may have slewed CLOCK_REALTIME meanwhile
    read -r rt mono < <("$LAB_GAPPROBE" clocks) && [[ -n $rt && -n $mono ]] && echo $((rt - mono)) >"$p.offset.end"
    sudo cat "$LAB_TRACEFS/trace" >"$p.trace.txt" || { echo "cannot read $LAB_TRACEFS/trace"; return 1; }
    : >"$p.stats"
    for c in $(sudo ls "$LAB_TRACEFS/per_cpu" 2>/dev/null); do
        sudo test -e "$LAB_TRACEFS/per_cpu/$c/stats" || continue
        echo "$c $(sudo grep -m1 '^overrun:' "$LAB_TRACEFS/per_cpu/$c/stats")" >>"$p.stats"
    done
    while IFS=';' read -r ev filter stack; do
        rel=events/${ev%%:*}/${ev#*:}
        tdir "$rel" || continue
        tset "$rel/enable" 0
        [[ -n $stack ]] && tset "$rel/trigger" "!stacktrace if $filter"
        tset "$rel/filter" 0
    done < <(lab_trace_events "$(cat "$p.cpu" 2>/dev/null)")
    tset trace ""
    tset buffer_size_kb 1408
    tset trace_clock local
}

# armed_before_spin <armed realtime ns> <summary.tsv>: whether tracing was
# on (trace_start done) before the run's spin started (its start_ns).
armed_before_spin() {
    local start
    start=$(awk -F'\t' 'NR == 1 { for (i = 1; i <= NF; i++) if ($i == "start_ns") c = i; next } NR == 2 && c { print $c }' "$2" 2>/dev/null)
    [[ $start =~ ^[0-9]+$ ]] && (($1 < start))
}

# cgroup_dir <pid>: the cgroup v2 directory of a process (its container's).
cgroup_dir() {
    local rel
    rel=$(sed -n 's/^0:://p' "$LAB_PROC_ROOT/$1/cgroup" 2>/dev/null) && [[ -n $rel ]] || return 1
    echo "$LAB_CGROUP_ROOT$rel"
}

# probe_pid <label>: the pid of the gapprobe spin process with that label.
probe_pid() {
    local p
    for p in $(pgrep -x gapprobe); do
        tr '\0' ' ' <"$LAB_PROC_ROOT/$p/cmdline" 2>/dev/null | grep -q -- " spin --label $1 " && { echo "$p"; return 0; }
    done
    return 1
}

# pod_threads <gapprobe pid>: "tid psr allowed comm" for every thread of the
# container's processes: the shell that started the probe and everything
# under it (the probe, its helper process, stress-ng or burn and their
# threads).
pod_threads() {
    local root p t psr comm allowed
    root=$(ps -o ppid= -p "$1" | tr -d ' ')
    for p in $(ps -e -o pid=,ppid= | awk -v r="$root" '
        { parent[$1] = $2 } END {
            keep[r] = 1
            do { more = 0; for (x in parent) if (!(x in keep) && (parent[x] in keep)) { keep[x] = 1; more = 1 } } while (more)
            for (x in keep) print x
        }'); do
        while read -r t psr comm; do
            allowed=$(awk '/^Cpus_allowed_list:/ { print $2 }' "$LAB_PROC_ROOT/$p/task/$t/status" 2>/dev/null)
            echo "$t $psr ${allowed:-?} $comm"
        done < <(ps -L -o tid=,psr=,comm= -p "$p" 2>/dev/null)
    done
}

# hot_tid <gapprobe pid>: the spin thread's tid (comm gp-hot).
hot_tid() { pod_threads "$1" | awk '$4 == "gp-hot" { print $1; exit }'; }

kube() { kubectl --context kind-ergon-lab "$@"; }

# The chart creates its namespace (templates/namespace.yaml, Pod Security
# privileged), so helm's --create-namespace collides with it ("namespaces
# ergon-system already exists"): the release record lives in default, the
# objects in ergon-system.
plugin_installed() { helm status ergon-shared-cpus -n default --kube-context kind-ergon-lab >/dev/null 2>&1; }

# plugin_shared: the sharedCpus the installed release has.
plugin_shared() {
    helm get values ergon-shared-cpus -n default --kube-context kind-ergon-lab -o json 2>/dev/null | jq -r '.config.sharedCpus // empty'
}

# plugin_install: the chart with SHARED_CPUS (a release with other shared
# CPUs is uninstalled first); ready once the node's allocatable
# ergon.io/shared-millicpus is 1000 a shared CPU.
plugin_install() {
    if plugin_installed; then
        [[ $(plugin_shared) == "$SHARED_CPUS" ]] && return 0
        say "the chart has sharedCpus $(plugin_shared); reinstalling with $SHARED_CPUS"
        plugin_uninstall || return 1
    fi
    say "installing the chart from the checkout (sharedCpus $SHARED_CPUS, the VM's /sys at /vm-sys)"
    helm install ergon-shared-cpus "$nri/charts/ergon-shared-cpus" --kube-context kind-ergon-lab \
        --namespace default \
        --set image.repository=ergon-shared-cpus --set image.tag=dev --set image.pullPolicy=Never \
        --set-string config.sharedCpus="$SHARED_CPUS" \
        --set 'config.namespaces={trading}' \
        --set hostContract.kubeletConfig.enabled=true \
        --set hostContract.sys.enabled=true --set hostContract.sys.path=/vm-sys \
        --set hostContract.proc.enabled=true || return 1
    local i ready alloc
    for ((i = 0; i < 90; i++)); do
        ready=$(kube -n ergon-system get ds ergon-shared-cpus -o jsonpath='{.status.numberReady}' 2>/dev/null)
        alloc=$(allocatable)
        [[ $ready == 1 && $alloc == "$((1000 * $(cpu_count "$SHARED_CPUS")))" ]] && { say "plugin ready, allocatable $alloc"; return 0; }
        sleep 2
    done
    kube -n ergon-system logs -l app=ergon-shared-cpus --tail=50 || true
    say "plugin not ready (ready=$ready allocatable=$alloc)"
    return 1
}

plugin_uninstall() {
    plugin_installed || return 0
    say "uninstalling the chart"
    helm uninstall ergon-shared-cpus -n default --kube-context kind-ergon-lab --wait --timeout 120s || return 1
    kube -n ergon-system wait --for=delete pod -l app=ergon-shared-cpus --timeout=120s >/dev/null 2>&1 || true
}

# allocatable: the node's allocatable ergon.io/shared-millicpus, plain decimal.
allocatable() {
    qty "$(kube get node ergon-lab-control-plane -o go-template='{{index .status.allocatable "ergon.io/shared-millicpus"}}' 2>/dev/null)"
}

# qty <quantity>: Kubernetes prints 1000 as 1k; plain decimal out.
qty() {
    case ${1:-0} in
    *M) echo $((${1%M} * 1000000)) ;;
    *k) echo $((${1%k} * 1000)) ;;
    *) echo "${1:-0}" ;;
    esac
}

# cpu_count <cpulist>
cpu_count() {
    local n=0 part
    IFS=, read -ra parts <<<"$1"
    for part in "${parts[@]}"; do n=$((n + ${part#*-} - ${part%-*} + 1)); done
    echo "$n"
}

# wait_log <pod> <seconds>: the probe's first line, once it is there.
wait_log() {
    local i line
    for ((i = 0; i < $2; i++)); do
        line=$(kube -n trading logs "$1" 2>/dev/null | grep -m1 '^gapprobe: label=') && { echo "$line"; return 0; }
        sleep 1
    done
    return 1
}

# wait_done <pod> <seconds>: until the pod has Succeeded or Failed; prints its phase.
wait_done() {
    local i phase
    for ((i = 0; i < $2; i++)); do
        phase=$(kube -n trading get pod "$1" -o jsonpath='{.status.phase}' 2>/dev/null)
        [[ $phase == Succeeded || $phase == Failed ]] && break
        sleep 1
    done
    echo "$phase"
}

# field <line> <key>: the value of key=value in a probe line.
field() { tr ' ' '\n' <<<"$1" | sed -n "s/^$2=//p" | head -1; }

# pod_failure <pod>: its phase, the container's waiting reason and message,
# and the scheduler's message when it is not scheduled (the contention case
# needs 4 of the 5 exclusive CPUs' allocatable cpu, next to the control
# plane's requests).
pod_failure() {
    kube -n trading get pod "$1" -o jsonpath='{.status.phase} {.status.containerStatuses[0].state.waiting.reason} {.status.containerStatuses[0].state.waiting.message} {.status.conditions[?(@.type=="PodScheduled")].message}' 2>/dev/null
}

step_e2e() {
    local bad=0 line cpu iso shr pid out i deny why
    plugin_install || fail "the plugin did not come up"
    kube -n trading delete pod e2e-hot --ignore-not-found --wait=true >/dev/null 2>&1
    lab_pod plugin-on e2e-hot 0 120 5 "$UDP_RATE" "$STRESS" "$HELPERS" | kube apply -f - || fail "the hot pod was not admitted"
    say "hot pod e2e-hot admitted"
    if ! line=$(wait_log e2e-hot 120); then
        why=$(pod_failure e2e-hot)
        kube -n trading delete pod e2e-hot --wait=true >/dev/null 2>&1
        fail "the hot pod did not start: $why"
    fi
    say "hot pod started: $line"
    cpu=$(field "$line" cpu) iso=$(field "$line" ERGON_ISOLATED_CPUS) shr=$(field "$line" ERGON_SHARED_CPUS)
    out=$results/e2e.txt
    {
        echo "probe line: $line"
        [[ $shr == "$SHARED_CPUS" ]] && echo "ok:   ERGON_SHARED_CPUS=$shr" || { echo "FAIL: ERGON_SHARED_CPUS=$shr, want $SHARED_CPUS"; bad=1; }
        [[ -n $iso && $cpu == "$(lab_first_cpu "$iso")" ]] && echo "ok:   spin on CPU $cpu, the first of ERGON_ISOLATED_CPUS=$iso" ||
            { echo "FAIL: spin on CPU $cpu, ERGON_ISOLATED_CPUS=$iso"; bad=1; }
        grep -qx "$cpu" < <(seq "${EXCLUSIVE_CPUS%-*}" "${EXCLUSIVE_CPUS#*-}") && echo "ok:   CPU $cpu is isolated ($EXCLUSIVE_CPUS)" ||
            { echo "FAIL: CPU $cpu is not in the isolated $EXCLUSIVE_CPUS"; bad=1; }
    } | tee "$out"
    grep -q '^FAIL' "$out" && bad=1
    sleep 8 # the 5 s delay, then the spin
    pid=$(probe_pid e2e-hot) || fail "no gapprobe process for e2e-hot on the VM"
    for i in 1 2 3; do
        echo "placement snapshot $i:"
        pod_threads "$pid" | lab_placement_check plugin-on "$cpu" "$SHARED_CPUS" || echo "FAIL: snapshot $i"
        sleep 2
    done | tee -a "$out"
    grep -q '^FAIL' "$out" && bad=1
    deny=$(kube apply -f "$nri/examples/try/bad-probe.yaml" 2>&1)
    if grep -q 'livenessProbe must not be exec' <<<"$deny"; then echo "ok:   exec-probe hot pod denied" | tee -a "$out"; else
        echo "FAIL: exec-probe hot pod not denied: $deny" | tee -a "$out"
        kube -n trading delete pod trader-bad-probe --ignore-not-found >/dev/null 2>&1
        bad=1
    fi
    kube -n trading delete pod e2e-hot --wait=true >/dev/null 2>&1
    rm -f "$results"/e2e-hot.*
    ((bad)) && fail "e2e: see $out"
    say "e2e: PASS"
}

# run_arm <arm> <rep>: one run; arm trace is a plugin-on run under ftrace.
run_arm() {
    local arm=$1 rep=$2 label=$1-$2 line cpu sampler phase restarts pid tid delay=$DELAY_SECONDS armed=
    [[ $arm == trace ]] && delay=$(lab_trace_delay "$DELAY_SECONDS")
    case $arm in
    plugin-on | trace) plugin_install || { echo "$label: the plugin did not come up" >>"$results/failures.txt"; return 1; } ;;
    plugin-off) plugin_uninstall || { echo "$label: the chart did not uninstall" >>"$results/failures.txt"; return 1; } ;;
    esac
    if ! nic_check "$IRQ_CPU" >/dev/null; then
        say "$label: a NIC interrupt moved off CPU $IRQ_CPU; pinning again"
        pin_interrupts "$IRQ_CPU" "$RESERVED_CPUS"
        nic_check "$IRQ_CPU" >"$results/$label.nic.txt" || echo "$label: NIC interrupts off CPU $IRQ_CPU (see $label.nic.txt)" >>"$results/failures.txt"
    fi
    cat "$LAB_PROC_ROOT/interrupts" >"$results/$label.interrupts.before"
    lab_pod "$arm" "$label" "$rep" "$SECONDS_PER_RUN" "$delay" "$UDP_RATE" "$STRESS" "$HELPERS" | kube apply -f - ||
        { echo "$label: pod refused" >>"$results/failures.txt"; return 1; }
    line=$(wait_log "$label" 120) || { echo "$label: did not start: $(pod_failure "$label")" >>"$results/failures.txt"; kube -n trading delete pod "$label" --wait=true >/dev/null 2>&1; return 1; }
    cpu=$(field "$line" cpu)
    say "$label: spin on CPU $cpu; sampler on CPU $IRQ_CPU"
    if [[ $arm == trace ]]; then
        # within the (longer) delay: tracing is on before the spin starts,
        # checked against the spin's start once it has run
        if trace_start "$cpu" "$results/$label" >"$results/$label.trace-setup.txt" 2>&1; then
            armed=$(date +%s%N)
        else
            echo "$label: trace_start failed (see $label.trace-setup.txt)" >>"$results/failures.txt"
        fi
    fi
    taskset -c "$IRQ_CPU" "$LAB_GAPPROBE" irqsample --cpu "$cpu" --seconds $((delay + SECONDS_PER_RUN + 15)) \
        --out "$results/$label.irq.tsv" --pin "$IRQ_CPU" &
    sampler=$!
    sleep $((delay + SECONDS_PER_RUN / 2))
    pid=$(probe_pid "$label") && tid=$(hot_tid "$pid")
    {
        echo "$label placement, mid-run:"
        [[ -n ${pid:-} ]] && pod_threads "$pid" | lab_placement_check "$arm" "$cpu" "$SHARED_CPUS"
    } >"$results/$label.ps.txt" 2>&1 || echo "$label: placement check failed (see $label.ps.txt)" >>"$results/failures.txt"
    phase=$(wait_done "$label" $((SECONDS_PER_RUN + 180)))
    restarts=$(kube -n trading get pod "$label" -o jsonpath='{.status.containerStatuses[0].restartCount}' 2>/dev/null)
    if [[ $arm == trace ]]; then
        trace_stop "$results/$label" >>"$results/$label.trace-setup.txt" 2>&1 ||
            echo "$label: trace_stop failed (see $label.trace-setup.txt)" >>"$results/failures.txt"
    fi
    kube -n trading logs "$label" >"$results/$label.log" 2>&1
    kube -n trading delete pod "$label" --wait=true >/dev/null 2>&1
    wait "$sampler" || echo "$label: the sampler failed" >>"$results/failures.txt"
    cat "$LAB_PROC_ROOT/interrupts" >"$results/$label.interrupts.after"
    [[ $phase == Succeeded && ${restarts:-0} == 0 ]] || echo "$label: phase $phase, restarts ${restarts:-?}" >>"$results/failures.txt"
    [[ -s $results/$label.summary.tsv ]] || echo "$label: no summary" >>"$results/failures.txt"
    if [[ -n $armed ]] && ! armed_before_spin "$armed" "$results/$label.summary.tsv"; then
        echo "$label: tracing was armed after the spin started: its setup's interrupts are inside the run" >>"$results/failures.txt"
    fi
    if [[ $arm == trace && -s $results/$label.trace.txt ]]; then
        "$LAB_GAPPROBE" trace-report --trace "$results/$label.trace.txt" --cpu "$cpu" ${tid:+--tid "$tid"} \
            --offset-ns "$(cat "$results/$label.offset")" --offset-end-ns "$(cat "$results/$label.offset.end" 2>/dev/null || cat "$results/$label.offset")" --summary "$results/$label.summary.tsv" \
            --gaps "$results/$label.gaps.csv" --events "$results/$label.events" --missing "$results/$label.missing" \
            --out "$results/trace-cpu$cpu.md" ||
            echo "$label: trace-report failed" >>"$results/failures.txt"
        gzip -f "$results/$label.trace.txt"
    fi
    say "$label: $(grep '^gapprobe: '"$label"' ' "$results/$label.log" || echo 'no result line')"
}

step_bench() {
    local rep arm order
    : >"$results/failures.txt"
    for ((rep = 1; rep <= REPS; rep++)); do
        order=$ARMS
        ((rep % 2)) || order=$(tr ' ' '\n' <<<"$ARMS" | tac | paste -sd' ' -)
        for arm in $order; do run_arm "$arm" "$rep"; done
    done
    [[ ${TRACE:-0} == 1 ]] && run_arm trace 1
    plugin_uninstall
    if [[ -s $results/failures.txt ]]; then
        say "bench: failures:"
        cat "$results/failures.txt"
        return 1
    fi
    say "bench: $((REPS * $(wc -w <<<"$ARMS"))) runs$([[ ${TRACE:-0} == 1 ]] && echo ", and trace-1")"
}

# scale_snapshot <dir> <when>: /proc/stat, and for every pod of the case its
# container cgroup's cpu.stat, its pod cgroup's (a pod-level quota
# throttles there, not in the container's) and its spin thread's schedstat.
scale_snapshot() {
    local d=$1 w=$2 name
    cat "$LAB_PROC_ROOT/stat" >"$d/proc.stat.$w"
    for name in "${!scale_cg[@]}"; do
        cat "${scale_cg[$name]}/cpu.stat" >"$d/$name.cpu.stat.$w" 2>/dev/null
        cat "$(dirname "${scale_cg[$name]}")/cpu.stat" >"$d/$name.pod.cpu.stat.$w" 2>/dev/null
        cat "$LAB_PROC_ROOT/${scale_pid[$name]}/task/${scale_tid[$name]}/schedstat" >"$d/$name.hot.schedstat.$w" 2>/dev/null
    done
}

# scale_case <case> <title> <expect> <name:cpu:millicpus:threads:duty>...:
# the pods started one after the other (each once the one before it runs:
# the plugin gives each the least-loaded shared CPUs at its start, so the
# order fixes who shares with whom), each spinning lab_scale_spin seconds
# (longer with more pods); once every spin runs, a SCALE_SECONDS
# window with snapshots around it and every thread's CPU each second; then
# the pods' end. Files: gapprobe/src/scale.rs's layout, under
# results/scale/<case>; each pod's .meta also has the ERGON_SHARED_CPUS and
# ERGON_ISOLATED_CPUS its probe saw.
scale_case() {
    local case=$1 title=$2 expect=$3 spec name cpu milli threads duty line pid t0 t1 i bad=0 phase f
    shift 3
    local d=$results/scale/$case spin started=0
    spin=$(lab_scale_spin "$SCALE_SECONDS" $#)
    declare -gA scale_cg=() scale_pid=() scale_tid=()
    mkdir -p "$d"
    for spec in "$@"; do
        IFS=: read -r name cpu milli threads duty <<<"$spec"
        printf 'cpu=%s\nmillicpus=%s\nthreads=%s\nduty=%s\n' "$cpu" "$milli" "$threads" "$duty" >"$d/$name.meta"
        lab_scale_pod "$name" "$case" "$cpu" "$milli" "$threads" "$duty" "$spin" 5 | kube apply -f - ||
            { echo "$case: $name refused" >>"$results/scale-failures.txt"; bad=1; continue; }
        line=$(wait_log "$name" 120) || { echo "$case: $name did not start: $(pod_failure "$name")" >>"$results/scale-failures.txt"; bad=1; continue; }
        printf 'shared_cpus=%s\nisolated_cpus=%s\n' "$(field "$line" ERGON_SHARED_CPUS)" "$(field "$line" ERGON_ISOLATED_CPUS)" >>"$d/$name.meta"
        ((started == 0)) && started=$(date +%s)
        say "$case: $line"
    done
    # the last pod's 5 s delay, then its spin; the first pod's spin must
    # outlast the window
    sleep 8
    (($(date +%s) - started + SCALE_SECONDS < spin + 5)) || echo "$case: the pods took too long to start; the first spin ends inside the window" >>"$results/scale-failures.txt"
    for spec in "$@"; do
        name=${spec%%:*}
        pid=$(probe_pid "$name") || { echo "$case: no gapprobe for $name" >>"$results/scale-failures.txt"; bad=1; continue; }
        scale_pid[$name]=$pid
        scale_tid[$name]=$(hot_tid "$pid")
        scale_cg[$name]=$(cgroup_dir "$pid") || { echo "$case: no cgroup for $name" >>"$results/scale-failures.txt"; bad=1; continue; }
        for f in cpu.weight cpu.max; do
            cat "${scale_cg[$name]}/$f" >"$d/$name.$f" 2>/dev/null
            cat "$(dirname "${scale_cg[$name]}")/$f" >"$d/$name.pod.$f" 2>/dev/null
        done
    done
    scale_snapshot "$d" before
    t0=$(date +%s%N)
    for ((i = 1; i <= SCALE_SECONDS; i++)); do
        sleep 1
        for name in "${!scale_pid[@]}"; do pod_threads "${scale_pid[$name]}" | sed "s/^/$i /" >>"$d/$name.psr.tsv"; done
        (($(date +%s%N) - t0 >= SCALE_SECONDS * 1000000000)) && break
    done
    scale_snapshot "$d" after
    t1=$(date +%s%N)
    printf 'case=%s\ntitle=%s\nshared=%s\nexclusive=%s\nwindow_ns=%s\nexpect=%s\n' \
        "$case" "$title" "$SHARED_CPUS" "$EXCLUSIVE_CPUS" $((t1 - t0)) "$expect" >"$d/meta.txt"
    for spec in "$@"; do
        name=${spec%%:*}
        phase=$(wait_done "$name" $((spin + 120)))
        kube -n trading logs "$name" >"$d/$name.log" 2>&1
        kube -n trading delete pod "$name" --wait=true >/dev/null 2>&1
        [[ $phase == Succeeded ]] || { echo "$case: $name phase $phase" >>"$results/scale-failures.txt"; bad=1; }
    done
    return $bad
}

# scale_accounting: allocatable ergon.io/shared-millicpus is 1000 a shared
# CPU; a pod asking 100 more stays Pending (Unschedulable, insufficient
# millicpus); one asking exactly that schedules. ok/FAIL lines.
scale_accounting() {
    local want=$((1000 * $(cpu_count "$SHARED_CPUS"))) got i st msg bad=0
    got=$(allocatable)
    if [[ $got == "$want" ]]; then echo "ok:   allocatable ergon.io/shared-millicpus $got (sharedCpus $SHARED_CPUS, oversubscription 1)"; else
        echo "FAIL: allocatable ergon.io/shared-millicpus $got, want $want"
        bad=1
    fi
    lab_scale_pod acct-over acct 1 $((want + 100)) 0 0 10 1 | kube apply -f - >/dev/null || { echo "FAIL: acct-over ($((want + 100))) refused at admission"; bad=1; }
    for ((i = 0; i < 30; i++)); do
        st=$(kube -n trading get pod acct-over -o jsonpath='{.status.conditions[?(@.type=="PodScheduled")].reason}' 2>/dev/null)
        [[ $st == Unschedulable ]] && break
        sleep 1
    done
    msg=$(kube -n trading get pod acct-over -o jsonpath='{.status.conditions[?(@.type=="PodScheduled")].message}' 2>/dev/null)
    if [[ $st == Unschedulable && $msg == *"ergon.io/shared-millicpus"* ]]; then
        echo "ok:   a pod asking $((want + 100)) stays Pending: $msg"
    else
        echo "FAIL: a pod asking $((want + 100)): PodScheduled reason ${st:-none}: $msg"
        bad=1
    fi
    kube -n trading delete pod acct-over --wait=true >/dev/null 2>&1
    lab_scale_pod acct-all acct 1 "$want" 0 0 10 1 | kube apply -f - >/dev/null || { echo "FAIL: acct-all ($want) refused at admission"; bad=1; }
    for ((i = 0; i < 60; i++)); do
        st=$(kube -n trading get pod acct-all -o jsonpath='{.status.conditions[?(@.type=="PodScheduled")].status}' 2>/dev/null)
        [[ $st == True ]] && break
        sleep 1
    done
    if [[ $st == True ]]; then echo "ok:   a pod asking $want schedules (node $(kube -n trading get pod acct-all -o jsonpath='{.spec.nodeName}'))"; else
        echo "FAIL: a pod asking $want: PodScheduled ${st:-none}: $(kube -n trading get pod acct-all -o jsonpath='{.status.conditions[?(@.type=="PodScheduled")].message}' 2>/dev/null)"
        bad=1
    fi
    kube -n trading delete pod acct-all --wait=true >/dev/null 2>&1
    return $bad
}

step_scale() {
    local h=${SCALE_THREADS:-4} rc=0 n
    # this step's own sampling (ps, awk, kubectl every second) on the IRQ
    # CPU, not on the shared CPUs it measures
    taskset -cp "$IRQ_CPU" $$ >/dev/null 2>&1 || say "scale: could not move this step to CPU $IRQ_CPU; its sampling runs on the housekeeping CPUs"
    SHARED_CPUS=$SCALE_SHARED_CPUS
    n=$(cpu_count "$SHARED_CPUS")
    : >"$results/scale-failures.txt"
    mkdir -p "$results/scale"
    plugin_install || fail "the plugin did not come up with sharedCpus $SHARED_CPUS"
    scale_accounting | tee "$results/scale/accounting.txt"
    grep -q '^FAIL' "$results/scale/accounting.txt" && echo "accounting: see scale/accounting.txt" >>"$results/scale-failures.txt"
    # the plugin gives a container ceil(millicpus/1000) shared CPUs (at
    # least one), the least loaded; no CPU quota
    scale_case a-burst-100m "one pod: cpu 1 exclusive, 100 shared millicpus, $h busy helper threads" \
        "its helpers on 1 of the $n shared CPUs (ceil(100/1000) = 1), using up to all of it: about 1 core, nr_throttled 0, the other shared CPU idle" \
        "scale-100m:1:100:$h:100" || rc=1
    scale_case b-burst-1001m "one pod: cpu 1 exclusive, 1001 shared millicpus, $h busy helper threads" \
        "its helpers on both shared CPUs (ceil(1001/1000) = 2): about $n cores, nr_throttled 0" \
        "scale-1001m:1:1001:$h:100" || rc=1
    scale_case c-idle "one pod: cpu 1 exclusive, 100 shared millicpus, $h helper threads busy 1 % of each 10 ms" \
        "the pod's helpers near 0.04 cores" \
        "scale-idle:1:100:$h:1" || rc=1
    scale_case d-contention "three pods, 100 shared millicpus and $h busy helper threads each, started in turn: A (cpu 1) and B (cpu 1) get the two shared CPUs, C (cpu 2) the least loaded, A's" \
        "A and C share one shared CPU, split by their cgroups' cpu.weight (from the cpu request: C's twice A's), not by their equal millicpus; B alone on the other" \
        "scale-a:1:100:$h:100" "scale-b:1:100:$h:100" "scale-c:2:100:$h:100" || rc=1
    "$LAB_GAPPROBE" scale-report "$results/scale" >"$results/scale.md" || { echo "scale-report failed" >>"$results/scale-failures.txt"; rc=1; }
    grep -q '| FAIL: ' "$results/scale.md" && echo "placement: a thread ran off its CPUs (see the placement column of scale.md)" >>"$results/scale-failures.txt"
    cat "$results/scale.md"
    plugin_uninstall
    if [[ -s $results/scale-failures.txt ]]; then
        say "scale: failures:"
        cat "$results/scale-failures.txt"
        return 1
    fi
    ((rc == 0)) || return 1
    say "scale: done"
}

step_report() {
    {
        echo "# ergon-shared-cpus latency lab"
        echo
        echo "| item | value |"
        echo "|---|---|"
        echo "| VM | ${LAB_SIZE:-?} ($(nproc) vCPUs, $(uname -m)), $(. /etc/os-release && echo "$PRETTY_NAME"), kernel $(uname -r) |"
        echo "| boot line | \`$(cat /proc/cmdline)\` |"
        echo "| isolated / nohz_full | $(cat /sys/devices/system/cpu/isolated 2>/dev/null || echo none) / $(cat /sys/devices/system/cpu/nohz_full 2>/dev/null || echo none) |"
        echo "| IRQ CPU / shared CPU (scale) / reservedSystemCPUs | $IRQ_CPU / $SHARED_CPUS ($SCALE_SHARED_CPUS) / $RESERVED_CPUS |"
        echo "| node settings | $([[ ${QUIET:-0} == 1 ]] && echo "quiet.txt (LAB_QUIET=1)" || echo "as booted (LAB_QUIET=0)") |"
        echo "| runs | $REPS reps x ${ARMS// /, }, ${SECONDS_PER_RUN} s of spin each, after ${DELAY_SECONDS} s of helpers$([[ ${TRACE:-0} == 1 ]] && echo "; trace-1: one more plugin-on run under ftrace, after $(lab_trace_delay "$DELAY_SECONDS") s of helpers") |"
        echo "| helper load | \`stress-ng $STRESS\` and a loopback UDP reader fed $UDP_RATE datagrams/s ($([[ ${HELPERS:-process} == thread ]] && echo "threads of the probe" || echo "a child process of the probe")), from the pod's main thread |"
        echo "| counter | $(grep -h -m1 -o 'counter_hz=[0-9]*' "$results"/*.log 2>/dev/null | head -1) (a percentile below one tick reads 0) |"
        echo "| sampler | /proc/interrupts and /proc/softirqs every 10 ms from CPU $IRQ_CPU: what overlaps a max sat within 10 ms of it |"
        for f in "$results"/trace-cpu*.md; do [[ -e $f ]] && echo "| kernel trace | ${f##*/}: every switch, IPI, timer and work item on the traced rep's spin CPU |"; done
        [[ -e $results/scale.md ]] && echo "| shared-CPU scaling | scale.md |"
        echo
        [[ -s $results/failures.txt ]] && { echo "**Failures:**"; echo; sed 's/^/- /' "$results/failures.txt"; echo; }
        [[ -s $results/scale-failures.txt ]] && { echo "**Scale failures:**"; echo; sed 's/^/- /' "$results/scale-failures.txt"; echo; }
        "$LAB_GAPPROBE" report "$results" --top 6
    } >"$results/report.md"
    cat "$results/report.md"
}

[[ ${VM_LIB_ONLY:-} == 1 ]] && return 0

# shellcheck source=/dev/null
[[ -f ~/lab/lab.env ]] && . ~/lab/lab.env
ARMS=${ARMS:-plugin-on}
HELPERS=${HELPERS:-process}
SCALE_SHARED_CPUS=${SCALE_SHARED_CPUS:-$SHARED_CPUS}
SCALE_SECONDS=${SCALE_SECONDS:-30}
step=${1:-}
case $step in
bootstrap)
    set -e
    sudo apt-get update -q
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq docker.io curl ca-certificates procps jq util-linux
    sudo usermod -aG docker "$USER"
    sudo systemctl disable --now irqbalance 2>/dev/null || true
    arch=arm64
    [[ $(uname -m) == x86_64 ]] && arch=amd64
    tmp=$(mktemp -d)
    curl -fsSLo "$tmp/kind" "https://kind.sigs.k8s.io/dl/$KIND_VERSION/kind-linux-$arch"
    curl -fsSLo "$tmp/kind.sha256" "https://kind.sigs.k8s.io/dl/$KIND_VERSION/kind-linux-$arch.sha256sum"
    (cd "$tmp" && echo "$(cut -d' ' -f1 kind.sha256)  kind" | sha256sum -c -)
    curl -fsSLo "$tmp/kubectl" "https://dl.k8s.io/release/$KUBECTL_VERSION/bin/linux/$arch/kubectl"
    (cd "$tmp" && echo "$(curl -fsSL "https://dl.k8s.io/release/$KUBECTL_VERSION/bin/linux/$arch/kubectl.sha256")  kubectl" | sha256sum -c -)
    curl -fsSLo "$tmp/helm.tgz" "https://get.helm.sh/helm-$HELM_VERSION-linux-$arch.tar.gz"
    (cd "$tmp" && echo "$(curl -fsSL "https://get.helm.sh/helm-$HELM_VERSION-linux-$arch.tar.gz.sha256sum" | cut -d' ' -f1)  helm.tgz" | sha256sum -c -)
    tar -xzf "$tmp/helm.tgz" -C "$tmp"
    sudo install -m 755 "$tmp/kind" "$tmp/kubectl" "$tmp/linux-$arch/helm" /usr/local/bin/
    rm -rf "$tmp"
    sudo mkdir -p "$results"
    # the lab user's own: `vm.sh cluster` copies gapprobe in without sudo
    mkdir -p ~/bin
    [[ -O ~/bin ]] || sudo chown "$(id -u):$(id -g)" ~/bin
    sudo chown "$USER" "$results"
    # kind's node runs systemd and many watchers
    printf 'fs.inotify.max_user_instances = 512\nfs.inotify.max_user_watches = 524288\n' | sudo tee /etc/sysctl.d/90-kind.conf >/dev/null
    args=$(lab_boot_args "$EXCLUSIVE_CPUS" "$IRQ_CPU")
    sudo mkdir -p /etc/default/grub.d
    echo "GRUB_CMDLINE_LINUX_DEFAULT=\"\$GRUB_CMDLINE_LINUX_DEFAULT $args\"" | sudo tee /etc/default/grub.d/99-ergon-lab.cfg >/dev/null
    sudo update-grub
    say "bootstrap: $args from the next boot"
    ;;
tune)
    args=$(lab_boot_args "$EXCLUSIVE_CPUS" "$IRQ_CPU")
    grep -q -- "${args%% *}" /proc/cmdline || fail "the boot line has no ${args%% *}: $(cat /proc/cmdline)"
    [[ $(cat "$LAB_SYS_ROOT/devices/system/cpu/isolated") == "$EXCLUSIVE_CPUS" ]] ||
        fail "isolated is $(cat "$LAB_SYS_ROOT/devices/system/cpu/isolated"), want $EXCLUSIVE_CPUS"
    [[ $(cat "$LAB_SYS_ROOT/devices/system/cpu/nohz_full") == "$EXCLUSIVE_CPUS" ]] ||
        fail "nohz_full is $(cat "$LAB_SYS_ROOT/devices/system/cpu/nohz_full"), want $EXCLUSIVE_CPUS"
    pin_interrupts "$IRQ_CPU" "$RESERVED_CPUS"
    if [[ ${QUIET:-0} == 1 ]]; then quiet_node "$RESERVED_CPUS" "$EXCLUSIVE_CPUS" | tee "$results/quiet.txt"; fi
    {
        echo "cmdline $(cat /proc/cmdline)"
        echo "isolated $(cat "$LAB_SYS_ROOT/devices/system/cpu/isolated") nohz_full $(cat "$LAB_SYS_ROOT/devices/system/cpu/nohz_full")"
        [[ -e $results/quiet.txt ]] && sed 's/^/quiet /' "$results/quiet.txt"
        lscpu
        nic_check "$IRQ_CPU"
    } | tee "$results/host.txt"
    nic_check "$IRQ_CPU" >/dev/null || fail "a NIC interrupt is not on CPU $IRQ_CPU: the plugin would refuse every hot pod"
    say "tune: done"
    ;;
cluster)
    set -e
    docker build -q -t ergon-shared-cpus:dev "$nri"
    docker build -q -t ergon-gapprobe:lab "$here/gapprobe"
    id=$(docker create ergon-gapprobe:lab)
    docker cp "$id:/usr/local/bin/gapprobe" ~/bin/gapprobe
    docker rm "$id" >/dev/null
    lab_kind_config "$RESERVED_CPUS" >~/lab/kind.yaml
    kind create cluster --config ~/lab/kind.yaml --wait 180s
    # containerd in the node can still be restarting just after Ready: wait
    # for its socket, retry the load once, and keep its journal if it fails
    node_containerd_up() {
        local i
        for i in $(seq 1 60); do
            docker exec ergon-lab-control-plane ctr --namespace=k8s.io version >/dev/null 2>&1 && return 0
            sleep 1
        done
        return 1
    }
    loaded=0
    for attempt in 1 2; do
        node_containerd_up || say "containerd in the node not answering (attempt $attempt)"
        if kind load docker-image ergon-shared-cpus:dev ergon-gapprobe:lab --name ergon-lab; then loaded=1; break; fi
    done
    if ((loaded == 0)); then
        say "kind load failed twice; the node's containerd journal:"
        # the first failure, not the restart loop it leaves behind
        docker exec ergon-lab-control-plane journalctl -u containerd --no-pager 2>&1 |
            grep -m3 -B25 -A5 -iE 'level=(fatal|error)|panic|SIGSEGV|signal: killed|Main process exited' |
            head -120 | sed 's/^/  containerd-first| /' || true
        docker exec ergon-lab-control-plane journalctl --no-pager -n 40 -u kubelet 2>&1 | sed 's/^/  kubelet| /' || true
        sudo dmesg 2>&1 | grep -iE 'oom|killed process|segfault|cgroup|containerd' | tail -20 | sed 's/^/  vm-dmesg| /' || true
        docker exec ergon-lab-control-plane journalctl -u containerd --no-pager -n 30 2>&1 | sed 's/^/  containerd| /' || true
        docker exec ergon-lab-control-plane systemctl status containerd --no-pager 2>&1 | sed 's/^/  containerd| /' || true
        exit 1
    fi
    docker exec ergon-lab-control-plane grep -q "reservedSystemCPUs: \"\?$RESERVED_CPUS\"\?" /var/lib/kubelet/config.yaml ||
        fail "the kind kubelet has no reservedSystemCPUs $RESERVED_CPUS"
    kube create namespace trading
    kube apply -f "$admission/priority-class.yaml" -f "$admission/validating-admission-policy.yaml"
    say "cluster: up"
    ;;
e2e) step_e2e ;;
scale) step_scale ;;
bench) step_bench ;;
report) step_report ;;
*)
    echo "usage: vm.sh bootstrap|tune|cluster|e2e|scale|bench|report" >&2
    exit 2
    ;;
esac
