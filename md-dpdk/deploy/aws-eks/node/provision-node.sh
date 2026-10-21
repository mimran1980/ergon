#!/usr/bin/env bash
# First-boot provisioning of an EKS (Amazon Linux 2023) node for the
# rusteron-media-driver DPDK ENA pair. userdata.sh unpacks the node scripts
# to /opt/rusteron-dpdk/node and runs this from cloud-init. nodeadm-run.service
# (which starts kubelet) is ordered after cloud-final, so this runs before
# kubelet has ever started, and the reboot at the end means kubelet's first
# start already sees the isolated CPUs and the boot-time hugepages:
#
#   1. Exit at once on a node that is already provisioned (its kernel command
#      line has isolcpus=), so the reboot can never loop. Fail on anything
#      but an Intel x86 CPU.
#   2. Turn SMT off on a node that still has it (bare metal: EC2's CpuOptions
#      cover a VM only) and split the CPUs: one CPU is one physical core, the
#      same as on Graviton. The housekeeping CPUs (RUSTERON_HOUSEKEEPING_CPUS,
#      kubelet's reservedSystemCPUs; default cpu0's core, one CPU) take the
#      system, IRQs and kubelet; every other CPU is isolated. Housekeeping
#      must be CPUs that run with SMT off, and at least four cores must
#      remain: the media driver's housekeeping, its sender, its receiver and
#      an app.
#   3. Wait (up to RUSTERON_ENI_WAIT_SECS, default 600) until IMDS lists an
#      ENI at each of RUSTERON_DPDK_DEVICE_NUMBERS (default "1 2"), which the
#      launcher attaches right after the instance is running.
#   4. Build igb_uio from dpdk-kmods (pinned) for the running kernel and
#      install it to /opt/rusteron-dpdk/igb_uio.ko. The network steps (dnf,
#      git) are tried five times, 5, 10, 20 and 40 s apart, before they fail.
#   5. Install bootstrap-dpdk-ena.sh, preflight.sh, irq-affinity.sh, the
#      per-boot unit and the IRQ watcher's unit, and enable both units.
#   6. Set the kernel command line with grubby: nosmt (SMT stays off on every
#      later boot), CPU isolation (isolcpus, nohz_full, rcu_nocbs,
#      irqaffinity), C1 as the deepest idle state on a
#      VM (not on bare metal), IOMMU passthrough,
#      transparent hugepages off from boot and 2 MiB hugepages
#      (RUSTERON_HUGEPAGES, default 2048). The clocksource is left to the
#      kernel unless bare metal and RUSTERON_FORCE_TSC=1 say otherwise; the
#      choice is recorded in $PREFIX/clocksource-policy.
#   7. Clear kubelet's CPU and memory manager state and reboot.
#
# Any failure holds kubelet back for good: a kubelet drop-in asserts that
# /var/lib/rusteron-dpdk/provision-failed is absent, and this writes that file,
# so the node never registers with the DPDK label and taint while it lacks
# isolated CPUs, hugepages or the per-boot unit. Replace such a node.
#
# Reads /etc/rusteron-dpdk/node.env (RUSTERON_NODE_ENV), which userdata.sh
# writes. Testable against fixtures: set RUSTERON_SYSFS_ROOT,
# RUSTERON_PROC_ROOT and RUSTERON_IMDS_ROOT to fixture paths and
# RUSTERON_DRY_RUN=1 to print every command instead of running it (see
# node/test/test-bootstrap.sh).
#
# Accepts no arguments.
set -euo pipefail

RUSTERON_NODE_ENV="${RUSTERON_NODE_ENV:-/etc/rusteron-dpdk/node.env}"
if [[ -f "$RUSTERON_NODE_ENV" ]]; then
    # shellcheck source=/dev/null
    . "$RUSTERON_NODE_ENV"
fi
RUSTERON_SYSFS_ROOT="${RUSTERON_SYSFS_ROOT:-/sys}"
RUSTERON_PROC_ROOT="${RUSTERON_PROC_ROOT:-/proc}"
RUSTERON_IMDS_ROOT="${RUSTERON_IMDS_ROOT:-http://169.254.169.254/latest/meta-data}"
RUSTERON_DRY_RUN="${RUSTERON_DRY_RUN:-0}"
RUSTERON_HOUSEKEEPING_CPUS="${RUSTERON_HOUSEKEEPING_CPUS:-}"
RUSTERON_DPDK_DEVICE_NUMBERS="${RUSTERON_DPDK_DEVICE_NUMBERS:-1 2}"
RUSTERON_HUGEPAGES="${RUSTERON_HUGEPAGES:-2048}"
RUSTERON_ENI_WAIT_SECS="${RUSTERON_ENI_WAIT_SECS:-600}"

# igb_uio comes from dpdk-kmods at a pinned commit (main on 2026-10-14).
DPDK_KMODS_URL=https://dpdk.org/git/dpdk-kmods
DPDK_KMODS_COMMIT=9b182be2ee4bf003c892e1312440e1e5d93eff2c
PREFIX=/opt/rusteron-dpdk
NODE_DIR="$(dirname "$(realpath "${BASH_SOURCE[0]}")")"
UNIT=rusteron-dpdk-ena.service
IRQ_UNIT=rusteron-irq-affinity.service

fail() { echo "provision FAIL: $*" >&2; exit 1; }
ok()   { echo "provision ok: $*" >&2; }

# A command that changes the host; printed instead under dry run.
run() {
    if ((RUSTERON_DRY_RUN)); then
        echo "provision ok: dry-run: would run: $*" >&2
    else
        "$@"
    fi
}

if ((RUSTERON_DRY_RUN)); then
    [[ "$RUSTERON_SYSFS_ROOT" != /sys && "$RUSTERON_PROC_ROOT" != /proc ]] \
        || fail "RUSTERON_DRY_RUN=1 is for fixtures: set RUSTERON_SYSFS_ROOT and RUSTERON_PROC_ROOT to fixture trees"
fi

# On a failed exit: hold kubelet back, so the node never joins.
FAILED_MARK=/var/lib/rusteron-dpdk/provision-failed
KUBELET_HOLD=/etc/systemd/system/kubelet.service.d/90-rusteron-provision-failed.conf
hold_kubelet() {
    local rc=$?
    ((rc != 0)) || return 0
    if ((RUSTERON_DRY_RUN)); then
        ok "dry-run: would hold kubelet back ($KUBELET_HOLD, $FAILED_MARK)"
        return 0
    fi
    mkdir -p "${FAILED_MARK%/*}" "${KUBELET_HOLD%/*}"
    echo "provision-node.sh exited $rc; its output is in /var/log/cloud-init-output.log" > "$FAILED_MARK"
    printf '[Unit]\nAssertPathExists=!%s\n' "$FAILED_MARK" > "$KUBELET_HOLD"
    systemctl daemon-reload
    echo "provision FAIL: kubelet held back ($KUBELET_HOLD): this node will not join the cluster; replace it" >&2
}
trap hold_kubelet EXIT

# retry <command...>: a network step, tried five times with 5, 10, 20 and 40 s
# between attempts; printed instead under dry run.
retry() {
    local attempt=1 delay=5
    if ((RUSTERON_DRY_RUN)); then
        ok "dry-run: would run, retried: $*"
        return 0
    fi
    until "$@"; do
        ((attempt < 5)) || return 1
        echo "provision: '$*' failed (attempt $attempt of 5); retrying in ${delay}s" >&2
        sleep "$delay"
        attempt=$((attempt + 1))
        delay=$((delay * 2))
    done
}

# Expand a CPU list ("0-2,8") to sorted, unique ids, one per line.
cpulist_expand() {
    local part lo hi c
    [[ "$1" =~ ^[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*$ ]] || return 1
    local IFS=,
    for part in $1; do
        if [[ "$part" == *-* ]]; then
            lo="${part%-*}"; hi="${part#*-}"
            for ((c = lo; c <= hi; c++)); do echo "$c"; done
        else
            echo "$part"
        fi
    done | sort -n -u
}

# Compress sorted CPU ids (one per line on stdin) to a list ("1-7,9-15").
cpulist_compress() {
    awk 'NR == 1 { lo = hi = $1; next }
         $1 == hi + 1 { hi = $1; next }
         { out = out (out == "" ? "" : ",") (lo == hi ? lo : lo "-" hi); lo = hi = $1 }
         END { if (NR) print out (out == "" ? "" : ",") (lo == hi ? lo : lo "-" hi) }'
}

siblings_of() { cat "$RUSTERON_SYSFS_ROOT/devices/system/cpu/cpu$1/topology/thread_siblings_list" 2>/dev/null; }

# --- 1. once only -----------------------------------------------------------
cmdline="$(cat "$RUSTERON_PROC_ROOT/cmdline")" || fail "cannot read $RUSTERON_PROC_ROOT/cmdline"
if [[ " $cmdline " == *" isolcpus="* ]]; then
    ok "already provisioned (isolcpus= on the kernel command line)"
    exit 0
fi

# x86 Intel only: production is x86, and intel_idle.max_cstate assumes Intel.
grep -q '^vendor_id[[:space:]]*:[[:space:]]*GenuineIntel' "$RUSTERON_PROC_ROOT/cpuinfo" \
    || fail "not an Intel x86 CPU ($(uname -m)): the node needs a c7i (or other Intel x86) instance"

# --- 2. SMT off, then housekeeping and isolated CPUs -------------------------
# One Kubernetes CPU is one physical core, the same as on Graviton (option 1,
# 2026-10-17). node/launch-template.py puts CpuOptions ThreadsPerCore=1 in
# the launch template where EC2 accepts it; bare metal takes no CPU options,
# so the sysfs write here is the last resort before kubelet's first start and
# the kernel's nosmt (step 6) holds every later boot. The topology keeps
# listing the offline twins after that, so every CPU list below is one CPU per
# physical core: the lowest id in each core's thread sibling set.
online="$(cpulist_expand "$(cat "$RUSTERON_SYSFS_ROOT/devices/system/cpu/online")")" \
    || fail "cannot read the online CPUs"
smt_active="$RUSTERON_SYSFS_ROOT/devices/system/cpu/smt/active"
smt_control="$RUSTERON_SYSFS_ROOT/devices/system/cpu/smt/control"
smt=0
if [[ -r "$smt_active" ]]; then
    smt="$(cat "$smt_active")"
else
    # no smt/active: the topology says (two online CPUs on one core)
    for cpu in $online; do
        sibs="$(siblings_of "$cpu")" || fail "cannot read CPU $cpu's thread siblings"
        for sib in $(cpulist_expand "$sibs"); do
            if [[ "$sib" != "$cpu" ]] && grep -qx "$sib" <<<"$online"; then
                smt=1
                break 2
            fi
        done
    done
fi
if ((smt)); then
    { printf '%s\n' off > "$smt_control"; } 2>/dev/null \
        || fail "cannot turn SMT off ($smt_control)"
    ok "SMT turned off (was active); nosmt goes on the kernel command line"
fi
live=""
for cpu in $online; do
    sibs="$(siblings_of "$cpu")" || fail "cannot read CPU $cpu's thread siblings"
    first="$(cpulist_expand "$sibs" | sed -n '1p')" \
        || fail "cannot parse CPU $cpu's thread siblings '$sibs'"
    if [[ "$cpu" == "$first" ]]; then
        live+="${live:+$'\n'}$cpu"
    fi
done
if [[ -z "$RUSTERON_HOUSEKEEPING_CPUS" ]]; then
    # cpu0's core, the one CPU that runs on it with SMT off.
    RUSTERON_HOUSEKEEPING_CPUS="$(cpulist_expand "$(siblings_of 0)" | sed -n '1p')" \
        || fail "cannot read CPU 0's thread siblings"
fi
hk_list="$(cpulist_expand "$RUSTERON_HOUSEKEEPING_CPUS")" \
    || fail "RUSTERON_HOUSEKEEPING_CPUS='$RUSTERON_HOUSEKEEPING_CPUS': a CPU list such as 0"
for cpu in $hk_list; do
    grep -qx "$cpu" <<<"$online" || fail "housekeeping CPU $cpu is not online"
    if ! grep -qx "$cpu" <<<"$live"; then
        sibs="$(siblings_of "$cpu")" || fail "cannot read CPU $cpu's thread siblings"
        first="$(cpulist_expand "$sibs" | sed -n '1p')"
        fail "housekeeping CPU $cpu will not run with SMT off (its core runs on $first): use $first"
    fi
done
HK="$(cpulist_compress <<<"$hk_list")"
iso_list="$(comm -23 <(sort <<<"$live") <(sort <<<"$hk_list") | sort -n)"
ISO="$(cpulist_compress <<<"$iso_list")"
iso_cores="$(for cpu in $iso_list; do siblings_of "$cpu"; done | sort -u | wc -l | tr -d ' ')" \
    || fail "cannot read the isolated CPUs' thread siblings"
((iso_cores >= 4)) \
    || fail "$iso_cores physical cores outside housekeeping ($HK); the node needs >= 4 (driver housekeeping, sender, receiver, app): use a larger instance"
ok "housekeeping CPUs $HK; isolated CPUs $ISO ($iso_cores physical cores)"

[[ "$RUSTERON_HUGEPAGES" =~ ^[1-9][0-9]*$ ]] || fail "RUSTERON_HUGEPAGES=$RUSTERON_HUGEPAGES: a count of 2 MiB pages"
read -r -a device_numbers <<<"$RUSTERON_DPDK_DEVICE_NUMBERS"
((${#device_numbers[@]} == 2)) || fail "RUSTERON_DPDK_DEVICE_NUMBERS='$RUSTERON_DPDK_DEVICE_NUMBERS': two device numbers"

# --- 3. wait for the two data ENIs ------------------------------------------
imds_get() {
    local path="$1" token
    if [[ "$RUSTERON_IMDS_ROOT" == http* ]]; then
        token="$(curl -fsS -m 2 -X PUT "$RUSTERON_IMDS_ROOT/../api/token" \
            -H "X-aws-ec2-metadata-token-ttl-seconds: 300" 2>/dev/null || true)"
        curl -fsS -m 2 -H "X-aws-ec2-metadata-token: $token" "$RUSTERON_IMDS_ROOT/$path" 2>/dev/null || true
    elif [[ -d "$RUSTERON_IMDS_ROOT/$path" ]]; then
        ls -1 "$RUSTERON_IMDS_ROOT/$path" 2>/dev/null | tr '\n' ' '
    else
        cat "$RUSTERON_IMDS_ROOT/$path" 2>/dev/null || true
    fi
}
attached_device_numbers() {
    local mac
    for mac in $(imds_get network/interfaces/macs/); do
        imds_get "network/interfaces/macs/${mac%/}/device-number"
        echo
    done
}
deadline=$((SECONDS + RUSTERON_ENI_WAIT_SECS))
while :; do
    attached="$(attached_device_numbers)"
    missing=()
    for n in "${device_numbers[@]}"; do
        grep -qx "$n" <<<"$attached" || missing+=("$n")
    done
    ((${#missing[@]})) || break
    ((SECONDS < deadline)) || fail "no ENI at device number(s) ${missing[*]} after ${RUSTERON_ENI_WAIT_SECS}s"
    sleep 5
done
ok "ENIs attached at device numbers ${device_numbers[*]}"

# --- 4. igb_uio for the running kernel --------------------------------------
kver="$(uname -r)"
src="$PREFIX/src/dpdk-kmods"
# Called from retry: set -e does not apply inside, so each step is checked.
install_build_deps() {
    local kpkg
    dnf install -y gcc make git ethtool "kernel-devel-$kver" && return 0
    # AL2023 also ships versioned kernel packages (kernel6.12 and the like),
    # whose headers are <name>-devel. Ask by what the running kernel provides
    # (rpm -qf on /boot/vmlinuz prints "not owned" on stdout when kernel-install
    # generated it); failing that, by the package owning its module directory.
    dnf install -y gcc make git ethtool "kernel-devel-uname-r = $kver" && return 0
    kpkg="$(rpm -qf --qf '%{NAME}\n' "/lib/modules/$kver/vmlinuz" 2>/dev/null)" || return 1
    kpkg=${kpkg%%$'\n'*}
    [[ -n "$kpkg" ]] || return 1
    dnf install -y gcc make git ethtool "$kpkg-devel-$kver"
}
fetch_dpdk_kmods() {
    rm -rf "$src" && git clone -q "$DPDK_KMODS_URL" "$src" && git -C "$src" checkout -q "$DPDK_KMODS_COMMIT"
}
retry install_build_deps || fail "cannot install gcc, make, git, ethtool and the headers for kernel $kver"
retry fetch_dpdk_kmods || fail "cannot fetch $DPDK_KMODS_URL at $DPDK_KMODS_COMMIT"
run make -s -C "$src/linux/igb_uio"
run install -D -m 0644 "$src/linux/igb_uio/igb_uio.ko" "$PREFIX/igb_uio.ko"
if ! ((RUSTERON_DRY_RUN)); then
    vermagic="$(modinfo -F vermagic "$PREFIX/igb_uio.ko")" || fail "cannot read igb_uio.ko's vermagic"
    [[ "${vermagic%% *}" == "$kver" ]] || fail "igb_uio.ko built for '${vermagic%% *}', running $kver"
fi
ok "igb_uio built for $kver at $PREFIX/igb_uio.ko"

# --- 5. per-boot units ------------------------------------------------------
# The IRQ watcher keeps the ENAs' queue IRQs off the isolated CPUs after boot
# (irq-affinity.sh says why); the bootstrap fails without it.
run install -D -m 0755 "$NODE_DIR/bootstrap-dpdk-ena.sh" /usr/local/sbin/bootstrap-dpdk-ena.sh
run install -D -m 0755 "$NODE_DIR/preflight.sh" /usr/local/sbin/preflight.sh
run install -D -m 0755 "$NODE_DIR/irq-affinity.sh" /usr/local/sbin/irq-affinity.sh
run install -D -m 0644 "$NODE_DIR/$UNIT" "/etc/systemd/system/$UNIT"
run install -D -m 0644 "$NODE_DIR/$IRQ_UNIT" "/etc/systemd/system/$IRQ_UNIT"
run systemctl daemon-reload
run systemctl enable "$UNIT" "$IRQ_UNIT"
ok "$UNIT and $IRQ_UNIT enabled from the next boot"

# --- 6. kernel command line -------------------------------------------------
# nosmt keeps SMT off on every later boot (a VM's launch template turned it
# off already with CpuOptions ThreadsPerCore=1; bare metal needs this).
# The isolation flags are the VM lab's (vm.sh isolate).
args="nosmt isolcpus=nohz,domain,managed_irq,$ISO nohz_full=$ISO rcu_nocbs=$ISO irqaffinity=$HK"
# C1 as the deepest idle state on a VM (the CPU reports the hypervisor flag):
# the housekeeping core takes every interrupt, and a deeper idle state wakes
# later. The one committed VM A/B (io/benches/BENCHMARK.md, run
# flags-aws-20261015T062428Z, cstate) was mixed: WebSocket worse, HTTP better
# at p99.99. Not on bare metal: there at least one rep of each capped arm went
# above 4 ms p50 at 288 B 1 M msgs/s (md-dpdk/BENCHMARKS.md, round 4).
is_vm=0
if grep -m1 '^flags' "$RUSTERON_PROC_ROOT/cpuinfo" | grep -qw hypervisor; then
    is_vm=1
    args+=" intel_idle.max_cstate=1 processor.max_cstate=1"
fi
# The clocksource is an explicit hardware profile, not a fix for a source the
# kernel rejected. `tsc=reliable` turns the kernel's TSC verification off, and
# the invariance flags say nothing about whether the TSC agrees across cores.
# So: never on a VM, where the hypervisor's clocksource is the kernel's
# considered choice; on bare metal only with RUSTERON_FORCE_TSC=1, which is
# the operator saying this machine's TSC has been qualified as the same across
# cores against a PTP/PHC reference. Either way the choice is written to
# $PREFIX/clocksource-policy so a later run can see what was decided.
clocksource_args=""
clocksource_policy="kernel: the kernel keeps its own clocksource and its TSC checks"
if ((is_vm)); then
    clocksource_policy="kernel: a VM gets no clocksource override (tsc=reliable would turn the kernel's TSC checks off)"
    ok "clocksource left to the kernel: this is a VM"
elif [[ "${RUSTERON_FORCE_TSC:-0}" == "1" ]]; then
    clocksource_args="clocksource=tsc tsc=reliable"
    clocksource_policy="forced: clocksource=tsc tsc=reliable on bare metal qualified as the same TSC across cores (RUSTERON_FORCE_TSC=1); the kernel's TSC checks are OFF, so keep an independent PTP/PHC health check"
    ok "clocksource forced to tsc (RUSTERON_FORCE_TSC=1): the kernel's TSC checks are off"
else
    clocksource_policy="kernel: bare metal, not qualified for a forced TSC (set RUSTERON_FORCE_TSC=1 only after measuring the same TSC across cores)"
    ok "clocksource left to the kernel: set RUSTERON_FORCE_TSC=1 only on bare metal measured as the same TSC across cores"
fi
# iommu=pt: bare metal has a real IOMMU, which igb_uio does not program;
# passthrough (identity mapping for host-owned devices) lets its DMA through,
# as in the metal lab. A VM has no IOMMU, so it changes nothing there.
# transparent_hugepage=never from boot, before the per-boot unit sets it.
if [[ -n "$clocksource_args" ]]; then
    args+=" $clocksource_args"
fi
args+=" iommu=pt transparent_hugepage=never"
args+=" hugepagesz=2M hugepages=$RUSTERON_HUGEPAGES"
if ((RUSTERON_DRY_RUN)); then
    ok "dry-run: kernel args: $args"
    ok "dry-run: would write $PREFIX/clocksource-policy: $clocksource_policy"
else
    mkdir -p "$PREFIX"
    printf '%s\n' "$clocksource_policy" > "$PREFIX/clocksource-policy" \
        || fail "cannot write $PREFIX/clocksource-policy"
    ok "clocksource policy recorded in $PREFIX/clocksource-policy"
fi
run grubby --update-kernel=ALL --args="$args"
if ! ((RUSTERON_DRY_RUN)); then
    info="$(grubby --info=DEFAULT)" || fail "grubby --info=DEFAULT failed"
    [[ "$info" == *"isolcpus=nohz,domain,managed_irq,$ISO "* ]] \
        || fail "grubby did not set the kernel arguments: $(grep '^args' <<<"$info")"
    ok "kernel args: $args"
fi

# --- 7. reboot before kubelet's first start ---------------------------------
run rm -f /var/lib/kubelet/cpu_manager_state /var/lib/kubelet/memory_manager_state
ok "rebooting into the provisioned kernel command line"
run systemctl reboot
