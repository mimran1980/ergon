#!/usr/bin/env bash
# Reboot-safe node bootstrap for the rusteron-media-driver DPDK ENA pair
# (plan sections 9 and 10.2). Run by systemd at every boot
# (rusteron-dpdk-ena.service), before kubelet starts, and idempotent on
# repeat runs:
#
#   1. Load the DPDK driver: igb_uio with wc_activate=1 by default (the only
#      write-combining binding on virtualised EC2), or vfio-pci.
#   2. Preflight the node (driver/IOMMU/hugepages/NUMA/ENA identity) and
#      select the sender and receiver ENAs by IMDS device number; NEVER the
#      primary (default-route) ENA, which stays kernel-owned for EKS, DNS and
#      telemetry and is the kernel-UDP rollback path, and never an ENI the
#      VPC CNI attached.
#   3. Check the kernel command line provision-node.sh set at first boot
#      (the C-state cap on a VM only; iommu=pt wherever there is an IOMMU).
#   4. Take the two ENAs from the OS's network configuration (systemd-networkd
#      leaves their MACs unmanaged; their hotplug units are masked for this
#      boot), then bind each to the DPDK driver (unbind, driver_override,
#      probe), skipping one that is already bound to it.
#   5. Tune the host the way the VM lab does at each boot: irqbalance off,
#      every IRQ on the housekeeping CPUs (irq-affinity.sh pin, which fails on
#      a movable IRQ it cannot place and reports the ones the kernel keeps on
#      an isolated CPU or moves only at their next interrupt), kept there
#      after boot by rusteron-irq-affinity.service, which re-pins them on
#      each link event (an ENA puts its queue IRQs back on their hints
#      whenever it comes up), transparent hugepages off, KSM and automatic
#      NUMA balancing off; vm.min_free_kbytes and busy polling on
#      the kernel sockets only when asked (RUSTERON_MIN_FREE_PCT,
#      RUSTERON_BUSY_POLL_US). Mount Aeron's hugetlbfs, refusing one larger
#      than the free 2 MiB pool.
#   6. Write /var/lib/rusteron-dpdk/ena-pairs.json atomically, recording the
#      sender/receiver PCI, driver, UIO device or IOMMU group, ENI id, MAC,
#      IPv4 CIDR, gateway, NUMA node and health state.
#
# Testable against fake fixtures: set RUSTERON_SYSFS_ROOT, RUSTERON_PROC_ROOT,
# RUSTERON_IMDS_ROOT, RUSTERON_STATE_DIR and RUSTERON_NETWORKD_DIR to fixture
# paths, and RUSTERON_DRY_RUN=1 to skip module loads, binds, mounts,
# systemctl and networkctl (writes to the fixture trees still happen). See
# node/test/test-bootstrap.sh.
#
# Accepts no arguments.
set -euo pipefail

RUSTERON_SYSFS_ROOT="${RUSTERON_SYSFS_ROOT:-/sys}"
RUSTERON_PROC_ROOT="${RUSTERON_PROC_ROOT:-/proc}"
RUSTERON_IMDS_ROOT="${RUSTERON_IMDS_ROOT:-http://169.254.169.254/latest/meta-data}"
RUSTERON_STATE_DIR="${RUSTERON_STATE_DIR:-/var/lib/rusteron-dpdk}"
RUSTERON_DRY_RUN="${RUSTERON_DRY_RUN:-0}"
RUSTERON_DPDK_DRIVER="${RUSTERON_DPDK_DRIVER:-igb_uio}"
RUSTERON_IGB_UIO_KO="${RUSTERON_IGB_UIO_KO:-/opt/rusteron-dpdk/igb_uio.ko}"
# Housekeeping CPUs: kubelet's reservedSystemCPUs and the kernel's
# irqaffinity. Default: cpu0's physical core.
RUSTERON_HOUSEKEEPING_CPUS="${RUSTERON_HOUSEKEEPING_CPUS:-}"
RUSTERON_THP="${RUSTERON_THP:-never}"
# vm.min_free_kbytes as a share of memory (1-5%), only when set: the kernel's
# own reserve otherwise. 2% caused NAK storms and a 12 s stall in the Azure
# lab (BENCHMARKS.md, round 5).
RUSTERON_MIN_FREE_PCT="${RUSTERON_MIN_FREE_PCT:-}"
# net.core.busy_poll and busy_read in microseconds: off (0) unless set. 50 is
# the lab's value for the kernel path (BENCHMARKS.md, recommended
# configuration); it reaches only kernel sockets (the primary ENA, rollback).
RUSTERON_BUSY_POLL_US="${RUSTERON_BUSY_POLL_US:-0}"
# systemd-networkd's administrator directory, for the data ENAs' unmanaged file
RUSTERON_NETWORKD_DIR="${RUSTERON_NETWORKD_DIR:-/etc/systemd/network}"
# Aeron's directory: a hugetlbfs of its own, sized (see below)
RUSTERON_AERON_HUGE_DIR="${RUSTERON_AERON_HUGE_DIR:-/mnt/huge-aeron}"
RUSTERON_AERON_HUGE_SIZE="${RUSTERON_AERON_HUGE_SIZE:-1G}"
INVENTORY="$RUSTERON_STATE_DIR/ena-pairs.json"
# preflight.sh and irq-affinity.sh sit beside this script
NODE_DIR="$(dirname "$(realpath "${BASH_SOURCE[0]}")")"
IRQ_UNIT=rusteron-irq-affinity.service
# "<mac> <bdf>" of every ENA this script has bound: a bound ENA has no netdev,
# so its MAC no longer leads to its BDF.
BOUND="$RUSTERON_STATE_DIR/bound-enas"

fail() { echo "bootstrap FAIL: $*" >&2; exit 1; }
ok()   { echo "bootstrap ok: $*" >&2; }

# Dry run writes the sysfs, proc and networkd trees it is given: only fixtures.
if ((RUSTERON_DRY_RUN)); then
    [[ "$RUSTERON_SYSFS_ROOT" != /sys && "$RUSTERON_PROC_ROOT" != /proc && "$RUSTERON_NETWORKD_DIR" != /etc/systemd/network ]] \
        || fail "RUSTERON_DRY_RUN=1 is for fixtures: set RUSTERON_SYSFS_ROOT, RUSTERON_PROC_ROOT and RUSTERON_NETWORKD_DIR to fixture trees"
fi

# A failed run leaves no inventory, so the device plugin advertises nothing
# on a node that cannot host the pair.
mkdir -p "$RUSTERON_STATE_DIR"
rm -f "$INVENTORY"

# Expand a CPU list ("0-2,8") to sorted, unique ids separated by spaces.
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
    done | sort -n -u | tr '\n' ' '
}

# A CPU list as the kernel's hex cpumask: 32-bit words, highest first,
# separated by commas ("0,8" -> "101").
cpulist_to_mask() {
    local cpu i out="" words=()
    for cpu in $(cpulist_expand "$1"); do
        i=$((cpu / 32))
        while ((${#words[@]} <= i)); do words+=(0); done
        words[i]=$((words[i] | (1 << (cpu % 32))))
    done
    for ((i = ${#words[@]} - 1; i >= 0; i--)); do
        if [[ -z "$out" ]]; then out="$(printf '%x' "${words[i]}")"; else out+=",$(printf '%08x' "${words[i]}")"; fi
    done
    echo "$out"
}

# Write one sysfs or proc value; a failed write fails the bootstrap.
write_value() {
    { printf '%s\n' "$2" > "$1"; } 2>/dev/null || fail "cannot write '$2' to $1"
}

# --- 1. load the DPDK driver ------------------------------------------------
# igb_uio is built for the running kernel by provision-node.sh; wc_activate=1
# maps the ENA's prefetchable BAR write-combined, which LLQ needs.
load_driver() {
    case "$RUSTERON_DPDK_DRIVER" in
    igb_uio)
        if [[ -d "$RUSTERON_SYSFS_ROOT/module/igb_uio" ]]; then
            ok "igb_uio already loaded"
        elif ((RUSTERON_DRY_RUN)); then
            ok "dry-run: would load uio and $RUSTERON_IGB_UIO_KO wc_activate=1"
        else
            [[ -f "$RUSTERON_IGB_UIO_KO" ]] || fail "$RUSTERON_IGB_UIO_KO missing (provision-node.sh builds it)"
            modprobe uio || fail "modprobe uio failed"
            insmod "$RUSTERON_IGB_UIO_KO" wc_activate=1 || fail "insmod $RUSTERON_IGB_UIO_KO wc_activate=1 failed"
            ok "loaded igb_uio with wc_activate=1"
        fi
        ;;
    vfio-pci)
        if [[ -d "$RUSTERON_SYSFS_ROOT/bus/pci/drivers/vfio-pci" ]]; then
            ok "vfio-pci already loaded"
        elif ((RUSTERON_DRY_RUN)); then
            ok "dry-run: would load vfio-pci"
        else
            modprobe vfio-pci || fail "modprobe vfio-pci failed"
            ok "loaded vfio-pci"
        fi
        ;;
    *) fail "RUSTERON_DPDK_DRIVER=$RUSTERON_DPDK_DRIVER: igb_uio or vfio-pci" ;;
    esac
}
load_driver

# --- 2. preflight and ENA selection (exit 1 before any change) -------------
# Sets primary_mac, SENDER_MAC and RECEIVER_MAC.
# shellcheck source=preflight.sh
source "$NODE_DIR/preflight.sh"
# preflight.sh defined its own log helpers; take the bootstrap's back.
fail() { echo "bootstrap FAIL: $*" >&2; exit 1; }
ok()   { echo "bootstrap ok: $*" >&2; }

# --- 3. kernel command line -------------------------------------------------
if [[ -z "$RUSTERON_HOUSEKEEPING_CPUS" ]]; then
    RUSTERON_HOUSEKEEPING_CPUS="$(cat "$RUSTERON_SYSFS_ROOT/devices/system/cpu/cpu0/topology/thread_siblings_list" 2>/dev/null || true)"
fi
hk_expanded="$(cpulist_expand "$RUSTERON_HOUSEKEEPING_CPUS")" \
    || fail "RUSTERON_HOUSEKEEPING_CPUS='$RUSTERON_HOUSEKEEPING_CPUS': a CPU list such as 0,8"

# A VM: the CPU reports the hypervisor flag (bare metal does not).
on_vm() {
    grep -m1 '^flags' "$RUSTERON_PROC_ROOT/cpuinfo" 2>/dev/null | grep -qw hypervisor
}

check_kernel_cmdline() {
    local cmdline w missing=() irqaff="" words=(isolcpus= nohz_full= rcu_nocbs= irqaffinity=)
    cmdline="$(cat "$RUSTERON_PROC_ROOT/cmdline" 2>/dev/null || true)"
    # C1 as the deepest idle state on a VM only: on bare metal it did not help
    # (BENCHMARKS.md, AWS round 4), and provision-node.sh leaves it off there.
    if on_vm; then words+=(intel_idle.max_cstate= processor.max_cstate=); fi
    # An IOMMU (bare metal): igb_uio's DMA passes only in passthrough.
    if compgen -G "$RUSTERON_SYSFS_ROOT/kernel/iommu_groups/*" >/dev/null; then words+=(iommu=pt); fi
    # "name=" takes any value; anything else is a whole word
    for w in "${words[@]}"; do
        if [[ "$w" == *= ]]; then
            [[ " $cmdline " == *" $w"* ]] || missing+=("${w%=}")
        else
            [[ " $cmdline " == *" $w "* ]] || missing+=("$w")
        fi
    done
    ((${#missing[@]} == 0)) \
        || fail "kernel command line lacks ${missing[*]} (provision-node.sh sets them at first boot; replace the node)"
    for w in $cmdline; do
        [[ "$w" != irqaffinity=* ]] || irqaff="${w#irqaffinity=}"
    done
    [[ "$(cpulist_expand "$irqaff" || true)" == "$hk_expanded" ]] \
        || fail "irqaffinity=$irqaff does not match the housekeeping CPUs $RUSTERON_HOUSEKEEPING_CPUS (RUSTERON_HOUSEKEEPING_CPUS)"
    ok "kernel command line isolates every CPU but $RUSTERON_HOUSEKEEPING_CPUS"
}
check_kernel_cmdline

# --- 4. take both ENAs from the OS and bind them to the DPDK driver --------
# The PCI BDF owning a MAC: from its netdev while the kernel's ena driver
# holds it, else (a rerun after binding: the netdev is gone) from the binding
# record, but only when that BDF is bound to the DPDK driver already.
mac_to_bdf() {
    local mac="$1" dev net bdf
    for dev in "$RUSTERON_SYSFS_ROOT"/bus/pci/devices/*/; do
        [[ -d "$dev" ]] || continue
        for net in "$dev"net/*; do
            [[ -d "$net" ]] || continue
            if [[ "$(cat "$net/address" 2>/dev/null || true)" == "$mac" ]]; then
                basename "$dev"
                return 0
            fi
        done
    done
    bdf="$(awk -v m="$mac" '$1 == m { b = $2 } END { print b }' "$BOUND" 2>/dev/null || true)"
    [[ -n "$bdf" ]] || return 1
    [[ "$(current_driver "$bdf")" == "$RUSTERON_DPDK_DRIVER" ]] || return 1
    echo "$bdf"
}

# The driver bound to a BDF, or nothing.
current_driver() {
    local link
    link="$(readlink "$RUSTERON_SYSFS_ROOT/bus/pci/devices/$1/driver" 2>/dev/null || true)"
    [[ -z "$link" ]] || basename "$link"
}

# Record a MAC's BDF before its netdev goes away.
record_bound() {
    local mac="$1" bdf="$2" tmp="$BOUND.tmp.$$"
    { awk -v m="$mac" '$1 != m' "$BOUND" 2>/dev/null || true; echo "$mac $bdf"; } > "$tmp"
    mv "$tmp" "$BOUND"
}

bind_driver() {
    local bdf="$1" cur
    local pci="$RUSTERON_SYSFS_ROOT/bus/pci/devices/$bdf"
    [[ -d "$pci" ]] || fail "no PCI device $bdf in sysfs"
    cur="$(current_driver "$bdf")"
    if [[ "$cur" == "$RUSTERON_DPDK_DRIVER" ]]; then
        ok "$bdf already bound to $RUSTERON_DPDK_DRIVER"
        return 0
    fi
    if ((RUSTERON_DRY_RUN)); then
        ok "dry-run: would bind $bdf to $RUSTERON_DPDK_DRIVER (from ${cur:-no driver})"
        return 0
    fi
    [[ -z "$cur" ]] || write_value "$pci/driver/unbind" "$bdf"
    write_value "$pci/driver_override" "$RUSTERON_DPDK_DRIVER"
    write_value "$RUSTERON_SYSFS_ROOT/bus/pci/drivers_probe" "$bdf"
    [[ "$(current_driver "$bdf")" == "$RUSTERON_DPDK_DRIVER" ]] \
        || fail "$bdf did not bind to $RUSTERON_DPDK_DRIVER (now: $(current_driver "$bdf"))"
    ok "bound $bdf to $RUSTERON_DPDK_DRIVER (from ${cur:-no driver})"
}

# The OS must never configure the data ENAs, now or when one goes back to the
# kernel's ena driver (a rollback, a kernel baseline): the lab's Debian VMs
# re-ran DHCP on each ENA back from DPDK through cloud-init's hotplug,
# replacing addresses and dropping routes (BENCHMARKS.md, AWS round 2). AL2023
# has the same path through udev: on the EKS AMI, 90-eks.rules starts
# udev-net-manager@<iface>.service for every ENA that appears, which writes
# /run/systemd/network/70-eks-<iface>.network (DHCP) and reloads networkd;
# on stock AL2023, amazon-ec2-net-utils' 99-vpc-policy-routes.rules starts
# policy-routes@<iface>.service, and its 80-ec2.network runs DHCP on every
# ENA. So a .network file that sorts before all of those leaves both MACs
# unmanaged, and each ENA's hotplug units are masked until the next boot
# (interface names are per boot; this runs at every boot).
release_from_os() {
    local file="$RUSTERON_NETWORKD_DIR/00-rusteron-dpdk-data.network" want bdf net iface units
    want="$(printf '# bootstrap-dpdk-ena.sh: the DPDK data ENAs, which the OS never configures\n[Match]\nMACAddress=%s %s\n\n[Link]\nUnmanaged=yes' \
        "$SENDER_MAC" "$RECEIVER_MAC")"
    if [[ "$(cat "$file" 2>/dev/null)" != "$want" ]]; then
        mkdir -p "$RUSTERON_NETWORKD_DIR"
        { printf '%s\n' "$want" > "$file.tmp.$$" && mv "$file.tmp.$$" "$file"; } 2>/dev/null || fail "cannot write $file"
        if ((RUSTERON_DRY_RUN)); then
            ok "dry-run: would reload systemd-networkd"
        elif systemctl is-active --quiet systemd-networkd; then
            # Without the file, the unit's restart writes it and reloads again.
            networkctl reload || { rm -f "$file"; fail "networkctl reload failed"; }
        fi
        ok "systemd-networkd leaves $SENDER_MAC and $RECEIVER_MAC unmanaged ($file)"
    fi
    for bdf in "$SENDER_BDF" "$RECEIVER_BDF"; do
        for net in "$RUSTERON_SYSFS_ROOT/bus/pci/devices/$bdf"/net/*; do
            [[ -d "$net" ]] || continue
            iface="$(basename "$net")"
            units=("udev-net-manager@$iface.service" "policy-routes@$iface.service" "refresh-policy-routes@$iface.timer")
            if ((RUSTERON_DRY_RUN)); then
                ok "dry-run: would mask for this boot: ${units[*]}"
                continue
            fi
            systemctl mask --runtime "${units[@]}" >/dev/null 2>&1 || fail "cannot mask ${units[*]}"
            # Stop whichever of them runs; one that never ran has nothing to stop.
            systemctl stop "${units[@]}" >/dev/null 2>&1 || true
            ok "masked for this boot: ${units[*]}"
        done
    done
}

SENDER_BDF="$(mac_to_bdf "$SENDER_MAC")" || fail "no PCI BDF found for sender ENA $SENDER_MAC"
RECEIVER_BDF="$(mac_to_bdf "$RECEIVER_MAC")" || fail "no PCI BDF found for receiver ENA $RECEIVER_MAC"
record_bound "$SENDER_MAC" "$SENDER_BDF"
record_bound "$RECEIVER_MAC" "$RECEIVER_BDF"
release_from_os
bind_driver "$SENDER_BDF"
bind_driver "$RECEIVER_BDF"

# --- 5. per-boot host tuning ------------------------------------------------
# A size as hugetlbfs's size= takes it (bytes, or K, M, G or T), in MiB.
size_mib() {
    local n unit
    [[ "$1" =~ ^([0-9]+)([KkMmGgTt]?)$ ]] || return 1
    n="${BASH_REMATCH[1]}" unit="${BASH_REMATCH[2]}"
    case "$unit" in
    "") echo $((n / 1048576)) ;;
    K | k) echo $((n / 1024)) ;;
    M | m) echo "$n" ;;
    G | g) echo $((n * 1024)) ;;
    T | t) echo $((n * 1048576)) ;;
    esac
}

# Aeron's hugetlbfs must fit in the 2 MiB pages still free: hugetlbfs reports
# the mount's size as its free space, so a mount larger than the pool passes
# Aeron's storage check for log buffers the pool cannot back.
check_aeron_huge_size() {
    local want free_pages
    want="$(size_mib "$RUSTERON_AERON_HUGE_SIZE")" \
        || fail "RUSTERON_AERON_HUGE_SIZE=$RUSTERON_AERON_HUGE_SIZE: a size such as 1G or 512M"
    free_pages="$(cat "$RUSTERON_SYSFS_ROOT/kernel/mm/hugepages/hugepages-2048kB/free_hugepages" 2>/dev/null || true)"
    [[ "$free_pages" =~ ^[0-9]+$ ]] || fail "cannot read the free 2 MiB hugepages"
    ((want <= free_pages * 2)) \
        || fail "RUSTERON_AERON_HUGE_SIZE=$RUSTERON_AERON_HUGE_SIZE is more than the $((free_pages * 2)) MiB of 2 MiB hugepages free (RUSTERON_HUGEPAGES at provisioning sets the pool)"
}

tune_host() {
    local hk="$RUSTERON_HOUSEKEEPING_CPUS" f mem_kb min_free="the kernel's"
    # irqbalance stays off: irq-affinity.sh is the one writer of IRQ affinity,
    # and its comments say why irqbalance does not keep the ENA's queue IRQs
    # off the isolated CPUs.
    if ((RUSTERON_DRY_RUN)); then
        ok "dry-run: would disable irqbalance"
    elif systemctl cat irqbalance.service >/dev/null 2>&1; then
        systemctl disable --now irqbalance.service >/dev/null 2>&1 || fail "cannot disable irqbalance"
        ok "irqbalance disabled"
    else
        ok "irqbalance not installed"
    fi
    # Every IRQ on the housekeeping CPUs: irqaffinity= placed the boot-time
    # IRQs, the default affinity places new ones, and irq-affinity.sh moves the
    # rest (the ENAs' queue IRQs sit on their drivers' per-queue hints). It
    # fails on a movable IRQ whose requested affinity it cannot make the
    # housekeeping CPUs, and reports, without failing, one the kernel refuses
    # to move (a managed or per-CPU IRQ), which is the kernel's to keep, and
    # one the kernel moves only at its next interrupt (an EC2 VM, without
    # interrupt remapping), which an idle queue's IRQ may not take for minutes.
    write_value "$RUSTERON_PROC_ROOT/irq/default_smp_affinity" "$(cpulist_to_mask "$hk")"
    RUSTERON_PROC_ROOT="$RUSTERON_PROC_ROOT" RUSTERON_HOUSEKEEPING_CPUS="$hk" "$NODE_DIR/irq-affinity.sh" pin \
        || fail "IRQs left on the isolated CPUs (above)"
    # ...and after boot: the ENA driver puts its queue IRQs back on their hints
    # whenever it brings an interface up (a device reset, a queue or ring
    # change, a data ENA handed back to the kernel), so the watcher re-pins
    # them on the link event that follows. Starting it is a no-op when it runs
    # already (it is enabled, and ordered before this unit).
    if ((RUSTERON_DRY_RUN)); then
        ok "dry-run: would start $IRQ_UNIT (irq-affinity.sh watch)"
    else
        { systemctl start "$IRQ_UNIT" && systemctl is-active --quiet "$IRQ_UNIT"; } >/dev/null 2>&1 \
            || fail "$IRQ_UNIT is not running, so an ENA's queue IRQs would stay on the isolated CPUs after a reset (provision-node.sh installs it; see systemctl status $IRQ_UNIT)"
        ok "$IRQ_UNIT re-pins every IRQ after each link event"
    fi

    # Transparent hugepages off also stops khugepaged, whose scans drain every
    # CPU's LRU batch and preempt the busy-spinning agents.
    [[ "$RUSTERON_THP" =~ ^(always|madvise|never)$ ]] || fail "RUSTERON_THP=$RUSTERON_THP: always, madvise or never"
    write_value "$RUSTERON_SYSFS_ROOT/kernel/mm/transparent_hugepage/enabled" "$RUSTERON_THP"
    # KSM and automatic NUMA balancing scan and migrate pages under the agents.
    # A kernel built without either has nothing to turn off.
    f="$RUSTERON_SYSFS_ROOT/kernel/mm/ksm/run"
    [[ ! -e "$f" ]] || write_value "$f" 0
    f="$RUSTERON_PROC_ROOT/sys/kernel/numa_balancing"
    [[ ! -e "$f" ]] || write_value "$f" 0
    # A larger free-memory reserve only when asked (see RUSTERON_MIN_FREE_PCT).
    if [[ -n "$RUSTERON_MIN_FREE_PCT" ]]; then
        [[ "$RUSTERON_MIN_FREE_PCT" =~ ^[1-5]$ ]] || fail "RUSTERON_MIN_FREE_PCT=$RUSTERON_MIN_FREE_PCT: 1 to 5, or unset for the kernel's reserve"
        mem_kb="$(awk '/^MemTotal:/ {print $2}' "$RUSTERON_PROC_ROOT/meminfo" 2>/dev/null || true)"
        [[ "$mem_kb" =~ ^[0-9]+$ ]] || fail "no MemTotal in $RUSTERON_PROC_ROOT/meminfo"
        min_free=$((mem_kb * RUSTERON_MIN_FREE_PCT / 100))
        write_value "$RUSTERON_PROC_ROOT/sys/vm/min_free_kbytes" "$min_free"
    fi
    # Busy polling on the kernel sockets (the primary ENA, the rollback path):
    # off unless asked. The primary ENA keeps its driver's interrupt
    # coalescing: turning it off made kernel busy polling worse on AWS
    # (BENCHMARKS.md, round 2).
    [[ "$RUSTERON_BUSY_POLL_US" =~ ^(0|[1-9][0-9]{0,3})$ ]] \
        || fail "RUSTERON_BUSY_POLL_US=$RUSTERON_BUSY_POLL_US: microseconds, 0 (off) to 9999; the lab's kernel path used 50"
    write_value "$RUSTERON_PROC_ROOT/sys/net/core/busy_poll" "$RUSTERON_BUSY_POLL_US"
    write_value "$RUSTERON_PROC_ROOT/sys/net/core/busy_read" "$RUSTERON_BUSY_POLL_US"
    # Room for Aeron's 2 MiB socket buffers (media-driver-config.yaml): the
    # driver refuses a receive window larger than its SO_RCVBUF.
    write_value "$RUSTERON_PROC_ROOT/sys/net/core/rmem_max" 16777216
    write_value "$RUSTERON_PROC_ROOT/sys/net/core/wmem_max" 16777216
    ok "THP $RUSTERON_THP, KSM and NUMA balancing off, min_free_kbytes $min_free, busy_poll/read $RUSTERON_BUSY_POLL_US, rmem/wmem_max 16 MiB"
    # Aeron's directory on a hugetlbfs of its own with a size: without one,
    # hugetlbfs reports no free space and Aeron's storage check refuses every
    # log buffer ("insufficient storage space"). As the VM lab's /mnt/huge;
    # the DPDK EAL keeps /dev/hugepages. Idempotent, every boot; the pool is
    # checked only before a mount (a mounted one holds pages already).
    if ((RUSTERON_DRY_RUN)); then
        check_aeron_huge_size
        ok "dry-run: would mount hugetlbfs pagesize=2M,size=$RUSTERON_AERON_HUGE_SIZE at $RUSTERON_AERON_HUGE_DIR"
    elif ! mountpoint -q "$RUSTERON_AERON_HUGE_DIR"; then
        check_aeron_huge_size
        mkdir -p "$RUSTERON_AERON_HUGE_DIR"
        mount -t hugetlbfs -o "pagesize=2M,size=$RUSTERON_AERON_HUGE_SIZE" nodev "$RUSTERON_AERON_HUGE_DIR" \
            || fail "cannot mount hugetlbfs at $RUSTERON_AERON_HUGE_DIR"
        ok "hugetlbfs pagesize=2M,size=$RUSTERON_AERON_HUGE_SIZE at $RUSTERON_AERON_HUGE_DIR (Aeron's directory)"
    fi
}
tune_host

# --- 6. write the inventory atomically --------------------------------------
pair_record() {
    local role="$1" mac="$2" bdf="$3"
    local pci="$RUSTERON_SYSFS_ROOT/bus/pci/devices/$bdf" uio="" group="" u eni ip subnet prefix gw numa
    case "$RUSTERON_DPDK_DRIVER" in
    igb_uio)
        for u in "$pci"/uio/uio*; do
            [[ -e "$u" ]] && uio="$(basename "$u")" && break
        done
        [[ "$uio" =~ ^uio[0-9]+$ ]] || fail "$role ENA $bdf on igb_uio has no /dev/uio device"
        ;;
    vfio-pci)
        group="$(basename "$(readlink "$pci/iommu_group" 2>/dev/null || true)")"
        [[ -n "$group" ]] || fail "$role ENA $bdf has no IOMMU group"
        ;;
    esac
    eni="$(imds_get "network/interfaces/macs/$mac/interface-id")"
    ip="$(imds_get "network/interfaces/macs/$mac/local-ipv4s" | head -1)"
    subnet="$(imds_get "network/interfaces/macs/$mac/subnet-ipv4-cidr-block")"
    [[ -n "$eni" && -n "$ip" && "$subnet" =~ ^[0-9.]+/([0-9]+)$ ]] \
        || fail "$role ENA $mac missing IMDS identity (eni/ip/subnet)"
    prefix="${BASH_REMATCH[1]}"
    gw="$(cidr_gateway "$subnet")"
    numa="$(cat "$pci/numa_node" 2>/dev/null || echo -1)"
    [[ "$numa" =~ ^-?[0-9]+$ ]] || numa=-1
    printf '      "%s": {"pci":"%s","driver":"%s","uio":"%s","iommu_group":"%s","eni_id":"%s","mac":"%s",' \
        "$role" "$bdf" "$RUSTERON_DPDK_DRIVER" "$uio" "$group" "$eni" "$mac"
    printf '"ipv4":"%s","prefix_len":%s,"subnet_cidr":"%s","gateway":"%s","numa_node":%s,"health":"healthy"}' \
        "$ip" "$prefix" "$subnet" "$gw" "$numa"
}

ip_to_int() { local IFS=. a b c d; read -r a b c d <<<"$1"; echo $((a * 16777216 + b * 65536 + c * 256 + d)); }
int_to_ip() { printf "%d.%d.%d.%d\n" $((($1 >> 24) & 255)) $((($1 >> 16) & 255)) $((($1 >> 8) & 255)) $(($1 & 255)); }

# The VPC subnet gateway (network address + 1, the AWS VPC convention) of an
# ip/prefix CIDR.
cidr_gateway() {
    local cidr="$1" page
    page=$((1 << (32 - ${cidr##*/})))
    int_to_ip $(($(ip_to_int "${cidr%/*}") / page * page + 1))
}

sender_json="$(pair_record sender "$SENDER_MAC" "$SENDER_BDF")"
receiver_json="$(pair_record receiver "$RECEIVER_MAC" "$RECEIVER_BDF")"
tmp="$INVENTORY.tmp.$$"
cat > "$tmp" <<JSON
{
  "generated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "primary_ena_mac": "$primary_mac",
  "pairs": [
    {
      "id": "dpdk-pair-0",
$sender_json,
$receiver_json
    }
  ]
}
JSON
chmod 0644 "$tmp"
mv "$tmp" "$INVENTORY"
ok "wrote $INVENTORY atomically"
