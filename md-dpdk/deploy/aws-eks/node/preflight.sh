#!/usr/bin/env bash
# Node preflight for the rusteron-media-driver DPDK ENA workload (plan sections
# 9 and 10.2). Verifies the host is safe to hand two secondary ENAs to DPDK:
#
#   * The DPDK driver (RUSTERON_DPDK_DRIVER) can map the ENA's low-latency
#     queues with write-combining, which Amazon's DPDK ENA guide makes
#     mandatory for LLQ:
#       - igb_uio (default): the module is loaded (bootstrap-dpdk-ena.sh
#         insmods it with wc_activate=1) and nothing translates its DMA, since
#         igb_uio programs no IOMMU: no IOMMU groups (virtualised EC2), or
#         every group in passthrough (`identity`, iommu=pt: *.metal, where
#         provision-node.sh sets it).
#       - vfio-pci (*.metal with the IOMMU on): real IOMMU groups,
#         vfio_iommu_type1 (never unsafe no-IOMMU mode), and
#         RUSTERON_DPDK_VFIO_WC=1 to confirm the vfio-pci in use maps the
#         prefetchable BAR write-combined, which mainline vfio-pci does not.
#         The driver pod's ConfigMap carries the same confirmation to the
#         transport, whose LLQ start check cannot see a vfio-pci mapping.
#   * 2 MiB hugepages are reserved and mounted at /dev/hugepages.
#   * NUMA topology is present (hugepage/node locality matters on Nitro).
#   * At least three ENA PCI functions exist (primary, sender, receiver),
#     counted by PCI class, so a rerun after binding still sees them.
#   * The sender and receiver are the ENIs at the IMDS device numbers in
#     RUSTERON_DPDK_DEVICE_NUMBERS (default "1 2", sender first), never the
#     primary and never an ENI the VPC CNI attached.
#   * IMDS returns a parseable IP/subnet identity for each of the two.
#   * (soft) chronyd is active, so both nodes' clocks agree.
#
# Sourced by bootstrap-dpdk-ena.sh, which then uses primary_mac, SENDER_MAC
# and RECEIVER_MAC. Testable against a fake fixture root: set
# RUSTERON_SYSFS_ROOT and RUSTERON_IMDS_ROOT to the fixture paths (see
# node/test/test-bootstrap.sh).
#
# Exit 0 = preflight OK. Exit 1 = unsafe/incomplete, with the failing check
# named. Accepts no arguments.
set -euo pipefail

RUSTERON_SYSFS_ROOT="${RUSTERON_SYSFS_ROOT:-/sys}"
RUSTERON_IMDS_ROOT="${RUSTERON_IMDS_ROOT:-http://169.254.169.254/latest/meta-data}"
RUSTERON_HUGEPAGES_DIR="${RUSTERON_HUGEPAGES_DIR:-/dev/hugepages}"
RUSTERON_DPDK_DRIVER="${RUSTERON_DPDK_DRIVER:-igb_uio}"
RUSTERON_DPDK_VFIO_WC="${RUSTERON_DPDK_VFIO_WC:-0}"
RUSTERON_DPDK_DEVICE_NUMBERS="${RUSTERON_DPDK_DEVICE_NUMBERS:-1 2}"
# Fixture-test overrides: a known primary MAC and a non-mount hugepage root
# (see node/test/test-bootstrap.sh). Absent in production.
RUSTERON_PRIMARY_MAC="${RUSTERON_PRIMARY_MAC:-}"
RUSTERON_SKIP_MOUNT_CHECK="${RUSTERON_SKIP_MOUNT_CHECK:-0}"

fail() { echo "preflight FAIL: $*" >&2; exit 1; }
ok()   { echo "preflight ok: $*" >&2; }
warn() { echo "preflight warn: $*" >&2; }

# IMDSv2-aware fetch; a non-http root is treated as a local fixture path.
imds_get() {
    local path="$1"
    if [[ "$RUSTERON_IMDS_ROOT" == http* ]]; then
        local token
        token="$(curl -fsS -X PUT "$RUSTERON_IMDS_ROOT/../api/token" \
            -H "X-aws-ec2-metadata-token-ttl-seconds: 21600" 2>/dev/null || true)"
        if [[ -n "$token" ]]; then
            curl -fsS -H "X-aws-ec2-metadata-token: $token" \
                "$RUSTERON_IMDS_ROOT/$path" 2>/dev/null || true
        else
            curl -fsS "$RUSTERON_IMDS_ROOT/$path" 2>/dev/null || true
        fi
    else
        # Local fixture path: a directory lists its children (like the IMDS
        # API's `macs/` listing); a file cats its contents.
        if [[ -d "$RUSTERON_IMDS_ROOT/$path" ]]; then
            ls -1 "$RUSTERON_IMDS_ROOT/$path" 2>/dev/null | tr '\n' ' '
        else
            cat "$RUSTERON_IMDS_ROOT/$path" 2>/dev/null || true
        fi
    fi
}

# --- 1. DPDK driver and write-combining ------------------------------------
iommu_groups="$RUSTERON_SYSFS_ROOT/kernel/iommu_groups"
# A kernel without an IOMMU may have no iommu_groups directory: no groups.
# (Under pipefail a find on a missing directory would end the script silently.)
n_groups=0
if [[ -d "$iommu_groups" ]]; then
    n_groups="$(find "$iommu_groups" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')"
fi
case "$RUSTERON_DPDK_DRIVER" in
igb_uio)
    # Bare metal has a real IOMMU. igb_uio maps nothing through it, so the
    # ENAs' DMA works only in passthrough: every group's default domain
    # `identity`, which iommu=pt gives (a group without a type is not).
    if ((n_groups > 0)); then
        translated=()
        for g in "$iommu_groups"/*/; do
            t="$(cat "${g}type" 2>/dev/null || echo unknown)"
            [[ "$t" == identity ]] || translated+=("$(basename "$g"):$t")
        done
        ((${#translated[@]} == 0)) \
            || fail "igb_uio programs no IOMMU, so every IOMMU group must be in passthrough (iommu=pt on the kernel command line, which provision-node.sh sets): ${#translated[@]} of $n_groups group(s) at $iommu_groups translate (${translated[*]:0:4})"
    fi
    [[ -d "$RUSTERON_SYSFS_ROOT/module/igb_uio" ]] \
        || fail "igb_uio not loaded (bootstrap-dpdk-ena.sh insmods it with wc_activate=1)"
    if ((n_groups > 0)); then
        ok "igb_uio loaded; IOMMU in passthrough ($n_groups group(s), all identity)"
    else
        ok "igb_uio loaded and no IOMMU groups"
    fi
    ;;
vfio-pci)
    [[ "$RUSTERON_DPDK_VFIO_WC" == 1 ]] \
        || fail "vfio-pci maps no write-combining in mainline Linux, and ENA LLQ needs it: set RUSTERON_DPDK_VFIO_WC=1 only for a vfio-pci with Amazon's write-combining patch"
    ((n_groups > 0)) \
        || fail "IOMMU groups missing/empty at $iommu_groups: vfio-pci needs the IOMMU on (a *.metal instance with intel_iommu=on iommu=pt)"
    ok "IOMMU groups present ($n_groups group(s))"
    [[ -d "$RUSTERON_SYSFS_ROOT/bus/pci/drivers/vfio-pci" ]] \
        || fail "vfio-pci driver not loaded (modprobe vfio-pci)"
    # Without vfio_iommu_type1, vfio falls back to unsafe no-IOMMU mode,
    # which must never back a kernel-bypass transport.
    [[ -d "$RUSTERON_SYSFS_ROOT/module/vfio_iommu_type1" ]] \
        || fail "vfio_iommu_type1 not loaded: vfio would run in unsafe no-IOMMU mode (modprobe vfio_iommu_type1)"
    ok "vfio-pci with vfio_iommu_type1 (real IOMMU backing) and write-combining"
    ;;
*)
    fail "RUSTERON_DPDK_DRIVER=$RUSTERON_DPDK_DRIVER: igb_uio or vfio-pci"
    ;;
esac

# --- 2. Hugepages -----------------------------------------------------------
huge_nr="$RUSTERON_SYSFS_ROOT/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages"
nr="$(cat "$huge_nr" 2>/dev/null || echo 0)"
((nr > 0)) || fail "no 2 MiB hugepages reserved (hugepages= on the kernel command line)"
if ! ((RUSTERON_SKIP_MOUNT_CHECK)) && ! mountpoint -q "$RUSTERON_HUGEPAGES_DIR" 2>/dev/null; then
    fail "$RUSTERON_HUGEPAGES_DIR is not a hugetlbfs mount (mount -t hugetlbfs -o pagesize=2M)"
fi
ok "2 MiB hugepages reserved ($nr) and mounted at $RUSTERON_HUGEPAGES_DIR"

# --- 3. NUMA ----------------------------------------------------------------
[[ -d "$RUSTERON_SYSFS_ROOT/devices/system/node/node0" ]] \
    || fail "NUMA topology not present: DPDK needs node-local hugepages/mempools"
ok "NUMA topology present"

# --- 4. ENA PCI functions ---------------------------------------------------
# Amazon's vendor id (0x1d0f) with the Ethernet class (0x0200xx): EBS NVMe
# controllers share the vendor id, and a bound ENA has no netdev, so neither
# vendor alone nor netdevs count ENAs.
n_ena=0
for dev in "$RUSTERON_SYSFS_ROOT"/bus/pci/devices/*/; do
    [[ -d "$dev" ]] || continue
    [[ "$(cat "${dev}vendor" 2>/dev/null || true)" == "0x1d0f" ]] || continue
    [[ "$(cat "${dev}class" 2>/dev/null || true)" == 0x0200* ]] || continue
    n_ena=$((n_ena + 1))
done
((n_ena >= 3)) || fail "expected >= 3 ENA PCI functions (primary + sender + receiver), found $n_ena"
ok "$n_ena ENA PCI function(s)"

# --- 5. Primary ENA and the sender/receiver by device number ----------------
# The primary ENA carries EKS, DNS, image pulls, telemetry and the kernel-UDP
# rollback path, so it is never a DPDK candidate. Overridable for fixtures.
primary_mac="$RUSTERON_PRIMARY_MAC"
if [[ -z "$primary_mac" ]]; then
    ifname="$(ip route show default 2>/dev/null | awk '{print $5; exit}' || true)"
    [[ -z "$ifname" ]] || primary_mac="$(cat "$RUSTERON_SYSFS_ROOT/class/net/$ifname/address" 2>/dev/null || true)"
fi
[[ -n "$primary_mac" ]] || fail "cannot identify the primary (default-route) ENA"

read -r -a device_numbers <<<"$RUSTERON_DPDK_DEVICE_NUMBERS"
if ((${#device_numbers[@]} != 2)) \
    || [[ ! "${device_numbers[0]}" =~ ^[1-9][0-9]*$ || ! "${device_numbers[1]}" =~ ^[1-9][0-9]*$ ]] \
    || [[ "${device_numbers[0]}" == "${device_numbers[1]}" ]]; then
    fail "RUSTERON_DPDK_DEVICE_NUMBERS='$RUSTERON_DPDK_DEVICE_NUMBERS': two distinct secondary device numbers (>= 1), sender first"
fi

imds_macs="$(imds_get network/interfaces/macs/)"
[[ -n "$imds_macs" ]] || fail "IMDS returned no network interfaces"
# "<device-number> <mac>" per ENI. Selection is by device number only: the
# VPC CNI attaches its own ENIs at other device numbers, whatever their MACs.
mac_devnums=""
for mac in $imds_macs; do
    mac="${mac%/}"
    [[ -n "$mac" ]] || continue
    mac_devnums+="$(imds_get "network/interfaces/macs/$mac/device-number") $mac"$'\n'
done

# The single ENI at one device number, else fail.
mac_at_device() {
    local want="$1" devnum mac found=""
    while read -r devnum mac; do
        [[ "$devnum" == "$want" ]] || continue
        [[ -z "$found" ]] || fail "IMDS lists two ENIs at device-number $want ($found, $mac)"
        found="$mac"
    done <<<"$mac_devnums"
    [[ -n "$found" ]] || fail "IMDS lists no ENI at device-number $want (RUSTERON_DPDK_DEVICE_NUMBERS='$RUSTERON_DPDK_DEVICE_NUMBERS')"
    [[ "$found" != "$primary_mac" ]] || fail "device-number $want is the primary ENA $primary_mac"
    echo "$found"
}
SENDER_MAC="$(mac_at_device "${device_numbers[0]}")"
RECEIVER_MAC="$(mac_at_device "${device_numbers[1]}")"
ok "primary ENA $primary_mac preserved; sender=$SENDER_MAC (device ${device_numbers[0]}) receiver=$RECEIVER_MAC (device ${device_numbers[1]})"

# --- 6. IP/subnet identity --------------------------------------------------
for mac in "$SENDER_MAC" "$RECEIVER_MAC"; do
    ip="$(imds_get "network/interfaces/macs/$mac/local-ipv4s" | head -1)"
    cidr="$(imds_get "network/interfaces/macs/$mac/subnet-ipv4-cidr-block")"
    eni="$(imds_get "network/interfaces/macs/$mac/interface-id")"
    [[ -n "$ip" && -n "$cidr" && -n "$eni" ]] \
        || fail "ENI $mac missing IMDS identity (eni/ip/cidr)"
    ok "ENI $mac -> eni=$eni ip=$ip cidr=$cidr"
done

# --- 7. Clock discipline (soft) ---------------------------------------------
# The transport timestamps in software unless asked for NIC RX timestamps, so
# a disciplined host clock (chrony) keeps the two nodes' offsets comparable.
# Not fatal: the transport works regardless.
if command -v systemctl >/dev/null 2>&1 && ! systemctl is-active --quiet chronyd 2>/dev/null; then
    warn "chronyd not active: the two nodes' clocks are undisciplined (runbook section 2)"
fi

echo "preflight PASS" >&2
