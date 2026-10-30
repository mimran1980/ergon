#!/usr/bin/env bash
# Fixture test for the node scripts: preflight.sh, bootstrap-dpdk-ena.sh,
# provision-node.sh and userdata.sh, plus the container entrypoint's CPU
# layout (plan §9: "Test scripts against fake sysfs/IMDS fixtures before live
# nodes").
#
# Builds a fake sysfs tree (an EBS NVMe controller and four Amazon ENAs:
# primary, sender, receiver and a VPC CNI ENI whose MAC sorts first), a fake
# /proc tree and a fake IMDS tree, then checks that:
#   * preflight passes for igb_uio (with or without an iommu_groups
#     directory, or with every IOMMU group in passthrough), and fails for
#     igb_uio behind a translating IOMMU, for vfio-pci without
#     write-combining, and for vfio-pci without real IOMMU backing;
#   * bootstrap (dry run: no host sysfs or /proc is touched) binds the
#     device-number 1 and 2 ENAs, never the CNI ENI, first marking them
#     unmanaged for systemd-networkd and masking their hotplug units, writes a
#     valid inventory, applies the per-boot host tuning to the fixture files
#     (vm.min_free_kbytes and busy polling only when asked), refuses an Aeron
#     hugetlbfs larger than the free 2 MiB pool, checks the kernel command
#     line as provisioned on a VM and on bare metal, and passes again once
#     the ENAs are bound (idempotent rerun);
#   * irq-affinity.sh moves every IRQ off the isolated CPUs, fails (and so
#     does the bootstrap) on a movable one it cannot place, reports one the
#     kernel refuses to move or moves only at its next interrupt, and its
#     watcher re-pins an ENA queue IRQ put back on its hint once a link event
#     follows; the bootstrap starts the watcher and provisioning installs and
#     enables it;
#   * provision-node.sh turns SMT off where the node still has it (bare metal:
#     no EC2 CPU options there) before it computes the CPU lists, so one
#     Kubernetes CPU is one physical core as the launch template leaves it;
#     it computes the kernel command line from that topology (nosmt, no
#     C-state cap), retries its network steps, holds kubelet back
#     when it fails, and does nothing on an already provisioned node; on
#     Graviton4 and Graviton5 (CPU parts 0xd4f, 0xd84) it builds the arm64
#     line (no nosmt, iommu.passthrough=1), and it fails closed on any other
#     CPU, on 64 KiB pages and without 2 MiB hugepages;
#   * userdata.sh fits EC2's user-data limit gzipped with an incompressible
#     CA, carries the kubelet settings (no Memory Manager, CPU quota left at
#     its default so every pod's CPU limit holds) and embeds the node
#     scripts as their header and code, which pass the bootstrap's dry run;
#   * launch-template.py puts CpuOptions ThreadsPerCore=1 (and the CoreCount
#     EC2 requires with it) in an x86 latency node group's launch template,
#     and none for Graviton or bare metal;
#   * the entrypoint maps a 4-core cpuset to housekeeping, sender and
#     receiver with no spinning conductor, taking the cpuset from
#     Cpus_allowed_list, never from the cgroup files a privileged pod sees.
#
# Run from anywhere:  bash node/test/test-bootstrap.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NODE_DIR="$(cd "$HERE/.." && pwd)"
PREFLIGHT="$NODE_DIR/preflight.sh"
BOOTSTRAP="$NODE_DIR/bootstrap-dpdk-ena.sh"
PROVISION="$NODE_DIR/provision-node.sh"
USERDATA="$NODE_DIR/userdata.sh"
ENTRYPOINT="$NODE_DIR/../entrypoint.sh"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
SYSFS="$TMP/sysfs"
PROC="$TMP/proc"
IMDS="$TMP/imds"
STATE="$TMP/state"
INVENTORY="$STATE/ena-pairs.json"

die() { echo "FAIL: $*" >&2; exit 1; }
# assert_file <path> <expected contents>
assert_file() {
    local got
    got="$(cat "$1")"
    [[ "$got" == "$2" ]] || die "$1 is '$got', expected '$2'"
}
# assert_grep <fixed string> <file>
assert_grep() { grep -qF -- "$1" "$2" || die "$2 lacks '$1'"; }
assert_no_grep() { ! grep -qF -- "$1" "$2" || die "$2 has '$1'"; }

# Primary on device 0, sender on 1, receiver on 2, and a VPC CNI ENI on 3
# whose MAC sorts before every other: selection is by device number, never
# by MAC order.
PRIMARY_MAC="02:00:00:00:00:01"
SENDER_MAC="02:00:00:00:00:02"
RECEIVER_MAC="02:00:00:00:00:03"
CNI_MAC="02:00:00:00:00:00"

make_pci() { # bdf iface mac numa uio
    local bdf="$1" iface="$2" mac="$3" numa="$4" uio="$5"
    local d="$SYSFS/bus/pci/devices/$bdf"
    mkdir -p "$d/net/$iface" "$SYSFS/class/net/$iface"
    echo "0x1d0f"   > "$d/vendor"                      # Amazon
    echo "0x020000" > "$d/class"                       # Ethernet controller
    echo "$mac"  > "$d/net/$iface/address"
    echo "$mac"  > "$SYSFS/class/net/$iface/address"
    echo "$numa" > "$d/numa_node"
    ln -sfn "$SYSFS/bus/pci/drivers/ena" "$d/driver"
    [[ -z "$uio" ]] || mkdir -p "$d/uio/$uio"
}

make_eni() { # mac device-number eni ip cidr
    local mac="$1" devnum="$2" eni="$3" ip="$4" cidr="$5"
    local d="$IMDS/network/interfaces/macs/$mac"
    mkdir -p "$d"
    echo "$devnum" > "$d/device-number"
    echo "$eni"    > "$d/interface-id"
    echo "$ip"     > "$d/local-ipv4s"
    echo "$cidr"   > "$d/subnet-ipv4-cidr-block"
}

# Runs a node script against the fixture with the given extra environment.
fixture_env() {
    env RUSTERON_SYSFS_ROOT="$SYSFS" \
        RUSTERON_PROC_ROOT="$PROC" \
        RUSTERON_IMDS_ROOT="$IMDS" \
        RUSTERON_STATE_DIR="$STATE" \
        RUSTERON_HUGEPAGES_DIR="$TMP/hugepages" \
        RUSTERON_SKIP_MOUNT_CHECK=1 \
        RUSTERON_PRIMARY_MAC="$PRIMARY_MAC" \
        RUSTERON_NODE_ENV="$TMP/no-node.env" \
        RUSTERON_NETWORKD_DIR="$TMP/networkd" \
        "$@"
}

# expect_fail <label> <message substring> <command...>
expect_fail() {
    local label="$1" needle="$2"
    shift 2
    if "$@" >"$TMP/out" 2>"$TMP/err"; then
        cat "$TMP/out" "$TMP/err" >&2
        die "$label: should have failed"
    fi
    grep -qF -- "$needle" "$TMP/err" || { cat "$TMP/err" >&2; die "$label: error lacks '$needle'"; }
    echo "$label: failed as expected"
}

# --- fake sysfs -------------------------------------------------------------
# A c7i.4xlarge as the launch template leaves it (CpuOptions ThreadsPerCore=1):
# 8 CPUs, 8 cores, one thread per core, one NUMA node. The SMT-on twin world
# of a bare metal first boot (16 CPUs, twins N and N+8) is section 5b.
mkdir -p "$SYSFS/kernel/iommu_groups" \
    "$SYSFS/bus/pci/drivers/ena" "$SYSFS/bus/pci/drivers/igb_uio" "$SYSFS/bus/pci/drivers/vfio-pci" \
    "$SYSFS/module/uio" "$SYSFS/module/igb_uio" \
    "$SYSFS/kernel/mm/hugepages/hugepages-2048kB" "$SYSFS/kernel/mm/transparent_hugepage" "$SYSFS/kernel/mm/ksm" \
    "$SYSFS/devices/system/node/node0"
echo 2048 > "$SYSFS/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages"
echo 2048 > "$SYSFS/kernel/mm/hugepages/hugepages-2048kB/free_hugepages"
echo "always [madvise] never" > "$SYSFS/kernel/mm/transparent_hugepage/enabled"
echo 1 > "$SYSFS/kernel/mm/ksm/run"
mkdir -p "$SYSFS/devices/system/cpu/smt"
echo 0-7 > "$SYSFS/devices/system/cpu/online"
for cpu in $(seq 0 7); do
    mkdir -p "$SYSFS/devices/system/cpu/cpu$cpu/topology"
    echo "$cpu" > "$SYSFS/devices/system/cpu/cpu$cpu/topology/thread_siblings_list"
done
echo 0 > "$SYSFS/devices/system/cpu/smt/active"
echo on > "$SYSFS/devices/system/cpu/smt/control"
# An EBS NVMe controller: Amazon's vendor id too, but no ENA.
mkdir -p "$SYSFS/bus/pci/devices/0000:00:04.0"
echo "0x1d0f"   > "$SYSFS/bus/pci/devices/0000:00:04.0/vendor"
echo "0x010802" > "$SYSFS/bus/pci/devices/0000:00:04.0/class"
make_pci 0000:00:05.0 eth0 "$PRIMARY_MAC"  0 ""     # primary (excluded)
make_pci 0000:00:06.0 eth1 "$SENDER_MAC"   0 uio0   # sender
make_pci 0000:00:07.0 eth2 "$RECEIVER_MAC" 0 uio1   # receiver
make_pci 0000:00:08.0 eth3 "$CNI_MAC"      0 ""     # VPC CNI ENI (excluded)

# --- fake /proc -------------------------------------------------------------
mkdir -p "$PROC/irq/24" "$PROC/irq/25" "$PROC/sys/kernel" "$PROC/sys/vm" "$PROC/sys/net/core"
echo 0-7 > "$PROC/irq/24/smp_affinity_list"
echo 0-7 > "$PROC/irq/25/smp_affinity_list"
echo ff > "$PROC/irq/default_smp_affinity"
echo 1 > "$PROC/sys/kernel/numa_balancing"
echo 67584 > "$PROC/sys/vm/min_free_kbytes"
echo 0 > "$PROC/sys/net/core/busy_poll"
echo 0 > "$PROC/sys/net/core/busy_read"
echo 212992 > "$PROC/sys/net/core/rmem_max"
echo 212992 > "$PROC/sys/net/core/wmem_max"
printf 'MemTotal:       32000000 kB\nMemFree:        30000000 kB\n' > "$PROC/meminfo"
# A VM (the hypervisor flag): c7i.4xlarge. METAL_CPUINFO is c7i.metal-24xl's.
VM_CPUINFO="$(printf 'processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Xeon(R) Platinum 8488C\nflags\t\t: fpu tsc constant_tsc nonstop_tsc avx512f hypervisor waitpkg\n')"
METAL_CPUINFO="${VM_CPUINFO/ hypervisor/}"
echo "$VM_CPUINFO" > "$PROC/cpuinfo"
ISO="1-7"
# As provision-node.sh writes it: nosmt always, never a C-state cap (the
# committed A/Bs gave it no gain: io/benches/BENCHMARK.md cstate on a VM,
# md-dpdk/BENCHMARKS.md round 4 on metal).
ISOLATION="nosmt isolcpus=nohz,domain,managed_irq,$ISO nohz_full=$ISO rcu_nocbs=$ISO irqaffinity=0"
# The clocksource is no longer forced by default: only bare metal with
# RUSTERON_FORCE_TSC=1 gets it, and a VM never does (tsc=reliable turns the
# kernel's TSC checks off).
COMMON_ARGS="iommu=pt transparent_hugepage=never hugepagesz=2M hugepages=2048"
TSC_ARGS="clocksource=tsc tsc=reliable $COMMON_ARGS"
PROVISIONED_CMDLINE="BOOT_IMAGE=(hd0,gpt1)/boot/vmlinuz root=UUID=x ro $ISOLATION $COMMON_ARGS"
METAL_CMDLINE="$PROVISIONED_CMDLINE"
echo "$PROVISIONED_CMDLINE" > "$PROC/cmdline"

# --- fake IMDS --------------------------------------------------------------
make_eni "$PRIMARY_MAC"  0 eni-1234567890abcdef0 10.0.0.10 10.0.0.0/24
make_eni "$SENDER_MAC"   1 eni-1234567890abcdef1 10.0.1.4  10.0.1.0/24
make_eni "$RECEIVER_MAC" 2 eni-1234567890abcdef2 10.0.1.5  10.0.1.0/24
make_eni "$CNI_MAC"      3 eni-1234567890abcdef3 10.0.0.77 10.0.0.0/24

# --- 1. preflight (igb_uio) -------------------------------------------------
echo "== preflight (healthy igb_uio fixture) =="
fixture_env "$PREFLIGHT"
echo "preflight PASSED (as expected)"

# Bare metal has a real IOMMU: igb_uio programs none, so its DMA passes only
# while every group is in passthrough (iommu=pt).
mkdir -p "$SYSFS/kernel/iommu_groups/7" "$SYSFS/kernel/iommu_groups/8"
expect_fail "preflight igb_uio, IOMMU groups without a type" "iommu=pt" fixture_env "$PREFLIGHT"
echo identity > "$SYSFS/kernel/iommu_groups/7/type"
echo DMA-FQ > "$SYSFS/kernel/iommu_groups/8/type"
expect_fail "preflight igb_uio behind a translating IOMMU" "iommu=pt" fixture_env "$PREFLIGHT"
assert_grep "8:DMA-FQ" "$TMP/err"
echo identity > "$SYSFS/kernel/iommu_groups/8/type"
fixture_env "$PREFLIGHT" 2>"$TMP/err" || { cat "$TMP/err" >&2; die "preflight igb_uio with IOMMU passthrough failed"; }
assert_grep "passthrough" "$TMP/err"
rm -rf "$SYSFS/kernel/iommu_groups/7" "$SYSFS/kernel/iommu_groups/8"

# A kernel without an IOMMU may have no iommu_groups directory at all.
mv "$SYSFS/kernel/iommu_groups" "$TMP/iommu_groups"
fixture_env "$PREFLIGHT" 2>"$TMP/err" || { cat "$TMP/err" >&2; die "preflight igb_uio without iommu_groups failed"; }
assert_grep "no IOMMU groups" "$TMP/err"
expect_fail "preflight vfio-pci without iommu_groups" "IOMMU groups missing" \
    fixture_env RUSTERON_DPDK_DRIVER=vfio-pci RUSTERON_DPDK_VFIO_WC=1 "$PREFLIGHT"
mv "$TMP/iommu_groups" "$SYSFS/kernel/iommu_groups"

mv "$SYSFS/module/igb_uio" "$TMP/igb_uio.module"
expect_fail "preflight igb_uio not loaded" "igb_uio" fixture_env "$PREFLIGHT"
mv "$TMP/igb_uio.module" "$SYSFS/module/igb_uio"

expect_fail "preflight unknown driver" "RUSTERON_DPDK_DRIVER" \
    fixture_env RUSTERON_DPDK_DRIVER=uio_pci_generic "$PREFLIGHT"
expect_fail "preflight device number 0" "RUSTERON_DPDK_DEVICE_NUMBERS" \
    fixture_env RUSTERON_DPDK_DEVICE_NUMBERS="0 1" "$PREFLIGHT"
expect_fail "preflight absent device number" "device-number 9" \
    fixture_env RUSTERON_DPDK_DEVICE_NUMBERS="1 9" "$PREFLIGHT"

# --- 2. bootstrap writes a complete inventory and tunes the host -----------
echo "== bootstrap (dry run) =="
fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP" >"$TMP/bootstrap.out" 2>"$TMP/bootstrap.err" \
    || { cat "$TMP/bootstrap.err" >&2; die "bootstrap failed"; }
cat "$TMP/bootstrap.err"
[[ ! -s "$TMP/bootstrap.out" ]] || die "bootstrap wrote to stdout: $(cat "$TMP/bootstrap.out")"
[[ -f "$INVENTORY" ]] || die "inventory not written"
echo "--- inventory ---"
cat "$INVENTORY"
python3 -c 'import json, sys; json.load(open(sys.argv[1]))' "$INVENTORY" || die "inventory is not valid JSON"

# The pair is the device-number 1 and 2 ENAs, with full identity.
python3 - "$INVENTORY" "$PRIMARY_MAC" <<'PY' || die "inventory content"
import json, sys
inv = json.load(open(sys.argv[1]))
assert inv["primary_ena_mac"] == sys.argv[2], inv["primary_ena_mac"]
(pair,) = inv["pairs"]
s, r = pair["sender"], pair["receiver"]
expect = {
    "sender": dict(pci="0000:00:06.0", driver="igb_uio", uio="uio0", iommu_group="",
                   eni_id="eni-1234567890abcdef1", mac="02:00:00:00:00:02", ipv4="10.0.1.4",
                   prefix_len=24, subnet_cidr="10.0.1.0/24", gateway="10.0.1.1", numa_node=0,
                   health="healthy"),
    "receiver": dict(pci="0000:00:07.0", driver="igb_uio", uio="uio1", iommu_group="",
                     eni_id="eni-1234567890abcdef2", mac="02:00:00:00:00:03", ipv4="10.0.1.5",
                     prefix_len=24, subnet_cidr="10.0.1.0/24", gateway="10.0.1.1", numa_node=0,
                     health="healthy"),
}
for role, want in expect.items():
    for k, v in want.items():
        assert pair[role][k] == v, (role, k, pair[role][k], v)
PY
assert_no_grep "0000:00:08.0" "$INVENTORY"
assert_no_grep "$CNI_MAC" "$INVENTORY"
assert_no_grep "0000:00:05.0" "$INVENTORY"
assert_grep "dry-run: would bind 0000:00:06.0 to igb_uio" "$TMP/bootstrap.err"
assert_grep "dry-run: would bind 0000:00:07.0 to igb_uio" "$TMP/bootstrap.err"
assert_no_grep "0000:00:08.0" "$TMP/bootstrap.err"
# Aeron's directory on its own sized hugetlbfs (hugetlbfs reports no free space
# without a size, and Aeron's storage check then refuses every log buffer)
assert_grep "dry-run: would mount hugetlbfs pagesize=2M,size=1G at /mnt/huge-aeron" "$TMP/bootstrap.err"
assert_grep "dry-run: would disable irqbalance" "$TMP/bootstrap.err"
# Every IRQ stays on the housekeeping CPUs after boot too: the ENA driver puts
# its queue IRQs back on their per-queue hints whenever it brings an interface
# up, so the bootstrap starts the watcher that re-pins them on each link event.
assert_grep "dry-run: would start rusteron-irq-affinity.service" "$TMP/bootstrap.err"
# The primary ENA's kernel path keeps its driver's interrupt coalescing: no
# committed run measures a change there.
assert_no_grep "coalescing" "$TMP/bootstrap.err"
assert_no_grep "ethtool" "$TMP/bootstrap.err"

# Before either ENA leaves the kernel, systemd-networkd is told to leave both
# alone, and their hotplug units (the EKS AMI's udev-net-manager, stock
# amazon-ec2-net-utils' policy routes) are masked for this boot, so an ENA
# handed back to the kernel is never re-addressed by DHCP behind our back.
NETWORKD_FILE="$TMP/networkd/00-rusteron-dpdk-data.network"
[[ -f "$NETWORKD_FILE" ]] || die "no $NETWORKD_FILE"
assert_grep "MACAddress=$SENDER_MAC $RECEIVER_MAC" "$NETWORKD_FILE"
assert_grep "Unmanaged=yes" "$NETWORKD_FILE"
assert_no_grep "$PRIMARY_MAC" "$NETWORKD_FILE"
assert_grep "dry-run: would reload systemd-networkd" "$TMP/bootstrap.err"
for iface in eth1 eth2; do
    assert_grep "dry-run: would mask for this boot: udev-net-manager@$iface.service policy-routes@$iface.service refresh-policy-routes@$iface.timer" "$TMP/bootstrap.err"
done
assert_no_grep "@eth0" "$TMP/bootstrap.err"
assert_no_grep "@eth3" "$TMP/bootstrap.err"
# networkd before the binds: the unmanaged marking precedes the first bind.
grep -n -e 'unmanaged' -e 'would bind' "$TMP/bootstrap.err" | head -1 | grep -q unmanaged \
    || die "the ENAs were bound before systemd-networkd was told to leave them alone"

# Per-boot host tuning, written to the fixture files.
assert_file "$PROC/irq/24/smp_affinity_list" "0"
assert_file "$PROC/irq/25/smp_affinity_list" "0"
assert_file "$PROC/irq/default_smp_affinity" "1"
assert_file "$SYSFS/kernel/mm/transparent_hugepage/enabled" "never"
assert_file "$SYSFS/kernel/mm/ksm/run" "0"
assert_file "$PROC/sys/kernel/numa_balancing" "0"
# The kernel's own reserve unless RUSTERON_MIN_FREE_PCT is set.
assert_file "$PROC/sys/vm/min_free_kbytes" "67584"
# Busy polling only when asked (RUSTERON_BUSY_POLL_US): off by default.
assert_file "$PROC/sys/net/core/busy_poll" "0"
assert_file "$PROC/sys/net/core/busy_read" "0"
# Aeron's 2 MiB socket buffers (and so its 2 MiB receive window) need these
assert_file "$PROC/sys/net/core/rmem_max" "16777216"
assert_file "$PROC/sys/net/core/wmem_max" "16777216"

# The optional settings, each when set and when out of range.
fixture_env RUSTERON_DRY_RUN=1 RUSTERON_MIN_FREE_PCT=2 RUSTERON_BUSY_POLL_US=50 "$BOOTSTRAP" 2>"$TMP/opt.err" \
    || { cat "$TMP/opt.err" >&2; die "bootstrap with RUSTERON_MIN_FREE_PCT and RUSTERON_BUSY_POLL_US failed"; }
assert_file "$PROC/sys/vm/min_free_kbytes" "640000"
assert_file "$PROC/sys/net/core/busy_poll" "50"
assert_file "$PROC/sys/net/core/busy_read" "50"
# networkd's file was already right: no second reload.
assert_no_grep "would reload systemd-networkd" "$TMP/opt.err"
echo 67584 > "$PROC/sys/vm/min_free_kbytes"
expect_fail "bootstrap min_free 7%" "RUSTERON_MIN_FREE_PCT" fixture_env RUSTERON_DRY_RUN=1 RUSTERON_MIN_FREE_PCT=7 "$BOOTSTRAP"
expect_fail "bootstrap busy poll not a number" "RUSTERON_BUSY_POLL_US" fixture_env RUSTERON_DRY_RUN=1 RUSTERON_BUSY_POLL_US=fast "$BOOTSTRAP"
assert_file "$PROC/sys/vm/min_free_kbytes" "67584"

# Aeron's hugetlbfs must fit in the free 2 MiB pool (2048 pages, 4 GiB here).
expect_fail "bootstrap Aeron hugetlbfs over the free pool" "free" \
    fixture_env RUSTERON_DRY_RUN=1 RUSTERON_AERON_HUGE_SIZE=5G "$BOOTSTRAP"
echo 100 > "$SYSFS/kernel/mm/hugepages/hugepages-2048kB/free_hugepages"
expect_fail "bootstrap Aeron hugetlbfs over a used pool" "200 MiB" fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP"
echo 2048 > "$SYSFS/kernel/mm/hugepages/hugepages-2048kB/free_hugepages"
expect_fail "bootstrap Aeron hugetlbfs size unparsed" "RUSTERON_AERON_HUGE_SIZE" \
    fixture_env RUSTERON_DRY_RUN=1 RUSTERON_AERON_HUGE_SIZE=50% "$BOOTSTRAP"
fixture_env RUSTERON_DRY_RUN=1 RUSTERON_AERON_HUGE_SIZE=4096M "$BOOTSTRAP" 2>"$TMP/opt.err" \
    || { cat "$TMP/opt.err" >&2; die "bootstrap with the whole free pool for Aeron failed"; }
echo "bootstrap assertions PASSED"

# --- 2b. IRQ placement after boot (irq-affinity.sh) ------------------------
# The ENA driver applies its per-queue affinity hint (queue i on CPU i) as the
# queue IRQ's affinity whenever it brings an interface up: after a device
# reset, a queue or ring change, or a data ENA's return to the kernel driver.
# They then land on the isolated CPUs until something re-pins them.
echo "== IRQ placement (irq-affinity.sh) =="
IRQ_AFFINITY="$NODE_DIR/irq-affinity.sh"
make_irq() { # irq smp_affinity_list effective_affinity_list ("" for none) action
    mkdir -p "$PROC/irq/$1/$4"
    echo "$2" > "$PROC/irq/$1/smp_affinity_list"
    [[ -z "$3" ]] || echo "$3" > "$PROC/irq/$1/effective_affinity_list"
}
# A primary ENA queue on its hint, CPU 3 (no effective affinity file: a kernel
# without one, so the requested affinity is where it fires), and the ENA's
# management IRQ already on CPU 0. One pass moves the first, leaves the
# second, and passes: nothing is left on an isolated CPU.
make_irq 26 3 "" eth0-Tx-Rx-3
make_irq 27 0 0 ena-mgmnt@pci:0000:00:05.0
# A vector allocated but never requested (no handler directory: an ENA
# allocates vectors for every queue it could have) fires nowhere, wherever x86
# parks it (the first online CPU, isolated when housekeeping is not CPU 0).
mkdir -p "$PROC/irq/30"
echo 0 > "$PROC/irq/30/smp_affinity_list"
echo 3 > "$PROC/irq/30/effective_affinity_list"
fixture_env "$IRQ_AFFINITY" pin 2>"$TMP/irq.err" || { cat "$TMP/irq.err" >&2; die "irq-affinity pin failed"; }
cat "$TMP/irq.err"
assert_file "$PROC/irq/26/smp_affinity_list" "0"
assert_file "$PROC/irq/27/smp_affinity_list" "0"
assert_grep "IRQs on CPUs 0 (1 moved, 0 refused by the kernel)" "$TMP/irq.err"
assert_no_grep "isolated" "$TMP/irq.err"

# The IRQ CPUs sleep no deeper than C1 (exit latency 1 us): on c7i a deeper
# state on the CPU that takes the NIC's interrupts cost p99 315-559 against
# 44-49 us (book/src/deploy/hft.md). Deeper states are disabled; POLL and C1
# stay; an isolated CPU's states are not touched. Graviton (no cpuidle
# driver) has nothing to cap.
make_idle() { # cpu state name latency
    mkdir -p "$SYSFS/devices/system/cpu/cpu$1/cpuidle/state$2"
    echo "$3" > "$SYSFS/devices/system/cpu/cpu$1/cpuidle/state$2/name"
    echo "$4" > "$SYSFS/devices/system/cpu/cpu$1/cpuidle/state$2/latency"
    echo 0 > "$SYSFS/devices/system/cpu/cpu$1/cpuidle/state$2/disable"
}
for cpu in 0 3; do
    make_idle "$cpu" 0 POLL 0
    make_idle "$cpu" 1 C1 1
    make_idle "$cpu" 2 C1E 2
    make_idle "$cpu" 3 C6 170
done
fixture_env "$IRQ_AFFINITY" pin 2>"$TMP/irq.err" || { cat "$TMP/irq.err" >&2; die "irq-affinity pin failed with cpuidle states"; }
for st in 0 1; do assert_file "$SYSFS/devices/system/cpu/cpu0/cpuidle/state$st/disable" "0"; done
for st in 2 3; do assert_file "$SYSFS/devices/system/cpu/cpu0/cpuidle/state$st/disable" "1"; done
for st in 0 1 2 3; do assert_file "$SYSFS/devices/system/cpu/cpu3/cpuidle/state$st/disable" "0"; done
assert_grep "IRQ CPUs 0 sleep no deeper than C1 (2 deeper states disabled)" "$TMP/irq.err"
# Opted out: deep idle left alone.
for st in 2 3; do echo 0 > "$SYSFS/devices/system/cpu/cpu0/cpuidle/state$st/disable"; done
fixture_env RUSTERON_IRQ_CPU_DEEP_IDLE=1 "$IRQ_AFFINITY" pin 2>"$TMP/irq.err" || { cat "$TMP/irq.err" >&2; die "irq-affinity pin failed, deep idle allowed"; }
for st in 2 3; do assert_file "$SYSFS/devices/system/cpu/cpu0/cpuidle/state$st/disable" "0"; done
assert_grep "RUSTERON_IRQ_CPU_DEEP_IDLE=1" "$TMP/irq.err"
rm -rf "$SYSFS"/devices/system/cpu/cpu[03]/cpuidle

# x86 without interrupt remapping (an EC2 VM) moves an MSI only at its next
# interrupt: its requested affinity reads the housekeeping CPUs at once, its
# effective one the isolated CPU until it fires there again, which an idle
# queue may not do for minutes. On its way, so reported, not fatal.
make_irq 31 6 6 eth0-Tx-Rx-6
fixture_env "$IRQ_AFFINITY" pin 2>"$TMP/irq.err" \
    || { cat "$TMP/irq.err" >&2; die "irq-affinity pin failed on an IRQ whose move waits for its next interrupt"; }
assert_file "$PROC/irq/31/smp_affinity_list" "0"
assert_grep "IRQ 31 (eth0-Tx-Rx-6) on isolated CPUs 6 until its next interrupt" "$TMP/irq.err"
fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP" 2>"$TMP/irq.err" \
    || { cat "$TMP/irq.err" >&2; die "bootstrap failed on an IRQ whose move waits for its next interrupt"; }
assert_grep "IRQ 31 (eth0-Tx-Rx-6) on isolated CPUs 6 until its next interrupt" "$TMP/irq.err"
rm -rf "$PROC/irq/31"

# An IRQ the pass cannot place fails the check, named, and so does the
# bootstrap, which then writes no inventory: the kernel took the write, but
# the requested affinity does not read back as the housekeeping CPUs (a link
# to /dev/null stands in: it takes any write and reads back nothing), and it
# fires on an isolated CPU.
mkdir -p "$PROC/irq/28/eth1-Tx-Rx-5"
ln -s /dev/null "$PROC/irq/28/smp_affinity_list"
echo 5 > "$PROC/irq/28/effective_affinity_list"
expect_fail "irq-affinity, a movable IRQ left on an isolated CPU" "IRQ 28 (eth1-Tx-Rx-5) on CPUs 5" \
    fixture_env "$IRQ_AFFINITY" pin
assert_grep "isolated" "$TMP/err"
expect_fail "bootstrap, an IRQ left on an isolated CPU" "IRQ 28 (eth1-Tx-Rx-5) on CPUs 5" \
    fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP"
[[ ! -e "$INVENTORY" ]] || die "a bootstrap with an IRQ on an isolated CPU left an inventory behind"
rm -rf "$PROC/irq/28"

# An IRQ the kernel refuses to move (a managed one: an NVMe queue whose spread
# holds isolated CPUs only, which fires only for I/O issued there) is the
# kernel's to keep: reported by name, not fatal. A link into a directory that
# does not exist stands in for a file no write reaches, root's included.
mkdir -p "$PROC/irq/29/nvme0q6"
ln -s "$TMP/no-such-dir/smp_affinity_list" "$PROC/irq/29/smp_affinity_list"
echo 6 > "$PROC/irq/29/effective_affinity_list"
fixture_env "$IRQ_AFFINITY" pin 2>"$TMP/irq.err" || { cat "$TMP/irq.err" >&2; die "irq-affinity pin failed on a managed IRQ"; }
assert_grep "IRQ 29 (nvme0q6) on isolated CPUs 6: the kernel refuses to move it" "$TMP/irq.err"
assert_grep "IRQs on CPUs 0 (0 moved, 1 refused by the kernel)" "$TMP/irq.err"
fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP" 2>"$TMP/irq.err" \
    || { cat "$TMP/irq.err" >&2; die "bootstrap failed on a managed IRQ"; }
assert_grep "IRQ 29 (nvme0q6) on isolated CPUs 6" "$TMP/irq.err"

# The watcher (rusteron-irq-affinity.service): a pass at start, then one after
# each link event. The fake monitor puts IRQ 26 back on its hint after the
# first pass, as ena_up does, reports the link event, and ends, which ends the
# watcher (systemd restarts it).
cat > "$TMP/link-monitor" <<EOF
#!/usr/bin/env bash
sleep 2
echo 3 > "$PROC/irq/26/smp_affinity_list"
echo "2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 9001 qdisc mq state UP mode DEFAULT"
EOF
chmod +x "$TMP/link-monitor"
expect_fail "irq-affinity watch, its link monitor ended" "link monitor" \
    fixture_env RUSTERON_LINK_MONITOR="$TMP/link-monitor" "$IRQ_AFFINITY" watch
assert_file "$PROC/irq/26/smp_affinity_list" "0"
assert_grep "moved IRQ 26 (eth0-Tx-Rx-3) from CPUs 3 to 0" "$TMP/err"
expect_fail "irq-affinity without a mode" "usage" fixture_env "$IRQ_AFFINITY"
expect_fail "irq-affinity, housekeeping CPUs unparsed" "RUSTERON_HOUSEKEEPING_CPUS" \
    fixture_env RUSTERON_HOUSEKEEPING_CPUS="0;8" "$IRQ_AFFINITY" pin
rm -rf "$PROC/irq/26" "$PROC/irq/27" "$PROC/irq/29" "$PROC/irq/30"

# Housekeeping is 0-2 and the IRQ destination is CPU 2. An IRQ on helper CPU 0
# is inside housekeeping; pin must still move it. Leaving it would put
# interrupts on a shared CPU. CPU 2 is already the destination. CPU 5 is isolated.
make_irq 42 0 0 eth0-Tx-Rx-0
make_irq 43 2 2 ena-mgmnt
make_irq 44 5 5 eth0-Tx-Rx-5
fixture_env RUSTERON_HOUSEKEEPING_CPUS=0-2 RUSTERON_IRQ_CPUS=2 "$IRQ_AFFINITY" pin 2>"$TMP/irq.err" \
    || { cat "$TMP/irq.err" >&2; die "irq-affinity left a helper CPU in the IRQ set"; }
assert_file "$PROC/irq/42/smp_affinity_list" "2"
assert_file "$PROC/irq/43/smp_affinity_list" "2"
assert_file "$PROC/irq/44/smp_affinity_list" "2"
assert_grep "IRQs on CPUs 2" "$TMP/irq.err"
# The kernel refuses the write, and the IRQ stays on helper CPU 0. That is not
# a managed IRQ on an isolated CPU: pin fails.
mkdir -p "$PROC/irq/45/eth0-Tx-Rx-1"
ln -s "$TMP/no-such-dir/smp_affinity_list" "$PROC/irq/45/smp_affinity_list"
echo 0 > "$PROC/irq/45/effective_affinity_list"
expect_fail "irq-affinity, helper CPU kept an IRQ" "outside the IRQ destination" \
    fixture_env RUSTERON_HOUSEKEEPING_CPUS=0-2 RUSTERON_IRQ_CPUS=2 "$IRQ_AFFINITY" pin
expect_fail "irq-affinity, IRQ CPU outside housekeeping" "not a housekeeping CPU" \
    fixture_env RUSTERON_HOUSEKEEPING_CPUS=0-2 RUSTERON_IRQ_CPUS=5 "$IRQ_AFFINITY" pin
rm -rf "$PROC/irq/42" "$PROC/irq/43" "$PROC/irq/44" "$PROC/irq/45"
echo "IRQ placement assertions PASSED"

# --- 3. idempotent rerun: the ENAs are bound and their netdevs are gone ----
echo "== bootstrap rerun (ENAs already on igb_uio) =="
for bdf in 0000:00:06.0 0000:00:07.0; do
    rm -rf "$SYSFS/bus/pci/devices/$bdf/net"
    ln -sfn "$SYSFS/bus/pci/drivers/igb_uio" "$SYSFS/bus/pci/devices/$bdf/driver"
done
rm -rf "$SYSFS/class/net/eth1" "$SYSFS/class/net/eth2"
# No inventory either, as after a run that failed between binding and writing it.
rm -f "$INVENTORY"
fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP" 2>"$TMP/rerun.err" \
    || { cat "$TMP/rerun.err" >&2; die "bootstrap rerun failed"; }
python3 -c 'import json, sys; json.load(open(sys.argv[1]))' "$INVENTORY" || die "rerun inventory is not valid JSON"
assert_grep '"pci":"0000:00:06.0"' "$INVENTORY"
assert_grep '"pci":"0000:00:07.0"' "$INVENTORY"
assert_grep "0000:00:06.0 already bound to igb_uio" "$TMP/rerun.err"
assert_no_grep "would bind" "$TMP/rerun.err"
# No netdev left to name a unit after, and networkd's file is unchanged.
assert_no_grep "would mask" "$TMP/rerun.err"
assert_no_grep "would reload systemd-networkd" "$TMP/rerun.err"
echo "idempotent rerun PASSED"

# A recorded BDF that is no longer on igb_uio is never reused.
rm -f "$SYSFS/bus/pci/devices/0000:00:06.0/driver"
expect_fail "bootstrap stale binding record" "no PCI BDF" fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP"
[[ ! -e "$INVENTORY" ]] || die "a failed bootstrap left an inventory behind"
ln -sfn "$SYSFS/bus/pci/drivers/igb_uio" "$SYSFS/bus/pci/devices/0000:00:06.0/driver"

# Dry run is for fixtures only: it would write the host's real /proc.
expect_fail "bootstrap dry run on the host's /proc" "RUSTERON_DRY_RUN" \
    fixture_env RUSTERON_PROC_ROOT=/proc RUSTERON_DRY_RUN=1 "$BOOTSTRAP"
expect_fail "bootstrap dry run on the host's /sys" "RUSTERON_DRY_RUN" \
    fixture_env RUSTERON_SYSFS_ROOT=/sys RUSTERON_DRY_RUN=1 "$BOOTSTRAP"
expect_fail "bootstrap dry run on the host's networkd directory" "RUSTERON_DRY_RUN" \
    fixture_env RUSTERON_NETWORKD_DIR=/etc/systemd/network RUSTERON_DRY_RUN=1 "$BOOTSTRAP"

# A kernel command line without the isolation flags fails the boot's bootstrap.
echo "BOOT_IMAGE=/boot/vmlinuz ro" > "$PROC/cmdline"
expect_fail "bootstrap unprovisioned kernel" "isolcpus" fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP"
echo "${PROVISIONED_CMDLINE/irqaffinity=0/irqaffinity=1}" > "$PROC/cmdline"
expect_fail "bootstrap irqaffinity mismatch" "irqaffinity" fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP"
# Three reserved CPUs, interrupts on CPU 2 only. irqaffinity=0-2 would include
# the helper CPUs and must fail. irqaffinity=2 matches RUSTERON_IRQ_CPUS.
echo "${PROVISIONED_CMDLINE/irqaffinity=0/irqaffinity=0-2}" > "$PROC/cmdline"
expect_fail "bootstrap irqaffinity includes helper CPUs" "irqaffinity" \
    fixture_env RUSTERON_HOUSEKEEPING_CPUS=0-2 RUSTERON_IRQ_CPUS=2 RUSTERON_DRY_RUN=1 "$BOOTSTRAP"
echo "${PROVISIONED_CMDLINE/irqaffinity=0/irqaffinity=2}" > "$PROC/cmdline"
fixture_env RUSTERON_HOUSEKEEPING_CPUS=0-2 RUSTERON_IRQ_CPUS=2 RUSTERON_DRY_RUN=1 "$BOOTSTRAP" 2>"$TMP/irqdest.err" \
    || { cat "$TMP/irqdest.err" >&2; die "bootstrap rejected IRQ destination CPU 2"; }
assert_grep "IRQs on 2" "$TMP/irqdest.err"
assert_file "$PROC/irq/default_smp_affinity" "4"
# No node needs a C-state cap: a VM and bare metal both pass without one.
echo "$METAL_CMDLINE" > "$PROC/cmdline"
fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP" 2>"$TMP/vm.err" \
    || { cat "$TMP/vm.err" >&2; die "bootstrap on a VM without the C-state cap failed"; }
assert_no_grep "max_cstate" "$TMP/vm.err"
echo "$METAL_CPUINFO" > "$PROC/cpuinfo"
fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP" 2>"$TMP/metal.err" \
    || { cat "$TMP/metal.err" >&2; die "bootstrap on bare metal without the C-state cap failed"; }
# Bare metal's IOMMU: igb_uio needs every group in passthrough, iommu=pt.
mkdir -p "$SYSFS/kernel/iommu_groups/7"
echo identity > "$SYSFS/kernel/iommu_groups/7/type"
fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP" 2>"$TMP/metal.err" \
    || { cat "$TMP/metal.err" >&2; die "bootstrap on bare metal with IOMMU passthrough failed"; }
echo "${METAL_CMDLINE/ iommu=pt/}" > "$PROC/cmdline"
expect_fail "bootstrap IOMMU without iommu=pt" "iommu=pt" fixture_env RUSTERON_DRY_RUN=1 "$BOOTSTRAP"
rm -rf "$SYSFS/kernel/iommu_groups/7"
echo "$VM_CPUINFO" > "$PROC/cpuinfo"
echo "$PROVISIONED_CMDLINE" > "$PROC/cmdline"

# --- 4. vfio-pci needs write-combining and real IOMMU backing ---------------
echo "== preflight and bootstrap (vfio-pci) =="
# Back on the kernel's ena driver, as after a reboot.
make_pci 0000:00:06.0 eth1 "$SENDER_MAC"   0 uio0
make_pci 0000:00:07.0 eth2 "$RECEIVER_MAC" 0 uio1
for g in 11 12; do mkdir -p "$SYSFS/kernel/iommu_groups/$g"; done
ln -sfn "$SYSFS/kernel/iommu_groups/11" "$SYSFS/bus/pci/devices/0000:00:06.0/iommu_group"
ln -sfn "$SYSFS/kernel/iommu_groups/12" "$SYSFS/bus/pci/devices/0000:00:07.0/iommu_group"
mkdir -p "$SYSFS/module/vfio_iommu_type1"
expect_fail "preflight vfio-pci without write-combining" "RUSTERON_DPDK_VFIO_WC" \
    fixture_env RUSTERON_DPDK_DRIVER=vfio-pci "$PREFLIGHT"
fixture_env RUSTERON_DPDK_DRIVER=vfio-pci RUSTERON_DPDK_VFIO_WC=1 "$PREFLIGHT"
fixture_env RUSTERON_DPDK_DRIVER=vfio-pci RUSTERON_DPDK_VFIO_WC=1 RUSTERON_DRY_RUN=1 "$BOOTSTRAP" 2>"$TMP/vfio.err" \
    || { cat "$TMP/vfio.err" >&2; die "vfio-pci bootstrap failed"; }
assert_grep '"driver":"vfio-pci"' "$INVENTORY"
assert_grep '"iommu_group":"11"' "$INVENTORY"
assert_grep '"iommu_group":"12"' "$INVENTORY"
assert_grep "dry-run: would bind 0000:00:06.0 to vfio-pci (from ena)" "$TMP/vfio.err"
rm -rf "$SYSFS/module/vfio_iommu_type1"
expect_fail "preflight vfio-pci without vfio_iommu_type1" "vfio_iommu_type1" \
    fixture_env RUSTERON_DPDK_DRIVER=vfio-pci RUSTERON_DPDK_VFIO_WC=1 "$PREFLIGHT"
rm -f "$SYSFS/bus/pci/devices/0000:00:0"{6,7}".0/iommu_group"
rmdir "$SYSFS/kernel/iommu_groups/"{11,12}

# --- 5. provision-node.sh ----------------------------------------------------
echo "== provision-node.sh (dry run) =="
echo "BOOT_IMAGE=/boot/vmlinuz ro" > "$PROC/cmdline"
printf 'RUSTERON_HOUSEKEEPING_CPUS=0\nRUSTERON_DPDK_DEVICE_NUMBERS="1 2"\nRUSTERON_HUGEPAGES=2048\n' > "$TMP/node.env"
fixture_env RUSTERON_NODE_ENV="$TMP/node.env" RUSTERON_DRY_RUN=1 "$PROVISION" 2>"$TMP/provision.err" \
    || { cat "$TMP/provision.err" >&2; die "provision failed"; }
cat "$TMP/provision.err"
assert_grep "dry-run: kernel args: $ISOLATION $COMMON_ARGS" "$TMP/provision.err"
assert_no_grep "max_cstate" "$TMP/provision.err"
# A VM never gets a clocksource override: the hypervisor's is the kernel's
# choice, and tsc=reliable would turn the kernel's TSC checks off.
assert_no_grep "clocksource=" "$TMP/provision.err"
assert_grep "clocksource left to the kernel: this is a VM" "$TMP/provision.err"
assert_grep "would write /opt/rusteron-dpdk/clocksource-policy: kernel: a VM gets no clocksource override" "$TMP/provision.err"
assert_grep "dry-run: would run, retried: install_build_deps" "$TMP/provision.err"
assert_grep "dry-run: would run, retried: fetch_dpdk_kmods" "$TMP/provision.err"
assert_grep "dry-run: would run: systemctl enable rusteron-dpdk-ena.service" "$TMP/provision.err"
# The IRQ watcher is installed and enabled with the bootstrap.
assert_grep "irq-affinity.sh /usr/local/sbin/irq-affinity.sh" "$TMP/provision.err"
assert_grep "rusteron-irq-affinity.service /etc/systemd/system/rusteron-irq-affinity.service" "$TMP/provision.err"
assert_grep "dry-run: would run: systemctl enable rusteron-dpdk-ena.service rusteron-irq-affinity.service" "$TMP/provision.err"
assert_grep "dry-run: would run: systemctl reboot" "$TMP/provision.err"
assert_no_grep "hold kubelet" "$TMP/provision.err"
# SMT is already off here (the launch template's ThreadsPerCore=1): nothing to
# write, and nosmt is still on the command line for every later boot.
assert_no_grep "SMT" "$TMP/provision.err"
# Bare metal (no hypervisor flag): no C-state cap, iommu=pt for igb_uio.
echo "$METAL_CPUINFO" > "$PROC/cpuinfo"
fixture_env RUSTERON_NODE_ENV="$TMP/node.env" RUSTERON_DRY_RUN=1 "$PROVISION" 2>"$TMP/provision.err" \
    || { cat "$TMP/provision.err" >&2; die "provision on bare metal failed"; }
assert_grep "dry-run: kernel args: $ISOLATION $COMMON_ARGS" "$TMP/provision.err"
assert_no_grep "max_cstate" "$TMP/provision.err"
# Unqualified bare metal still keeps the kernel's clocksource: the TSC is
# only forced when the operator says this machine was measured for it.
assert_no_grep "clocksource=" "$TMP/provision.err"
assert_grep "RUSTERON_FORCE_TSC=1 only on bare metal" "$TMP/provision.err"
# The one case that forces it: bare metal, qualified, opt in. The policy is
# recorded, and it says the kernel's TSC checks are off.
fixture_env RUSTERON_NODE_ENV="$TMP/node.env" RUSTERON_FORCE_TSC=1 RUSTERON_DRY_RUN=1 "$PROVISION" 2>"$TMP/provision.err" \
    || { cat "$TMP/provision.err" >&2; die "provision with a forced TSC failed"; }
assert_grep "dry-run: kernel args: $ISOLATION $TSC_ARGS" "$TMP/provision.err"
assert_grep "clocksource forced to tsc (RUSTERON_FORCE_TSC=1): the kernel's TSC checks are off" "$TMP/provision.err"
assert_grep "would write /opt/rusteron-dpdk/clocksource-policy: forced:" "$TMP/provision.err"
echo "$VM_CPUINFO" > "$PROC/cpuinfo"
# The housekeeping CPUs default to cpu0's core, one CPU (its only thread).
fixture_env RUSTERON_DRY_RUN=1 "$PROVISION" 2>"$TMP/provision.err" \
    || { cat "$TMP/provision.err" >&2; die "provision with default housekeeping failed"; }
assert_grep "irqaffinity=0 " "$TMP/provision.err"
expect_fail "provision housekeeping CPU not online" "not online" \
    fixture_env RUSTERON_HOUSEKEEPING_CPUS=8 RUSTERON_DRY_RUN=1 "$PROVISION"
# A failed provision holds kubelet back, so the node never joins with the
# DPDK label and taint.
assert_grep "dry-run: would hold kubelet back" "$TMP/err"
expect_fail "provision too few cores" "physical cores" \
    fixture_env RUSTERON_HOUSEKEEPING_CPUS=0-4 RUSTERON_DRY_RUN=1 "$PROVISION"
# Helpers 0-1 stay in the reserved set (isolated list still starts at 3) and
# IRQs go only to CPU 2.
fixture_env RUSTERON_HOUSEKEEPING_CPUS=0-2 RUSTERON_IRQ_CPUS=2 RUSTERON_DRY_RUN=1 "$PROVISION" 2>"$TMP/provision.err" \
    || { cat "$TMP/provision.err" >&2; die "provision with a separate IRQ destination failed"; }
assert_grep "irqaffinity=2 " "$TMP/provision.err"
assert_grep "isolcpus=nohz,domain,managed_irq,3-7 " "$TMP/provision.err"
expect_fail "provision IRQ CPU outside housekeeping" "not a housekeeping CPU" \
    fixture_env RUSTERON_HOUSEKEEPING_CPUS=0-2 RUSTERON_IRQ_CPUS=5 RUSTERON_DRY_RUN=1 "$PROVISION"
expect_fail "provision dry run on the host's /sys" "RUSTERON_DRY_RUN" \
    fixture_env RUSTERON_SYSFS_ROOT=/sys RUSTERON_DRY_RUN=1 "$PROVISION"
cp "$PROC/cpuinfo" "$TMP/cpuinfo.intel"
# An x86 CPU that is not Intel still fails closed.
printf 'processor\t: 0\nvendor_id\t: AuthenticAMD\nflags\t\t: fpu tsc hypervisor\n' > "$PROC/cpuinfo"
expect_fail "provision on AMD x86" "Intel x86" fixture_env RUSTERON_DRY_RUN=1 "$PROVISION"
assert_grep "dry-run: would hold kubelet back" "$TMP/err"
cp "$TMP/cpuinfo.intel" "$PROC/cpuinfo"
echo "$PROVISIONED_CMDLINE" > "$PROC/cmdline"
fixture_env RUSTERON_DRY_RUN=1 "$PROVISION" 2>"$TMP/provision.err" \
    || { cat "$TMP/provision.err" >&2; die "provision on a provisioned node failed"; }
assert_grep "already provisioned" "$TMP/provision.err"
assert_no_grep "would run" "$TMP/provision.err"
assert_no_grep "hold kubelet" "$TMP/provision.err"
echo "provision assertions PASSED"

# --- 5b. provision turns SMT off (a bare metal first boot) -------------------
# EC2's CpuOptions cover a VM only, so a bare metal node still has SMT when
# this runs. The kernel keeps listing the offline twins in the topology after
# nosmt, so the CPU lists come from one CPU per physical core (the lowest id),
# and naming a twin fails: that CPU will not run.
echo "== provision-node.sh (SMT on: a bare metal node) =="
for cpu in $(seq 0 15); do
    mkdir -p "$SYSFS/devices/system/cpu/cpu$cpu/topology"
    echo "$((cpu % 8)),$((cpu % 8 + 8))" > "$SYSFS/devices/system/cpu/cpu$cpu/topology/thread_siblings_list"
done
echo 0-15 > "$SYSFS/devices/system/cpu/online"
echo 1 > "$SYSFS/devices/system/cpu/smt/active"
echo on > "$SYSFS/devices/system/cpu/smt/control"
echo "BOOT_IMAGE=/boot/vmlinuz ro" > "$PROC/cmdline"
printf 'RUSTERON_DPDK_DEVICE_NUMBERS="1 2"\nRUSTERON_HUGEPAGES=2048\n' > "$TMP/node.env"
fixture_env RUSTERON_NODE_ENV="$TMP/node.env" RUSTERON_DRY_RUN=1 "$PROVISION" 2>"$TMP/smt.err" \
    || { cat "$TMP/smt.err" >&2; die "provision with SMT on failed"; }
cat "$TMP/smt.err"
# The last resort before kubelet's first start, written to the fixture.
assert_file "$SYSFS/devices/system/cpu/smt/control" "off"
assert_grep "SMT turned off" "$TMP/smt.err"
# Housekeeping is cpu0's core, one CPU; the isolated list is the other
# physical cores, never the twins that SMT-off takes down.
assert_grep "dry-run: kernel args: nosmt isolcpus=nohz,domain,managed_irq,1-7 nohz_full=1-7 rcu_nocbs=1-7 irqaffinity=0 " "$TMP/smt.err"
assert_no_grep "irqaffinity=0,8" "$TMP/smt.err"
assert_no_grep "9-15" "$TMP/smt.err"
expect_fail "provision names an SMT twin" "will not run with SMT off" \
    fixture_env RUSTERON_HOUSEKEEPING_CPUS=0,8 RUSTERON_DRY_RUN=1 "$PROVISION"
assert_grep "hold kubelet back" "$TMP/err"
# Back to the launch template's SMT-off world for the rest of the test.
for cpu in $(seq 0 7); do
    echo "$cpu" > "$SYSFS/devices/system/cpu/cpu$cpu/topology/thread_siblings_list"
done
for cpu in $(seq 8 15); do rm -rf "$SYSFS/devices/system/cpu/cpu$cpu"; done
echo 0-7 > "$SYSFS/devices/system/cpu/online"
echo 0 > "$SYSFS/devices/system/cpu/smt/active"
echo on > "$SYSFS/devices/system/cpu/smt/control"
echo "$PROVISIONED_CMDLINE" > "$PROC/cmdline"
echo "SMT transition assertions PASSED"

# --- 5c. provision on Graviton (aarch64) --------------------------------------
# Graviton4 (c8g, Neoverse V2, CPU part 0xd4f) and Graviton5 (c9g, Neoverse
# V3, 0xd84; Linux arch/arm64/include/asm/cputype.h). No SMT, no TSC, no
# hypervisor flag in arm64's cpuinfo. arm64 ignores x86's iommu=pt; Graviton
# metal's SMMU needs iommu.passthrough=1 (benches/cloud/host.sh boot_args).
# A 64 KiB-page kernel (not qualified) fails closed, as does any
# other CPU.
echo "== provision-node.sh (Graviton) =="
graviton_cpuinfo() { # <CPU part> [implementer]
    printf 'processor\t: 0\nBogoMIPS\t: 2000.00\nFeatures\t: fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics sve2\nCPU implementer\t: %s\nCPU architecture: 8\nCPU variant\t: 0x0\nCPU part\t: %s\nCPU revision\t: 1\n' \
        "${2:-0x41}" "$1"
}
GRAVITON_ARGS="isolcpus=nohz,domain,managed_irq,$ISO nohz_full=$ISO rcu_nocbs=$ISO irqaffinity=0 iommu.passthrough=1 transparent_hugepage=never hugepagesz=2M hugepages=2048"
echo "BOOT_IMAGE=/boot/vmlinuz ro" > "$PROC/cmdline"
for part in 0xd4f 0xd84; do
    graviton_cpuinfo "$part" > "$PROC/cpuinfo"
    fixture_env RUSTERON_NODE_ENV="$TMP/node.env" RUSTERON_PAGE_SIZE=4096 RUSTERON_DRY_RUN=1 "$PROVISION" \
        2>"$TMP/graviton.err" || { cat "$TMP/graviton.err" >&2; die "provision on Graviton ($part) failed"; }
    assert_grep "dry-run: kernel args: $GRAVITON_ARGS" "$TMP/graviton.err"
    assert_no_grep "iommu=pt" "$TMP/graviton.err"
    assert_no_grep "nosmt" "$TMP/graviton.err"
    assert_no_grep "clocksource=" "$TMP/graviton.err"
    assert_no_grep "hold kubelet" "$TMP/graviton.err"
    assert_grep "dry-run: would run: systemctl reboot" "$TMP/graviton.err"
done
assert_grep "Graviton5 (Neoverse V3)" "$TMP/graviton.err"
graviton_cpuinfo 0xd4f > "$PROC/cpuinfo"
fixture_env RUSTERON_NODE_ENV="$TMP/node.env" RUSTERON_PAGE_SIZE=4096 RUSTERON_DRY_RUN=1 "$PROVISION" 2>"$TMP/graviton.err" \
    || { cat "$TMP/graviton.err" >&2; die "provision on Graviton4 failed"; }
assert_grep "Graviton4 (Neoverse V2)" "$TMP/graviton.err"
# Graviton3 (0xd40), Graviton2 (0xd0c) and any other Arm core fail closed, and
# so does a V2 part from another implementer.
for part in 0xd40 0xd0c 0xd8e; do
    graviton_cpuinfo "$part" > "$PROC/cpuinfo"
    expect_fail "provision on Arm CPU part $part" "implementer/part 0x41/$part" \
        fixture_env RUSTERON_PAGE_SIZE=4096 RUSTERON_DRY_RUN=1 "$PROVISION"
    assert_grep "dry-run: would hold kubelet back" "$TMP/err"
done
graviton_cpuinfo 0xd4f 0x48 > "$PROC/cpuinfo"
expect_fail "provision on a non-Arm implementer" "implementer/part 0x48/0xd4f" \
    fixture_env RUSTERON_PAGE_SIZE=4096 RUSTERON_DRY_RUN=1 "$PROVISION"
# No CPU part at all (no vendor either): fail closed, never a guess.
printf 'processor\t: 0\nBogoMIPS\t: 2000.00\n' > "$PROC/cpuinfo"
expect_fail "provision on an unknown CPU" "not Intel x86 or Graviton4/5 (vendor '', implementer/part /)" fixture_env RUSTERON_DRY_RUN=1 "$PROVISION"
graviton_cpuinfo 0xd4f > "$PROC/cpuinfo"
# 64 KiB pages: only 4 KiB pages are qualified; fail closed.
expect_fail "provision on 64 KiB pages" "65536-byte pages" \
    fixture_env RUSTERON_PAGE_SIZE=65536 RUSTERON_DRY_RUN=1 "$PROVISION"
assert_grep "dry-run: would hold kubelet back" "$TMP/err"
# 4 KiB pages but no 2 MiB hugepage size: also fail closed.
mv "$SYSFS/kernel/mm/hugepages/hugepages-2048kB" "$TMP/hugepages-2048kB.saved"
expect_fail "provision without 2 MiB hugepages" "need 4 KiB pages and 2 MiB hugepages" \
    fixture_env RUSTERON_PAGE_SIZE=4096 RUSTERON_DRY_RUN=1 "$PROVISION"
mv "$TMP/hugepages-2048kB.saved" "$SYSFS/kernel/mm/hugepages/hugepages-2048kB"
# The page size override is for fixtures only: the dry-run guard already
# refuses the host's /sys, and RUSTERON_FORCE_TSC is x86's.
expect_fail "provision Graviton with a forced TSC" "x86 only" \
    fixture_env RUSTERON_FORCE_TSC=1 RUSTERON_PAGE_SIZE=4096 RUSTERON_DRY_RUN=1 "$PROVISION"
cp "$TMP/cpuinfo.intel" "$PROC/cpuinfo"
echo "$PROVISIONED_CMDLINE" > "$PROC/cmdline"
echo "Graviton provision assertions PASSED"

# --- 6. userdata.sh ----------------------------------------------------------
echo "== userdata.sh =="
# As long as an EKS cluster's CA data (base64 of a PEM certificate, about
# 1.5 KB) and as incompressible: zeros would gzip to nothing and make the size
# check meaningless.
CA_B64="$(python3 -c 'import base64, random; print(base64.b64encode(random.Random(1).randbytes(1100)).decode())')"
"$USERDATA" ergon-lab-test https://ABC.gr7.us-east-1.eks.amazonaws.com "$CA_B64" 172.20.0.0/16 0 > "$TMP/userdata.gz"
size="$(wc -c < "$TMP/userdata.gz" | tr -d ' ')"
echo "user data: $size bytes gzipped"
((size <= 16384)) || die "user data is $size bytes, over EC2's 16384"
gzip -dc "$TMP/userdata.gz" > "$TMP/userdata.mime" || die "user data is not gzip"
for needle in \
    'MIME-Version: 1.0' \
    'Content-Type: multipart/mixed; boundary=' \
    'Content-Type: text/x-shellscript' \
    'Content-Type: application/node.eks.aws' \
    'apiVersion: node.eks.aws/v1alpha1' \
    'kind: NodeConfig' \
    'name: ergon-lab-test' \
    'apiServerEndpoint: https://ABC.gr7.us-east-1.eks.amazonaws.com' \
    "certificateAuthority: $CA_B64" \
    'cidr: 172.20.0.0/16' \
    'cpuManagerPolicy: static' \
    'full-pcpus-only: "true"' \
    'reservedSystemCPUs: "0"' \
    'kubeReservedCgroup: ""' \
    'systemReservedCgroup: ""' \
    'topologyManagerPolicy: single-numa-node' \
    'topologyManagerScope: pod' \
    'memoryManagerPolicy: None' \
    '--node-labels=rusteron-dpdk/ena=true' \
    '--register-with-taints=rusteron-dpdk=true:NoSchedule' \
    'RUSTERON_HOUSEKEEPING_CPUS=0' \
    'RUSTERON_IRQ_CPUS=0' \
    'RUSTERON_DPDK_DEVICE_NUMBERS="1 2"' \
    'RUSTERON_HUGEPAGES=2048' \
    'exec ./provision-node.sh' \
    ; do
    assert_grep "$needle" "$TMP/userdata.mime"
done
# The Memory Manager stays off: every c7i size below metal-48xl has one NUMA
# node, and Static would need reservedMemory to match the reservations exactly.
for needle in 'memoryManagerPolicy: Static' reservedMemory 'kubeReserved:' 'systemReserved:' evictionHard; do
    assert_no_grep "$needle" "$TMP/userdata.mime"
done
# The CPU quota setting stays at kubelet's default (true), so every pod's CPU
# limit holds: kubelet 1.33+ drops the quota itself only for containers with
# exclusive CPUs. `cpuCFSQuota: false` would remove the limit of every pod on
# the node.
for needle in cpuCFSQuota cpu-cfs-quota; do
    assert_no_grep "$needle" "$TMP/userdata.mime"
done
# The MIME document parses into the two parts, and the shell part writes each
# node file as its header and its code: the comment lines in between stay in
# the repo (EC2's 16 KiB limit).
python3 - "$TMP/userdata.mime" "$TMP/part" <<'PY' || die "user data is not a two-part MIME document"
import email, sys
msg = email.message_from_binary_file(open(sys.argv[1], "rb"))
parts = {p.get_content_type(): p.get_payload(decode=True) for p in msg.walk() if not p.is_multipart()}
assert set(parts) == {"text/x-shellscript", "application/node.eks.aws"}, parts.keys()
open(sys.argv[2] + ".sh", "wb").write(parts["text/x-shellscript"])
PY
mkdir -p "$TMP/unpacked"
for f in provision-node.sh bootstrap-dpdk-ena.sh preflight.sh rusteron-dpdk-ena.service \
    irq-affinity.sh rusteron-irq-affinity.service; do
    sed -n "/^cat > $f <<'RUSTERON_NODE_FILE_EOF'\$/,/^RUSTERON_NODE_FILE_EOF\$/p" "$TMP/part.sh" | sed '1d;$d' > "$TMP/unpacked/$f"
    python3 - "$NODE_DIR/$f" "$TMP/unpacked/$f" <<'PY' || die "embedded $f is not the repo's less its comment lines"
import re, sys
src = open(sys.argv[1]).read().splitlines(keepends=True)
head = 1 if src and src[0].startswith("#!") else 0
want = src[:head] + [line for line in src[head:] if not re.match(r"\s*#", line)]
assert "".join(want) == open(sys.argv[2]).read()
PY
done
# The embedded scripts work as the repo's do: a bootstrap dry run from them.
chmod +x "$TMP/unpacked/"*.sh
fixture_env RUSTERON_DRY_RUN=1 "$TMP/unpacked/bootstrap-dpdk-ena.sh" 2>"$TMP/unpacked.err" \
    || { cat "$TMP/unpacked.err" >&2; die "the embedded bootstrap failed its dry run"; }
assert_grep "wrote $INVENTORY atomically" "$TMP/unpacked.err"
expect_fail "userdata bad reserved CPUs" "reserved" \
    "$USERDATA" ergon-lab-test https://x "$CA_B64" 172.20.0.0/16 "0;8"
expect_fail "userdata bad IRQ CPUs" "IRQ" \
    "$USERDATA" ergon-lab-test https://x "$CA_B64" 172.20.0.0/16 0-2 "2;8"
expect_fail "userdata missing arguments" "usage" "$USERDATA" ergon-lab-test
# Optional sixth argument: housekeeping 0-2, interrupts only on CPU 2.
"$USERDATA" ergon-lab-test https://ABC.gr7.us-east-1.eks.amazonaws.com "$CA_B64" 172.20.0.0/16 0-2 2 > "$TMP/userdata-irq.gz"
gzip -dc "$TMP/userdata-irq.gz" > "$TMP/userdata-irq.mime" || die "IRQ user data is not gzip"
assert_grep 'RUSTERON_HOUSEKEEPING_CPUS=0-2' "$TMP/userdata-irq.mime"
assert_grep 'RUSTERON_IRQ_CPUS=2' "$TMP/userdata-irq.mime"
assert_grep 'reservedSystemCPUs: "0-2"' "$TMP/userdata-irq.mime"
irq_size="$(wc -c < "$TMP/userdata-irq.gz" | tr -d ' ')"
echo "user data with an IRQ destination: $irq_size bytes gzipped"
((irq_size <= 16384)) || die "IRQ user data is $irq_size bytes, over EC2's 16384"
echo "userdata assertions PASSED"

# --- 6b. launch-template.py --------------------------------------------------
# CpuOptions ThreadsPerCore=1 (with the CoreCount EC2 requires alongside it)
# turns SMT off on an x86 latency node group; Graviton and bare metal get none.
echo "== launch-template.py =="
LT="$NODE_DIR/launch-template.py"
printf 'user data bytes' > "$TMP/lt-userdata"
lt_json() { # out instance-type vcpus cores threads-per-core arch
    python3 "$LT" ami-0123456789abcdef0 "$2" arn:aws:iam::123456789012:instance-profile/ergon-lab \
        "$TMP/lt-userdata" 20261017 ergon-lab-test "$3" "$4" "$5" "$6" > "$1"
}
lt_json "$TMP/lt.json" c7i.4xlarge 16 8 2 x86_64
python3 - "$TMP/lt.json" <<'PY' || die "launch-template c7i.4xlarge"
import base64, json, sys
data = json.load(open(sys.argv[1]))
assert data["CpuOptions"] == {"CoreCount": 8, "ThreadsPerCore": 1}, data.get("CpuOptions")
assert data["InstanceType"] == "c7i.4xlarge"
assert base64.b64decode(data["UserData"]) == b"user data bytes"
assert any(t["ResourceType"] == "instance" for t in data["TagSpecifications"])
PY
# Graviton is already one thread per core, and EC2 takes no CPU options there.
lt_json "$TMP/lt.json" c8g.4xlarge 16 16 1 arm64
python3 - "$TMP/lt.json" <<'PY' || die "launch-template c8g.4xlarge"
import json, sys
data = json.load(open(sys.argv[1]))
assert "CpuOptions" not in data, data["CpuOptions"]
PY
# Bare metal takes no CPU options at all (provision-node.sh turns SMT off).
lt_json "$TMP/lt.json" c7i.metal-24xl 96 48 2 x86_64
python3 - "$TMP/lt.json" <<'PY' || die "launch-template c7i.metal-24xl"
import json, sys
data = json.load(open(sys.argv[1]))
assert "CpuOptions" not in data, data["CpuOptions"]
PY
# C7a (like M7a and R7a) already runs one thread per core and takes no change.
lt_json "$TMP/lt.json" c7a.4xlarge 8 8 1 x86_64
python3 - "$TMP/lt.json" <<'PY' || die "launch-template c7a.4xlarge"
import json, sys
data = json.load(open(sys.argv[1]))
assert "CpuOptions" not in data, data["CpuOptions"]
PY
expect_fail "launch-template cores over vCPUs" "over" \
    python3 "$LT" ami-1 c7i.4xlarge arn:x "$TMP/lt-userdata" s n 16 16 2 x86_64
expect_fail "launch-template missing arguments" "usage" \
    python3 "$LT" ami-1 c7i.4xlarge
echo "launch-template assertions PASSED"

# --- 7. entrypoint CPU layout ------------------------------------------------
echo "== entrypoint (cpuset 1-4,9-12 on 16 CPUs) =="
echo "1-4,9-12" > "$TMP/cpuset"
for cpu in $(seq 0 15); do
    printf 'processor\t: %d\nphysical id\t: 0\ncore id\t\t: %d\n\n' "$cpu" "$((cpu % 8))"
done > "$TMP/cpuinfo"
RUSTERON_CPUSET_FILE="$TMP/cpuset" RUSTERON_CPUINFO="$TMP/cpuinfo" RUSTERON_ENTRYPOINT_PRINT_ONLY=1 \
    HOSTNAME=test bash "$ENTRYPOINT" true > "$TMP/entrypoint.env" \
    || die "entrypoint failed"
cat "$TMP/entrypoint.env"
for line in \
    'AERON_CONDUCTOR_CPU_AFFINITY=1' \
    'AERON_SENDER_CPU_AFFINITY=2' \
    'AERON_RECEIVER_CPU_AFFINITY=3' \
    'RUSTERON_DPDK_EAL_ARGS=-l 1' \
    'AERON_SENDER_WILDCARD_PORT_RANGE=20000 20999' \
    'AERON_RECEIVER_WILDCARD_PORT_RANGE=21000 21999' \
    ; do
    grep -qxF -- "$line" "$TMP/entrypoint.env" || die "entrypoint env lacks '$line'"
done
! grep -qx 'AERON_CONDUCTOR_IDLE_STRATEGY=spin' "$TMP/entrypoint.env" || die "entrypoint spins the conductor"

# In a privileged pod /sys/fs/cgroup is the host's root, so the entrypoint
# takes the pod's cpuset from Cpus_allowed_list in /proc/self/status, never
# from cpuset.cpus.effective.
echo "== entrypoint (privileged: allowed list 1-4,9-12, host cgroup root) =="
printf 'Name:\tbash\nCpus_allowed:\t1e1e\nCpus_allowed_list:\t1-4,9-12\n' > "$TMP/status"
RUSTERON_PROC_STATUS="$TMP/status" RUSTERON_CPUINFO="$TMP/cpuinfo" RUSTERON_ENTRYPOINT_PRINT_ONLY=1 \
    HOSTNAME=test bash "$ENTRYPOINT" true > "$TMP/entrypoint.env" \
    || die "entrypoint failed"
cat "$TMP/entrypoint.env"
for line in \
    'AERON_CONDUCTOR_CPU_AFFINITY=1' \
    'AERON_SENDER_CPU_AFFINITY=2' \
    'AERON_RECEIVER_CPU_AFFINITY=3' \
    'RUSTERON_DPDK_HOUSEKEEPING_CPU=1' \
    'RUSTERON_DPDK_EAL_ARGS=-l 1' \
    ; do
    grep -qxF -- "$line" "$TMP/entrypoint.env" || die "entrypoint env lacks '$line'"
done
grep -q 'cpuset=1-4,9-12 ' "$TMP/entrypoint.env" || die "entrypoint did not report the allowed list as its cpuset"
! grep -v '^[[:space:]]*#' "$ENTRYPOINT" | grep -q '/sys/fs/cgroup' || die "entrypoint still reads the cgroup cpuset file"
echo "entrypoint assertions PASSED"

echo "ALL TESTS PASSED"
