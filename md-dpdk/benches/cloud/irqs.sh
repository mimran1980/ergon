# Sourced by vm.sh: every IRQ the kernel lets move kept on the housekeeping
# CPU, and the data NICs' interrupts counted wherever else they land. Reads
# and writes $LAB_PROC_ROOT (/proc; test/test-lab.sh gives it a fake tree).
#
# `tune` pinning them once a boot is not enough: the kernel's ENA driver sets
# each queue IRQ's affinity again (its hint, one queue a CPU) whenever it
# brings an interface up (or resets it: a ring change, its own recovery), so
# once the data NICs came back from a DPDK mode, or a ring change reset them,
# the kernel modes took their interrupts on the isolated CPUs (an early lab
# run's trace; its output is not committed). vm.sh pins again after each of those
# and just before each driver starts; a reset mid-run is not undone until
# then, and `loss` counts what landed elsewhere (isolated-nic-irqs).
LAB_PROC_ROOT=${LAB_PROC_ROOT:-/proc}
LAB_SYS_ROOT=${LAB_SYS_ROOT:-/sys}

# irqs_pin <housekeeping CPU> <when>: irqbalance stopped, every IRQ's
# affinity and new IRQs' default the housekeeping CPU (the first core's, so
# its mask is one word), then irqs_check's line, "irqs <when>: ...", on
# stderr. An IRQ that refuses (per-CPU, or managed) is the kernel's to keep:
# irqs_check lists it.
irqs_pin() {
    local hk=$1 when=$2
    sudo systemctl stop irqbalance 2>/dev/null || true
    # one sudo for them all
    sudo bash -c 'for f in "$2"/irq/[0-9]*/smp_affinity_list; do echo "$1" >"$f"; done
        printf "%x\n" $((1 << $1)) >"$2/irq/default_smp_affinity"' _ "$hk" "$LAB_PROC_ROOT" 2>/dev/null || true
    echo "irqs $when: $(irqs_check "$hk")" >&2
}

# irqs_check <housekeeping CPU>: how many of the IRQs with a handler (those
# in /proc/interrupts) have the housekeeping CPU alone, how many not, each of
# those as <irq>:<name>@<CPUs>, and the default mask for new IRQs. An IRQ's
# CPUs are the kernel's effective ones (effective_affinity_list), or its
# affinity where a kernel keeps no effective list or an IRQ not started has
# it empty. Managed IRQs (NVMe's queues, say) stay where the kernel spread
# them: listed, not fatal.
irqs_check() {
    awk -v hk="$1" -v root="$LAB_PROC_ROOT" '
        function first(path, line) {
            line = ""
            if ((getline line <path) <= 0) line = ""
            close(path)
            return line
        }
        NR == 1 {n = NF; next}
        $1 ~ /^[0-9]+:$/ {
            irq = substr($1, 1, length($1) - 1)
            name = ""
            # after the chip and its IRQ number: x86 joins the trigger type
            # to the number (2-edge), arm64 gives it a field (27 Level)
            from = $(n + 4) ~ /^(Edge|Level)$/ ? n + 5 : n + 4
            for (i = from; i <= NF; i++) name = name $i
            if (name == "") name = $NF
            cpus = first(root "/irq/" irq "/effective_affinity_list")
            if (cpus == "") cpus = first(root "/irq/" irq "/smp_affinity_list")
            if (cpus == hk) on++
            else {off++; list = list (list == "" ? "" : " ") irq ":" name "@" (cpus == "" ? "?" : cpus)}
        }
        END {
            mask = first(root "/irq/default_smp_affinity")
            printf "%d on CPU %s alone, %d elsewhere%s, default for new ones %s\n", on, hk, off,
                (off ? " (" list ")" : ""), (mask == "" ? "?" : mask)
        }' "$LAB_PROC_ROOT/interrupts"
}

# irqs_nic_off [-q <queue CPU>] <housekeeping CPU> [<NIC>...]: the
# interrupts the data NICs' IRQs have taken on every CPU but the housekeeping
# one, a running count (/proc/interrupts; `loss` reads it before and after
# each run). A NIC is its names and PCI address, comma-separated
# (<interface>,<VF>,<address>; under DPDK the address alone; empty ones
# skipped), and its IRQs those named after one of its names (ENA's queues,
# <interface>-Tx-Rx-<n>) or naming its address (its MSI-X chip,
# PCI-MSIX-<address>; ENA's management IRQ, ena-mgmnt@pci:<address>; MANA's,
# mana_q<n>@pci:<address>; under DPDK igb_uio's, through which the ENA PMD
# takes its admin queue's interrupts). An IRQ counts once, however many NICs
# it names (MANA's ports share one address). With -q (a mode's @IRQ=, its
# queue IRQs pinned to that CPU: irqs_pin_queues), a queue IRQ's interrupts
# count on every CPU but that one instead, the NICs' other IRQs' as ever. 0
# with no NIC given; nothing, never 0, when a NIC given matches no IRQ:
# whichever driver holds a NIC keeps an IRQ on it, so that NIC was not found,
# and a count without it is not the data NICs'.
irqs_nic_off() {
    local queue_cpu=
    if [[ ${1:-} == -q ]]; then queue_cpu=$2; shift 2; fi
    local hk=$1 nic id alt res='' ids names=''
    shift
    for nic in "$@"; do
        alt=
        IFS=, read -ra ids <<<"$nic"
        for id in "${ids[@]}"; do
            if [[ -n $id ]]; then alt+=${alt:+|}${id//./[.]}; fi
            # an interface's name, not a PCI address: its queues are named after it
            if [[ -n $id && $id != *:* ]]; then names+=" $id"; fi
        done
        if [[ -n $alt ]]; then res+=${res:+ }$alt; fi
    done
    [[ -n $res ]] || { echo 0; return; }
    # one pattern a NIC, each name or address whole: after a space, -, : or
    # @, before a space or -
    awk -v hk="CPU$hk" -v q="CPU${queue_cpu:-$hk}" -v res="$res" -v names="$names" '
        BEGIN {k = split(res, re, " "); for (j = 1; j <= k; j++) re[j] = "[ :@-](" re[j] ")[ -]"
            m = split(names, qn, " ")}
        NR == 1 {n = NF; for (i = 1; i <= NF; i++) {if ($i != hk) col[i + 1] = 1; if ($i != q) qcol[i + 1] = 1}; next}
        $1 ~ /^[0-9]+:$/ {
            tail = " "
            for (i = n + 2; i <= NF; i++) tail = tail $i " "
            hit = 0
            for (j = 1; j <= k; j++) if (tail ~ re[j]) {seen[j] = 1; hit = 1}
            if (!hit) next
            queue = 0
            for (j = 1; j <= m; j++) if (index($NF, qn[j] "-") == 1) queue = 1
            if (queue) {for (c in qcol) s += $c} else {for (c in col) s += $c}
        }
        END {
            for (j = 1; j <= k; j++) if (!(j in seen)) exit
            printf "%.0f\n", s
        }' "$LAB_PROC_ROOT/interrupts"
}

# irqs_nic_queues <interface>...: the data NICs' queue IRQs, "<irq> <name>" a
# line: those named after one of the interfaces (ENA's <interface>-Tx-Rx-<n>;
# its management IRQ, ena-mgmnt@pci:<address>, is not one, nor an interface
# whose name only starts with one of them)
irqs_nic_queues() {
    awk -v names="$*" '
        BEGIN {m = split(names, qn, " ")}
        $1 ~ /^[0-9]+:$/ {for (j = 1; j <= m; j++) if (index($NF, qn[j] "-") == 1) {print substr($1, 1, length($1) - 1), $NF; next}}
    ' "$LAB_PROC_ROOT/interrupts"
}

# irqs_queues_check <CPU> <interface>...: "<k> of <n> data NIC queue IRQs on
# CPU <CPU> alone[, elsewhere: <irq>:<name>@<CPUs> ...][; effective elsewhere
# until their next interrupt (...): <irq>:<name>@<CPUs> ...]"; 1 unless every
# one's affinity is that CPU, or when none is found. Judged by the affinity
# (smp_affinity_list, which a refused write leaves as it was): a VM moves an
# IRQ only at its next interrupt, so a quiet queue's effective CPU lags until
# then (an early lab run stopped on exactly that), and is reported, not failed;
# isolated-nic-irqs counts what interrupts actually land elsewhere in a run
irqs_queues_check() {
    local cpu=$1 irq name want got on=0 n=0 off='' lag=''
    shift
    while read -r irq name; do
        n=$((n + 1))
        want=$(cat "$LAB_PROC_ROOT/irq/$irq/smp_affinity_list" 2>/dev/null) || want=
        got=$(cat "$LAB_PROC_ROOT/irq/$irq/effective_affinity_list" 2>/dev/null) || got=
        if [[ $want == "$cpu" ]]; then
            on=$((on + 1))
            [[ -z $got || $got == "$cpu" ]] || lag+=" $irq:$name@$got"
        else
            off+=" $irq:$name@${want:-?}"
        fi
    done < <(irqs_nic_queues "$@")
    echo "$on of $n data NIC queue IRQs on CPU $cpu alone${off:+, elsewhere:$off}${lag:+; effective elsewhere until their next interrupt (a VM moves an IRQ then):$lag}"
    ((n > 0 && on == n))
}

# irqs_pin_queues <CPU> <when> <interface>...: a mode's @IRQ=: after
# irqs_pin (every IRQ on housekeeping), the data NICs' queue IRQs on <CPU>,
# then irqs_queues_check's line, "irqs <when>: ...", on stderr; 1 when one is
# not there (a write refused) or there is none to move, saying so
irqs_pin_queues() {
    local cpu=$1 when=$2 irqs
    shift 2
    irqs=$(irqs_nic_queues "$@" | awk '{print $1}')
    if [[ -z $irqs ]]; then
        echo "irqs $when: no queue IRQ named after $* (ENA's <interface>-Tx-Rx-<n>)" >&2
        return 1
    fi
    # one sudo for them all
    # shellcheck disable=SC2086
    sudo bash -c 'cpu=$1 root=$2; shift 2; for irq in "$@"; do echo "$cpu" >"$root/irq/$irq/smp_affinity_list"; done' \
        _ "$cpu" "$LAB_PROC_ROOT" $irqs 2>/dev/null || true
    local line rc=0
    line=$(irqs_queues_check "$cpu" "$@") || rc=1
    echo "irqs $when: $line" >&2
    return $rc
}

# The transport suite's IRQ core (host.sh transport_cpu_map): every data-NIC
# interrupt on one CPU, by the NICs' own sysfs: each interface's MSI IRQs
# (class/net/<if>/device/msi_irqs: the VF's queues; MANA's one PCI function
# serves every port, the management NIC's too) and its VMBus channels
# (class/net/<if>/device/channels/<n>/cpu: Hyper-V signals a netvsc channel
# on that CPU). irqs_pin_data <CPU> <when> <iface>...: each moved there (after
# irqs_pin put everything else on housekeeping), then irqs_data_check's
# lines, "irqs <when>: ..." on stderr; 1 when one stays elsewhere or there is
# none to move.
irqs_pin_data() {
    local cpu=$1 when=$2 i f rc=0 line
    shift 2
    for i in "$@"; do
        for f in "$LAB_SYS_ROOT/class/net/$i/device/msi_irqs"/*; do
            [[ -e $f ]] || continue
            echo "$cpu" | sudo tee "$LAB_PROC_ROOT/irq/${f##*/}/smp_affinity_list" >/dev/null 2>&1 || true
        done
        for f in "$LAB_SYS_ROOT/class/net/$i/device/channels"/*/cpu; do
            [[ -e $f ]] || continue
            echo "$cpu" | sudo tee "$f" >/dev/null 2>&1 || true
        done
    done
    line=$(irqs_data_check "$cpu" "$@") || rc=1
    echo "irqs $when: $(tr '\n' ';' <<<"$line" | sed 's/;$//')" >&2
    return $rc
}

# irqs_data_settle <CPU> <when> <kick> <iface>...: every data-NIC MSI IRQ
# irqs_pin_data gave <CPU> effective there before a point starts. x86 under
# Hyper-V (hv_pci, no interrupt remapping) moves an IRQ to a new affinity
# only at its next interrupt, which its old CPU still takes: in a real
# session the mlx5 completion vectors no traffic had used since pinning each
# fired once on its own isolated CPU mid-run, whenever a connection first
# used its queue. So each IRQ still effective elsewhere is fired now:
# `<kick> <cpu> <iface>` (the caller's: a frame sent from <cpu>, which goes
# out on that CPU's queue, whose completion raises its vector) from each CPU
# it is effective on, up to three times. Then "irqs <when>: <k> of <n>
# data-NIC IRQs effective on CPU <CPU>" and what moved, or what still lags,
# on stderr. A lag is reported, not fatal (fairness.py's irq rule counts what
# lands on a hot CPU); an IRQ with no effective list is the affinity's.
irqs_data_settle() {
    local cpu=$1 when=$2 kick=$3 i f irq iface eff c try from n=0 moved='' lag=''
    shift 3
    while read -r irq iface; do
        [[ -n $irq ]] || continue
        n=$((n + 1))
        eff=$(cat "$LAB_PROC_ROOT/irq/$irq/effective_affinity_list" 2>/dev/null) || eff=
        [[ -z $eff || $eff == "$cpu" ]] && continue
        from=$eff
        for try in 1 2 3; do
            for c in $(cpu_list_expand "$eff"); do
                [[ $c == "$cpu" ]] || "$kick" "$c" "$iface" >/dev/null 2>&1 </dev/null || true
            done
            eff=$(cat "$LAB_PROC_ROOT/irq/$irq/effective_affinity_list" 2>/dev/null) || eff=
            [[ $eff == "$cpu" ]] && break
        done
        if [[ $eff == "$cpu" ]]; then moved+="${moved:+, }$irq moved by a frame from CPU $from"; else lag+="${lag:+ }$irq@$eff"; fi
    done < <(for i in "$@"; do
        for f in "$LAB_SYS_ROOT/class/net/$i/device/msi_irqs"/*; do
            [[ -e $f ]] && echo "${f##*/} $i"
        done
    done | sort -n -u -k1,1)
    local lagging
    lagging=$(wc -w <<<"$lag")
    echo "irqs $when: $((n - lagging)) of $n data-NIC IRQs effective on CPU $cpu${moved:+ ($moved)}${lag:+; still on another until their next interrupt: $lag (after 3 frames from each)}" >&2
    return 0
}

# cpu_list_expand <list>: a CPU list ("1-3,5") one CPU a word
cpu_list_expand() {
    local part
    for part in ${1//,/ }; do
        if [[ $part == *-* ]]; then seq "${part%-*}" "${part#*-}"; else echo "$part"; fi
    done | tr '\n' ' '
}

# irqs_data_check <CPU> <iface>...: one line per data-NIC IRQ (its affinity)
# and VMBus channel (its cpu): "ok <iface> irq <n>" or "FAIL <iface> irq <n>
# on <CPUs>"; then "<k> of <n> on CPU <CPU>"; 1 unless all are, or with none
irqs_data_check() {
    local cpu=$1 i f c n=0 ok=0
    shift
    for i in "$@"; do
        for f in "$LAB_SYS_ROOT/class/net/$i/device/msi_irqs"/*; do
            [[ -e $f ]] || continue
            n=$((n + 1))
            c=$(cat "$LAB_PROC_ROOT/irq/${f##*/}/smp_affinity_list" 2>/dev/null) || c=
            if [[ $c == "$cpu" ]]; then ok=$((ok + 1)); echo "ok $i irq ${f##*/}"; else echo "FAIL $i irq ${f##*/} on ${c:-?}"; fi
        done
        for f in "$LAB_SYS_ROOT/class/net/$i/device/channels"/*/cpu; do
            [[ -e $f ]] || continue
            n=$((n + 1))
            c=$(cat "$f" 2>/dev/null) || c=
            f=${f%/cpu}
            if [[ $c == "$cpu" ]]; then ok=$((ok + 1)); echo "ok $i channel ${f##*/}"; else echo "FAIL $i channel ${f##*/} on ${c:-?}"; fi
        done
    done
    echo "$ok of $n data-NIC interrupts on CPU $cpu"
    ((n > 0 && ok == n))
}

# irqs_hot_delta <before> <after> <cpulist>: the interrupts each listed CPU
# took between two /proc/interrupts snapshots, "cpu<n>=<all>/<device>" for
# each (all: every row, the timer and IPIs too; device: the numbered IRQs'),
# space-separated; nothing for a CPU neither snapshot lists
irqs_hot_delta() {
    awk -v cpus=",$3," '
        FNR == 1 {f++; for (i = 1; i <= NF; i++) col[f, i + 1] = $i; ncol[f] = NF; next}
        {
            dev = $1 ~ /^[0-9]+:$/
            for (i = 2; i <= ncol[f] + 1; i++) {
                name = col[f, i]
                if ($i !~ /^[0-9]+$/) break
                c = substr(name, 4)
                if (index(cpus, "," c ",") == 0) continue
                all[f, c] += $i
                if (dev) dv[f, c] += $i
                seen[c] = 1
            }
        }
        END {
            n = split(substr(cpus, 2, length(cpus) - 2), list, ",")
            for (j = 1; j <= n; j++) {
                c = list[j]
                if (!(c in seen)) continue
                printf "%scpu%s=%d/%d", sep, c, all[2, c] - all[1, c], dv[2, c] - dv[1, c]
                sep = " "
            }
            print ""
        }' "$1" "$2"
}

# irqs_on_cpu <CPU>: "irqs=<n> timer=<n>[ <IPI>=<n>...]" taken on that CPU,
# from its own column, found by its name in the header (CPU<n>): with CPUs
# offline (SMT=off) /proc/interrupts lists the online ones only, so a
# position is not a CPU number. The local timer is x86's LOC row or arm64's
# arch_timer interrupt; the IPIs that can stop a spinning CPU are irq_work,
# rescheduling, function calls and TLB shootdowns (arm64 names its IPIs by
# number, and broadcasts TLB maintenance in hardware). Nothing when the CPU
# is not listed.
irqs_on_cpu() {
    awk -v name="CPU$1" '
        NR == 1 {for (i = 1; i <= NF; i++) if ($i == name) col = i + 1; next}
        !col {exit}
        NF >= col {s += $col}
        $1 == "LOC:" || / arch_timer$/ {loc += $col; f = 1}
        $1 ~ /^(IWI|RES|CAL|TLB):$/ {ipi = ipi sprintf(" %s=%s", tolower(substr($1, 1, 3)), $col)}
        $1 ~ /^IPI[0-9]+:$/ && /IRQ work interrupts/ {ipi = ipi sprintf(" iwi=%s", $col)}
        $1 ~ /^IPI[0-9]+:$/ && /Rescheduling interrupts/ {ipi = ipi sprintf(" res=%s", $col)}
        $1 ~ /^IPI[0-9]+:$/ && /Function call interrupts/ {ipi = ipi sprintf(" cal=%s", $col)}
        END {if (col) printf "irqs=%d timer=%s%s\n", s, (f ? loc : ""), ipi}' "$LAB_PROC_ROOT/interrupts"
}
