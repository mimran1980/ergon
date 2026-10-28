#!/usr/bin/env bash
# vm.sh's transport commands for the local smoke (transport.sh smoke,
# LAB_LOCAL_SMOKE=1): hosts a and b are homes on one Linux box (HOME names the
# host: .../hosts/a or .../hosts/b), the data NICs loopback addresses on lo,
# one media driver for both hosts (the box's), the box's own kernel settings,
# netem on lo. host.sh's functions do the work, as on a VM.
#
#   local-vm.sh transport-tune        this host's data addresses on lo, the sysctls of
#                                     transport-settings.env written and read back
#   local-vm.sh transport-map         "<housekeeping> <irq> <isolated,...>": the box's online CPUs,
#                                     the first housekeeping, the second IRQ, the rest isolated
#                                     (LOCAL_SMOKE_MAP to set one); 2 with fewer than eight
#   local-vm.sh nics                  vm.sh nics' lines: sender and receiver on lo
#                                     (a: 127.0.1.1 and 127.0.1.2, b: 127.0.2.1 and 127.0.2.2)
#   local-vm.sh hostcheck [aeron dir] the real hostcheck on the map's CPUs (exit as it says)
#   local-vm.sh aeron-page            the page size of the smoke's Aeron directory's filesystem
#                                     (the smoke runs no arm on a hugetlbfs)
#   local-vm.sh driver <mode>         kernel-busy...: the box's media driver (one for both hosts:
#                                     started by the first, a no-op while it runs), DEDICATED, its
#                                     sender, receiver and conductor on the map's first three isolated
#                                     CPUs (host.sh transport_layout), busy-spinning; dpdk refused
#   local-vm.sh stop                  the media driver stopped (if running)
#   local-vm.sh transport-net [check] the sysctls written (check: read) back; an IRQ core idle
#                                     cap (TR_IRQ_IDLE other than all) refused: the box's idle
#                                     is not the smoke's to set
#   local-vm.sh netem set <pct>|clear|check <pct|none>|stats   netem on lo
#   local-vm.sh sample | irqsnap | nstat | settings | napi   as vm.sh's (napi: lo's own)
#   local-vm.sh idle | nicstats       as vm.sh's lines, with nothing in them: no cpuidle state
#                                     (so no cap: all), lo has no interrupt moderation or counters
#   local-vm.sh events-start <point> <traced cpus> <spinner cpus|-> [aeron] | events-stop <point>   as vm.sh's,
#                                     each host its own trace instance; osnoise host a's alone
set -euo pipefail
export PATH=/usr/local/sbin:/usr/sbin:/sbin:$PATH
self=$(readlink -f "$0")
here=$(dirname "$self")
# shellcheck source=irqs.sh
. "$here/irqs.sh"
# shellcheck source=host.sh
. "$here/host.sh"
# shellcheck source=transport-settings.env
. "$here/transport-settings.env"
transport_overrides || exit 2
TR_AERON_DIR=${LOCAL_SMOKE_AERON_DIR:-/dev/shm/aeron-transport-smoke}
run=$HOME/bench
mkdir -p "$run"
shared=${LOCAL_SMOKE_ROOT:?LOCAL_SMOKE_ROOT: the smoke directory}/shared
mkdir -p "$shared"
host=${HOME##*/}
tools=$HOME/ergon/target/release

map() {
    if [[ -n ${LOCAL_SMOKE_MAP:-} ]]; then echo "$LOCAL_SMOKE_MAP"; return; fi
    local cpus
    read -ra cpus < <(cpulist_expand "$(cat /sys/devices/system/cpu/online)")
    ((${#cpus[@]} >= 8)) || { echo "local-vm: ${#cpus[@]} CPUs: the smoke needs eight (housekeeping, IRQ, the driver's three, three for the clients)" >&2; return 2; }
    echo "${cpus[0]} ${cpus[1]} $(IFS=,; echo "${cpus[*]:2}")"
}

nics() {
    local n=1
    [[ $host == b ]] && n=2
    printf 'sender lo lo 00:00:00:00:00:00 127.0.%s.1 8 -\nreceiver lo lo 00:00:00:00:00:00 127.0.%s.2 8 -\n' "$n" "$n"
}

driver_pid() { cat "$shared/driver.pid" 2>/dev/null; }
driver_up() { local p; p=$(driver_pid) && [[ -n $p ]] && kill -0 "$p" 2>/dev/null; }

case ${1:-} in
transport-tune)
    # the hosts' data addresses on lo (Aeron's interface= finds an interface
    # by its address; 127/8 routes to lo without being one of its addresses)
    nics | while read -r _ _ _ _ ip _; do ip -4 addr show dev lo | grep -q " $ip/" || ip addr add "$ip/32" dev lo; done
    transport_sysctls_apply && transport_ports_check "$TR_PORT_BASE" "$TR_PORT_SPAN"
    ;;
transport-map) map ;;
nics) nics ;;
hostcheck)
    read -r _ _ iso < <(map)
    "$tools/hostcheck" --hot-cpus "$iso" --nic lo --aeron-dir "${2:-$TR_AERON_DIR}" --out "$run/hostcheck.txt" && rc=0 || rc=$?
    cat "$run/hostcheck.txt" 2>/dev/null
    exit $rc
    ;;
driver)
    mode=${2:?mode}
    [[ $mode == dpdk* ]] && { echo "local-vm: no DPDK in the smoke" >&2; exit 2; }
    if driver_up; then echo "driver: the box's media driver runs (pid $(driver_pid))"; exit 0; fi
    read -r hk app sender receiver app2 < <(transport_layout "$(map)")
    rm -rf "$TR_AERON_DIR"
    (
        export AERON_DIR=$TR_AERON_DIR RUSTERON_MEDIA_DRIVER_DIR=$TR_AERON_DIR AERON_DIR_DELETE_ON_START=true
        export AERON_TERM_BUFFER_SPARSE_FILE=false AERON_TERM_BUFFER_LENGTH=$TR_TERM_LENGTH AERON_IPC_TERM_BUFFER_LENGTH=$TR_TERM_LENGTH
        export AERON_THREADING_MODE=DEDICATED AERON_CONDUCTOR_CPU_AFFINITY=$app2 AERON_SENDER_CPU_AFFINITY=$sender
        export AERON_RECEIVER_CPU_AFFINITY=$receiver AERON_CONDUCTOR_IDLE_STRATEGY=spin AERON_SENDER_IDLE_STRATEGY=noop
        export AERON_RECEIVER_IDLE_STRATEGY=noop AERON_MTU_LENGTH=$TR_MTU AERON_SOCKET_SO_SNDBUF=2097152
        export AERON_SOCKET_SO_RCVBUF=2097152 AERON_RCV_INITIAL_WINDOW_LENGTH=2097152
        export AERON_SENDER_WILDCARD_PORT_RANGE='20000 20999' AERON_RECEIVER_WILDCARD_PORT_RANGE='21000 21999'
        export ERGON_HOST_CHECKS=warn
        # the mode's own driver settings (a tuning variant's: the socket
        # buffer, the NAK delay, SHARED threading on the sender's CPU)
        for kv in ${mode//@/ }; do
            case $kv in
            AERON_THREADING_MODE=SHARED) export AERON_THREADING_MODE=SHARED AERON_CONDUCTOR_CPU_AFFINITY=$sender AERON_SHARED_IDLE_STRATEGY=noop ;;
            AERON_*=* | RUSTERON_*=*) export "${kv?}" ;;
            esac
        done
        exec nohup taskset -c "$hk" "$tools/media_driver" </dev/null >"$shared/driver.log" 2>&1
    ) &
    echo $! >"$shared/driver.pid"
    for _ in $(seq 100); do
        grep -q "media driver started" "$shared/driver.log" 2>/dev/null && break
        driver_up || { cat "$shared/driver.log" >&2; exit 1; }
        sleep 0.2
    done
    grep -q "media driver started" "$shared/driver.log" || { cat "$shared/driver.log" >&2; exit 1; }
    # kernel-busy's busy poll; kernel (no busy poll) 0, as vm.sh driver has them
    busy=$TR_BUSY_READ
    [[ ${mode%%@*} == kernel ]] && busy=0
    sysctl -qw net.core.busy_read="$busy" net.core.busy_poll="$busy"
    echo "driver: $mode, the box's (pid $(driver_pid)): conductor $app2, sender $sender, receiver $receiver (app $app unused)"
    ;;
stop)
    if driver_up; then
        p=$(driver_pid)
        kill -TERM "$p"
        for _ in $(seq 50); do kill -0 "$p" 2>/dev/null || break; sleep 0.1; done
        kill -KILL "$p" 2>/dev/null || true
    fi
    rm -f "$shared/driver.pid"
    ;;
transport-net)
    if [[ ${2:-} == check ]]; then transport_sysctls_apply check >/dev/null && echo "transport-net: settings hold"; exit; fi
    [[ $TR_IRQ_IDLE == all ]] || { echo "transport-net: irq_idle $TR_IRQ_IDLE: the local smoke caps no idle state" >&2; exit 2; }
    transport_cc_load
    transport_sysctls_apply
    ;;
cc-available)
    modprobe tcp_bbr 2>/dev/null || true
    cat /proc/sys/net/ipv4/tcp_available_congestion_control
    ;;
netem)
    case ${2:-} in
    set) netem_set "${3:?loss %}" lo && netem_check "$3" lo ;;
    clear) netem_clear lo && netem_check none lo ;;
    check) netem_check "${3:?loss % or none}" lo ;;
    stats) netem_stats lo ;;
    show) netem_show lo ;;
    *) echo "netem set <loss %> | clear | check <loss %|none> | stats" >&2; exit 2 ;;
    esac
    ;;
sample)
    read -r _ _ iso < <(map)
    transport_threads 1 >"$run/sample.ps"
    transport_placement "$iso" <"$run/sample.ps" || true
    cat "$run/sample.ps"
    ;;
irqsnap) cat /proc/interrupts ;;
aeron-page) stat -f -c %S "$TR_AERON_DIR" ;;
napi)
    # vm.sh napi's lines for the data NICs, all lo here: the box's own (lo
    # takes no interrupts; the smoke sets nothing on it)
    nics | while read -r _ _ vf _; do
        echo "$vf $(cat "/sys/class/net/$vf/napi_defer_hard_irqs" 2>/dev/null || echo 0) $(cat "/sys/class/net/$vf/gro_flush_timeout" 2>/dev/null || echo 0)"
    done
    ;;
idle) echo "driver none governor none max_cstate - (local smoke: the box's idle is its own)" ;;
nicstats) nics | while read -r _ _ vf _; do echo "$vf coalesce none"; done ;;
nstat) transport_nstat ;;
settings) transport_settings_read ;;
events-start)
    # as vm.sh's, each host its own trace instance (one kernel for both): host
    # a also runs osnoise (vm.sh's knoise_start) where the kernel has it
    read -r hk _ _ < <(map)
    [[ -x $shared/gapspin ]] || gcc -O2 -o "$shared/gapspin" "$here/gapspin.c" || exit 1
    if [[ $host == a ]]; then
        eval "$(for f in knoise_start knoise_stop; do awk -v f="$f() {" '$0 == f {on = 1} on {print} on && $0 == "}" {exit}' "$here/vm.sh"; done)"
    else
        export EVENTS_NO_OSNOISE="the smoke's two hosts are one kernel: host a's osnoise"
    fi
    adir=-
    [[ ${5:-} == aeron ]] && adir=$TR_AERON_DIR
    counters=-
    [[ -x $tools/counters ]] && counters=$tools/counters
    events_start "$run/events/${2:?point}" "ergon-events-$host" "$hk" "${3:?traced cpus}" "${4:--}" "$shared/gapspin" "$TR_EVENTS_GAPSPIN_US" \
        "$adir" "$counters" "$TR_EVENTS_BUFFER_KB" lo
    ;;
events-stop-all)
    read -r _ _ iso < <(map)
    for d in "$run"/events/*/; do [[ -d $d ]] && events_stop "${d%/}" "ergon-events-$host" "$iso" >/dev/null 2>&1; done
    true
    ;;
events-stop)
    read -r _ _ iso < <(map)
    if [[ $host == a ]]; then
        eval "$(awk -v f='knoise_stop() {' '$0 == f {on = 1} on {print} on && $0 == "}" {exit}' "$here/vm.sh")"
    fi
    events_stop "$run/events/${2:?point}" "ergon-events-$host" "$iso"
    ;;
*)
    awk 'NR > 1 && !/^#/ {exit} NR > 1 {print}' "$self" >&2
    exit 2
    ;;
esac
