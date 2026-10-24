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
group=${LAB_GROUP:-ergon-lab-md-dpdk}
region=${LAB_REGION:-northcentralus}
size=${LAB_SIZE:-Standard_D8s_v6}
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
max_hours=${LAB_MAX_HOURS:-8}
detach=${LAB_DETACH:-}
# a warm-up load before each mode's first ladder, unrecorded (lib.sh
# load_sweep): its seconds and messages a second; none by default
warmup_seconds=${LAB_WARMUP_SECONDS:-0}
warmup_rate=${LAB_WARMUP_RATE:-0}
tput_seconds=${TPUT_SECONDS:-5}
user=azureuser
# a detached run's directory is its parent's (LAB_RUN_OUT, set only there)
out=${LAB_RUN_OUT:-$repo/target/md-dpdk-bench/$(date +%Y%m%d-%H%M%S)}
stamp=${out##*/}
# on every resource, so one listing finds whatever a run left
tags=(owner=ergon-lab "run=$stamp")
known=$out/known_hosts
created=0

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
for m in "${modes[@]}"; do
    why=$(mode_check "$m" remote "$arch" azure "" 2>&1) || die "$why"
    # the layout's fifth core on both sizes (vm.sh ready would find it missing
    # only after the build)
    why=$(spare_types_check "$m" azure "$size_a" "$size_b" 2>&1) || die "$why"
done
for m in ${local_modes:-}; do
    why=$(mode_check "$m" local "$arch" azure "" 2>&1) || die "$why"
done
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
    (LAB_DETACHED=1 LAB_RUN_OUT=$out nohup bash "$0" "$@" >"$out/lab.log" 2>&1 </dev/null & echo $! >"$out/lab.pid")
    log "detached: about $hours h: pid $(cat "$out/lab.pid"), log $out/lab.log (stop it with kill -TERM $(cat "$out/lab.pid"); LAB_DETACH=0 runs in the foreground)"
    trap - EXIT
    exit 0
fi
mkdir -p "$out"

teardown() {
    local rc=$?
    # an interrupt must not abort the delete, nor a create still running race it
    trap '' INT TERM
    [[ -n ${follow:-} ]] && kill "$follow" 2>/dev/null
    [[ -n ${watch:-} ]] && kill "$watch" 2>/dev/null
    [[ -n ${pa:-} ]] && kill "$pa" "${pb:-}" 2>/dev/null
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
        left=$(az resource list --tag run="$stamp" --query 'length(@)' -o tsv 2>&1 | az_words)
        if [[ $left == 0 ]]; then
            log "left tagged run=$stamp: 0"
        else
            log "WARNING: $left resource(s) still tagged run=$stamp (or the listing failed): az resource list --tag run=$stamp -o table"
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
log "creating $group in $region: a $size_a, b $size_b ($arch, $image), SSH from $me, tagged ${tags[*]}"
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
            --os-disk-name "$n-os" --os-disk-size-gb 64 --tags "${tags[@]}" -o none &&
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
a_private=$(az network nic show -g "$group" -n a-mgmt --query 'ipConfigurations[0].privateIPAddress' -o tsv)
b_private=$(az network nic show -g "$group" -n b-mgmt --query 'ipConfigurations[0].privateIPAddress' -o tsv)
log "a=$A b=$B (private $a_private $b_private)"

data_subnet=10.20.1.0/24
. "$here/../lib.sh"
prepare_vms
run_bench
