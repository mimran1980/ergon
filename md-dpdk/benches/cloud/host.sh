# Sourced by vm.sh: the host settings that depend on the architecture
# (x86_64, or aarch64: AWS Graviton, run only with lab.sh's LAB_ALLOW_ARM=1)
# and the media driver's CPUs under each Aeron threading mode, as functions
# that print what vm.sh applies, so test/test-lab.sh checks them against
# fakes. Reads $LAB_SYS_ROOT (/sys; the test gives it a fake tree).
#
# Graviton differs from x86 where these say so; everything else vm.sh does
# (isolcpus, nohz_full, rcu_nocbs, IRQs on housekeeping, 2 MiB hugepages,
# transparent hugepages off) is the same on both. Sources: AWS's DPDK ENA
# README and Graviton guides, the kernel's kernel-parameters.txt, and Debian's
# cloud-arm64 kernel configuration. What is a choice rather than a necessity
# (nohlt, the threading modes) is an A/B: a state or a mode, never the
# baseline.
LAB_SYS_ROOT=${LAB_SYS_ROOT:-/sys}
# jumbo_mtus and the mode settings' other readers
# shellcheck source=modes.sh
. "$(dirname "${BASH_SOURCE[0]}")/modes.sh"

# host_arch_ok <uname -m> <allow_arm>: x86_64; aarch64 only where lab.sh's
# LAB_ALLOW_ARM=1 reached the host (1); anything else refused (2)
host_arch_ok() {
    case $1 in
    x86_64) return 0 ;;
    aarch64)
        [[ $2 == 1 ]] && return 0
        echo "vm.sh: aarch64 host without lab.sh's LAB_ALLOW_ARM=1: production runs on x86, so Graviton runs only on request" >&2
        return 2
        ;;
    *)
        echo "vm.sh: $1 host: the lab runs on x86_64, or on aarch64 (Graviton) with LAB_ALLOW_ARM=1" >&2
        return 2
        ;;
    esac
}

# debian_arch <uname -m>: Debian's name for it (its cloud kernel is
# linux-image-cloud-<it>)
debian_arch() {
    case $1 in
    x86_64) echo amd64 ;;
    aarch64) echo arm64 ;;
    *) return 1 ;;
    esac
}

# build_env <uname -m>: the compiler settings vm.sh exports for every cargo
# build and bench, as one export line (none on x86). Graviton: code for the
# core it runs on (the lab builds on the instance itself), so atomics compile
# to single LSE instructions (ldadd) rather than the outline-atomics helper
# calls gcc and rustc emit for generic aarch64, on every counter bump of the
# send and receive paths (AWS's Graviton C/C++ and Rust guides). RUSTFLAGS
# reaches the Rust and CFLAGS md-dpdk's C (the cc crate), but not Aeron's:
# rusteron gives CMake its own CMAKE_C_FLAGS, so CMake ignores CFLAGS, and
# builds Aeron's C with -march=native on every architecture (rusteron's
# release default; RUSTERON_C_MARCH overrides it). x86 thus differs only in
# its Rust and md-dpdk's C, which stay generic there. Every cargo call must
# see the same flags, or it rebuilds the media driver without them.
build_env() {
    if [[ $1 == aarch64 ]]; then echo "export CFLAGS='-mcpu=native' RUSTFLAGS='-Ctarget-cpu=native'"; fi
}

# boot_args <uname -m> <cloud>: the kernel command line every boot needs from
# bootstrap on, so the pinned state has it too. Graviton metal's SMMU
# translates DMA, which igb_uio does not program: iommu.passthrough=1 (arm64
# ignores x86's iommu=pt, an [X86] parameter). A virtualised Graviton has no
# IOMMU, so it changes nothing there.
boot_args() {
    if [[ $1 == aarch64 && $2 == aws ]]; then echo iommu.passthrough=1; fi
}

# isolation_args <uname -m> <cloud> <last CPU> [poll]: the isolated state's
# kernel command line, every CPU but the housekeeping one (0) off the
# scheduler's balancing, the tick, RCU callbacks and managed IRQs (generic:
# the same on Graviton); x86 AWS adds iommu=pt for metal's IOMMU (Graviton's
# passthrough is boot_args'). poll (the isolated-poll state): idle CPUs poll
# instead of halting, for the interrupt-driven paths (kernel UDP's
# housekeeping CPU). x86: idle=poll. arm64 has no C-states the OS controls
# (Debian's cloud-arm64 kernel builds no cpuidle driver; idle is WFI, which
# the hypervisor may trap): nohlt, which busy-waits in do_idle instead.
isolation_args() {
    local arch=$1 cloud=$2 last=$3 variant=${4:-}
    local args="isolcpus=nohz,domain,managed_irq,1-$last nohz_full=1-$last rcu_nocbs=1-$last irqaffinity=0"
    if [[ $arch == x86_64 && $cloud == aws ]]; then args+=" iommu=pt"; fi
    case $variant in
    '') ;;
    poll)
        case $arch in
        x86_64) args+=" idle=poll" ;;
        aarch64) args+=" nohlt" ;;
        *) return 2 ;;
        esac
        ;;
    *)
        echo "isolate $variant: nothing, or poll (idle CPUs poll)" >&2
        return 2
        ;;
    esac
    echo "$args"
}

# cpu_layout [noisy CPUs]: "<housekeeping> <app> <sender> <receiver> <app2>":
# the first thread of the first five cores, in core order: housekeeping (the
# driver's conductor, its housekeeping and local-I/O threads, the clients'
# conductors), app (ping, pong, publisher or subscriber), the driver's sender
# (under SHARED_NETWORK and SHARED, its one shared agent: threading_env), its
# receiver, and a second app core for the localhost runs (both ends on one
# host; empty on a four-core VM). Physical cores always: Azure numbers SMT
# twins adjacently (0-1, 2-3, ...), AWS x86 as N and N+cores, and Graviton
# has no SMT (each CPU's siblings are itself), so each role has a core to
# itself. Cores with a CPU the pre-flight saw stall (vm.sh preflight's
# noisy_cpus) are skipped, housekeeping's excepted, while enough quiet ones
# remain; the noisy ones fill in last, so a short VM still gets a layout.
cpu_layout() {
    local cpu first siblings primaries=() quiet=() noisy=() part s hit
    local bad=" ${1:-} " dir=$LAB_SYS_ROOT/devices/system/cpu
    for cpu in $(printf '%s\n' "$dir"/cpu[0-9]* | sed 's/.*cpu//' | sort -n); do
        # an offline CPU (SMT=off's siblings) has no topology: not a core
        [[ -r $dir/cpu$cpu/topology/thread_siblings_list ]] || continue
        siblings=$(cat "$dir/cpu$cpu/topology/thread_siblings_list")
        first=${siblings%%[,-]*}
        [[ $first == "$cpu" ]] || continue
        primaries+=("$cpu")
        hit=0
        for part in ${siblings//,/ }; do
            for s in $(seq "${part%-*}" "${part#*-}"); do
                [[ $bad == *" $s "* ]] && hit=1
            done
        done
        if ((${#primaries[@]} == 1 || hit == 0)); then quiet+=("$cpu"); else noisy+=("$cpu"); fi
    done
    local ordered=("${quiet[@]}" "${noisy[@]}")
    echo "${ordered[@]:0:5}"
}

# threading_env <mode> <housekeeping CPU> <sender CPU> <receiver CPU>: the
# driver's environment for an Aeron threading mode (vm.sh driver
# <mode>@AERON_THREADING_MODE=<it>; DEDICATED otherwise). Aeron pins its
# agents by role name (aeron_set_thread_affinity_on_start, which
# ergon-md-dpdk's pin_agents_to_configured_cpus installs in every transport
# mode): the shared network agent by the sender's CPU, the shared agent by
# the conductor's (there are no variables of their own). Every agent on the
# data path busy-spins on a core of its own from the layout, as the dedicated
# sender and receiver do (noop; on arm64 Aeron's spin is the same loop, its
# proc_yield being empty there); an @AERON_*_IDLE_STRATEGY= override in the
# same mode applies after these.
#   DEDICATED       conductor on housekeeping, sender and receiver a core each
#   SHARED_NETWORK  conductor on housekeeping, sender and receiver as one
#                   agent ("[sender, receiver]") on the sender's core
#   SHARED          conductor, sender and receiver as one agent on the
#                   sender's core: the media driver's main thread, which runs
#                   Aeron's first agent itself, with the native resource
#                   agent's work (name resolution, freeing log buffers) inline
#   INVOKER         no media driver process: each benchmark client embeds the
#                   driver (vm.sh client, HARNESS_DRIVER=invoker) and runs its
#                   own work, its client conductor's and the driver's on its
#                   one thread, pinned to its app CPU as ever. No agent
#                   threads to place; the DPDK transport's housekeeping and
#                   local-I/O threads on the housekeeping CPU, explicitly, as
#                   no conductor CPU names it
# A mode Aeron does not know is refused: it would take it as DEDICATED with
# only a warning.
threading_env() {
    local mode=$1 hk=$2 sender=$3 receiver=$4
    case $mode in
    DEDICATED)
        [[ -n $hk && -n $sender && -n $receiver ]] || { echo "AERON_THREADING_MODE=$mode: the layout has no sender and receiver cores" >&2; return 2; }
        cat <<ENV
export AERON_THREADING_MODE='DEDICATED'
export AERON_CONDUCTOR_CPU_AFFINITY='$hk'
export AERON_SENDER_CPU_AFFINITY='$sender'
export AERON_RECEIVER_CPU_AFFINITY='$receiver'
export AERON_SENDER_IDLE_STRATEGY='noop'
export AERON_RECEIVER_IDLE_STRATEGY='noop'
ENV
        ;;
    SHARED_NETWORK)
        [[ -n $hk && -n $sender ]] || { echo "AERON_THREADING_MODE=$mode: the layout has no sender core" >&2; return 2; }
        cat <<ENV
export AERON_THREADING_MODE='SHARED_NETWORK'
export AERON_CONDUCTOR_CPU_AFFINITY='$hk'
export AERON_SENDER_CPU_AFFINITY='$sender'
export AERON_SHAREDNETWORK_IDLE_STRATEGY='noop'
ENV
        ;;
    SHARED)
        [[ -n $sender ]] || { echo "AERON_THREADING_MODE=$mode: the layout has no sender core" >&2; return 2; }
        cat <<ENV
export AERON_THREADING_MODE='SHARED'
export AERON_CONDUCTOR_CPU_AFFINITY='$sender'
export AERON_SHARED_IDLE_STRATEGY='noop'
ENV
        ;;
    INVOKER)
        [[ -n $hk ]] || { echo "AERON_THREADING_MODE=$mode: the layout has no housekeeping core" >&2; return 2; }
        cat <<ENV
export AERON_THREADING_MODE='INVOKER'
export RUSTERON_DPDK_HOUSEKEEPING_CPU='$hk'
ENV
        ;;
    *)
        echo "AERON_THREADING_MODE=$mode: DEDICATED, SHARED_NETWORK, SHARED or INVOKER (Aeron would take anything else as DEDICATED)" >&2
        return 2
        ;;
    esac
}

# setting_cpu <hk|receiver|spare> <threading mode> <housekeeping> <app>
# <sender> <receiver> <app2>: the CPU a mode's @IRQ= (the data NICs' queue
# IRQs) or @NAPI_BUSY= (its kthreads) names, from the layout (cpu_layout):
#   hk        the housekeeping CPU, where every IRQ is otherwise (the baseline)
#   receiver  the Aeron receiver agent's (threading_env): DEDICATED's receiver
#             core; SHARED_NETWORK's and SHARED's one network agent, on the
#             sender's core; INVOKER's, each client's own thread, on its app
#             CPU. AWS's ENA guidance: interrupts on the application's vCPU
#   spare     the layout's fifth core, which only the localhost runs and a
#             second flow use (lab.sh and lib.sh keep both off it under these);
#             none on a four-core host: 2, saying so
setting_cpu() {
    local what=$1 threading=${2:-DEDICATED} hk=$3 app=$4 sender=$5 receiver=$6 app2=${7:-}
    case $what in
    hk) echo "$hk" ;;
    receiver)
        case $threading in
        DEDICATED) echo "$receiver" ;;
        SHARED_NETWORK | SHARED) echo "$sender" ;;
        INVOKER) echo "$app" ;;
        *) echo "receiver: AERON_THREADING_MODE $threading: DEDICATED, SHARED_NETWORK, SHARED or INVOKER" >&2; return 2 ;;
        esac
        ;;
    spare)
        [[ -n $app2 ]] || {
            echo "spare: no fifth core on this host (its layout $hk $app $sender $receiver: four cores), so no spare one: a host with five cores or more (c7i.4xlarge, c8g.2xlarge and up)" >&2
            return 2
        }
        echo "$app2"
        ;;
    *) echo "$what: hk, receiver or spare" >&2; return 2 ;;
    esac
}

# diag_spare_cpu <the layout's CPUs>...: the CPU the diagnostics' gap
# spinner takes (vm.sh diag-start), never one of the layout's: the first core
# the layout leaves (its first thread), else housekeeping's SMT twin (the
# first of the layout's CPUs is housekeeping), else none (1): a four-core
# Graviton has neither, and the spinner on a layout core would take it
diag_spare_cpu() {
    local dir=$LAB_SYS_ROOT/devices/system/cpu cpu siblings used=" $* " s part
    for cpu in $(printf '%s\n' "$dir"/cpu[0-9]* | sed 's/.*cpu//' | sort -n); do
        [[ -r $dir/cpu$cpu/topology/thread_siblings_list ]] || continue
        siblings=$(cat "$dir/cpu$cpu/topology/thread_siblings_list")
        [[ ${siblings%%[,-]*} == "$cpu" && $used != *" $cpu "* ]] && { echo "$cpu"; return 0; }
    done
    [[ -n ${1:-} && -r $dir/cpu$1/topology/thread_siblings_list ]] || return 1
    siblings=$(cat "$dir/cpu$1/topology/thread_siblings_list")
    for part in ${siblings//,/ }; do
        for s in $(seq "${part%-*}" "${part#*-}"); do
            [[ $used != *" $s "* ]] && { echo "$s"; return 0; }
        done
    done
    return 1
}

# smt_takes_offline <CPU>: 0 when SMT=off takes the CPU offline: a thread
# of its core other than the first (the only ones the layout uses)
smt_takes_offline() {
    local f=$LAB_SYS_ROOT/devices/system/cpu/cpu$1/topology/thread_siblings_list
    [[ -r $f ]] || return 1
    local siblings
    siblings=$(cat "$f")
    [[ ${siblings%%[,-]*} != "$1" ]]
}

# kernel_at_least <uname -r> <major> <minor>: whether that kernel release is
# <major>.<minor> or later (Debian's: 7.2.6+deb13-cloud-amd64)
kernel_at_least() {
    [[ $1 =~ ^([0-9]+)\.([0-9]+) ]] || return 1
    local major=${BASH_REMATCH[1]} minor=${BASH_REMATCH[2]}
    ((major > $2 || (major == $2 && minor >= $3)))
}

# napi_defer_args <n>/<ns>: "<n> <ns>", a mode's @NAPI_DEFER= for the data
# NICs' napi_defer_hard_irqs (empty polls before their IRQ is armed again; the
# kernel takes at most S32_MAX) and gro_flush_timeout (ns); a slash between,
# as a comma would split the mode's results.csv rows. 2 for anything else.
napi_defer_args() {
    local n ns
    if [[ $1 =~ ^([0-9]{1,10})/([0-9]{1,18})$ ]]; then
        n=$((10#${BASH_REMATCH[1]})) ns=$((10#${BASH_REMATCH[2]}))
        if ((n <= 2147483647)); then
            echo "$n $ns"
            return 0
        fi
    fi
    echo "NAPI_DEFER=$1: <n>/<ns>: napi_defer_hard_irqs (0 to 2147483647) and gro_flush_timeout in ns" >&2
    return 2
}

# Threaded NAPI busy polling (@NAPI_BUSY=spare, Linux 6.17 and later): each
# NAPI of the data NICs polled without end by a kernel thread of its own, not
# woken by its interrupt, the thread pinned to a core no one else uses. The
# kthread spins its core at 100% for as long as the mode runs. The sysfs
# `threaded` file takes 0 or 1 only (Linux 7.2), so busy polling is set per
# NAPI over generic netlink (the netdev family's napi-set, threaded =
# busy-poll, NETDEV_NAPI_THREADED_BUSY_POLL), with the kernel's own tool,
# ynl (tools/net/ynl; Documentation/networking/napi.rst, "Threaded NAPI busy
# polling"). `ynl` is the caller's: vm.sh's runs the one vm.sh build-ynl
# fetched for the running kernel, as root. Its replies are Python literals
# (pprint), or JSON with --output-json: ynl_reply reads either.

# ynl_reply <field>: a ynl reply on stdin (a NAPI's attributes, or a dump's
# list of them), "<ifindex> <id> <threaded> <pid>" a NAPI (- for a field it
# lacks); with <field>, that field alone
ynl_reply() {
    python3 -c '
import ast, json, sys
text = sys.stdin.read().strip()
try:
    reply = json.loads(text)
except ValueError:
    reply = ast.literal_eval(text)
for napi in reply if isinstance(reply, list) else [reply]:
    if len(sys.argv) > 1:
        print(napi.get(sys.argv[1], "-"))
    else:
        print(*(napi.get(k, "-") for k in ("ifindex", "id", "threaded", "pid")))
' "$@"
}

# napi_list <ifindex>...: "<ifindex> <NAPI id>" a line, every NAPI of those
# interfaces (napi-get, dumped by ifindex); 1 when a dump fails or an
# interface has none
napi_list() {
    local ix reply rows
    for ix in "$@"; do
        reply=$(ynl --dump napi-get --json "{\"ifindex\": $ix}") || { echo "NAPI_BUSY: napi-get dump of ifindex $ix failed: $reply" >&2; return 1; }
        rows=$(ynl_reply <<<"$reply" | awk -v ix="$ix" '$1 == ix && $2 ~ /^[0-9]+$/ {print $1, $2}')
        [[ -n $rows ]] || { echo "NAPI_BUSY: no NAPI on ifindex $ix (napi-get: $reply)" >&2; return 1; }
        echo "$rows"
    done
}

# napi_busy_on <CPU> <ifindex>...: every NAPI of those interfaces set to
# threaded busy-poll, its kthread's pid read back and pinned to <CPU>
# (taskset), as "napi <id> ifindex <ifindex> pid <pid> cpu <CPU>" a line;
# 1, saying why, at the first NAPI that fails (refused, a kthread with no
# pid, a pin refused), its line printed with pid and CPU unknown (?), so the
# caller turns off every NAPI listed (napi_busy_off)
napi_busy_on() {
    local cpu=$1 ix id got threaded pid napis
    shift
    napis=$(napi_list "$@") || return 1
    while read -r ix id; do
        got=$(ynl --do napi-set --json "{\"id\": $id, \"threaded\": \"busy-poll\"}" 2>&1) ||
            { echo "NAPI_BUSY: napi-set of NAPI $id (ifindex $ix) to threaded busy-poll failed: $got" >&2; return 1; }
        read -r threaded pid < <(ynl --do napi-get --json "{\"id\": $id}" | ynl_reply | awk '{print $3, $4}')
        if [[ $threaded != busy-poll || ! ${pid:-} =~ ^[1-9][0-9]*$ ]]; then
            echo "napi $id ifindex $ix pid ? cpu ?"
            echo "NAPI_BUSY: NAPI $id (ifindex $ix) is threaded ${threaded:-?} but has no kthread pid (napi-get: threaded ${threaded:-?})" >&2
            return 1
        fi
        if ! sudo taskset -pc "$cpu" "$pid" >/dev/null 2>&1; then
            echo "napi $id ifindex $ix pid ? cpu ?"
            echo "NAPI_BUSY: taskset -pc $cpu $pid (NAPI $id's kthread) failed" >&2
            return 1
        fi
        echo "napi $id ifindex $ix pid $pid cpu $cpu"
    done <<<"$napis"
}

# napi_busy_off <NAPI id>...: each back to threaded disabled (interrupt-driven
# NAPI, softirq); 1 if any refuses, after trying them all
napi_busy_off() {
    local id rc=0 got
    for id in "$@"; do
        got=$(ynl --do napi-set --json "{\"id\": $id, \"threaded\": \"disabled\"}" 2>&1) ||
            { echo "NAPI_BUSY: NAPI $id back to threaded disabled failed: $got" >&2; rc=1; }
    done
    return $rc
}

# threading_of <overrides>: the AERON_THREADING_MODE in a mode's @-separated
# overrides (empty without one); two are refused
threading_of() {
    local pair found=
    local IFS=@
    for pair in $1; do
        [[ $pair == AERON_THREADING_MODE=* ]] || continue
        [[ -z $found ]] || { echo "two AERON_THREADING_MODE overrides in one mode" >&2; return 2; }
        found=${pair#*=}
    done
    echo "$found"
}

# ena_llq_check <driver log> <driver's memory map> <PCI address>...: whether
# DPDK's ENA PMD put every port's transmit queues in the device's low-latency
# queues (LLQ: "Placement policy: Low latency", one line a port; "Regular" is
# host memory, AWS's "huge performance degradation") and mapped each port's
# LLQ BAR (BAR 2) write-combining (its resource2_wc in the driver's map;
# DPDK falls back to the uncached resource2 without it). One line; 1 unless
# every port has both. On arm64 the mapping is the evidence (x86's PAT table
# does not exist there; the kernel maps resource2_wc Normal-NC).
ena_llq_check() {
    local log=$1 maps=$2 bdf low regular ok=0 wc='' mapped also=''
    shift 2
    low=$(grep -c 'Placement policy: Low latency' "$log" 2>/dev/null) || true
    regular=$(grep -c 'Placement policy: Regular' "$log" 2>/dev/null) || true
    low=${low:-0} regular=${regular:-0}
    if ((low < $# || regular > 0)); then ok=1; fi
    if ((regular > 0)); then also=", $regular Regular"; fi
    for bdf in "$@"; do
        mapped=$(grep -F "/$bdf/resource2" "$maps" 2>/dev/null | awk '{print $NF}' | sed 's|.*/||' | sort -u | tr '\n' ' ')
        mapped=${mapped% }
        if [[ " $mapped " == *" resource2_wc "* ]]; then
            wc+=${wc:+, }"$bdf resource2_wc"
        else
            wc+=${wc:+, }"$bdf NOT (${mapped:-unmapped})"
            ok=1
        fi
    done
    echo "llq $low of $# ports Low latency$also; write-combining: $wc"
    return $ok
}

# invoker_llq_probe <driver env> <housekeeping CPU> <app CPU> <media driver>
# <log> <map> <PCI address>...: INVOKER under DPDK on Graviton (vm.sh
# driver). ena_llq_check reads a driver process's log and memory map, which
# no client's embedded driver leaves where vm.sh can read them before the
# client measures, so the bundled driver starts once with the embedded
# drivers' environment (driver.env, INVOKER with lab.rs embed's settings: the
# conductor, so the one thread, on the app CPU, its idle strategy noop;
# started on the housekeeping CPU, as each client), its PMD's lines to <log>
# and its map to <map>; then it stops (SIGINT, as `stop`), whatever the
# check found. Same host, NICs, igb_uio, DPDK and environment: the embedded
# drivers get what it got. 1 if it never starts or the check fails. (ps, not
# kill -0: the driver runs as root.)
invoker_llq_probe() {
    local env=$1 hk=$2 app=$3 bin=$4 log=$5 maps=$6 pid rc=0
    shift 6
    # the log and the map are the caller's files: root only runs the driver and reads its map
    # shellcheck disable=SC2024
    sudo bash -c ". '$env' && export AERON_CONDUCTOR_CPU_AFFINITY='$app' AERON_SHARED_IDLE_STRATEGY='noop' && exec taskset -c $hk '$bin'" >"$log" 2>&1 &
    pid=$!
    for _ in $(seq 120); do
        grep -q "media driver started" "$log" 2>/dev/null && break
        ps -p "$pid" >/dev/null || break
        sleep 0.5
    done
    if grep -q "media driver started" "$log" 2>/dev/null; then
        # shellcheck disable=SC2024
        sudo cat "/proc/$(pgrep -x media_driver | head -1)/maps" >"$maps" 2>/dev/null || true
        ena_llq_check "$log" "$maps" "$@" || rc=1
    else
        echo "INVOKER's LLQ check: the media driver did not start ($(tail -1 "$log" 2>/dev/null))" >&2
        rc=1
    fi
    sudo pkill -INT -x media_driver 2>/dev/null || true
    for _ in $(seq 40); do ps -p "$pid" >/dev/null || break; sleep 0.25; done
    sudo pkill -KILL -x media_driver 2>/dev/null || true
    wait "$pid" 2>/dev/null
    return $rc
}

# iommu_passthrough_ok <PCI address>...: igb_uio gives the device physical
# addresses, so DMA works only where no IOMMU translates for the device: no
# IOMMU group (a virtualised instance), or one in passthrough (identity, as
# boot_args' iommu.passthrough=1 leaves Graviton metal's SMMU). 1, naming the
# device, otherwise.
iommu_passthrough_ok() {
    local bdf group type ok=0
    for bdf in "$@"; do
        group=$LAB_SYS_ROOT/bus/pci/devices/$bdf/iommu_group
        [[ -e $group ]] || continue
        type=$(cat "$group/type" 2>/dev/null || echo unknown)
        if [[ $type != identity ]]; then
            echo "$bdf: IOMMU group $(basename "$(readlink "$group")") translates DMA ($type): igb_uio needs iommu.passthrough=1" >&2
            ok=1
        fi
    done
    return $ok
}

# mthp_changes <PMD size in kB>: the transparent hugepage sizes (mTHP, since
# Linux 6.9; on arm64 also 16-64 KiB contiguous-PTE ones) not at the lab's
# setting, as "<enabled file> <value>" lines: the PMD size inherits the
# top-level setting (never, the baseline; @THP= otherwise) and every other
# size is never, the kernel's defaults, so the top level alone decides, as
# on x86
mthp_changes() {
    local pmd=$1 f size cur want
    for f in "$LAB_SYS_ROOT"/kernel/mm/transparent_hugepage/hugepages-*kB/enabled; do
        [[ -e $f ]] || continue
        size=${f%kB/enabled}
        size=${size##*hugepages-}
        cur=$(sed -n 's/.*\[\(.*\)\].*/\1/p' "$f")
        want=never
        [[ $size == "$pmd" ]] && want=inherit
        [[ $cur == "$want" ]] || echo "$f $want"
    done
}

# ping_stats: ping -q's summary on stdin as "<min> <avg> <max> <mdev> (ns)
# <replies>/<sent>"; 1 when nothing came back (no rtt line)
ping_stats() {
    awk '/packets transmitted/ {sent = $1; got = $4}
        /^(rtt|round-trip) / {split($4, v, "/"); line = 1}
        END {
            if (!line) exit 1
            printf "%.0f %.0f %.0f %.0f %d/%d\n", v[1] * 1e6, v[2] * 1e6, v[3] * 1e6, v[4] * 1e6, got, sent
        }'
}

# subnet_gateway <address>/<prefix>: the subnet's first address, where AWS's
# VPC router (and Azure's gateway) answers: the next hop to anything off the
# subnet, a peer in another zone's subnet or another region's VPC
subnet_gateway() {
    python3 -c 'import ipaddress, sys; print(next(ipaddress.ip_network(sys.argv[1], strict=False).hosts()))' "$1"
}

# policy_routes <address> <prefix> <interface> <table> [<peer subnet>]: the
# routes `ip route replace` gives a data NIC's own table (vm.sh
# data_kernel_up): its subnet, and the peer's data subnet through the
# gateway when the peer is off it (cross-AZ, cross-region; lab.sh writes it
# to ~/bench/peer_subnet), one a line
policy_routes() {
    local ip=$1 prefix=$2 iface=$3 table=$4 peer=${5:-} net
    net=$(python3 -c 'import ipaddress, sys; print(ipaddress.ip_network(sys.argv[1], strict=False))' "$ip/$prefix")
    echo "$net dev $iface src $ip table $table"
    if [[ -n $peer && $peer != "$net" ]]; then
        echo "$peer via $(subnet_gateway "$ip/$prefix") dev $iface src $ip table $table"
    fi
}

# jumbo_env <placement> <kernel|dpdk>: the driver's environment under JUMBO=1
# (vm.sh appends it; the data NICs' own MTU is vm.sh's): Aeron's MTU, and the
# DPDK transport's (RUSTERON_DPDK_MAX_AERON_MTU: it sets the port's L3 MTU to
# that plus 28), the jumbo MTU's less IP, UDP and alignment (modes.sh jumbo_mtus)
jumbo_env() {
    local mtus aeron
    mtus=$(jumbo_mtus "$1")
    aeron=${mtus#* }
    echo "export AERON_MTU_LENGTH='$aeron'"
    if [[ $2 == dpdk ]]; then echo "export RUSTERON_DPDK_MAX_AERON_MTU='$aeron'"; fi
}

# path_mtu_check <MTU> <local address>... -- <peer address>...: whether
# packets of <MTU> bytes pass from each local data address to each of the
# peer's, unfragmented (ping -M do: Don't Fragment, as Aeron's frames; a path
# that cannot carry them drops them, inter-Region VPC peering above 8500
# bytes). Each pair retried up to LAB_PMTU_TRIES (20) times a second apart:
# the peer sets its own MTU at the same time. A line per pair; 1 if any fails.
path_mtu_check() {
    local mtu=$1 src peer tries=${LAB_PMTU_TRIES:-20} srcs=() peers=() rc=0 ok
    shift
    while (($#)) && [[ $1 != -- ]]; do srcs+=("$1"); shift; done
    shift
    peers=("$@")
    for src in "${srcs[@]}"; do
        for peer in "${peers[@]}"; do
            ok=0
            for _ in $(seq "$tries"); do
                if ping -M do -s $((mtu - 28)) -c 1 -W 1 -I "$src" "$peer" >/dev/null 2>&1; then ok=1; break; fi
                sleep 1
            done
            if ((ok)); then
                echo "pmtu $src -> $peer: $mtu bytes pass"
            else
                echo "pmtu $src -> $peer: $mtu bytes do not pass (ping -M do -s $((mtu - 28)), $tries tries): the path's MTU is lower"
                rc=1
            fi
        done
    done
    return $rc
}

# effective_mtu <driver env file> <publications' channel parameters> <DPDK
# holds the NICs: 0|1> [<data NIC MTU>...]: "<link MTU> <Aeron MTU>" a run
# used: under DPDK the port's L3 MTU (the transport's max Aeron MTU plus 28),
# else the data NICs' smallest; Aeron's what the publications send at, their
# channels' mtu (PUB_PARAMS), else the driver's last AERON_MTU_LENGTH
effective_mtu() {
    local env=$1 pub=$2 dpdk=$3 aeron link nic
    shift 3
    aeron=$(sed -n "s/^export AERON_MTU_LENGTH='\([0-9]*\)'$/\1/p" "$env" | tail -1)
    if [[ "|$pub|" =~ \|mtu=([0-9]+)\| ]]; then aeron=${BASH_REMATCH[1]}; fi
    if ((dpdk)); then
        link=$(sed -n "s/^export RUSTERON_DPDK_MAX_AERON_MTU='\([0-9]*\)'$/\1/p" "$env" | tail -1)
        link=$((${link:-1408} + 28))
    else
        link=
        for nic in "$@"; do if [[ -z $link ]] || ((nic < link)); then link=$nic; fi; done
    fi
    echo "${link:-?} ${aeron:-?}"
}

# rtt_lost_timeout_us <subscription channel parameters> <base round trip max,
# us>: how long the round-trip client waits before it counts a ping lost: on
# an unreliable channel (reliable=false), which never repairs one, 50 ms, or
# four of the network's own round trips where that is longer (cross-region);
# 0, never, on a reliable one
rtt_lost_timeout_us() {
    [[ "|$1|" == *"|reliable=false|"* ]] || { echo 0; return; }
    local base=${2:-0}
    echo $((4 * base > 50000 ? 4 * base : 50000))
}

# shared_agent_check <threading mode> </proc/<driver pid>> <sender CPU>: the
# shared modes' one busy agent on the CPU threading_env gave it, read from
# each thread's allowed CPUs (as vm.sh jitter lists them): SHARED_NETWORK's
# "[sender, receiver]" (the kernel keeps 15 characters of a name), SHARED's
# "[conductor, sender, receiver]", else the media driver's main thread, which
# runs it. 2, saying where it is, when it may run anywhere but that CPU or
# is not there (vm.sh stops the driver: a run measured elsewhere would be
# another mode's); 0 under DEDICATED and INVOKER (nothing shared to check)
shared_agent_check() {
    local mode=$1 dir=$2 cpu=$3 want t found= cpus
    case $mode in
    SHARED_NETWORK) want='[sender, receiv' ;;
    SHARED) want='[conductor, sen' ;;
    *) return 0 ;;
    esac
    for t in "$dir"/task/*; do
        [[ $(cat "$t/comm" 2>/dev/null) == "$want"* ]] && { found=$t; break; }
    done
    if [[ -z $found && $mode == SHARED && -e $dir/task/${dir##*/} ]]; then found=$dir/task/${dir##*/}; fi
    [[ -n $found ]] || { echo "$mode: no thread of its shared agent ($want) in the media driver" >&2; return 2; }
    cpus=$(awk '/^Cpus_allowed_list:/ {print $2}' "$found/status" 2>/dev/null)
    [[ $cpus == "$cpu" ]] && return 0
    echo "$mode: its shared agent ($(cat "$found/comm")/${found##*/}) may run on ${cpus:-?}, not on the sender CPU $cpu alone" >&2
    return 2
}

# conductor_check </proc/<driver pid>> <CPU>: @CONDUCTOR=own's conductor on
# the CPU vm.sh gave it (the layout's fifth core) alone, read from its
# thread's allowed CPUs (as vm.sh jitter lists them): a thread of its own
# named after Aeron's role (conductor; aeron-md-cnd under Aeron's newer
# thread names), else the media driver's main thread, which runs the first
# agent under Aeron's manual main loop (ergon-md-dpdk's driver: the conductor
# under DEDICATED and SHARED_NETWORK). 2, saying where it may run, otherwise
# (vm.sh stops the driver: a run measured with it elsewhere would be another
# mode's)
conductor_check() {
    local dir=$1 cpu=$2 t found= what cpus
    for t in "$dir"/task/*; do
        [[ $(cat "$t/comm" 2>/dev/null) =~ ^(conductor|aeron-md-cnd)$ ]] && { found=$t what="its own thread"; break; }
    done
    if [[ -z $found && -e $dir/task/${dir##*/} ]]; then found=$dir/task/${dir##*/} what="the main thread"; fi
    [[ -n $found ]] || { echo "CONDUCTOR=own: no media driver threads in $dir" >&2; return 2; }
    cpus=$(awk '/^Cpus_allowed_list:/ {print $2}' "$found/status" 2>/dev/null)
    [[ $cpus == "$cpu" ]] && return 0
    echo "CONDUCTOR=own: the conductor ($(cat "$found/comm" 2>/dev/null)/${found##*/}, $what) may run on ${cpus:-?}, not on the fifth core $cpu alone" >&2
    return 2
}

# cnc_identity <cnc.dat>: an embedded driver's name, "<pid>-<start ms>", from
# Aeron's CnC metadata (aeron_cnc_metadata_t, packed to 4: six int32s, the
# client liveness timeout, then the start timestamp and pid, int64s); 1
# without a readable file
cnc_identity() {
    python3 -c 'import struct, sys
d = open(sys.argv[1], "rb").read(48)
start, pid = struct.unpack_from("<qq", d, 32)
print(f"{pid}-{start}")' "$1" 2>/dev/null
}

# Under INVOKER each client embeds its own driver, whose counters start at
# zero and stay in its CnC file after it exits, until the next client's
# driver replaces it. vm.sh loss keeps a ledger, one file per embedded driver
# (named by cnc_identity) holding the last of its loss lines read, and
# reports their sum: counters that grow across a mode's clients as one
# driver's would, so a run's growth is still after less before.
# invoker_ledger_record <ledger dir> <identity> <loss line>
invoker_ledger_record() {
    mkdir -p "$1"
    printf '%s\n' "$3" >"$1/$2"
}

# invoker_ledger_total <ledger dir>: "invoker-ledger=<drivers>;<key>=<sum>;..."
# (keys in the order first read; lib.sh loss_row counts a key the line before
# a run lacks from zero, and drops the marker). Aeron's telemetry's longest
# cycles and thresholds (aeron_telemetry: *-max-cycle-ns, *-threshold-ns)
# are the highest of the drivers', not their sum
invoker_ledger_total() {
    local f files=()
    for f in "$1"/*; do [[ -f $f ]] && files+=("$f"); done
    if ((${#files[@]} == 0)); then echo "invoker-ledger=0"; return; fi
    awk -v n="${#files[@]}" -F';' '{for (i = 1; i <= NF; i++) if (split($i, kv, "=") == 2) {
            if (!(kv[1] in sum)) {order[++k] = kv[1]; sum[kv[1]] = kv[2] + 0; continue}
            if (kv[1] ~ /-(max-cycle|threshold)-ns$/) {if (kv[2] + 0 > sum[kv[1]]) sum[kv[1]] = kv[2] + 0}
            else sum[kv[1]] += kv[2]}}
        END {printf "invoker-ledger=%d", n; for (i = 1; i <= k; i++) printf ";%s=%d", order[i], sum[order[i]]; print ""}' "${files[@]}"
}

# aeron_telemetry: every counter of a driver (`counters ''`: "<value> <type>
# <label>" a line) on stdin; one "info-<key>=<value>;..." line of Aeron's
# stall and flow-control telemetry, matched by Aeron 1.53.3's labels exactly
# (aeron_system_counters.c; the conductor appends ": <threading mode>" to the
# agents' longest cycles and ": threshold=<duration> <threading mode>" to
# their cycles over it, aeron_driver_conductor.c), system counters (type 0)
# only: each agent's longest duty cycle (ns, since the driver started), its
# cycles over the threshold and the threshold (ns, from the label: vm.sh
# common_env sets them); the sender's flow-control limits (back-pressure
# events), status messages sent and received, flow control under and over
# runs. Nothing printed when none is there (another driver build)
aeron_telemetry() {
    awk '
        BEGIN {
            n = split("conductor-max-cycle-ns conductor-cycles-over conductor-threshold-ns sender-max-cycle-ns sender-cycles-over sender-threshold-ns receiver-max-cycle-ns receiver-cycles-over receiver-threshold-ns sender-fc-limits sms-sent sms-received fc-under-runs fc-over-runs", keys, " ")
            exact["Sender flow control limits, i.e. back-pressure events"] = "sender-fc-limits"
            exact["Status Messages sent"] = "sms-sent"
            exact["Status Messages received"] = "sms-received"
            exact["Flow control under runs"] = "fc-under-runs"
            exact["Flow control over runs"] = "fc-over-runs"
            split("Conductor Sender Receiver", agent, " ")
            split("conductor sender receiver", role, " ")
            unit["ns"] = 1; unit["us"] = 1000; unit["ms"] = 1000000; unit["s"] = 1000000000
        }
        # a label may hold any character but the two fields before it
        $2 != "0" || $1 !~ /^-?[0-9]+$/ {next}
        {
            label = $0
            sub(/^[ \t]*-?[0-9]+[ \t]+[0-9]+[ \t]/, "", label)
            if (label in exact) {got[exact[label]] = $1; next}
            for (i = 1; i <= 3; i++) {
                base = agent[i] " max cycle time doing its work in ns"
                if (label == base || index(label, base ": ") == 1) got[role[i] "-max-cycle-ns"] = $1
                base = agent[i] " work cycle exceeded threshold count"
                if (label == base || index(label, base ": ") == 1) {
                    got[role[i] "-cycles-over"] = $1
                    if (match(label, /: threshold=[0-9]+(ns|us|ms|s)( |$)/)) {
                        t = substr(label, RSTART + 12, RLENGTH - 12)
                        sub(/ $/, "", t)
                        u = t
                        sub(/^[0-9]+/, "", u)
                        got[role[i] "-threshold-ns"] = sprintf("%d", (t + 0) * unit[u])
                    }
                }
            }
        }
        END {
            for (i = 1; i <= n; i++) if (keys[i] in got) {printf "%sinfo-%s=%s", sep, keys[i], got[keys[i]]; sep = ";"}
            if (sep != "") print ""
        }'
}

# invoker_client_ready <cnc.dat> <ledger dir>: 0 when no CnC file is there or
# its driver is in the ledger; 1, saying so, when the last client's embedded
# driver was never read by vm.sh loss (the next client's would delete its
# counters unread)
invoker_client_ready() {
    [[ -e $1 ]] || return 0
    local id
    id=$(cnc_identity "$1") || { echo "the last client's embedded driver's CnC file cannot be read" >&2; return 1; }
    [[ -e $2/$id ]] && return 0
    echo "the last client's embedded driver (CnC $id) was never read by vm.sh loss: its loss counters would be lost" >&2
    return 1
}

# smt_check <uname -m> <smt/control>: whether SMT=off can apply here: x86
# whose control is on (or already off); arm64 has no SMT, and notsupported,
# forceoff (the kernel command line) or notimplemented leave nothing to turn
# off. 2 with why otherwise.
smt_check() {
    [[ $1 == x86_64 ]] || { echo "SMT=off: $1 has no SMT (each CPU a core)" >&2; return 2; }
    case $2 in
    on | off) return 0 ;;
    *) echo "SMT=off: smt/control is ${2:-missing}: nothing to turn off" >&2; return 2 ;;
    esac
}

# Time on AWS. Debian's EC2 image keeps time with systemd-timesyncd
# (FallbackNTP=169.254.169.123), which slews the clock by up to some
# thousands of ppm for minutes after a boot and records nothing: vm.sh
# bootstrap puts chrony in its place, on the Amazon Time Sync Service (with
# LAB_PLACEMENT=precision-time on the ENA's PTP hardware clock first, from
# the boot after build-ptp turns it on), lib.sh waits for it to settle
# before each state and logs it around every mode. Azure's image keeps its
# own chrony on the Hyper-V PTP clock (ptp_hyperv).

# chrony_conf <aws|precision-time>: Debian's chrony.conf on stdin as the lab
# runs it, on stdout: its pool (and any server or peer) commented out, and
# leapseclist, or Debian 12's leapsectz (the service smears leap seconds,
# and Debian's own comment says to drop it for such a source), and its
# makestep (the lab's below); then, between marker lines, AWS's own line
# for the Amazon Time Sync Service at its link-local address, polled every
# 16 s, and a step of any offset over 100 us in the first three updates.
# precision-time (vm.sh phc_setup's, from the next boot): the PHC
# first (/dev/ptp_ena, vm.sh phc_setup; AWS's line: poll 0, the driver's
# default dpoll of 0, inside ENA's 125 reads a second, so never under -2),
# the service a backup, not preferred. Its own output again gives the same
# (the marked block replaced, nothing commented twice). 2 for another kind.
chrony_conf() {
    local kind=$1 block
    case $kind in
    aws)
        block='# The Amazon Time Sync Service at its link-local address, AWS'\''s own line for
# chrony. It serves leap-smeared time, so no leapseclist (above). Any offset
# over 100 us stepped in the first three updates, slewed after.
server 169.254.169.123 prefer iburst minpoll 4 maxpoll 4
makestep 0.0001 3'
        ;;
    precision-time)
        block='# The ENA'\''s PTP hardware clock on the primary ENI first (/dev/ptp_ena: vm.sh
# phc_setup), polled every second (the driver'\''s default dpoll, 0: ENA allows
# 125 reads a second, so never under -2), AWS'\''s own line; the Amazon Time
# Sync Service a backup, not preferred. It serves leap-smeared time, so no
# leapseclist (above). Any offset over 100 us stepped in the first three
# updates, slewed after.
refclock PHC /dev/ptp_ena poll 0 delay 0.000010 prefer
server 169.254.169.123 iburst minpoll 4 maxpoll 4
makestep 0.0001 3'
        ;;
    *)
        echo "chrony_conf: aws or precision-time, not $kind" >&2
        return 2
        ;;
    esac
    awk '
        /^# ergon-lab time sync begin/ {skip = 1; next}
        skip && /^# ergon-lab time sync end/ {skip = 0; next}
        skip {next}
        /^(pool|server|peer|refclock|leapseclist|leapsectz|makestep)([ \t]|$)/ {print "#" $0; next}
        {print}' || return 2
    printf '# ergon-lab time sync begin (vm.sh bootstrap)\n%s\n# ergon-lab time sync end\n' "$block"
}

# phc_selected <bound, ns>: `chronyc -c -n sources` on stdin (mode, state,
# name, stratum, poll, reach, last rx, adjusted offset (s), ...); 0 when the
# PHC (a reference clock, #, named PHC<n>) is chrony's selected source (*)
# and its last offset is within the bound; 2 otherwise. A line saying which,
# for the run's record
phc_selected() {
    awk -F, -v bound="$1" '
        $1 == "#" && $3 ~ /^PHC/ {phc = $3; state = $2; off = $8 * 1e9}
        $2 == "*" {selected = $3}
        END {
            if (phc == "") {print "phc none among chrony'\''s sources"; exit 2}
            if (state != "*") {printf "phc not selected: chrony'\''s selected source is %s (%s is %s)\n", (selected == "" ? "none" : selected), phc, state; exit 2}
            if (off > bound + 0 || -off > bound + 0) {printf "phc %s selected, last offset %.0f ns: over %s ns\n", phc, off, bound; exit 2}
            printf "phc %s selected, last offset %.0f ns (within %s ns)\n", phc, off, bound
        }'
}

# phc_udev_rule <PCI address>: /dev/ptp_ena, the name chrony reads the PHC
# by, for the PTP clock of the primary ENI at that address alone (with the
# PHC on there only, no data ENI's clock could take the name)
phc_udev_rule() {
    echo "SUBSYSTEM==\"ptp\", KERNELS==\"$1\", SYMLINK+=\"ptp_ena\""
}

# phc_boot_script <PCI address>: what turns the primary ENI's PHC on at each
# boot, before the network is up. Linux's own ENA driver has the PHC from
# 6.17 (this kernel, 7.2, included), off unless devlink turns it on:
# enable_phc, a driverinit parameter, so it takes effect at the reload, and
# lost at every boot. Fails (set -e) when the device never comes or the
# clock is not there after
phc_boot_script() {
    cat <<SCRIPT
#!/bin/sh
# ergon-lab (vm.sh phc_setup): the ENA's PTP hardware clock on the primary
# ENI, $1, for chrony (/dev/ptp_ena)
set -e
for _ in \$(seq 60); do [ -e /sys/bus/pci/devices/$1/net ] && break; sleep 0.5; done
devlink dev param set pci/$1 name enable_phc value true cmode driverinit
devlink dev reload pci/$1
udevadm settle
test -e /dev/ptp_ena
SCRIPT
}

# phc_unit: the systemd unit that runs it at every boot, before the network
# and chrony
phc_unit() {
    cat <<'UNIT'
[Unit]
Description=ergon-lab: the ENA's PTP hardware clock on the primary ENI, for chrony
DefaultDependencies=no
After=systemd-udev-trigger.service
Before=network-pre.target chrony.service
Wants=network-pre.target
[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/usr/local/sbin/ergon-lab-phc
[Install]
WantedBy=multi-user.target
UNIT
}

# ---- The transport suite (transport.sh; vm.sh transport-*, netem, sample) ----
# Every function reads and writes under $LAB_SYS_ROOT and $LAB_PROC_ROOT, so
# test/test-transport-runner.sh runs them against fake trees.

# transport_cpu_map [off|on]: "<housekeeping> <irq> <isolated,...>": the
# first thread of each core in core order (an offline CPU, SMT off's twins,
# has no topology and is no core): the first core housekeeping, the second
# every data-NIC IRQ, the rest isolated. Standard_D16s_v6 with SMT off: 0 2
# 4,6,8,10,12,14. 2 with fewer than six cores (an Aeron arm needs four
# isolated ones: its sender, receiver and conductor, and a client thread).
# `on` (a session whose sets file says `smt on`): SMT stays on, and each
# isolated core is listed whole, its first thread then its siblings, so a
# loop's CPU and its SMT sibling are both isolated and neighbours in the
# list (c7i.4xlarge: 0 1 2,10,3,11,4,12,5,13,6,14,7,15); the housekeeping and
# IRQ cores' siblings are in no list (not isolated, nothing pinned there).
# 2 when no core has a sibling (no SMT to keep on).
transport_cpu_map() {
    local smt=${1:-off} dir=$LAB_SYS_ROOT/devices/system/cpu cpu siblings cores=() whole=() part s pair=0
    for cpu in $(printf '%s\n' "$dir"/cpu[0-9]* | sed 's/.*cpu//' | sort -n); do
        [[ -r $dir/cpu$cpu/topology/thread_siblings_list ]] || continue
        siblings=$(cat "$dir/cpu$cpu/topology/thread_siblings_list")
        [[ ${siblings%%[,-]*} == "$cpu" ]] || continue
        cores+=("$cpu")
        # the core's threads, its first one first
        s=$cpu
        # (an offline sibling, without topology, is none)
        for part in $(cpulist_expand "$siblings"); do
            [[ $part != "$cpu" && -r $dir/cpu$part/topology/thread_siblings_list ]] || continue
            s+=",$part" pair=1
        done
        whole+=("$s")
    done
    ((${#cores[@]} >= 6)) ||
        { echo "transport: ${#cores[@]} cores: the map needs six (housekeeping, IRQ, an Aeron arm's four isolated)" >&2; return 2; }
    local iso
    case $smt in
    off) iso=$(IFS=,; echo "${cores[*]:2}") ;;
    on)
        ((pair)) || { echo "transport: smt on: no core has an SMT sibling (one thread a core): nothing to keep on" >&2; return 2; }
        iso=$(IFS=,; echo "${whole[*]:2}")
        ;;
    *) echo "transport_cpu_map: smt $smt: off or on" >&2; return 2 ;;
    esac
    echo "${cores[0]} ${cores[1]} $iso"
}

# transport_layout <map>: vm.sh layout's "<housekeeping> <app> <sender>
# <receiver> <app2>" for the media driver under the map: the sender, the
# receiver and (CONDUCTOR=own, app2) the conductor on the first three
# isolated CPUs, app the fourth. transport_clients <map> <setup>: the CPUs a
# bench binary gets (--cpus): every isolated one, less the driver's three
# under kernel and dpdk.
transport_layout() {
    local hk iso
    read -r hk _ iso <<<"$1"
    IFS=, read -ra iso <<<"$iso"
    echo "$hk ${iso[3]} ${iso[0]} ${iso[1]} ${iso[2]}"
}
transport_clients() {
    local iso
    read -r _ _ iso <<<"$1"
    case $2 in
    kernel | dpdk) IFS=, read -ra iso <<<"$iso"; (IFS=,; echo "${iso[*]:3}") ;;
    *) echo "$iso" ;;
    esac
}

# transport_boot_args <map>: the isolated CPUs off the scheduler's balancing,
# the tick, RCU callbacks and managed IRQs; every other IRQ's default the
# housekeeping CPU (the data NICs' then go to the IRQ CPU: irqs.sh
# irqs_pin_data)
transport_boot_args() {
    local hk iso
    read -r hk _ iso <<<"$1"
    echo "isolcpus=nohz,domain,managed_irq,$iso nohz_full=$iso rcu_nocbs=$iso irqaffinity=$hk"
}

# cpulist_expand <list>: a kernel CPU list (0-2,4) as "0 1 2 4"
cpulist_expand() {
    local part out=() c
    for part in ${1//,/ }; do
        [[ $part =~ ^([0-9]+)(-([0-9]+))?$ ]] || continue
        for ((c = BASH_REMATCH[1]; c <= ${BASH_REMATCH[3]:-${BASH_REMATCH[1]}}; c++)); do out+=("$c"); done
    done
    echo "${out[*]}"
}

# transport_boot_check <map>: 0 when the running kernel isolated exactly the
# map's CPUs with nohz_full (the boot line took), else 1 saying what it has
transport_boot_check() {
    local iso want got nohz
    read -r _ _ iso <<<"$1"
    # (as sets: an SMT-on map lists each core's threads together, the
    # kernel in CPU order)
    want=$(cpulist_expand "$iso" | tr ' ' '\n' | sort -n | paste -sd' ' -)
    got=$(cpulist_expand "$(cat "$LAB_SYS_ROOT/devices/system/cpu/isolated" 2>/dev/null)")
    nohz=$(cpulist_expand "$(cat "$LAB_SYS_ROOT/devices/system/cpu/nohz_full" 2>/dev/null)")
    if [[ -n $want && $got == "$want" && $nohz == "$want" ]]; then
        echo "boot: isolated and nohz_full $iso"
        return 0
    fi
    echo "boot: the kernel isolated [$got] with nohz_full [$nohz], not the map's [$want]: the boot line did not take (vm.sh transport-isolate, then a reboot)" >&2
    return 1
}

# transport_ports_check <base> <span> [<ephemeral lo> <hi>]: 0 when the
# bench ports, base to base + span - 1, stay clear of the media driver's
# wildcard ranges (20000-21999: vm.sh common_env) and of the kernel's
# ephemeral range (given, else read from ip_local_port_range when there is
# one), else 1 saying which they meet
transport_ports_check() {
    local base=$1 span=$2 lo=${3:-} hi=${4:-} last f=${LAB_PROC_ROOT:-/proc}/sys/net/ipv4/ip_local_port_range
    last=$((base + span - 1))
    if ((base <= 21999 && last >= 20000)); then
        echo "ports $base-$last meet the media driver's wildcard ranges 20000-21999" >&2
        return 1
    fi
    if [[ -z $lo && -r $f ]]; then read -r lo hi <"$f"; fi
    if [[ -n $lo ]] && ((base <= hi && last >= lo)); then
        echo "ports $base-$last meet the kernel's ephemeral range $lo-$hi" >&2
        return 1
    fi
    echo "ports $base-$last clear of 20000-21999${lo:+ and the ephemeral range $lo-$hi}"
}

# transport_huge_pages <term length> <streams>: the 2 MiB pages a hugetlbfs
# Aeron directory needs for one IPC arm, twice over. Each file is a whole
# number of pages (AERON_FILE_PAGE_SIZE 2 MiB, vm.sh common_env): the CnC
# (two 1 MiB buffers, 1 MiB of counter values, their 4 MiB of metadata, the
# 1 MiB error log: 5 pages) and the loss report (1 page); a log is three
# terms and its 4 KiB of metadata. A point's run holds each stream's ping
# and echo publications, and the run before it may still be lingering its
# own: 4 logs a stream.
transport_huge_pages() {
    local term=$1 streams=$2 page=2097152 log
    log=$(((3 * term + 4096 + page - 1) / page))
    echo $((2 * (6 + 4 * streams * log)))
}

# transport_huge_mount <mount point> <pages>: a hugetlbfs of <pages> 2 MiB
# pages at <mount point> (mode 1777, as /dev/shm), every page reserved for
# it at mount (min_size), so nothing else on the host can take them; then
# verified (transport_huge_check). Refused (1, nothing mounted) when the
# kernel has fewer 2 MiB pages free and unreserved; one already mounted is
# kept only when it is right.
transport_huge_mount() {
    local mnt=$1 pages=$2 hp=$LAB_SYS_ROOT/kernel/mm/hugepages/hugepages-2048kB free resv bytes
    bytes=$((pages * 2097152))
    if mountpoint -q "$mnt"; then
        transport_huge_check "$mnt" "$pages"
        return
    fi
    free=$(cat "$hp/free_hugepages" 2>/dev/null) resv=$(cat "$hp/resv_hugepages" 2>/dev/null)
    if ! [[ $free =~ ^[0-9]+$ && $resv =~ ^[0-9]+$ ]] || ((free - resv < pages)); then
        echo "hugetlbfs $mnt: $pages pages of 2 MiB wanted, the kernel has $((${free:-0} - ${resv:-0})) free and unreserved" \
            "(vm.nr_hugepages $(cat "$hp/nr_hugepages" 2>/dev/null || echo unread)): not mounted" >&2
        return 1
    fi
    sudo mkdir -p "$mnt" || return 1
    sudo mount -t hugetlbfs -o "mode=1777,pagesize=2M,size=$bytes,min_size=$bytes" none "$mnt" ||
        { echo "hugetlbfs $mnt: mount failed" >&2; return 1; }
    transport_huge_check "$mnt" "$pages"
}

# transport_huge_check <mount point> <pages>: 0, saying so, when
# <mount point> is a mount point that /proc/mounts lists as hugetlbfs with
# pagesize=2M, size and min_size <pages> of them; else 1 saying what it is
transport_huge_check() {
    local mnt=$1 pages=$2 bytes fs opts
    bytes=$((pages * 2097152))
    mountpoint -q "$mnt" || { echo "hugetlbfs $mnt: not a mount point" >&2; return 1; }
    read -r fs opts < <(awk -v m="$mnt" '$2 == m {fs = $3; o = $4} END {print fs, o}' "${LAB_PROC_ROOT:-/proc}/mounts")
    if [[ $fs == hugetlbfs && ,$opts, == *,pagesize=2M,* && ,$opts, == *,size=$bytes,* && ,$opts, == *,min_size=$bytes,* ]]; then
        echo "hugetlbfs $mnt: pagesize 2M, size $bytes, min_size $bytes ($pages pages of 2 MiB, reserved)"
        return 0
    fi
    # (the filesystem named only when it is not hugetlbfs)
    [[ $fs == hugetlbfs ]] || opts="$fs $opts"
    echo "hugetlbfs $mnt: $opts, not hugetlbfs with pagesize=2M, size=$bytes and min_size=$bytes" >&2
    return 1
}

# transport_smt_off: SMT off (smt/control) and verified: control off (or
# forceoff) and smt/active 0; nothing to do where the CPU has none. 1 when
# the kernel kept it on.
transport_smt_off() {
    local d=$LAB_SYS_ROOT/devices/system/cpu/smt c a
    c=$(cat "$d/control" 2>/dev/null) || { echo "smt: no $d/control: SMT state unknown" >&2; return 1; }
    case $c in
    notsupported | notimplemented) echo "smt: $c (no SMT to turn off)"; return 0 ;;
    forceoff) ;;
    *) echo off | sudo tee "$d/control" >/dev/null 2>&1 ;;
    esac
    c=$(cat "$d/control" 2>/dev/null) a=$(cat "$d/active" 2>/dev/null)
    if [[ ($c == off || $c == forceoff) && $a == 0 ]]; then
        echo "smt: control $c active $a online $(cat "$LAB_SYS_ROOT/devices/system/cpu/online" 2>/dev/null)"
        return 0
    fi
    echo "smt: control [$c] active [$a] after writing off: SMT still on" >&2
    return 1
}

# transport_smt_on: SMT kept on (a session whose sets file says `smt on`)
# and verified: control on and smt/active 1. 1 when it is off, or the CPU
# has none.
transport_smt_on() {
    local d=$LAB_SYS_ROOT/devices/system/cpu/smt c a
    c=$(cat "$d/control" 2>/dev/null) || { echo "smt: no $d/control: SMT state unknown" >&2; return 1; }
    a=$(cat "$d/active" 2>/dev/null)
    if [[ $c == on && $a == 1 ]]; then
        echo "smt: on as the session declares (control $c active $a online $(cat "$LAB_SYS_ROOT/devices/system/cpu/online" 2>/dev/null))"
        return 0
    fi
    echo "smt: control [$c] active [$a]: the session declares SMT on" >&2
    return 1
}

# transport_smt_read: the SMT state every point reads back on each host
# (vm.sh settings), "devices.system.cpu.smt.active=<0|1|absent>": the
# session declares it (transport.sh tr_smt_declared), fairness.py's settings
# rule holds every point to it
transport_smt_read() {
    local f=$LAB_SYS_ROOT/devices/system/cpu/smt/active
    if [[ -r $f ]]; then echo "devices.system.cpu.smt.active=$(cat "$f")"; else echo "devices.system.cpu.smt.active=absent"; fi
}

# transport_quiet_settings <housekeeping cpulist> <isolated cpulist>: the
# lab's quiet node settings (k8s/nri-shared-cpus/lab/lib.sh
# lab_quiet_settings: one list for both labs), "path|value|why" a line
transport_quiet_settings() {
    (
        # shellcheck source=../../../k8s/nri-shared-cpus/lab/lib.sh
        . "$(dirname "${BASH_SOURCE[0]}")/../../../k8s/nri-shared-cpus/lab/lib.sh"
        lab_quiet_settings "$1" "$2"
    )
}

# cpumask_norm <hex mask>: the mask without commas, leading zeros or case,
# so 00000000,00000003 and 3 compare equal
cpumask_norm() {
    local m
    m=$(printf '%s' "${1//,/}" | tr 'A-F' 'a-f')
    while [[ $m == 0?* ]]; do m=${m#0}; done
    echo "${m:-0}"
}

# cpulist_norm <cpu list>: its CPUs one per comma, ranges expanded, sorted,
# so 0-1,4 and 0,1,4 compare equal
cpulist_norm() {
    local part a b out=()
    local IFS=,
    for part in $1; do
        if [[ $part == *-* ]]; then
            a=${part%-*} b=${part#*-}
            while ((a <= b)); do out+=("$a"); a=$((a + 1)); done
        elif [[ -n $part ]]; then
            out+=("$part")
        fi
    done
    printf '%s\n' "${out[@]}" | sort -n | paste -sd, -
}

# transport_quiet_apply <housekeeping cpulist> <isolated cpulist>: each one
# this kernel has written and read back, "quiet <path>: <before> -> <after>";
# one it lacks "quiet skip: <path> (not on this kernel)"; 1 when a write was
# refused or does not read back
transport_quiet_apply() {
    local path value why f before after rc=0
    while IFS='|' read -r path value why; do
        case $path in
        /proc/*) f=${LAB_PROC_ROOT:-/proc}/${path#/proc/} ;;
        /sys/*) f=$LAB_SYS_ROOT/${path#/sys/} ;;
        *) continue ;;
        esac
        if ! sudo test -e "$f"; then
            echo "quiet skip: $path (not on this kernel)"
            continue
        fi
        before=$(sudo cat "$f" 2>/dev/null)
        echo "$value" | sudo tee "$f" >/dev/null 2>&1 || true
        after=$(sudo cat "$f" 2>/dev/null)
        # THP reads back as its choices, the one in force bracketed; a sysfs
        # cpumask zero-padded and comma-grouped (03 for 3); a proc cpu list as
        # ranges (0-1 for 0,1)
        if [[ $after == "$value" || $after == *"[$value]"* ]] ||
            [[ $path == /sys/*/cpumask && $(cpumask_norm "$after") == "$(cpumask_norm "$value")" ]] ||
            [[ $path == /proc/*_cpumask && $(cpulist_norm "$after") == "$(cpulist_norm "$value")" ]]; then
            echo "quiet $path: $before -> $after"
        else
            echo "quiet REFUSED $path: stays [$after], wanted $value ($why)" >&2
            rc=1
        fi
    done < <(transport_quiet_settings "$1" "$2")
    return $rc
}

# transport_overrides: TR_OVERRIDES ("TR_<NAME>=<value> ..."), the runner's
# for one arm's host set-up (transport.sh tr_host_overrides: a tuning
# variant's congestion control, the kernel path's busy poll), over the
# settings sourced; 2 for one not of that form
transport_overrides() {
    local kv
    for kv in ${TR_OVERRIDES:-}; do
        [[ $kv =~ ^TR_[A-Z0-9_]+=[A-Za-z0-9._-]+$ ]] || { echo "TR_OVERRIDES: $kv is not TR_<NAME>=<value>" >&2; return 2; }
        printf -v "${kv%%=*}" '%s' "${kv#*=}"
    done
}

# transport_cc_load: the congestion control TR_TCP_CONGESTION names loaded
# where it is a module (tcp_bbr), so the sysctl takes it; never fatal
transport_cc_load() {
    [[ $TR_TCP_CONGESTION == cubic || $TR_TCP_CONGESTION == reno ]] || sudo modprobe "tcp_$TR_TCP_CONGESTION" 2>/dev/null || true
}

# transport_sysctls: "<proc/sys path> <value> <required|optional>" a line,
# from transport-settings.env (TR_*): every socket arm's TCP and busy-poll
# settings; the optional ones are the kernel's from 6.15 (absent before)
transport_sysctls() {
    cat <<EOT
net/ipv4/tcp_congestion_control $TR_TCP_CONGESTION required
net/ipv4/tcp_sack $TR_TCP_SACK required
net/ipv4/tcp_timestamps $TR_TCP_TIMESTAMPS required
net/ipv4/tcp_slow_start_after_idle $TR_TCP_SLOW_START_AFTER_IDLE required
net/ipv4/tcp_rmem $TR_TCP_RMEM required
net/ipv4/tcp_wmem $TR_TCP_WMEM required
net/core/rmem_max $TR_CORE_RMEM_MAX required
net/core/wmem_max $TR_CORE_WMEM_MAX required
net/core/busy_read $TR_BUSY_READ required
net/core/busy_poll $TR_BUSY_POLL required
net/ipv4/tcp_rto_min_us $TR_TCP_RTO_MIN_US optional
net/ipv4/tcp_delack_max_us $TR_TCP_DELACK_MAX_US optional
EOT
}

# transport_sysctls_apply [check]: each written (check: only read) and read
# back, "sysctl <path> <value>", or "sysctl <path> absent (optional: ...)";
# 1 when a required one is absent or any reads back otherwise
transport_sysctls_apply() {
    local path words value need f got rc=0
    while read -r path words; do
        need=${words##* } value=${words% *}
        f=${LAB_PROC_ROOT:-/proc}/sys/$path
        if [[ ! -e $f ]]; then
            if [[ $need == optional ]]; then echo "sysctl $path absent (optional: kernel before 6.15)"; continue; fi
            echo "sysctl MISSING $path (wanted $value)" >&2
            rc=1
            continue
        fi
        [[ ${1:-} == check ]] || echo "$value" | sudo tee "$f" >/dev/null 2>&1 || true
        got=$(tr -s ' \t' '  ' <"$f" 2>/dev/null)
        if [[ $got == "$value" ]]; then
            echo "sysctl $path $got"
        else
            echo "sysctl WRONG $path: [$got], wanted [$value]" >&2
            rc=1
        fi
    done < <(transport_sysctls)
    return $rc
}

# transport_settings_read: each transport_sysctls knob as this host has it
# now, "<knob>=<value>" a line, the knob dotted (net.ipv4.tcp_sack), a value
# of several words single-spaced, and "absent" for one this kernel lacks:
# what fairness.py checks every point against (vm.sh settings)
transport_settings_read() {
    local path words f
    while read -r path words; do
        f=${LAB_PROC_ROOT:-/proc}/sys/$path
        if [[ -e $f ]]; then
            echo "${path//\//.}=$(tr -s ' \t' '  ' <"$f" 2>/dev/null)"
        else
            echo "${path//\//.}=absent"
        fi
    done < <(transport_sysctls)
}

# transport_nstat: the kernel's protocol counters as nstat reads them
# (/proc/net/snmp and /proc/net/netstat: a header line of names, then a line
# of values, per protocol), for their growth over a point (transport_collect.py)
transport_nstat() {
    cat "${LAB_PROC_ROOT:-/proc}/net/snmp" "${LAB_PROC_ROOT:-/proc}/net/netstat" 2>/dev/null
}

# transport_quickack_routes <subnet>: an `ip route change` argument list a
# line for every route to <subnet> in every table (ip -4 route show table
# all), each with quickack 1: no ACK on the data subnet delayed
transport_quickack_routes() {
    ip -4 route show table all 2>/dev/null | awk -v net="$1" '
        $1 == net {
            out = $1
            for (i = 2; i <= NF; i++) {
                if ($i ~ /^(dev|table|proto|scope|src|metric|via)$/) {out = out " " $i " " $(i + 1); i++}
                else if ($i == "quickack") i++
            }
            print out " quickack 1"
        }'
}

# transport_quickack_apply <subnet>: quickack 1 on every route to <subnet>.
# Each is deleted (prefix, table and dev pick it) and appended with quickack
# 1: `ip route change` picks by prefix and table alone, so with one route per
# data NIC on the subnet (the sender and receiver ports) it would change the
# first twice and leave the other without quickack. Run before the data path
# carries traffic.
transport_quickack_apply() {
    local routes r
    routes=$(transport_quickack_routes "$1")
    [[ -n $routes ]] || return 0
    while read -r r; do
        # shellcheck disable=SC2086
        [[ -n $r ]] && sudo ip route del ${r% quickack 1}
    done <<<"$routes"
    while read -r r; do
        # shellcheck disable=SC2086
        [[ -n $r ]] && sudo ip route append $r
    done <<<"$routes"
    return 0
}

# transport_quickack_check <subnet>: 0 when every route to it has quickack
# 1, else 1 naming those without; 1 too when there is none
transport_quickack_check() {
    local lines
    lines=$(ip -4 route show table all 2>/dev/null | awk -v net="$1" '$1 == net')
    [[ -n $lines ]] || { echo "quickack: no route to $1" >&2; return 1; }
    if grep -v 'quickack 1' <<<"$lines" >&2; then
        echo "quickack: the routes above lack quickack 1" >&2
        return 1
    fi
    echo "quickack: $(grep -c . <<<"$lines") route(s) to $1 with quickack 1"
}

# transport_ring <vf> <rx> <tx>: the NIC's rings at that size and read back
# (ethtool -g's current settings); 1 when above its maximum or not taken
transport_ring() {
    local vf=$1 rx=$2 tx=$3 cur
    sudo ethtool -G "$vf" rx "$rx" tx "$tx" >/dev/null 2>&1 || true
    cur=$(ethtool -g "$vf" 2>/dev/null | awk '/^Current hardware settings/ {c = 1} c && /^RX:/ {r = $2} c && /^TX:/ {t = $2} END {print r, t}')
    if [[ $cur == "$rx $tx" ]]; then echo "ring $vf rx $rx tx $tx"; return 0; fi
    echo "ring $vf: [$cur] after asking rx $rx tx $tx (ethtool -g: above its maximum, or refused)" >&2
    return 1
}

# transport_coalesce_off <vf>: interrupt coalescing off (adaptive off, 0 us)
# and read back; a driver with none to set (ethtool -c not supported) said
# so, not a failure; 1 when some is left on
transport_coalesce_off() {
    local vf=$1 cur
    if ! cur=$(ethtool -c "$vf" 2>&1); then
        if [[ $cur == *"not supported"* ]]; then echo "coalesce $vf: none (the driver has no interrupt moderation)"; return 0; fi
        echo "coalesce $vf: ethtool -c failed: $cur" >&2
        return 1
    fi
    # only what the driver reports: ethtool refuses the whole -C for one
    # setting the driver lacks (ENA has no adaptive-tx: "Adaptive TX: n/a")
    local args
    args=$(awk '/^Adaptive RX:/ {if ($3 != "n/a") a = a " adaptive-rx off"; if ($5 != "n/a") a = a " adaptive-tx off"}
        /^rx-usecs:/ {if ($2 != "n/a") a = a " rx-usecs 0"} /^tx-usecs:/ {if ($2 != "n/a") a = a " tx-usecs 0"} END {print a}' <<<"$cur")
    # shellcheck disable=SC2086
    [[ -z $args ]] || sudo ethtool -C "$vf" $args >/dev/null 2>&1 || true
    cur=$(ethtool -c "$vf" 2>/dev/null | awk '/^Adaptive RX:/ {ar = $3; at = $5} /^rx-usecs:/ {r = $2} /^tx-usecs:/ {t = $2} END {print ar, at, r, t}')
    if [[ $cur =~ ^(off|n/a)\ (off|n/a)\ (0|n/a)\ (0|n/a)$ ]]; then echo "coalesce $vf off ($cur)"; return 0; fi
    echo "coalesce $vf: [$cur] (adaptive rx, tx, rx-usecs, tx-usecs) after asking off and 0" >&2
    return 1
}

# netem_set <loss %> <iface>...: netem on each one's root (its egress),
# dropping that share of packets (0: a netem qdisc that drops nothing);
# netem_clear <iface>...: each root back to the default qdisc
netem_set() {
    local p=$1 i
    shift
    for i in "$@"; do sudo tc qdisc replace dev "$i" root netem limit 100000 loss "$p%" || return 1; done
}
netem_clear() {
    local i
    for i in "$@"; do
        if tc qdisc show dev "$i" 2>/dev/null | grep -q '^qdisc netem .* root'; then sudo tc qdisc del dev "$i" root || return 1; fi
    done
}

# ---- Onload over AF_XDP (the transport suite's setup onload: vm.sh onload-up,
# its undoing in vm.sh stop). Onload's sfc_resource registers a NIC by name
# (/sys/module/sfc_resource/afxdp/register) and links its XDP program by the
# driver's ndo_bpf. The ENA's native XDP takes it only with one buffer a frame
# (MTU at most ENA_XDP_MAX_MTU, 3498 at 4 KiB pages: ena_xdp.h) and its queues
# at most half its maximum (ena_xdp.c: "the Rx/Tx channel count should be at
# most half"), and has no ntuple filters (no set_rxnfc), so Onload runs with
# its flow filters off and the NIC on one queue: every TCP and UDP frame lands
# on queue 0, the one Onload binds (its XDP program redirects by queue).
# Onload adds the NIC even when its XDP set-up failed ("hardware init failed",
# resource_driver.c efrm_nic_add, which carries on), and a stack on such a NIC
# froze EC2 instances (Xilinx-CNS/onload#231, #210): so each precondition is
# set first, and the XDP program's taking is read back (the ENA's maxmtu drops
# to ENA_XDP_MAX_MTU while one is attached), fail closed.

# onload_max_mtu <iface>: the interface's largest MTU now (ip -d: maxmtu)
onload_max_mtu() {
    ip -d -j link show dev "$1" 2>/dev/null | python3 -c 'import json, sys
try:
    print(json.load(sys.stdin)[0]["max_mtu"])
except Exception:
    pass'
}

# onload_combined <iface> [max]: the interface's combined queues now (max:
# its maximum), from ethtool -l
onload_combined() {
    ethtool -l "$1" 2>/dev/null | awk -v want="${2:-current}" '/^Pre-set maximums/ {s = "max"} /^Current hardware settings/ {s = "current"}
        /^Combined:/ && s == want {print $2}'
}

# onload_nic_up <iface> <state dir>: the interface on one queue, its MTU at
# most TR_ONLOAD_MTU and its interrupts not deferred (TR_ONLOAD_NAPI_DEFER),
# each one's value first kept in <state dir> for onload_nic_down, then
# registered with Onload and its XDP program read back; 1 saying what failed
onload_nic_up() {
    local i=$1 st=$2 mtu max comb before after sysd=${LAB_SYS_ROOT:-/sys}/module/sfc_resource
    mkdir -p "$st"
    [[ -w $sysd/afxdp/register || -e $sysd/afxdp/register ]] || { echo "onload: $sysd/afxdp/register missing (the sfc_resource module not loaded)" >&2; return 1; }
    mtu=$(cat "${LAB_SYS_ROOT:-/sys}/class/net/$i/mtu") comb=$(onload_combined "$i") max=$(onload_combined "$i" max)
    [[ $comb =~ ^[0-9]+$ && $max =~ ^[0-9]+$ ]] || { echo "onload: $i's combined queues not read (ethtool -l: [$comb] of [$max])" >&2; return 1; }
    ((max >= 2)) || { echo "onload: $i takes at most $max queue: ENA's XDP needs its queues at most half the maximum" >&2; return 1; }
    [[ -e $st/$i.mtu ]] || echo "$mtu" >"$st/$i.mtu"
    [[ -e $st/$i.combined ]] || echo "$comb" >"$st/$i.combined"
    if ((comb != 1)); then
        local rc=0
        sudo ethtool -L "$i" combined 1 2>"$st/ethtool.err" || rc=$?
        [[ $rc == 0 || $rc == 80 ]] || { echo "onload: ethtool -L $i combined 1: $(cat "$st/ethtool.err")" >&2; return 1; }
    fi
    [[ $(onload_combined "$i") == 1 ]] || { echo "onload: $i has $(onload_combined "$i") combined queues after asking 1" >&2; return 1; }
    if ((mtu > TR_ONLOAD_MTU)); then sudo ip link set dev "$i" mtu "$TR_ONLOAD_MTU" || { echo "onload: $i's MTU $TR_ONLOAD_MTU refused" >&2; return 1; }; fi
    echo "$TR_ONLOAD_NAPI_DEFER" | sudo tee "${LAB_SYS_ROOT:-/sys}/class/net/$i/napi_defer_hard_irqs" >/dev/null
    echo 0 | sudo tee "${LAB_SYS_ROOT:-/sys}/class/net/$i/gro_flush_timeout" >/dev/null
    before=$(onload_max_mtu "$i")
    # registered from here on, whatever follows: onload_nic_down unregisters it
    : >"$st/$i.registered"
    echo "$i" | sudo tee "$sysd/afxdp/register" >/dev/null || { echo "onload: registering $i with Onload's AF_XDP failed" >&2; return 1; }
    after=$(onload_max_mtu "$i")
    if [[ -z $after || $after == "$before" ]]; then
        echo "onload: $i's XDP program not attached after registering it (maxmtu $before, then ${after:-unread}): Onload's AF_XDP set-up failed on it (dmesg: hardware init failed)" >&2
        return 1
    fi
    echo "onload $i: registered, AF_XDP on its native XDP (maxmtu $before -> $after), 1 queue (was $comb of $max), MTU $(cat "${LAB_SYS_ROOT:-/sys}/class/net/$i/mtu") (was $mtu), napi_defer $TR_ONLOAD_NAPI_DEFER"
}

# onload_nic_down <iface> <state dir>: what onload_nic_up did, undone:
# unregistered (its XDP program unlinked: maxmtu back up), then its queues and
# MTU as they were; nothing for an interface it never touched; 1 saying what
# did not come back
onload_nic_down() {
    local i=$1 st=$2 rc=0 sysd=${LAB_SYS_ROOT:-/sys}/module/sfc_resource v
    if [[ -e $st/$i.registered ]]; then
        if [[ -e $sysd/afxdp/unregister ]]; then
            echo "$i" | sudo tee "$sysd/afxdp/unregister" >/dev/null || { echo "onload: unregistering $i failed" >&2; rc=1; }
        fi
        v=$(onload_max_mtu "$i")
        if [[ -n $v ]] && ((v <= TR_ONLOAD_MTU)); then echo "onload: $i's maxmtu still $v after unregistering: its XDP program left attached" >&2; rc=1; fi
        rm -f "$st/$i.registered"
    fi
    if [[ -e $st/$i.combined ]]; then
        v=$(cat "$st/$i.combined")
        if [[ $(onload_combined "$i") != "$v" ]]; then
            local erc=0
            sudo ethtool -L "$i" combined "$v" 2>"$st/ethtool.err" || erc=$?
            [[ $erc == 0 || $erc == 80 ]] || { echo "onload: ethtool -L $i combined $v: $(cat "$st/ethtool.err")" >&2; rc=1; }
        fi
        [[ $(onload_combined "$i") == "$v" ]] && rm -f "$st/$i.combined" || { echo "onload: $i has $(onload_combined "$i") combined queues, not its $v" >&2; rc=1; }
    fi
    if [[ -e $st/$i.mtu ]]; then
        v=$(cat "$st/$i.mtu")
        [[ $(cat "${LAB_SYS_ROOT:-/sys}/class/net/$i/mtu") == "$v" ]] || sudo ip link set dev "$i" mtu "$v" || rc=1
        [[ $(cat "${LAB_SYS_ROOT:-/sys}/class/net/$i/mtu") == "$v" ]] && rm -f "$st/$i.mtu" || { echo "onload: $i's MTU not back at $v" >&2; rc=1; }
    fi
    return $rc
}

# onload_stacks_verdict <process name>: onload_stackdump's stack list on
# stdin: "ok <n> stack(s) of <name>: <stack:pids>..." when a stack has a
# process of that name (its pids checked against ps), else "FAIL ..."
onload_stacks_verdict() {
    local name=$1 lines pid stack ok=() all=()
    lines=$(awk '/^#/ {next} NF >= 3 && $1 ~ /^[0-9]+$/ {print $1, $3}' | tr -d '[]')
    while read -r stack pids; do
        [[ -n $stack ]] || continue
        all+=("$stack:$pids")
        for pid in ${pids//,/ }; do
            [[ $(cat "${LAB_PROC_ROOT:-/proc}/$pid/comm" 2>/dev/null) == "$name" ]] && { ok+=("$stack:$pid"); break; }
        done
    done <<<"$lines"
    if ((${#ok[@]})); then echo "ok ${#ok[@]} stack(s) of $name: ${ok[*]}"; return 0; fi
    echo "FAIL no Onload stack of $name (stacks: ${all[*]:-none})"
    return 1
}

# netem_check <loss %|none> <iface>...: 0 when each one's root qdisc is netem
# at that loss (none: no netem on it), else 1 saying which
netem_check() {
    local want=$1 i line got rc=0
    shift
    for i in "$@"; do
        line=$(tc qdisc show dev "$i" 2>/dev/null | grep '^qdisc netem .* root' || true)
        if [[ $want == none ]]; then
            [[ -z $line ]] || { echo "netem: $i still has [$line]" >&2; rc=1; }
            continue
        fi
        [[ -n $line ]] || { echo "netem: $i has no netem root qdisc (wanted loss $want%)" >&2; rc=1; continue; }
        got=$(sed -n 's/.* loss \([0-9.]*\)%.*/\1/p' <<<"$line")
        awk -v a="${got:-0}" -v b="$want" 'BEGIN {d = a - b; exit !(d < 1e-9 && d > -1e-9)}' ||
            { echo "netem: $i loss [${got:-0}]%, wanted $want% ($line)" >&2; rc=1; }
    done
    ((rc)) || echo "netem: $* at $want"
    return $rc
}

# netem_show <iface>...: "<iface> <root qdisc>" a line (netem, mq, fq_codel...):
# a no-qdisc round's proof that none is netem
netem_show() {
    local i
    for i in "$@"; do
        tc qdisc show dev "$i" 2>/dev/null | awk -v i="$i" '/ root / {print i, $2; exit}'
    done
}

# netem_stats <iface>...: "<iface> <sent packets> <dropped>" a line from each
# root netem qdisc's tc -s counters (nothing for one without netem)
netem_stats() {
    local i
    for i in "$@"; do
        tc -s qdisc show dev "$i" 2>/dev/null | awk -v i="$i" '
            /^qdisc / {on = ($2 == "netem" && / root /)}
            on && /^ *Sent / {d = $7; sub(/,$/, "", d); print i, $4, d; on = 0}'
    done
}

# transport_threads [<seconds>]: every thread's CPU use over the next
# <seconds> (1), "<tid> <cpu> <% of a CPU> <comm>" a line, as `ps -eLo
# tid,psr,pcpu,comm` prints but over that window: ps's pcpu is a thread's
# average since it started (and the process's total on its main thread's
# line), which no busy thread's placement can be judged by. From
# /proc/<pid>/task/<tid>/stat (utime and stime, in clock ticks; the CPU it
# last ran on).
transport_threads() {
    local s=${1:-1} hz a b
    hz=$(getconf CLK_TCK 2>/dev/null || echo 100)
    a=$(transport_task_times)
    sleep "$s"
    b=$(transport_task_times)
    awk -v hz="$hz" -v s="$s" 'FNR == NR {t[$1] = $2; next} ($1 in t) {printf "%7d %3d %5.1f %s\n", $1, $3, ($2 - t[$1]) * 100 / hz / s, $4}' \
        <(echo "$a") <(echo "$b")
}

# transport_task_times: "<tid> <utime+stime> <cpu> <comm>" for every thread
# now (comm's spaces as _)
transport_task_times() {
    cat "${LAB_PROC_ROOT:-/proc}"/[0-9]*/task/[0-9]*/stat 2>/dev/null | awk '{
        i = index($0, " ("); j = length($0); while (j > 0 && substr($0, j, 1) != ")") j--
        comm = substr($0, i + 2, j - i - 2); gsub(/ /, "_", comm)
        n = split(substr($0, j + 2), f, " ")
        if (n >= 37) print $1, f[12] + f[13], f[37], comm
    }'
}

# transport_placement <isolated cpulist>: transport_threads' lines (or
# `ps -eLo tid,psr,pcpu,comm`'s, no header) on stdin; "ok <n> busy threads" when every busy thread (50 % of a
# CPU or more) ran on an isolated CPU, no two on one, else "FAIL <what>", 1
transport_placement() {
    awk -v iso=",$1," '
        $3 + 0 >= 50 {
            n++
            if (index(iso, "," $2 ",") == 0) bad = bad sprintf(" %s(%s) on CPU %s, not isolated;", $4, $1, $2)
            if ($2 in on) bad = bad sprintf(" %s(%s) and %s on CPU %s;", $4, $1, on[$2], $2)
            on[$2] = $4 "(" $1 ")"
        }
        END {
            if (bad != "") {print "FAIL" bad; exit 1}
            print "ok " n + 0 " busy threads"
        }'
}

# ---- Per-point events (transport.sh around every point: vm.sh and
# local-vm.sh events-start and events-stop; transport_events.py reads what
# events_stop prints into <host>-events-<tag>.csv) ----
# Everything is set up before the point's binaries start, so before their
# warm-up, and read back after its window: enabling a tracepoint or a kprobe
# patches kernel text (an IPI to every CPU), never inside a window. During
# the window only the sampler (on the housekeeping CPU) and the gap spinners
# (on hot CPUs the point leaves idle) run, and the trace records.

# events_trace_start <instance> <cpus> <buffer KiB>: as root, a tracefs
# instance of its own tracing <cpus> (a list: 4,6,8) on CLOCK_MONOTONIC:
# context switches, IRQs and softirqs with their exits, IPIs (x86's
# irq_vectors with their exits, arm64's ipi_entry and ipi_exit) and page
# faults (x86's exceptions tracepoints, else a kprobe and a kretprobe on
# handle_mm_fault). Prints "events: ..." lines: what is on, and what this
# kernel lacks ("unavailable"), which the events file keeps as notes.
events_trace_start() {
    local name=$1 cpus=$2 kb=$3 t=$LAB_SYS_ROOT/kernel/tracing i e c mask=0 missing= pf=
    i=$t/instances/$name
    # tracefs where nothing mounted it yet (a container's /sys)
    [[ -e $t/trace ]] || mount -t tracefs nodev "$t" 2>/dev/null || true
    [[ -e $t/trace ]] || { echo "events: trace unavailable: no tracefs at $t"; return 0; }
    [[ ! -d $i ]] || events_trace_stop "$name" >/dev/null 2>&1
    mkdir "$i" 2>/dev/null || { echo "events: trace unavailable: cannot create the tracefs instance $i"; return 0; }
    echo 0 >"$i/tracing_on"
    for c in ${cpus//,/ }; do mask=$((mask | (1 << c))); done
    printf '%x' "$mask" >"$i/tracing_cpumask"
    echo "$kb" >"$i/buffer_size_kb"
    if grep -qw mono "$i/trace_clock" 2>/dev/null; then
        echo mono >"$i/trace_clock"
    else
        echo "events: trace unavailable: no mono trace clock in [$(cat "$i/trace_clock" 2>/dev/null)], so no wall clock for it"
    fi
    for e in sched/sched_switch irq/irq_handler_entry irq/irq_handler_exit irq/softirq_entry irq/softirq_exit; do
        if [[ -e $i/events/$e/enable ]]; then echo 1 >"$i/events/$e/enable"; else missing+=" $e"; fi
    done
    if [[ -d $i/events/irq_vectors ]]; then
        for e in "$i"/events/irq_vectors/*_entry "$i"/events/irq_vectors/*_exit; do
            [[ -e $e/enable ]] && echo 1 >"$e/enable"
        done
    elif [[ -e $i/events/ipi/ipi_entry/enable ]]; then
        echo 1 >"$i/events/ipi/ipi_entry/enable"
        [[ -e $i/events/ipi/ipi_exit/enable ]] && echo 1 >"$i/events/ipi/ipi_exit/enable"
    else
        missing+=" irq_vectors/ipi_entry (IPIs)"
    fi
    if [[ -e $i/events/exceptions/page_fault_user/enable ]]; then
        echo 1 >"$i/events/exceptions/page_fault_user/enable"
        [[ -e $i/events/exceptions/page_fault_kernel/enable ]] && echo 1 >"$i/events/exceptions/page_fault_kernel/enable"
        pf="the exceptions tracepoints"
    elif grep -qw handle_mm_fault "$t/available_filter_functions" 2>/dev/null; then
        e=ergon_pf_${name//[^A-Za-z0-9]/_}
        { echo "p:$e handle_mm_fault" >>"$t/kprobe_events" && echo "r:${e}_ret handle_mm_fault" >>"$t/kprobe_events"; } 2>/dev/null
        if [[ -e $i/events/kprobes/$e/enable && -e $i/events/kprobes/${e}_ret/enable ]]; then
            echo 1 >"$i/events/kprobes/$e/enable" && echo 1 >"$i/events/kprobes/${e}_ret/enable" && pf="a kprobe on handle_mm_fault"
        fi
    fi
    [[ -n $pf ]] || echo "events: page faults unavailable: no exceptions tracepoints and no kprobe on handle_mm_fault"
    [[ -z $missing ]] || echo "events: trace events unavailable:$missing"
    echo 1 >"$i/tracing_on"
    echo "events: trace on cpus $cpus, $kb KiB a CPU, page faults by ${pf:-nothing}"
}

# events_trace_dump <instance> <cpus>: as root, the instance's events, then
# "# overrun cpu<n> <events lost>" for each traced CPU whose buffer filled
events_trace_dump() {
    local i=$LAB_SYS_ROOT/kernel/tracing/instances/$1 c n
    [[ -d $i ]] || return 0
    echo 0 >"$i/tracing_on"
    grep -v '^#' "$i/trace" 2>/dev/null
    for c in ${2//,/ }; do
        n=$(awk '/^overrun:/ {print $2}' "$i/per_cpu/cpu$c/stats" 2>/dev/null)
        [[ -z $n || $n == 0 ]] || echo "# overrun cpu$c $n"
    done
    return 0
}

# events_trace_stop <instance>: as root, its events off, its kprobes
# removed, the instance removed
events_trace_stop() {
    local name=$1 t=$LAB_SYS_ROOT/kernel/tracing e
    [[ -d $t/instances/$name ]] || return 0
    echo 0 >"$t/instances/$name/tracing_on"
    echo 0 >"$t/instances/$name/events/enable" 2>/dev/null
    e=ergon_pf_${name//[^A-Za-z0-9]/_}
    # (kprobe_events lists them as p:kprobes/<name> ...)
    if grep -qE "[:/]$e " "$t/kprobe_events" 2>/dev/null; then
        { echo "-:${e}_ret" >>"$t/kprobe_events"; echo "-:$e" >>"$t/kprobe_events"; } 2>/dev/null
    fi
    rmdir "$t/instances/$name" 2>/dev/null || true
}

# events_tcp_counters: this host's TCP loss counters now, one
# "<name>=<value>;..." line (/proc/net/snmp and netstat, as nstat names them
# less the protocol): retransmitted segments, RTOs, tail-loss probes, DSACKs
# received. Host-wide: every TCP connection's, the bench's among them.
events_tcp_counters() {
    awk '/^(Tcp|TcpExt):/ {
            if (!($1 in hdr)) { hdr[$1] = 1; for (i = 2; i <= NF; i++) name[$1, i] = $i; next }
            for (i = 2; i <= NF; i++) v[$1 name[$1, i]] = $i
        }
        END {
            printf "RetransSegs=%d;TCPTimeouts=%d;TCPLossProbes=%d;TCPDSACKRecv=%d;TCPDSACKOfoRecv=%d\n",
                v["Tcp:RetransSegs"], v["TcpExt:TCPTimeouts"], v["TcpExt:TCPLossProbes"], v["TcpExt:TCPDSACKRecv"],
                v["TcpExt:TCPDSACKOfoRecv"]
        }' "${LAB_PROC_ROOT:-/proc}/net/snmp" "${LAB_PROC_ROOT:-/proc}/net/netstat" 2>/dev/null
}

# events_sampler <stop file> <aeron dir|-> <counters binary|-> <iface>...:
# one line a second until <stop file> exists: "t=<CLOCK_REALTIME ns>
# tc=<iface>:<sent>:<dropped>,... tcp=<events_tcp_counters> aeron=<counters
# loss>" (tc: each root netem qdisc's tc -s, - with none; aeron: the media
# driver's loss counters read from its CnC file, - for an arm without one).
# events_start runs it on the housekeeping CPU.
events_sampler() {
    local stop=$1 adir=$2 counters=$3 tc aer
    shift 3
    while [[ ! -e $stop ]]; do
        tc=$(netem_stats "$@" | awk '{printf "%s%s:%s:%s", (NR > 1 ? "," : ""), $1, $2, $3}')
        aer=-
        [[ $adir == - || $counters == - ]] || aer=$(AERON_DIR=$adir "$counters" loss 2>/dev/null) || aer=-
        echo "t=$(date +%s%N) tc=${tc:--} tcp=$(events_tcp_counters) aeron=${aer:--}"
        sleep 1
    done
}

# events_clock: "<CLOCK_MONOTONIC ns> <CLOCK_REALTIME ns>", read together:
# the trace's clock (mono) onto the wall clock, at the start and at the end
events_clock() {
    python3 -c 'import time; m = time.clock_gettime_ns(time.CLOCK_MONOTONIC); r = time.time_ns(); print(m, r)'
}

# events_start <dir> <instance> <housekeeping cpu> <traced cpus> <spinner
# cpus|-> <gapspin> <threshold us> <aeron dir|-> <counters|-> <buffer KiB>
# <iface>...: a point's events begun in <dir> (made anew): the trace
# (events_trace_start, as root), osnoise on the traced CPUs where the kernel
# has it (knoise_start where the caller defines it; its line, or why not),
# the clocks, a gap spinner on each spinner CPU and the sampler on the
# housekeeping CPU. Their PIDs in <dir>; "events: ..." notes in <dir>/notes.
events_start() {
    local d=$1 name=$2 hk=$3 cpus=$4 spin=$5 gapspin=$6 us=$7 adir=$8 counters=$9 kb=${10} c
    shift 10
    rm -rf "$d" && mkdir -p "$d" || return 1
    echo "$cpus" >"$d/cpus"
    sudo env LAB_SYS_ROOT="$LAB_SYS_ROOT" bash -c "$(declare -f events_trace_start events_trace_stop); events_trace_start $name $cpus $kb" >"$d/notes" 2>&1
    if declare -F knoise_start >/dev/null; then
        sudo env LAB_SYS_ROOT="$LAB_SYS_ROOT" bash -c "$(declare -f knoise_start); knoise_start $cpus" 2>&1 | sed 's/^/events: /' >>"$d/notes"
    else
        echo "events: osnoise not tried: ${EVENTS_NO_OSNOISE:-this host runs no osnoise}" >>"$d/notes"
    fi
    events_clock >"$d/clock"
    if [[ $spin != - ]]; then
        for c in ${spin//,/ }; do
            nohup "$gapspin" "$c" "$us" >"$d/gapspin-$c.log" 2>&1 </dev/null &
            echo $! >>"$d/spin.pids"
        done
    else
        echo "events: gap spinner unavailable: every hot CPU busy in this point" >>"$d/notes"
    fi
    nohup taskset -c "$hk" bash -c "$(declare -f events_sampler events_tcp_counters netem_stats); events_sampler $d/stop $adir $counters $*" \
        >"$d/samples" 2>&1 </dev/null &
    echo $! >"$d/sampler.pid"
    echo "events: started in $d ($(grep -c . "$d/notes") notes)"
}

# events_stop <dir> <instance> <traced cpus>: the point's events, sections a
# reader splits on (== clock, == notes, == gapspin <cpu>, == trace,
# == osnoise, == samples), then everything events_start began stopped and
# <dir> removed
events_stop() {
    local d=$1 name=$2 cpus=$3 f p
    [[ -d $d ]] || { echo "== notes"; echo "events: not started (no $d)"; return 1; }
    [[ -s $d/cpus ]] && cpus=$(cat "$d/cpus")
    : >"$d/stop"
    p=$(cat "$d/sampler.pid" 2>/dev/null)
    for _ in $(seq 30); do [[ -n $p ]] && kill -0 "$p" 2>/dev/null || break; sleep 0.1; done
    [[ -z $p ]] || kill "$p" 2>/dev/null || true
    while read -r p; do kill "$p" 2>/dev/null || true; done < <(cat "$d/spin.pids" 2>/dev/null)
    events_clock >>"$d/clock"
    echo "== clock"
    cat "$d/clock"
    echo "== notes"
    cat "$d/notes"
    for f in "$d"/gapspin-*.log; do
        [[ -e $f ]] || continue
        f=${f##*/gapspin-}
        echo "== gapspin ${f%.log}"
        cat "$d/gapspin-$f"
    done
    echo "== trace"
    sudo env LAB_SYS_ROOT="$LAB_SYS_ROOT" bash -c "$(declare -f events_trace_dump); events_trace_dump $name $cpus"
    echo "== osnoise"
    if declare -F knoise_stop >/dev/null; then
        sudo grep -v '^#' "$LAB_SYS_ROOT/kernel/tracing/instances/knoise/trace" 2>/dev/null || true
        sudo env LAB_SYS_ROOT="$LAB_SYS_ROOT" bash -c "$(declare -f knoise_stop); knoise_stop" >/dev/null 2>&1
    fi
    echo "== samples"
    cat "$d/samples"
    sudo env LAB_SYS_ROOT="$LAB_SYS_ROOT" bash -c "$(declare -f events_trace_stop); events_trace_stop $name" >/dev/null 2>&1
    rm -rf "$d"
}
