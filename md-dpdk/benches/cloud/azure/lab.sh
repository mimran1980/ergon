#!/usr/bin/env bash
# Kernel UDP against DPDK (Azure MANA) for the Aeron media driver, on two Azure
# VMs that exist only while this script runs. Runs on macOS or Linux with the az
# CLI logged in and an SSH public key (~/.ssh/id_ed25519.pub or id_rsa.pub).
#
#   md-dpdk/benches/cloud/azure/lab.sh
#
# Creates resource group $LAB_GROUP (refusing one that exists) in $LAB_REGION:
# every resource tagged owner=ergon-lab run=<stamp> (the results directory's
# name), a proximity placement group, a VNet with a management subnet (10.20.0.0/24,
# SSH from this machine's address only) and a data subnet (10.20.1.0/24), and
# two $LAB_SIZE VMs (Debian 13: amd64, or arm64 on Cobalt 100's Arm sizes,
# Standard_D8ps_v6 and kin, a p after the vCPUs), each with an accelerated management NIC and two
# accelerated data NICs: the media driver's sender port (A .4, B .6) and
# receiver port (A .5, B .7). Then, on both VMs at once (../vm.sh, ../lib.sh): bootstrap, a
# reboot into the backports kernel, vm.sh build. Then $LAB_REPS reps of every
# mode in $LAB_MODES (a fresh random order each rep): both drivers started in
# the mode, a round-trip run (A pings, B pongs) and a throughput run (A
# publishes, B subscribes). Results go to target/md-dpdk-bench/<stamp>/ and the
# tables to its summary.md (../analyse.py). The group is deleted on any exit,
# and every 15 minutes the log lists what is in it. Before any az call the plan
# is estimated (../estimate.py); one over 2 h detaches (a nohup subshell, its
# log $out/lab.log), so it outlives the shell or session that started it.
#
#   LAB_REGION (northcentralus) LAB_SIZE (Standard_D8s_v6; LAB_SIZE_A, LAB_SIZE_B per VM) LAB_REPS (5)
#   LAB_MODES ("kernel kernel-busy dpdk"; also dpdk-spin, and a +idle suffix
#   holding an idle localhost subscription on each driver) LAB_LOCAL_MODES (none;
#   e.g. "kernel dpdk dpdk-spin": localhost runs on VM a, needs 5 cores) LAB_STATES ("pinned isolated": the stock
#   kernel, then CPUs 1-7 isolated on the kernel command line) LAB_GROUP (ergon-lab-md-dpdk)
#   LAB_IMAGE (Debian:debian-13:13-gen2:latest) LAB_KEEP=1 keeps the group;
#   LAB_REPS=0 with LAB_KEEP=1 stops after the build, for work on the VMs by hand
#   LAB_MIX (none; e.g. "trading=orders:3001:10000:32 md:3002:200000:288:50 bulk:3003:20000:1344:10")
#   LAB_MIX_SECONDS (10); round trips of 100 µs or more are logged with their times (spikes-*.csv)
#   LAB_TPUT_FLOWS (none; unset, 0 and 1 run nothing of it; 2 or more, at most one flow per
#   app CPU on each VM, so 2 where both have a fifth core): in each cross-host mode but
#   +idle ones, after everything else, a sustained maximum over that many flows at once, at
#   32 B and each LAB_LOAD_SIZES size. Each flow is a publication on its own channel, so its
#   own UDP port and 5-tuple (a channel is one flow whatever its streams), each end on its
#   own app CPU. Against one flow's maximum it shows whether a ceiling is per flow, as AWS's
#   8 Gbit/s appears to be, or per NIC or instance; a run whose flows' measured windows
#   overlapped less than 0.9 of the shortest is left out of it (analyse.py)
#   LAB_NO_FLATOUT=1: no phase that sends as fast as it can (no 32 B throughput run,
#   no sustained maximum, no several flows): each LAB_LOAD_SIZES size needs its
#   LAB_LOAD_LADDER rates. Modes take the settings of ../modes.sh (threading modes,
#   INVOKER cross-host only, PUB_PARAMS and SUB_PARAMS, CONDUCTOR=own and NAPI_BUSY=spare
#   on sizes with a fifth core, Standard_D16s_v6 and up, the DPDK transport's values
#   within MANA's 256-descriptor rings); JUMBO and IRQ are AWS's only
#   LAB_MAX_HOURS (8): a backstop process of its own (nohup) deletes the whole
#   group that long after creation, should this script's teardown never run
#   (killed), LAB_KEEP or not; only while the group carries this run's tag, so
#   never a later run's of the same name. It runs on this machine: asleep or
#   offline, nothing deletes the group, so check after a run. Under a day, each
#   VM's Azure auto-shutdown at the same deadline deallocates it, which stops
#   its compute billing even then (its disks and addresses bill until deleted).
#   A plan estimated to take longer is refused before any az call, with the
#   hours to give it
#   LAB_ESTIMATE=1: the plan's hours and dollars (../estimate.py: pay-as-you-go
#   Linux prices in northcentralus), then exit, with no az call
#   LAB_DETACH (unset: detach a plan estimated over 2 h; 1 always; 0 never)
#   LAB_WARMUP_SECONDS (0: none) and LAB_WARMUP_RATE: a warm-up load of that many
#   seconds at that rate before each mode's first load ladder, in each rep, at its
#   first size, recorded nowhere (Aeron's own harness warms 30 s at 25,000 msgs/s);
#   refused without LAB_LOAD_SIZES, where it would never run
#   LAB_RTT_ALIGN (none; e.g. 58): start each round-trip run at that second of the
#   minute, so every mode misses (or meets) a host event recurring once a minute alike
#   LAB_RTT_SPIKE_NS (30000) and LAB_RTT_STALL_NS (10000): the cross-host round trip's
#   clients stamp every round trip, and every stall of their own loop, of at least that
#   many ns (spikes.py gives each rep's 10 largest every event overlapping them; on
#   Kubernetes the client pods' LOAD_SPIKE_NS and LOOP_STALL_NS, pod environment)
#   LAB_GAPSPIN_US (50): the diagnostics' gap spinner, on a spare CPU, records every gap
#   of its loop of at least that many us (vm.sh diag-start): 10 sees the 20-50 us agent
#   loop gaps' size, whether the whole VM paused (spikes.py's agent gap table)
#   LAB_BASE (none; a git revision of this checkout): also built on both VMs, into
#   ~/bin-base, for modes with @BIN=bin-base: LAB_MODES="dpdk dpdk@BIN=bin-base" A/Bs
#   that build against this one (analyse.py --ab dpdk dpdk@BIN=bin-base <results.csv>).
#   Without it, @BIN=<dir> runs any build placed in <dir> on both VMs by hand
#   (its media_driver, and rtt, tput and load where it has them)
#   Every mode runs on the recommended configuration (vm.sh baseline: transparent
#   hugepages off, and in DPDK modes the receiver's IO vectors at 16); a mode's
#   @THP=always or @AERON_RECEIVER_IO_VECTOR_CAPACITY=4 A/Bs the earlier state
#   RTT_MESSAGES (300000) RTT_WARMUP (50000) TPUT_SECONDS (5)
#
#   The transport latency suite, LAB_SUITE=transport (latency: the above, by
#   default): one pair session of ../transport-matrix.tsv (../transport.sh;
#   the bench binaries' CLI ../transport-cli.md) in place of the modes and
#   states. LAB_PAIR (a or b, required): the session, its own resource group
#   (LAB_GROUP, ergon-lab-transport-<pair>-<stamp>) and run directory
#   (target/md-dpdk-bench/<stamp>-transport-<pair>), every resource tagged
#   owner=ergon-lab run=<stamp> pair=<pair>, so two pairs started together
#   (LAB_RUN_STAMP: one stamp for both) never touch each other's resources
#   or count them as their own leftovers. LAB_SIZE defaults to
#   Standard_D16s_v6 (SMT off: 8 cores, the CPU map of vm.sh
#   transport-isolate), LAB_MAX_HOURS to 10. LAB_SEED (random): the session's
#   seed for every round's arm order, recorded. LAB_ARMS_FILE: more arm and
#   set rows (transport-matrix.tsv's format), checked with the matrix before
#   any az call; LAB_EXTRA_SRC: a source tree copied to ~/extra on both VMs
#   and built there by its own lab-build.sh, whose binaries the arms file's
#   arms may name. TRANSPORT_WARMUP_S (30), TRANSPORT_POINT_S (30) and
#   TRANSPORT_SWITCH_S (5): the plan's durations. LAB_SETS_FILE: set rows
#   in place of the matrix's (its arms kept): the cloud smoke's
#   ../transport-cloud-smoke.tsv, every arm at one load.
#   LAB_TUNE=1: the pair's tuning session in place of its sets (session
#   tune-<pair>, ../transport_tune.py): each arm's knob sweeps of
#   ../transport-settings.env (TR_SWEEP_*; pair a the arms of no loss set,
#   pair b the TCP arms and aeron-rel), the better values confirmed, the
#   frozen choice written into this checkout's transport-settings.env (its
#   tuned section) as the session ends, for the comparison sessions;
#   LAB_TUNE_ARMS and LAB_TUNE_KNOBS (comma lists) cut it down. Its
#   estimate gives the confirmation at most and likely
#   LAB_PRIORITY (Regular; Spot for smoke and tuning runs only): Spot VMs
#   with eviction policy Delete; each minute (LAB_EVICTION_POLL) both are
#   looked for, and one Azure evicted ends the run through its teardown at
#   once, exit 75
#   Both suites check that each size takes the three NICs (az vm list-skus'
#   MaxNetworkInterfaces) before creating anything, and refuse one unread
#   To stop a run early: kill -TERM <its pid> (the one whose parent is 1). Bash
#   ignores SIGINT while an ssh step runs that does not die of it, so Ctrl-C can
#   take until that step ends; TERM runs the teardown as soon as the step returns.
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
repo=${LAB_REPO:-$(cd "$here/../../../.." && pwd)}
# Run from a snapshot of these scripts: bash reads a script as it runs, so an
# edit to the checkout mid-run would otherwise break this run (and its teardown).
# In $TMPDIR, by template: macOS's mktemp -d without one ignores TMPDIR.
if [[ -z ${LAB_SNAPSHOT:-} ]]; then
    snapshot=$(mktemp -d "${TMPDIR:-/tmp}/ergon-lab.XXXXXX") || { echo "lab.sh: no snapshot directory in ${TMPDIR:-/tmp}" >&2; exit 1; }
    if ! cp -R "$here/.." "$snapshot/cloud"; then
        [[ -n $snapshot && -d $snapshot ]] && rm -rf -- "$snapshot"
        echo "lab.sh: cannot snapshot $here/.. into $snapshot" >&2
        exit 1
    fi
    LAB_SNAPSHOT=$snapshot LAB_REPO=$repo exec bash "$snapshot/cloud/${here##*/}/${0##*/}" "$@"
fi
# remove_snapshot: the snapshot this run executes from, removed as it exits:
# the EXIT trap's last step (the teardown, which runs from it, first), so
# runs leave no copy behind in $TMPDIR. Only by its own path, checked: a
# directory under $TMPDIR named as mktemp made it, holding this very script
remove_snapshot() {
    local s=${LAB_SNAPSHOT:-}
    [[ -n $s && $s == "${TMPDIR:-/tmp}"/ergon-lab.* && ${s##*/} =~ ^ergon-lab\.[A-Za-z0-9]{6}$ && -d $s &&
        -f $s/cloud/${here##*/}/${0##*/} ]] || return 0
    rm -rf -- "$s"
}
# until the teardown's own trap (below) takes over, which calls it last
trap remove_snapshot EXIT
# the suite: the Aeron transports' latency lab (the default), or the
# transport latency suite (transport.sh), one pair session of it
suite=${LAB_SUITE:-latency}
pair=${LAB_PAIR:-}
# LAB_TUNE=1: the pair's tuning session (tune-a, tune-b) instead of its sets
tune=${LAB_TUNE:-}
[[ $tune == 1 && ($pair == a || $pair == b) ]] && pair=tune-$pair
priority=${LAB_PRIORITY:-Regular}
arms_file=${LAB_ARMS_FILE:-}
sets_file=${LAB_SETS_FILE:-}
extra_src=${LAB_EXTRA_SRC:-}
case $suite in
latency) default_size=Standard_D8s_v6 default_hours=8 ;;
transport) default_size=Standard_D16s_v6 default_hours=10 ;;
*) echo "LAB_SUITE $suite: latency (the default) or transport" >&2; exit 2 ;;
esac
region=${LAB_REGION:-northcentralus}
size=${LAB_SIZE:-$default_size}
# per-VM overrides: VM a hosts the localhost runs (LAB_LOCAL_MODES), which need five cores
size_a=${LAB_SIZE_A:-$size}
size_b=${LAB_SIZE_B:-$size}
# what the lab asks of Azure (az.sh)
. "$here/az.sh"
# x86_64, or arm64 on Cobalt 100's Arm sizes (a `p` after the vCPUs:
# Standard_D8ps_v6, Standard_D8pls_v6, Standard_E8ps_v6): both VMs one, for
# one image and one build. DPDK 24.11 and 25.11 build the MANA and netvsc
# PMDs on arm64 too (drivers/net/mana/meson.build: x86 or arm64 Linux), and
# Debian ships librte_net_mana for arm64, so the DPDK modes run on both.
arch=$(azure_arch "$size_a")
if [[ $(azure_arch "$size_b") != "$arch" ]]; then
    echo "LAB_SIZE_A $size_a ($arch) and LAB_SIZE_B $size_b ($(azure_arch "$size_b")): one architecture for both VMs (one image, one build)" >&2
    exit 2
fi
image=${LAB_IMAGE:-$(azure_image "$arch")}
# vm.sh bootstraps an aarch64 host only with it (lib.sh: bench/allow_arm)
allow_arm=
[[ $arch == arm64 ]] && allow_arm=1
reps=${LAB_REPS:-5}
read -ra modes <<<"${LAB_MODES:-kernel kernel-busy dpdk}"
local_modes=${LAB_LOCAL_MODES:-}
# latency under load: message sizes (none: skipped), fractions of each size's
# sustained maximum, seconds per point
load_sizes=${LAB_LOAD_SIZES:-}
load_fractions=${LAB_LOAD_FRACTIONS:-10 50 80 95 100}
load_seconds=${LAB_LOAD_SECONDS:-5}
# absolute rates per size instead of fractions: "1344:200000,400000 288:..."
load_ladder=${LAB_LOAD_LADDER:-}
# several streams at once, each with its own rate, size and bursts (lib.sh
# mix_run): profiles "<profile>=<name:stream:msgs/s:size[:burst]> ...;..."
mix=${LAB_MIX:-}
mix_seconds=${LAB_MIX_SECONDS:-10}
# a sustained maximum over several flows at once (lib.sh multi_flow_max): how
# many flows; unset, 0 or 1, none
tput_flows=${LAB_TPUT_FLOWS:-}
# the cost guard: no phase that sends as fast as it can (lib.sh)
no_flatout=${LAB_NO_FLATOUT:-}
read -ra states <<<"${LAB_STATES:-pinned isolated}"
rtt_messages=${RTT_MESSAGES:-300000}
rtt_warmup=${RTT_WARMUP:-50000}
rtt_align=${LAB_RTT_ALIGN:-}
# the cross-host round trip's stamps: every round trip and every loop stall of
# at least that many ns (lib.sh)
rtt_spike_ns=${LAB_RTT_SPIKE_NS:-30000}
rtt_stall_ns=${LAB_RTT_STALL_NS:-10000}
# the gap spinner's threshold, us (lib.sh, vm.sh diag-start)
gapspin_us=${LAB_GAPSPIN_US:-50}
base=${LAB_BASE:-}
# the backstop deletes the group this long after creation (hours)
max_hours=${LAB_MAX_HOURS:-$default_hours}
detach=${LAB_DETACH:-}
# a warm-up load before each mode's first ladder, unrecorded (lib.sh
# load_sweep): its seconds and messages a second; none by default
warmup_seconds=${LAB_WARMUP_SECONDS:-0}
warmup_rate=${LAB_WARMUP_RATE:-0}
tput_seconds=${TPUT_SECONDS:-5}
user=azureuser
# a detached run's directory is its parent's (LAB_RUN_OUT, set only there)
if [[ $suite == transport ]]; then
    # one stamp for both pairs of a run (LAB_RUN_STAMP), each pair its own
    # directory and group; the run tag the stamp, the pair tag its pair
    run_stamp=${LAB_RUN_STAMP:-$(date +%Y%m%d-%H%M%S)}
    out=${LAB_RUN_OUT:-$repo/target/md-dpdk-bench/$run_stamp-transport-$pair}
    stamp=$run_stamp
    group=${LAB_GROUP:-ergon-lab-transport-$pair-$run_stamp}
    tags=(owner=ergon-lab "run=$stamp" "pair=$pair")
else
    out=${LAB_RUN_OUT:-$repo/target/md-dpdk-bench/$(date +%Y%m%d-%H%M%S)}
    stamp=${out##*/}
    group=${LAB_GROUP:-ergon-lab-md-dpdk}
    # on every resource, so one listing finds whatever a run left
    tags=(owner=ergon-lab "run=$stamp")
fi
known=$out/known_hosts
created=0

# the run's own stderr, kept on fd 8: the teardown logs there whatever
# redirection the step it interrupted had (a TERM can land inside one)
exec 8>&2
log() { echo "[$(date +%T)] $*" >&2; }
die() { log "ERROR: $*"; exit 1; }

key=
for k in ~/.ssh/id_ed25519.pub ~/.ssh/id_rsa.pub; do [[ -f $k ]] && { key=$k; break; }; done
[[ -n $key ]] || die "no ~/.ssh/id_ed25519.pub or ~/.ssh/id_rsa.pub"
# a base revision that cannot be built is found before anything is created
if [[ -n $base ]] && ! git -C "$repo" rev-parse --verify -q "$base^{commit}" >/dev/null; then
    die "LAB_BASE $base: not a commit in $repo"
fi
[[ -z $tput_flows || $tput_flows =~ ^[0-9]+$ ]] || die "LAB_TPUT_FLOWS $tput_flows: not a number of flows"
# every mode's settings, as aws/lab.sh checks them (modes.sh mode_check: a
# comma would split every results.csv row of it, an Aeron threading mode or
# URI parameter Aeron does not know runs as if it were not there, INVOKER
# cannot run on one host, jumbo frames are AWS's), before any az call
. "$here/../modes.sh"
[[ $priority == Regular || $priority == Spot ]] || die "LAB_PRIORITY $priority: Regular (the default) or Spot (smoke and tuning runs: Azure may evict it)"
if [[ $suite == transport ]]; then
    # the pair, the arms file and the extra tree, and every point's mode
    # (transport.sh check: modes.sh's arm settings, NETEM refused on DPDK
    # arms), the Aeron arms' driver modes and their conductor's core on both
    # sizes, before any az call
    # LAB_ESTIMATE=1 without a pair: both pairs' estimates
    [[ -z $tune || $tune == 1 ]] || die "LAB_TUNE is 1 or unset, not $tune"
    [[ $pair == a || $pair == b || $pair == tune-a || $pair == tune-b || ( -z $pair && ${LAB_ESTIMATE:-} == 1 ) ]] ||
        die "LAB_PAIR is a or b with LAB_SUITE=transport (the session of transport-matrix.tsv to run), not [${pair}]"
    [[ -z $tune || -z $sets_file ]] || die "LAB_TUNE=1 sweeps the matrix's arms in their own sets' context: no LAB_SETS_FILE"
    if [[ -n $sets_file ]]; then
        [[ -r $sets_file ]] || die "LAB_SETS_FILE $sets_file: not a readable file"
        sets_file=$(cd "$(dirname "$sets_file")" && pwd)/${sets_file##*/}
    fi
    # a copy taken now, beside this run's script snapshot: an edit to the
    # checkout while the run goes on (a merge) never changes what it runs
    if [[ -n $sets_file ]]; then
        cp "$sets_file" "$here/../.lab-sets-$$.tsv" || die "LAB_SETS_FILE $sets_file: cannot copy"
        sets_file=$here/../.lab-sets-$$.tsv
    fi
    tr_sets_file=$sets_file
    [[ $run_stamp =~ ^[0-9]{8}-[0-9]{6}$ ]] || die "LAB_RUN_STAMP is <yyyymmdd>-<hhmmss>, not $run_stamp"
    [[ ${LAB_MODES:-}${LAB_LOCAL_MODES:-} == "" ]] || die "LAB_MODES and LAB_LOCAL_MODES are the latency suite's: LAB_SUITE=transport runs transport-matrix.tsv's arms"
    if [[ -n $extra_src ]]; then
        [[ -d $extra_src && -f $extra_src/lab-build.sh ]] || die "LAB_EXTRA_SRC $extra_src: a directory with a lab-build.sh (the VMs build it with: bash lab-build.sh <bin dir>)"
        extra_src=$(cd "$extra_src" && pwd)
    fi
    [[ -z $arms_file ]] || arms_file=$(cd "$(dirname "$arms_file")" && pwd)/${arms_file##*/}
    # (an unreadable one is left for transport.sh's own refusal)
    if [[ -n $arms_file && -r $arms_file ]]; then
        cp "$arms_file" "$here/../.lab-arms-$$.tsv" || die "LAB_ARMS_FILE $arms_file: cannot copy"
        arms_file=$here/../.lab-arms-$$.tsv
    fi
    # shellcheck source=../transport.sh
    . "$here/../transport.sh"
    all_pairs="a b"
    [[ $tune == 1 ]] && all_pairs="tune-a tune-b"
    for p in ${pair:-$all_pairs}; do
        points=$(tr_check "$p" "$arms_file" 2>&1) || die "$points"
    done
    for m in "kernel-busy$TR_AERON_MODE_SUFFIX" "dpdk$TR_AERON_MODE_SUFFIX"; do
        why=$(spare_types_check "$m" azure "$size_a" "$size_b" 2>&1) || die "$why"
    done
    for v in TRANSPORT_WARMUP_S TRANSPORT_POINT_S TRANSPORT_SWITCH_S; do
        [[ -z ${!v:-} || ${!v} =~ ^[0-9]{1,5}$ ]] || die "$v is a whole number of seconds, not ${!v}"
    done
    seed=${LAB_SEED:-$(od -An -N4 -tu4 /dev/urandom | tr -d ' ')}
    [[ $seed =~ ^[0-9A-Za-z_-]{1,40}$ ]] || die "LAB_SEED is letters, digits, - and _ (the session's seed), not $seed"
else
    [[ -z $pair && -z $arms_file && -z $extra_src && -z $sets_file && -z $tune ]] ||
        die "LAB_PAIR, LAB_ARMS_FILE, LAB_SETS_FILE, LAB_EXTRA_SRC and LAB_TUNE are the transport suite's (LAB_SUITE=transport)"
    for m in "${modes[@]}"; do
        why=$(mode_check "$m" remote "$arch" azure "" 2>&1) || die "$why"
        # the layout's fifth core on both sizes (vm.sh ready would find it missing
        # only after the build)
        why=$(spare_types_check "$m" azure "$size_a" "$size_b" 2>&1) || die "$why"
    done
    for m in ${local_modes:-}; do
        why=$(mode_check "$m" local "$arch" azure "" 2>&1) || die "$why"
    done
fi
[[ -z $no_flatout || $no_flatout == 1 ]] || die "LAB_NO_FLATOUT is 1 or unset"
[[ $rtt_spike_ns =~ ^[1-9][0-9]{0,9}$ ]] || die "LAB_RTT_SPIKE_NS is a whole number of nanoseconds, from 1 (the round trips stamped), not $rtt_spike_ns"
[[ $rtt_stall_ns =~ ^[1-9][0-9]{0,9}$ ]] || die "LAB_RTT_STALL_NS is a whole number of nanoseconds, from 1 (the client loop stalls stamped), not $rtt_stall_ns"
[[ $gapspin_us =~ ^[1-9][0-9]{0,5}$ ]] || die "LAB_GAPSPIN_US is a whole number of microseconds, 1 to 999999 (the gap spinner's threshold), not $gapspin_us"
[[ $warmup_seconds =~ ^[0-9]{1,6}$ ]] || die "LAB_WARMUP_SECONDS is a whole number of seconds (0: no warm-up), not $warmup_seconds"
[[ $warmup_rate =~ ^[0-9]{1,9}$ ]] || die "LAB_WARMUP_RATE is a whole number of messages a second, not $warmup_rate"
((10#$warmup_seconds == 0 || 10#$warmup_rate > 0)) ||
    die "LAB_WARMUP_SECONDS=$warmup_seconds needs LAB_WARMUP_RATE, its messages a second (Aeron's own harness warms 30 s at 25,000)"
# the warm-up runs before each mode's first load ladder (lib.sh load_sweep):
# with no load sizes there is none, and it would never run
((10#$warmup_seconds == 0)) || [[ -n ${load_sizes// /} ]] ||
    die "LAB_WARMUP_SECONDS=$warmup_seconds without LAB_LOAD_SIZES: the warm-up runs before each mode's first load ladder, so it would never run (give LAB_LOAD_SIZES, or no warm-up)"
if [[ $no_flatout == 1 ]]; then
    [[ ${tput_flows:-0} -lt 2 ]] || die "LAB_TPUT_FLOWS runs several flows flat out: not with LAB_NO_FLATOUT=1"
    for size in $load_sizes; do
        [[ " $load_ladder " == *" $size:"* ]] ||
            die "LAB_NO_FLATOUT=1: size $size has no LAB_LOAD_LADDER rates (LAB_LOAD_FRACTIONS are fractions of the flat-out maximum, which does not run)"
    done
fi
[[ $max_hours =~ ^[1-9][0-9]{0,2}$ ]] || die "LAB_MAX_HOURS is a whole number of hours, 1 to 999, not $max_hours"
[[ -z $detach || $detach == 0 || $detach == 1 ]] || die "LAB_DETACH is 0, 1 or unset, not $detach"
# the plan's hours and dollars (estimate.py, the timings aws/lab.sh's runs
# measured: the same steps), before any az call
plan=(--cloud azure --type-a "$size_a" --type-b "$size_b" --placement ppg --reps "$reps"
    --modes "${modes[*]}" --local-modes "$local_modes" --states "${states[*]}" --load-sizes "$load_sizes"
    --load-fractions "$load_fractions" --load-ladder "$load_ladder" --load-seconds "$load_seconds" --mix "$mix"
    --mix-seconds "$mix_seconds" --tput-seconds "$tput_seconds" --tput-flows "${tput_flows:-0}"
    --rtt-messages "$rtt_messages" --rtt-warmup "$rtt_warmup" --no-flatout "$no_flatout" --max-hours "$max_hours"
    --warmup-seconds "$warmup_seconds" --warmup-rate "$warmup_rate")
if [[ $suite == transport ]]; then
    # the pair's sets (both pairs' without one, LAB_ESTIMATE=1), timed as
    # transport.sh runs them (estimate.py transport_estimate)
    plan=(--cloud azure --suite transport --pair "$pair" --type-a "$size_a" --type-b "$size_b" --reps "$reps"
        --max-hours "$max_hours" --priority "$priority" --transport-warmup-s "${TRANSPORT_WARMUP_S:-30}"
        --transport-point-s "${TRANSPORT_POINT_S:-30}" --transport-switch-s "${TRANSPORT_SWITCH_S:-5}"
        --matrix "$here/../transport-matrix.tsv${sets_file:+:arms}" ${sets_file:+--matrix "$sets_file"}
        ${arms_file:+--matrix "$arms_file"} ${tune:+--tune --settings "$here/../transport-settings.env"}
        --tune-arms "${LAB_TUNE_ARMS:-}" --tune-knobs "${LAB_TUNE_KNOBS:-}")
fi
if [[ ${LAB_ESTIMATE:-} == 1 ]]; then
    python3 "$here/../estimate.py" "${plan[@]}"
    exit $?
fi
# a plan longer than the backstop's deadline would lose its group hours in
why=$(python3 "$here/../estimate.py" "${plan[@]}" --refuse-over-deadline 2>&1) || die "$why"
hours=$(python3 "$here/../estimate.py" "${plan[@]}" --print-hours) || die "estimate.py: $hours"
# before detaching, so a run that cannot start says so here, not in its log
command -v az >/dev/null || die "az is not on PATH"
az account show -o none 2>/dev/null || die "az is not logged in (az login)"
# Detach a plan over 2 h: a nohup subshell, parented to init, running this
# snapshot again with this run's directory, so it outlives the shell or
# session that started it (a harness's background task dies at 2 h, mid-run
# or mid-teardown). The snapshot becomes the detached run's: it removes it.
if [[ -z ${LAB_DETACHED:-} ]] && { [[ $detach == 1 ]] || { [[ -z $detach ]] && awk -v h="$hours" 'BEGIN {exit !(h > 2)}'; }; }; then
    mkdir -p "$out" || die "cannot make $out"
    (LAB_DETACHED=1 LAB_RUN_OUT=$out LAB_RUN_STAMP=${run_stamp:-} LAB_SEED=${seed:-} nohup bash "$0" "$@" >"$out/lab.log" 2>&1 </dev/null & echo $! >"$out/lab.pid")
    log "detached: about $hours h: pid $(cat "$out/lab.pid"), log $out/lab.log (stop it with kill -TERM $(cat "$out/lab.pid"); LAB_DETACH=0 runs in the foreground)"
    trap - EXIT
    exit 0
fi
mkdir -p "$out"

teardown() {
    local rc=$?
    exec 2>&8
    # an interrupt must not abort the delete, nor a create still running race it
    trap '' INT TERM
    [[ -n ${follow:-} ]] && kill "$follow" 2>/dev/null
    [[ -n ${watch:-} ]] && kill "$watch" 2>/dev/null
    [[ -n ${pa:-} ]] && kill "$pa" "${pb:-}" 2>/dev/null
    [[ -n ${evict:-} ]] && kill "$evict" 2>/dev/null
    # a Spot VM Azure evicted: the run ends here, whatever it was doing
    # (exit 75), and nothing of the broken pair is kept, LAB_KEEP or not
    local evicted
    evicted=$(cd "$out" 2>/dev/null && ls evicted-* 2>/dev/null | sed 's/^evicted-//' | tr '\n' ' ')
    if [[ -n $evicted ]]; then
        log "spot eviction: VM(s) $evicted evicted by Azure: the run ends (exit 75), its group deleted"
        rc=75
        unset LAB_KEEP
    fi
    if ((created)) && [[ -z ${LAB_KEEP:-} ]]; then
        log "deleting resource group $group"
        { az group delete -n "$group" --yes --no-wait -o none &&
            az group wait -n "$group" --deleted --timeout 3600 -o none; } ||
            log "WARNING: delete $group by hand: az group delete -n $group --yes"
        if [[ $(az group exists -n "$group") == false ]]; then
            log "verified: resource group $group no longer exists"
            [[ -n ${reaper:-} ]] && kill "$reaper" 2>/dev/null && log "backstop (pid $reaper) stopped: nothing left for it"
        else
            log "WARNING: resource group $group still exists: az group delete -n $group --yes${reaper:+ (the backstop, pid $reaper, deletes it $max_hours h after creation)}"
        fi
        # this run's own: the transport suite's other pair shares its run
        # tag, so only this pair's are counted
        left=$(azure_left "$stamp" "$pair")
        if [[ $left == 0 ]]; then
            log "left tagged run=$stamp${pair:+ pair=$pair}: 0"
        else
            log "WARNING: $left resource(s) still tagged run=$stamp${pair:+ pair=$pair} (or the listing failed): az resource list --tag run=$stamp -o table"
        fi
    elif ((created)); then
        log "LAB_KEEP: resource group $group kept; delete it with: az group delete -n $group --yes${reaper:+ (the backstop, pid $reaper, deletes it $max_hours h after creation)}"
    elif [[ -n ${reaper:-} ]]; then
        # no group was created: nothing for it to delete
        kill "$reaper" 2>/dev/null
    fi
    log "results: $out"
    # last: the snapshot this ran from
    remove_snapshot
    exit "$rc"
}
trap teardown EXIT
trap 'exit 130' INT TERM

if [[ $(az group exists -n "$group") == true ]]; then
    die "resource group $group exists: delete it or set LAB_GROUP"
fi

# LAB_SSH_FROM: this machine's public addresses (space-separated), for a
# connection that fails over between links; default the current one
me=${LAB_SSH_FROM:-$(curl -fsS https://api.ipify.org)} || die "cannot find this machine's public address"
me=$(printf '%s\n' $me | sort -u | tr '\n' ' ') # a repeated address is an error to Azure
read -ra me_cidrs <<<"$(printf '%s/32 ' $me)"
# both VMs' vCPUs within their quota families' and the region's (az.sh)
quota=$(azure_quota_check "$region" "$size_a" "$size_b" 2>&1) || die "$quota"
log "$quota"
# each size takes the management NIC and both data NICs (az.sh), or
# nothing is created
nic_cap=$(azure_nic_check "$region" 3 "$size_a" "$size_b" 2>&1) || die "$nic_cap"
log "$nic_cap"
# The backstop (az.sh): the whole group deleted LAB_MAX_HOURS from now,
# should the teardown never run. Started, and seen running, before the group
# exists, so no moment has a group and no backstop. Deleting, never
# deallocating: a deallocated VM still bills for its disks and public addresses.
reaper=$(azure_reaper "$group" "$stamp" $((max_hours * 3600)) "$out/reaper.log")
sleep 0.5
[[ $reaper =~ ^[0-9]+$ ]] && kill -0 "$reaper" 2>/dev/null ||
    die "the backstop did not start (pid [$reaper]): $(cat "$out/reaper.log" 2>/dev/null)"
log "backstop: resource group $group deleted $max_hours h from now at the latest (pid $reaper); its log $out/reaper.log"
# Each VM's auto-shutdown at the same deadline: Azure's own, so it runs with
# this machine asleep or offline, which the backstop does not. It only
# deallocates (compute stops billing; disks and addresses do not), so it
# caps the cost and the backstop or teardown still deletes. A daily schedule:
# at a deadline a day or more away it would fire first, so none then.
off=
if ((max_hours < 24)); then
    off=$(date -u -v+"${max_hours}"H +%H%M 2>/dev/null || date -u -d "+${max_hours} hours" +%H%M)
else
    log "no auto-shutdown: LAB_MAX_HOURS=$max_hours is a day or more (a daily schedule would fire first): only the backstop and teardown end this run"
fi
read -ra priority_args <<<"$(azure_priority_args "$priority")"
log "creating $group in $region: a $size_a, b $size_b ($arch, $image, $priority), SSH from $me, tagged ${tags[*]}"
# before the call: a create that fails having made the group is deleted too
created=1
az group create -n "$group" -l "$region" --tags "${tags[@]}" -o none || die "group create failed"
# every 15 minutes, what is in the group
azure_watch "$group" 900 & watch=$!
az ppg create -g "$group" -n ppg -l "$region" --intent-vm-sizes $(printf "%s\n" "$size_a" "$size_b" | sort -u) --tags "${tags[@]}" -o none || die "ppg create failed"
az network vnet create -g "$group" -n vnet --address-prefixes 10.20.0.0/16 \
    --subnet-name mgmt --subnet-prefixes 10.20.0.0/24 --tags "${tags[@]}" -o none || die "vnet create failed"
az network vnet subnet create -g "$group" --vnet-name vnet -n data --address-prefixes 10.20.1.0/24 -o none ||
    die "data subnet create failed"
az network nsg create -g "$group" -n nsg --tags "${tags[@]}" -o none || die "nsg create failed"
az network nsg rule create -g "$group" --nsg-name nsg -n ssh --priority 100 --source-address-prefixes "${me_cidrs[@]}" \
    --destination-port-ranges 22 --protocol Tcp --access Allow -o none || die "nsg rule create failed"

# Follow this machine's public address: a connection that fails over between
# links gets a new one every few minutes, and SSH must keep working
follow_address() {
    local now known_ips=" $me "
    while sleep 20; do
        now=$(curl -fsS -m 5 https://api.ipify.org 2>/dev/null) || continue
        [[ $known_ips == *" $now "* ]] && continue
        known_ips="$known_ips$now "
        # shellcheck disable=SC2086
        az network nsg rule update -g "$group" --nsg-name nsg -n ssh \
            --source-address-prefixes $(printf '%s/32 ' $known_ips) -o none &&
            log "SSH now also allowed from $now"
    done
}
follow_address & follow=$!

declare_node() { # name sender-ip receiver-ip size
    local n=$1
    az network public-ip create -g "$group" -n "$n-ip" --sku Standard --tags "${tags[@]}" -o none &&
        az network nic create -g "$group" -n "$n-mgmt" --vnet-name vnet --subnet mgmt --network-security-group nsg \
            --public-ip-address "$n-ip" --accelerated-networking true --tags "${tags[@]}" -o none &&
        az network nic create -g "$group" -n "$n-snd" --vnet-name vnet --subnet data --private-ip-address "$2" \
            --accelerated-networking true --tags "${tags[@]}" -o none &&
        az network nic create -g "$group" -n "$n-rcv" --vnet-name vnet --subnet data --private-ip-address "$3" \
            --accelerated-networking true --tags "${tags[@]}" -o none &&
        az vm create -g "$group" -n "$n" --size "$4" --image "$image" --ppg ppg \
            --nics "$n-mgmt" "$n-snd" "$n-rcv" --admin-username "$user" --ssh-key-values "$key" \
            --os-disk-name "$n-os" --os-disk-size-gb 64 ${priority_args[@]+"${priority_args[@]}"} --tags "${tags[@]}" -o none &&
        { [[ -z $off ]] || az vm auto-shutdown -g "$group" -n "$n" --time "$off" -o none ||
            log "WARNING: no auto-shutdown on $n: only the backstop and teardown end it"; } &&
        # a VM's tags do not reach its OS disk; the Tags API, not `az
        # resource tag`, which asks for the provider's newest API version and
        # a region may not serve it yet
        disk_id=$(az disk show -g "$group" -n "$n-os" --query id -o tsv) && [[ -n $disk_id ]] &&
        az tag update --resource-id "$disk_id" --operation merge --tags "${tags[@]}" -o none
}
declare_node a 10.20.1.4 10.20.1.5 "$size_a" & pa=$!
declare_node b 10.20.1.6 10.20.1.7 "$size_b" & pb=$!
wait $pa || die "node a create failed"
wait $pb || die "node b create failed"
A=$(az network public-ip show -g "$group" -n a-ip --query ipAddress -o tsv) && [[ -n $A ]] || die "no public address for a"
B=$(az network public-ip show -g "$group" -n b-ip --query ipAddress -o tsv) && [[ -n $B ]] || die "no public address for b"
# Spot: each minute both VMs looked for; one Azure evicted (and so deleted)
# TERMs this run (az.sh), and every SSH session to the pair is ended and no
# new one tried (lib.sh on(): lab_abandon), so the teardown, which deletes
# the group, runs at once, not after an SSH session to a VM no longer there
# times out
lab_abandon=$out/abandon
if [[ $priority == Spot ]]; then
    {
        azure_eviction_watch "$group" "${LAB_EVICTION_POLL:-60}" $$ "$out" a b &&
            [[ -n $(cd "$out" && ls evicted-* 2>/dev/null) ]] && : >"$lab_abandon" &&
            { pkill -f -- "$user@$A" 2>/dev/null; pkill -f -- "$user@$B" 2>/dev/null; true; }
    } & evict=$!
    log "spot: both VMs looked for every ${LAB_EVICTION_POLL:-60} s; an eviction ends the run (pid $evict)"
fi
a_private=$(az network nic show -g "$group" -n a-mgmt --query 'ipConfigurations[0].privateIPAddress' -o tsv)
b_private=$(az network nic show -g "$group" -n b-mgmt --query 'ipConfigurations[0].privateIPAddress' -o tsv)
log "a=$A b=$B (private $a_private $b_private)"

data_subnet=10.20.1.0/24
. "$here/../lib.sh"
prepare_vms
if [[ $suite == transport ]]; then
    vm_a=$group/a vm_b=$group/b
    echo "$seed" >"$out/seed"
    transport_prepare
    session_ok=1
    transport_remote "$pair" || session_ok=
    if [[ $tune == 1 && -s $out/transport/tuned.env ]]; then
        # the frozen choice into this checkout's transport-settings.env (the
        # comparison sessions run it; commit it with the run's tune-report.md).
        # A session with failed variants still froze: a knob whose rounds did
        # not complete keeps its current value (tune-report.md says which)
        python3 "$here/../transport_tune.py" apply "$repo/md-dpdk/benches/cloud/transport-settings.env" "$out/transport/tuned.env" ||
            die "the tuned section was not written ($out/transport/tuned.env)"
        log "tune: $(grep -c '^# changed' "$out/transport/tuned.env") knobs changed: $repo/md-dpdk/benches/cloud/transport-settings.env (evidence $out/transport/tune-report.md)"
    fi
    [[ -n $session_ok ]] || die "transport pair $pair failed ($out/transport)"
    log "transport pair $pair: $(grep -c . "$out/transport/results.csv" 2>/dev/null || echo 0) rows in $out/transport/results.csv"
else
    run_bench
fi
