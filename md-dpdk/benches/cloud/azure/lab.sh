#!/usr/bin/env bash
# Kernel UDP against DPDK (Azure MANA) for the Aeron media driver, on two Azure
# VMs that exist only while this script runs. Runs on macOS or Linux with the az
# CLI logged in and an SSH public key (~/.ssh/id_ed25519.pub or id_rsa.pub).
#
#   md-dpdk/benches/cloud/azure/lab.sh
#
# Creates resource group $LAB_GROUP (refusing one that exists) in $LAB_REGION:
# a proximity placement group, a VNet with a management subnet (10.20.0.0/24,
# SSH from this machine's address only) and a data subnet (10.20.1.0/24), and
# two $LAB_SIZE VMs (Debian 13), each with an accelerated management NIC and two
# accelerated data NICs: the media driver's sender port (A .4, B .6) and
# receiver port (A .5, B .7). Then, on both VMs at once (../vm.sh, ../lib.sh): bootstrap, a
# reboot into the backports kernel, vm.sh build. Then $LAB_REPS reps of every
# mode in $LAB_MODES (a fresh random order each rep): both drivers started in
# the mode, a round-trip run (A pings, B pongs) and a throughput run (A
# publishes, B subscribes). Results go to target/md-dpdk-bench/<stamp>/ and the
# tables to its summary.md (../analyse.py). The group is deleted on any exit.
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
#   LAB_MAX_HOURS (8): Azure deallocates both VMs that long after creation,
#   should this machine never delete the group (asleep, offline)
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
# x86 only: production runs on x86. Azure's Arm sizes carry a `p` after the
# family letters (Standard_D2ps_v6, Standard_E4pds_v6, ...).
for s in "$size_a" "$size_b"; do
    if [[ $s =~ ^Standard_[A-Z]+[0-9]+[a-z]*p[a-z]*_ ]]; then
        echo "LAB_SIZE $s is an Arm size: the benchmarks run on x86 only" >&2
        exit 2
    fi
done
image=${LAB_IMAGE:-Debian:debian-13:13-gen2:latest}
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
base=${LAB_BASE:-}
# both VMs deallocate themselves this long after creation (hours)
max_hours=${LAB_MAX_HOURS:-8}
# a warm-up load before each mode's first ladder, unrecorded (lib.sh
# load_sweep): its seconds and messages a second; none by default
warmup_seconds=${LAB_WARMUP_SECONDS:-0}
warmup_rate=${LAB_WARMUP_RATE:-0}
tput_seconds=${TPUT_SECONDS:-5}
user=azureuser
out=$repo/target/md-dpdk-bench/$(date +%Y%m%d-%H%M%S)
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
    why=$(mode_check "$m" remote x86_64 azure "" 2>&1) || die "$why"
    # the layout's fifth core on both sizes (vm.sh ready would find it missing
    # only after the build)
    why=$(spare_types_check "$m" azure "$size_a" "$size_b" 2>&1) || die "$why"
done
for m in ${local_modes:-}; do
    why=$(mode_check "$m" local x86_64 azure "" 2>&1) || die "$why"
done
[[ -z $no_flatout || $no_flatout == 1 ]] || die "LAB_NO_FLATOUT is 1 or unset"
[[ $rtt_spike_ns =~ ^[1-9][0-9]{0,9}$ ]] || die "LAB_RTT_SPIKE_NS is a whole number of nanoseconds, from 1 (the round trips stamped), not $rtt_spike_ns"
[[ $rtt_stall_ns =~ ^[1-9][0-9]{0,9}$ ]] || die "LAB_RTT_STALL_NS is a whole number of nanoseconds, from 1 (the client loop stalls stamped), not $rtt_stall_ns"
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
command -v az >/dev/null || die "az is not on PATH"
az account show -o none 2>/dev/null || die "az is not logged in (az login)"
mkdir -p "$out"

teardown() {
    local rc=$?
    # an interrupt must not abort the delete, nor a create still running race it
    trap '' INT TERM
    [[ -n ${follow:-} ]] && kill "$follow" 2>/dev/null
    [[ -n ${pa:-} ]] && kill "$pa" "${pb:-}" 2>/dev/null
    if ((created)) && [[ -z ${LAB_KEEP:-} ]]; then
        log "deleting resource group $group"
        { az group delete -n "$group" --yes --no-wait -o none &&
            az group wait -n "$group" --deleted --timeout 3600 -o none; } ||
            log "WARNING: delete $group by hand: az group delete -n $group --yes"
        if [[ $(az group exists -n "$group") == false ]]; then
            log "verified: resource group $group no longer exists"
        else
            log "WARNING: resource group $group still exists: az group delete -n $group --yes"
        fi
    elif ((created)); then
        log "LAB_KEEP: resource group $group kept; delete it with: az group delete -n $group --yes"
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
log "creating $group in $region: a $size_a, b $size_b, SSH from $me"
az group create -n "$group" -l "$region" -o none || die "group create failed"
created=1
az ppg create -g "$group" -n ppg -l "$region" --intent-vm-sizes $(printf "%s\n" "$size_a" "$size_b" | sort -u) -o none || die "ppg create failed"
az network vnet create -g "$group" -n vnet --address-prefixes 10.20.0.0/16 \
    --subnet-name mgmt --subnet-prefixes 10.20.0.0/24 -o none || die "vnet create failed"
az network vnet subnet create -g "$group" --vnet-name vnet -n data --address-prefixes 10.20.1.0/24 -o none ||
    die "data subnet create failed"
az network nsg create -g "$group" -n nsg -o none || die "nsg create failed"
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
    az network public-ip create -g "$group" -n "$n-ip" --sku Standard -o none &&
        az network nic create -g "$group" -n "$n-mgmt" --vnet-name vnet --subnet mgmt --network-security-group nsg \
            --public-ip-address "$n-ip" --accelerated-networking true -o none &&
        az network nic create -g "$group" -n "$n-snd" --vnet-name vnet --subnet data --private-ip-address "$2" \
            --accelerated-networking true -o none &&
        az network nic create -g "$group" -n "$n-rcv" --vnet-name vnet --subnet data --private-ip-address "$3" \
            --accelerated-networking true -o none &&
        az vm create -g "$group" -n "$n" --size "$4" --image "$image" --ppg ppg \
            --nics "$n-mgmt" "$n-snd" "$n-rcv" --admin-username "$user" --ssh-key-values "$key" \
            --os-disk-size-gb 64 -o none
}
declare_node a 10.20.1.4 10.20.1.5 "$size_a" & pa=$!
declare_node b 10.20.1.6 10.20.1.7 "$size_b" & pb=$!
wait $pa || die "node a create failed"
wait $pb || die "node b create failed"
A=$(az network public-ip show -g "$group" -n a-ip --query ipAddress -o tsv)
B=$(az network public-ip show -g "$group" -n b-ip --query ipAddress -o tsv)
a_private=$(az network nic show -g "$group" -n a-mgmt --query 'ipConfigurations[0].privateIPAddress' -o tsv)
b_private=$(az network nic show -g "$group" -n b-mgmt --query 'ipConfigurations[0].privateIPAddress' -o tsv)
log "a=$A b=$B (private $a_private $b_private)"
# The dead-man's switch: Azure deallocates both VMs (their compute stops
# billing) LAB_MAX_HOURS after creation, should this machine never delete the
# group (asleep, offline). A schedule in the group, so it goes with the group.
off=$(date -u -v+"${max_hours}"H +%H%M 2>/dev/null || date -u -d "+${max_hours} hours" +%H%M)
for n in a b; do
    az vm auto-shutdown -g "$group" -n "$n" --time "$off" -o none ||
        log "WARNING: no auto-shutdown on $n: only this machine's teardown will delete it"
done

data_subnet=10.20.1.0/24
. "$here/../lib.sh"
prepare_vms
run_bench
