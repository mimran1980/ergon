# SPDX-License-Identifier: Apache-2.0
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

# lab_layout <vcpus>: "irq shared exclusive reserved" for a VM of that many
# vCPUs (Cobalt has no SMT: one CPU a core). CPU 0 takes every NIC IRQ and
# VMBus channel; CPU 1 is the shared housekeeping CPU the plugin hands to
# helpers; CPUs 2 to the last are isolated and the kubelet hands them out
# as exclusive. reservedSystemCPUs is 0-1 (the IRQ CPU and the shared CPU),
# so the plugin's IRQ destination, reserved minus shared, is CPU 0.
lab_layout() {
    local n=$1
    [[ $n =~ ^[0-9]+$ ]] && ((n >= 4)) || return 1
    echo "0 1 2-$((n - 1)) 0-1"
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

# lab_estimate_minutes <reps> <seconds> <delay>: wall time of a run: 35
# minutes of create, boot, reboot, image builds, kind and e2e, then per rep
# both arms, each its spin plus the delay plus 90 s of chart toggle, pod
# start and teardown, then 10 minutes of fetch and delete.
lab_estimate_minutes() {
    local reps=$1 seconds=$2 delay=$3
    echo $((35 + (reps * 2 * (seconds + delay + 90) + 59) / 60 + 10))
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
    image: kindest/node:v1.33.0
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

# lab_pod <arm> <name> <rep> <seconds> <delay> <udp rate> <stress args>:
# the probe pod. Both arms: Guaranteed, cpu "2" (two exclusive CPUs from the
# static CPU manager), the same command: stress-ng and the probe's socket
# reader and sender as helpers, the spin on --cpu auto. plugin-on: the hot
# label, the plugin token, the shared CPU (ergon.io/shared-millicpus 1000),
# a priority class and resizePolicy, as the admission policy requires; the
# spin's CPU is the first of ERGON_ISOLATED_CPUS and the helpers start on
# the shared CPU. plugin-off: none of that; the spin's CPU is the lowest of
# the two and the helpers run on both.
lab_pod() {
    local arm=$1 name=$2 rep=$3 seconds=$4 delay=$5 rate=$6 stress=$7
    local labels="app: gapprobe" annotations="" extra="" shared="" policy="Never" resize=""
    case $arm in
    plugin-on)
        labels="app: gapprobe
    ergon.io/latency: \"hot\""
        annotations="  annotations:
    required-plugins.noderesource.dev: '[\"ergon-shared-cpus\"]'"
        extra="  priorityClassName: ergon-hot"
        shared="
          ergon.io/shared-millicpus: \"1000\""
        policy=OnFailure
        resize="      resizePolicy:
        - resourceName: cpu
          restartPolicy: RestartContainer"
        ;;
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
          gapprobe spin --label $name --arm $arm --rep $rep --seconds $seconds --delay-seconds $delay --udp-rate $rate --cpu auto --out /results
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

# lab_placement_check <arm> <hot CPU> <shared CPUs>: reads "tid psr allowed
# comm" lines (every thread of the probe pod) and prints one ok/FAIL line
# for each. The spin thread (gp-hot) runs on the hot CPU and may run nowhere
# else, in both arms. plugin-on: every other thread may run only on the
# shared CPUs and is on one. plugin-off: no check on the others (they run on
# the pod's exclusive CPUs: the arm's point), only recorded. Fails when any
# line fails, or no gp-hot thread or no helper is listed.
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
            if (arm != "plugin-on") { printf "seen: %s %s on CPU %s, allowed %s\n", comm, tid, psr, allowed; next }
            if (allowed == shared && inlist(psr, shared)) printf "ok:   %s %s on CPU %s, allowed %s\n", comm, tid, psr, allowed
            else { printf "FAIL: %s %s on CPU %s, allowed %s: want the shared CPUs %s\n", comm, tid, psr, allowed, shared; bad++ }
        }
        END {
            if (!hots) { print "FAIL: no gp-hot thread"; bad++ }
            if (!helpers) { print "FAIL: no helper thread"; bad++ }
            exit bad ? 1 : 0
        }'
}
