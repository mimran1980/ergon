#!/usr/bin/env bash
# Pins movable IRQs on RUSTERON_IRQ_CPUS (default RUSTERON_HOUSEKEEPING_CPUS),
# off helper CPUs and off isolated CPUs where the DPDK agents busy-spin.
# usage: irq-affinity.sh pin|watch
set -euo pipefail
# pin: one pass moves every IRQ whose affinity reaches past the IRQ
# destination onto it, then reports each IRQ whose effective affinity still
# includes an isolated CPU. Exits 1 on one that can move but whose requested
# affinity is not the destination after the pass. One the kernel refuses to
# move (a managed IRQ, such as an NVMe queue's, or a per-CPU one) is the
# kernel's to keep, and one it moves only at its next interrupt (an EC2 VM:
# no interrupt remapping) is on its way: both only reported.
# bootstrap-dpdk-ena.sh runs it at every boot; run it again after handing a
# data ENA back to the kernel's ena driver.
# watch: a pass, then one after every network link event, until stopped
# (rusteron-irq-affinity.service). It sleeps in the kernel between events.
# Fixtures: RUSTERON_PROC_ROOT, RUSTERON_SYSFS_ROOT, RUSTERON_LINK_MONITOR.
# See node/test/test-bootstrap.sh.

# Why a link monitor. The kernel's ENA driver sets each queue IRQ's affinity
# to its hint (queue i on CPU i) whenever it brings an interface up: after a
# device reset (its own recovery included), a queue or ring change
# (ethtool -L, -G), an ifdown/ifup, and a rebind to the kernel driver (a
# rollback, a kernel baseline); an MTU or coalescing change does not.
# irqaffinity= and the boot-time pinning cannot stop that, so the interrupts
# land on the isolated CPUs until something re-pins them.
# Every one of those paths ends with the interface's carrier or flags
# changing, which the kernel announces on netlink at once (a carrier change
# at most a second later), so a pass on each link event leaves a hint in
# place for milliseconds; a VPC CNI ENI attached later is caught the same way.
# On a VM the hint itself waits for the queue's next interrupt (see pass()),
# so a pass that comes first leaves the IRQ where it was; one that comes
# after costs that isolated CPU the interrupt in which the kernel moves it
# (two, if the hint's own move had not yet seen one there) and the old
# vector's clean-up (an IPI on 6.1, a timer on 6.12), as any move off an
# isolated CPU does.
# Weighed and set aside:
#   * irqbalance with the isolated CPUs banned (IRQBALANCE_BANNED_CPULIST,
#     which it also derives from isolcpus by itself; AWS's ENA guide prefers
#     it running): it rewrites an IRQ's affinity only after moving that IRQ
#     itself, so an interface brought up again in place keeps its hints until
#     irqbalance next rebalances those IRQs, and it sees a reset's new
#     vectors only at its next scan (10 s). It also moves IRQs on load, so
#     irqbalance stays off and this is the one writer of IRQ affinity.
#   * A udev rule: udev sees a device added, not an interface brought up
#     again after a reset or a ring change.
#   * Fewer queues on the primary ENA (ethtool -L combined 1), so its only
#     hint is CPU 0: right only while CPU 0 is a housekeeping CPU, undone by
#     the next ethtool -L, and no help for a data ENA handed back to the
#     kernel (a fresh probe, every queue) or an ENI attached later.
# The watcher runs where systemd starts it: on the housekeeping CPUs, since
# isolcpus=domain keeps every process off the isolated ones.

RUSTERON_SYSFS_ROOT="${RUSTERON_SYSFS_ROOT:-/sys}"
RUSTERON_PROC_ROOT="${RUSTERON_PROC_ROOT:-/proc}"
RUSTERON_HOUSEKEEPING_CPUS="${RUSTERON_HOUSEKEEPING_CPUS:-}"
RUSTERON_IRQ_CPUS="${RUSTERON_IRQ_CPUS:-}"
RUSTERON_LINK_MONITOR="${RUSTERON_LINK_MONITOR:-ip -o monitor link}"

fail() { echo "irq-affinity FAIL: $*" >&2; exit 1; }
ok()   { echo "irq-affinity ok: $*" >&2; }
warn() { echo "irq-affinity warn: $*" >&2; }

mode="${1:-}"
if [[ $# != 1 || ("$mode" != pin && "$mode" != watch) ]]; then
    echo "usage: $0 pin|watch" >&2
    exit 2
fi

if [[ -z "$RUSTERON_HOUSEKEEPING_CPUS" ]]; then
    RUSTERON_HOUSEKEEPING_CPUS="$(cat "$RUSTERON_SYSFS_ROOT/devices/system/cpu/cpu0/topology/thread_siblings_list" 2>/dev/null || true)"
fi
HK="$RUSTERON_HOUSEKEEPING_CPUS"
CPULIST='^[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*$'
[[ "$HK" =~ $CPULIST ]] || fail "RUSTERON_HOUSEKEEPING_CPUS='$HK': a CPU list such as 0,8"

# hk[c] is set for each housekeeping CPU c. A pass runs in bash alone, with
# no command forked, however many IRQs there are.
hk=()
IFS=, read -r -a parts <<<"$HK"
for part in "${parts[@]}"; do
    for ((c = ${part%-*}; c <= ${part#*-}; c++)); do hk[c]=1; done
done

# The IRQ destination defaults to the housekeeping list, so a one-CPU node
# keeps today's behaviour. A helper CPU is in housekeeping and not here.
DEST="${RUSTERON_IRQ_CPUS:-$HK}"
[[ "$DEST" =~ $CPULIST ]] || fail "RUSTERON_IRQ_CPUS='$DEST': a CPU list such as 0"
dest=()
IFS=, read -r -a parts <<<"$DEST"
for part in "${parts[@]}"; do
    for ((c = ${part%-*}; c <= ${part#*-}; c++)); do
        [[ -n "${hk[c]:-}" ]] || fail "IRQ CPU $c is not a housekeeping CPU ($HK)"
        dest[c]=1
    done
done

# 0 when every CPU in a list is a housekeeping CPU; an empty list too (an IRQ
# with no effective CPU fires nowhere).
on_hk() {
    local part c IFS=,
    [[ -n "$1" ]] || return 0
    [[ "$1" =~ $CPULIST ]] || return 1
    for part in $1; do
        for ((c = ${part%-*}; c <= ${part#*-}; c++)); do
            [[ -n "${hk[c]:-}" ]] || return 1
        done
    done
}

# 0 when every CPU in a list is an IRQ-destination CPU. Same empty-list rule.
on_dest() {
    local part c IFS=,
    [[ -n "$1" ]] || return 0
    [[ "$1" =~ $CPULIST ]] || return 1
    for part in $1; do
        for ((c = ${part%-*}; c <= ${part#*-}; c++)); do
            [[ -n "${dest[c]:-}" ]] || return 1
        done
    done
}

# Sets label to an IRQ's handlers, empty for none: /proc/irq/<n>/ holds a
# directory for each, named as its driver asked (eth0-Tx-Rx-3, nvme0q6).
name_irq() {
    local a
    label=""
    for a in "$1"*/; do
        [[ -d "$a" ]] || continue
        a="${a%/}"
        label+="${label:+ }${a##*/}"
    done
}

# One pass. Moves every IRQ whose requested affinity reaches past the
# housekeeping CPUs (a write the kernel refuses marks one it keeps), then
# collects each IRQ with a handler whose effective affinity (the requested
# one on a kernel without it) still includes an isolated CPU: kept, if the
# kernel refused to move it; moving, if its requested affinity now reads the
# housekeeping CPUs; else stray. One without a handler (a vector allocated
# but never requested, which x86 parks on the first online CPU) fires
# nowhere.
# Moving is the rule on an EC2 VM: x86 moves a PCI MSI in process context
# only with interrupt remapping (IRQD_MOVE_PCNTXT), which a VM lacks, so the
# kernel takes the write as pending (irq_set_affinity_locked), shows the
# pending mask as smp_affinity_list, and carries out the move in the IRQ's
# next interrupt, on its old CPU. With remapping (bare metal) it moves at
# once, unless the old vector of the hint's own move is still being cleaned
# up (-EBUSY, a jiffy at most), which defers it the same way. Nothing short
# of that interrupt moves it, so it is reported, not failed: an idle queue's
# IRQ can wait for minutes.
pass() {
    local d f irq cur eff
    moved=() refused=() kept=() moving=() stray=()
    for d in "$RUSTERON_PROC_ROOT"/irq/[0-9]*/; do
        [[ -d "$d" ]] || continue
        f="${d}smp_affinity_list"
        cur=""
        if { read -r cur < "$f"; } 2>/dev/null && on_dest "$cur"; then continue; fi
        irq="${d%/}" && irq="${irq##*/}"
        if { printf '%s\n' "$DEST" > "$f"; } 2>/dev/null; then
            moved+=("$irq")
            if [[ "$mode" == watch ]]; then
                name_irq "$d"
                ok "moved IRQ $irq (${label:-no handler}) from CPUs ${cur:-unknown} to $DEST"
            fi
        else
            refused+=("$irq")
        fi
    done
    for d in "$RUSTERON_PROC_ROOT"/irq/[0-9]*/; do
        [[ -d "$d" ]] || continue
        f="${d}effective_affinity_list"
        [[ -e "$f" ]] || f="${d}smp_affinity_list"
        { read -r eff < "$f"; } 2>/dev/null || continue
        on_dest "$eff" && continue
        name_irq "$d"
        [[ -n "$label" ]] || continue
        irq="${d%/}" && irq="${irq##*/}"
        # An empty or unreadable requested affinity places nothing: stray.
        # An IRQ the kernel will not move off a helper CPU (inside
        # housekeeping, outside the destination) fails pin: leaving it would
        # let interrupts run on a shared CPU.
        cur=""
        { read -r cur < "${d}smp_affinity_list"; } 2>/dev/null || true
        if [[ " ${refused[*]} " == *" $irq "* ]] && on_hk "$eff" && ! on_dest "$eff"; then
            stray+=("IRQ $irq ($label) on CPUs $eff, outside the IRQ destination $DEST")
        elif [[ " ${refused[*]} " == *" $irq "* ]]; then
            kept+=("IRQ $irq ($label) on isolated CPUs $eff")
        elif [[ -n "$cur" ]] && on_dest "$cur"; then
            moving+=("IRQ $irq ($label) on isolated CPUs $eff")
        else
            stray+=("IRQ $irq ($label) on CPUs $eff")
        fi
    done
}

# pin: the summary, each IRQ the kernel keeps on an isolated CPU or moves at
# its next interrupt, and a failure for any other still there. watch: only
# what is wrong after the pass; the kept ones were reported at boot, and a
# moving one is the pass's own write taking effect.
report() {
    local k list
    if [[ "$mode" == watch ]]; then
        for k in "${stray[@]}"; do warn "still on an isolated CPU after the pass: $k"; done
        return 0
    fi
    for k in "${kept[@]}"; do warn "$k: the kernel refuses to move it (a managed or per-CPU IRQ)"; done
    for k in "${moving[@]}"; do warn "$k until its next interrupt, in which the kernel moves it to $DEST"; done
    if ((${#stray[@]})); then
        printf -v list '%s; ' "${stray[@]}"
        fail "IRQs still on isolated CPUs after the pass: ${list%; }"
    fi
    ok "IRQs on CPUs $DEST (${#moved[@]} moved, ${#refused[@]} refused by the kernel)"
}

# The monitor subscribes before the first pass (it has a second to print its
# first event, which the pass then covers), so no change between the two is
# missed. One pass covers a burst: the events queued while the last one ran
# are read first. The monitor's errors are lines too, so a netlink overflow,
# which loses events, still brings a pass.
watch_links() {
    local mon line last=""
    # shellcheck disable=SC2086 # the command is words: "ip -o monitor link"
    exec {mon}< <(exec $RUSTERON_LINK_MONITOR 2>&1)
    if read -r -t 1 -u "$mon" line; then last="$line"; fi
    pass
    report
    while read -r -u "$mon" line; do
        last="$line"
        while read -r -t 0 -u "$mon" && read -r -u "$mon" line; do last="$line"; done
        pass
        report
    done
    fail "the link monitor ($RUSTERON_LINK_MONITOR) ended${last:+, last saying: $last}; systemd restarts the watcher"
}

case "$mode" in
pin)
    pass
    report
    ;;
watch) watch_links ;;
esac
