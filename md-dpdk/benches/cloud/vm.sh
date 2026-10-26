#!/usr/bin/env bash
# One VM's side of md-dpdk/benches/cloud/{azure,aws}/lab.sh, on Azure (MANA) or AWS
# (ENA): the cloud is read from the instance metadata service. Every command is
# idempotent and runs as the admin user from ~/ergon (sudo where it must).
#
#   vm.sh bootstrap          packages, the backports kernel, hugepages, the cloud's DPDK prerequisites,
#                            on AWS chrony in systemd-timesyncd's place (timesync_setup)
#                            (x86_64; aarch64, AWS Graviton, only with lab.sh's LAB_ALLOW_ARM=1)
#   vm.sh build              the media driver (static, dpdk), the rtt/load/tput/counters clients with
#                            the driver's features (INVOKER embeds it), igb_uio (AWS), purefloor
#                            (purefloor.c, against pkg-config's libdpdk; why not in
#                            ~/bench/purefloor_unavailable, never fatal); on aarch64,
#                            should md-dpdk's dpdk feature not build there, both without dpdk
#                            (DPDK modes then refuse to start, saying so)
#   vm.sh build-ynl          (build, where lib.sh's bench/want_ynl is 1: a mode with @NAPI_BUSY) the
#                            kernel's own netlink tool, ynl, with its netdev spec, from Debian's source
#                            of exactly the running kernel (no Debian package ships ynl); fail closed,
#                            the reason in ~/bench/ynl_unavailable
#   vm.sh build-base         the revision lib.sh unpacked in ~/ergon-base (LAB_BASE): its media
#                            driver and benchmark clients into ~/bin-base, for modes with @BIN=bin-base
#   vm.sh build-dpdk <build> (build, for each build lib.sh's bench/want_dpdk names: the modes'
#                            @DPDK=) DPDK 25.11 from source, stock or patched-refill
#                            (dpdk-ena-deferred-refill.patch on ENA's PMD), under a prefix of its
#                            own, and the media driver and benchmark clients linked against it
#                            (its library directory their RPATH) into ~/bin-dpdk-<build>; AWS only
#   vm.sh isolate [poll]     CPU isolation on the kernel command line (takes effect at the next boot);
#                            poll (the isolated-poll state): idle CPUs poll rather than halt
#   vm.sh tune               after each boot: irqbalance off, every IRQ on the housekeeping CPU, hugetlbfs
#                            AERON_DIR, transparent hugepages off (the baseline); the IRQs are pinned
#                            again wherever the kernel may have placed them anew (irqs.sh)
#   vm.sh baseline           the settings every mode runs on top of (the recommended configuration's
#                            host-wide and DPDK settings; per-mode ones such as @RX_RING=4096 are not in it)
#   vm.sh layout             "<housekeeping> <app> <sender> <receiver> [<app2>]" CPUs: the first thread of
#                            four cores, and of a fifth where the host has one (every Graviton CPU is a core)
#   vm.sh ready <mode>...    what each mode's settings need of this host (lib.sh asks after the build):
#                            a fifth core for @IRQ=spare, @NAPI_BUSY=spare and @CONDUCTOR=own, Linux 6.17
#                            and the kernel's ynl for @NAPI_BUSY, its build for @DPDK= (build-dpdk);
#                            a line for each mode that cannot run here, exit 2
#   vm.sh host               the architecture and the host settings in effect (lib.sh's env file,
#                            each boot's tune log), and what was skipped as x86-only
#   vm.sh nics               the data NICs from the cloud's metadata: one line per role
#   vm.sh driver <mode>      start the media driver: kernel, kernel-busy or dpdk; a -mono
#                            suffix clocks the transport and clients with CLOCK_MONOTONIC, not TSC;
#                            @AERON_THREADING_MODE=INVOKER starts none (on Graviton under DPDK one
#                            for its LLQ check, then stops it): everything else is set up, and
#                            each client embeds the driver (vm.sh client); the kernel modes name
#                            the data NICs (RUSTERON_MEDIA_DRIVER_DATA_NICS) for its NIC checks;
#                            the raw floor's kernel-raw and dpdk-raw start none either: kernel-busy's
#                            or dpdk's set-up under INVOKER's threading (one thread sends and
#                            receives), for the floor client (no Aeron), its wire in ~/bench/floor;
#                            the pure floor's dpdk-pure dpdk's set-up, its wire pure, for purefloor
#   vm.sh client <label> <rtt|tput|load|floor|purefloor> <args>   a benchmark client against the running
#                            driver (floor, purefloor: the raw or pure floor's, on the wire vm.sh driver
#                            set up), ended at
#                            its deadline (LAB_CLIENT_DEADLINE seconds, an hour without it; SIGKILL
#                            LAB_CLIENT_GRACE, 10 s, after SIGTERM): exit 124 then
#   vm.sh stop               stop the media driver and give the data NICs back to the kernel
#   vm.sh counters           validity counters: VF or allowance counters, data-path switches; under
#                            DPDK also the transport's frames, bytes, refused and queued sends, and
#                            the ENA's per-queue TX statistics
#   vm.sh wc-check           AWS, DPDK: whether the ENA's low-latency queue BAR is mapped write-combining
#   vm.sh pmd-log            the DPDK run's PMD and EAL lines on how the NIC was set up (LLQ, placement)
#   vm.sh errors             the driver's distinct error log and loss report
#   vm.sh loss               one "key=value;..." line of every loss counter: Aeron's (NAKs,
#                            retransmits, short sends, gap fills), the DPDK transport's drops,
#                            the kernel's UDP errors, the data NICs' kernel drop counters and
#                            their missing routes (routes-missing, as found); then a check on
#                            the run that is not loss: the data NICs' interrupts on any CPU but
#                            the housekeeping one (isolated-nic-irqs, irqs.sh; under a mode's @IRQ=
#                            its queue IRQs' on any CPU but the one it chose); last, Aeron's own
#                            telemetry, never loss (info-: the agents' longest cycles and cycles over
#                            threshold, back-pressure, status messages, flow control; host.sh
#                            aeron_telemetry), which lib.sh writes as aeron rows
#   vm.sh clockcheck [secs]  the clocks' read cost and the clients' clock's (quanta's) drift, on the app CPU
#   vm.sh criterion          the clock Criterion bench (benches/clock.rs), on the app CPU
#   vm.sh hostchecks         the media driver's start-up host checks as it printed them (its
#                            `ergon-md-dpdk warning:`/`info:` lines, md-dpdk/src/checks.rs) from this
#                            run's driver.log; none under INVOKER, whose clients print their drivers'
#   vm.sh ptp                the PTP hardware clock: device, devlink's enable_phc, chrony's lock on it,
#                            NIC timestamping
#   vm.sh timesync [check]   chrony's tracking and sources (lib.sh: around every mode); check, under
#                            precision-time, fails unless the PHC is chrony's source within 10 us
#   vm.sh timesync-wait      AWS: chronyc waitsync until chrony has settled (lib.sh: before each state);
#                            1 if it never does
#   vm.sh rx-hw-delay        "<mean ns>,<max ns>,<frames>,<min ns>,<negative>" NIC arrival -> the frame's dispatch on the receiver port
#   vm.sh pmtu <peer ip>...  jumbo frames: the data NICs at the jumbo MTU and the path to each peer
#                            address checked at it (ping -M do), before a JUMBO=1 mode's drivers start
#   vm.sh mtu                "<link MTU> <Aeron MTU>" the running mode uses
#   vm.sh base-rtt <peer ip> the network's own round trip, sender NIC to <peer ip> (ping, 200 a 10 ms):
#                            "<min> <avg> <max> <mdev> (ns) <replies>/<sent>"
#
# The transport suite (transport.sh; settings from transport-settings.env):
#   vm.sh transport-isolate  the CPU map (housekeeping, IRQ, isolated: host.sh transport_cpu_map),
#                            the driver's layout under it, and its boot line from the next boot
#   vm.sh transport-tune     after that boot: SMT off, the boot line checked, the quiet settings,
#                            irqbalance off, the data NICs (rings, coalescing off, quickack routes,
#                            every interrupt on the IRQ CPU) and the sysctls, each read back
#   vm.sh transport-net [check]  a socket arm's kernel path (NAPI, busy poll, TCP), or only read back
#   vm.sh transport-map      the map;  vm.sh transport-build  the bench binaries and LAB_EXTRA_SRC's
#   vm.sh netem set <loss %>|clear|check <loss %|none>|stats   netem on every data NIC's egress
#   vm.sh sample             every thread's CPU, judged against the isolated CPUs (first line)
#   vm.sh irqsnap            /proc/interrupts;  vm.sh hostcheck  the ergon-tune report (1 unless clean)
#   vm.sh nstat              the kernel's protocol counters;  vm.sh settings  every transport sysctl
#                            as it is now, <knob>=<value> (host.sh transport_settings_read)
#   vm.sh events-start <point> <traced cpus> <spinner cpus|-> [aeron]   a point's events begun before
#                            its binaries start (trace of its hot CPUs, osnoise, gap spinners, the 1 s sampler)
#   vm.sh events-stop <point>  its events read back (transport_events.py's input) and stopped
#
# aarch64 (AWS Graviton, lab.sh LAB_ALLOW_ARM=1): Graviton's own settings where
# they differ from x86's (host.sh): builds for the instance's own core,
# iommu.passthrough=1 from bootstrap (metal's SMMU), nohlt for isolated-poll
# (no C-states to cap), 4 KiB pages checked,
# every transparent hugepage size following the top level, receive packet
# steering off, igb_uio's LLQ and write-combining checked at every DPDK start
# (fail closed); x86-only steps are refused or skipped with the reason.
#
# AWS: chrony in place of systemd-timesyncd, on the Amazon Time Sync Service
# (bootstrap, timesync_setup). With ~/bench/placement = precision-time (lab.sh
# LAB_PLACEMENT): DPDK 25.11 from source (ENA RX timestamps), the in-tree ENA
# driver's PTP hardware clock turned on by devlink on the primary ENI at every
# boot (build-ptp, phc_setup), chrony on that clock first, and the DPDK driver
# asks for NIC RX timestamps.
#
# Roles: the data NIC with the lower private IP is the driver's sender port,
# the higher its receiver port (both on the data subnet, DATA_SUBNET).
set -euo pipefail
# ethtool, sysctl and modprobe live in /usr/sbin, off a non-login ssh PATH
export PATH=/usr/local/sbin:/usr/sbin:/sbin:$PATH
self=$(readlink -f "$0")
# irqs_pin, irqs_check and irqs_nic_off: the IRQs kept on the housekeeping CPU
# shellcheck source=irqs.sh
. "$(dirname "$self")/irqs.sh"
# what differs by architecture and threading mode: boot and isolation
# arguments, the CPU layout, the agents' CPUs, the LLQ and IOMMU checks
# shellcheck source=host.sh
. "$(dirname "$self")/host.sh"
# the transport suite's declared settings (TR_*: transport-* commands)
# shellcheck source=transport-settings.env
. "$(dirname "$self")/transport-settings.env"
# one arm's own over them (transport.sh tr_host_overrides: TR_OVERRIDES)
transport_overrides || exit 2
cd "$HOME/ergon"
run=$HOME/bench
mkdir -p "$run"
# The media driver and the benchmark clients (rtt, tput, load): this
# checkout's build, or another build's (an A/B against older code) for one
# run, <mode>@BIN=<dir> (`driver` records it in $run/bin, `stop` removes it),
# or by hand, BENCH_BIN. The diagnostic tools (counters, clockcheck) are
# always this checkout's, so every mode's counters are read alike.
tools=$HOME/ergon/target/release
bin=$(cat "$run/bin" 2>/dev/null || true)
bin=${bin:-${BENCH_BIN:-$tools}}
# The lab's baseline: settings that apply to every mode or every DPDK mode.
# Every mode runs on top of it, and an A/B against it is a mode's override
# (@THP=always, @AERON_RECEIVER_IO_VECTOR_CAPACITY=4). Transparent hugepages
# are host-wide, so every mode's (never: not separated in md-dpdk/BENCHMARKS.md);
# the receiver's IO vectors are DPDK modes' (16: its recommended
# configuration). A setting for one transport only, such as kernel busy
# polling's 4,096-entry receive ring, stays in the mode (@RX_RING=4096).
BASE_THP=never
BASE_RCV_IO_VECTORS=16
log() { echo "[vm $(hostname) $(date +%T)] $*" >&2; }
# lab.sh writes the data subnet here once (ssh passes no environment)
DATA_SUBNET=${DATA_SUBNET:-$(cat "$run/data_subnet" 2>/dev/null || true)}
# the other host's data subnet when it is not this one's (lab.sh, cross-AZ or
# cross-region): reached through this subnet's gateway (host.sh policy_routes)
PEER_SUBNET=$(cat "$run/peer_subnet" 2>/dev/null || true)

# Azure answers its metadata header; anything else here is AWS (IMDSv2).
cloud() {
    if [[ -f $run/cloud ]]; then cat "$run/cloud"; return; fi
    if curl -fsS -m 2 -H Metadata:true "http://169.254.169.254/metadata/instance?api-version=2021-02-01" >/dev/null 2>&1; then
        echo azure | tee "$run/cloud"
    else
        echo aws | tee "$run/cloud"
    fi
}
CLOUD=$(cloud)
ARCH=$(uname -m)
# Graviton: every build for the instance's own core (host.sh build_env)
eval "$(build_env "$ARCH")"
PLACEMENT=$(cat "$run/placement" 2>/dev/null || true)
precision_time() { [[ $CLOUD == aws && $PLACEMENT == precision-time ]]; }
# DPDK 25.11, the first with ENA RX timestamps (Debian 13 has 24.11)
DPDK_VERSION=25.11
# (LAB_OPT_ROOT: /opt; test/test-lab.sh's fake)
DPDK_PREFIX=${LAB_OPT_ROOT:-/opt}/dpdk-$DPDK_VERSION
if precision_time; then
    export PKG_CONFIG_PATH=$DPDK_PREFIX/lib/$ARCH-linux-gnu/pkgconfig
fi

# "<housekeeping> <app> <sender> <receiver> <app2>": a physical core each
# (host.sh cpu_layout), skipping cores with a CPU the pre-flight saw stall
# (vm.sh preflight writes $run/noisy_cpus: on Azure some vCPUs stall for
# milliseconds at one second of every minute)
# While a mode's SMT=off has the twins offline, the layout it was given with
# SMT on (host_setting SMT keeps it): cpu_layout ranks a core by its twins'
# pre-flight stalls, and an offline twin would rank it anew, moving the roles
# off the cores the SMT-on baseline ran on
# The transport suite's (transport-isolate writes it): its CPU map's layout,
# whatever the cores' pre-flight said (host.sh transport_layout)
layout() {
    if [[ -s $run/layout-transport ]]; then
        cat "$run/layout-transport"
        return
    fi
    if [[ -s $run/layout-smt-off && $(cat "$LAB_SYS_ROOT/devices/system/cpu/smt/control" 2>/dev/null) == off ]]; then
        cat "$run/layout-smt-off"
        return
    fi
    cpu_layout "$(cat "$run/noisy_cpus" 2>/dev/null)"
}
read -r HK_CPU APP_CPU SENDER_CPU RECEIVER_CPU APP2_CPU <<<"$(layout)"

# the kernel's netlink tool (vm.sh build-ynl's), as root: host.sh's NAPI
# functions call it (@NAPI_BUSY)
ynl() { sudo "$run/ynl" "$@"; }
# ynl_ready: 0 when build-ynl left the tool here; 1, saying why not
ynl_ready() {
    [[ -x $run/ynl && ! -s $run/ynl_unavailable ]] && return 0
    echo "no ynl, the kernel's netlink tool (vm.sh build-ynl): $(cat "$run/ynl_unavailable" 2>/dev/null ||
        echo "not built (vm.sh build fetches it where lib.sh's bench/want_ynl is 1: a mode with @NAPI_BUSY)")" >&2
    return 1
}

# AERON_DIR on 2 MiB hugetlbfs, as rusteron's tuned runs (vm.sh tune mounts it);
# LAB_AERON_DIR in its place for the transport suite (transport.sh: TR_AERON_DIR)
export AERON_DIR=${LAB_AERON_DIR:-/mnt/huge/aeron-bench}

# invoker: whether the running mode embeds the driver in each client (`driver`
# <mode>@AERON_THREADING_MODE=INVOKER leaves ~/bench/harness_driver; `stop`
# removes it)
invoker() { [[ $(cat "$run/harness_driver" 2>/dev/null) == invoker ]]; }
# driver_cnc: whether a driver's counters can be read: a media driver
# running, or under INVOKER the last embedded driver's CnC file, which stays
# after its client exits until the next client's driver replaces it
driver_cnc() { pgrep -x media_driver >/dev/null || { invoker && sudo test -e "$AERON_DIR/cnc.dat"; }; }
# cnc_now: the embedded driver's name in AERON_DIR now (host.sh
# cnc_identity, from a copy of the CnC file's metadata); 1 with none
cnc_now() {
    sudo test -e "$AERON_DIR/cnc.dat" || return 1
    sudo head -c 64 "$AERON_DIR/cnc.dat" >"$run/cnc.head" && cnc_identity "$run/cnc.head"
}
# The driver's settings, as `export K='v'` lines the driver's shell sources;
# common_env [threading mode]: the agents' threads, CPUs and idle strategies
# for that Aeron threading mode (DEDICATED unless a mode says otherwise:
# host.sh threading_env), and two of the duty-cycle thresholds Aeron counts
# each agent's stalls against (Aeron 1.53.3's C names and duration syntax,
# aeron_driver_context.c: AERON_DRIVER_*_CYCLE_THRESHOLD, 100 ms each by
# default): the sender's and receiver's 50 us. The conductor's is NOT set
# here. It has to sit strictly above whatever park the idle strategy left in
# play takes, so it is derived after the mode's overrides
# (conductor_cycle_threshold_apply). `loss` reads them back with the agents'
# longest cycles (host.sh aeron_telemetry)
common_env() {
    local threads
    threads=$(threading_env "${1:-DEDICATED}" "$HK_CPU" "$SENDER_CPU" "$RECEIVER_CPU") || return 2
    cat <<ENV
export AERON_DIR='$AERON_DIR'
export AERON_DIR_DELETE_ON_START='true'
export AERON_FILE_PAGE_SIZE='2097152'
export AERON_TERM_BUFFER_SPARSE_FILE='false'
$threads
export AERON_SENDER_WILDCARD_PORT_RANGE='20000 20999'
export AERON_RECEIVER_WILDCARD_PORT_RANGE='21000 21999'
export AERON_MTU_LENGTH='1408'
export AERON_SOCKET_SO_SNDBUF='2097152'
export AERON_SOCKET_SO_RCVBUF='2097152'
export AERON_RCV_INITIAL_WINDOW_LENGTH='2097152'
export AERON_DRIVER_SENDER_CYCLE_THRESHOLD='50us'
export AERON_DRIVER_RECEIVER_CYCLE_THRESHOLD='50us'
export RUSTERON_MEDIA_DRIVER_DIR='$AERON_DIR'
ENV
}

# duration_ns <value>: Aeron's aeron_parse_duration_ns
# (aeron_parse_util.c): a bare number is nanoseconds, and the suffixes s, ms,
# us and ns (any case) scale it. Integers only, and no more than 18 digits,
# so the arithmetic below cannot overflow; anything else fails.
duration_ns() {
    local v=${1-} unit n lim
    [[ $v =~ ^[0-9]+[A-Za-z]*$ ]] || return 1
    unit=${v##*[0-9]}
    n=${v%"$unit"}
    unit=$(printf '%s' "$unit" | tr '[:upper:]' '[:lower:]')
    ((${#n} <= 18)) || return 1
    case $unit in
    '' | ns) lim=9223372036854775807 ;;
    us) lim=9223372036854775 ;;
    ms) lim=9223372036 ;;
    s) lim=9223372036 ;;
    *) return 1 ;;
    esac
    ((10#$n <= lim)) || return 1
    case $unit in
    '' | ns) printf '%s\n' "$((10#$n))" ;;
    us) printf '%s\n' "$((10#$n * 1000))" ;;
    ms) printf '%s\n' "$((10#$n * 1000000))" ;;
    s) printf '%s\n' "$((10#$n * 1000000000))" ;;
    esac
}

# format_duration_ns <ns>: Aeron's aeron_format_duration_ns
# (aeron_parse_util.c) — the largest unit it divides by exactly.
format_duration_ns() {
    local ns=$1
    if ((ns >= 1000000000 && ns % 1000000000 == 0)); then
        printf '%ss\n' "$((ns / 1000000000))"
    elif ((ns >= 1000000 && ns % 1000000 == 0)); then
        printf '%sms\n' "$((ns / 1000000))"
    elif ((ns >= 1000 && ns % 1000 == 0)); then
        printf '%sus\n' "$((ns / 1000))"
    else
        printf '%sns\n' "$ns"
    fi
}

# conductor_max_park_ns <idle strategy> [init args]: the longest park that
# strategy's idle can take, in nanoseconds — Aeron's backoff defaults
# (aeron_agent.h AERON_IDLE_STRATEGY_BACKOFF_MAX_PARK_PERIOD_NS, 1 ms) with
# no init args, the fourth of backoff's "maxSpins-maxYields-minPark-maxPark"
# (aeron_agent.c aeron_idle_strategy_backoff_state_init_args), and 0 for the
# strategies that never park (spin, noop). Anything else fails: a park this
# lab cannot read is not a park it can place a threshold above.
conductor_max_park_ns() {
    local strategy=${1-} args=${2-}
    case $strategy in
    spin | noop)
        printf '0\n'
        return 0
        ;;
    backoff) ;;
    *) return 1 ;;
    esac
    if [[ -z $args ]]; then
        printf '1000000\n'
        return 0
    fi
    [[ $args =~ ^[0-9]+-[0-9]+-[0-9A-Za-z]+-([0-9A-Za-z]+)$ ]] || return 1
    duration_ns "${BASH_REMATCH[1]}"
}

# conductor_cycle_threshold <idle strategy> [init args]: the value for
# AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD. Aeron's tracker reports a cycle
# only when it is STRICTLY longer than the threshold
# (aeron_duty_cycle_tracker.h: cycle_time_ns > threshold_ns), so a threshold
# at or below the strategy's longest park turns every full park into a
# reported stall and its "max cycle" into a nanosleep oversleep — what the
# lab's fixed 1ms did to backoff's 1ms. Twice the park, and never below
# 1 ms, so a spin or noop conductor still only reports real work.
conductor_cycle_threshold() {
    local park want
    park=$(conductor_max_park_ns "$@") || return 1
    ((park <= 4611686018427387903)) || return 1
    want=$((park * 2))
    if ((want < 1000000)); then want=1000000; fi
    format_duration_ns "$want"
}

# conductor_cycle_threshold_apply <env file>: the conductor's duty-cycle
# threshold for this run, written into the environment the driver (or
# INVOKER's clients) will source if it does not already name one. Called
# after the mode's @KEY=value overrides and before anything starts, because
# the threshold has to clear the park of whatever idle strategy those
# overrides left in play. An explicit threshold that does not is refused
# before the driver starts, naming both values and the one to use.
# override_export <KEY=value>: a mode's override as the driver environment's
# export line. A mode's name cannot hold a comma (it would split its
# results.csv rows: modes.sh mode_check), so RUSTERON_DPDK_ENA_DEVARGS, a
# comma-separated list of ENA device arguments, is written with + between
# them, as SUB_PARAMS and PUB_PARAMS are: @RUSTERON_DPDK_ENA_DEVARGS=
# miss_txc_to=0+control_path_poll_interval=1000.
override_export() {
    local key=${1%%=*} value=${1#*=}
    [[ $key == RUSTERON_DPDK_ENA_DEVARGS ]] && value=${value//+/,}
    echo "export $key='$value'"
}

conductor_cycle_threshold_apply() {
    local env_file=${1-} got strategy args threshold park want have
    [[ -f $env_file ]] || return 2
    # the file is `export K='v'` lines the driver's shell sources: read it
    # that way, so a later line wins as it will at start
    got=$(
        set +eu
        # shellcheck disable=SC1090
        . "$env_file" >/dev/null 2>&1 || exit 2
        printf '%s|%s|%s\n' \
            "${AERON_CONDUCTOR_IDLE_STRATEGY-backoff}" \
            "${AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS-}" \
            "${AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD-}"
    ) || return 2
    IFS='|' read -r strategy args threshold <<<"$got" || true
    if ! park=$(conductor_max_park_ns "$strategy" "$args"); then
        if [[ $strategy == backoff ]]; then
            echo "conductor: AERON_CONDUCTOR_IDLE_STRATEGY='$strategy' AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS='$args': the park is not readable (backoff's init args are maxSpins-maxYields-minPark-maxPark, e.g. 100-10-1us-20us)" >&2
        else
            echo "conductor: AERON_CONDUCTOR_IDLE_STRATEGY='$strategy': not one whose longest park this lab knows (backoff, spin, noop)" >&2
        fi
        return 2
    fi
    want=$(conductor_cycle_threshold "$strategy" "$args") || return 2
    if [[ -n $threshold ]]; then
        if ! have=$(duration_ns "$threshold"); then
            echo "conductor: AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='$threshold': not an Aeron duration (bare ns, or the suffixes s, ms, us, ns)" >&2
            return 2
        fi
        if ((have <= park)); then
            echo "conductor: AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='$threshold' ($have ns) is not above $strategy's longest park ($park ns): every full park would count as a stall. Use '$want' or leave it unset." >&2
            return 2
        fi
        return 0
    fi
    echo "export AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='$want'" >>"$env_file"
}

# "<mac> <ip> <prefix>" of each data NIC, lowest address first, from the
# metadata service once (with retries: one empty answer from it killed a run),
# then from $run/data_macs: an instance's NICs never change.
data_macs() {
    local subnet=${DATA_SUBNET:?DATA_SUBNET: the data subnet, e.g. 10.20.1.0/24} lines
    if [[ -s $run/data_macs && $(wc -l <"$run/data_macs") -eq 2 ]]; then
        cat "$run/data_macs"
        return
    fi
    # a failed lookup must reach nics()'s message, not exit here under set -e
    lines=$(data_macs_imds "$subnet") || true
    if [[ $(wc -l <<<"$lines") -eq 2 && $lines != *"  "* ]]; then printf '%s\n' "$lines" >"$run/data_macs"; fi
    printf '%s\n' "$lines"
}
data_macs_imds() {
    local subnet=$1
    local get=(curl -fsS -m 5 --retry 5 --retry-all-errors --retry-delay 1)
    case $CLOUD in
    azure)
        "${get[@]}" -H Metadata:true "http://169.254.169.254/metadata/instance/network?api-version=2021-02-01&format=json" |
            python3 -c '
import ipaddress, json, sys
net = ipaddress.ip_network(sys.argv[1])
for i in json.load(sys.stdin)["interface"]:
    mac = ":".join(i["macAddress"][k:k + 2] for k in range(0, 12, 2)).lower()
    for a in i["ipv4"]["ipAddress"]:
        if ipaddress.ip_address(a["privateIpAddress"]) in net:
            print(mac, a["privateIpAddress"], net.prefixlen)
' "$subnet"
        ;;
    aws)
        local token mac
        token=$("${get[@]}" -X PUT -H "X-aws-ec2-metadata-token-ttl-seconds: 300" http://169.254.169.254/latest/api/token)
        local md=http://169.254.169.254/latest/meta-data/network/interfaces/macs
        for mac in $("${get[@]}" -H "X-aws-ec2-metadata-token: $token" "$md/" | tr -d /); do
            local ip
            ip=$("${get[@]}" -H "X-aws-ec2-metadata-token: $token" "$md/$mac/local-ipv4s" | head -1)
            [[ -n $ip ]] || { echo "metadata: no IPv4 for $mac" >&2; continue; }
            python3 -c 'import ipaddress,sys; sys.exit(ipaddress.ip_address(sys.argv[1]) not in ipaddress.ip_network(sys.argv[2]))' \
                "$ip" "$subnet" && echo "$mac $ip ${subnet#*/}"
        done
        ;;
    esac | sort -t. -k4,4n
}

# "<role> <iface> <vf iface> <mac> <ip> <prefix> <bdf>" for the two data NICs.
# Azure: the netvsc interface has the MAC and no master, and its MANA VF is
# enslaved to it. AWS: the ENA is the interface and the PCI device both.
nics() {
    local lines
    lines=$(data_macs)
    [[ $(wc -l <<<"$lines") -eq 2 ]] || { echo "expected two data NICs on $DATA_SUBNET, found: $lines" >&2; return 1; }
    local role=sender
    while read -r mac ip prefix; do
        local iface vf bdf
        iface=$(for d in /sys/class/net/*; do
            if [[ $(cat "$d/address") == "$mac" && ! -e $d/master ]]; then basename "$d"; fi
        done | head -1)
        if [[ $CLOUD == azure ]]; then
            vf=$(ip -br link show master "$iface" | awk '{print $1}' | head -1)
        else
            vf=$iface
        fi
        bdf=$(ethtool -i "$vf" | awk '/bus-info/ {print $2}')
        echo "$role $iface $vf $mac $ip $prefix $bdf"
        role=receiver
    done <<<"$lines"
}

# The data NICs are this script's alone: the OS's network manager must not run
# DHCP on them or tidy away the addresses, rules and routes data_kernel_up
# gives them. On AWS's Debian images cloud-init handles NIC hotplug: each time
# an ENA came back from DPDK it ran DHCP on every NIC and re-applied netplan,
# replacing the data NICs' addresses 2 s after data_kernel_up set them (so the
# kernel dropped their routes: sends failed with ENETUNREACH, or left by the
# other NIC and were dropped) and resetting the management NIC's. So: cloud-init
# keeps to boot-time network configuration and its hotplug socket is masked;
# systemd-networkd sees them unmanaged (an earlier-sorting .network file wins
# over netplan's) and leaves foreign rules and routes alone (from its next
# start: `isolate` reboots); a DHCP client running on one is stopped by PID.
data_unmanaged() {
    local macs file=/etc/systemd/network/00-ergon-lab-data.network conf=/etc/systemd/networkd.conf.d/ergon-lab.conf
    macs=$(data_macs | awk '{printf "%s%s", sep, $1; sep = " "}')
    [[ -n $macs ]] || return 1
    local want
    want=$(printf '[Match]\nMACAddress=%s\n\n[Link]\nUnmanaged=yes\n' "$macs")
    if [[ $(cat "$file" 2>/dev/null) != "$want" ]]; then
        sudo mkdir -p "$(dirname "$file")" "$(dirname "$conf")"
        printf '%s\n' "$want" | sudo tee "$file" >/dev/null
        printf '[Network]\nManageForeignRoutingPolicyRules=no\nManageForeignRoutes=no\n' | sudo tee "$conf" >/dev/null
        if systemctl is-active --quiet systemd-networkd; then
            sudo networkctl reload || echo "networkctl reload failed: the data NICs are unmanaged from the next boot" >&2
        fi
    fi
    local nohp=/etc/cloud/cloud.cfg.d/99-ergon-lab-no-hotplug.cfg
    if [[ -d /etc/cloud/cloud.cfg.d && ! -f $nohp ]]; then
        printf 'updates:\n  network:\n    when: [boot]\n' | sudo tee "$nohp" >/dev/null
    fi
    if systemctl cat cloud-init-hotplugd.socket >/dev/null 2>&1 &&
        [[ $(systemctl is-enabled cloud-init-hotplugd.socket 2>/dev/null) != masked ]]; then
        sudo systemctl mask --now cloud-init-hotplugd.socket cloud-init-hotplugd.service >/dev/null 2>&1 ||
            echo "could not mask cloud-init's hotplug socket" >&2
    fi
    local pid iface
    for iface in $(nics | awk '{print $2; if ($3 != $2) print $3}'); do
        for pid in $(pgrep -x 'dhclient|dhcpcd' || true); do
            if tr '\0' '\n' 2>/dev/null <"/proc/$pid/cmdline" | grep -qx "$iface"; then
                echo "stopping DHCP client $pid on data NIC $iface" >&2
                sudo kill "$pid" 2>/dev/null || true
            fi
        done
    done
}

# pin_irqs <when>: every IRQ on the housekeeping CPU (irqs.sh irqs_pin), then
# under a mode's @IRQ= (`driver` leaves the CPU it chose in $run/irq_cpu,
# `stop` removes it) the data NICs' queue IRQs on that CPU (irqs_pin_queues):
# wherever the housekeeping pin runs again after a change to the NICs, so
# does this. One left elsewhere is said here; `driver` checks them all before
# it starts the media driver, fail closed.
pin_irqs() {
    irqs_pin "$HK_CPU" "$1"
    local cpu
    # the transport suite: every data-NIC interrupt on its map's IRQ CPU
    # (irqs.sh irqs_pin_data), the NICs' interfaces and VFs
    # (fail closed; none to move only while DPDK holds every NIC)
    if [[ -s ${run:-}/transport-map ]]; then
        local ifaces
        ifaces=$(data_ifaces 2>/dev/null) || ifaces=
        read -r _ cpu _ <"$run/transport-map"
        [[ -n $ifaces ]] || { echo "irqs $1: no data NIC in the kernel's hands" >&2; return 0; }
        # shellcheck disable=SC2086
        irqs_pin_data "$cpu" "$1" $ifaces
        return
    fi
    cpu=$(cat "$run/irq_cpu" 2>/dev/null) || return 0
    [[ -n $cpu && $cpu != "$HK_CPU" ]] || return 0
    # shellcheck disable=SC2046
    irqs_pin_queues "$cpu" "$1" $(data_ifaces) || true
}

# data_ifaces: the data NICs' kernel interfaces (and Azure's VFs), one a line
data_ifaces() { nics | awk '{print $2; if ($3 != $2) print $3}'; }
# data_nics_env: the socket transport's data NICs as the driver's environment
# (RUSTERON_MEDIA_DRIVER_DATA_NICS, comma-separated: every data_ifaces line,
# never the management NIC), so the media driver checks them at start (T2,
# T6, N1, N2, N4) and again after a reset; 1 where they cannot be read
data_nics_env() {
    local ifaces
    ifaces=$(data_ifaces)
    [[ -n $ifaces ]] || return 1
    echo "export RUSTERON_MEDIA_DRIVER_DATA_NICS='$(paste -sd, - <<<"$ifaces")'"
}

# Give the data NICs back to the kernel. Both are on one subnet, and both clouds
# drop a packet whose source address is not the sending NIC's, so each NIC gets
# a routing table its own source address selects. Nothing is removed before
# its replacement is in place, so the addresses' traffic always has a route.
# Then every IRQ back on the housekeeping CPU (and a mode's @IRQ= queue IRQs
# on its CPU: pin_irqs): an ENA brought up (after DPDK gave it back, `stop`'s
# rebind, or a ring change's reset) sets its queue IRQs' affinity anew, one
# queue a CPU.
data_kernel_up() {
    data_unmanaged
    local table=100
    nics | while read -r _ iface vf _ ip prefix _; do
        sudo ip link set "$vf" up
        sudo ip link set "$iface" up
        sudo ip addr replace "$ip/$prefix" dev "$iface"
        local route
        # its subnet, and a peer's off it through the gateway (host.sh policy_routes)
        while read -r route; do
            # shellcheck disable=SC2086
            sudo ip route replace $route
        done < <(policy_routes "$ip" "$prefix" "$iface" "$table" "$PEER_SUBNET")
        if ! ip rule show | awk -v ip="$ip" -v t="$table" '$1 == t":" && $3 == ip && $5 == t {f = 1} END {exit !f}'; then
            sudo ip rule add from "$ip" lookup "$table" priority "$table"
        fi
        # NICs can come back in another order after a DPDK run: any other rule
        # left for this address goes, now that its current one is in place
        # (plain "from <ip> lookup <table>" rules only: HAIRPIN's from/to
        # rules stay)
        ip rule show | awk -v ip="$ip" -v t="$table" '$2 == "from" && $3 == ip && $4 == "lookup" && !($1 == t":" && $5 == t) {sub(":", "", $1); print $1, $5}' |
            while read -r prio tbl; do sudo ip rule del priority "$prio" from "$ip" lookup "$tbl"; done
        table=$((table + 1))
    done
    # Graviton: receive packet steering off on the data NICs, as AWS's
    # Graviton tuning has it (not needed from Graviton2 on), so a NIC's
    # softirq work stays where its interrupt is: off by default, set
    # whatever the image did
    if [[ $ARCH == aarch64 ]]; then
        local f
        for f in $(nics | awk '{print "/sys/class/net/" $2 "/queues/rx-*/rps_cpus"}'); do
            if [[ -e $f ]]; then echo 0 | sudo tee "$f" >/dev/null; fi
        done
    fi
    pin_irqs "after the data NICs came up"
    # nothing measured on a broken path: each NIC's rule and route must hold
    local missing
    missing=$(data_routes_missing)
    [[ $missing == 0 ]] || { echo "data NICs: $missing policy route(s) missing after configuring them" >&2; return 1; }
    # the transport suite: its NIC settings again, as a NIC brought up (or
    # given back by DPDK) has its driver's own, and its routes just replaced
    if [[ -s ${run:-}/transport-map ]]; then transport_nics; fi
}

# transport_nics: the transport suite's data-NIC settings (transport-settings.env),
# each read back: every VF's rings and interrupt coalescing off, every route
# to the data subnet with quickack 1, every data-NIC interrupt on the IRQ CPU;
# 1, saying what, when one does not hold
transport_nics() {
    local vf rc=0 route cpu
    for vf in $(nics | awk '{print $3}'); do
        transport_ring "$vf" "$TR_RX_RING" "$TR_TX_RING" || rc=1
        if [[ $TR_COALESCE_OFF == 1 ]]; then transport_coalesce_off "$vf" || rc=1; fi
    done
    if [[ $TR_QUICKACK_ROUTES == 1 ]]; then
        transport_quickack_apply "$DATA_SUBNET"
        transport_quickack_check "$DATA_SUBNET" || rc=1
    fi
    read -r _ cpu _ <"$run/transport-map"
    # shellcheck disable=SC2046
    irqs_data_check "$cpu" $(data_ifaces) | tail -1 || rc=1
    return $rc
}

# How many data NICs lack their policy rule or one of their table's routes
# (its subnet's; a peer's off it, through the gateway) (0 while DPDK owns
# them: no kernel interfaces then). `loss` reports it, so a route lost mid-run
# shows as a loss counter that grew.
data_routes_missing() {
    local table=100 missing=0 ok dest
    while read -r _ iface _ _ ip prefix _; do
        # a NIC DPDK owns has no kernel interface: nothing to route
        [[ -n $iface && -e /sys/class/net/$iface ]] || { table=$((table + 1)); continue; }
        ok=1
        ip rule show | awk -v ip="$ip" -v t="$table" '$1 == t":" && $3 == ip && $5 == t {f = 1} END {exit !f}' || ok=0
        while read -r dest _; do
            ip route show table "$table" "$dest" | grep -q " dev $iface " || ok=0
        done < <(policy_routes "$ip" "$prefix" "$iface" "$table" "$PEER_SUBNET")
        ((ok)) || missing=$((missing + 1))
        table=$((table + 1))
    done < <(nics 2>/dev/null)
    echo "$missing"
}

# data_mtu <MTU>: the data NICs' MTU (kernel), each one's own first kept for
# `stop` to put back, then their IRQs pinned again (a driver may reset the
# device for it)
data_mtu() {
    local dev
    for dev in $(nics | awk '{print $2; if ($3 != $2) print $3}'); do
        save_host "mtu-$dev" "$(cat "/sys/class/net/$dev/mtu")"
        sudo ip link set dev "$dev" mtu "$1"
    done
    pin_irqs "after the data NICs' MTU $1"
}

# append_params <file> <params>: Aeron URI parameters added to a client's
# channel parameters (key=value|...), each experiment's in turn
append_params() {
    local cur
    cur=$(cat "$1" 2>/dev/null || true)
    printf '%s\n' "${cur:+$cur|}$2" >"$1"
}

# Data NICs a DPDK run took from the kernel go back to it.
data_return_to_kernel() {
    local dev rebound=0
    # Azure: VMBus devices on uio_hv_generic go back to hv_netvsc
    for dev in /sys/bus/vmbus/drivers/uio_hv_generic/*-*-*-*-*; do
        [[ -e $dev ]] || continue
        echo "$(basename "$dev")" | sudo tee /sys/bus/vmbus/drivers/uio_hv_generic/unbind >/dev/null
        echo "$(basename "$dev")" | sudo tee /sys/bus/vmbus/drivers/hv_netvsc/bind >/dev/null
        rebound=1
    done
    # AWS: ENAs on igb_uio go back to ena
    for dev in /sys/bus/pci/drivers/igb_uio/0000:*; do
        [[ -e $dev ]] || continue
        local bdf
        bdf=$(basename "$dev")
        echo "$bdf" | sudo tee /sys/bus/pci/drivers/igb_uio/unbind >/dev/null
        echo | sudo tee /sys/bus/pci/devices/"$bdf"/driver_override >/dev/null
        echo "$bdf" | sudo tee /sys/bus/pci/drivers_probe >/dev/null
        rebound=1
    done
    if ((rebound)); then sleep 3; fi
}

dpdk_active() {
    [[ -n $(ls -d "$LAB_SYS_ROOT"/bus/vmbus/drivers/uio_hv_generic/*-*-*-*-* "$LAB_SYS_ROOT"/bus/pci/drivers/igb_uio/0000:* 2>/dev/null) ]]
}

# "tx_packets: <n> rx_packets: <n>": the frames a kernel interface sent and received
nic_packets() {
    local s=/sys/class/net/$1/statistics
    printf 'tx_packets: %s rx_packets: %s' "$(cat "$s/tx_packets" 2>/dev/null || echo 0)" "$(cat "$s/rx_packets" 2>/dev/null || echo 0)"
}

# A host setting's value before an experiment changed it (vm.sh driver
# <mode>@KEY=value), kept once per setting until `stop` puts it back. (The
# NAPI settings' and SMT's sysfs is under host.sh's $LAB_SYS_ROOT, /sys: the
# test's fake.)
save_host() {
    mkdir -p "$run/restore"
    [[ -f $run/restore/$1 ]] || printf '%s\n' "$2" >"$run/restore/$1"
}

# ethtool -C/-G exit 80 when nothing changed: the value is already in effect.
ethtool_set() {
    local rc=0
    sudo ethtool "$@" 2>"$run/ethtool.err" || rc=$?
    [[ $rc == 0 || $rc == 80 ]] || { echo "ethtool $*: $(cat "$run/ethtool.err")" >&2; return 1; }
}

# The host settings AWS's low-latency guidance names, one experiment at a time
# (<mode>@KEY=value; anything else is an environment variable for the driver).
# One that cannot apply on this host fails the run, never passes as a no-op.
#   THP=<always|madvise|never>  transparent hugepages (never, the baseline,
#        also stops khugepaged, whose every scan drains each CPU's LRU batch,
#        preempting busy-spinning agents; `stop` puts the baseline back)
#   BIN=<dir>         the media driver and benchmark clients from <dir>
#        (absolute, or relative to the home directory) instead of this
#        checkout's build: dpdk@BIN=bin-base A/Bs an older build (lab.sh
#        LAB_BASE builds one there) against the current one. A client <dir>
#        lacks is this checkout's, and the client says so
#   DPDK=stock|patched-refill  (DPDK modes) the media driver and every
#        client from ~/bin-dpdk-<build> (vm.sh build-dpdk: linked against
#        DPDK 25.11 from source; patched-refill with
#        dpdk-ena-deferred-refill.patch on ENA's PMD), as BIN= takes a
#        build, never beside it; once the driver starts, its log shows the
#        PMD the build loads (dpdk_variant_check, fail closed); `stop`
#        forgets it
#   CSTATE=<n>        no idle state deeper than the n-th on any CPU
#        (intel_idle.max_cstate=n at run time; 1 = C1, 0 = poll only). x86
#        only: arm64 has no C-states the OS controls, so it is refused there
#        (the isolated-poll state boots with nohlt instead)
#   BUSY_POLL=<us>    kernel-busy's net.core.busy_read and busy_poll (50
#        otherwise; AWS's ENA latency guidance says 70)
#   COALESCE=off|<rx>/<tx>  the data NICs' interrupt moderation, adaptive off
#        (kernel modes; off = rx-usecs 0 tx-usecs 0)
#   RX_RING=<n>       the data NICs' receive ring (kernel modes)
#   MIN_FREE_PCT=<n>  vm.min_free_kbytes at n% of memory (no direct reclaim stalls)
#   CLOCKSOURCE=<name> the kernel's clocksource
#   RELIABLE=false    the clients subscribe with reliable=false|group=false|
#        tether=false: no NAKs or retransmits, a gap filled instead (counted as
#        loss gap fills), so a run past capacity sheds rather than storms
#   AGENTS=off        the cloud's guest agents stopped (Azure: walinuxagent and
#        hv-kvp-daemon, suspected of a once-a-minute stall of network
#        syscalls; AWS images run none)
#   SMT=off           (x86) the SMT siblings offline for the mode
#        (/sys/devices/system/cpu/smt/control; refused where it is
#        notsupported, forceoff or notimplemented, and on arm64), the IRQs
#        pinned again, the core layout checked unchanged (one thread a core
#        either way: the siblings were never used); `stop` turns them back on
#   JUMBO=1           jumbo frames (AWS; modes.sh): kernel modes' data NICs at
#        the jumbo MTU for the placement (9001, or 8500 across inter-Region
#        peering), Aeron's MTU and the DPDK transport's to match (host.sh
#        jumbo_env; an @AERON_MTU_LENGTH= in the mode wins); `stop` puts the
#        NICs' MTU back. lib.sh checks the path (vm.sh pmtu) first
#   PUB_PARAMS=<k>=<v>+...  Aeron URI parameters on every publication the
#        clients create (PUB_CHANNEL_PARAMS; modes.sh channel_params checks
#        them), and
#   SUB_PARAMS=<k>=<v>+...  on every subscription (SUB_CHANNEL_PARAMS, after
#        RELIABLE=false's if both)
#   IRQ=hk|receiver|spare  (kernel modes, AWS) the data NICs' queue IRQs
#        (ENA's <interface>-Tx-Rx-<n>) on one CPU for the mode (host.sh
#        setting_cpu: housekeeping's; the Aeron receiver agent's by threading
#        mode, AWS's "interrupts on the application's vCPU"; the layout's
#        fifth core, refused without one), every other IRQ on housekeeping's
#        as ever; pinned there again after every change to the NICs in the
#        mode (pin_irqs, wherever the housekeeping pin runs) and checked
#        before the driver starts (fail closed); `loss` counts their
#        interrupts off that CPU; `stop` puts them back on housekeeping's
#   NAPI_DEFER=<n>/<ns>  (kernel modes) the data NICs' napi_defer_hard_irqs
#        and gro_flush_timeout for the mode (after the transport's own: 0 and
#        0 kernel, 2 and 200000 kernel-busy); `stop` puts them back
#   NAPI_BUSY=spare  (kernel modes, Linux 6.17 and up, the kernel's ynl:
#        vm.sh build-ynl) threaded NAPI busy polling on every NAPI of the data
#        NICs, each kthread pinned to the layout's fifth core, which they spin
#        at 100% (host.sh napi_busy_on), set after every other setting (a ring
#        or MTU change resets the NICs); fail closed; `stop` sets them back to
#        threaded disabled
#   CONDUCTOR=own  the driver's conductor alone on the layout's fifth core,
#        spinning (AERON_CONDUCTOR_CPU_AFFINITY, AERON_CONDUCTOR_IDLE_STRATEGY
#        =spin; an override later in the mode wins), the DPDK transport's
#        housekeeping and local-I/O threads named onto the housekeeping CPU
#        (RUSTERON_DPDK_HOUSEKEEPING_CPU: they would follow a conductor that
#        does not spin); refused under SHARED (the conductor is the shared
#        agent) and INVOKER (no conductor thread), without a fifth core, and
#        beside @IRQ=spare or @NAPI_BUSY=spare (one core each); its thread
#        checked on that core alone once the driver starts (agents_placed,
#        fail closed); `stop` forgets it
#   HAIRPIN=1  (the raw floor's modes, AWS) the round trip between this host's
#        own two data NICs (the floor client's xhairpin): the pong on the
#        receiver core (refused without one); on the kernel wire each end's
#        sends routed out of its own NIC, not over loopback (rules before the
#        local table's, accept_local, no reverse-path filter on the data
#        NICs), checked by ip route get both ways (fail closed); `stop` puts
#        the rules and sysctls back
#   RTT_GAP_US=<us>, KEEPALIVE_US=<us>, FLUSH_KB=<KiB>  the floor client's
#        pacing, keep-alives and cache flush (lib.sh hands them to the
#        client): nothing for the host or the driver
# Returns 1 when KEY is not a host setting.
host_setting() {
    local key=$1 value=$2 mode=$3 st vf names=() cur
    case $key in
    THP)
        [[ $value =~ ^(always|madvise|never)$ ]] || { echo "THP=$value: always, madvise or never" >&2; exit 2; }
        echo "$value" | sudo tee /sys/kernel/mm/transparent_hugepage/enabled >/dev/null || exit 2
        ;;
    BIN)
        local dir=$value
        [[ $dir == /* ]] || dir=$HOME/${dir#\~/}
        [[ -x $dir/media_driver ]] || { echo "BIN=$value: no media_driver in $dir" >&2; exit 2; }
        printf '%s\n' "$dir" >"$run/bin"
        save_host bin set
        bin=$dir
        log "BIN=$value: the media driver and clients from $dir ($(ls "$dir" | tr '\n' ' '))"
        ;;
    DPDK)
        [[ $DPDK_BUILDS == *" $value "* ]] || { echo "DPDK=$value: stock or patched-refill" >&2; exit 2; }
        [[ $mode == dpdk ]] || { echo "DPDK=$value: DPDK modes only (the media driver and clients linked against that DPDK build)" >&2; exit 2; }
        [[ ! -s $run/bin ]] || { echo "DPDK=$value with BIN=: one build a mode" >&2; exit 2; }
        local dir=$HOME/bin-dpdk-$value
        [[ -x $dir/media_driver ]] || { echo "DPDK=$value: no media_driver in $dir (vm.sh build-dpdk $value)" >&2; exit 2; }
        printf '%s\n' "$dir" >"$run/bin"
        save_host bin set
        bin=$dir
        printf '%s\n' "$value" >"$run/dpdk_variant"
        save_host dpdk_variant set
        log "DPDK=$value: the media driver and clients from $dir"
        ;;
    CSTATE)
        [[ $value =~ ^[0-9]$ ]] || { echo "CSTATE=$value: an idle state index" >&2; exit 2; }
        if [[ $ARCH == aarch64 ]]; then
            echo "CSTATE=$value: x86 only: arm64 has no C-states the OS controls (Debian's cloud-arm64 kernel builds no cpuidle driver; idle is WFI): LAB_STATES=isolated-poll boots with nohlt instead" >&2
            exit 2
        fi
        if [[ ! -d /sys/devices/system/cpu/cpu0/cpuidle ]]; then
            echo "CSTATE=$value: no cpuidle driver here ($(cat /sys/devices/system/cpu/cpuidle/current_driver 2>/dev/null || echo none)): the hypervisor owns idle" >&2
            exit 2
        fi
        save_host CSTATE all-enabled
        for st in /sys/devices/system/cpu/cpu[0-9]*/cpuidle/state[0-9]*; do
            if ((${st##*state} > value)); then
                echo 1 | sudo tee "$st/disable" >/dev/null
                names+=("$(cat "$st/name")")
            fi
        done
        log "CSTATE=$value: disabled $(printf '%s\n' "${names[@]}" | sort -u | tr '\n' ' ')on every CPU"
        ;;
    COALESCE)
        [[ $mode == kernel || $mode == kernel-busy ]] || { echo "COALESCE: kernel modes only (DPDK takes no interrupts)" >&2; exit 2; }
        local args
        if [[ $value == off ]]; then
            args="adaptive-rx off rx-usecs 0 tx-usecs 0"
        elif [[ $value =~ ^([0-9]+)/([0-9]+)$ ]]; then
            args="adaptive-rx off rx-usecs ${BASH_REMATCH[1]} tx-usecs ${BASH_REMATCH[2]}"
        else
            echo "COALESCE=$value: off or <rx usecs>/<tx usecs>" >&2
            exit 2
        fi
        while read -r _ _ vf _; do
            cur=$({ ethtool -c "$vf" 2>/dev/null || true; } | awk '/^Adaptive RX:/ {a = $3} /^rx-usecs:/ {r = $2} /^tx-usecs:/ {t = $2}
                END {if (a != "n/a" && r != "n/a") printf "adaptive-rx %s rx-usecs %s tx-usecs %s", a, r, t}')
            [[ -n $cur ]] || { echo "COALESCE: $vf ($(ethtool -i "$vf" | awk '/^driver/ {print $2}')) has no interrupt moderation to set" >&2; exit 2; }
            save_host "coalesce-$vf" "$cur"
            # shellcheck disable=SC2086
            ethtool_set -C "$vf" $args || exit 2
        done < <(nics)
        # a driver may reset the device for it, placing its IRQs anew
        pin_irqs "after COALESCE=$value"
        log "COALESCE=$value: $args on the data NICs"
        ;;
    BUSY_POLL)
        [[ $mode == kernel-busy ]] || { echo "BUSY_POLL: kernel-busy only" >&2; exit 2; }
        [[ $value =~ ^[1-9][0-9]*$ ]] || { echo "BUSY_POLL=$value: microseconds, from 1" >&2; exit 2; }
        sudo sysctl -qw net.core.busy_read="$value" net.core.busy_poll="$value"
        log "BUSY_POLL=$value: net.core.busy_read and busy_poll $value us"
        ;;
    RX_RING)
        [[ $mode == kernel || $mode == kernel-busy ]] || { echo "RX_RING: kernel modes only (DPDK sets its own descriptors)" >&2; exit 2; }
        [[ $value =~ ^[0-9]+$ ]] || { echo "RX_RING=$value: a descriptor count" >&2; exit 2; }
        while read -r _ _ vf _; do
            cur=$({ ethtool -g "$vf" 2>/dev/null || true; } | awk '/^Current/ {c = 1} c && /^RX:/ {print $2; exit}')
            [[ -n $cur ]] || { echo "RX_RING: $vf reports no ring size" >&2; exit 2; }
            save_host "ring-$vf" "$cur"
            ethtool_set -G "$vf" rx "$value" || exit 2
        done < <(nics)
        # a ring change resets the device: its addresses and routes again, and
        # its IRQs (data_kernel_up pins them)
        data_kernel_up
        log "RX_RING=$value on the data NICs"
        ;;
    MIN_FREE_PCT)
        [[ $value =~ ^[1-5]$ ]] || { echo "MIN_FREE_PCT=$value: 1 to 5" >&2; exit 2; }
        save_host min_free_kbytes "$(cat /proc/sys/vm/min_free_kbytes)"
        cur=$(awk -v p="$value" '/^MemTotal:/ {printf "%d", $2 * p / 100}' /proc/meminfo)
        sudo sysctl -qw vm.min_free_kbytes="$cur"
        log "MIN_FREE_PCT=$value: vm.min_free_kbytes $cur"
        ;;
    RELIABLE)
        [[ $value == false ]] || { echo "RELIABLE=$value: false" >&2; exit 2; }
        append_params "$run/sub_channel_params" 'reliable=false|group=false|tether=false'
        save_host sub_channel_params set
        log "RELIABLE=false: subscriptions with reliable=false|group=false|tether=false"
        ;;
    PUB_PARAMS | SUB_PARAMS)
        local side=pub params
        [[ $key == SUB_PARAMS ]] && side=sub
        params=$(channel_params "$side" "$value") || exit 2
        append_params "$run/${side}_channel_params" "$params"
        save_host "${side}_channel_params" set
        log "$key: every $([[ $side == pub ]] && echo publication || echo subscription) channel the clients create with |$params"
        ;;
    SMT)
        [[ $value == off ]] || { echo "SMT=$value: off" >&2; exit 2; }
        local control=$LAB_SYS_ROOT/devices/system/cpu/smt/control before after spinner
        smt_check "$ARCH" "$(cat "$control" 2>/dev/null || true)" || exit 2
        before=$(layout)
        save_host smt "$(cat "$control")"
        # the diagnostics' gap spinner, where its CPU goes offline (a
        # four-core x86's: housekeeping's twin, host.sh diag_spare_cpu): the
        # kernel would move it, spinning, onto a CPU in use, so it stops for
        # the mode and `stop` starts it again once SMT is back
        spinner=$(cat "$run/diag-spare" 2>/dev/null || true)
        if [[ -n $spinner ]] && pgrep -x gapspin >/dev/null && smt_takes_offline "$spinner"; then
            pkill -x gapspin 2>/dev/null || true
            echo "$spinner" >"$run/diag-paused"
            # when, for spikes.py: no host stall is seen until it resumes
            echo "# paused $(date +%s.%N)" >>"$run/diag-gapspin.log"
            log "SMT=off: the gap spinner's CPU $spinner goes offline: the spinner stopped for the mode"
        fi
        echo off | sudo tee "$control" >/dev/null
        # the mode keeps the SMT-on layout (layout() returns it while SMT is
        # off): its every CPU must still be a core online
        for after in $before; do
            if [[ ! -r $LAB_SYS_ROOT/devices/system/cpu/cpu$after/topology/thread_siblings_list ]]; then
                echo "SMT=off: CPU $after of the layout ($before) went offline" >&2
                restore_host
                exit 2
            fi
        done
        echo "$before" >"$run/layout-smt-off"
        pin_irqs "after SMT=off"
        log "SMT=off: smt active $(cat "$LAB_SYS_ROOT/devices/system/cpu/smt/active" 2>/dev/null), CPUs online $(cat "$LAB_SYS_ROOT/devices/system/cpu/online" 2>/dev/null), layout $before kept"
        ;;
    JUMBO)
        [[ $value == 1 ]] || { echo "JUMBO=$value: 1 (jumbo frames) or nothing" >&2; exit 2; }
        [[ $CLOUD == aws ]] || { echo "JUMBO: AWS only (ENA's 9001-byte frames)" >&2; exit 2; }
        local transport=kernel nic_mtu
        [[ $mode == dpdk ]] && transport=dpdk
        nic_mtu=$(jumbo_mtus "$PLACEMENT")
        nic_mtu=${nic_mtu%% *}
        # an @AERON_MTU_LENGTH= override in the mode wins, wherever it stands
        # (modes.sh mode_mtu reads it so)
        if mode_get "x@$overrides" AERON_MTU_LENGTH >/dev/null; then
            jumbo_env "$PLACEMENT" "$transport" | grep -v "AERON_MTU_LENGTH=" >>"$env_file" || true
        else
            jumbo_env "$PLACEMENT" "$transport" >>"$env_file"
        fi
        # DPDK sets its port's MTU itself; a kernel mode's NICs here
        if [[ $transport == kernel ]]; then data_mtu "$nic_mtu"; fi
        log "JUMBO=1: $(jumbo_env "$PLACEMENT" "$transport" | sed "s/^export //" | tr '\n' ' ')$([[ $transport == kernel ]] && echo "data NICs at MTU $nic_mtu")"
        ;;
    AGENTS)
        [[ $value == off ]] || { echo "AGENTS=$value: off" >&2; exit 2; }
        local unit stopped=()
        for unit in walinuxagent hv-kvp-daemon; do
            if systemctl is-active --quiet "$unit"; then
                sudo systemctl stop "$unit"
                stopped+=("$unit")
            fi
        done
        ((${#stopped[@]} > 0)) || { echo "AGENTS=off: no guest agent running here" >&2; exit 2; }
        save_host agents "${stopped[*]}"
        log "AGENTS=off: stopped ${stopped[*]}"
        ;;
    IRQ)
        [[ $mode == kernel || $mode == kernel-busy ]] || { echo "IRQ: kernel modes only (DPDK takes the data NICs' queues: no interrupts)" >&2; exit 2; }
        [[ $CLOUD == aws ]] || { echo "IRQ: AWS only (ENA's queue IRQs, named after their interfaces)" >&2; exit 2; }
        local cpu whose
        cpu=$(setting_cpu "$value" "${threading:-DEDICATED}" "$HK_CPU" "$APP_CPU" "$SENDER_CPU" "$RECEIVER_CPU" "${APP2_CPU:-}") || exit 2
        case $value in
        hk) whose="housekeeping's, as every IRQ's" ;;
        receiver) whose="the Aeron receiver agent's under ${threading:-DEDICATED}" ;;
        spare) whose="the layout's fifth core" ;;
        esac
        save_host irq set
        echo "$cpu" >"$run/irq_cpu"
        pin_irqs "after IRQ=$value"
        log "IRQ=$value: the data NICs' queue IRQs on CPU $cpu ($whose), every other IRQ on housekeeping's ($HK_CPU)"
        ;;
    NAPI_DEFER)
        [[ $mode == kernel || $mode == kernel-busy ]] || { echo "NAPI_DEFER: kernel modes only (DPDK holds the NICs)" >&2; exit 2; }
        local args n ns
        args=$(napi_defer_args "$value") || exit 2
        read -r n ns <<<"$args"
        while read -r _ _ vf _; do
            save_host "napi-defer-$vf" "$(cat "$LAB_SYS_ROOT/class/net/$vf/napi_defer_hard_irqs") $(cat "$LAB_SYS_ROOT/class/net/$vf/gro_flush_timeout")"
            echo "$n" | sudo tee "$LAB_SYS_ROOT/class/net/$vf/napi_defer_hard_irqs" >/dev/null || exit 2
            echo "$ns" | sudo tee "$LAB_SYS_ROOT/class/net/$vf/gro_flush_timeout" >/dev/null || exit 2
            names+=("$vf")
        done < <(nics)
        log "NAPI_DEFER=$value: napi_defer_hard_irqs $n, gro_flush_timeout $ns ns on ${names[*]}"
        ;;
    NAPI_BUSY)
        [[ $mode == kernel || $mode == kernel-busy ]] || { echo "NAPI_BUSY: kernel modes only (DPDK holds the NICs)" >&2; exit 2; }
        [[ $value == spare ]] || { echo "NAPI_BUSY=$value: spare (the kthreads on the layout's fifth core)" >&2; exit 2; }
        kernel_at_least "$(uname -r)" 6 17 ||
            { echo "NAPI_BUSY: threaded NAPI busy polling needs Linux 6.17 or later (this is $(uname -r))" >&2; exit 2; }
        ynl_ready || exit 2
        local cpu
        cpu=$(setting_cpu spare "${threading:-DEDICATED}" "$HK_CPU" "$APP_CPU" "$SENDER_CPU" "$RECEIVER_CPU" "${APP2_CPU:-}") || exit 2
        # applied after every other setting, before the driver starts
        save_host napi_busy set
        echo "$cpu" >"$run/napi_busy_cpu"
        ;;
    CONDUCTOR)
        [[ $value == own ]] || { echo "CONDUCTOR=$value: own (the conductor alone on the layout's fifth core, spinning)" >&2; exit 2; }
        case ${threading:-DEDICATED} in
        SHARED) echo "CONDUCTOR=own under SHARED: the conductor is the one shared agent, on the sender's core" >&2; exit 2 ;;
        INVOKER) echo "CONDUCTOR=own under INVOKER: the driver has no conductor thread" >&2; exit 2 ;;
        esac
        local cpu other
        for other in IRQ NAPI_BUSY; do
            [[ $(mode_get "x@$overrides" "$other") != spare ]] ||
                { echo "CONDUCTOR=own with $other=spare: both would take the layout's fifth core" >&2; exit 2; }
        done
        cpu=$(setting_cpu spare "${threading:-DEDICATED}" "$HK_CPU" "$APP_CPU" "$SENDER_CPU" "$RECEIVER_CPU" "${APP2_CPU:-}") || exit 2
        save_host conductor set
        echo "$cpu" >"$run/conductor_cpu"
        {
            echo "export AERON_CONDUCTOR_CPU_AFFINITY='$cpu'"
            echo "export AERON_CONDUCTOR_IDLE_STRATEGY='spin'"
            echo "export RUSTERON_DPDK_HOUSEKEEPING_CPU='$HK_CPU'"
        } >>"$env_file"
        log "CONDUCTOR=own: the conductor alone on CPU $cpu (the layout's fifth core), spinning; housekeeping and local I/O on $HK_CPU"
        ;;
    RTT_GAP_US | KEEPALIVE_US | FLUSH_KB) ;;
    HAIRPIN)
        # the raw floor's round trip between this host's own two data NICs
        # (modes.sh HAIRPIN; the floor client's xhairpin): its pong on the
        # receiver core, which the raw floor leaves free (bench/hairpin for
        # the run). The kernel delivers a packet for its own address over
        # loopback, so on the kernel wire each end's sends (from its own
        # address: floor binds them so) are routed out of its own NIC by a
        # rule before the local table's, which moves from priority 0 to 50
        # (its new rule in place first); the data NICs then take their own
        # host's packets (accept_local, no reverse-path filter). Checked both
        # ways by ip route get, fail closed; `stop` puts everything back
        [[ $value == 1 ]] || { echo "HAIRPIN=$value: 1 or nothing" >&2; exit 2; }
        [[ $CLOUD == aws ]] || { echo "HAIRPIN: AWS only (two ENIs on the data subnet)" >&2; exit 2; }
        [[ -n ${floor_wire:-} ]] || { echo "HAIRPIN: the raw floor's modes only (kernel-raw, dpdk-raw): the floor client runs both ends" >&2; exit 2; }
        [[ -n ${RECEIVER_CPU:-} ]] ||
            { echo "HAIRPIN: no receiver core on this host (its layout $HK_CPU ${APP_CPU:-} ${SENDER_CPU:-}): the hairpin's pong runs there; four cores (c9g.xlarge and up)" >&2; exit 2; }
        save_host hairpin set
        echo "$RECEIVER_CPU" >"$run/hairpin"
        if [[ $floor_wire == kernel ]]; then
            local snd_if snd_ip rcv_if rcv_ip sysctls= k fwd back
            # the tables data_kernel_up gives them: 100 the sender's, 101 the receiver's
            read -r _ snd_if _ _ snd_ip _ < <(nics | sed -n 1p)
            read -r _ rcv_if _ _ rcv_ip _ < <(nics | sed -n 2p)
            [[ -n $snd_ip && -n $rcv_ip ]] || { echo "HAIRPIN: the data NICs not read" >&2; restore_host; exit 1; }
            for k in all.rp_filter "$snd_if.rp_filter" "$rcv_if.rp_filter" "$snd_if.accept_local" "$rcv_if.accept_local"; do
                sysctls+=" net.ipv4.conf.$k=$(sysctl -n "net.ipv4.conf.$k")"
            done
            save_host hairpin-sysctl "${sysctls# }"
            save_host hairpin-routes set
            sudo sysctl -qw net.ipv4.conf.all.rp_filter=0 "net.ipv4.conf.$snd_if.rp_filter=0" "net.ipv4.conf.$rcv_if.rp_filter=0" \
                "net.ipv4.conf.$snd_if.accept_local=1" "net.ipv4.conf.$rcv_if.accept_local=1"
            sudo ip rule add priority 10 iif lo from "$snd_ip" to "$rcv_ip" lookup 100
            sudo ip rule add priority 11 iif lo from "$rcv_ip" to "$snd_ip" lookup 101
            ip rule show | grep -q '^50:' || sudo ip rule add priority 50 lookup local
            sudo ip rule del priority 0 lookup local
            fwd=$(ip route get "$rcv_ip" from "$snd_ip" 2>&1 || true)
            back=$(ip route get "$snd_ip" from "$rcv_ip" 2>&1 || true)
            if [[ $fwd != *" dev $snd_if "* || $fwd == local* || $back != *" dev $rcv_if "* || $back == local* ]]; then
                echo "HAIRPIN: this host's own addresses still routed over loopback, not out of their NICs ($fwd; $back): stopping" >&2
                restore_host
                exit 1
            fi
            log "HAIRPIN=1: $snd_if ($snd_ip) to $rcv_if ($rcv_ip) and back out of the NICs ($fwd; $back); the pong on CPU $RECEIVER_CPU"
        else
            log "HAIRPIN=1: the DPDK sender port to the receiver port and back; the pong on CPU $RECEIVER_CPU"
        fi
        ;;
    CLOCKSOURCE)
        local cs=/sys/devices/system/clocksource/clocksource0
        grep -qw -- "$value" "$cs/available_clocksource" || { echo "CLOCKSOURCE=$value: not among $(cat "$cs/available_clocksource")" >&2; exit 2; }
        save_host clocksource "$(cat "$cs/current_clocksource")"
        echo "$value" | sudo tee "$cs/current_clocksource" >/dev/null
        ;;
    *) return 1 ;;
    esac
}

# dpdk_variant_check <driver log>: under a mode's @DPDK=<build>, the ENA PMD
# the log shows loaded is that build's: patched-refill's line
# (dpdk-ena-deferred-refill.patch's, one per RX queue) there, or under stock
# none; 1, saying why, when not. 0 without @DPDK=.
dpdk_variant_check() {
    local v n
    v=$(cat "$run/dpdk_variant" 2>/dev/null) || return 0
    n=$(grep -c 'ergon-lab deferred-refill ENA PMD' "$1" 2>/dev/null) || n=0
    case $v in
    patched-refill)
        ((n > 0)) || { echo "DPDK=patched-refill: no RX queue on the patched ENA PMD (no 'ergon-lab deferred-refill ENA PMD' line in $1): stopping" >&2; return 1; }
        log "DPDK=patched-refill: the patched ENA PMD on $n RX queues ($1)"
        ;;
    *)
        ((n == 0)) || { echo "DPDK=$v: $n RX queues on the patched ENA PMD in $1, not $v's: stopping" >&2; return 1; }
        log "DPDK=$v: the $v ENA PMD (no patched RX queue in $1)"
        ;;
    esac
}

# Every host setting an experiment changed goes back (`stop`), the data NICs'
# once the kernel has them again (a ring or interrupt moderation put back
# resets them: `stop`'s data_kernel_up then pins their IRQs again).
restore_host() {
    local f key st n ns dev spin
    for f in "$run"/restore/*; do
        [[ -e $f ]] || continue
        key=${f##*/}
        case $key in
        CSTATE)
            for st in /sys/devices/system/cpu/cpu[0-9]*/cpuidle/state[0-9]*/disable; do
                echo 0 | sudo tee "$st" >/dev/null
            done
            ;;
        # shellcheck disable=SC2046
        coalesce-*) [[ -e /sys/class/net/${key#coalesce-} ]] && ethtool_set -C "${key#coalesce-}" $(cat "$f") || true ;;
        ring-*) [[ -e /sys/class/net/${key#ring-} ]] && ethtool_set -G "${key#ring-}" rx "$(cat "$f")" || true ;;
        min_free_kbytes) sudo sysctl -qw vm.min_free_kbytes="$(cat "$f")" ;;
        # shellcheck disable=SC2046
        agents) sudo systemctl start $(cat "$f") ;;
        sub_channel_params) rm -f "$run/sub_channel_params" ;;
        pub_channel_params) rm -f "$run/pub_channel_params" ;;
        # shellcheck disable=SC2015
        mtu-*) [[ -e /sys/class/net/${key#mtu-} ]] && sudo ip link set dev "${key#mtu-}" mtu "$(cat "$f")" || true ;;
        bin) rm -f "$run/bin" ;;
        dpdk_variant) rm -f "$run/dpdk_variant" ;;
        clocksource) sudo tee /sys/devices/system/clocksource/clocksource0/current_clocksource <"$f" >/dev/null ;;
        smt)
            sudo tee "$LAB_SYS_ROOT/devices/system/cpu/smt/control" <"$f" >/dev/null
            rm -f "$run/layout-smt-off"
            pin_irqs "after SMT back to $(cat "$f")"
            # the gap spinner SMT=off stopped (its CPU went offline), again,
            # its CPU read before it starts: read in the background job, the
            # file could be gone first, and a spinner given no CPU spins CPU 0
            if [[ -s $run/diag-paused ]]; then
                spin=$(cat "$run/diag-paused")
                echo "# resumed $(date +%s.%N)" >>"$run/diag-gapspin.log"
                nohup "$run/gapspin" "$spin" "$(gapspin_us)" >>"$run/diag-gapspin.log" 2>&1 </dev/null &
                rm -f "$run/diag-paused"
            fi
            ;;
        irq)
            rm -f "$run/irq_cpu"
            pin_irqs "after IRQ= back to housekeeping"
            ;;
        conductor) rm -f "$run/conductor_cpu" ;;
        hairpin) rm -f "$run/hairpin" ;;
        hairpin-routes)
            # the local table back at 0 before its moved rule goes
            sudo ip rule add priority 0 lookup local 2>/dev/null || true
            sudo ip rule del priority 50 lookup local 2>/dev/null || true
            sudo ip rule del priority 10 2>/dev/null || true
            sudo ip rule del priority 11 2>/dev/null || true
            ;;
        # shellcheck disable=SC2046
        hairpin-sysctl) sudo sysctl -qw $(cat "$f") || true ;;
        # shellcheck disable=SC2015
        napi-defer-*)
            if [[ -e $LAB_SYS_ROOT/class/net/${key#napi-defer-} ]]; then
                read -r n ns <"$f"
                echo "$n" | sudo tee "$LAB_SYS_ROOT/class/net/${key#napi-defer-}/napi_defer_hard_irqs" >/dev/null || true
                echo "$ns" | sudo tee "$LAB_SYS_ROOT/class/net/${key#napi-defer-}/gro_flush_timeout" >/dev/null || true
            fi
            ;;
        napi_busy)
            # every NAPI set (a failed start's too), back to threaded
            # disabled; the netdevs' own threaded flag 0 as well
            # shellcheck disable=SC2046
            napi_busy_off $(awk '{print $2}' "$run/napi_busy" 2>/dev/null) || true
            for dev in $(data_ifaces 2>/dev/null); do
                if [[ -e $LAB_SYS_ROOT/class/net/$dev/threaded ]]; then echo 0 | sudo tee "$LAB_SYS_ROOT/class/net/$dev/threaded" >/dev/null || true; fi
            done
            rm -f "$run/napi_busy" "$run/napi_busy_cpu"
            ;;
        esac
        rm -f "$f"
    done
}

# timesync_setup (bootstrap, AWS): chrony in systemd-timesyncd's place, on
# the Amazon Time Sync Service, its configuration Debian's as host.sh
# chrony_conf turns it, enabled and restarted. Under every placement: with
# LAB_PLACEMENT=precision-time the ENA's PTP hardware clock comes first only
# from the boot after build-ptp (phc_setup), as /dev/ptp_ena is there only
# once ergon-lab-phc.service has run at a boot, and chrony (before 4.9, which
# adds refclock's optional; Debian 13 has 4.6) will not start with a
# refclock whose device is missing. Debian's EC2 image keeps time with
# systemd-timesyncd, which slews the clock by up to some thousands of ppm for
# minutes after a boot and records nothing. Azure's image keeps its own
# chrony on the Hyper-V PTP clock: left alone. ($LAB_ETC_ROOT: /etc;
# test/test-lab.sh's fake)
timesync_setup() {
    [[ $CLOUD == aws ]] || return 0
    local conf=${LAB_ETC_ROOT:-/etc}/chrony/chrony.conf
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq chrony
    sudo systemctl disable --now systemd-timesyncd 2>/dev/null || true
    chrony_conf aws <"$conf" >"$run/chrony.conf" || return 1
    sudo install -m 644 "$run/chrony.conf" "$conf"
    sudo systemctl enable chrony
    sudo systemctl restart chrony
}

# phc_setup (build-ptp, precision-time): the ENA's PTP hardware clock on the
# primary ENI (the default route's), for chrony: Linux's own ENA driver has
# it from 6.17, off unless devlink turns it on (host.sh phc_boot_script),
# which a unit does at every boot, before the network (phc_unit; lib.sh
# reboots into it), and /dev/ptp_ena names that ENI's clock alone, by its
# PCI address (phc_udev_rule). Never a data ENI's: DPDK takes those, and the
# transport asks their PMD for RX timestamps itself
# (RUSTERON_DPDK_RX_HW_TIMESTAMP). No out-of-tree driver: the in-tree one's
# PHC is all chrony needs. Then chrony on the PHC first, the service its
# backup (host.sh chrony_conf precision-time), from that boot too: installed
# and not restarted, as chrony would not start before the device is there
# (the unit runs before chrony.service). 1, saying why, when the primary ENI
# has no PCI address or is a data NIC, chrony left on the service.
# ($LAB_ETC_ROOT, $LAB_USR_ROOT: /etc, /usr; the test's fakes)
phc_setup() {
    local dev bdf etc=${LAB_ETC_ROOT:-/etc} usr=${LAB_USR_ROOT:-/usr}
    dev=$(ip -o route show default | awk '{for (i = 1; i < NF; i++) if ($i == "dev") {print $(i + 1); exit}}')
    bdf=$(ethtool -i "${dev:-none}" 2>/dev/null | awk '/^bus-info:/ {print $2}')
    [[ $bdf =~ ^[0-9a-f]{4}:[0-9a-f]{2}:[0-9a-f]{2}\.[0-7]$ ]] ||
        { echo "build-ptp: the primary ENI (${dev:-no default route}) has no PCI address" >&2; return 1; }
    if nics | awk '{print $7}' | grep -qx "$bdf"; then
        echo "build-ptp: the default route's NIC $dev ($bdf) is a data NIC" >&2
        return 1
    fi
    phc_udev_rule "$bdf" | sudo tee "$etc/udev/rules.d/53-ec2-network-interfaces-ptp.rules" >/dev/null
    phc_boot_script "$bdf" | sudo tee "$usr/local/sbin/ergon-lab-phc" >/dev/null
    sudo chmod 755 "$usr/local/sbin/ergon-lab-phc"
    phc_unit | sudo tee "$etc/systemd/system/ergon-lab-phc.service" >/dev/null
    sudo systemctl daemon-reload
    sudo systemctl enable ergon-lab-phc.service
    chrony_conf precision-time <"$etc/chrony/chrony.conf" >"$run/chrony.conf" || return 1
    sudo install -m 644 "$run/chrony.conf" "$etc/chrony/chrony.conf"
    echo "$bdf" >"$run/phc_bdf"
    echo "build-ptp: the PHC of the primary ENI $dev ($bdf), and chrony on it, from every boot from the next (ergon-lab-phc.service)"
}

# agents_placed <threading mode> </proc/<driver pid>>: once the driver has
# started, its agents where threading_env and the mode put them, each read
# from its thread's allowed CPUs while it pins itself (up to 5 s): SHARED's
# and SHARED_NETWORK's one busy agent on the sender's CPU (host.sh
# shared_agent_check), @CONDUCTOR=own's conductor alone on its core
# (conductor_check; $run/conductor_cpu). 2, with why on stderr, when one
# never is there: `driver` then stops the driver (fail closed)
agents_placed() {
    local threading=$1 dir=$2 ok cpu
    if [[ $threading == SHARED || $threading == SHARED_NETWORK ]]; then
        ok=0
        for _ in $(seq 20); do
            if shared_agent_check "$threading" "$dir" "$SENDER_CPU" 2>"$run/agent-check"; then ok=1; break; fi
            sleep 0.25
        done
        ((ok)) || { cat "$run/agent-check" >&2; return 2; }
        log "driver: $threading's shared agent on CPU $SENDER_CPU"
    fi
    if [[ -s $run/conductor_cpu ]]; then
        cpu=$(cat "$run/conductor_cpu")
        ok=0
        for _ in $(seq 20); do
            if conductor_check "$dir" "$cpu" 2>"$run/agent-check"; then ok=1; break; fi
            sleep 0.25
        done
        ((ok)) || { cat "$run/agent-check" >&2; return 2; }
        log "driver: the conductor alone on CPU $cpu"
    fi
}

# build_dpdk <prefix> [<patch>]: DPDK $DPDK_VERSION from fast.dpdk.org,
# unpacked afresh in ~/<prefix's name> (<patch> applied exactly: -p1, no
# fuzz), only what the ENA path needs, installed under <prefix>; nothing when
# <prefix> has it already, built from the same patch (its SHA-256 in
# <prefix>/ergon-lab-patch.sha256). Never put in ld.so.conf here: build-ptp
# does that for its own; build-dpdk links against its builds by RPATH.
# Graviton: for the core it is built on (-Dplatform=native, DPDK's default:
# the Neoverse's flags), with this host's lcores and NUMA nodes rather than
# DPDK's table's (native Neoverse V2's 144 lcores, or -Dplatform=graviton4's
# 96 and one node, would not cover c8g.metal-48xl's 192 CPUs on two sockets)
build_dpdk() {
    local prefix=$1 patch=${2:-} src sum= blog platform=()
    if [[ -n $patch ]]; then sum=$(sha256sum "$patch" | cut -d' ' -f1); fi
    if [[ -f $prefix/lib/$ARCH-linux-gnu/pkgconfig/libdpdk.pc ]] &&
        [[ -z $patch || $(cat "$prefix/ergon-lab-patch.sha256" 2>/dev/null) == "$sum" ]]; then
        return 0
    fi
    if [[ $ARCH == aarch64 ]]; then platform=(-Dplatform=native -Dmax_lcores=detect -Dmax_numa_nodes=detect); fi
    src=$HOME/${prefix##*/}
    rm -rf "$src" && mkdir -p "$src"
    curl -fsSL "https://fast.dpdk.org/rel/dpdk-$DPDK_VERSION.tar.xz" | tar -xJ -C "$src" --strip-components=1
    if [[ -n $patch ]]; then
        (cd "$src" && patch -p1 --forward --fuzz=0 <"$patch") ||
            { echo "build_dpdk: $patch does not apply to DPDK $DPDK_VERSION" >&2; return 1; }
    fi
    sudo rm -rf "$prefix"
    # meson's and ninja's output in $run/dpdk-build-<prefix's name>.log
    blog=$run/dpdk-build-${prefix##*/}.log
    (cd "$src" &&
        meson setup -Dprefix="$prefix" -Dtests=false -Ddisable_apps='*' ${platform[@]+"${platform[@]}"} \
            -Denable_drivers=bus/pci,bus/vdev,mempool/ring,net/ena build &&
        ninja -C build && sudo ninja -C build install) >"$blog" 2>&1 ||
        { echo "build_dpdk: DPDK $DPDK_VERSION did not build under $prefix ($blog): $(grep -m1 -E 'ERROR|error:' "$blog" || tail -1 "$blog")" >&2; return 1; }
    if [[ -n $patch ]]; then echo "$sum" | sudo tee "$prefix/ergon-lab-patch.sha256" >/dev/null; fi
}

# build_driver <marker>: the media driver, with the DPDK transport (static,
# dpdk). On aarch64 (Graviton), should md-dpdk's dpdk feature not build there
# (an older checkout's build.rs and lib.rs allow x86_64 only, as a LAB_BASE
# revision may), without it: the kernel transports only, and why in
# <marker>, which `driver`, `criterion`, `host` and lib.sh's dpdk_ready read,
# so a DPDK mode is refused before it starts, never run on the kernel. x86
# must build with it.
build_driver() {
    local marker=$1
    rm -f "$marker"
    if [[ $ARCH != aarch64 ]]; then
        cargo build --release -p ergon-md-dpdk --features static,dpdk --bin media_driver
        return
    fi
    if cargo build --release -p ergon-md-dpdk --features static,dpdk --bin media_driver 2>&1 | tee "$marker.log"; then
        rm -f "$marker.log"
        return 0
    fi
    # the gate's own words (md-dpdk's build.rs or lib.rs) over cargo's
    # generic "failed to run custom build command"
    echo "md-dpdk's dpdk feature did not build on aarch64: $({ grep -m1 'requires Linux' "$marker.log" || grep -m1 '^error' "$marker.log"; } | cut -c1-200)" >"$marker"
    echo "build: $(cat "$marker"); the media driver without it, kernel modes only" >&2
    cargo build --release -p ergon-md-dpdk --features static --bin media_driver
}

# client_features <marker>: the benchmark clients' cargo features, the media
# driver's: static, and dpdk unless build_driver left <marker> (no DPDK
# transport here) or this checkout's harness has no dpdk feature (a LAB_BASE
# revision from before it). Under INVOKER each client embeds the driver with
# the transport the mode selects, and a client built without dpdk refuses
# dpdk-ena (FeatureDisabled) at every start.
client_features() {
    if [[ ! -e $1 ]] && grep -q '^dpdk = ' md-dpdk/harness/Cargo.toml 2>/dev/null; then
        echo static,dpdk
    else
        echo static
    fi
}

# trace_anchor: "ergon-anchor <CLOCK_REALTIME ns>" into the trace from the
# app CPU (traced: tracing_cpumask drops a marker from any other), the clock
# read just before the write, so spikes.py puts the trace's events on the
# wall clock to a few us (/proc/uptime has 10 ms). Never fatal.
trace_anchor() {
    sudo taskset -c "$APP_CPU" python3 -c 'import os, time
fd = os.open("/sys/kernel/tracing/trace_marker", os.O_WRONLY)
t = time.time_ns()
os.write(fd, b"ergon-anchor %d" % t)' 2>/dev/null || echo "trace_anchor: no marker written (the trace stays on /proc/uptime's 10 ms)" >&2
}

# gapspin_us: the diagnostics' gap spinner's threshold, us (lab.sh's
# LAB_GAPSPIN_US, which lib.sh leaves in bench/gapspin_us; 50 without it)
gapspin_us() {
    local us
    us=$(cat "$run/gapspin_us" 2>/dev/null) || us=
    [[ $us =~ ^[1-9][0-9]{0,5}$ ]] || us=50
    echo "$us"
}

# KNOISE: guest-kernel noise on the agent and app CPUs, with durations, in a
# tracefs instance of its own (knoise: its own buffer and cpumask, every CPU,
# apart from diag-start's trace). knoise_start <cpus> (a list: 1,2,3,4), as
# root, against $LAB_SYS_ROOT/kernel/tracing:
#   * the osnoise tracer with NO_OSNOISE_WORKLOAD on <cpus>: no workload
#     thread, it times what else runs there (irq_noise, softirq_noise,
#     thread_noise, nmi_noise: each a start and a net duration);
#   * who sends IPIs, with their callsite (ipi_send_cpumask, ipi_send_cpu:
#     spikes.py keeps those to <cpus>), compaction (mm_compaction_begin) and
#     page migration (mm_migrate_pages), on every CPU;
#   * cpumasks printed as lists where the kernel has the option (bitmask-list).
# Enabling a tracer or event flips static keys, an IPI to every CPU (arm64:
# kick_all_cpus_sync), so diag-start runs it only before reps. Prints one
# "osnoise ..." line, "osnoise unavailable: <why>" where the kernel lacks
# the tracer (a limit of the run's diagnostics, recorded: lib.sh
# record_knoise; also where NO_OSNOISE_WORKLOAD cannot be set, before Linux
# 6.3), and "knoise events missing: ..." for events it lacks.
# On Kubernetes this is node configuration (root on the node's tracefs: a
# privileged node DaemonSet), never the app pod's.
knoise_start() {
    local cpus=$1 t=$LAB_SYS_ROOT/kernel/tracing i e missing= old_cpus got
    i=$t/instances/knoise
    if [[ ! -d $t ]]; then
        echo "osnoise unavailable: no tracefs at $t"
        return 0
    fi
    mkdir -p "$i" || { echo "osnoise unavailable: cannot create the tracefs instance $i"; return 0; }
    echo 0 >"$i/tracing_on"
    { echo nop >"$i/current_tracer"; } 2>/dev/null
    : >"$i/trace"
    echo 8192 >"$i/buffer_size_kb"
    [[ -e $i/options/bitmask-list ]] && echo 1 >"$i/options/bitmask-list"
    for e in ipi/ipi_send_cpumask ipi/ipi_send_cpu compaction/mm_compaction_begin migrate/mm_migrate_pages; do
        if [[ -e $i/events/$e/enable ]]; then echo 1 >"$i/events/$e/enable"; else missing+=" $e"; fi
    done
    if ! grep -qw osnoise "$t/available_tracers" 2>/dev/null; then
        echo "osnoise unavailable: the kernel has no osnoise tracer (available_tracers: $(cat "$t/available_tracers" 2>/dev/null))"
    elif ! { echo NO_OSNOISE_WORKLOAD >"$t/osnoise/options"; } 2>/dev/null ||
        ! grep -qw NO_OSNOISE_WORKLOAD "$t/osnoise/options" 2>/dev/null; then
        # Before Linux 6.3 osnoise has no options: the tracer would start its
        # workload threads, a CPU hog on every agent CPU, so it stays off
        echo "osnoise unavailable: this osnoise tracer has no NO_OSNOISE_WORKLOAD option (Linux 6.3 and later), so it would run its workload on the agent CPUs"
    else
        old_cpus=$(cat "$t/osnoise/cpus")
        echo "$cpus" >"$t/osnoise/cpus"
        { echo osnoise >"$i/current_tracer"; } 2>/dev/null
        got=$(cat "$i/current_tracer" 2>/dev/null) || got=unreadable
        if [[ $got == osnoise ]]; then
            for e in irq_noise softirq_noise thread_noise nmi_noise; do
                if [[ -e $i/events/osnoise/$e/enable ]]; then echo 1 >"$i/events/osnoise/$e/enable"; else missing+=" osnoise/$e"; fi
            done
            echo "osnoise on cpus $cpus (NO_OSNOISE_WORKLOAD: noise timed on them, no workload)"
        else
            # (a kernel whose instances cannot run it): its settings put back
            echo OSNOISE_WORKLOAD >"$t/osnoise/options"
            echo "$old_cpus" >"$t/osnoise/cpus"
            echo "osnoise unavailable: the knoise instance did not take the osnoise tracer (it has $got)"
        fi
    fi
    [[ -z $missing ]] || echo "knoise events missing:$missing"
    echo 1 >"$i/tracing_on"
}

# knoise_stop: KNOISE off (diag-stop, after the reps): the instance's tracing
# and tracer, the osnoise workload option back to its default, the instance
# removed where the kernel lets it
knoise_stop() {
    local t=$LAB_SYS_ROOT/kernel/tracing
    [[ -d $t/instances/knoise ]] || return 0
    echo 0 >"$t/instances/knoise/tracing_on"
    echo nop >"$t/instances/knoise/current_tracer"
    [[ -f $t/osnoise/options ]] && echo OSNOISE_WORKLOAD >"$t/osnoise/options"
    rmdir "$t/instances/knoise" 2>/dev/null || true
}

case ${1:-} in
bootstrap)
    # x86_64; aarch64 (Graviton) only with lab.sh's LAB_ALLOW_ARM=1, which
    # lib.sh leaves in ~/bench/allow_arm: production runs on x86
    host_arch_ok "$ARCH" "$(cat "$run/allow_arm" 2>/dev/null || true)" || exit 2
    deb=$(debian_arch "$ARCH")
    # Debian 13 with the trixie-backports kernel (MANA DPDK needs Linux 6.14+;
    # AWS gets the same kernel for a like-for-like comparison) and its headers
    # (AWS builds igb_uio against them), DPDK 24.11 and the build tools. The
    # backports kernel brings backports libelf, so libelf-dev comes from there too.
    echo 'deb http://deb.debian.org/debian trixie-backports main' | sudo tee /etc/apt/sources.list.d/backports.list >/dev/null
    sudo apt-get update -q
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq -t trixie-backports "linux-image-cloud-$deb" "linux-headers-cloud-$deb"
    # perf, for profiling only (never fatal): the backports kernel's, else
    # trixie's (an older perf reads a newer kernel); a versioned binary
    # (perf_<version>) gets its plain name
    command -v perf >/dev/null ||
        sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq -t trixie-backports linux-perf ||
        sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq -t trixie linux-perf || true
    if ! command -v perf >/dev/null; then
        versioned=$(ls /usr/bin/perf_* 2>/dev/null | sort -V | tail -1) || true
        [[ -z $versioned ]] || sudo ln -sf "$versioned" /usr/local/bin/perf
    fi
    command -v perf >/dev/null || echo "vm.sh: no perf installed (profiling unavailable)" >&2
    if command -v perf >/dev/null; then perf --version; fi
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq build-essential pkg-config cmake clang libclang-dev \
        libbsd-dev uuid-dev default-jdk-headless git curl python3 ethtool iputils-ping dpdk dpdk-dev libdpdk-dev \
        libelf-dev/trixie-backports
    if [[ $CLOUD == azure ]]; then
        sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq rdma-core ibverbs-providers libibverbs-dev
        echo mana_ib | sudo tee /etc/modules-load.d/mana_ib.conf >/dev/null
    fi
    if precision_time; then
        sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq meson ninja-build python3-pyelftools libnuma-dev
    fi
    # AWS: chrony on the Amazon Time Sync Service, in systemd-timesyncd's
    # place (precision-time: on the PHC first from the boot after build-ptp)
    timesync_setup
    command -v cargo >/dev/null || [[ -x ~/.cargo/bin/cargo ]] ||
        curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
    ~/.cargo/bin/rustup toolchain install "$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)" --profile minimal
    # 2 MiB hugepages for the DPDK EAL and the hugetlbfs AERON_DIR, every boot
    echo 'vm.nr_hugepages = 2048' | sudo tee /etc/sysctl.d/90-dpdk.conf >/dev/null
    # two NICs on one subnet: replies leave by the first, so loose reverse-path filtering
    printf 'net.ipv4.conf.all.rp_filter = 2\nnet.ipv4.conf.default.rp_filter = 2\nnet.core.rmem_max = 16777216\nnet.core.wmem_max = 16777216\n' |
        sudo tee /etc/sysctl.d/90-bench.conf >/dev/null
    # what this architecture needs on every boot from the next (host.sh
    # boot_args: Graviton metal's SMMU in passthrough, for igb_uio), so the
    # pinned state has it as the isolated one does
    args=$(boot_args "$ARCH" "$CLOUD")
    if [[ -n $args ]]; then
        sudo mkdir -p /etc/default/grub.d
        echo "GRUB_CMDLINE_LINUX_DEFAULT=\"\$GRUB_CMDLINE_LINUX_DEFAULT $args\"" | sudo tee /etc/default/grub.d/98-ergon-boot.cfg >/dev/null
        sudo update-grub >/dev/null 2>&1
        echo "bootstrap: every boot from the next with $args"
    fi
    # the data NICs are the lab's alone, from the next boot on (`isolate`)
    data_unmanaged
    ;;
build)
    export PATH=$HOME/.cargo/bin:$PATH
    precision_time && "$self" build-ptp
    build_driver "$run/dpdk_unavailable"
    cargo build --release -p ergon-md-dpdk-harness --features "$(client_features "$run/dpdk_unavailable")" \
        --bin rtt --bin load --bin tput --bin floor --bin counters --bin clockcheck
    if [[ $CLOUD == aws ]]; then
        # No IOMMU on virtualised EC2 and no write-combining in mainline
        # vfio-pci, which ENA's low-latency queues need: Amazon's documented
        # route is igb_uio with wc_activate=1, built for the running kernel.
        [[ -d $HOME/dpdk-kmods ]] || git clone -q https://dpdk.org/git/dpdk-kmods "$HOME/dpdk-kmods"
        # a kernel module: Kbuild's flags, never the userspace build's (build_env)
        env -u CFLAGS -u RUSTFLAGS make -s -C "$HOME/dpdk-kmods/linux/igb_uio" >/dev/null
    fi
    # @NAPI_BUSY's netlink tool, where lib.sh asked for it (a mode has it);
    # never fatal here: vm.sh ready says why it is missing, before any state
    if [[ $(cat "$run/want_ynl" 2>/dev/null) == 1 ]]; then
        "$self" build-ynl || echo "build: no ynl ($(cat "$run/ynl_unavailable" 2>/dev/null)): @NAPI_BUSY modes are refused (vm.sh ready)" >&2
    fi
    # the pure floor's client (dpdk-pure), against the DPDK the media driver
    # links (pkg-config's libdpdk: Debian's, or build-ptp's); never fatal
    # here: vm.sh ready refuses a dpdk-pure mode without it, saying why
    rm -f "$run/purefloor" "$run/purefloor_unavailable"
    # shellcheck disable=SC2046,SC2086
    if ! gcc -O2 -Wall -Wextra -Werror ${CFLAGS:-} $(pkg-config --cflags libdpdk) -o "$run/purefloor" "$(dirname "$self")/purefloor.c" \
        $(pkg-config --libs libdpdk) 2>"$run/purefloor.err"; then
        rm -f "$run/purefloor"
        echo "purefloor.c did not compile: $(grep -m1 'error' "$run/purefloor.err" || tail -1 "$run/purefloor.err")" >"$run/purefloor_unavailable"
        echo "build: $(cat "$run/purefloor_unavailable"): dpdk-pure modes are refused (vm.sh ready)" >&2
    fi
    # the DPDK builds the modes' @DPDK= ask for, where lib.sh named them
    for b in $(cat "$run/want_dpdk" 2>/dev/null); do "$self" build-dpdk "$b"; done
    ;;
build-ynl)
    # @NAPI_BUSY's tool: ynl, the kernel's own (tools/net/ynl), and its
    # netdev spec (Documentation/netlink/specs/netdev.yaml, with the
    # genetlink schema ynl checks it against). No Debian package ships ynl,
    # so both come from Debian's source of the running kernel: linux-source-
    # <major>.<minor> at the archive's version of exactly this release (its
    # tarball, with Debian's patches), so the tool and the spec are this
    # kernel's netlink family's, pinned by that version, from the signed
    # archive. Taken out of the tarball, the package purged. Fails closed:
    # why in $run/ynl_unavailable, which vm.sh ready (lib.sh, before any
    # state) and `driver` report. (LAB_SRC_ROOT: the tarball's directory,
    # /usr/src; test/test-lab.sh's fake.)
    rm -rf "$run/ynl" "$run/ynl.version" "$run/ynl_unavailable"
    unavailable() { echo "$*" | tee "$run/ynl_unavailable" >&2; exit 1; }
    src=${LAB_SRC_ROOT:-/usr/src}
    krel=$(uname -r)
    release=${krel%%[+-]*}
    [[ $release =~ ^([0-9]+)\.([0-9]+) ]] || unavailable "kernel $krel: no release in its name"
    pkg=linux-source-${BASH_REMATCH[1]}.${BASH_REMATCH[2]}
    # the archive's versions of it (newest first), and this release's
    versions=$(apt-cache madison "$pkg" 2>/dev/null | awk -F'|' '{gsub(/ /, "", $2); if ($2 != "") print $2}')
    ver=$(awk -v r="$release" 'substr($0, 1, length(r) + 1) == r "-" {print; exit}' <<<"$versions")
    # shellcheck disable=SC2086
    [[ -n $ver ]] || unavailable "$pkg has no $release (the running kernel's release) in the archive: only $(echo ${versions:-nothing})"
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq --no-install-recommends "$pkg=$ver" python3-yaml python3-jsonschema ||
        unavailable "apt-get install $pkg=$ver failed"
    dest=$HOME/ynl
    rm -rf "$dest" && mkdir -p "$dest"
    # the tarball's top directory, then the tool and the specs alone
    top=$(tar -tJf "$src/$pkg.tar.xz" 2>/dev/null | head -1 || true)
    top=${top%%/*}
    extracted=1
    tar -xJf "$src/$pkg.tar.xz" -C "$dest" --strip-components=1 "$top/tools/net/ynl" "$top/Documentation/netlink" 2>"$run/ynl.err" || extracted=
    sudo DEBIAN_FRONTEND=noninteractive apt-get purge -yq "$pkg" >/dev/null || true
    [[ -n $extracted ]] || unavailable "$pkg $ver: tools/net/ynl or Documentation/netlink not in $src/$pkg.tar.xz ($(tail -1 "$run/ynl.err"))"
    spec=$dest/Documentation/netlink/specs/netdev.yaml
    cli=$dest/tools/net/ynl/pyynl/cli.py
    # (before Linux 6.14 the tool sat a directory up)
    [[ -f $cli ]] || cli=$dest/tools/net/ynl/cli.py
    [[ -f $cli && -f $spec ]] || unavailable "$pkg $ver: no ynl ($cli) or netdev spec ($spec) in it"
    grep -qw 'busy-poll' "$spec" || unavailable "the netdev spec of $pkg $ver has no threaded busy-poll (Linux 6.17 and later)"
    printf '#!/bin/sh\nexec python3 %s --spec %s "$@"\n' "'$cli'" "'$spec'" >"$run/ynl"
    chmod +x "$run/ynl"
    "$run/ynl" --help >/dev/null 2>"$run/ynl.err" || { rm -f "$run/ynl"; unavailable "$pkg $ver: ynl does not start ($(tail -1 "$run/ynl.err"))"; }
    echo "$pkg $ver (Linux $krel): tools/net/ynl and Documentation/netlink" | tee "$run/ynl.version"
    ;;
build-base)
    # lab.sh LAB_BASE: the revision lib.sh unpacked in ~/ergon-base (its
    # tracked files, from git archive), built as `build` builds this checkout,
    # and its media driver and benchmark clients installed in ~/bin-base, for
    # modes with @BIN=bin-base. A client the revision lacks is left out.
    export PATH=$HOME/.cargo/bin:$PATH
    cd "$HOME/ergon-base" 2>/dev/null || { echo "build-base: no ~/ergon-base (lab.sh LAB_BASE)" >&2; exit 2; }
    if [[ -f rust-toolchain.toml ]]; then
        rustup toolchain install "$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)" --profile minimal
    fi
    build_driver "$run/dpdk_unavailable-base"
    clients=()
    for c in rtt load tput floor; do
        if [[ -f md-dpdk/harness/src/bin/$c.rs ]]; then clients+=(--bin "$c"); fi
    done
    if ((${#clients[@]})); then
        cargo build --release -p ergon-md-dpdk-harness --features "$(client_features "$run/dpdk_unavailable-base")" "${clients[@]}"
    fi
    rm -rf "$HOME/bin-base" && mkdir -p "$HOME/bin-base"
    for b in media_driver rtt load tput floor; do
        if [[ -x target/release/$b ]]; then cp "target/release/$b" "$HOME/bin-base/"; fi
    done
    echo "bin-base $(ls "$HOME/bin-base" | tr '\n' ' ')"
    ;;
build-dpdk)
    # vm.sh build-dpdk <build>: a mode's @DPDK=<build> (modes.sh DPDK_BUILDS),
    # built where lib.sh's bench/want_dpdk names it (`build`): DPDK 25.11 from
    # source (build_dpdk; patched-refill with dpdk-ena-deferred-refill.patch
    # applied, under its own prefix; stock under $DPDK_PREFIX, precision-time's
    # build-ptp's), in any placement; then this checkout's media driver and
    # every benchmark client (under INVOKER each embeds the driver) linked
    # against it, in a target directory of their own (this checkout's build
    # stays as it was), the build's library directory their DT_RPATH
    # (--disable-new-dtags: unlike DT_RUNPATH, their libraries' own
    # dependencies and the PMDs the EAL loads are found there too, ahead of
    # ld.so.conf's), and only at the link (cargo rustc: every crate compiled
    # as this checkout's), into ~/bin-dpdk-<build>. Fail closed: every
    # librte_* each binary loads from that directory, or the build is removed.
    build=${2:-}
    [[ $DPDK_BUILDS == *" $build "* && -n $build ]] ||
        { echo "build-dpdk ${build:-(none)}: stock (DPDK 25.11 from source) or patched-refill (with dpdk-ena-deferred-refill.patch on ENA's PMD)" >&2; exit 2; }
    [[ $CLOUD == aws ]] || { echo "build-dpdk $build: AWS only (a build of ENA's PMD)" >&2; exit 2; }
    export PATH=$HOME/.cargo/bin:$PATH
    # what `host` (the env file) records of this build: only once it passed
    rm -f "$run/bin-dpdk-$build.txt"
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq meson ninja-build python3-pyelftools libnuma-dev
    prefix=$DPDK_PREFIX patch=
    if [[ $build == patched-refill ]]; then
        prefix=$DPDK_PREFIX-$build patch=$(dirname "$self")/dpdk-ena-deferred-refill.patch
    fi
    build_dpdk "$prefix" "$patch" || { echo "build-dpdk $build: DPDK $DPDK_VERSION did not build" >&2; exit 1; }
    libdir=$prefix/lib/$ARCH-linux-gnu
    export PKG_CONFIG_PATH=$libdir/pkgconfig CARGO_TARGET_DIR=$HOME/ergon/target-dpdk
    link=(-- -C "link-arg=-Wl,--disable-new-dtags,-rpath,$libdir")
    cargo rustc --release -p ergon-md-dpdk --features static,dpdk --bin media_driver "${link[@]}"
    for c in rtt load tput floor; do
        cargo rustc --release -p ergon-md-dpdk-harness --features static,dpdk --bin "$c" "${link[@]}"
    done
    out=$HOME/bin-dpdk-$build
    rm -rf "$out" && mkdir -p "$out"
    for c in media_driver rtt load tput floor; do cp "$CARGO_TARGET_DIR/release/$c" "$out/"; done
    for c in "$out"/*; do
        loads=$(ldd "$c" | awk '$1 ~ /^librte_/ {print $1, ($3 == "not" ? "nowhere" : $3)}')
        [[ -n $loads ]] || { rm -rf "$out"; echo "build-dpdk $build: $c loads no librte_* library: removed" >&2; exit 1; }
        while read -r lib path; do
            [[ $path == "$libdir/"* ]] ||
                { rm -rf "$out"; echo "build-dpdk $build: $c loads $lib from $path, not $libdir: removed" >&2; exit 1; }
        done <<<"$loads"
    done
    echo "dpdk-build $build: DPDK $DPDK_VERSION in $prefix${patch:+, ${patch##*/} sha256 $(cat "$prefix/ergon-lab-patch.sha256")}: $out" >"$run/bin-dpdk-$build.txt"
    echo "bin-dpdk-$build: $(ls "$out" | tr '\n' ' ')against DPDK $DPDK_VERSION in $prefix${patch:+ (${patch##*/})}"
    ;;
build-ptp)
    # precision-time only, run by `build` before the media driver.
    # DPDK 25.11: only what the ENA path needs, installed under $DPDK_PREFIX
    # (build_dpdk), on the loader's path: this checkout's build links it.
    if [[ ! -f $DPDK_PREFIX/lib/$ARCH-linux-gnu/pkgconfig/libdpdk.pc ]]; then
        build_dpdk "$DPDK_PREFIX"
        echo "$DPDK_PREFIX/lib/$ARCH-linux-gnu" | sudo tee /etc/ld.so.conf.d/dpdk-$DPDK_VERSION.conf >/dev/null
        sudo ldconfig
    fi
    # The PTP hardware clock: Linux's own ENA driver (7.2 here) has it, on
    # the primary ENI by devlink at every boot from the next (lib.sh reboots),
    # /dev/ptp_ena for chrony (phc_setup's refclock, from that boot), which keeps
    # CLOCK_REALTIME on it: the clock the transport compares NIC RX
    # timestamps against (phc_setup). No out-of-tree driver: Amazon's own is
    # needed only for its phc_error_bound, which nothing here reads
    phc_setup
    ;;
ptp)
    if ! precision_time; then echo "ptp: not a precision-time run"; exit 0; fi
    echo "ptp $(readlink -f /dev/ptp_ena 2>/dev/null || echo missing) $(cat /sys/class/ptp/ptp*/clock_name 2>/dev/null | tr '\n' ' ')"
    bdf=$(cat "$run/phc_bdf" 2>/dev/null || true)
    echo "ena $(modinfo -F version ena 2>/dev/null) primary ENI ${bdf:-?} enable_phc: $(sudo devlink dev param show "pci/${bdf:-none}" name enable_phc 2>/dev/null | awk '/value/ {printf "%s ", $0}' || true)"
    systemctl is-active ergon-lab-phc.service 2>/dev/null | sed 's/^/ergon-lab-phc.service /' || true
    chronyc -n sources 2>/dev/null | sed 's/^/chrony /'
    chronyc tracking 2>/dev/null | sed -n 's/^\(Reference ID\|System time\|RMS offset\) *: */chrony \1: /p'
    nics | while read -r role iface _; do echo "$role $(ethtool -T "$iface" 2>/dev/null | grep -c hardware-receive) hardware-receive"; done
    ;;
timesync)
    # vm.sh timesync [check]: chrony's view of this host's clock (lib.sh logs
    # it on both hosts around every mode: spikes.py notes the hosts' offsets
    # where it matches events across them). check, before a mode's drivers
    # start: under precision-time the PHC must be chrony's selected source
    # with its last offset within PHC_MAX_OFFSET_NS (10 us), said so in the
    # record, or the mode stops here (host.sh phc_selected)
    command -v chronyc >/dev/null || { echo "timesync: no chronyc here"; exit 0; }
    chronyc -n tracking 2>&1 || true
    chronyc -n sources 2>&1 || true
    if [[ ${2:-} == check ]] && precision_time; then
        chronyc -c -n sources 2>/dev/null | phc_selected "${PHC_MAX_OFFSET_NS:-10000}" ||
            { echo "timesync: precision-time without the PHC as chrony's source: stopping" >&2; exit 1; }
    fi
    ;;
timesync-wait)
    # vm.sh timesync-wait: AWS, before each state is measured (lib.sh), until
    # chrony has settled: chronyc waitsync, 300 tries a second apart (after
    # a boot, polling every 16 s, chrony's skew falls below 1 ppm over
    # minutes, not seconds, and a run given up on here is lost), the
    # correction still to apply under 10 us and the frequency's skew under
    # 1 ppm. That bounds what chrony has yet to correct, not the clock's
    # error against UTC (the source's own: Amazon's Time Sync Service, or the
    # PHC). Tracking printed after, whatever came of it; 1 if it never
    # settled. Azure's image's chrony is left as it is: not gated
    if [[ $CLOUD != aws ]]; then
        echo "timesync-wait: Azure: the image's chrony on the Hyper-V clock (ptp_hyperv), not gated"
        exit 0
    fi
    rc=0
    chronyc waitsync 300 0.00001 1 1 || rc=1
    chronyc -n tracking 2>&1 || true
    if ((rc)); then
        echo "timesync-wait: chrony did not settle in 5 minutes (waitsync: correction under 10 us, skew under 1 ppm): the run's clocks are not to be trusted" >&2
        exit 1
    fi
    ;;
isolate)
    # rusteron's "isolated" state: every CPU but the housekeeping one off the
    # scheduler's balancing, the tick, RCU callbacks and managed IRQs; on x86
    # AWS iommu=pt, as metal has a real IOMMU, which igb_uio does not program
    # (passthrough lets its DMA through; a virtualised instance has none).
    # `isolate poll`, the isolated-poll state: idle CPUs poll too (x86
    # idle=poll, Graviton nohlt). host.sh isolation_args. Each call replaces
    # the last; all CPUs counted, as once isolated a login's own are
    # housekeeping's.
    last=$(($(nproc --all) - 1))
    args=$(isolation_args "$ARCH" "$CLOUD" "$last" "${2:-}") || exit 2
    sudo mkdir -p /etc/default/grub.d
    echo "GRUB_CMDLINE_LINUX_DEFAULT=\"\$GRUB_CMDLINE_LINUX_DEFAULT $args\"" |
        sudo tee /etc/default/grub.d/99-ergon-isolation.cfg >/dev/null
    sudo update-grub >/dev/null 2>&1
    echo "isolate: $args from the next boot"
    ;;
tune)
    # the data NICs up with their addresses and routing (unmanaged by the OS,
    # so nothing else does it at boot), before their IRQs are placed; a VF
    # not there yet is left to the first kernel run, which does it again
    data_kernel_up || echo "tune: data NICs not configured yet; the first kernel run will" >&2
    networkctl list --no-pager 2>/dev/null | sed 's/^/networkd /' || true
    pin_irqs tune
    echo 0 | sudo tee /sys/kernel/mm/ksm/run >/dev/null 2>&1 || true
    sudo sysctl -qw kernel.numa_balancing=0 2>/dev/null || true
    # the baseline's transparent hugepages, from every boot on
    echo "$BASE_THP" | sudo tee /sys/kernel/mm/transparent_hugepage/enabled >/dev/null
    # Graviton: 4 KiB pages, so 2 MiB hugepages as on x86 (Debian's arm64
    # kernel; its 16 KiB flavour would make them contiguous-PTE ones), and
    # every transparent hugepage size (mTHP, 16-64 KiB too on arm64)
    # following the top level (host.sh mthp_changes)
    if [[ $ARCH == aarch64 ]]; then
        [[ $(getconf PAGESIZE) == 4096 && -d /sys/kernel/mm/hugepages/hugepages-2048kB ]] ||
            { echo "tune: $(getconf PAGESIZE)-byte pages: the lab needs 4 KiB pages and 2 MiB hugepages (Debian's cloud-arm64 kernel)" >&2; exit 2; }
        mthp_changes 2048 | while read -r f v; do
            echo "$v" | sudo tee "$f" >/dev/null
            echo "tune: ${f#/sys/kernel/mm/transparent_hugepage/} $v"
        done
    fi
    sudo mkdir -p /mnt/huge
    mountpoint -q /mnt/huge || sudo mount -t hugetlbfs -o pagesize=2M,size=1G none /mnt/huge
    echo "cloud $CLOUD layout $(layout) cpus $(nproc)"
    echo "cmdline $(cat /proc/cmdline)"
    echo "isolated $(cat /sys/devices/system/cpu/isolated) nohz_full $(cat /sys/devices/system/cpu/nohz_full 2>/dev/null)"
    echo "clocksource $(cat /sys/devices/system/clocksource/clocksource0/current_clocksource)"
    echo "thp $(cat /sys/kernel/mm/transparent_hugepage/enabled)"
    "$self" host || true
    ;;
baseline)
    # what every mode runs on top of, and what is in effect now
    echo "baseline thp=$BASE_THP (now: $(cat /sys/kernel/mm/transparent_hugepage/enabled)); dpdk modes: AERON_RECEIVER_IO_VECTOR_CAPACITY=$BASE_RCV_IO_VECTORS"
    ;;
host)
    # the architecture and the host settings in effect; each read falls back,
    # so a missing file shows as such and never fails the step
    echo "arch $ARCH (Debian $(debian_arch "$ARCH" || echo ?), kernel $(uname -r))"
    if [[ $ARCH == aarch64 ]]; then
        awk -F': *' '/^CPU implementer/ {i = $2} /^CPU part/ {p = $2} END {
            n = p == "0xd40" ? "Neoverse V1, Graviton3" : p == "0xd4f" ? "Neoverse V2, Graviton4" : p == "0xd84" ? "Neoverse V3, Graviton5" : p == "0xd0c" ? "Neoverse N1, Graviton2" : "?"
            printf "cpu implementer %s part %s (%s)\n", i, p, n}' /proc/cpuinfo
        echo "counter $(sudo dmesg 2>/dev/null | grep -m1 -o 'arch_timer: cp15 timer(s) running at [^ ]*' || echo 'frequency not in dmesg') $(grep -qw ecv /proc/cpuinfo && echo '(ECV: self-synchronised reads)' || echo '(no ECV)')"
    fi
    echo "smt $(cat /sys/devices/system/cpu/smt/active 2>/dev/null || echo n/a) (cpu1's siblings $(cat /sys/devices/system/cpu/cpu1/topology/thread_siblings_list 2>/dev/null || echo ?))"
    echo "idle $(cat /sys/devices/system/cpu/cpuidle/current_driver 2>/dev/null || echo 'none: no cpuidle driver') $(grep -ow 'nohlt\|idle=poll' /proc/cmdline | tr '\n' ' ')"
    echo "clocksource $(cat /sys/devices/system/clocksource/clocksource0/current_clocksource 2>/dev/null)"
    echo "pages $(getconf PAGESIZE) hugepages-2048kB $(cat /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages 2>/dev/null || echo none) free $(cat /sys/kernel/mm/hugepages/hugepages-2048kB/free_hugepages 2>/dev/null || echo -)"
    echo "thp $(sed 's/.*\[\(.*\)\].*/\1/' /sys/kernel/mm/transparent_hugepage/enabled 2>/dev/null) sizes $(for f in /sys/kernel/mm/transparent_hugepage/hugepages-*kB/enabled; do
        [[ -e $f ]] && printf '%s=%s ' "$(basename "$(dirname "$f")" | sed 's/hugepages-//')" "$(sed 's/.*\[\(.*\)\].*/\1/' "$f")"; done)"
    echo "iommu groups $(find /sys/kernel/iommu_groups -mindepth 1 -maxdepth 1 2>/dev/null | wc -l | tr -d ' ') $(grep -o 'iommu[.=][^ ]*' /proc/cmdline | tr '\n' ' ')"
    { nics 2>/dev/null || true; } | while read -r role iface _ _ _ _ bdf; do
        group=/sys/bus/pci/devices/$bdf/iommu_group
        echo "nic $role $bdf numa $(cat "/sys/bus/pci/devices/$bdf/numa_node" 2>/dev/null || echo ?)" \
            "iommu $([[ -e $group ]] && cat "$group/type" 2>/dev/null || echo none)" \
            "rps $(cat /sys/class/net/"$iface"/queues/rx-*/rps_cpus 2>/dev/null | sort -u | tr '\n' ' ' || echo n/a)"
    done
    echo "ena $(for p in rx_queue_size enable_bql force_large_llq_header lpc_size phc_enable; do printf '%s=%s ' "$p" "$(cat "/sys/module/ena/parameters/$p" 2>/dev/null || echo -)"; done)"
    echo "dpdk $(pkg-config --modversion libdpdk 2>/dev/null || echo none)$([[ -s $run/dpdk_unavailable ]] && echo ", not in the media driver: $(cat "$run/dpdk_unavailable")")"
    # each @DPDK= build's DPDK (build-dpdk): the modes with it link that one
    # rather than the line above's
    for f in "$run"/bin-dpdk-*.txt; do if [[ -f $f ]]; then cat "$f"; fi; done
    echo "ynl $(cat "$run/ynl.version" 2>/dev/null || cat "$run/ynl_unavailable" 2>/dev/null || echo 'not built (no @NAPI_BUSY mode)')"
    echo "build CFLAGS=${CFLAGS:-} RUSTFLAGS=${RUSTFLAGS:-}"
    if [[ $ARCH == aarch64 ]]; then
        echo "skipped as x86-only: C-state caps (@CSTATE: no cpuidle driver, idle is WFI; isolated-poll boots nohlt)," \
            "PAT memory types (wc-check: arm64 has none; the resource2_wc mapping is the evidence)," \
            "iommu=pt (arm64 takes iommu.passthrough=1, from bootstrap)"
    fi
    ;;
layout) layout ;;
ready)
    # vm.sh ready <mode>...: what each mode's settings need of this host,
    # asked by lib.sh after the build, before any state (a mode that cannot
    # run here would fail at its first driver start, hours in): a fifth core
    # for @IRQ=spare, @NAPI_BUSY=spare and @CONDUCTOR=own; Linux 6.17 or later and the
    # kernel's ynl (build-ynl) for @NAPI_BUSY; its build (build-dpdk) for
    # @DPDK=. A line for each mode that cannot run here, then exit 2;
    # nothing, exit 0, when all can
    shift
    rc=0
    for m in "$@"; do
        if mode_hairpin "$m" && [[ -z ${RECEIVER_CPU:-} ]]; then
            echo "mode $m: no receiver core on this host (its layout $(layout)): the hairpin's pong runs there; four cores (c9g.xlarge and up)"
            rc=2
            continue
        fi
        if b=$(mode_get "$m" DPDK) && [[ ! -x $HOME/bin-dpdk-$b/media_driver ]]; then
            echo "mode $m: no DPDK=$b build: no media_driver in $HOME/bin-dpdk-$b (vm.sh build-dpdk $b, which vm.sh build runs for each build bench/want_dpdk names)"
            rc=2
            continue
        fi
        if mode_spare "$m" && ! why=$(setting_cpu spare DEDICATED "$HK_CPU" "$APP_CPU" "$SENDER_CPU" "$RECEIVER_CPU" "${APP2_CPU:-}" 2>&1 >/dev/null); then
            echo "mode $m: $why"
            rc=2
            continue
        fi
        if mode_pure "$m" && [[ ! -x $run/purefloor ]]; then
            echo "mode $m: purefloor did not build here (vm.sh build): $(cat "$run/purefloor_unavailable" 2>/dev/null || echo "no $run/purefloor")"
            rc=2
            continue
        fi
        mode_get "$m" NAPI_BUSY >/dev/null || continue
        if ! kernel_at_least "$(uname -r)" 6 17; then
            echo "mode $m: threaded NAPI busy polling needs Linux 6.17 or later (this is $(uname -r))"
            rc=2
        elif ! why=$(ynl_ready 2>&1); then
            echo "mode $m: $why"
            rc=2
        fi
    done
    exit $rc
    ;;
nics) nics ;;
driver)
    mode=${2:?kernel, kernel-busy, dpdk or dpdk-spin, optionally -mono, then @KEY=value overrides}
    "$self" stop
    # `stop` removed any earlier run's BIN: this run's own, if any, follows
    bin=${BENCH_BIN:-$tools}
    # the last run's DPDK event log (a mode's @RUSTERON_DPDK_EVENT_LOG=
    # <home>/bench/events.csv): a run without it must leave none to fetch
    rm -f "$run/events.csv"
    env_file=$run/driver.env
    # <mode>@KEY=value@...: environment overrides for one experiment, applied
    # last so they win (e.g. dpdk@RUSTERON_DPDK_BURST_SIZE=32)
    overrides=
    if [[ $mode == *@* ]]; then
        overrides=${mode#*@}
        mode=${mode%%@*}
    fi
    # @AERON_THREADING_MODE=SHARED_NETWORK|SHARED: the agents' threads and
    # CPUs (host.sh threading_env), set before the other overrides, which can
    # then change an idle strategy
    threading=$(threading_of "$overrides") || exit 2
    # kernel-raw, dpdk-raw: the raw floor (the floor client, no Aeron) on
    # kernel-busy's or dpdk's set-up (modes.sh raw_base), its one thread
    # sending and receiving as INVOKER's does (threading_env); no media
    # driver, as under INVOKER (below)
    raw=$(raw_base "$mode") floor_wire=
    if [[ -n $raw ]]; then
        threading=INVOKER floor_wire=${raw%-busy}
    fi
    # dpdk-pure: dpdk's set-up for purefloor (DPDK alone), its wire pure,
    # which only purefloor takes
    if mode_pure "$mode"; then
        [[ -x $run/purefloor ]] ||
            { echo "dpdk-pure: purefloor did not build here (vm.sh build): $(cat "$run/purefloor_unavailable" 2>/dev/null || echo "no $run/purefloor")" >&2; exit 2; }
        floor_wire=pure
    fi
    common_env "${threading:-DEDICATED}" >"$env_file" || exit 2
    clock=tsc
    if [[ $mode == *-mono ]]; then clock=monotonic; mode=${mode%-mono}; fi
    [[ -n $raw ]] && mode=$raw
    echo "export RUSTERON_DPDK_CLOCK='$clock'" >>"$env_file"
    echo "$clock" >"$run/bench_clock"
    # dpdk-spin: the DPDK driver with its agents reading host-local kernel
    # sockets every duty cycle, not when the local-I/O thread flags them
    if [[ $mode == dpdk-spin || $mode == dpdk-adaptive || $mode == dpdk-epoll ]]; then
        echo "export RUSTERON_DPDK_LOCAL_IO='${mode#dpdk-}'" >>"$env_file"
        mode=dpdk
    fi
    case $mode in
    kernel | kernel-busy)
        data_kernel_up
        busy=0
        [[ $mode == kernel-busy ]] && busy=50
        sudo sysctl -qw net.core.busy_read=$busy net.core.busy_poll=$busy
        nics | while read -r _ _ vf _; do
            if [[ $mode == kernel-busy ]]; then defer=2 gro=200000; else defer=0 gro=0; fi
            echo $defer | sudo tee /sys/class/net/"$vf"/napi_defer_hard_irqs >/dev/null
            echo $gro | sudo tee /sys/class/net/"$vf"/gro_flush_timeout >/dev/null
        done
        # the data NICs named, so the driver's NIC checks run (under DPDK they
        # are the ports, by PCI address, and the variable is ignored)
        data_nics_env >>"$env_file" || { echo "$mode: the data NICs not read for RUSTERON_MEDIA_DRIVER_DATA_NICS" >&2; exit 2; }
        ;;
    dpdk)
        # a build without md-dpdk's dpdk feature (vm.sh build): no DPDK here
        if [[ -s $run/dpdk_unavailable && $bin == "$tools" ]]; then
            echo "dpdk: this host's media driver has no DPDK transport: $(cat "$run/dpdk_unavailable")" >&2
            exit 2
        fi
        if [[ $CLOUD == azure ]]; then
            sudo modprobe mana_ib
            sudo modprobe uio_hv_generic
            # the network-device class GUID, so uio_hv_generic may bind NICs
            echo f8615163-df3e-46c5-913f-f2d2f965ed0e | sudo tee /sys/bus/vmbus/drivers/uio_hv_generic/new_id >/dev/null 2>&1 || true
        else
            sudo modprobe uio
            lsmod | grep -q '^igb_uio' || sudo insmod "$HOME/dpdk-kmods/linux/igb_uio/igb_uio.ko" wc_activate=1
            # Graviton: igb_uio gives the ENA physical addresses, so metal's
            # SMMU must pass its DMA through (bootstrap's iommu.passthrough=1)
            if [[ $ARCH == aarch64 ]]; then
                iommu_passthrough_ok $(nics | awk '{print $7}') || exit 2
            fi
        fi
        {
            echo "export RUSTERON_MEDIA_DRIVER_TRANSPORT='dpdk-ena'"
            echo "export RUSTERON_DPDK_FILE_PREFIX='ergon-bench'"
            # the baseline: the receiver drains up to this many frames a duty
            # cycle (an @AERON_RECEIVER_IO_VECTOR_CAPACITY=n override, later, wins)
            echo "export AERON_RECEIVER_IO_VECTOR_CAPACITY='$BASE_RCV_IO_VECTORS'"
            # the EAL's only lcore is the housekeeping CPU: no worker threads
            # elsewhere. On AWS the ENA PMD's and the PCI bus's probe messages
            # at info: whether the low-latency queues and write-combining took
            # (vm.sh pmd-log)
            eal="-l $HK_CPU"
            if [[ $CLOUD == aws ]]; then eal="$eal --log-level=pmd.net.ena.*:info --log-level=bus.pci:info"; fi
            echo "export RUSTERON_DPDK_EAL_ARGS='$eal'"
            if [[ $CLOUD == azure ]]; then
                # MANA queues hold at most 256 descriptors
                # (LAB_RX_DESC: a smaller ring, for loss experiments)
                echo "export RUSTERON_DPDK_RX_DESCRIPTORS='${LAB_RX_DESC:-256}'"
                echo "export RUSTERON_DPDK_TX_DESCRIPTORS='256'"
            fi
            echo "export RUSTERON_DPDK_MBUFS_PER_PORT='16384'"
            # precision-time: NIC RX timestamps on every port. Asking for them
            # changes the ENA's RX descriptors (each completion carries its
            # timestamp), so such a run is not the cluster placement's DPDK:
            # aws/lab.sh refuses one without a DPDK baseline that turns them
            # off (@RUSTERON_DPDK_RX_HW_TIMESTAMP=0, an override, so later)
            precision_time && echo "export RUSTERON_DPDK_RX_HW_TIMESTAMP='1'"
            data_nics=$(nics)
            while read -r role iface vf mac ip prefix bdf; do
                R=${role^^}
                # the subnet's gateway: the next hop to a peer off it (cross-AZ, cross-region)
                gw=$(subnet_gateway "$ip/$prefix")
                echo "export RUSTERON_DPDK_${R}_PCI='$bdf'"
                echo "export RUSTERON_DPDK_${R}_IPV4_CIDR='$ip/$prefix'"
                echo "export RUSTERON_DPDK_${R}_GATEWAY='$gw'"
                sudo ip link set "$iface" down
                if [[ $CLOUD == azure ]]; then
                    # The netvsc PMD takes the NIC's synthetic channel (moved to
                    # uio_hv_generic) and drives the MANA VF beneath it, keeping
                    # the VF data path switched in; the port is selected by MAC.
                    echo "export RUSTERON_DPDK_${R}_MAC='$mac'"
                    dev=$(basename "$(readlink /sys/class/net/"$iface"/device)")
                    sudo ip link set "$vf" down
                    echo "$dev" | sudo tee /sys/bus/vmbus/drivers/hv_netvsc/unbind >/dev/null
                    echo "$dev" | sudo tee /sys/bus/vmbus/drivers/uio_hv_generic/bind >/dev/null
                else
                    # The ENA moves from the kernel's ena driver to igb_uio.
                    echo igb_uio | sudo tee /sys/bus/pci/devices/"$bdf"/driver_override >/dev/null
                    echo "$bdf" | sudo tee /sys/bus/pci/devices/"$bdf"/driver/unbind >/dev/null
                    echo "$bdf" | sudo tee /sys/bus/pci/drivers_probe >/dev/null
                fi
            done <<<"$data_nics"
        } >>"$env_file"
        ;;
    *) echo "unknown mode $mode" >&2; exit 2 ;;
    esac
    if [[ -n $overrides ]]; then
        IFS=@ read -ra pairs <<<"$overrides"
        for pair in "${pairs[@]}"; do
            [[ $pair =~ ^[A-Z][A-Z0-9_]*=[^\'[:space:]]*$ ]] || { echo "bad override $pair" >&2; exit 2; }
            # in common_env already
            [[ $pair == AERON_THREADING_MODE=* ]] && continue
            # a host setting (host_setting) for this run, which `stop` undoes;
            # anything else is the driver's environment
            host_setting "${pair%%=*}" "${pair#*=}" "$mode" && continue
            override_export "$pair" >>"$env_file"
        done
    fi
    # The conductor's duty-cycle threshold, derived from whatever idle
    # strategy the overrides above left in play: too low and every full park
    # counts as a stall (conductor_cycle_threshold). Fail closed here, before
    # anything this run placed is left behind.
    if ! conductor_cycle_threshold_apply "$env_file"; then
        echo "$mode: the conductor's duty-cycle threshold is not above its idle strategy's longest park: stopping" >&2
        exit 2
    fi
    # what this run ran on, for its notes: the baseline as in effect now
    # (after any override) and whose binaries
    log "driver: $mode${overrides:+ with $overrides} (thp $(sed 's/.*\[\(.*\)\].*/\1/' /sys/kernel/mm/transparent_hugepage/enabled), receiver IO vectors $(sed -n "s/^export AERON_RECEIVER_IO_VECTOR_CAPACITY='\(.*\)'$/\1/p" "$env_file" | tail -1 | grep . || echo "Aeron's default"), binaries $bin)"
    # @NAPI_BUSY=spare: after every other setting (a ring or MTU change
    # resets the NICs, their NAPIs with them): each NAPI of the data NICs
    # busy-polled by its kthread on the spare core (each spins it at 100%),
    # every one listed in $run/napi_busy for `stop`; fail closed
    if [[ -s $run/napi_busy_cpu ]]; then
        busy_cpu=$(cat "$run/napi_busy_cpu")
        ifindexes=$(nics | while read -r _ _ vf _; do cat "$LAB_SYS_ROOT/class/net/$vf/ifindex"; done)
        # shellcheck disable=SC2086
        if ! napi_busy_on "$busy_cpu" $ifindexes >"$run/napi_busy"; then
            cat "$run/napi_busy" >&2
            echo "NAPI_BUSY: threaded busy polling not set on every NAPI of the data NICs: stopping" >&2
            "$self" stop >/dev/null 2>&1 || true
            exit 1
        fi
        log "NAPI_BUSY=spare: $(wc -l <"$run/napi_busy" | tr -d ' ') NAPIs of the data NICs (ifindex $(echo $ifindexes)) threaded busy-poll, their kthreads on CPU $busy_cpu, which they share and spin at 100%: $(awk '{printf "%snapi %s pid %s", sep, $2, $6; sep = ", "}' "$run/napi_busy")"
    fi
    # Every IRQ on the housekeeping CPU once more, after everything above
    # that may have placed them anew (the NICs back from DPDK, a ring or
    # moderation change), and where each is now: the run's own record (and a
    # mode's @IRQ= queue IRQs on its CPU, every one there or the mode stops)
    pin_irqs "before the driver starts"
    if [[ -s $run/irq_cpu && $(cat "$run/irq_cpu") != "$HK_CPU" ]]; then
        # shellcheck disable=SC2046
        if ! irq_line=$(irqs_queues_check "$(cat "$run/irq_cpu")" $(data_ifaces)); then
            echo "IRQ: the data NICs' queue IRQs not all on CPU $(cat "$run/irq_cpu") ($irq_line): stopping" >&2
            "$self" stop >/dev/null 2>&1 || true
            exit 1
        fi
        log "IRQ: $irq_line"
    fi
    # The EAL needs root (hugepages, the device), so the driver runs as root in
    # every mode, and the clients (`vm.sh client`) do too, to map its files.
    sudo rm -rf "$AERON_DIR"
    # INVOKER: no media driver process. Everything above stands (the NICs,
    # the host settings, the environment in driver.env); each client embeds
    # the driver with it (`client`), and `loss` keeps a ledger of the embedded
    # drivers' counters (host.sh invoker_ledger_total)
    if [[ $threading == INVOKER ]]; then
        if [[ -n $floor_wire ]]; then
            # the floor client's wire (`client`, `stop`): no embedded driver
            echo "${floor_wire}" >"$run/floor"
        else
            echo invoker >"$run/harness_driver"
            rm -rf "$run/invoker-ledger"
        fi
        : >"$run/driver.log"
        # Graviton, DPDK: ENA's low-latency queues and write-combining, as
        # below, checked on a driver started once with the embedded drivers'
        # environment and stopped (host.sh invoker_llq_probe), fail closed;
        # its directory goes, so the first client finds no CnC unread
        if [[ $mode == dpdk && $CLOUD == aws && $ARCH == aarch64 ]]; then
            # shellcheck disable=SC2046
            if ! invoker_llq_probe "$env_file" "$HK_CPU" "$APP_CPU" "$bin/media_driver" "$run/driver.log" "$run/driver.maps" \
                $(sed -nE "s/^export RUSTERON_DPDK_(SENDER|RECEIVER)_PCI='(.+)'$/\2/p" "$env_file") >&2; then
                echo "dpdk: ENA's low-latency queues or their write-combining missing: stopping" >&2
                "$self" stop >/dev/null 2>&1 || true
                exit 1
            fi
            # @DPDK=: the probe's driver loaded the build's PMD
            if ! dpdk_variant_check "$run/driver.log"; then
                "$self" stop >/dev/null 2>&1 || true
                exit 1
            fi
            sudo rm -rf "$AERON_DIR"
        fi
        # no driver log of its own (`pmd-log` reads this, after the probe's
        # lines if any): each client's output has its driver's
        if [[ -n $floor_wire ]]; then
            echo "raw floor: no media driver process; each floor client logs its transport's lines to its output" >>"$run/driver.log"
            log "driver: raw floor ($floor_wire): no media driver process; the floor clients run with driver.env, no Aeron"
            exit 0
        fi
        echo "INVOKER: no media driver process; each client's embedded driver logs to the client's output" >>"$run/driver.log"
        log "driver: INVOKER: no media driver process; each client embeds the driver (HARNESS_DRIVER=invoker) with driver.env"
        exit 0
    fi
    # confined to the housekeeping CPU: every thread that is not an agent
    # (signal handler, Aeron's, the EAL's) stays there; the agents pin themselves
    # the transport suite's bench binaries are this user's, not root's: the
    # driver's files (cnc.dat, the log buffers) writable by them (umask 0)
    umask_cmd=
    [[ -s $run/transport-map ]] && umask_cmd='umask 0000 && '
    sudo bash -c "$umask_cmd. '$env_file' && exec taskset -c $HK_CPU '$bin/media_driver'" >"$run/driver.log" 2>&1 &
    pid=$!
    for _ in $(seq 120); do
        grep -q "media driver started" "$run/driver.log" 2>/dev/null && break
        if ! ps -p "$pid" >/dev/null; then cat "$run/driver.log" >&2; exit 1; fi
        sleep 0.5
    done
    grep -q "media driver started" "$run/driver.log" || { cat "$run/driver.log" >&2; exit 1; }
    grep "transport backend" "$run/driver.log" >&2
    # @DPDK=: the driver loaded that build's ENA PMD, fail closed
    if ! dpdk_variant_check "$run/driver.log"; then
        "$self" stop >/dev/null 2>&1 || true
        exit 1
    fi
    # SHARED's and SHARED_NETWORK's one busy agent, and @CONDUCTOR=own's
    # conductor, where they were put, fail closed: a run measured with one
    # elsewhere would be another mode's (agents_placed)
    dpid=$(pgrep -x media_driver | head -1)
    if ! agents_placed "${threading:-DEDICATED}" "/proc/$dpid"; then
        "$self" stop >/dev/null 2>&1 || true
        exit 1
    fi
    # Graviton, DPDK: every port's transmit queues in ENA's low-latency
    # queues, their BAR mapped write-combining (host.sh ena_llq_check); without
    # them a send costs about 100 times as much, so the run stops here
    if [[ $mode == dpdk && $CLOUD == aws && $ARCH == aarch64 ]]; then
        sudo cat "/proc/$(pgrep -x media_driver | head -1)/maps" >"$run/driver.maps" 2>/dev/null || true
        # shellcheck disable=SC2046
        if ! ena_llq_check "$run/driver.log" "$run/driver.maps" \
            $(sed -nE "s/^export RUSTERON_DPDK_(SENDER|RECEIVER)_PCI='(.+)'$/\2/p" "$env_file") >&2; then
            echo "dpdk: ENA's low-latency queues or their write-combining missing: stopping" >&2
            "$self" stop >/dev/null 2>&1 || true
            exit 1
        fi
    fi
    ;;
client)
    # vm.sh client <label> <rtt|tput|load|floor|purefloor> <args...>: a benchmark client against the running driver (floor, purefloor: the raw or pure floor's)
    label=${2:?label}
    prog=${3:?rtt or tput}
    shift 3
    # SIGTERM from here on (a pkill -f naming the client's words names vm.sh
    # too) is noted, and waited out once the client runs (below); one before
    # it starts stops it starting
    termed=0
    trap 'termed=1' TERM
    # the last run's spikes and histogram: a client that writes none this time
    # must leave none to fetch (lib.sh takes each load point's histogram)
    # (floor xhairpin: both ends here, so both loop-stall files)
    if [[ $prog =~ ^(rtt|floor|purefloor)$ && ${1:-} =~ ^x(ping|hairpin)$ || $prog == load && ${1:-} != xecho ]]; then
        rm -f "$run/load-spikes.csv" "$run/rtt-histogram.csv"
    fi
    [[ $prog =~ ^(rtt|floor|purefloor)$ && ${1:-} =~ ^x(ping|hairpin)$ ]] && rm -f "$run/stalls-ping.csv" "$run/one-way.csv"
    [[ $prog =~ ^(rtt|floor|purefloor)$ && ${1:-} =~ ^x(pong|hairpin)$ ]] && rm -f "$run/stalls-pong.csv"
    # a build under BIN without this client: this checkout's, said in its log
    # (purefloor: vm.sh build's, on this host)
    exe=$bin/$prog
    if [[ $prog == purefloor ]]; then
        exe=$run/purefloor
    elif [[ ! -x $exe ]]; then
        log "client: no $prog in $bin: this checkout's"
        exe=$tools/$prog
    fi
    # started on the housekeeping CPU, so the client's conductor stays there;
    # the measured thread then pins itself to its own CPU (lab.sh passes the app CPU)
    # the mode's channel parameters (RELIABLE, PUB_PARAMS, SUB_PARAMS), and an
    # unreliable channel's lost-round-trip timeout, stretched by a long
    # network round trip (lib.sh measures it: ~/bench/base_rtt_us)
    sub_params=$(cat "$run/sub_channel_params" 2>/dev/null || true) lost_params=
    # INVOKER: the client embeds the driver, configured from driver.env as the
    # media driver would be (AERON_*, RUSTERON_DPDK_*, the transport), with
    # HARNESS_DRIVER=invoker; refused while the last client's embedded driver
    # was never read by `loss` (its counters would go with its CnC file)
    launch=()
    if invoker; then
        if id=$(cnc_now); then invoker_client_ready "$run/cnc.head" "$run/invoker-ledger" || exit 2; fi
        launch=(bash -c '. "$1"; shift; exec "$@"' _ "$run/driver.env" env HARNESS_DRIVER=invoker)
    fi
    # the raw floor: only on the wire `driver` set up (kernel-raw, dpdk-raw),
    # with its environment (the DPDK transport's, the kernel's selector
    # absent); raw UDP repairs nothing, so a lost round trip times out as an
    # unreliable channel's does
    if [[ $prog == floor || $prog == purefloor ]]; then
        if [[ $prog == floor && ( ! -s $run/floor || $(cat "$run/floor") == pure ) ]]; then
            log "client: floor: no raw floor set up here (vm.sh driver kernel-raw or dpdk-raw first)"
            exit 2
        fi
        if [[ $prog == purefloor && $(cat "$run/floor" 2>/dev/null) != pure ]]; then
            log "client: purefloor: no pure floor set up here (vm.sh driver dpdk-pure first)"
            exit 2
        fi
        if [[ $prog == purefloor && ! -x $exe ]]; then
            log "client: purefloor: not built here (vm.sh build): $(cat "$run/purefloor_unavailable" 2>/dev/null)"
            exit 2
        fi
        launch=(bash -c '. "$1"; shift; exec "$@"' _ "$run/driver.env" env)
        lost_params="reliable=false"
    fi
    # The lab's deadline for the client (LAB_CLIENT_DEADLINE, seconds from
    # here; an hour unless lib.sh gives one): GNU timeout, as root beside it,
    # sends it SIGTERM then and SIGKILL LAB_CLIENT_GRACE (10 s) later, or that
    # grace after any SIGTERM timeout gets itself (the lab's, lib.sh
    # end_client: the client's path names it), so a client that ignores
    # SIGTERM (an echo stuck back-pressured) still ends. vm.sh takes SIGTERM
    # (a pkill -f naming the client's words names it too) by waiting for the
    # client, then exits itself: 124 when the deadline ended the client, else the
    # client's status (137: SIGKILL ended it), never killed by a signal, so
    # the SSH session that runs it never ends 255 (lib.sh on()).
    deadline=${LAB_CLIENT_DEADLINE:-3600} grace=${LAB_CLIENT_GRACE:-10}
    if ! [[ $deadline =~ ^[1-9][0-9]*$ && $grace =~ ^[1-9][0-9]*$ ]]; then
        log "client: LAB_CLIENT_DEADLINE=$deadline and LAB_CLIENT_GRACE=$grace: whole seconds, from 1"
        exit 2
    fi
    if ((termed)); then
        log "client: $prog ${1:-}: SIGTERM before it started: not started"
        exit 143
    fi
    # precision-time: rtt's ping splits each spike one way by both hosts' NIC
    # receive timestamps (the PTP hardware clock's), where its mode stamps them
    one_way=
    precision_time && one_way=$run/one-way.csv
    rc=0 started=$SECONDS
    # RTT_GAP_US (lib.sh, a mode's @RTT_GAP_US): the ping's pacing, 0 none
    sudo ${launch[@]+"${launch[@]}"} env AERON_DIR="$AERON_DIR" LABEL="$label" RTT_SECONDS="${RTT_SECONDS:-20}" RTT_GAP_US="${RTT_GAP_US:-0}" KEEPALIVE_US="${KEEPALIVE_US:-0}" FLUSH_KB="${FLUSH_KB:-0}" RTT_ONE_WAY="$one_way" \
        SUB_CHANNEL_PARAMS="$sub_params" PUB_CHANNEL_PARAMS="$(cat "$run/pub_channel_params" 2>/dev/null || true)" \
        RTT_LOST_TIMEOUT_US="$(rtt_lost_timeout_us "${lost_params:-$sub_params}" "$(cat "$run/base_rtt_us" 2>/dev/null || true)")" \
        RTT_HISTOGRAM="$run/rtt-histogram.csv" LOOP_STALLS_DIR="$run" LOAD_SPIKES="$run/load-spikes.csv" LOAD_SPIKE_NS="${LOAD_SPIKE_NS:-100000}" LOOP_STALL_NS="${LOOP_STALL_NS:-20000}" BENCH_CLOCK="$(cat "$run/bench_clock" 2>/dev/null || echo tsc)" AERON_TERM_BUFFER_SPARSE_FILE=false AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY=true \
        taskset -c "$HK_CPU" timeout --kill-after="$grace" "$deadline" "$exe" "$@" || rc=$?
    what="$prog ${1:-}"
    if ((rc == 124 || rc == 137)) && ((SECONDS - started >= deadline)); then
        if ((rc == 124)); then
            log "client: $what overran its deadline of $deadline s: SIGTERM ended it"
        else
            log "client: $what overran its deadline of $deadline s: it ignored SIGTERM, and SIGKILL ended it $grace s later"
        fi
        exit 124
    fi
    if ((termed)); then
        if ((rc == 137)); then
            log "client: $what: SIGTERM: it ignored it, and SIGKILL ended it $grace s later (137)"
        else
            log "client: $what: SIGTERM: it ended ($rc)"
        fi
    elif ((rc == 137)); then
        # the lab's SIGTERM (lib.sh end_client) names the client and its
        # timeout, not vm.sh: one the client ignored, SIGKILL the grace later
        log "client: $what: SIGKILL ended it (137): it ignored a SIGTERM, and timeout(1) sends SIGKILL $grace s after one"
    fi
    exit $rc
    ;;
stop)
    sudo pkill -INT -x media_driver 2>/dev/null || true
    for _ in $(seq 40); do pgrep -x media_driver >/dev/null || break; sleep 0.25; done
    sudo pkill -KILL -x media_driver 2>/dev/null || true
    # INVOKER: a client still running holds the NICs (its embedded driver)
    if invoker; then
        for c in rtt load tput; do sudo pkill -x "$c" 2>/dev/null || true; done
        for _ in $(seq 40); do pgrep -x 'rtt|load|tput' >/dev/null || break; sleep 0.25; done
        rm -rf "$run/harness_driver" "$run/invoker-ledger"
    fi
    # the raw floor: a floor client still running holds the NICs (DPDK's)
    if [[ -s $run/floor ]]; then
        sudo pkill -x floor 2>/dev/null || true
        sudo pkill -x purefloor 2>/dev/null || true
        for _ in $(seq 40); do pgrep -x 'floor|purefloor' >/dev/null || break; sleep 0.25; done
        rm -f "$run/floor"
    fi
    data_return_to_kernel
    restore_host
    data_kernel_up
    sudo sysctl -qw net.core.busy_read=0 net.core.busy_poll=0
    # the baseline again, after an experiment's @THP= (THP_DEFAULT: by hand)
    echo "${THP_DEFAULT:-$BASE_THP}" | sudo tee /sys/kernel/mm/transparent_hugepage/enabled >/dev/null || true
    ;;
loss)
    line= tel=
    # Aeron's own telemetry, never loss (host.sh aeron_telemetry: the agents'
    # longest cycles and cycles over threshold, back-pressure, status
    # messages, flow control), as info- keys after the loss counters: lib.sh
    # loss_row writes them as an aeron row of their own
    telemetry() { { sudo env AERON_DIR="$AERON_DIR" timeout 20 "$tools/counters" '' 2>/dev/null || true; } | aeron_telemetry; }
    if invoker; then
        # the embedded driver in AERON_DIR now (running or its client gone)
        # into the ledger, then every embedded driver's counters summed
        if id=$(cnc_now); then
            cur=$(sudo env AERON_DIR="$AERON_DIR" timeout 20 "$tools/counters" loss 2>/dev/null) || cur=aeron-read-failed=1
            tel=$(telemetry) || tel=
            invoker_ledger_record "$run/invoker-ledger" "$id" "$cur${tel:+;$tel}"
        fi
        line=$(invoker_ledger_total "$run/invoker-ledger")
        tel=
    elif pgrep -x media_driver >/dev/null; then
        line=$(sudo env AERON_DIR="$AERON_DIR" timeout 20 "$tools/counters" loss 2>/dev/null) || line=aeron-read-failed=1
        tel=$(telemetry) || tel=
    fi
    snmp=$(awk '$1 == "Udp:" { if (!seen) { for (i = 2; i <= NF; i++) name[i] = $i; seen = 1; next }
        for (i = 2; i <= NF; i++) if (name[i] ~ /^(InErrors|RcvbufErrors|SndbufErrors)$/) printf "kernel-udp-%s=%s;", name[i], $i }' "$LAB_PROC_ROOT/net/snmp")
    drops=0 nic_ids=()
    while read -r _ iface vf _ _ _ bdf; do
        nic_ids+=("$iface,$vf,$bdf")
        for dev in "$iface" "$vf"; do
            for stat in rx_dropped rx_missed_errors rx_fifo_errors; do
                f=/sys/class/net/$dev/statistics/$stat
                [[ -n $dev && -r $f ]] && drops=$((drops + $(cat "$f")))
            done
        done
    done < <(nics 2>/dev/null)
    # then a check on the run, not loss: the data NICs' interrupts on any CPU
    # but housekeeping's (their IRQs placed anew and not pinned again:
    # irqs.sh). Under DPDK the kernel still holds an IRQ on each (on AWS
    # igb_uio's, the ENA PMD's admin queue's; on Azure MANA's own), found by
    # the PCI addresses the driver was given. Left out, never 0, unless both
    # NICs are found and each matches an IRQ (analyse.py then reports the run
    # unread); a failed read never costs the loss counters their line
    if dpdk_active; then
        mapfile -t nic_ids < <(sed -nE "s/^export RUSTERON_DPDK_(SENDER|RECEIVER)_PCI='(.+)'$/\2/p" "$run/driver.env" 2>/dev/null)
    fi
    nic_irqs= queue_cpu=()
    # a mode's @IRQ=: its queue IRQs counted off the CPU it chose
    irq_cpu=$(cat "$run/irq_cpu" 2>/dev/null || true)
    if [[ -n $irq_cpu && $irq_cpu != "$HK_CPU" ]] && ! dpdk_active; then queue_cpu=(-q "$irq_cpu"); fi
    if ((${#nic_ids[@]} == 2)); then
        nic_irqs=$(irqs_nic_off ${queue_cpu[@]+"${queue_cpu[@]}"} "$HK_CPU" "${nic_ids[@]}") || nic_irqs=
    fi
    [[ $nic_irqs =~ ^[0-9]+$ ]] || { echo "loss: the data NICs' interrupts not read" >&2; nic_irqs=; }
    echo "${line:+$line;}${snmp}nic-kernel-drops=$drops;routes-missing=$(data_routes_missing)${nic_irqs:+;isolated-nic-irqs=$nic_irqs}${tel:+;$tel}"
    ;;
errors)
    # the driver's distinct error log and loss report (from its CnC file, also
    # once it has stopped): what Aeron's errors counter and NAKs were about
    sudo env AERON_DIR="$AERON_DIR" timeout 20 "$tools/counters" errors 2>&1 || true
    ;;
pmd-log)
    # After a DPDK run: the PMD's and EAL's lines on how the NIC was set up
    # (on AWS at info, `driver`'s EAL arguments): ENA's placement policy (its
    # low-latency queues or host memory), LLQ, the queue sizes, and whether
    # its BAR could be mapped write-combining; under @DPDK=patched-refill the
    # patched PMD's line for each RX queue
    grep -E 'Placement policy|LLQ|tx queue size|cannot be mapped|write combin|WC|ergon-lab deferred-refill' "$run/driver.log" 2>/dev/null ||
        echo "pmd-log: no such line in the driver's log"
    ;;
wc-check)
    # AWS, during a DPDK run: ENA's low-latency queues need its memory BAR
    # (BAR 2) mapped write-combining. The kernel offers a prefetchable BAR as
    # resource2_wc, which DPDK maps for ENA when it is there; igb_uio with
    # wc_activate=1 (`driver` loads it so) keeps no uncached mapping of the
    # BAR, to whose type PAT would demote DPDK's. Per data NIC: the file,
    # PAT's memory type for the BAR, and the media driver's mapping of it;
    # then igb_uio's kernel messages.
    if [[ $CLOUD != aws ]]; then
        echo "wc-check: AWS (ENA) only"
        exit 0
    fi
    pid=$(pgrep -x media_driver | head -1 || true)
    mountpoint -q /sys/kernel/debug || sudo mount -t debugfs none /sys/kernel/debug 2>/dev/null || true
    pat=/sys/kernel/debug/x86/pat_memtype_list
    sudo test -r "$pat" || pat=
    echo "wc igb_uio $(lsmod | awk '$1 == "igb_uio" {f = 1} END {print f ? "loaded" : "not loaded"}') wc_activate=$(cat /sys/module/igb_uio/parameters/wc_activate 2>/dev/null || echo n/a)"
    while read -r role bdf; do
        dev=/sys/bus/pci/devices/$bdf
        start= end=
        read -r start end _ < <(sed -n 3p "$dev/resource" 2>/dev/null) || true
        echo "wc $role $bdf driver=$(basename "$(readlink "$dev/driver" 2>/dev/null || echo none)") resource2_wc=$([[ -e $dev/resource2_wc ]] && echo yes || echo no) bar2=${start:-?}-${end:-?}"
        if [[ -n $pat && -n $start ]]; then
            echo "wc $role pat $({ sudo grep -i -- "$start" "$pat" || echo "no entry at $start"; } | head -3 | tr '\n' ' ')"
        elif [[ $ARCH == aarch64 ]]; then
            echo "wc $role pat n/a: x86 only (arm64 maps resource2_wc Normal-NC; its mapping below is the evidence)"
        else
            echo "wc $role pat unavailable (no pat_memtype_list)"
        fi
        maps="none (no media driver running)"
        if [[ -n $pid ]]; then
            maps=$(sudo grep -F "$bdf/resource" "/proc/$pid/maps" | awk '{printf "%s %s %s; ", $1, $2, $6}' || true)
        fi
        echo "wc $role maps ${maps:-none}"
    done < <(sed -nE "s/^export RUSTERON_DPDK_(SENDER|RECEIVER)_PCI='(.*)'$/\1 \2/p" "$run/driver.env" 2>/dev/null | tr 'A-Z' 'a-z')
    sudo dmesg | { grep -iE 'igb_uio|wc_activate' || true; } | tail -20 | sed 's/^/wc dmesg /'
    ;;
counters)
    # Validity. Under DPDK, the driver's mirrored PMD stats: on Azure the VF's
    # packets (vf_*) must equal the totals; on AWS the ENA allowance counters
    # must stay at zero. In a kernel mode, the same from ethtool. On Azure, the
    # kernel's count of VF data-path switches.
    # Frames per message (lib.sh snapshots these around each flat-out run):
    # under DPDK the transport's own frames (pkts) and bytes, its sends a full
    # ring refused back to Aeron (tx-eagain) or queued to go first
    # (tx-queued), sent descriptors reclaimed on idle polls (tx-idle-cleaned,
    # RUSTERON_DPDK_TX_IDLE_CLEAN), and the ENA's per-queue TX statistics (type 92, named by
    # the PMD: packets, doorbells, completion polls, free descriptors); in a
    # kernel mode the data NICs' packet counts. On receive: frames whose UDP
    # checksum the transport verified in software (the NIC did not), the
    # deepest backlog one poll drained, and the polls that nearly emptied a
    # full ring. Under @DPDK=patched-refill, the patched PMD's RX queue
    # counters: bursts that returned frames and left a due refill
    # (rx_q<n>_refill_deferred), and those that refilled at its floor
    # (rx_q<n>_refill_floor).
    pattern=' (vf_)?(rx|tx)_good_packets$|allowance_exceeded$|dir=- clock$| dir=(rx|tx) pkts$| dir=tx (bytes|tx-eagain|tx-queued|tx-idle-cleaned)$| dir=rx (sw-checksum-verified|rx-backlog-max|rx-ring-full-suspected)$| tx_q[0-9]+_(cnt|doorbells|tx_poll|available_desc)$| rx_q[0-9]+_refill_(deferred|floor)$'
    if driver_cnc && dpdk_active; then
        # under INVOKER, whose driver's CnC this is (analyse.py counts a run's
        # frames from zero over another embedded driver's snapshot)
        if invoker; then echo "cnc $(cnc_now || echo none)"; fi
        # Every counter (the PMD's are labelled with their bare names, so
        # each takes the role of the port listed before it: a port registers
        # its own counters, then its PMD's), filtered to the pattern
        { sudo env AERON_DIR="$AERON_DIR" "$tools/counters" '' | awk -v pat="$pattern" '
            / rusteron-dpdk role=/ {match($0, /role=[a-z]+/); role = substr($0, RSTART, RLENGTH)}
            $2 == 92 && role != "" {$3 = role " " $3}
            $0 ~ pat' | tr -s ' ' | tr '\n' ';'; } || true
        echo
    elif invoker && dpdk_active; then
        # INVOKER under DPDK before the mode's first client: no embedded
        # driver has run, so there is no CnC to read, and the NICs are DPDK's
        # (ethtool has nothing): no driver named, which analyse.py counts as
        # zero, the next client's driver starting from zero too
        echo "cnc none"
        echo
    elif [[ -s $run/floor ]] && dpdk_active; then
        # the raw floor under DPDK: its clients register the transport's
        # counters privately (no CnC), and the NICs are DPDK's (ethtool has
        # nothing)
        echo "floor: no CnC"
        echo
    elif [[ $CLOUD == azure ]]; then
        # netvsc's own stats split the VF path (vf_*) from the synthetic one (per queue)
        nics | while read -r role iface _; do
            echo "$role $(ethtool -S "$iface" 2>/dev/null | awk '
                $1 == "vf_rx_packets:" || $1 == "vf_tx_packets:" {printf "%s %s ", $1, $2}
                $1 ~ /^(rx|tx)_queue_[0-9]+_packets:$/ {synthetic += $2}
                END {printf "synthetic_packets: %d", synthetic}') $(nic_packets "$iface")"
        done
    else
        nics | while read -r role iface _; do
            echo "$role $(ethtool -S "$iface" 2>/dev/null | awk '$1 ~ /allowance_exceeded:$/ {printf "%s %s ", $1, $2}')$(nic_packets "$iface")"
        done
    fi
    [[ $CLOUD == azure ]] && echo "path-switches $(sudo dmesg | grep -ci 'data path switched' || true)"
    true
    ;;
jitter)
    # Where a spike can come from: each media-driver thread's allowed CPUs and
    # context switches (nvcsw: preempted), and on each measured CPU the
    # interrupts taken and the hypervisor's steal (ticks). Before/after a run.
    # (none under INVOKER: no media driver process)
    pid=$(pgrep -x media_driver | head -1 || true)
    if [[ -n $pid ]]; then
        # RUSTERON_MEDIA_DRIVER_MLOCKALL=1 shows here as locked memory
        echo "driver $(sudo grep -hE '^(VmLck|VmRSS)' /proc/"$pid"/status 2>/dev/null | tr -s ' \t\n' ' ')"
        for t in /proc/"$pid"/task/*; do
            sudo awk -v comm="$(sudo cat "$t/comm" 2>/dev/null)" -v tid="${t##*/}" '
                /^Cpus_allowed_list:/ {c = $2} /^voluntary_ctxt_switches:/ {v = $2} /^nonvoluntary_ctxt_switches:/ {n = $2}
                END {printf "thread %s/%s cpus=%s vcsw=%s nvcsw=%s\n", comm, tid, c, v, n}' "$t/status" 2>/dev/null
        done
    fi
    # the agents' own stalls, per port (DPDK only): gaps between polls that
    # found frames (rx-poll-gap*) and between any polls (poll-cycle-gap*);
    # not under INVOKER, whose CnC between clients is the last one's
    if [[ -n $pid ]] && dpdk_active && ! invoker; then
        sudo env AERON_DIR="$AERON_DIR" timeout 20 "$tools/counters" -gap 2>/dev/null | sed 's/^/counter /'
        # PMD burst timing (RUSTERON_DPDK_GAP_ATTRIB=1 or burst; zero otherwise)
        sudo env AERON_DIR="$AERON_DIR" timeout 20 "$tools/counters" -burst 2>/dev/null | sed 's/^/counter /'
        # receive checksums the transport verified in software (the NIC did
        # not), and sent descriptors reclaimed on idle polls
        # (RUSTERON_DPDK_TX_IDLE_CLEAN; zero otherwise)
        sudo env AERON_DIR="$AERON_DIR" timeout 20 "$tools/counters" sw-checksum-verified 2>/dev/null | sed 's/^/counter /'
        sudo env AERON_DIR="$AERON_DIR" timeout 20 "$tools/counters" tx-idle-cleaned 2>/dev/null | sed 's/^/counter /'
    fi
    for cpu in $HK_CPU $APP_CPU $SENDER_CPU $RECEIVER_CPU $APP2_CPU; do
        # its interrupts, local timer and IPIs from its own column (irqs.sh
        # irqs_on_cpu: by name, so SMT=off's offline siblings shift nothing)
        steal=$(awk -v c="cpu$cpu" '$1 == c {print $9}' "$LAB_PROC_ROOT/stat")
        echo "cpu $cpu $(irqs_on_cpu "$cpu") steal=$steal"
    done
    ;;
clockcheck)
    taskset -c "$APP_CPU" "$tools/clockcheck" "${2:-60}"
    ;;
preflight)
    # vm.sh preflight [seconds]: before any benchmark, spin on every CPU but
    # housekeeping's and report its stalls of 50 us or more (count, how many
    # of 100 us or more, longest), then how many stalls of 100 us or more hit
    # two or more CPUs within a millisecond (the whole VM paused: the host).
    # A host that stalls is worth redeploying before hours are spent on it.
    # (`nproc --all`: once isolated, a login's own CPU mask is housekeeping's.)
    gcc -O2 -o "$run/gapspin" "$(dirname "$self")/gapspin.c" || exit 1
    secs=${2:-120}
    pids=()
    for cpu in $(seq 0 $(($(nproc --all) - 1))); do
        [[ $cpu == "$HK_CPU" ]] && continue
        "$run/gapspin" "$cpu" 50 "$secs" >"$run/preflight-$cpu.log" & pids+=($!)
    done
    wait "${pids[@]}"
    python3 - "$run" <<'PY'
import glob, os, sys
gaps = []
for path in sorted(glob.glob(os.path.join(sys.argv[1], "preflight-*.log")), key=lambda p: int(p.rsplit("-", 1)[1][:-4])):
    cpu = int(path.rsplit("-", 1)[1][:-4])
    rows = [l.split() for l in open(path) if not l.startswith("#")]
    us = [float(r[1]) for r in rows]
    gaps += [(float(r[0]), float(r[1]), cpu) for r in rows if float(r[1]) >= 100]
    print(f"preflight cpu {cpu} gaps50={len(us)} gaps100={sum(u >= 100 for u in us)} max_us={max(us, default=0):.0f}")
gaps.sort()
# CPUs that stalled 300 us or more: the layout keeps the run's threads off
# their cores (vm.sh layout)
noisy = sorted({c for _, us, c in gaps if us >= 300})
open(os.path.join(sys.argv[1], "noisy_cpus"), "w").write(" ".join(map(str, noisy)) + "\n")
print(f"preflight noisy_cpus {' '.join(map(str, noisy)) or 'none'} (stalls of 300 us or more; seconds of the minute: "
      + ", ".join(sorted({f':{int(t % 60):02d}' for t, us, _ in gaps if us >= 300})) + ")")
shared = 0
i = 0
while i < len(gaps):
    j = i
    while j + 1 < len(gaps) and gaps[j + 1][0] - gaps[i][0] < 0.001:
        j += 1
    if len({g[2] for g in gaps[i:j + 1]}) > 1:
        shared += 1
    i = j + 1
print(f"preflight stalls100={len(gaps)} on_several_cpus={shared}")
PY
    ;;
diag-start)
    # Only before reps: a trace, a tracer or a kprobe set up flips static keys,
    # an IPI to every CPU, agents included; refused while a rep runs (a media
    # driver, or a client: INVOKER's embed the driver)
    if running=$(pgrep -x 'media_driver|rtt|load|tput|floor|purefloor'); then
        log "diag-start: a rep is running (pid $(echo $running)): diagnostics are set up only before reps (enabling them flips static keys, an IPI to every CPU)"
        exit 2
    fi
    # For the whole state: the gap spinner on a spare CPU (the first core
    # outside the layout, else housekeeping's SMT twin, else none: host.sh
    # diag_spare_cpu; a four-core Graviton has neither, and a spinner on a
    # core of the layout would take it from the run) and a trace of the app
    # and agent CPUs (context switches, work items, IRQs and IPIs with their
    # exits, so each is timed; on CLOCK_MONOTONIC, anchored to the wall clock
    # by trace_anchor at the start and at diag-fetch), plus who calls
    # lru_add_drain_all (every CPU), so each spike can be told apart:
    # a guest preemption (traced), a host stall (the spinner too), or neither.
    gcc -O2 -o "$run/gapspin" "$(dirname "$self")/gapspin.c" || exit 1
    # shellcheck disable=SC2086
    spare=$(diag_spare_cpu $HK_CPU $APP_CPU $SENDER_CPU $RECEIVER_CPU ${APP2_CPU:-}) || spare=
    pkill -x gapspin 2>/dev/null || true
    rm -f "$run/diag-paused"
    printf '%s\n' "$spare" >"$run/diag-spare"
    if [[ -n $spare ]]; then
        nohup "$run/gapspin" "$spare" "$(gapspin_us)" >"$run/diag-gapspin.log" 2>&1 </dev/null &
    else
        echo "# no spinner: no CPU outside the layout (no fifth core, no SMT twin)" >"$run/diag-gapspin.log"
    fi
    # every change to addresses, routes, rules and links, timestamped: who
    # took a data NIC's route away, if anything does
    if [[ -s $run/diag-ipmon.pid ]]; then kill "$(cat "$run/diag-ipmon.pid")" 2>/dev/null || true; fi
    nohup ip -ts monitor address route rule link >"$run/diag-ipmon.log" 2>&1 </dev/null &
    echo $! >"$run/diag-ipmon.pid"
    mask=0
    for cpu in $APP_CPU $SENDER_CPU $RECEIVER_CPU $APP2_CPU $spare; do mask=$((mask | (1 << cpu))); done
    # arm64's IPIs (x86's are the irq_vectors events below)
    ipi=
    if [[ $ARCH == aarch64 ]]; then ipi='[ -d events/ipi/ipi_entry ] && echo 1 > events/ipi/ipi_entry/enable && echo 1 > events/ipi/ipi_exit/enable;'; fi
    sudo bash -c "cd /sys/kernel/tracing && echo 0 > tracing_on && echo > trace && printf '%x' $mask > tracing_cpumask &&
        echo 8192 > buffer_size_kb && echo 1 > events/sched/sched_switch/enable &&
        echo 1 > events/workqueue/workqueue_execute_start/enable && echo 1 > events/irq/irq_handler_entry/enable && echo 1 > events/irq/irq_handler_exit/enable &&
        for e in call_function_entry call_function_single_entry reschedule_entry; do
            [ -d events/irq_vectors/\$e ] && echo 1 > events/irq_vectors/\$e/enable; done; $ipi
        { grep -q drainall kprobe_events || echo 'p:drainall lru_add_drain_all' >> kprobe_events; } &&
        for k in text_poke_bp_batch synchronize_rcu_expedited; do
            grep -qw \$k available_filter_functions && { grep -q \"p:\$k \" kprobe_events || echo \"p:\$k \$k\" >> kprobe_events; }; done;
        mkdir -p instances/drain && echo 1 > instances/drain/events/kprobes/enable && echo 1 > instances/drain/options/stacktrace &&
        { ! grep -qw mono trace_clock || echo mono > trace_clock; } &&
        echo 1 > tracing_on"
    trace_anchor
    echo "diag spinner cpu ${spare:-none (no CPU outside the layout: host stalls go unattributed)}, traced cpus $(printf '%x' $mask)"
    # KNOISE on the agent and app CPUs (the spare keeps the spinner)
    # shellcheck disable=SC2086
    cpus=$(echo $APP_CPU $SENDER_CPU $RECEIVER_CPU ${APP2_CPU:-} | tr ' ' ',')
    echo "$cpus" >"$run/knoise-cpus"
    sudo env LAB_SYS_ROOT="$LAB_SYS_ROOT" bash -c "$(declare -f knoise_start); knoise_start $cpus"
    ;;
diag-fetch)
    # the spinner's gaps, then the trace with the uptime and wall clock it
    # counts against (to put its events on the wall clock), then the drains
    echo "== gapspin"
    cat "$run/diag-gapspin.log" 2>/dev/null
    echo "== trace"
    trace_anchor
    sudo grep -E 'sched_switch|workqueue_execute_start|irq_handler_(entry|exit)|call_function|reschedule_entry|ipi_(entry|exit)|ergon-anchor' /sys/kernel/tracing/trace
    echo "UPTIME $(cut -d' ' -f1 /proc/uptime) REALTIME $(date +%s.%N)"
    echo "== drains"
    # who drains LRU batches, patches kernel text (a static key flipped on
    # every CPU) or waits for an expedited RCU grace period, with their stacks
    sudo grep -v '^#' /sys/kernel/tracing/instances/drain/trace 2>/dev/null
    echo "== ipmon"
    cat "$run/diag-ipmon.log" 2>/dev/null
    # KNOISE's events (knoise_start), on the trace's clock: the agent and app
    # CPUs it timed, and how its cpumasks print
    echo "== knoise"
    echo "# cpus $(cat "$run/knoise-cpus" 2>/dev/null)"
    echo "# cpumask $([[ $(sudo cat "$LAB_SYS_ROOT/kernel/tracing/instances/knoise/options/bitmask-list" 2>/dev/null) == 1 ]] && echo list || echo hex)"
    sudo grep -v '^#' "$LAB_SYS_ROOT/kernel/tracing/instances/knoise/trace" 2>/dev/null
    ;;
diag-stop)
    pkill -x gapspin 2>/dev/null || true
    rm -f "$run/diag-paused"
    if [[ -s $run/diag-ipmon.pid ]]; then kill "$(cat "$run/diag-ipmon.pid")" 2>/dev/null || true; rm -f "$run/diag-ipmon.pid"; fi
    echo 0 | sudo tee /sys/kernel/tracing/tracing_on >/dev/null
    sudo env LAB_SYS_ROOT="$LAB_SYS_ROOT" bash -c "$(declare -f knoise_stop); knoise_stop"
    ;;
hostchecks)
    # vm.sh hostchecks: the media driver's start-up host checks
    # (ConfiguredDriver::from_env, md-dpdk/src/checks.rs: one line each that
    # is not fine, `ergon-md-dpdk warning: ...` or `ergon-md-dpdk info: ...`)
    # as this run's driver printed them in driver.log, nothing else of it;
    # nothing where it said none, or under INVOKER (no driver process: each
    # client's embedded driver prints its own in the client's output, which
    # lib.sh reads there)
    grep -E '^ergon-md-dpdk (warning|info): ' "$run/driver.log" 2>/dev/null || true
    ;;
criterion)
    # the same features as `build`: cargo also builds the package's binaries
    # for a bench, and a different feature set would relink media_driver
    if [[ -s $run/dpdk_unavailable ]]; then
        echo "criterion: the clock bench builds with the dpdk feature, which this host lacks: $(cat "$run/dpdk_unavailable")" >&2
        exit 1
    fi
    export PATH=$HOME/.cargo/bin:$PATH
    cargo bench -q -p ergon-md-dpdk --features static,dpdk --bench clock --no-run 2>/dev/null
    taskset -c "$APP_CPU" cargo bench -q -p ergon-md-dpdk --features static,dpdk --bench clock 2>&1 |
        grep -E '^(clock_read|send_32b|poll_32b)/|time:|change:|no invariant'
    ;;
pmtu)
    # jumbo frames' path check (lib.sh, before a JUMBO=1 mode's drivers start,
    # on both hosts at once): the data NICs at the jumbo MTU for the placement
    # (kept for `stop` to put back; the driver's start sets it again), then
    # each peer address at it from each data address (host.sh path_mtu_check)
    shift
    (($#)) || { echo "pmtu: the peer's data addresses" >&2; exit 2; }
    read -r nic_mtu _ <<<"$(jumbo_mtus "$PLACEMENT")"
    data_mtu "$nic_mtu"
    # shellcheck disable=SC2046
    path_mtu_check "$nic_mtu" $(nics | awk '{print $5}') -- "$@"
    ;;
mtu)
    # the MTUs the running mode uses: the link's (DPDK: its port's L3 MTU) and
    # Aeron's (its publications': a PUB_PARAMS mtu, else the driver's)
    dpdk=0
    if dpdk_active; then dpdk=1; fi
    # shellcheck disable=SC2046
    effective_mtu "$run/driver.env" "$(cat "$run/pub_channel_params" 2>/dev/null || true)" "$dpdk" \
        $(nics | awk '{print $2}' | while read -r dev; do cat "/sys/class/net/$dev/mtu" 2>/dev/null; done)
    ;;
base-rtt)
    # the network's own round trip, this host's sender NIC to <peer>: 200
    # pings 10 ms apart (root: under 200 ms), as host.sh ping_stats prints it
    peer=${2:?the peer address}
    src=$(nics | awk '$1 == "sender" {print $5}')
    sudo ping -q -c 200 -i 0.01 -I "$src" "$peer" | ping_stats
    ;;
rx-hw-delay)
    # the receiver port's NIC-arrival-to-dispatch counters (type 81), as
    # "<mean ns>,<max ns>,<frames>,<min ns>,<negative>": the mean over the
    # frames in the sum; apart, the frames the NIC stamped after the host
    # clock read at their dispatch (PHC and host clock skew). The endpoint is
    # each frame's own dispatch. A driver built before the labels said so (an
    # @BIN= base build) has no min or negative: empty. Only skewed frames (a
    # NIC clock chrony does not keep the host on) give ",,0,,<negative>"; no
    # stamped frame gives nothing. Older drivers labelled these
    # nic-arrival-to-rx-burst-* (one read a burst, a lower bound for every
    # frame after the first) and rx-hw-delay-*: both still parse.
    driver_cnc || exit 0
    sudo env AERON_DIR="$AERON_DIR" "$tools/counters" 'role=receiver' | awk '
        / (nic-arrival-to-rx-burst|nic-arrival-to-dispatch|rx-hw-delay)-max-ns$/ {max = $1}
        / (nic-arrival-to-rx-burst|nic-arrival-to-dispatch|rx-hw-delay)-sum-ns$/ {sum = $1}
        / (nic-arrival-to-rx-burst-count|nic-arrival-to-dispatch-count|rx-hw-timestamped)$/ {n = $1}
        / nic-arrival-to-rx-burst-min-ns$/ {min = $1}
        / nic-arrival-to-rx-burst-negative$/ {neg = $1}
        / nic-arrival-to-dispatch-min-ns$/ {min = $1}
        / nic-arrival-to-dispatch-negative$/ {neg = $1}
        END {
            if (n > 0) printf "%.0f,%d,%d,%s,%s\n", sum / n, max, n, min, neg
            else if (neg > 0) printf ",,0,,%d\n", neg
        }'
    ;;
transport-isolate)
    # the transport suite's CPU map (host.sh transport_cpu_map), kept for
    # every later command, its layout for the media driver, and its boot
    # line (replacing `isolate`'s) from the next boot
    map=$(transport_cpu_map) || exit 2
    echo "$map" >"$run/transport-map"
    transport_layout "$map" >"$run/layout-transport"
    args=$(transport_boot_args "$map")
    sudo mkdir -p /etc/default/grub.d
    echo "GRUB_CMDLINE_LINUX_DEFAULT=\"\$GRUB_CMDLINE_LINUX_DEFAULT $args\"" |
        sudo tee /etc/default/grub.d/99-ergon-isolation.cfg >/dev/null
    sudo update-grub >/dev/null 2>&1
    echo "transport-isolate: map $map (housekeeping, IRQ, isolated); layout $(cat "$run/layout-transport"); $args from the next boot"
    ;;
transport-tune)
    # after the boot transport-isolate set up: SMT off, the boot line
    # checked, the quiet settings, irqbalance off, the data NICs (rings,
    # coalescing, quickack routes, every interrupt on the IRQ CPU) and the
    # sysctls, each read back; its record bench/transport-tune.txt; 1 at the
    # first setting that does not hold
    [[ -s $run/transport-map ]] || { echo "transport-tune: no transport map (vm.sh transport-isolate first)" >&2; exit 2; }
    map=$(cat "$run/transport-map")
    read -r hk irq iso <<<"$map"
    tune_steps() {
        echo "map $map"
        transport_smt_off || return 1
        local now
        now=$(transport_cpu_map) && [[ $now == "$map" ]] ||
            { echo "transport-tune: the CPU map with SMT off is [$now], not [$map]" >&2; return 1; }
        transport_boot_check "$map" || return 1
        transport_quiet_apply "$hk,$irq" "$iso" || return 1
        sudo systemctl disable --now irqbalance >/dev/null 2>&1 || true
        if systemctl is-active --quiet irqbalance 2>/dev/null; then echo "transport-tune: irqbalance still running" >&2; return 1; fi
        echo "irqbalance off"
        data_kernel_up || return 1
        transport_sysctls_apply || return 1
        transport_ports_check "$TR_PORT_BASE" "$TR_PORT_SPAN" || return 1
        echo "kernel $(uname -r) cmdline $(cat "${LAB_PROC_ROOT}/cmdline" 2>/dev/null)"
    }
    tune_steps >"$run/transport-tune.txt" 2>&1 && rc=0 || rc=$?
    cat "$run/transport-tune.txt"
    exit $rc
    ;;
transport-net)
    # an arm on the kernel's data path, no media driver (the socket arms):
    # the data NICs as transport-tune left them (data_kernel_up: rings,
    # coalescing, quickack, IRQs), NAPI as kernel-busy has it, every sysctl;
    # `check`: only read back (after a driver started on the kernel's path)
    [[ -s $run/transport-map ]] || { echo "transport-net: no transport map" >&2; exit 2; }
    if [[ ${2:-} == check ]]; then
        transport_sysctls_apply check >/dev/null || exit 1
        transport_nics >/dev/null || exit 1
        echo "transport-net: settings hold"
        exit 0
    fi
    data_kernel_up
    nics | while read -r _ _ vf _; do
        echo "$TR_NAPI_DEFER" | sudo tee "$LAB_SYS_ROOT/class/net/$vf/napi_defer_hard_irqs" >/dev/null
        echo "$TR_GRO_FLUSH_NS" | sudo tee "$LAB_SYS_ROOT/class/net/$vf/gro_flush_timeout" >/dev/null
    done
    transport_cc_load
    transport_sysctls_apply
    ;;
transport-map)
    # "<housekeeping> <irq> <isolated,...>" as transport-isolate kept it
    cat "$run/transport-map"
    ;;
cc-available)
    # the congestion controls this kernel offers, bbr loaded first where it
    # is a module (the tuning session sweeps only what both hosts have)
    sudo modprobe tcp_bbr 2>/dev/null || true
    cat "$LAB_PROC_ROOT/sys/net/ipv4/tcp_available_congestion_control"
    ;;
transport-build)
    # the transport suite's bench binaries (transport-cli.md), then
    # LAB_EXTRA_SRC's tree (lib.sh copies it to ~/extra) by its own
    # lab-build.sh into ~/extra/bin
    # (transport-build.sh: the local smoke builds them the same way)
    export PATH=$HOME/.cargo/bin:$PATH
    bash "$(dirname "$self")/transport-build.sh" "$HOME/ergon" "$HOME/ergon/target" "$(client_features "$run/dpdk_unavailable")"
    if [[ -f $HOME/extra/lab-build.sh ]]; then
        mkdir -p "$HOME/extra/bin"
        (cd "$HOME/extra" && bash lab-build.sh "$HOME/extra/bin")
    fi
    ;;
netem)
    # netem on every data NIC's egress (the interfaces the kernel sends on;
    # under accelerated networking the VF below forwards what they send):
    # netem set <loss %> | clear | check <loss %|none> | stats (host.sh)
    mapfile -t ifaces < <(nics | awk '{print $2}')
    ((${#ifaces[@]})) || { echo "netem: no data NIC in the kernel's hands" >&2; exit 1; }
    case ${2:-} in
    set) netem_set "${3:?loss %}" "${ifaces[@]}" && netem_check "$3" "${ifaces[@]}" ;;
    clear) netem_clear "${ifaces[@]}" && netem_check none "${ifaces[@]}" ;;
    check) netem_check "${3:?loss % or none}" "${ifaces[@]}" ;;
    stats) netem_stats "${ifaces[@]}" ;;
    show) netem_show "${ifaces[@]}" ;;
    *) echo "netem set <loss %> | clear | check <loss %|none> | stats | show" >&2; exit 2 ;;
    esac
    ;;
sample)
    # where every thread ran: the verdict on the map's isolated CPUs (host.sh
    # transport_placement) on the first line, then ps's own lines
    read -r _ _ iso <"$run/transport-map"
    transport_threads 1 >"$run/sample.ps"
    transport_placement "$iso" <"$run/sample.ps" || true
    cat "$run/sample.ps"
    ;;
irqsnap) cat "$LAB_PROC_ROOT/interrupts" ;;
nstat) transport_nstat ;;
settings) transport_settings_read ;;
events-start)
    # events-start <point> <traced cpus> <spinner cpus|-> [aeron]: before the
    # point's binaries start (transport.sh), so before their warm-up: host.sh
    # events_start on the hot CPUs given (trace, osnoise where the kernel has
    # it), gap spinners on the spinner CPUs, the sampler on the housekeeping
    # CPU, the media driver's counters with aeron
    [[ -s $run/transport-map ]] || { echo "events-start: no transport map" >&2; exit 2; }
    read -r hk _ _ <"$run/transport-map"
    [[ -x $run/gapspin ]] || gcc -O2 -o "$run/gapspin" "$(dirname "$self")/gapspin.c" || exit 1
    adir=-
    [[ ${5:-} == aeron ]] && adir=$TR_AERON_DIR
    counters=-
    [[ -x $tools/counters ]] && counters=$tools/counters
    mapfile -t ifaces < <(nics | awk '{print $2}')
    events_start "$run/events/${2:?point}" ergon-events "$hk" "${3:?traced cpus}" "${4:--}" "$run/gapspin" "$TR_EVENTS_GAPSPIN_US" \
        "$adir" "$counters" "$TR_EVENTS_BUFFER_KB" "${ifaces[@]}"
    ;;
events-stop-all)
    # whatever events a runner that died left running (its EXIT trap)
    [[ -s $run/transport-map ]] || exit 0
    read -r _ _ iso <"$run/transport-map"
    for d in "$run"/events/*/; do [[ -d $d ]] && events_stop "${d%/}" ergon-events "$iso" >/dev/null 2>&1; done
    true
    ;;
events-stop)
    # events-stop <point>: after the point's window, its events as
    # transport_events.py reads them (host.sh events_stop), all of it stopped
    [[ -s $run/transport-map ]] || { echo "events-stop: no transport map" >&2; exit 2; }
    read -r _ _ iso <"$run/transport-map"
    events_stop "$run/events/${2:?point}" ergon-events "$iso"
    ;;
hostcheck)
    # the ergon-tune checks on the hot CPUs and data NICs (transport-cli.md),
    # its report in bench/hostcheck.txt; 1 on any warning or error
    read -r _ _ iso <"$run/transport-map"
    rm -f "$run/hostcheck.txt"
    # (exit 2 not clean, 1 not run: either stops the session; a warn line too)
    PATH=$tools:$PATH hostcheck --hot-cpus "$iso" --nic "$(data_ifaces | paste -sd, -)" --aeron-dir "$TR_AERON_DIR" \
        --out "$run/hostcheck.txt" && rc=0 || rc=$?
    cat "$run/hostcheck.txt" 2>/dev/null
    if ((rc)) || grep -qE '^[^ ]+ warn: ' "$run/hostcheck.txt" 2>/dev/null; then
        echo "hostcheck: not clean (exit $rc)" >&2
        exit 1
    fi
    ;;
*)
    awk 'NR > 1 && !/^#/ {exit} NR > 1 {print}' "$self" >&2
    exit 2
    ;;
esac
