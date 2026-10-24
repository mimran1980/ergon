#!/usr/bin/env bash
# Kernel UDP against DPDK (ENA) for the Aeron media driver, on two EC2 instances
# that exist only while this script runs. Runs on macOS or Linux with the aws
# CLI signed in (`aws login`) and an SSH public key (~/.ssh/id_ed25519.pub or
# id_rsa.pub).
#
#   md-dpdk/benches/cloud/aws/lab.sh                                   c7i.2xlarge
#   LAB_TYPE=c7i.metal-24xl md-dpdk/benches/cloud/aws/lab.sh
#   LAB_PLACEMENT=precision-time md-dpdk/benches/cloud/aws/lab.sh        NIC RX timestamps
#   LAB_ALLOW_ARM=1 LAB_TYPE=c8g.4xlarge md-dpdk/benches/cloud/aws/lab.sh   Graviton
#   LAB_PLACEMENT=cross-az LAB_NO_FLATOUT=1 LAB_LOAD_SIZES=288 LAB_LOAD_LADDER=288:10000 ...
#   LAB_PLACEMENT=cross-region LAB_REGION2=eu-west-1 LAB_NO_FLATOUT=1 ...
#   LAB_ESTIMATE=1 <any of the above>                                   its cost, nothing created
#
# Creates, in $LAB_REGION, a VPC of its own (10.30.0.0/16) with a management
# subnet (10.30.0.0/24, an internet gateway, SSH from this machine's address
# only) and a data subnet (10.30.1.0/24), a cluster placement group (with
# LAB_PLACEMENT=precision-time, nested in a precision-time parent group, so the
# instances also get the Nitro PTP hardware clock and NIC RX timestamps: refused
# before any cloud call on a type whose family has no PTP hardware clock, and
# without a DPDK mode with RUSTERON_DPDK_RX_HW_TIMESTAMP=0 beside the DPDK modes
# asking for them, as asking changes the ENA's RX descriptors), an
# imported key pair, and two $LAB_TYPE instances (Debian 13), each with its
# primary ENI on the management subnet and two ENIs attached on the data
# subnet: the media driver's sender port (a .4, b .6) and receiver port (a .5,
# b .7). Everything carries the tags owner=ergon-lab and run=<stamp>, and is
# deleted on any exit, then the deletion is verified by tag (runs at once each
# claim a stamp of their own: one started in another's second takes the next).
# The benchmark itself is ../lib.sh and ../vm.sh, as on Azure. Both hosts keep
# time with chrony on the Amazon Time Sync Service (vm.sh bootstrap; with
# precision-time on the PTP hardware clock), each state waiting for it to
# settle, and its view of the clock logged around every mode.
#
# LAB_PLACEMENT=cross-az: one VPC, a's subnets in one zone (10.30.0.0/24, data
# 10.30.1.0/24), b's in another (10.30.2.0/24, data 10.30.3.0/24: b's data
# ENIs at .6 and .7), no placement group; LAB_AZS="<a's zone> <b's zone>", or
# the first pair of zones with capacity for both types. Each host reaches the
# other's data subnet through its own subnet's gateway (DPDK's
# RUSTERON_DPDK_*_GATEWAY; kernel UDP a route in each data NIC's table).
# LAB_PLACEMENT=cross-region: b in LAB_REGION2, in a VPC of its own there
# (10.31.0.0/16, data 10.31.1.0/24), each region's Debian AMI and the key pair
# imported in both; the VPCs peered (inter-Region VPC peering: requested here,
# accepted there, a route to the other VPC on each side, every subnet on it;
# the security groups take the other VPC's CIDR, as a group cannot name one in
# another region), everything tagged as above, the peering connection in both
# regions; teardown and LAB_DELETE delete and verify in both (the regions are
# kept in the run's directory). Traffic between zones or regions is billed per
# GB, so both need LAB_NO_FLATOUT=1, refused otherwise before any cloud call.
# Neither takes a placement group, so neither takes precision-time.
#
#   LAB_REGION (us-east-1) LAB_TYPE (c7i.2xlarge; LAB_TYPE_A, LAB_TYPE_B per instance:
#   a hosts the localhost runs, which need five cores, so c7i.4xlarge for those)
#   LAB_ALLOW_ARM=1: Graviton (Arm) types too (c7g, c7gn, c8g, c8gn, c9g, ...), refused
#   without it, as production runs on x86. Both instances are of one architecture
#   (AWS confirms arm64), on Debian 13's arm64 AMI, with vm.sh's Graviton settings
#   (../host.sh); should md-dpdk's dpdk feature not build there, DPDK modes stop the
#   run before its first state and kernel modes run (lib.sh dpdk_ready)
#   LAB_REPS (5) LAB_PLACEMENT (cluster; precision-time, cross-az, cross-region: above)
#   LAB_NO_FLATOUT=1: the cost guard: no phase that sends as fast as it can (no 32 B
#   throughput run, no sustained maximum, no several flows at once); every
#   LAB_LOAD_SIZES size then needs its LAB_LOAD_LADDER rates (its fractions are of the
#   flat-out maximum), and LAB_TPUT_FLOWS is refused
#   LAB_ESTIMATE=1: print the plan's instance-hours, the GB it sends between the hosts
#   and its dollars (../estimate.py: on-demand prices, data transfer by placement),
#   then stop, with no cloud call (with LAB_SPOT=1 the on-demand figure, as an upper
#   bound: spot costs less, at a price not looked up)
#   LAB_SPOT=1: every instance of the run a one-time spot instance (run-instances
#   --instance-market-options: SpotInstanceType=one-time, InstanceInterruptionBehavior=
#   terminate), tagged, deadlined (LAB_MAX_HOURS) and deleted exactly as an on-demand
#   one; its spot request tagged with the run too (ResourceType=spot-instances-request),
#   and the teardown cancels any of the run's still open or active and lists them in its
#   verification (the run's directory says spot, so LAB_DELETE does too). While the
#   instances run, each one's state is looked at every 30 s (describe-instances, the
#   CLI lock taken for each look): one AWS reclaims (Server.SpotInstanceTermination)
#   ends the run through the normal teardown as soon as the step in hand returns (an
#   SSH step to the host that went waits out its keepalive, 5 minutes at most), saying
#   so, exit 75 (EX_TEMPFAIL: try again), with LAB_KEEP=1 too, which keeps nothing of a
#   broken pair; one gone otherwise (the deadline, by hand) ends it too. The market
#   (spot or on-demand) is in instances.txt and each host's env file
#   LAB_THREADS_PER_CORE=1: SMT off from launch, every core of the type at one thread
#   (run-instances --cpu-options); refused for Graviton, bare metal, C7a, M7a, R7a, T2.
#   Per mode instead: <mode>@SMT=off (x86; vm.sh)
#   LAB_MODES ("kernel kernel-busy dpdk"; the raw floor's kernel-raw and dpdk-raw run
#   rtt's round trip with no Aeron, over kernel UDP or the DPDK transport's
#   bindings alone, on kernel-busy's or dpdk's set-up: the round trip only,
#   cross-host, host and RUSTERON_* settings only (../modes.sh);
#   <mode>@AERON_THREADING_MODE=SHARED_NETWORK or
#   SHARED runs the driver's agents that way, each busy one on a core of its own:
#   ../host.sh threading_env; INVOKER runs no media driver: each client embeds it
#   (HARNESS_DRIVER=invoker), cross-host only; DEDICATED otherwise. Also @JUMBO=1
#   (jumbo frames: 9001-byte frames, 8500 cross-region, Aeron's MTU 8960 or 8448,
#   the path checked first), @SMT=off, @PUB_PARAMS=<k>=<v>+... and
#   @SUB_PARAMS=<k>=<v>+... (Aeron URI parameters on the clients' channels: + for
#   Aeron's |), and, kernel modes only, the data NICs' interrupts and NAPI:
#   @IRQ=hk|receiver|spare (their queue IRQs on housekeeping's CPU, the Aeron
#   receiver agent's, or the layout's fifth core), @NAPI_DEFER=<n>/<ns>
#   (napi_defer_hard_irqs, gro_flush_timeout) and @NAPI_BUSY=spare (threaded NAPI
#   busy polling, its kthreads on the fifth core; Linux 6.17 and up); in any mode,
#   @CONDUCTOR=own (the driver's conductor alone on the fifth core, spinning; not
#   beside the other two on it). @HAIRPIN=1 (the raw floor's modes): the round
#   trip on host a alone, from its sender ENI to its receiver ENI and back
#   (../modes.sh), both hosts of four cores or more; @RTT_GAP_US=<us> (1 to 1000000,
#   cross-host): one ping every that many microseconds, a rep of 20 to 60 s
#   (../modes.sh rtt_plan: 20,000 round trips at 1 ms, 1,200 at 50 ms, 300 at
#   200 ms, 60 at 1 s; p99.99 means something from 10,000). The fifth core is not on a four-core type
#   (c7i.2xlarge, c8g.xlarge, c9g.xlarge): refused here, before any cloud call, by
#   the types' physical cores (../modes.sh type_cores: a type its table lacks is
#   refused too, said so), as NAPI_BUSY is after the build on a kernel or host that
#   cannot. Any other upper-case @KEY must be an AERON_* or RUSTERON_* variable, and
#   the DPDK transport's RUSTERON_DPDK_* values must be ones it and the NIC take.
#   Each checked here: ../modes.sh. Message sizes go up to 8192 bytes;
#   a size over a mode's largest message is skipped in that mode, said so first)
#   LAB_WARMUP_SECONDS (0: none) and LAB_WARMUP_RATE: a warm-up load of that many
#   seconds at that rate before each mode's first load ladder, in each rep, at its
#   first size, recorded nowhere (Aeron's own harness warms 30 s at 25,000 msgs/s;
#   the load client's own warm-up is its first second at each point's rate); refused
#   without LAB_LOAD_SIZES, where it would never run
#   LAB_STATES ("pinned isolated"; isolated-poll: isolated, and idle CPUs poll rather
#   than halt, x86 idle=poll, Graviton nohlt; any of them ending -nodiag: that state
#   without the diagnostics, vm.sh diag-start, for an A/A that bounds their effect)
#   LAB_LOCAL_MODES, LAB_LOAD_SIZES, LAB_LOAD_FRACTIONS, LAB_LOAD_SECONDS,
#   LAB_LOAD_LADDER, LAB_MIX, LAB_MIX_SECONDS, LAB_RTT_ALIGN, LAB_RTT_SPIKE_NS, LAB_RTT_STALL_NS, LAB_GAPSPIN_US, LAB_BASE, LAB_TPUT_FLOWS: as azure/lab.sh
#   (LAB_TPUT_FLOWS: one UDP flow carries at most 8.0 Gbit/s of frames here, on every size
#   measured; LAB_TPUT_FLOWS=2, where both instances have a fifth core (c7i.4xlarge and
#   up), adds the sustained maximum over two flows at once, which shows whether two carry
#   twice that, up to the instance's own bandwidth: 12.5 Gbit/s on c7i.4xlarge, 37.5 on
#   c7i.metal-24xl. Unset, 0 or 1, it does not run)
#   LAB_KEEP=1 keeps everything (delete it with LAB_DELETE=<stamp> lab.sh);
#   LAB_MAX_HOURS (8): each instance terminates itself that long after launch,
#   should this machine never delete it (asleep, offline), LAB_KEEP or not; a plan
#   estimated to take longer (LAB_ESTIMATE=1 says so) is refused before any cloud
#   call, with the hours to give it, and each state is waited for as long;
#   LAB_REPS=0 with LAB_KEEP=1 stops after the build, for work by hand
#   RTT_MESSAGES (300000) RTT_WARMUP (50000) TPUT_SECONDS (5)
#   To stop a run early: kill -TERM <its pid> (the one whose parent is 1). Bash
#   ignores SIGINT while an ssh step runs that does not die of it, so Ctrl-C can
#   take until that step ends; TERM runs the teardown as soon as the step returns.
#   Once the teardown has begun, TERM and Ctrl-C are ignored until it has deleted and
#   verified everything (kill -KILL abandons it: then LAB_DELETE=<stamp> lab.sh).
set -uo pipefail
export AWS_PAGER=""
# The original stderr, kept for the teardown: a TERM (an operator's, or the
# spot watch's) that lands in a step whose stderr is redirected (on ...
# 2>/dev/null) runs the trap, and so the teardown, with that redirection in
# place, which would hide its every line, the verification's included.
exec 9>&2

here=$(cd "$(dirname "$0")" && pwd)
repo=${LAB_REPO:-$(cd "$here/../../../.." && pwd)}
# this script in the checkout, for the commands the run prints (LAB_DELETE=
# <stamp> ...): the snapshot it runs from is gone once it exits
self=${LAB_SELF:-$here/${0##*/}}
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
    LAB_SNAPSHOT=$snapshot LAB_REPO=$repo LAB_SELF=$self exec bash "$snapshot/cloud/${here##*/}/${0##*/}" "$@"
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
region=${LAB_REGION:-us-east-1}
type=${LAB_TYPE:-c7i.2xlarge}
type_a=${LAB_TYPE_A:-$type}
type_b=${LAB_TYPE_B:-$type}
# Graviton (Arm) types only with LAB_ALLOW_ARM=1: production runs on x86.
# Graviton families carry a `g` after the generation (c7g, c7gn, c8g, m8g,
# ...), and a1 is the first. Both instances must be of one architecture (one
# AMI, one build). Refused before any cloud call; LAB_DELETE, which only
# deletes, never is.
allow_arm=${LAB_ALLOW_ARM:-}
arm_type() { [[ ${1%%.*} =~ ^[a-z]+[0-9]+g || ${1%%.*} == a1 ]]; }
arch=x86_64
if [[ -z ${LAB_DELETE:-} ]]; then
    for t in "$type_a" "$type_b"; do
        if arm_type "$t" && [[ $allow_arm != 1 ]]; then
            echo "LAB_TYPE $t is a Graviton (Arm) type: production runs on x86, so Arm runs only with LAB_ALLOW_ARM=1" >&2
            exit 2
        fi
    done
    arm_a=0 arm_b=0
    arm_type "$type_a" && arm_a=1
    arm_type "$type_b" && arm_b=1
    if ((arm_a != arm_b)); then
        echo "LAB_TYPE_A $type_a and LAB_TYPE_B $type_b: both instances must be of one architecture (one AMI, one build)" >&2
        exit 2
    fi
    ((arm_a)) && arch=arm64
fi
placement=${LAB_PLACEMENT:-cluster}
[[ $placement =~ ^(cluster|precision-time|cross-az|cross-region)$ ]] ||
    { echo "LAB_PLACEMENT is cluster, precision-time, cross-az or cross-region" >&2; exit 2; }
# cross-region: b in this region, a in $region
region2=${LAB_REGION2:-}
# cross-az: a's zone and b's ("" = the first pair of zones with capacity)
azs=${LAB_AZS:-}
# the cost guard: no phase that sends as fast as it can (lib.sh)
no_flatout=${LAB_NO_FLATOUT:-}
threads_per_core=${LAB_THREADS_PER_CORE:-}
# one-time spot instances (LAB_SPOT=1), or on demand
spot=${LAB_SPOT:-}
market=on-demand
[[ $spot == 1 ]] && market=spot
reps=${LAB_REPS:-5}
read -ra modes <<<"${LAB_MODES:-kernel kernel-busy dpdk}"
read -ra states <<<"${LAB_STATES:-pinned isolated}"
local_modes=${LAB_LOCAL_MODES:-}
load_sizes=${LAB_LOAD_SIZES:-}
load_fractions=${LAB_LOAD_FRACTIONS:-10 50 80 95 100}
load_seconds=${LAB_LOAD_SECONDS:-5}
load_ladder=${LAB_LOAD_LADDER:-}
mix=${LAB_MIX:-}
mix_seconds=${LAB_MIX_SECONDS:-10}
tput_flows=${LAB_TPUT_FLOWS:-}
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
# the instances' own deadline (hours from launch), in case this machine never
# gets to delete them (it slept, lost its network): see deadline-userdata.sh
max_hours=${LAB_MAX_HOURS:-8}
# a warm-up load before each mode's first ladder, unrecorded (lib.sh
# load_sweep): its seconds and messages a second (Aeron's own harness warms
# 30 s at 25,000); none by default
warmup_seconds=${LAB_WARMUP_SECONDS:-0}
warmup_rate=${LAB_WARMUP_RATE:-0}
tput_seconds=${TPUT_SECONDS:-5}
user=admin
stamp=${LAB_DELETE:-$(date +%Y%m%d-%H%M%S)}
out=$repo/target/md-dpdk-bench/aws-$stamp
known=$out/known_hosts
data_subnet=10.30.1.0/24
created=0

log() { echo "[$(date +%T)] $*" >&2; }
die() { log "ERROR: $*"; exit 1; }
# One AWS CLI call at a time on this machine: `aws login` refreshes its token
# into a shared cache, and two refreshes at once (labs, billing checks, other
# sessions) leave it corrupt, after which every call fails, teardown included.
# The lock is the one the other lab sessions on this machine take.
aws_cli() {
    perl -MFcntl=:flock -e '$^F = 255; open(my $l, ">>", shift) or die "lock: $!"; flock($l, LOCK_EX) or die "lock: $!"; exec @ARGV' \
        "${TMPDIR:-/tmp}/ergon-lab-aws.lock" aws "$@"
}
ec2() { aws_cli ec2 --region "$region" "$@"; }
tags() { echo "ResourceType=$1,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$stamp},{Key=Name,Value=ergon-md-dpdk-$2}]"; }
tagged() { echo "Name=tag:run,Values=$stamp"; }

# a base revision that cannot be built is found before anything is created
# (never in the way of LAB_DELETE, which builds nothing)
if [[ -n $base && -z ${LAB_DELETE:-} ]] && ! git -C "$repo" rev-parse --verify -q "$base^{commit}" >/dev/null; then
    die "LAB_BASE $base: not a commit in $repo"
fi
if [[ -n $tput_flows && -z ${LAB_DELETE:-} && ! $tput_flows =~ ^[0-9]+$ ]]; then
    die "LAB_TPUT_FLOWS $tput_flows: not a number of flows"
fi
# Every mode's settings (modes.sh mode_check), each run where vm.sh would
# otherwise find it wrong only at that mode's first driver start, or never
# (an Aeron threading mode Aeron does not know runs as DEDICATED, with a
# warning only; an Aeron URI parameter it does not know is ignored); a comma
# in a mode would split every results.csv row of it (COALESCE takes
# <rx>/<tx>); an MTU the path or the DPDK transport cannot carry, and an
# override the transport refuses at install (a network agent that does not
# spin, a pacing wait under a one-agent mode), would fail or drop frames
# hours in; and a state lib.sh does not know runs as pinned. All refused
# here, before any cloud call.
. "$here/../modes.sh"
if [[ -z ${LAB_DELETE:-} ]]; then
    for m in "${modes[@]}"; do
        why=$(mode_check "$m" remote "$arch" aws "$threads_per_core" "$placement" 2>&1) || die "$why"
        # the layout's fifth core on both types (vm.sh ready would find it
        # missing only after the build)
        why=$(spare_types_check "$m" aws "$type_a" "$type_b" 2>&1) || die "$why"
        # a hairpin's ping and pong on host a's own cores (host b sets the
        # mode up too, so its type is checked as well)
        why=$(hairpin_types_check "$m" aws "$type_a" "$type_b" 2>&1) || die "$why"
    done
    for m in ${local_modes:-}; do
        why=$(mode_check "$m" local "$arch" aws "$threads_per_core" "$placement" 2>&1) || die "$why"
    done
    for s in "${states[@]}"; do
        [[ $s =~ ^(pinned|isolated|isolated-poll)(-nodiag)?$ ]] ||
            die "LAB_STATES $s: pinned, isolated or isolated-poll, optionally ending -nodiag"
    done
    # precision-time: a PTP hardware clock on both types (chrony's reference,
    # and the NIC's RX timestamps), and the DPDK modes' timestamp pair
    # (modes.sh precision_modes_check)
    if [[ $placement == precision-time ]]; then
        phc_families=${PHC_FAMILIES# }
        for t in "$type_a" "$type_b"; do
            phc_type "$t" ||
                die "LAB_PLACEMENT=precision-time: $t has no PTP hardware clock: AWS gives one, in a precision-time placement group, to the families ${phc_families% } (modes.sh PHC_FAMILIES)"
        done
        # shellcheck disable=SC2086
        why=$(precision_modes_check "${modes[@]}" ${local_modes:-} 2>&1) || die "$why"
    fi
    [[ $warmup_seconds =~ ^[0-9]{1,6}$ ]] || die "LAB_WARMUP_SECONDS is a whole number of seconds (0: no warm-up), not $warmup_seconds"
    [[ $warmup_rate =~ ^[0-9]{1,9}$ ]] || die "LAB_WARMUP_RATE is a whole number of messages a second, not $warmup_rate"
    ((10#$warmup_seconds == 0 || 10#$warmup_rate > 0)) ||
        die "LAB_WARMUP_SECONDS=$warmup_seconds needs LAB_WARMUP_RATE, its messages a second (Aeron's own harness warms 30 s at 25,000)"
    # the warm-up runs before each mode's first load ladder (lib.sh
    # load_sweep): with no load sizes there is none, and it would never run
    ((10#$warmup_seconds == 0)) || [[ -n ${load_sizes// /} ]] ||
        die "LAB_WARMUP_SECONDS=$warmup_seconds without LAB_LOAD_SIZES: the warm-up runs before each mode's first load ladder, so it would never run (give LAB_LOAD_SIZES, or no warm-up)"
    # where the hosts are, and what that costs: traffic between two zones or
    # two regions is billed per GB, so those placements run nothing flat out
    [[ -z $no_flatout || $no_flatout == 1 ]] || die "LAB_NO_FLATOUT is 1 or unset"
    [[ -z $spot || $spot == 1 ]] || die "LAB_SPOT is 1 (one-time spot instances) or unset"
    # removed with the clock benchmarks: refused, never a full run in their place
    [[ -z ${LAB_CLOCK_ONLY:-}${LAB_DISCIPLINE_SECONDS:-} ]] ||
        die "LAB_CLOCK_ONLY and LAB_DISCIPLINE_SECONDS were removed with the clock benchmarks: unset them"
    [[ $max_hours =~ ^[1-9][0-9]*$ ]] || die "LAB_MAX_HOURS is a whole number of hours, from 1 (the instances' deadline), not $max_hours"
    [[ $rtt_spike_ns =~ ^[1-9][0-9]{0,9}$ ]] || die "LAB_RTT_SPIKE_NS is a whole number of nanoseconds, from 1 (the round trips stamped), not $rtt_spike_ns"
    [[ $rtt_stall_ns =~ ^[1-9][0-9]{0,9}$ ]] || die "LAB_RTT_STALL_NS is a whole number of nanoseconds, from 1 (the client loop stalls stamped), not $rtt_stall_ns"
    [[ $gapspin_us =~ ^[1-9][0-9]{0,5}$ ]] || die "LAB_GAPSPIN_US is a whole number of microseconds, 1 to 999999 (the gap spinner's threshold), not $gapspin_us"
    if [[ $placement == cross-az || $placement == cross-region ]] && [[ $no_flatout != 1 ]]; then
        die "LAB_PLACEMENT=$placement: the traffic between the hosts is billed per GB, so no flat-out phases: set LAB_NO_FLATOUT=1 (LAB_ESTIMATE=1 prints the cost)"
    fi
    if [[ $placement == cross-region ]]; then
        [[ -n $region2 ]] || die "LAB_PLACEMENT=cross-region needs LAB_REGION2, b's region"
        [[ $region2 != "$region" ]] || die "LAB_REGION2 $region2: another region than LAB_REGION's"
    elif [[ -n $region2 ]]; then
        die "LAB_REGION2 is for LAB_PLACEMENT=cross-region"
    fi
    if [[ -n $azs ]]; then
        [[ $placement == cross-az ]] || die "LAB_AZS is for LAB_PLACEMENT=cross-az"
        read -ra zone_pair <<<"$azs"
        ((${#zone_pair[@]} == 2)) && [[ ${zone_pair[0]} != "${zone_pair[1]}" ]] ||
            die "LAB_AZS $azs: two zones, a's then b's (e.g. \"${region}a ${region}b\")"
    fi
    # SMT off from launch: AWS takes CPU options for neither bare metal nor
    # Graviton (no SMT), nor C7a, M7a, R7a or T2 (one thread a core already)
    if [[ -n $threads_per_core ]]; then
        [[ $threads_per_core == 1 ]] || die "LAB_THREADS_PER_CORE is 1 (SMT off from launch) or unset"
        for t in "$type_a" "$type_b"; do
            ! arm_type "$t" || die "LAB_THREADS_PER_CORE=1: $t is Graviton, which has no SMT (AWS takes no threads per core for it)"
            [[ $t != *metal* ]] || die "LAB_THREADS_PER_CORE=1: $t is bare metal, which takes no CPU options (AWS)"
            [[ ! ${t%%.*} =~ ^(c7a|m7a|r7a|t2)$ ]] || die "LAB_THREADS_PER_CORE=1: AWS does not let C7a, M7a, R7a or T2 ($t) change threads per core"
        done
    fi
    if [[ $no_flatout == 1 ]]; then
        [[ ${tput_flows:-0} -lt 2 ]] || die "LAB_TPUT_FLOWS runs several flows flat out: not with LAB_NO_FLATOUT=1"
        for size in $load_sizes; do
            [[ " $load_ladder " == *" $size:"* ]] ||
                die "LAB_NO_FLATOUT=1: size $size has no LAB_LOAD_LADDER rates (LAB_LOAD_FRACTIONS are fractions of the flat-out maximum, which does not run)"
        done
    fi
    # message sizes: up to 8192 bytes, each run in the modes whose MTU fits it
    # (modes.sh mode_payload; jumbo frames for the large ones) and skipped,
    # said so here, in the rest; one that fits no mode is refused. Several
    # flows at once run at 32 B too. A mix profile runs where all its streams fit.
    sweeping=()
    for m in "${modes[@]}" ${local_modes:-}; do [[ $m == *+idle ]] || sweeping+=("$m"); done
    mix_sizes=$(tr ';' '\n' <<<"$mix" | sed 's/^[^=]*=//' | tr ' ' '\n' | awk -F: 'NF >= 4 {print $4}')
    for size in $load_sizes $mix_sizes; do
        [[ $size =~ ^[1-9][0-9]*$ ]] && ((size <= 8192)) || die "message size $size: a number of bytes up to 8192"
        fits=0
        for m in "${sweeping[@]}"; do
            payload=$(mode_payload "$m" "$placement")
            if ((size <= payload)); then fits=1; else
                echo "$size B is over mode $m's largest message ($payload B at MTU $(mode_mtu "$m" "$placement")): skipped there" >&2
            fi
        done
        ((fits)) || die "message size $size B fits no mode's MTU: give one @JUMBO=1 (up to $(($(jumbo_mtus "$placement" | cut -d' ' -f2) - 32)) B)"
    done
fi
# LAB_ESTIMATE=1: what the plan would cost (estimate.py: instance-hours, GB
# between the hosts, dollars), then stop, before any cloud call
plan=(--type-a "$type_a" --type-b "$type_b" --placement "$placement" --reps "$reps"
    --modes "${modes[*]}" --local-modes "$local_modes" --states "${states[*]}" --load-sizes "$load_sizes"
    --load-fractions "$load_fractions" --load-ladder "$load_ladder" --load-seconds "$load_seconds" --mix "$mix"
    --mix-seconds "$mix_seconds" --tput-seconds "$tput_seconds" --tput-flows "${tput_flows:-0}"
    --rtt-messages "$rtt_messages" --rtt-warmup "$rtt_warmup" --no-flatout "$no_flatout" --max-hours "$max_hours"
    --market "$market" --warmup-seconds "$warmup_seconds" --warmup-rate "$warmup_rate")
if [[ ${LAB_ESTIMATE:-} == 1 && -z ${LAB_DELETE:-} ]]; then
    python3 "$here/../estimate.py" "${plan[@]}"
    exit $?
fi
# a plan longer than the instances' own deadline (LAB_MAX_HOURS) would be
# cut short by it, hours in: refused here, with the hours to give it
if [[ -z ${LAB_DELETE:-} ]]; then
    why=$(python3 "$here/../estimate.py" "${plan[@]}" --refuse-over-deadline 2>&1) || die "$why"
fi
command -v aws >/dev/null || die "aws is not on PATH"
aws_cli sts get-caller-identity >/dev/null 2>&1 || die "aws is not signed in (aws login)"
if [[ -n ${LAB_DELETE:-} ]]; then
    mkdir -p "$out"
else
    # A run directory of its own, claimed (mkdir fails on one there): runs
    # started in one second (three pairs at once) would otherwise share a
    # stamp, and with it their tags, names and results, each one's teardown
    # deleting the others' resources. Taken: the next second's stamp.
    mkdir -p "${out%/*}" || die "cannot make ${out%/*}"
    claimed=
    for _ in $(seq 60); do
        if mkdir "$out" 2>/dev/null; then claimed=1; break; fi
        sleep 1
        stamp=$(date +%Y%m%d-%H%M%S) out=$repo/target/md-dpdk-bench/aws-$stamp known=$out/known_hosts
    done
    [[ -n $claimed ]] || die "no run directory of its own under ${out%/*} in 60 tries"
fi
# LAB_DELETE of a spot run: its directory says so (lab.sh writes it before
# creating anything), so its spot requests are swept even without LAB_SPOT=1
if [[ -n ${LAB_DELETE:-} && $(cat "$out/market" 2>/dev/null) == spot ]]; then
    spot=1 market=spot
fi
# a run whose instance AWS reclaimed (spot_gone): lib.sh's on() then tries no
# more SSH, so no step waits out its retries before the teardown
lab_abandon=$out/instance-gone

# The regions a run uses: LAB_DELETE reads them back from its directory
# (lab.sh writes them before creating anything), so a cross-region run is
# swept in both even when LAB_REGION2 is not given again; LAB_REGION2, if
# given, is added
regions=$region
[[ -n $region2 ]] && regions+=" $region2"
if [[ -n ${LAB_DELETE:-} && -s $out/regions ]]; then
    regions=$(printf '%s\n' $(cat "$out/regions") $regions | awk '!seen[$0]++' | tr '\n' ' ')
    regions=${regions% }
fi
ec2r() { local r=$1; shift; aws_cli ec2 --region "$r" "$@"; }

# wait_state <region> <state> <instance id>...: poll until every instance is
# in <state> (a call every 10 s, up to 10 minutes), the CLI lock taken for
# each call and never across a wait (`aws ec2 wait` would hold it throughout:
# every other session's call, `aws login`'s refresh included, behind it); 1,
# saying so, if they never are, or at once when waiting for running and one
# is already shutting down or gone (it never will be)
wait_state() {
    local r=$1 want=$2 got=
    shift 2
    for _ in $(seq 60); do
        got=$(ec2r "$r" describe-instances --instance-ids "$@" --query 'Reservations[].Instances[].State.Name' --output text) &&
            [[ -n $got && -z $(tr -s '\t ' '\n\n' <<<"$got" | grep -vx "$want") ]] && return 0
        if [[ $want == running && " ${got//$'\t'/ } " =~ \ (shutting-down|terminated)\  ]]; then
            log "instances $* in $r: ${got//$'\t'/ }, so never running"
            return 1
        fi
        sleep 10
    done
    log "timed out after 600 s waiting for $* in $r to be $want (last: ${got:-not listed})"
    return 1
}

# Delete everything tagged with this run, in dependency order, then list again
# by tag and report what (if anything) is left. The instances terminated in
# every region and polled until they are (wait_state: never an aws ... wait,
# which would hold the CLI lock), the peering connection deleted, then each
# region's network, and each region verified.
teardown_run() {
    log "deleting run $stamp in ${regions// / and }"
    local ids id r
    # Credentials can expire during a long run. Without them nothing below can
    # delete or list anything, so the instances are shut down over SSH instead
    # (instance-initiated shutdown terminates them, as the deadline unit
    # does), and the run is reported as not verified, never as clean.
    if ! aws_cli sts get-caller-identity --region "$region" >/dev/null 2>&1; then
        log "WARNING: AWS credentials failed: cannot delete or verify run $stamp"
        local host
        for host in ${A:-} ${B:-}; do
            [[ -n $host && $host != None ]] || continue
            # one try: a host powering off drops the connection (on() would retry it for minutes)
            ssh "${ssh_opts[@]}" "$user@$host" 'sudo systemctl poweroff' 2>/dev/null || true
            log "asked $host to power off (instance-initiated shutdown terminates it)"
        done
        log "WARNING: refresh the credentials, then: LAB_DELETE=$stamp $self (deletes the rest and verifies by tag)"
        return 1
    fi
    # spot (LAB_SPOT=1, or the run's directory says so): a one-time spot
    # request ends with its instance (closed), but any of the run's still open
    # (a launch refused after its request was made) or active is cancelled
    # first, so none can launch an instance once the instances are listed
    if [[ $spot == 1 ]]; then
        for r in $regions; do
            ids=$(ec2r "$r" describe-spot-instance-requests --filters "$(tagged)" Name=state,Values=open,active \
                --query 'SpotInstanceRequests[].SpotInstanceRequestId' --output text)
            # shellcheck disable=SC2086
            [[ -z $ids ]] || ec2r "$r" cancel-spot-instance-requests --spot-instance-request-ids $ids >/dev/null
        done
    fi
    local terminating=()
    for r in $regions; do
        ids=$(ec2r "$r" describe-instances --filters "$(tagged)" "Name=instance-state-name,Values=pending,running,stopping,stopped" \
            --query 'Reservations[].Instances[].InstanceId' --output text)
        if [[ -n $ids ]]; then
            ec2r "$r" terminate-instances --instance-ids $ids >/dev/null
            terminating+=("$r $ids")
        fi
    done
    for ids in ${terminating[@]+"${terminating[@]}"}; do
        read -r r ids <<<"$ids"
        # shellcheck disable=SC2086
        wait_state "$r" terminated $ids || log "WARNING: instances $ids in $r not yet terminated"
    done
    # the peering connection, from whichever region lists it (tagged in both)
    if [[ $regions == *" "* ]]; then
        for r in $regions; do
            for id in $(ec2r "$r" describe-vpc-peering-connections --filters "$(tagged)" \
                --query 'VpcPeeringConnections[?Status.Code!=`deleted`].VpcPeeringConnectionId' --output text); do
                ec2r "$r" delete-vpc-peering-connection --vpc-peering-connection-id "$id" >/dev/null
            done
        done
    fi
    for r in $regions; do
        for id in $(ec2r "$r" describe-network-interfaces --filters "$(tagged)" --query 'NetworkInterfaces[].NetworkInterfaceId' --output text); do
            ec2r "$r" delete-network-interface --network-interface-id "$id" 2>/dev/null
        done
        local vpc
        vpc=$(ec2r "$r" describe-vpcs --filters "$(tagged)" --query 'Vpcs[0].VpcId' --output text)
        if [[ -n $vpc && $vpc != None ]]; then
            for id in $(ec2r "$r" describe-internet-gateways --filters "Name=attachment.vpc-id,Values=$vpc" --query 'InternetGateways[].InternetGatewayId' --output text); do
                ec2r "$r" detach-internet-gateway --internet-gateway-id "$id" --vpc-id "$vpc"
                ec2r "$r" delete-internet-gateway --internet-gateway-id "$id"
            done
            for id in $(ec2r "$r" describe-subnets --filters "Name=vpc-id,Values=$vpc" --query 'Subnets[].SubnetId' --output text); do
                ec2r "$r" delete-subnet --subnet-id "$id"
            done
            # this run's own (tagged) route table; the VPC's main one goes with the VPC
            for id in $(ec2r "$r" describe-route-tables --filters "Name=vpc-id,Values=$vpc" "$(tagged)" \
                --query 'RouteTables[].RouteTableId' --output text); do
                ec2r "$r" delete-route-table --route-table-id "$id"
            done
            for id in $(ec2r "$r" describe-security-groups --filters "Name=vpc-id,Values=$vpc" \
                --query 'SecurityGroups[?GroupName!=`default`].GroupId' --output text); do
                ec2r "$r" delete-security-group --group-id "$id"
            done
            ec2r "$r" delete-vpc --vpc-id "$vpc"
        fi
        ec2r "$r" delete-placement-group --group-name "ergon-md-dpdk-$stamp" 2>/dev/null
        ec2r "$r" delete-placement-group --group-name "ergon-md-dpdk-pt-$stamp" 2>/dev/null
        ec2r "$r" delete-key-pair --key-name "ergon-md-dpdk-$stamp" 2>/dev/null
    done
    # verify: nothing billable (or otherwise) left with this run's tag, in
    # each region; a failed listing is a failed verification, never an empty one
    local left listed clean=0
    for r in $regions; do
        listed=0
        left=$( {
            ec2r "$r" describe-instances --filters "$(tagged)" "Name=instance-state-name,Values=pending,running,stopping,stopped" \
                --query 'Reservations[].Instances[].InstanceId' --output text || echo LISTFAIL
            ec2r "$r" describe-volumes --filters "$(tagged)" --query 'Volumes[].VolumeId' --output text || echo LISTFAIL
            ec2r "$r" describe-network-interfaces --filters "$(tagged)" --query 'NetworkInterfaces[].NetworkInterfaceId' --output text || echo LISTFAIL
            ec2r "$r" describe-addresses --filters "$(tagged)" --query 'Addresses[].AllocationId' --output text || echo LISTFAIL
            ec2r "$r" describe-vpcs --filters "$(tagged)" --query 'Vpcs[].VpcId' --output text || echo LISTFAIL
            ec2r "$r" describe-route-tables --filters "$(tagged)" --query 'RouteTables[].RouteTableId' --output text || echo LISTFAIL
            ec2r "$r" describe-subnets --filters "$(tagged)" --query 'Subnets[].SubnetId' --output text || echo LISTFAIL
            ec2r "$r" describe-security-groups --filters "$(tagged)" --query 'SecurityGroups[].GroupId' --output text || echo LISTFAIL
            ec2r "$r" describe-internet-gateways --filters "$(tagged)" --query 'InternetGateways[].InternetGatewayId' --output text || echo LISTFAIL
            ec2r "$r" describe-placement-groups --filters "$(tagged)" --query 'PlacementGroups[].GroupName' --output text || echo LISTFAIL
            ec2r "$r" describe-key-pairs --filters "$(tagged)" --query 'KeyPairs[].KeyName' --output text || echo LISTFAIL
            if [[ $regions == *" "* ]]; then
                ec2r "$r" describe-vpc-peering-connections --filters "$(tagged)" \
                    --query 'VpcPeeringConnections[?Status.Code!=`deleted`].VpcPeeringConnectionId' --output text || echo LISTFAIL
            fi
            # a spot request still open could launch an instance later
            if [[ $spot == 1 ]]; then
                ec2r "$r" describe-spot-instance-requests --filters "$(tagged)" Name=state,Values=open,active \
                    --query 'SpotInstanceRequests[].SpotInstanceRequestId' --output text || echo LISTFAIL
            fi
        } 2>/dev/null | tr -s ' \t\n' ' ')
        [[ $left != *LISTFAIL* ]] && aws_cli sts get-caller-identity --region "$r" >/dev/null 2>&1 && listed=1
        if ((!listed)); then
            log "WARNING: could not list run $stamp's resources in $r: NOT verified (rerun with LAB_DELETE=$stamp $self)"
            clean=1
        elif [[ -z ${left// /} ]]; then
            log "verified: nothing tagged run=$stamp remains in $r"
        else
            log "WARNING: still present for run $stamp in $r: $left (rerun with LAB_DELETE=$stamp $self)"
        fi
    done
    return $clean
}

if [[ -n ${LAB_DELETE:-} ]]; then
    teardown_run
    exit 0
fi

# spot_gone: one look at the run's two instances (their states and why), the
# CLI lock taken for it alone; 0 when one is shutting down, gone or stopped:
# a spot instance AWS reclaimed (Server.SpotInstanceTermination), or one
# ended otherwise (its deadline, by hand), either of which ends the run. Its
# line (whose instance, a or b, and why) is logged and kept in
# $out/instance-gone, which lib.sh's on() reads as the run's end. 1 while
# both run, or when the look fails (credentials, throttling: the next decides).
spot_gone() {
    local rows id state reason who line= q='Reservations[].Instances[].[InstanceId,State.Name,StateReason.Code]'
    if [[ $region_b == "$region" ]]; then
        rows=$(ec2 describe-instances --instance-ids "$ida" "$idb" --query "$q" --output text 2>/dev/null) || return 1
    else
        rows=$(ec2 describe-instances --instance-ids "$ida" --query "$q" --output text 2>/dev/null) &&
            rows+=$'\n'$(ec2r "$region_b" describe-instances --instance-ids "$idb" --query "$q" --output text 2>/dev/null) || return 1
    fi
    while read -r id state reason; do
        [[ $state =~ ^(shutting-down|terminated|stopping|stopped)$ ]] || continue
        who=a
        [[ $id == "$idb" ]] && who=b
        if [[ $reason == Server.SpotInstanceTermination ]]; then
            line="spot instance $id ($who) was reclaimed by AWS ($reason): run $stamp ends here, through its teardown"
        else
            line="instance $id ($who) is $state (${reason:-no reason given}), not by this run: run $stamp ends here, through its teardown"
        fi
        break
    done <<<"$rows"
    [[ -n $line ]] || return 1
    echo "$line" >"$out/instance-gone"
    log "$line"
}
# spot_watch: in the background while a spot run's instances run, spot_gone
# every 30 s; once one is gone, TERM to lab.sh, whose trap runs the teardown
# as soon as the step in hand returns (on() tries no more SSH by then)
spot_watch() {
    while sleep 30; do
        spot_gone || continue
        kill -TERM $$
        return
    done
}

teardown() {
    local rc=$? keep=${LAB_KEEP:-}
    # an interrupt (an operator's second TERM or Ctrl-C, or the spot watch's
    # racing a run that ended on its own) must not cut the delete short, nor
    # its verification (as azure/lab.sh's teardown)
    trap '' INT TERM
    exec 2>&9
    [[ -n ${follow:-} ]] && kill "$follow" 2>/dev/null
    # the watch first: the teardown's own terminations are no reclaim
    [[ -n ${watch:-} ]] && kill "$watch" 2>/dev/null
    # a spot run that failed before the watch's next look (an SSH step to a
    # host that went, say), or as its instance never ran: was it reclaimed?
    if ((created)) && [[ $spot == 1 && -n ${pair:-} && ! -s $out/instance-gone ]]; then spot_gone || true; fi
    # EX_TEMPFAIL: nothing wrong with the run itself; try it again
    if grep -qs 'reclaimed by AWS' "$out/instance-gone"; then
        rc=75
        log "run $stamp did not finish: a spot instance was reclaimed; exit 75 (EX_TEMPFAIL): run it again"
        # the pair is broken: the other instance would bill to LAB_MAX_HOURS
        # for nothing, while a retry starts another pair
        if [[ -n $keep ]]; then
            log "LAB_KEEP: not kept: a spot instance was reclaimed, so the pair is broken"
            keep=
        fi
    fi
    if ((created)) && [[ -z $keep ]]; then
        teardown_run
    elif ((created)); then
        log "LAB_KEEP: run $stamp kept; delete it with: LAB_DELETE=$stamp $self"
    fi
    log "results: $out"
    # last: the snapshot this ran from
    remove_snapshot
    exit "$rc"
}
trap teardown EXIT
trap 'exit 130' INT TERM

key=
for k in ~/.ssh/id_ed25519.pub ~/.ssh/id_rsa.pub; do [[ -f $k ]] && { key=$k; break; }; done
[[ -n $key ]] || die "no ~/.ssh/id_ed25519.pub or ~/.ssh/id_rsa.pub"
# LAB_SSH_FROM: this machine's public addresses (space-separated), for a
# connection that fails over between links; default the current one
me=${LAB_SSH_FROM:-$(curl -fsS https://api.ipify.org)} || die "cannot find this machine's public address"
me=$(printf '%s\n' $me | sort -u | tr '\n' ' ') # a repeated address is an error to Azure

# the availability zones of <region> (default $region) that offer a type
offered() { ec2r "${2:-$region}" describe-instance-type-offerings --location-type availability-zone \
    --filters "Name=instance-type,Values=$1" --query 'InstanceTypeOfferings[].Location' --output text | tr '\t' '\n' | sort; }
# an Arm run: AWS's word that both types are arm64, the family's g being a
# naming convention, not a guarantee
if [[ $arch == arm64 ]]; then
    for t in $(printf '%s\n' "$type_a" "$type_b" | sort -u); do
        archs=$(ec2 describe-instance-types --instance-types "$t" \
            --query 'InstanceTypes[0].ProcessorInfo.SupportedArchitectures' --output text) || die "cannot describe $t"
        [[ " ${archs//[[:space:]]/ } " == *" arm64 "* ]] || die "$t is not arm64 (AWS: ${archs:-nothing}), yet named like a Graviton type"
    done
fi
# image <region>: Debian 13's latest AMI there, and its root device, which the
# disk below resizes, as "<ami> <device>"
image() {
    local ami root_device=/dev/xvda
    if [[ $arch == arm64 ]]; then
        read -r ami root_device < <(ec2r "$1" describe-images --owners 136693071363 \
            --filters "Name=name,Values=debian-13-arm64-*" "Name=architecture,Values=arm64" \
            --query 'sort_by(Images,&CreationDate)[-1].[ImageId,RootDeviceName]' --output text)
        [[ -n ${root_device:-} && $root_device != None ]] || root_device=/dev/xvda
    else
        ami=$(ec2r "$1" describe-images --owners 136693071363 --filters "Name=name,Values=debian-13-amd64-*" "Name=architecture,Values=x86_64" \
            --query 'sort_by(Images,&CreationDate)[-1].ImageId' --output text)
    fi
    [[ -n ${ami:-} && $ami != None ]] || return 1
    echo "$ami $root_device"
}
# LAB_THREADS_PER_CORE=1: each type's every core (AWS's default count), run
# at one thread each
cores_a= cores_b=
if [[ $threads_per_core == 1 ]]; then
    cores_a=$(ec2r "$region" describe-instance-types --instance-types "$type_a" --query 'InstanceTypes[0].VCpuInfo.DefaultCores' --output text)
    cores_b=$(ec2r "${region2:-$region}" describe-instance-types --instance-types "$type_b" --query 'InstanceTypes[0].VCpuInfo.DefaultCores' --output text)
    [[ $cores_a =~ ^[1-9][0-9]*$ && $cores_b =~ ^[1-9][0-9]*$ ]] || die "cannot read the core counts of $type_a and $type_b (AWS: $cores_a, $cores_b)"
fi
case $placement in
cluster | precision-time)
    zones=$(comm -12 <(offered "$type_a") <(offered "$type_b"))
    [[ -n $zones ]] || die "$type_a and $type_b are not both offered in one zone of $region"
    read -r ami root_device < <(image "$region") || die "no Debian 13 AMI in $region"
    ;;
cross-az)
    # a in one zone, b in another: every pair of zones that offer the types,
    # in order (LAB_AZS: that pair only, each offering its type)
    zones_a=$(offered "$type_a") zones_b=$(offered "$type_b")
    zone_pairs=()
    if [[ -n $azs ]]; then
        read -r za zb <<<"$azs"
        grep -qx "$za" <<<"$zones_a" || die "LAB_AZS: $za does not offer $type_a (it is offered in $(echo $zones_a))"
        grep -qx "$zb" <<<"$zones_b" || die "LAB_AZS: $zb does not offer $type_b (it is offered in $(echo $zones_b))"
        zone_pairs=("$za $zb")
    else
        for za in $zones_a; do
            for zb in $zones_b; do [[ $za == "$zb" ]] || zone_pairs+=("$za $zb"); done
        done
    fi
    ((${#zone_pairs[@]})) || die "no two zones of $region offer $type_a and $type_b"
    read -r ami root_device < <(image "$region") || die "no Debian 13 AMI in $region"
    ;;
cross-region)
    zones_a=$(offered "$type_a") zones_b=$(offered "$type_b" "$region2")
    [[ -n $zones_a ]] || die "$type_a is not offered in $region"
    [[ -n $zones_b ]] || die "$type_b is not offered in $region2"
    read -r ami root_device < <(image "$region") || die "no Debian 13 AMI in $region"
    read -r ami_b root_device_b < <(image "$region2") || die "no Debian 13 AMI in $region2"
    ;;
esac
log "run $stamp: a $type_a, b $type_b in $region${region2:+ (b in $region2)}, $placement (AMI $ami${ami_b:+, b $ami_b}$([[ $arch == arm64 ]] && echo ", arm64")), SSH from $me"

created=1
echo "$regions" >"$out/regions"
echo "$market" >"$out/market"
# make_network <region> <VPC CIDR>: the VPC, its internet gateway, a route
# table with the default route through it, and a security group with SSH from
# this machine, as net_vpc, net_igw, net_rt and net_sg
make_network() {
    local r=$1 ip
    net_vpc=$(ec2r "$r" create-vpc --cidr-block "$2" --tag-specifications "$(tags vpc vpc)" --query Vpc.VpcId --output text) || die "vpc"
    net_igw=$(ec2r "$r" create-internet-gateway --tag-specifications "$(tags internet-gateway igw)" \
        --query InternetGateway.InternetGatewayId --output text) || die "igw"
    ec2r "$r" attach-internet-gateway --internet-gateway-id "$net_igw" --vpc-id "$net_vpc" || die "igw attach"
    net_rt=$(ec2r "$r" create-route-table --vpc-id "$net_vpc" --tag-specifications "$(tags route-table rt)" --query RouteTable.RouteTableId --output text) || die "rt"
    ec2r "$r" create-route --route-table-id "$net_rt" --destination-cidr-block 0.0.0.0/0 --gateway-id "$net_igw" >/dev/null || die "route"
    net_sg=$(ec2r "$r" create-security-group --vpc-id "$net_vpc" --group-name "ergon-md-dpdk-$stamp" --description "ergon md-dpdk bench $stamp" \
        --tag-specifications "$(tags security-group sg)" --query GroupId --output text) || die "sg"
    for ip in $me; do
        ec2r "$r" authorize-security-group-ingress --group-id "$net_sg" --protocol tcp --port 22 --cidr "$ip/32" >/dev/null || die "sg ssh"
    done
}
# a's VPC (cross-region: b's is 10.31.0.0/16, in its region)
make_network "$region" 10.30.0.0/16
vpc=$net_vpc igw=$net_igw rt=$net_rt sg=$net_sg
sg_b=
if [[ $placement == cross-region ]]; then
    make_network "$region2" 10.31.0.0/16
    vpc_b=$net_vpc rt_b=$net_rt sg_b=$net_sg
fi
# Follow this machine's public address: a connection that fails over between
# links gets a new one every few minutes, and SSH must keep working
follow_address() {
    local now known_ips=" $me "
    while sleep 20; do
        now=$(curl -fsS -m 5 https://api.ipify.org 2>/dev/null) || continue
        [[ $known_ips == *" $now "* ]] && continue
        known_ips="$known_ips$now "
        ec2 authorize-security-group-ingress --group-id "$sg" --protocol tcp --port 22 --cidr "$now/32" >/dev/null &&
            log "SSH now also allowed from $now"
        if [[ -n $sg_b ]]; then
            ec2r "$region2" authorize-security-group-ingress --group-id "$sg_b" --protocol tcp --port 22 --cidr "$now/32" >/dev/null
        fi
    done
}
follow_address & follow=$!
ec2 authorize-security-group-ingress --group-id "$sg" --protocol -1 --cidr 10.30.0.0/16 >/dev/null || die "sg vpc"
ec2 import-key-pair --key-name "ergon-md-dpdk-$stamp" --public-key-material "fileb://$key" \
    --tag-specifications "$(tags key-pair key)" >/dev/null || die "key pair"
if [[ $placement == cross-region ]]; then
    # Across regions a security group cannot name another region's group: each
    # takes everything from the other VPC's CIDR. b's own key pair too.
    ec2r "$region2" authorize-security-group-ingress --group-id "$sg_b" --protocol -1 --cidr 10.31.0.0/16 >/dev/null || die "sg vpc b"
    ec2 authorize-security-group-ingress --group-id "$sg" --protocol -1 --cidr 10.31.0.0/16 >/dev/null || die "sg peer"
    ec2r "$region2" authorize-security-group-ingress --group-id "$sg_b" --protocol -1 --cidr 10.30.0.0/16 >/dev/null || die "sg peer b"
    ec2r "$region2" import-key-pair --key-name "ergon-md-dpdk-$stamp" --public-key-material "fileb://$key" \
        --tag-specifications "$(tags key-pair key)" >/dev/null || die "key pair b"
    # Inter-region VPC peering: requested from a's region, accepted in b's
    # once it shows there, tagged in b's too (so LAB_DELETE finds it from
    # either), then a route to the other VPC on each side
    pcx=$(ec2 create-vpc-peering-connection --vpc-id "$vpc" --peer-vpc-id "$vpc_b" --peer-region "$region2" \
        --tag-specifications "$(tags vpc-peering-connection pcx)" \
        --query VpcPeeringConnection.VpcPeeringConnectionId --output text) || die "peering"
    status=
    for _ in $(seq 30); do
        status=$(ec2r "$region2" describe-vpc-peering-connections --vpc-peering-connection-ids "$pcx" \
            --query 'VpcPeeringConnections[0].Status.Code' --output text 2>/dev/null) && [[ $status == pending-acceptance ]] && break
        sleep 5
    done
    [[ $status == pending-acceptance ]] || die "peering $pcx: not ready to accept in $region2 (${status:-not listed})"
    ec2r "$region2" accept-vpc-peering-connection --vpc-peering-connection-id "$pcx" >/dev/null || die "peering accept"
    ec2r "$region2" create-tags --resources "$pcx" --tags Key=owner,Value=ergon-lab "Key=run,Value=$stamp" \
        Key=Name,Value=ergon-md-dpdk-pcx || die "peering tags"
    for _ in $(seq 30); do
        status=$(ec2 describe-vpc-peering-connections --vpc-peering-connection-ids "$pcx" \
            --query 'VpcPeeringConnections[0].Status.Code' --output text 2>/dev/null) && [[ $status == active ]] && break
        sleep 5
    done
    [[ $status == active ]] || die "peering $pcx: not active (${status:-not listed})"
    ec2 create-route --route-table-id "$rt" --destination-cidr-block 10.31.0.0/16 --vpc-peering-connection-id "$pcx" >/dev/null ||
        die "route to b"
    ec2r "$region2" create-route --route-table-id "$rt_b" --destination-cidr-block 10.30.0.0/16 --vpc-peering-connection-id "$pcx" >/dev/null ||
        die "route to a"
fi
parent=()
if [[ $placement == precision-time ]]; then
    # PHC-equipped hosts (precision-time) as the parent, the cluster group
    # inside it: NIC timestamps without giving up cluster placement
    pt=$(ec2 create-placement-group --group-name "ergon-md-dpdk-pt-$stamp" --strategy precision-time \
        --tag-specifications "$(tags placement-group pg-pt)" --query PlacementGroup.GroupId --output text) ||
        die "precision-time placement group"
    parent=(--parent-group-id "$pt")
fi
if [[ $placement == cluster || $placement == precision-time ]]; then
    ec2 create-placement-group --group-name "ergon-md-dpdk-$stamp" --strategy cluster "${parent[@]}" \
        --tag-specifications "$(tags placement-group pg)" >/dev/null || die "placement group"
fi

# The dead-man's switch, as user data: the instance powers off at the
# deadline, at whichever boot reaches it (the isolated state reboots), and,
# launched to terminate on shutdown, is deleted with its disks and data ENIs.
# Only if this machine never deletes it first.
deadline_userdata() {
    [[ -f $out/deadline-userdata.sh ]] || cat >"$out/deadline-userdata.sh" <<USERDATA
#!/bin/bash
echo $(($(date +%s) + max_hours * 3600)) >/etc/ergon-lab-deadline
cat >/usr/local/sbin/ergon-lab-deadline <<'SCRIPT'
#!/bin/sh
left=\$((\$(cat /etc/ergon-lab-deadline) - \$(date +%s)))
[ "\$left" -le 60 ] && exec /sbin/poweroff
exec /sbin/shutdown -h "+\$((left / 60))" "ergon-lab deadline"
SCRIPT
chmod +x /usr/local/sbin/ergon-lab-deadline
cat >/etc/systemd/system/ergon-lab-deadline.service <<'UNIT'
[Unit]
Description=ergon-lab dead-man's switch: power off (and so terminate) at the lab's deadline
[Service]
Type=oneshot
ExecStart=/usr/local/sbin/ergon-lab-deadline
[Install]
WantedBy=multi-user.target
UNIT
systemctl daemon-reload
systemctl enable --now ergon-lab-deadline.service
USERDATA
}
# launch <region> <type> <AMI> <root device> <mgmt subnet> <security group>
# <zone> [<run-instances option>...]: one instance, its id (its error in
# run-instances-<zone>.log)
launch() {
    local r=$1 t=$2 image_id=$3 root=$4 subnet=$5 group=$6 zone=$7 cpu=() market_opts=() spot_tags=()
    shift 7
    if [[ $threads_per_core == 1 ]]; then
        cpu=(--cpu-options "CoreCount=$([[ $t == "$type_a" && $r == "$region" ]] && echo "$cores_a" || echo "$cores_b"),ThreadsPerCore=1")
    fi
    # LAB_SPOT=1: a one-time spot instance, terminated on interruption (as on
    # instance-initiated shutdown, the deadline's), its spot request tagged
    if [[ $spot == 1 ]]; then
        market_opts=(--instance-market-options 'MarketType=spot,SpotOptions={SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}')
        spot_tags=("$(tags spot-instances-request spot)")
    fi
    ec2r "$r" run-instances --image-id "$image_id" --instance-type "$t" --count 1 --key-name "ergon-md-dpdk-$stamp" \
        --instance-initiated-shutdown-behavior terminate --user-data "file://$out/deadline-userdata.sh" \
        --subnet-id "$subnet" --security-group-ids "$group" "$@" \
        --block-device-mappings "DeviceName=$root,Ebs={VolumeSize=40,VolumeType=gp3,DeleteOnTermination=true}" \
        --metadata-options HttpTokens=required ${cpu[@]+"${cpu[@]}"} ${market_opts[@]+"${market_opts[@]}"} \
        --tag-specifications "$(tags instance node)" "$(tags volume disk)" "$(tags network-interface mgmt-eni)" ${spot_tags[@]+"${spot_tags[@]}"} \
        --query 'Instances[0].InstanceId' --output text 2>>"$out/run-instances-$zone.log"
}
# subnet <region> <VPC> <CIDR> <zone> <name>: a subnet, its id
subnet() {
    ec2r "$1" create-subnet --vpc-id "$2" --cidr-block "$3" --availability-zone "$4" \
        --tag-specifications "$(tags subnet "$5")" --query Subnet.SubnetId --output text
}
# mgmt_subnet <region> <VPC> <CIDR> <zone> <route table>: the management
# subnet: public addresses, the route table's internet route
mgmt_subnet() {
    local id
    id=$(subnet "$1" "$2" "$3" "$4" mgmt) || return 1
    ec2r "$1" modify-subnet-attribute --subnet-id "$id" --map-public-ip-on-launch
    ec2r "$1" associate-route-table --route-table-id "$5" --subnet-id "$id" >/dev/null
    echo "$id"
}
# give_back <region> <instance id>: a lone instance terminated, polled until
# it is (never holding the CLI lock across a wait)
give_back() {
    ec2r "$1" terminate-instances --instance-ids "$2" >/dev/null
    wait_state "$1" terminated "$2" || log "WARNING: $2 not yet terminated (teardown deletes it)"
}

deadline_userdata
launched=
case $placement in
cluster | precision-time)
    # Launch both instances in one zone, trying each zone that offers the type
    # until one has the capacity.
    for zone in $zones; do
        mgmt=$(mgmt_subnet "$region" "$vpc" 10.30.0.0/24 "$zone" "$rt") || die "mgmt subnet"
        data=$(subnet "$region" "$vpc" "$data_subnet" "$zone" data) || die "data subnet"
        if ida=$(launch "$region" "$type_a" "$ami" "$root_device" "$mgmt" "$sg" "$zone" --placement "GroupName=ergon-md-dpdk-$stamp"); then
            if idb=$(launch "$region" "$type_b" "$ami" "$root_device" "$mgmt" "$sg" "$zone" --placement "GroupName=ergon-md-dpdk-$stamp"); then
                launched=$zone
                break
            fi
            # a alone in this zone: give it back before trying the next
            give_back "$region" "$ida"
        fi
        log "no capacity for $type_a and $type_b in $zone: $(tail -1 "$out/run-instances-$zone.log")"
        ec2 delete-subnet --subnet-id "$mgmt"
        ec2 delete-subnet --subnet-id "$data"
    done
    [[ -n $launched ]] || die "no zone had capacity for $type_a and $type_b"
    data_a=$data data_b=$data
    data_subnet_a=$data_subnet data_subnet_b=$data_subnet
    ;;
cross-az)
    # a's subnets in its zone (10.30.0.0/24, data 10.30.1.0/24), b's in its
    # own (10.30.2.0/24, data 10.30.3.0/24): b's data ENIs are off a's data
    # subnet, reached through the subnet's gateway. Each pair of zones in turn
    # until both have the capacity, no placement group.
    data_subnet_a=10.30.1.0/24 data_subnet_b=10.30.3.0/24
    for pair in "${zone_pairs[@]}"; do
        read -r za zb <<<"$pair"
        mgmt_a=$(mgmt_subnet "$region" "$vpc" 10.30.0.0/24 "$za" "$rt") || die "mgmt subnet"
        data_a=$(subnet "$region" "$vpc" "$data_subnet_a" "$za" data) || die "data subnet"
        mgmt_b=$(mgmt_subnet "$region" "$vpc" 10.30.2.0/24 "$zb" "$rt") || die "mgmt subnet b"
        data_b=$(subnet "$region" "$vpc" "$data_subnet_b" "$zb" data) || die "data subnet b"
        if ida=$(launch "$region" "$type_a" "$ami" "$root_device" "$mgmt_a" "$sg" "$za"); then
            if idb=$(launch "$region" "$type_b" "$ami" "$root_device" "$mgmt_b" "$sg" "$zb"); then
                launched="a $za b $zb"
                break
            fi
            give_back "$region" "$ida"
            log "no capacity for $type_b in $zb: $(tail -1 "$out/run-instances-$zb.log")"
        else
            log "no capacity for $type_a in $za: $(tail -1 "$out/run-instances-$za.log")"
        fi
        for id in "$mgmt_a" "$data_a" "$mgmt_b" "$data_b"; do ec2 delete-subnet --subnet-id "$id"; done
    done
    [[ -n $launched ]] || die "no pair of zones had capacity for $type_a and $type_b"
    ;;
cross-region)
    # each host in the first zone of its region with the capacity, its data
    # subnet on the route table too (the route to the other VPC)
    data_subnet_a=10.30.1.0/24 data_subnet_b=10.31.1.0/24
    # place <region> <VPC> <route table> <sg> <type> <AMI> <root device>
    # <mgmt CIDR> <data CIDR> <zones>: "<zone> <instance> <data subnet>"
    place() {
        local r=$1 v=$2 table=$3 group=$4 t=$5 image_id=$6 root=$7 zone m d id
        for zone in ${10}; do
            m=$(mgmt_subnet "$r" "$v" "$8" "$zone" "$table") || return 1
            d=$(subnet "$r" "$v" "$9" "$zone" data) || return 1
            ec2r "$r" associate-route-table --route-table-id "$table" --subnet-id "$d" >/dev/null || return 1
            if id=$(launch "$r" "$t" "$image_id" "$root" "$m" "$group" "$zone"); then
                echo "$zone $id $d"
                return 0
            fi
            log "no capacity for $t in $zone: $(tail -1 "$out/run-instances-$zone.log")"
            ec2r "$r" delete-subnet --subnet-id "$m"
            ec2r "$r" delete-subnet --subnet-id "$d"
        done
        return 1
    }
    read -r za ida data_a < <(place "$region" "$vpc" "$rt" "$sg" "$type_a" "$ami" "$root_device" 10.30.0.0/24 "$data_subnet_a" "$zones_a") ||
        die "no zone of $region had capacity for $type_a"
    read -r zb idb data_b < <(place "$region2" "$vpc_b" "$rt_b" "$sg_b" "$type_b" "$ami_b" "$root_device_b" 10.31.0.0/24 "$data_subnet_b" "$zones_b") ||
        die "no zone of $region2 had capacity for $type_b"
    [[ -n ${ida:-} && -n ${idb:-} ]] || die "instances not launched"
    launched="a $za b $zb"
    ;;
esac
region_b=${region2:-$region}
# the run's two instances, from here on (spot_gone: either one reclaimed)
pair=1
log "instances $ida $idb in $launched ($market); waiting for them to run"
wait_state "$region" running "$ida" && wait_state "$region_b" running "$idb" || die "instances did not start"
# spot: a look at both every 30 s while they run (spot_watch)
if [[ $spot == 1 ]]; then
    spot_watch &
    watch=$!
fi

# two data ENIs each, attached at device indexes 1 and 2, deleted with the instance
attach() { # region instance device-index ip data-subnet security-group
    local eni att
    eni=$(ec2r "$1" create-network-interface --subnet-id "$5" --groups "$6" --private-ip-address "$4" \
        --tag-specifications "$(tags network-interface data-eni)" --query NetworkInterface.NetworkInterfaceId --output text) || return 1
    att=$(ec2r "$1" attach-network-interface --network-interface-id "$eni" --instance-id "$2" --device-index "$3" \
        --query AttachmentId --output text) || return 1
    ec2r "$1" modify-network-interface-attribute --network-interface-id "$eni" \
        --attachment "AttachmentId=$att,DeleteOnTermination=true" || return 1
}
# a's at .4 and .5 of its data subnet, b's at .6 and .7 of its own
net_a=${data_subnet_a%.*/*} net_b=${data_subnet_b%.*/*}
attach "$region" "$ida" 1 "$net_a.4" "$data_a" "$sg" && attach "$region" "$ida" 2 "$net_a.5" "$data_a" "$sg" &&
    attach "$region_b" "$idb" 1 "$net_b.6" "$data_b" "${sg_b:-$sg}" && attach "$region_b" "$idb" 2 "$net_b.7" "$data_b" "${sg_b:-$sg}" ||
    die "data ENIs"
A=$(ec2 describe-instances --instance-ids "$ida" --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)
B=$(ec2r "$region_b" describe-instances --instance-ids "$idb" --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)
a_private=$(ec2 describe-instances --instance-ids "$ida" --query 'Reservations[0].Instances[0].PrivateIpAddress' --output text)
b_private=$(ec2r "$region_b" describe-instances --instance-ids "$idb" --query 'Reservations[0].Instances[0].PrivateIpAddress' --output text)
log "a=$A b=$B"
{
    echo "type a $type_a b $type_b"
    echo "arch $arch"
    echo "zone $launched"
    echo "ami $ami${ami_b:+ b $ami_b}"
    echo "placement $placement"
    echo "market $market"
    [[ -n $region2 ]] && echo "region a $region b $region2"
    [[ $threads_per_core == 1 ]] && echo "threads per core 1 (cores a $cores_a b $cores_b)"
} >"$out/instances.txt"

. "$here/../lib.sh"
prepare_vms
run_bench
