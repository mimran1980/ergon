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
#                     fails unless every NIC interrupt is there
#   vm.sh cluster     builds ergon-shared-cpus:dev and ergon-gapprobe:lab from
#                     the uploaded checkout, creates the kind cluster
#                     (lab_kind_config), loads both images, applies the
#                     admission policy and namespace trading
#   vm.sh e2e         installs the chart from the checkout; a hot probe pod is
#                     admitted and starts, its spin thread runs on its
#                     isolated CPU alone and every helper on the shared CPU
#                     alone (ps -L and Cpus_allowed_list, three snapshots);
#                     an exec-probe hot pod is denied
#   vm.sh bench       REPS reps of both arms, alternating which goes first:
#                     plugin-on (chart installed, hot pod) and plugin-off
#                     (chart uninstalled, plain static CPU manager pod). Each
#                     run: interrupts snapshot, pod, the sampler on the IRQ
#                     CPU for the spin's CPU, a placement snapshot mid-run,
#                     logs, snapshot again
#   vm.sh report      results/report.md: host facts, then gapprobe report
#
# Settings come from ~/lab/lab.env (azure-vm.sh writes it): IRQ_CPU,
# SHARED_CPUS, EXCLUSIVE_CPUS, RESERVED_CPUS, REPS, SECONDS_PER_RUN,
# DELAY_SECONDS, UDP_RATE, STRESS. Results go to /var/lib/ergon-lab/results.
# LAB_SYS_ROOT and LAB_PROC_ROOT (default /sys, /proc) let test/test-lab.sh
# run the NIC check on a fake tree; VM_LIB_ONLY=1 sources the functions only.
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
. "$here/lib.sh"
LAB_SYS_ROOT=${LAB_SYS_ROOT:-/sys}
LAB_PROC_ROOT=${LAB_PROC_ROOT:-/proc}
nri=$(cd "$here/.." && pwd)
admission=$(cd "$nri/.." && pwd)/nri-latency-pods/admission
results=/var/lib/ergon-lab/results
KIND_VERSION=v0.33.0
KUBECTL_VERSION=v1.33.0
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

# cpumask <cpulist>: as a hex mask (CPUs below 64).
cpumask() {
    local m=0 part a b c
    IFS=, read -ra parts <<<"$1"
    for part in "${parts[@]}"; do
        a=${part%-*} b=${part#*-}
        for ((c = a; c <= b; c++)); do m=$((m | (1 << c))); done
    done
    printf '%x\n' "$m"
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
# under it (the probe, stress-ng and its workers).
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

kube() { kubectl --context kind-ergon-lab "$@"; }

# The chart creates its namespace (templates/namespace.yaml, Pod Security
# privileged), so helm's --create-namespace collides with it ("namespaces
# ergon-system already exists"): the release record lives in default, the
# objects in ergon-system.
plugin_installed() { helm status ergon-shared-cpus -n default --kube-context kind-ergon-lab >/dev/null 2>&1; }

plugin_install() {
    plugin_installed && return 0
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
        alloc=$(qty "$(kube get node ergon-lab-control-plane -o go-template='{{index .status.allocatable "ergon.io/shared-millicpus"}}' 2>/dev/null)")
        [[ $ready == 1 && $alloc == "$((1000 * $(cpu_count "$SHARED_CPUS")))" ]] && { say "plugin ready, allocatable $alloc"; return 0; }
        sleep 2
    done
    kube -n ergon-system logs -l app=ergon-shared-cpus --tail=50 || true
    say "plugin not ready (ready=$ready allocatable=$alloc)"
    return 1
}

plugin_uninstall() {
    plugin_installed || return 0
    say "uninstalling the chart (plugin-off: the plain static CPU manager)"
    helm uninstall ergon-shared-cpus -n default --kube-context kind-ergon-lab --wait --timeout 120s || return 1
    kube -n ergon-system wait --for=delete pod -l app=ergon-shared-cpus --timeout=120s >/dev/null 2>&1 || true
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

# field <line> <key>: the value of key=value in a probe line.
field() { tr ' ' '\n' <<<"$1" | sed -n "s/^$2=//p" | head -1; }

pod_failure() {
    kube -n trading get pod "$1" -o jsonpath='{.status.phase} {.status.containerStatuses[0].state.waiting.reason} {.status.containerStatuses[0].state.waiting.message}' 2>/dev/null
}

step_e2e() {
    local bad=0 line cpu iso shr pid out i deny why
    plugin_install || fail "the plugin did not come up"
    kube -n trading delete pod e2e-hot --ignore-not-found --wait=true >/dev/null 2>&1
    lab_pod plugin-on e2e-hot 0 120 5 "$UDP_RATE" "$STRESS" | kube apply -f - || fail "the hot pod was not admitted"
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

# run_arm <arm> <rep>
run_arm() {
    local arm=$1 rep=$2 label=$1-$2 line cpu sampler i phase restarts
    case $arm in
    plugin-on) plugin_install || { echo "$label: the plugin did not come up" >>"$results/failures.txt"; return 1; } ;;
    plugin-off) plugin_uninstall || { echo "$label: the chart did not uninstall" >>"$results/failures.txt"; return 1; } ;;
    esac
    if ! nic_check "$IRQ_CPU" >/dev/null; then
        say "$label: a NIC interrupt moved off CPU $IRQ_CPU; pinning again"
        pin_interrupts "$IRQ_CPU" "$RESERVED_CPUS"
        nic_check "$IRQ_CPU" >"$results/$label.nic.txt" || echo "$label: NIC interrupts off CPU $IRQ_CPU (see $label.nic.txt)" >>"$results/failures.txt"
    fi
    cat "$LAB_PROC_ROOT/interrupts" >"$results/$label.interrupts.before"
    lab_pod "$arm" "$label" "$rep" "$SECONDS_PER_RUN" "$DELAY_SECONDS" "$UDP_RATE" "$STRESS" | kube apply -f - ||
        { echo "$label: pod refused" >>"$results/failures.txt"; return 1; }
    line=$(wait_log "$label" 120) || { echo "$label: did not start: $(pod_failure "$label")" >>"$results/failures.txt"; kube -n trading delete pod "$label" --wait=true >/dev/null 2>&1; return 1; }
    cpu=$(field "$line" cpu)
    say "$label: spin on CPU $cpu; sampler on CPU $IRQ_CPU"
    taskset -c "$IRQ_CPU" ~/bin/gapprobe irqsample --cpu "$cpu" --seconds $((DELAY_SECONDS + SECONDS_PER_RUN + 15)) \
        --out "$results/$label.irq.tsv" --pin "$IRQ_CPU" &
    sampler=$!
    sleep $((DELAY_SECONDS + SECONDS_PER_RUN / 2))
    {
        echo "$label placement, mid-run:"
        pid=$(probe_pid "$label") && pod_threads "$pid" | lab_placement_check "$arm" "$cpu" "$SHARED_CPUS"
    } >"$results/$label.ps.txt" 2>&1 || echo "$label: placement check failed (see $label.ps.txt)" >>"$results/failures.txt"
    for ((i = 0; i < SECONDS_PER_RUN + 180; i++)); do
        phase=$(kube -n trading get pod "$label" -o jsonpath='{.status.phase}' 2>/dev/null)
        [[ $phase == Succeeded || $phase == Failed ]] && break
        sleep 1
    done
    restarts=$(kube -n trading get pod "$label" -o jsonpath='{.status.containerStatuses[0].restartCount}' 2>/dev/null)
    kube -n trading logs "$label" >"$results/$label.log" 2>&1
    kube -n trading delete pod "$label" --wait=true >/dev/null 2>&1
    wait "$sampler" || echo "$label: the sampler failed" >>"$results/failures.txt"
    cat "$LAB_PROC_ROOT/interrupts" >"$results/$label.interrupts.after"
    [[ $phase == Succeeded && ${restarts:-0} == 0 ]] || echo "$label: phase $phase, restarts ${restarts:-?}" >>"$results/failures.txt"
    [[ -s $results/$label.summary.tsv ]] || echo "$label: no summary" >>"$results/failures.txt"
    say "$label: $(grep '^gapprobe: '"$label"' ' "$results/$label.log" || echo 'no result line')"
}

step_bench() {
    local rep arm order
    : >"$results/failures.txt"
    for ((rep = 1; rep <= REPS; rep++)); do
        if ((rep % 2)); then order="plugin-on plugin-off"; else order="plugin-off plugin-on"; fi
        for arm in $order; do run_arm "$arm" "$rep"; done
    done
    plugin_uninstall
    if [[ -s $results/failures.txt ]]; then
        say "bench: failures:"
        cat "$results/failures.txt"
        return 1
    fi
    say "bench: $((REPS * 2)) runs"
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
        echo "| IRQ CPU / shared CPU / reservedSystemCPUs | $IRQ_CPU / $SHARED_CPUS / $RESERVED_CPUS |"
        echo "| runs | $REPS reps x 2 arms, ${SECONDS_PER_RUN} s of spin each, after ${DELAY_SECONDS} s of helpers |"
        echo "| helper load | \`stress-ng $STRESS\` and a loopback UDP reader fed $UDP_RATE datagrams/s, from the pod's main thread |"
        echo "| counter | $(grep -h -m1 -o 'counter_hz=[0-9]*' "$results"/*.log 2>/dev/null | head -1) (a percentile below one tick reads 0) |"
        echo "| sampler | /proc/interrupts and /proc/softirqs every 10 ms from CPU $IRQ_CPU: what overlaps a max sat within 10 ms of it |"
        echo
        [[ -s $results/failures.txt ]] && { echo "**Failures:**"; echo; sed 's/^/- /' "$results/failures.txt"; echo; }
        ~/bin/gapprobe report "$results" --top 6
    } >"$results/report.md"
    cat "$results/report.md"
}

[[ ${VM_LIB_ONLY:-} == 1 ]] && return 0

# shellcheck source=/dev/null
[[ -f ~/lab/lab.env ]] && . ~/lab/lab.env
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
    sudo mkdir -p "$results" ~/bin
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
    {
        echo "cmdline $(cat /proc/cmdline)"
        echo "isolated $(cat "$LAB_SYS_ROOT/devices/system/cpu/isolated") nohz_full $(cat "$LAB_SYS_ROOT/devices/system/cpu/nohz_full")"
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
    kind load docker-image ergon-shared-cpus:dev ergon-gapprobe:lab --name ergon-lab
    docker exec ergon-lab-control-plane grep -q "reservedSystemCPUs: \"\?$RESERVED_CPUS\"\?" /var/lib/kubelet/config.yaml ||
        fail "the kind kubelet has no reservedSystemCPUs $RESERVED_CPUS"
    kube create namespace trading
    kube apply -f "$admission/priority-class.yaml" -f "$admission/validating-admission-policy.yaml"
    say "cluster: up"
    ;;
e2e) step_e2e ;;
bench) step_bench ;;
report) step_report ;;
*)
    echo "usage: vm.sh bootstrap|tune|cluster|e2e|bench|report" >&2
    exit 2
    ;;
esac
