#!/usr/bin/env bash
# The NRI latency-pod extensions on Amazon EKS AL2023: a cluster and two
# SPOT Graviton nodes that exist only while this script runs, a round trip
# between them under seven latency-pod modes (M0 baseline, M1 hugepages,
# M2 isolated partition, M3 seccomp +io_uring, M4 PTP device, M5 liveness
# probe harm, M6 all-on), then everything deleted and the
# deletion verified. Runs on macOS or Linux with the aws CLI signed in
# (`aws login`), kubectl, docker (buildx) and python3; no eksctl or helm,
# no SSH key, no bastion, no NAT gateway.
#
#   md-dpdk/benches/cloud/aws/eks-nri.sh                          2 x c8g.xlarge SPOT
#   EKS_CHECK_ONLY=1 md-dpdk/benches/cloud/aws/eks-nri.sh         print the cost estimate and stop
#   EKS_DELETE=eks-nri-<stamp> md-dpdk/benches/cloud/aws/eks-nri.sh   delete a run
#
# Why AWS and not Azure: AKS has no supported containerd change (the NRI
# plugin cannot be installed) and no reservedSystemCPUs. Everything here
# needs one of those. EKS AL2023 ships containerd 2.x with NRI on by
# default, so the plugin path exists. SPOT c8g.xlarge (4 vCPU Graviton, no
# SMT) is the minimum shape that still works: 2 reserved CPUs (0 = system
# and the shared pool, 1 = NIC IRQs) plus 2 isolated CPUs for the latency
# pod. Cost gate, before anything is created: the estimated hourly rate
# must be at most $0.30/h and EKS_MAX_MINUTES at most 180; the estimate is
# printed first. Spot is tried first, across every enabled region. If no
# region can place 2 spot instances of a 4-vCPU no-SMT Graviton, the script
# tries c9g.xlarge and then c7g.xlarge (same pin map: CPU 0 shared, 1 IRQs,
# 2–3 isolated) and only then on-demand, up to EKS_ONDEMAND_MAX_HOURLY.
# On-demand is logged. It is never a silent fleet setting.
#
# Then, in $EKS_REGION, everything tagged owner=ergon-lab and run=<stamp>
# and named ergon-lab-<stamp>: two IAM roles and an instance profile; a VPC
# (10.31.0.0/16) with an internet gateway and public subnets only, no NAT;
# two ECR repositories (shared-cpus plugin, workload); the EKS cluster
# (control-plane logging off, API access entries, the caller admin); a launch
# template whose user data is eks-nri-userdata.sh's (kernel command line with
# hugepages and the isolated CPUs, kubelet with the static CPU manager and
# reservedSystemCPUs, NRI's default validator, one reboot before kubelet);
# and 2 x $EKS_TYPE through one EC2 Fleet request per zone
# (price-capacity-optimized or capacity-optimized). Spot requests set
# OnDemandTargetCapacity=0. On-demand is a separate request, used only after
# spot could not be placed.
#
# It deploys the shared-cpus DaemonSet + ConfigMap (sharedCpus 0, opt-in
# annotations, namespace ergon-latency-pods), the latency namespace's validating
# admission policy (no liveness probes, no exec readiness probes, no
# sidecars), and a privileged probe DaemonSet for host commands. It records
# the node facts the run depends on (containerd config dump, runc, kernel,
# NUMA, /dev/ptp*, RT_GROUP_SCHED, rmem_max), then runs $EKS_REPS round
# trips per mode (rtt xping on a, xpong on b, HARNESS_DRIVER=invoker) and
# prints p50/p99/p99.9/p99.99 and MAX with the loss counters and each max's
# attributed cause. Results also go to benchmarks.md.
#
# Everything is deleted on any exit (EXIT, INT, TERM, HUP; a watchdog sends
# TERM after EKS_MAX_MINUTES), in dependency order, and verified by tag and
# by name for what carries no tag.
#
#   EKS_REGION (us-east-1, tried first; every enabled region is searched for spot)
#   EKS_TYPE (c8g.xlarge; Graviton, 4 vCPU, no SMT)
#   EKS_K8S (1.36) EKS_REPS (3; 5 if the clock allows) EKS_MAX_MINUTES (120; the hard cap is 180)
#   EKS_ONDEMAND_MAX_HOURLY (0.55; ceiling for the on-demand fallback only)
#   EKS_SPOT_STRATEGY (price-capacity-optimized; capacity-optimized works too)
#   EKS_MODES (M0 M1 M2 M3 M4 M5 M6) EKS_HUGEPAGES (256 2 MiB pages a node)
#   EKS_CHECK_ONLY=1 stops after the estimate; EKS_DELETE=<stamp> deletes.
#   RTT_MESSAGES (300000) RTT_WARMUP (50000) RTT_SECONDS (60)
#   EKS_PHASE_X (1: before the modes, phase X: the EKS-lane checks of
#   k8s/nri-shared-cpus/RELEASE.md, placement evidence, a helper's wake-up wait
#   on the shared CPU idle and under load with 1 ns and 50 us timer slack, the
#   pin's cost, and the SQPOLL placements; 0 skips it) X_SECONDS (20 a cell)
#   EKS_HOLD_MINUTES (0): after the report, keep the cluster up to that many
#   minutes (inside EKS_MAX_MINUTES) for work by hand; `touch <out>/release`
#   ends the hold and deletes everything. EKS_HOLD_ON_FAIL (0): the same hold
#   when the run fails, before the teardown, to look at a live failure.
#
# Modes (latency-build-plan.md §9). SCHED_FIFO is deliberately absent: the one
# AWS A/B was inconclusive (io/benches/BENCHMARK.md). Rlimits (MEMLOCK) ride along
# on the hugepage modes so the app can mlock the pages.
#   M0 baseline   AERON_DIR on a plain emptyDir; no extension annotations
#   M1 +hugepages hugepages.ergon.io/app=2M + hugepages-2Mi request; the
#                 plugin mounts a sized hugetlbfs at /dev/hugepages
#   M2 +partition cpu-partition.ergon.io/app=isolated (cpuset.cpus.partition)
#   M3 +seccomp   seccomp.ergon.io/app=io-uring (RuntimeDefault + io_uring_*)
#   M4 +devices   ptp.ergon.io/app=auto (the NIC PHC; cpuqos is x86-only)
#   M5 +probe     M0 plus a livenessProbe.exec every 1s (the harm)
#   M6 all-on     M1+M2+M3+M4 together, no probe
set -uo pipefail
export AWS_PAGER=""

here=$(cd "$(dirname "$0")" && pwd)
repo=${EKS_REPO:-$(cd "$here/../../../.." && pwd)}
# Run from a snapshot of these scripts: bash reads a script as it runs, so an
# edit to the checkout mid-run would otherwise break this run (and its teardown).
if [[ -z ${EKS_SNAPSHOT:-} ]]; then
    snapshot=$(mktemp -d)
    # the quiet-CPU unit the user data installs (absent only in a fake repo)
    { cp -R "$here/.." "$snapshot/cloud" && mkdir -p "$snapshot/quiet" &&
        { [[ ! -d $repo/samples/low-latency-k8s/node ]] || cp -R "$repo/samples/low-latency-k8s/node/." "$snapshot/quiet/"; }; } ||
        { echo "cannot snapshot the scripts into $snapshot" >&2; exit 1; }
    EKS_SNAPSHOT=$snapshot EKS_REPO=$repo exec bash "$snapshot/cloud/aws/eks-nri.sh" "$@"
fi
scripts=$EKS_SNAPSHOT/cloud/aws
benches=$EKS_SNAPSHOT/cloud
self=$repo/md-dpdk/benches/cloud/aws/eks-nri.sh # for messages: the checkout's path
region=${EKS_REGION:-us-east-1}
home_region=$region
type=${EKS_TYPE:-c8g.xlarge}
preferred_type=$type
k8s=${EKS_K8S:-1.36}
reps=${EKS_REPS:-3}
max_minutes=${EKS_MAX_MINUTES:-120}
ondemand_cap=${EKS_ONDEMAND_MAX_HOURLY:-0.55}
spot_strategy=${EKS_SPOT_STRATEGY:-price-capacity-optimized}
market=spot
prefer_zone=
lt_version=1
modes=${EKS_MODES:-M0 M1 M2 M3 M4 M5 M6}
hugepages=${EKS_HUGEPAGES:-256}
check_only=${EKS_CHECK_ONLY:-}
delete=${EKS_DELETE:-}
rtt_messages=${RTT_MESSAGES:-300000}
rtt_warmup=${RTT_WARMUP:-50000}
rtt_seconds=${RTT_SECONDS:-60}
phase_x=${EKS_PHASE_X:-1}
x_seconds=${X_SECONDS:-20}
hold_minutes=${EKS_HOLD_MINUTES:-0}
hold_on_fail=${EKS_HOLD_ON_FAIL:-0}

# A unique stamp: eks-nri-YYYYmmdd-HHMMSS, never an eks.sh one (both tag
# owner=ergon-lab, run=<stamp>), so EKS_DELETE can only select this lab's runs.
stamp=${delete:-eks-nri-$(date +%Y%m%d-%H%M%S)}
[[ $stamp =~ ^eks-nri-[0-9]{8}-[0-9]{6}$ ]] || { echo "EKS_DELETE=$stamp: an eks-nri.sh stamp (eks-nri-YYYYmmdd-HHMMSS)" >&2; exit 2; }
N=ergon-lab-$stamp
out=$repo/target/md-dpdk-bench/$stamp
results=$out/results.csv

# Cost gates, before anything exists. 180 minutes is the hard cap the
# harness set; $0.30/h keeps the run inside the lab's spot budget.
if ((max_minutes > 180)); then
    echo "EKS_MAX_MINUTES=$max_minutes: the cap is 180 minutes (cost discipline)" >&2
    exit 2
fi
if [[ ! $ondemand_cap =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
    echo "EKS_ONDEMAND_MAX_HOURLY=$ondemand_cap: a USD amount" >&2
    exit 2
fi
if ((reps < 0)); then
    echo "EKS_REPS=$reps: a count" >&2
    exit 2
fi

# The node layout of a 4-vCPU no-SMT node. The measuring loop takes the first
# isolated CPU, the gap spinner the second.
reserved="0,1"   # 0 = system + the shared pool, 1 = NIC IRQs
isolated="2,3"   # the latency pod's exclusive CPUs
shared_cpus="0"  # the plugin's sharedCpus, inside reserved minus the IRQ CPU
irq_cpu=1
# One exclusive CPU a pod: 4 CPUs less 2 reserved leave 2000m allocatable, and
# the node's own pods (aws-node, kube-proxy, the plugin) request part of it,
# so a cpu: 2 Guaranteed pod never schedules (run eks-nri-20261025-082023:
# "Insufficient cpu"). The gap spinner then has no CPU of its own and is off.
pod_cpus=${EKS_POD_CPUS:-1}

cluster_role=$N-cluster
node_role=$N-node
profile=$N-node
lt=$N-node
created=0
collected=
watchdog_pid=

log() { echo "[$(date +%T)] $*" >&2; }
die() { log "ERROR: $*"; exit 1; }
ec2() { aws ec2 --region "$region" "$@"; }
eks() { aws eks --region "$region" "$@"; }
ecr() { aws ecr --region "$region" "$@"; }
tags() { echo "ResourceType=$1,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$stamp},{Key=Name,Value=$N-$2}]"; }
tagged() { echo "Name=tag:run,Values=$stamp"; }
sha() { if command -v sha256sum >/dev/null; then sha256sum; else shasum -a 256; fi | cut -c1-12; }
k() { kubectl --request-timeout=60s "$@"; }
kx() { kubectl exec "$@"; }

wait_for() {
    local limit=$1 interval=$2 what=$3 deadline
    shift 3
    deadline=$((SECONDS + limit))
    until "$@"; do
        ((SECONDS >= deadline)) && { log "timed out after ${limit}s waiting for $what"; return 1; }
        sleep "$interval"
    done
}

# Per-node values, a and b.
nv() { local v="$1_$2"; echo "${!v:-}"; }
nv_set() { printf -v "$1_$2" '%s' "$3"; }

for t in aws kubectl docker python3 helm; do command -v "$t" >/dev/null || die "$t is not on PATH"; done
ident=$(aws sts get-caller-identity --query '[Account,Arn]' --output text 2>/dev/null) || die "aws is not signed in (aws login)"
read -r account caller <<<"$ident"
registry=$account.dkr.ecr.$region.amazonaws.com
mkdir -p "$out"

# ---- teardown ----------------------------------------------------------------

cluster_status() {
    local s
    if s=$(eks describe-cluster --name "$N" --query cluster.status --output text 2>&1); then
        echo "$s"
    elif [[ $s == *ResourceNotFoundException* ]]; then
        echo NONE
    else
        echo UNKNOWN
    fi
}
cluster_not_creating() { [[ $(cluster_status) != CREATING ]]; }
cluster_gone() { [[ $(cluster_status) == NONE ]]; }
delete_cluster_once() {
    case $(cluster_status) in
    NONE | DELETING) return 0 ;;
    esac
    eks delete-cluster --name "$N" >/dev/null 2>&1
}
instances_of_run() {
    ec2 describe-instances --filters "$(tagged)" "Name=instance-state-name,Values=pending,running,shutting-down,stopping,stopped" \
        --query 'Reservations[].Instances[].InstanceId' --output text
}
instances_gone() { local ids; ids=$(instances_of_run 2>/dev/null) || return 1; [[ -z $ids ]]; }
run_vpc() { local v; v=$(ec2 describe-vpcs --filters "$(tagged)" --query 'Vpcs[0].VpcId' --output text 2>/dev/null); [[ $v == None ]] || echo "$v"; }

vpc_delete_pass() {
    local vpc=$1 id res
    for id in $(ec2 describe-internet-gateways --filters "Name=attachment.vpc-id,Values=$vpc" --query 'InternetGateways[].InternetGatewayId' --output text 2>/dev/null); do
        ec2 detach-internet-gateway --internet-gateway-id "$id" --vpc-id "$vpc" 2>/dev/null
        ec2 delete-internet-gateway --internet-gateway-id "$id" 2>/dev/null
    done
    for id in $(ec2 describe-subnets --filters "Name=vpc-id,Values=$vpc" --query 'Subnets[].SubnetId' --output text 2>/dev/null); do
        ec2 delete-subnet --subnet-id "$id" 2>/dev/null
    done
    for id in $(ec2 describe-route-tables --filters "Name=vpc-id,Values=$vpc" \
        --query 'RouteTables[].[RouteTableId,length(Associations[?Main])]' --output text 2>/dev/null | awk '$2 == 0 {print $1}'); do
        ec2 delete-route-table --route-table-id "$id" 2>/dev/null
    done
    for id in $(ec2 describe-security-groups --filters "Name=vpc-id,Values=$vpc" \
        --query 'SecurityGroups[?GroupName!=`default`].GroupId' --output text 2>/dev/null); do
        ec2 delete-security-group --group-id "$id" 2>/dev/null
    done
    if res=$(ec2 delete-vpc --vpc-id "$vpc" 2>&1); then
        log "deleted VPC $vpc"
        return 0
    fi
    [[ $res == *InvalidVpcID.NotFound* ]] && return 0
    log "VPC $vpc not deleted yet: ${res##*: }"
    return 1
}

iam_delete_role() {
    local role=$1 id
    aws iam get-role --role-name "$role" >/dev/null 2>&1 || return 0
    for id in $(aws iam list-instance-profiles-for-role --role-name "$role" --query 'InstanceProfiles[].InstanceProfileName' --output text); do
        aws iam remove-role-from-instance-profile --instance-profile-name "$id" --role-name "$role"
    done
    for id in $(aws iam list-attached-role-policies --role-name "$role" --query 'AttachedPolicies[].PolicyArn' --output text); do
        aws iam detach-role-policy --role-name "$role" --policy-arn "$id"
    done
    for id in $(aws iam list-role-policies --role-name "$role" --query 'PolicyNames' --output text); do
        aws iam delete-role-policy --role-name "$role" --policy-name "$id"
    done
    aws iam delete-role --role-name "$role" && log "deleted IAM role $role"
}

iam_left() {
    local res
    if [[ $1 == role ]]; then
        res=$(aws iam get-role --role-name "$2" 2>&1) && { echo "$2"; return 0; }
    else
        res=$(aws iam get-instance-profile --instance-profile-name "$2" 2>&1) && { echo "$2"; return 0; }
    fi
    [[ $res == *NoSuchEntity* ]] && return 0
    echo "$res" >&2
    return 1
}
oidc_left() {
    local all id
    id=$(cat "$out/oidc-id" 2>/dev/null)
    [[ -n $id && $id != - && $id != None ]] || return 0
    all=$(aws iam list-open-id-connect-providers --query 'OpenIDConnectProviderList[].Arn' --output text) || return 1
    tr '\t' '\n' <<<"$all" | grep -F "/id/$id" || true
}
cluster_left() { eks list-clusters --query "clusters[?@=='$N']" --output text; }
repos_left() { ecr describe-repositories --query "repositories[?starts_with(repositoryName, '$N/')].repositoryName" --output text; }
fleets_left() {
    # describe-fleets does not list an instant fleet unless asked by id, and a
    # tag filter therefore reports none while the fleet is still active.
    local id state
    if [[ -f $out/fleet-ids ]]; then
        while read -r id; do
            [[ -z $id ]] && continue
            state=$(ec2 describe-fleets --fleet-ids "$id" --query 'Fleets[0].FleetState' --output text 2>/dev/null || true)
            case $state in
                # deleted_terminating is a record whose instances are going: the
                # instances check above covers them, and the record bills nothing
                active|deleted_running) printf '%s\n' "$id" ;;
            esac
        done <"$out/fleet-ids"
    fi
}
log_groups_left() {
    aws logs describe-log-groups --region "$region" --log-group-name-prefix "/aws/eks/$N/" --query 'logGroups[].logGroupName' --output text
}

left=
check() {
    local what=$1 found rc
    shift
    found=$("$@" 2>&1)
    rc=$?
    found=$(tr -s ' \t\n' '   ' <<<"$found")
    found=${found# }
    found=${found% }
    if ((rc != 0)); then
        log "  ?    $what: could not check (${found:0:200})"
        left+=" $what:unchecked"
    elif [[ -n $found && $found != None ]]; then
        log "  LEFT $what: $found"
        left+=" $what:$found"
    else
        log "  none $what"
    fi
}

verify_run() {
    local vpc
    left=
    log "verifying run $stamp in $region:"
    vpc=$(run_vpc)
    check "instances (tag run)" instances_of_run
    check "volumes (tag run)" ec2 describe-volumes --filters "$(tagged)" --query 'Volumes[].VolumeId' --output text
    check "elastic IPs (tag run)" ec2 describe-addresses --filters "$(tagged)" --query 'Addresses[].AllocationId' --output text
    check "ENIs (tag run)" ec2 describe-network-interfaces --filters "$(tagged)" --query 'NetworkInterfaces[].NetworkInterfaceId' --output text
    check "ENIs (VPC CNI tag cluster.k8s.amazonaws.com/name=$N)" ec2 describe-network-interfaces \
        --filters "Name=tag:cluster.k8s.amazonaws.com/name,Values=$N" --query 'NetworkInterfaces[].NetworkInterfaceId' --output text
    check "ENIs (EKS: Amazon EKS $N)" ec2 describe-network-interfaces \
        --filters "Name=description,Values=Amazon EKS $N" --query 'NetworkInterfaces[].NetworkInterfaceId' --output text
    [[ -n $vpc ]] && check "ENIs (in $vpc)" ec2 describe-network-interfaces \
        --filters "Name=vpc-id,Values=$vpc" --query 'NetworkInterfaces[].NetworkInterfaceId' --output text
    check "security groups (tag aws:eks:cluster-name=$N)" ec2 describe-security-groups \
        --filters "Name=tag:aws:eks:cluster-name,Values=$N" --query 'SecurityGroups[].GroupId' --output text
    check "security groups (tag run)" ec2 describe-security-groups --filters "$(tagged)" --query 'SecurityGroups[].GroupId' --output text
    check "subnets (tag run)" ec2 describe-subnets --filters "$(tagged)" --query 'Subnets[].SubnetId' --output text
    check "route tables (tag run)" ec2 describe-route-tables --filters "$(tagged)" --query 'RouteTables[].RouteTableId' --output text
    check "internet gateways (tag run)" ec2 describe-internet-gateways --filters "$(tagged)" --query 'InternetGateways[].InternetGatewayId' --output text
    check "VPCs (tag run)" ec2 describe-vpcs --filters "$(tagged)" --query 'Vpcs[].VpcId' --output text
    check "placement groups (tag run)" ec2 describe-placement-groups --filters "$(tagged)" --query 'PlacementGroups[].GroupName' --output text
    check "launch templates (tag run)" ec2 describe-launch-templates --filters "$(tagged)" --query 'LaunchTemplates[].LaunchTemplateName' --output text
    check "EC2 fleets (tag run)" fleets_left
    check "EKS cluster $N" cluster_left
    check "ECR repositories $N/*" repos_left
    check "IAM role $cluster_role" iam_left role "$cluster_role"
    check "IAM role $node_role" iam_left role "$node_role"
    check "IAM instance profile $profile" iam_left instance-profile "$profile"
    check "IAM OIDC provider for $N" oidc_left
    check "CloudWatch log groups /aws/eks/$N/" log_groups_left
    local idx
    idx=$(aws resourcegroupstaggingapi get-resources --region "$region" --tag-filters "Key=run,Values=$stamp" \
        --query 'ResourceTagMappingList[].ResourceARN' --output text 2>/dev/null)
    [[ -n $idx ]] && log "  info: the tagging index still lists (it lags deletions): $(tr -s ' \t\n' '   ' <<<"$idx")"
    if [[ -z $left ]]; then
        log "verified: nothing of run $stamp remains in $region (the free service-linked role AWSServiceRoleForAmazonEKS may stay)"
        return 0
    fi
    log "WARNING: still present for run $stamp:$left (rerun: EKS_DELETE=$stamp $self)"
    return 1
}

# Best-effort delete of a probe that ran in a region other than $region.
# The real teardown below still owns the region the cluster was created in.
sweep_region_resources() {
    local ids id
    ids=$(instances_of_run 2>/dev/null || true)
    if [[ -n ${ids// /} ]]; then
        ec2 terminate-instances --instance-ids $ids >/dev/null 2>&1 || true
        wait_for 180 10 "probe instances in $region to terminate" instances_gone || true
    fi
    for id in $(fleets_left 2>/dev/null); do
        ec2 delete-fleets --fleet-ids "$id" --terminate-instances >/dev/null 2>&1 || true
    done
    for id in $(ec2 describe-launch-templates --filters "$(tagged)" --query 'LaunchTemplates[].LaunchTemplateId' --output text 2>/dev/null); do
        [[ -z $id || $id == None ]] && continue
        ec2 delete-launch-template --launch-template-id "$id" >/dev/null 2>&1 || true
    done
    for id in $(ec2 describe-vpcs --filters "$(tagged)" --query 'Vpcs[].VpcId' --output text 2>/dev/null); do
        [[ -z $id || $id == None ]] && continue
        vpc_delete_pass "$id" || true
    done
}
teardown_run() {
    log "deleting run $stamp ($N) in $region"
    if [[ -f $out/probe-regions.txt ]]; then
        local keep=$region r
        while read -r r; do
            [[ -z $r || $r == "$keep" ]] && continue
            region=$r
            log "deleting probe leftovers tagged run=$stamp in $r"
            sweep_region_resources
        done <"$out/probe-regions.txt"
        region=$keep
    fi
    local ids id vpc deadline res
    local tries
    for tries in 1 2 3 4 5 6; do
        if ids=$(instances_of_run 2>/dev/null); then break; fi
        ids=
        log "listing run $stamp's instances failed (try $tries); retrying"
        sleep 10
    done
    if [[ -n $ids ]]; then
        ec2 terminate-instances --instance-ids $ids >/dev/null ||
            log "WARNING: terminate-instances $ids failed; waiting anyway, verify_run reports what is left"
        wait_for 900 15 "instances $ids to terminate" instances_gone
    fi
    # fleets: instant fleets should already be gone; anything left is deleted
    for id in $(fleets_left 2>/dev/null); do
        ec2 delete-fleets --fleet-ids "$id" --terminate-instances >/dev/null 2>&1 && log "deleted fleet $id"
    done
    if [[ $(cluster_status) == CREATING ]]; then
        log "cluster $N is still creating: waiting for it to finish"
        wait_for 1200 30 "cluster $N to finish creating" cluster_not_creating
    fi
    if [[ $(cluster_status) != NONE ]]; then
        if [[ ! -s $out/oidc-id ]]; then
            mkdir -p "$out"
            id=$(eks describe-cluster --name "$N" --query 'cluster.identity.oidc.issuer' --output text 2>/dev/null)
            [[ -n $id && $id != None ]] && echo "${id##*/}" >"$out/oidc-id"
        fi
        wait_for 300 30 "delete-cluster $N to be accepted" delete_cluster_once
        wait_for 1500 30 "cluster $N to be deleted" cluster_gone && log "deleted cluster $N"
    fi
    vpc=$(run_vpc)
    for id in $(ec2 describe-network-interfaces --filters "$(tagged)" "Name=status,Values=available" \
        --query 'NetworkInterfaces[].NetworkInterfaceId' --output text 2>/dev/null); do
        ec2 delete-network-interface --network-interface-id "$id" 2>/dev/null && log "deleted ENI $id"
    done
    if [[ -n $vpc ]]; then
        deadline=$((SECONDS + 600))
        while :; do
            res=$(ec2 describe-network-interfaces --filters "Name=vpc-id,Values=$vpc" \
                --query 'NetworkInterfaces[].[NetworkInterfaceId,Status]' --output text 2>/dev/null)
            [[ -z $res ]] && break
            for id in $(awk '$2 == "available" {print $1}' <<<"$res"); do
                ec2 delete-network-interface --network-interface-id "$id" 2>/dev/null && log "deleted ENI $id"
            done
            ((SECONDS >= deadline)) && { log "ENIs still in $vpc after 10 minutes: $(tr -s ' \t\n' '   ' <<<"$res")"; break; }
            sleep 20
        done
    fi
    for id in $repos; do
        ecr describe-repositories --repository-names "$id" >/dev/null 2>&1 &&
            ecr delete-repository --repository-name "$id" --force >/dev/null && log "deleted ECR repository $id"
    done
    docker logout "$registry" >/dev/null 2>&1
    if aws iam get-instance-profile --instance-profile-name "$profile" >/dev/null 2>&1; then
        aws iam remove-role-from-instance-profile --instance-profile-name "$profile" --role-name "$node_role" 2>/dev/null
        aws iam delete-instance-profile --instance-profile-name "$profile" && log "deleted instance profile $profile"
    fi
    iam_delete_role "$node_role"
    iam_delete_role "$cluster_role"
    for id in $(ec2 describe-launch-templates --filters "$(tagged)" --query 'LaunchTemplates[].LaunchTemplateId' --output text 2>/dev/null); do
        ec2 delete-launch-template --launch-template-id "$id" >/dev/null && log "deleted launch template $id"
    done
    for vpc in $(ec2 describe-vpcs --filters "$(tagged)" --query 'Vpcs[].VpcId' --output text 2>/dev/null); do
        [[ -z $vpc || $vpc == None ]] && continue
        deadline=$((SECONDS + 600))
        until vpc_delete_pass "$vpc"; do
            ((SECONDS >= deadline)) && { log "VPC $vpc not deleted after 10 minutes"; break; }
            sleep 30
        done
    done
    verify_run
}

if [[ -n $delete ]]; then
    repos="$N/shared-cpus $N/workload"
    teardown_run
    exit $?
fi
repos="$N/shared-cpus $N/workload"

kill_tree() {
    local c
    for c in $(pgrep -P "$1"); do kill_tree "$c"; done
    kill -KILL "$1" 2>/dev/null
    return 0
}
bounded() {
    local secs=$1 pid killer rc
    shift
    "$@" <&0 &
    pid=$!
    { sleep "$secs" && kill_tree "$pid"; } 2>/dev/null &
    killer=$!
    wait "$pid"
    rc=$?
    kill_tree "$killer"
    wait "$killer" 2>/dev/null
    return "$rc"
}

# Keep the cluster for work by hand until <out>/release exists, the minutes
# pass, or EKS_MAX_MINUTES (less a 10-minute margin for the teardown) is near.
hold() {
    local minutes=$1 why=$2 until_s cap_s
    ((minutes > 0)) || return 0
    until_s=$(($(date +%s) + minutes * 60))
    cap_s=$((run_t0 + max_minutes * 60 - 600))
    ((until_s > cap_s)) && until_s=$cap_s
    {
        echo "HOLD ($why) until epoch $until_s ($(((until_s - $(date +%s)) / 60)) min) or touch $out/release"
        echo "KUBECONFIG=$out/kubeconfig"
    } | tee "$out/HOLD" >&2
    while (($(date +%s) < until_s)) && [[ ! -e $out/release ]]; do sleep 10; done
    rm -f "$out/HOLD"
    log "hold over"
}
run_t0=$(date +%s)

teardown() {
    local rc=$?
    trap '' INT TERM HUP
    if ((rc != 0 && created && hold_on_fail > 0)) && [[ ! -e $out/watchdog-fired && -s $out/kubeconfig ]]; then
        hold "$hold_on_fail" "run failed, rc $rc"
    fi
    [[ -n $watchdog_pid ]] && kill "$watchdog_pid" 2>/dev/null
    jobs -p | xargs kill 2>/dev/null
    if ((created)); then
        if ! teardown_run && ((rc == 0)); then rc=1; fi
    fi
    log "results: $out"
    exit "$rc"
}

# ---- phase 0: free, before anything is created --------------------------------

log "run $stamp: EKS cluster $N, 2 x $type SPOT, Kubernetes $k8s, $region"
docker info >/dev/null 2>&1 || die "docker is not running"
docker buildx version >/dev/null 2>&1 || die "docker buildx is not installed"

read -r vcpus cores tpc arch <<<"$(ec2 describe-instance-types --instance-types "$type" --query \
    'InstanceTypes[0].[VCpuInfo.DefaultVCpus,VCpuInfo.DefaultCores,VCpuInfo.DefaultThreadsPerCore,ProcessorInfo.SupportedArchitectures[0]]' \
    --output text 2>/dev/null)"
[[ -n ${arch:-} && $arch != None ]] || die "cannot describe $type in $region"
[[ $arch == arm64 ]] || die "$type is $arch: this lab runs on Graviton (minimum spec, cheap spot)"
((vcpus >= 4)) || die "$type has $vcpus vCPUs; the node needs 4 (2 reserved + 2 isolated)"
if ((tpc > 1)); then
    log "WARNING: $type has SMT (threads-per-core $tpc); c8g has none. One Kubernetes CPU is not one physical core here."
fi
[[ $k8s =~ ^1\.([0-9]+)$ ]] && ((BASH_REMATCH[1] >= 36)) || die "EKS_K8S $k8s: want Kubernetes 1.36 or later"

# Leftovers anywhere: what this account has billing under ergon-lab before
# this run adds to it. An ergon-lab EKS cluster refuses (an earlier run whose
# teardown never ran).
sweep_leftovers() {
    local regions r tmp found
    regions=$(ec2 describe-regions --query 'Regions[].RegionName' --output text) || die "cannot list the regions"
    tmp=$(mktemp -d)
    for r in $regions; do
        (
            aws ec2 --region "$r" describe-instances --filters Name=tag:owner,Values=ergon-lab \
                Name=instance-state-name,Values=pending,running,stopping,stopped \
                --query 'Reservations[].Instances[].[InstanceId,InstanceType,State.Name]' --output text | sed "s/^/$r instance /"
            aws ec2 --region "$r" describe-volumes --filters Name=tag:owner,Values=ergon-lab \
                --query 'Volumes[].[VolumeId,State]' --output text | sed "s/^/$r volume /"
            aws ec2 --region "$r" describe-addresses --filters Name=tag:owner,Values=ergon-lab \
                --query 'Addresses[].PublicIp' --output text | tr '\t' '\n' | sed '/^$/d' | sed "s/^/$r elastic-ip /"
            aws ec2 --region "$r" describe-nat-gateways --filter Name=state,Values=pending,available \
                --query 'NatGateways[].NatGatewayId' --output text | tr '\t' '\n' | sed '/^$/d' | sed "s/^/$r nat-gateway /"
            aws eks --region "$r" list-clusters --query 'clusters[?starts_with(@, `ergon-lab-`)]' --output text |
                tr '\t' '\n' | sed '/^$/d' | sed "s/^/$r eks-cluster /"
        ) >"$tmp/$r" 2>/dev/null &
    done
    wait
    {
        cat "$tmp"/*
        ec2 describe-vpcs --filters Name=tag:owner,Values=ergon-lab --query 'Vpcs[].[VpcId,Tags[?Key==`run`]|[0].Value]' \
            --output text 2>/dev/null | sed "s/^/$region vpc (free) /"
        ecr describe-repositories --query 'repositories[?starts_with(repositoryName, `ergon-lab-`)].repositoryName' \
            --output text 2>/dev/null | tr '\t' '\n' | sed '/^$/d' | sed "s/^/$region ecr /"
        aws iam list-roles --query 'Roles[?starts_with(RoleName, `ergon-lab-`)].RoleName' --output text 2>/dev/null |
            tr '\t' '\n' | sed '/^$/d' | sed 's/^/global iam-role (free) /'
    } >"$out/leftovers.txt"
    rm -rf "$tmp"
    found=$(grep -c . "$out/leftovers.txt")
    if ((found)); then
        log "already in this account (ergon-lab tags, ergon-lab-* names, any NAT gateway):"
        sed 's/^/    /' "$out/leftovers.txt" >&2
    else
        log "no ergon-lab leftovers in any region"
    fi
    # EKS_ALLOW_CLUSTERS: names of other live (or still deleting) runs' clusters to accept
    local c others=
    for c in $(awk '$2 == "eks-cluster" {print $3}' "$out/leftovers.txt"); do
        [[ " ${EKS_ALLOW_CLUSTERS:-} " == *" $c "* ]] && { log "  $c: accepted (EKS_ALLOW_CLUSTERS)"; continue; }
        others+=" $c"
    done
    [[ -z $others ]] || die "an ergon-lab EKS cluster exists (above:$others): delete it first with EKS_DELETE=<its stamp> $self"
}
sweep_leftovers

ami=$(aws ssm get-parameter --region "$region" \
    --name "/aws/service/eks/optimized-ami/$k8s/amazon-linux-2023/arm64/standard/recommended/image_id" \
    --query Parameter.Value --output text 2>/dev/null) || die "no EKS AL2023 arm64 AMI for Kubernetes $k8s in $region"

# What this costs. On-demand Linux from the price list (the floor: spot can
# never be above it for long); spot from the last hour's price history, the
# highest of it (price-protected: the run must fit the gate at the worst price
# it is walking into). EKS control plane $0.10/h, public IPv4 $0.005/h each,
# gp3 20 GB about $0.004/h each: ~$0.02 on top. On-demand fallback is OFF, so
# the spot price is the only node price that can be charged.
price() {
    aws pricing get-products --region us-east-1 --service-code AmazonEC2 --filters \
        "Type=TERM_MATCH,Field=instanceType,Value=$1" "Type=TERM_MATCH,Field=regionCode,Value=$region" \
        "Type=TERM_MATCH,Field=operatingSystem,Value=Linux" "Type=TERM_MATCH,Field=tenancy,Value=Shared" \
        "Type=TERM_MATCH,Field=preInstalledSw,Value=NA" "Type=TERM_MATCH,Field=capacitystatus,Value=Used" \
        "Type=TERM_MATCH,Field=marketoption,Value=OnDemand" \
        --query 'PriceList[0]' --output text 2>/dev/null | python3 -c '
import json, sys
try:
    for term in json.loads(sys.stdin.read())["terms"]["OnDemand"].values():
        for dim in term["priceDimensions"].values():
            print(dim["pricePerUnit"]["USD"])
            sys.exit()
except Exception:
    pass
print("?")'
}
spot_price() {
    ec2 describe-spot-price-history --instance-types "$1" --product-descriptions "Linux/UNIX" \
        --start-time "$(python3 -c 'import datetime;print((datetime.datetime.utcnow()-datetime.timedelta(hours=1)).isoformat())')" \
        --query 'SpotPriceHistory[].SpotPrice' --output text 2>/dev/null |
        tr '\t' '\n' | grep -E '^[0-9.]+$' | sort -g | tail -1
}
# Newest price per zone, cheapest zone first. The fleet launches in one zone,
# so the gate looks at that zone's price, not the dearest zone in the region.
zone_spots() {
    aws ec2 --region "$region" describe-spot-price-history --instance-types "$1" \
        --product-descriptions "Linux/UNIX" \
        --start-time "$(python3 -c 'import datetime;print((datetime.datetime.utcnow()-datetime.timedelta(hours=1)).isoformat())')" \
        --query 'SpotPriceHistory[].[AvailabilityZone,SpotPrice]' --output text 2>/dev/null |
        python3 -c '
import sys
seen = {}
for line in sys.stdin:
    parts = line.split()
    if len(parts) != 2 or parts[0] in seen:
        continue
    try:
        seen[parts[0]] = float(parts[1])
    except ValueError:
        pass
for zone, price in sorted(seen.items(), key=lambda kv: kv[1]):
    print("%s %.6f" % (zone, price))
'
}
ami_for() {
    aws ssm get-parameter --region "$1" \
        --name "/aws/service/eks/optimized-ami/$k8s/amazon-linux-2023/arm64/standard/recommended/image_id" \
        --query Parameter.Value --output text 2>/dev/null
}
# The benchmark pins CPU 0 (shared pool), 1 (NIC IRQs) and 2–3 (the latency
# pod). That map is the same on every 4-vCPU Graviton with no SMT. A bigger
# box or an SMT box would change it, so those types are not fallbacks.
layout_ok() {
    local t=$1 r=$2 line v c tp a
    line=$(aws ec2 --region "$r" describe-instance-types --instance-types "$t" \
        --query 'InstanceTypes[0].[VCpuInfo.DefaultVCpus,VCpuInfo.DefaultCores,VCpuInfo.DefaultThreadsPerCore,ProcessorInfo.SupportedArchitectures[0]]' \
        --output text 2>/dev/null) || return 1
    read -r v c tp a <<<"$line"
    [[ $a == arm64 && $tp == 1 && $v == 4 ]]
}
fallback_shape_ok() {
    layout_ok "$1" "$home_region" && return 0
    [[ $home_region == us-east-1 ]] && return 1
    layout_ok "$1" us-east-1
}
offered_zones() {
    aws ec2 --region "$region" describe-instance-type-offerings --location-type availability-zone \
        --filters "Name=instance-type,Values=$1" \
        --query 'InstanceTypeOfferings[].Location' --output text 2>/dev/null | tr '\t' '\n' | sed '/^$/d'
}
# Prints the on-demand hourly total (2 nodes + $0.12) or returns non-zero when
# it is above EKS_ONDEMAND_MAX_HOURLY or the price is missing.
ondemand_hourly() {
    local od
    od=$(price "$1")
    python3 -c 'import sys
try:
    od = float(sys.argv[1])
except ValueError:
    sys.exit(2)
hourly = 2 * od + 0.12
print("%.3f" % hourly)
sys.exit(0 if hourly <= float(sys.argv[2]) else 1)
' "$od" "$ondemand_cap"
}
# True when the cheapest zone of the preferred type in the current region
# fits the spot gate. The estimate above uses the dearest zone; one expensive
# zone must not hide a zone the fleet can actually buy.
cheapest_zone_ok() {
    local line price
    line=$(zone_spots "$preferred_type" | head -1)
    [[ -z $line ]] && return 1
    price=${line#* }
    python3 -c 'import sys; sys.exit(0 if 2*float(sys.argv[1])+0.12 <= 0.30 else 1)' "$price"
}
# True when some enabled region other than $home_region has a spot zone
# inside the $0.30/h gate. The fake tests have one region.
affordable_elsewhere() {
    local r price home=$region line
    while read -r r; do
        [[ -z $r || $r == "$home" ]] && continue
        region=$r
        line=$(zone_spots "$preferred_type" | head -1)
        region=$home
        [[ -z $line ]] && continue
        price=${line#* }
        python3 -c 'import sys; sys.exit(0 if 2*float(sys.argv[1])+0.12 <= 0.30 else 1)' "$price" && return 0
    done < <(aws ec2 --region "$home" describe-regions --query 'Regions[].RegionName' --output text | tr '\t' '\n')
    region=$home
    return 1
}
# Delete one capacity probe. Returns non-zero if the VPC is still there.
probe_clean() {
    local r=$1 vpc=$2 subnet=$3 lt=$4 fleet=$5
    shift 5
    local ids="$*" i eni left
    if [[ -n ${ids// /} ]]; then
        aws ec2 --region "$r" terminate-instances --instance-ids $ids >/dev/null 2>>"$out/probe.log" || true
    fi
    if [[ -n $fleet ]]; then
        aws ec2 --region "$r" delete-fleets --fleet-ids "$fleet" --terminate-instances >/dev/null 2>>"$out/probe.log" || true
    fi
    if [[ -n ${ids// /} ]]; then
        i=0
        while ((i < 36)); do
            left=$(aws ec2 --region "$r" describe-instances --instance-ids $ids \
                --query 'Reservations[].Instances[?State.Name!=`terminated`].InstanceId' --output text 2>/dev/null || true)
            [[ -z $left || $left == None ]] && break
            sleep 10
            i=$((i + 1))
        done
    fi
    # A terminated instance keeps its ENI "in use" for a minute or more:
    # the subnet and the VPC wait for that (up to 3 and 2 minutes).
    if [[ -n $subnet ]]; then
        i=0
        while ((i < 36)); do
            for eni in $(aws ec2 --region "$r" describe-network-interfaces --filters "Name=subnet-id,Values=$subnet" \
                --query 'NetworkInterfaces[].NetworkInterfaceId' --output text 2>/dev/null); do
                [[ -z $eni || $eni == None ]] && continue
                aws ec2 --region "$r" delete-network-interface --network-interface-id "$eni" >/dev/null 2>>"$out/probe.log" || true
            done
            aws ec2 --region "$r" delete-subnet --subnet-id "$subnet" >/dev/null 2>>"$out/probe.log" && break
            sleep 5
            i=$((i + 1))
        done
    fi
    if [[ -n $vpc ]]; then
        i=0
        while ((i < 24)); do
            aws ec2 --region "$r" delete-vpc --vpc-id "$vpc" >/dev/null 2>>"$out/probe.log" && break
            sleep 5
            i=$((i + 1))
        done
        if aws ec2 --region "$r" describe-vpcs --vpc-ids "$vpc" --query 'Vpcs[0].VpcId' --output text 2>/dev/null | grep -q '^vpc-'; then
            log "ERROR: probe VPC $vpc in $r is still present"
            return 1
        fi
    fi
    if [[ -n $lt ]]; then
        aws ec2 --region "$r" delete-launch-template --launch-template-name "$lt" >/dev/null 2>>"$out/probe.log" || true
    fi
    return 0
}
# Launch 2 instances and delete them. Returns 0 only when the fleet actually
# got both. A probe that cannot be deleted stops the run so the trap can
# sweep the tagged leftovers.
probe_fleet() {
    local how=$1 r=$2 t=$3 zone=$4 pami=$5
    local pvpc="" psub="" plt="" pfleet="" pids="" rc=1 clean_rc=0 target fj
    plt="$N-probe-$RANDOM"
    echo "$r" >>"$out/probe-regions.txt"
    pvpc=$(aws ec2 --region "$r" create-vpc --cidr-block 10.42.0.0/16 \
        --tag-specifications "$(tags vpc probe)" --query Vpc.VpcId --output text 2>>"$out/probe.log") || pvpc=""
    if [[ -n $pvpc && $pvpc != None ]]; then
        psub=$(aws ec2 --region "$r" create-subnet --vpc-id "$pvpc" --cidr-block 10.42.1.0/24 \
            --availability-zone "$zone" --tag-specifications "$(tags subnet probe)" \
            --query Subnet.SubnetId --output text 2>>"$out/probe.log") || psub=""
    fi
    if [[ -n $psub && $psub != None ]]; then
        python3 - "$pami" "$t" "$stamp" "$N" >"$out/probe-lt.json" <<'PY'
import json, sys
ami, itype, stamp, name = sys.argv[1:5]
def tag(role):
    return {"ResourceType": role, "Tags": [
        {"Key": "owner", "Value": "ergon-lab"},
        {"Key": "run", "Value": stamp},
        {"Key": "Name", "Value": "%s-probe-%s" % (name, role)},
    ]}
print(json.dumps({
    "ImageId": ami,
    "InstanceType": itype,
    "BlockDeviceMappings": [{
        "DeviceName": "/dev/xvda",
        "Ebs": {"VolumeSize": 20, "VolumeType": "gp3", "DeleteOnTermination": True},
    }],
    "NetworkInterfaces": [{"DeviceIndex": 0, "AssociatePublicIpAddress": False}],
    "TagSpecifications": [tag("instance"), tag("volume"), tag("network-interface")],
    "MetadataOptions": {"HttpTokens": "required", "HttpEndpoint": "enabled"},
    "InstanceInitiatedShutdownBehavior": "terminate",
}))
PY
        if aws ec2 --region "$r" create-launch-template --launch-template-name "$plt" \
            --launch-template-data "file://$out/probe-lt.json" \
            --tag-specifications "$(tags launch-template probe)" >/dev/null 2>>"$out/probe.log"; then
            fj=$out/probe-fleet-$r-$zone.json
            if [[ $how == on-demand ]]; then
                target="TotalTargetCapacity=2,DefaultTargetCapacityType=on-demand,OnDemandTargetCapacity=2"
                aws ec2 --region "$r" create-fleet --type instant \
                    --target-capacity-specification "$target" \
                    --launch-template-config "LaunchTemplateSpecification={LaunchTemplateName=$plt,Version=1},Overrides=[{SubnetId=$psub}]" \
                    --tag-specifications "$(tags fleet probe)" \
                    --output json >"$fj" 2>>"$out/probe.log" || true
            else
                target="TotalTargetCapacity=2,DefaultTargetCapacityType=spot,OnDemandTargetCapacity=0"
                aws ec2 --region "$r" create-fleet --type instant \
                    --spot-options "AllocationStrategy=capacity-optimized" \
                    --target-capacity-specification "$target" \
                    --launch-template-config "LaunchTemplateSpecification={LaunchTemplateName=$plt,Version=1},Overrides=[{SubnetId=$psub}]" \
                    --tag-specifications "$(tags fleet probe)" \
                    --output json >"$fj" 2>>"$out/probe.log" || true
            fi
            if [[ -s $fj ]]; then
                read -r pfleet pids <<<"$(python3 -c '
import json, sys
f = json.load(open(sys.argv[1]))
ids = [i for inst in f.get("Instances", []) for i in inst.get("InstanceIds", [])]  # InstanceIds is a list of ids
print((f.get("FleetId") or "-") + " " + " ".join(ids))
' "$fj" 2>/dev/null || echo "-")"
                [[ $pfleet == "-" ]] && pfleet=""
                if [[ $(wc -w <<<"$pids") -eq 2 ]]; then rc=0; fi
            fi
        fi
    fi
    probe_clean "$r" "$pvpc" "$psub" "$plt" "$pfleet" $pids || clean_rc=1
    if ((clean_rc != 0)); then
        region=$r
        die "probe in $r left a VPC (see $out/probe.log). Teardown will try again."
    fi
    return $rc
}
# Pick region, type and market before the cluster exists. Spot c8g.xlarge
# first, then spot c9g.xlarge and c7g.xlarge when they keep the 4-core pin
# map, then on-demand of those types. The first probe that receives 2
# instances wins; the probe instances are deleted before this returns.
select_capacity() {
    local t r zone price ami_r hourly offered nzone types region_words line
    mkdir -p "$out"
    : >"$out/probe-regions.txt"
    region_words=$home_region
    while read -r r; do
        [[ -z $r || $r == "$home_region" ]] && continue
        region_words+=" $r"
    done < <(aws ec2 --region "$home_region" describe-regions --query 'Regions[].RegionName' --output text | tr '\t' '\n' | sort)
    types=$preferred_type
    for t in c9g.xlarge c7g.xlarge c8g.xlarge; do
        case " $types " in
        *" $t "*) ;;
        *) types+=" $t" ;;
        esac
    done
    log "looking for spot capacity for 2 instances. $home_region first, then every enabled region."
    for t in $types; do
        if [[ $t != "$preferred_type" ]] && ! fallback_shape_ok "$t"; then
            log "skip $t: not a 4-vCPU Graviton with no SMT. The pins stay CPU 0 shared, 1 IRQs, 2-3 isolated."
            continue
        fi
        for r in $region_words; do
            region=$r
            ami_r=$(ami_for "$r") || { log "no Kubernetes $k8s arm64 AMI in $r"; continue; }
            offered=$(offered_zones "$t" | tr '\n' ' ')
            [[ -n ${offered// /} ]] || continue
            while read -r zone price; do
                [[ -z ${zone:-} ]] && continue
                case " $offered " in
                *" $zone "*) ;;
                *) continue ;;
                esac
                python3 -c 'import sys; sys.exit(0 if 2*float(sys.argv[1])+0.12 <= 0.30 else 1)' "$price" || continue
                log "probing spot: 2 x $t in $zone ($r), last price \$$price/h"
                if probe_fleet spot "$r" "$t" "$zone" "$ami_r"; then
                    type=$t
                    market=spot
                    prefer_zone=$zone
                    ami=$ami_r
                    region=$r
                    log "spot capacity: 2 x $type in $prefer_zone ($region)"
                    return 0
                fi
                log "no spot capacity for 2 x $t in $zone ($r)"
            done < <(zone_spots "$t")
        done
    done
    log "no spot capacity under the \$0.30/h gate for 2 x $preferred_type, c9g.xlarge or c7g.xlarge in any enabled region"
    log "on-demand fallback, ceiling \$$ondemand_cap/h. c8g.xlarge first, so the documented machine is kept when it can be placed."
    for t in $types; do
        if [[ $t != "$preferred_type" ]] && ! fallback_shape_ok "$t"; then
            continue
        fi
        for r in $region_words; do
            region=$r
            ami_r=$(ami_for "$r") || continue
            hourly=$(ondemand_hourly "$t") || {
                log "on-demand 2 x $t in $r is above the \$$ondemand_cap/h ceiling, or has no price"
                continue
            }
            offered=$(offered_zones "$t" | tr '\n' ' ')
            nzone=0
            for zone in $offered; do
                nzone=$((nzone + 1))
                ((nzone > 3)) && break
                log "probing on-demand: 2 x $t in $zone ($r), estimated \$$hourly/h"
                if probe_fleet on-demand "$r" "$t" "$zone" "$ami_r"; then
                    type=$t
                    market=on-demand
                    prefer_zone=$zone
                    ami=$ami_r
                    region=$r
                    log "on-demand fallback: 2 x $type in $prefer_zone ($region), estimated \$$hourly/h"
                    return 0
                fi
                log "no on-demand capacity for 2 x $t in $zone ($r)"
            done
        done
    done
    region=$home_region
    die "no spot capacity in any enabled region, and on-demand could not place 2 instances of a 4-vCPU no-SMT Graviton under \$$ondemand_cap/h"
}
node_on_demand=$(price "$type")
node_spot=$(spot_price "$type")
estimate=$(python3 -c '
import sys
od, sp, gate = sys.argv[1], sys.argv[2], float(sys.argv[3])
def f(x):
    try:
        return float(x)
    except ValueError:
        return None
o, s = f(od), f(sp)
if s is not None:
    basis = f"spot ${s:.4f}/h (max of the last hour)"
    hourly = 2 * s + 0.12
elif o is not None:
    basis = f"no spot history; on-demand ${o:.4f}/h as the floor"
    hourly = 2 * o + 0.12
else:
    basis = "no price found"
    hourly = 1e9
print(f"{hourly:.3f} {basis}")
if hourly > gate:
    sys.exit(3)
' "$node_on_demand" "$node_spot" 0.30)
price_rc=$?
if ((price_rc == 3)); then
    if ! cheapest_zone_ok && ! affordable_elsewhere; then
        die "estimated hourly rate is above the \$0.30/h gate (2 x $type + the control plane). Spot is too expensive right now: try another region (EKS_REGION) or later. Refusing to start."
    fi
    log "a spot zone is inside the \$0.30/h gate even though the dearest zone is not. The run uses a zone inside the gate."
    if [[ -n $check_only ]]; then
        log "EKS_CHECK_ONLY=1: nothing created."
        exit 0
    fi
elif ((price_rc != 0)); then
    die "cannot price $type"
else
    read -r hourly basis <<<"$estimate"
    log "cost estimate BEFORE anything is created:"
    log "  2 x $type SPOT ($basis) + EKS control plane \$0.10/h + public IPv4 and gp3"
    log "  estimated \$$hourly/h, deleted at exit; at most EKS_MAX_MINUTES=$max_minutes minutes"
    log "  gate: at most \$0.30/h for spot and 180 minutes. No spot in any region falls through to c9g.xlarge and c7g.xlarge, then on-demand up to \$$ondemand_cap/h."
    if [[ -n $check_only ]]; then
        log "EKS_CHECK_ONLY=1: nothing created. Estimated \$$hourly/h."
        exit 0
    fi
fi

# Capacity before the cluster, so a region with no spot does not bill a
# control plane. The trap covers a probe that cannot delete itself.
trap teardown EXIT
trap 'exit 130' INT TERM
trap 'exit 129' HUP
created=1
select_capacity
registry=$account.dkr.ecr.$region.amazonaws.com
ami=$(ami_for "$region") || die "no EKS AL2023 arm64 AMI for Kubernetes $k8s in $region"
log "run $stamp: $market, 2 x $type, $region, zone $prefer_zone"

# ---- images ------------------------------------------------------------------

commit=$(git -C "$repo" rev-parse --short=12 HEAD) || die "$repo is not a git checkout"
dirty=$(cd "$repo" && { git diff HEAD; git ls-files -o --exclude-standard; git ls-files -o --exclude-standard -z | xargs -0 cat 2>/dev/null; } | sha)
tag=$commit-$dirty
git -C "$repo" rev-parse HEAD >"$out/commit"
git -C "$repo" status --porcelain >"$out/dirty"

plugin_src=$repo/k8s/nri-shared-cpus
[[ -f $plugin_src/Dockerfile ]] || die "no $plugin_src/Dockerfile (the shared-cpus NRI plugin)"
[[ -f $scripts/eks-nri-workload.Dockerfile ]] || die "no $scripts/eks-nri-workload.Dockerfile"
[[ -f $scripts/prelock.c && -f $scripts/io-uring-probe.c ]] || die "the prelock and io_uring_probe sources are missing from $scripts"

build_image() { # <local name> <context> <Dockerfile>
    local img=ergon-nri/$1:$tag
    if docker image inspect "$img" >/dev/null 2>&1; then
        log "image $img: built already"
        return 0
    fi
    log "building $img (linux/arm64), log $out/build-$1.log"
    docker buildx build --platform linux/arm64 --load -f "$3" -t "$img" "$2" \
        >"$out/build-$1.log" 2>&1 || { tail -30 "$out/build-$1.log" >&2; die "image $1 did not build"; }
}
# The plugin: its own Dockerfile (Go, static). The workload: the harness with
# static Aeron plus the small C tools. Both arm64; both pushed to ECR (the
# nodes have no registry other than that, and no SSH to load a tarball).
plugin_ctx=$out/plugin-src
mkdir -p "$plugin_ctx"
cp -R "$plugin_src"/. "$plugin_ctx"/
build_image shared-cpus "$plugin_ctx" "$plugin_ctx/Dockerfile"
build_image workload "$repo" "$scripts/eks-nri-workload.Dockerfile"

# ---- create ------------------------------------------------------------------

watchdog() {
    local t0 now next
    t0=$(date +%s)
    next=$((t0 + 900))
    while sleep 60; do
        now=$(date +%s)
        if ((now - t0 >= max_minutes * 60)); then
            log "watchdog: EKS_MAX_MINUTES=$max_minutes reached; stopping the run, which deletes it"
            touch "$out/watchdog-fired"
            kill -TERM $$ 2>/dev/null
            pkill -TERM -P $$ 2>/dev/null
            return
        fi
        if ((now >= next)); then
            next=$((next + 900))
            {
                echo "[$(date +%T)] watch: run $stamp, $(((now - t0) / 60)) minutes"
                ec2 describe-instances --filters "$(tagged)" --query 'Reservations[].Instances[].[InstanceId,InstanceType,State.Name]' --output text
                echo "cluster $N: $(cluster_status)"
                local v
                v=$(run_vpc)
                [[ -n $v ]] && echo "ENIs in $v: $(ec2 describe-network-interfaces --filters "Name=vpc-id,Values=$v" --query 'length(NetworkInterfaces)' --output text)"
            } 2>&1 | tee -a "$out/watch.log" >&2
        fi
    done
}
watchdog &
watchdog_pid=$!
created=1

iam_tags=(--tags Key=owner,Value=ergon-lab "Key=run,Value=$stamp")
trust() { printf '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"%s"},"Action":["sts:AssumeRole"%s]}]}' "$1" "${2:-}"; }
cluster_role_arn=$(aws iam create-role --role-name "$cluster_role" --assume-role-policy-document "$(trust eks.amazonaws.com ',"sts:TagSession"')" \
    "${iam_tags[@]}" --query Role.Arn --output text) || die "IAM role $cluster_role"
for p in arn:aws:iam::aws:policy/AmazonEKSClusterPolicy; do aws iam attach-role-policy --role-name "$cluster_role" --policy-arn "$p" || die "attach $p"; done
node_role_arn=$(aws iam create-role --role-name "$node_role" --assume-role-policy-document "$(trust ec2.amazonaws.com)" \
    "${iam_tags[@]}" --query Role.Arn --output text) || die "IAM role $node_role"
for p in arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryPullOnly arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy; do
    aws iam attach-role-policy --role-name "$node_role" --policy-arn "$p" || die "attach $p"
done
profile_arn=$(aws iam create-instance-profile --instance-profile-name "$profile" "${iam_tags[@]}" \
    --query InstanceProfile.Arn --output text) || die "instance profile $profile"
aws iam add-role-to-instance-profile --instance-profile-name "$profile" --role-name "$node_role" || die "instance profile role"

# Network: public subnets only, no NAT (the nodes reach ECR and the AMI's
# repositories through their public addresses). DNS on (EKS needs both).
vpc=$(ec2 create-vpc --cidr-block 10.31.0.0/16 --tag-specifications "$(tags vpc vpc)" --query Vpc.VpcId --output text) || die "vpc"
ec2 modify-vpc-attribute --vpc-id "$vpc" --enable-dns-support '{"Value":true}' || die "vpc dns support"
ec2 modify-vpc-attribute --vpc-id "$vpc" --enable-dns-hostnames '{"Value":true}' || die "vpc dns hostnames"
igw=$(ec2 create-internet-gateway --tag-specifications "$(tags internet-gateway igw)" \
    --query InternetGateway.InternetGatewayId --output text) || die "igw"
ec2 attach-internet-gateway --internet-gateway-id "$igw" --vpc-id "$vpc" || die "igw attach"
rt=$(ec2 create-route-table --vpc-id "$vpc" --tag-specifications "$(tags route-table rt)" --query RouteTable.RouteTableId --output text) || die "rt"
ec2 create-route --route-table-id "$rt" --destination-cidr-block 0.0.0.0/0 --gateway-id "$igw" >/dev/null || die "route"
sg=$(ec2 create-security-group --vpc-id "$vpc" --group-name "$N-nodes" --description "ergon nri EKS $stamp" \
    --tag-specifications "$(tags security-group sg)" --query GroupId --output text) || die "sg"
ec2 authorize-security-group-ingress --group-id "$sg" --protocol -1 --cidr 10.31.0.0/16 >/dev/null || die "sg vpc"
# the control plane's subnets: two zones, never use1-az3 (EKS refuses it)
cluster_zones=$(ec2 describe-availability-zones --filters Name=state,Values=available Name=zone-type,Values=availability-zone \
    --query 'AvailabilityZones[?ZoneId!=`use1-az3`].ZoneName' --output text | tr '\t' '\n' | sort | head -2)
(($(wc -w <<<"$cluster_zones") == 2)) || die "fewer than two usable zones in $region"
cluster_subnets=
i=2
for zone in $cluster_zones; do
    id=$(ec2 create-subnet --vpc-id "$vpc" --cidr-block "10.31.$i.0/24" --availability-zone "$zone" \
        --tag-specifications "$(tags subnet "cluster-$zone")" --query Subnet.SubnetId --output text) || die "cluster subnet $zone"
    ec2 associate-route-table --route-table-id "$rt" --subnet-id "$id" >/dev/null || die "cluster subnet route"
    cluster_subnets+="${cluster_subnets:+,}\"$id\""
    i=$((i + 1))
done
for r in $repos; do
    ecr create-repository --repository-name "$r" --tags Key=owner,Value=ergon-lab "Key=run,Value=$stamp" >/dev/null || die "ECR $r"
done

# Push the images before the cluster is created: a login or a push to a
# repository created seconds ago can drop the connection (observed: "Head
# .../blobs/...: EOF", and one login that failed while the same command
# worked by hand). Retrying is enough, and doing it here means a failure
# costs only the VPC and the roles, not a cluster teardown.
ecr_login() {
    aws ecr get-login-password --region "$region" 2>>"$out/push.log" |
        docker login --username AWS --password-stdin "$registry" >>"$out/push.log" 2>&1
}
login_ok=0
for attempt in 1 2 3 4 5; do
    if ecr_login; then
        login_ok=1
        break
    fi
    log "docker login to $registry attempt $attempt failed; retrying"
    tail -3 "$out/push.log" >&2
    sleep $((attempt * 5))
done
((login_ok)) || die "docker login to $registry"
push_image() {
    local name=$1 attempt
    docker tag "ergon-nri/$name:$tag" "$registry/$N/$name:$tag" || die "docker tag $name"
    for attempt in 1 2 3 4 5; do
        if docker push "$registry/$N/$name:$tag" >>"$out/push.log" 2>&1; then
            return 0
        fi
        log "push $name attempt $attempt failed; re-login and retry"
        tail -3 "$out/push.log" >&2
        ecr_login || true
        sleep $((attempt * 5))
    done
    tail -5 "$out/push.log" >&2
    die "push $name"
}
for name in shared-cpus workload; do
    push_image "$name"
done

create_cluster() {
    case $(cluster_status) in
    NONE) ;;
    UNKNOWN) return 1 ;;
    *) return 0 ;;
    esac
    local me
    me=$(curl -fsS -m 10 https://checkip.amazonaws.com | tr -d '[:space:]') || return 1
    [[ $me =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] || { log "cannot tell this machine's public IPv4 ($me)"; return 1; }
    # The /24 of this machine's address, not the /32: its egress can move
    # between neighbouring addresses (seen: .116 and .117 within minutes), and
    # a /32 then locks this machine out of its own cluster mid-run.
    eks create-cluster --name "$N" --kubernetes-version "$k8s" --role-arn "$cluster_role_arn" \
        --resources-vpc-config "{\"subnetIds\":[$cluster_subnets],\"endpointPublicAccess\":true,\"publicAccessCidrs\":[\"${me%.*}.0/24\"],\"endpointPrivateAccess\":true}" \
        --access-config authenticationMode=API,bootstrapClusterCreatorAdminPermissions=true \
        --upgrade-policy supportType=STANDARD \
        --logging '{"clusterLogging":[{"types":["api","audit","authenticator","controllerManager","scheduler"],"enabled":false}]}' \
        --tags "owner=ergon-lab,run=$stamp" >/dev/null 2>>"$out/create-cluster.log"
}
wait_for 180 15 "create-cluster to accept the new role" create_cluster || { tail -3 "$out/create-cluster.log" >&2; die "create-cluster"; }
log "cluster $N creating"
cluster_active() {
    local s
    s=$(cluster_status)
    [[ $s == FAILED ]] && die "cluster $N failed to create"
    [[ $s == ACTIVE ]]
}
wait_for 1500 30 "cluster $N to become active" cluster_active || die "cluster $N did not become active"
eks describe-cluster --name "$N" --output json >"$out/cluster.json" || die "describe-cluster"
read -r endpoint ca svc csg oidc <<<"$(python3 -c '
import json, sys
c = json.load(open(sys.argv[1]))["cluster"]
print(c["endpoint"], c["certificateAuthority"]["data"], c["kubernetesNetworkConfig"]["serviceIpv4Cidr"],
      c["resourcesVpcConfig"]["clusterSecurityGroupId"], c.get("identity", {}).get("oidc", {}).get("issuer", "-"))' "$out/cluster.json")"
[[ -n ${csg:-} ]] || die "cannot read the cluster's endpoint, CA, service CIDR and security group"
echo "${oidc##*/}" >"$out/oidc-id"
eks update-kubeconfig --name "$N" --kubeconfig "$out/kubeconfig" >/dev/null || die "update-kubeconfig"
export KUBECONFIG=$out/kubeconfig
can_admin() { [[ $(k auth can-i '*' '*' -A 2>/dev/null) == yes ]]; }
wait_for 120 10 "cluster admin access for $caller" can_admin ||
    die "$caller is not a cluster admin of $N (bootstrapClusterCreatorAdminPermissions did not take; a root caller is a likely cause: use an IAM user or role)"
eks create-access-entry --cluster-name "$N" --principal-arn "$node_role_arn" --type EC2_LINUX \
    --tags "owner=ergon-lab,run=$stamp" >/dev/null || die "access entry for the nodes"

# The launch template: user data from eks-nri-userdata.sh (kernel command
# line, sysctls, one reboot, then nodeadm's NodeConfig), 20 GB gp3, public
# IP, no key pair, terminate on shutdown (the dead-man's switch).
QUIET_DIR=$EKS_SNAPSHOT/quiet bash "$scripts/eks-nri-userdata.sh" "$N" "$endpoint" "$ca" "$svc" "$reserved" "$isolated" "$hugepages" "$irq_cpu" \
    >"$out/userdata.gz" 2>"$out/userdata.log" || { cat "$out/userdata.log" >&2; die "eks-nri-userdata.sh failed (over the 16 KiB user-data limit?)"; }
log "$(cat "$out/userdata.log")"
python3 - "$ami" "$type" "$profile_arn" "$csg" "$sg" "$stamp" "$N" "$out/userdata.gz" >"$out/launch-template.json" <<'PY'
import base64, json, sys
ami, itype, profile, csg, sg, stamp, name, udpath = sys.argv[1:9]
ud = base64.b64encode(open(udpath, "rb").read()).decode()
tag = lambda role: {
    "ResourceType": role,
    "Tags": [
        {"Key": "owner", "Value": "ergon-lab"},
        {"Key": "run", "Value": stamp},
        {"Key": "Name", "Value": f"ergon-lab-{stamp}-{role}"},
    ],
}
print(json.dumps({
    "ImageId": ami,
    "InstanceType": itype,
    "IamInstanceProfile": {"Arn": profile},
    "UserData": ud,
    "BlockDeviceMappings": [{
        "DeviceName": "/dev/xvda",
        "Ebs": {"VolumeSize": 20, "VolumeType": "gp3", "DeleteOnTermination": True},
    }],
    "NetworkInterfaces": [{
        "DeviceIndex": 0,
        "Groups": [csg, sg],
        "AssociatePublicIpAddress": True,
    }],
    "TagSpecifications": [tag("instance"), tag("volume"), tag("network-interface")],
    "MetadataOptions": {"HttpTokens": "required", "HttpEndpoint": "enabled"},
    "InstanceInitiatedShutdownBehavior": "terminate",
}, indent=1))
PY
ec2 create-launch-template --launch-template-name "$lt" --launch-template-data "file://$out/launch-template.json" \
    --tag-specifications "$(tags launch-template lt)" >/dev/null || die "launch template"

# Both nodes in one zone, so the round trip does not cross AZs. Spot
# requests set OnDemandTargetCapacity=0. The probe already found a zone;
# these passes cover the gap after those probe instances were released.
# If spot still cannot be placed, try the other 4-core Graviton types and
# then on-demand, in this region. The cluster already exists here.
set_lt_type() {
    ec2 create-launch-template-version --launch-template-name "$lt" --source-version 1 \
        --launch-template-data "{\"InstanceType\":\"$1\"}" \
        --query 'LaunchTemplateVersion.VersionNumber' --output text
}
seek_launch() {
    local pass zone nodes_subnet fleet_json fleet_id n target passes=4
    [[ $market == on-demand ]] && passes=2
    zones=$(ec2 describe-instance-type-offerings --location-type availability-zone --filters "Name=instance-type,Values=$type" \
        --query 'InstanceTypeOfferings[].Location' --output text | tr '\t' '\n' | sed '/^$/d' | sort)
    [[ -n $zones ]] || { log "$type is not offered in $region"; return 1; }
    if [[ -n $prefer_zone ]]; then
        zones="$prefer_zone $(printf '%s\n' $zones | grep -vx "$prefer_zone" || true)"
    fi
    launched=
    for pass in $(seq "$passes"); do
        if ((pass > 1)); then log "$market pass $pass after a wait"; sleep 30; fi
        for zone in $zones; do
            # A fresh CIDR per attempt: a failed attempt's subnet can refuse to
            # delete while a fleet still holds it. 10.31.2.0/24 and 10.31.3.0/24
            # are the cluster's own subnets.
            subnet_idx=$((subnet_idx + 1))
            nodes_subnet=$(ec2 create-subnet --vpc-id "$vpc" --cidr-block "10.31.$subnet_idx.0/24" --availability-zone "$zone" \
                --tag-specifications "$(tags subnet nodes)" --query Subnet.SubnetId --output text 2>>"$out/subnet.log") || {
                log "pass $pass zone $zone: no subnet: $(tail -1 "$out/subnet.log" 2>/dev/null)"
                continue
            }
            ec2 modify-subnet-attribute --subnet-id "$nodes_subnet" --map-public-ip-on-launch || die "node subnet public IPs"
            ec2 associate-route-table --route-table-id "$rt" --subnet-id "$nodes_subnet" >/dev/null || die "node subnet route"
            fleet_json=$out/fleet-$market-$pass-$zone.json
            if [[ $market == on-demand ]]; then
                target="TotalTargetCapacity=2,DefaultTargetCapacityType=on-demand,OnDemandTargetCapacity=2"
                ec2 create-fleet --type instant \
                    --target-capacity-specification "$target" \
                    --launch-template-config "LaunchTemplateSpecification={LaunchTemplateName=$lt,Version=$lt_version},Overrides=[{SubnetId=$nodes_subnet}]" \
                    --tag-specifications "$(tags fleet fleet)" \
                    --output json >"$fleet_json" 2>"$out/create-fleet-$market-$pass-$zone.log" || true
            else
                target="TotalTargetCapacity=2,DefaultTargetCapacityType=spot,OnDemandTargetCapacity=0"
                ec2 create-fleet --type instant \
                    --spot-options "AllocationStrategy=$spot_strategy" \
                    --target-capacity-specification "$target" \
                    --launch-template-config "LaunchTemplateSpecification={LaunchTemplateName=$lt,Version=$lt_version},Overrides=[{SubnetId=$nodes_subnet}]" \
                    --tag-specifications "$(tags fleet fleet)" \
                    --output json >"$fleet_json" 2>"$out/create-fleet-$market-$pass-$zone.log" || true
            fi
            fleet_id=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("FleetId") or "")' "$fleet_json" 2>/dev/null || true)
            [[ -n $fleet_id ]] && printf '%s\n' "$fleet_id" >>"$out/fleet-ids"
            ids=$(python3 -c '
import json, sys
try:
    f = json.load(open(sys.argv[1]))
except Exception:
    sys.exit(0)
print("\n".join(i for inst in f.get("Instances", []) for i in inst.get("InstanceIds", [])))  # a list of ids' "$fleet_json" 2>/dev/null || true)
            n=$(wc -w <<<"$ids")
            if ((n == 2)); then
                launched=$zone
                return 0
            fi
            log "pass $pass zone $zone gave $n of 2 $market instances: $(tail -1 "$out/create-fleet-$market-$pass-$zone.log" 2>/dev/null)"
            # A failed instant fleet stays active and holds its vCPUs. Delete
            # it before the next zone, or a few misses exhaust the quota.
            if [[ -n $fleet_id ]]; then
                ec2 delete-fleets --fleet-ids "$fleet_id" --terminate-instances >/dev/null 2>&1 ||
                    log "pass $pass zone $zone: could not delete fleet $fleet_id"
            fi
            ec2 delete-subnet --subnet-id "$nodes_subnet" >/dev/null 2>&1 || true
        done
    done
    return 1
}
launched=
subnet_idx=10
seek_launch || true
if [[ -z $launched && $market == spot ]]; then
    for alt in c9g.xlarge c7g.xlarge c8g.xlarge; do
        [[ $alt == "$type" ]] && continue
        if ! fallback_shape_ok "$alt"; then
            log "skip $alt: not a 4-vCPU Graviton with no SMT, so the CPU pins would change"
            continue
        fi
        log "no spot for 2 x $type in $region; trying spot $alt"
        lt_version=$(set_lt_type "$alt") || continue
        type=$alt
        seek_launch && break
    done
fi
if [[ -z $launched ]]; then
    log "on-demand fallback in $region. Spot could not place 2 instances."
    market=on-demand
    if [[ $type != "$preferred_type" ]] && layout_ok "$preferred_type" "$region"; then
        type=$preferred_type
    fi
    lt_version=$(set_lt_type "$type") || die "cannot set the launch template to $type"
    seek_launch || true
fi
if [[ -z $launched ]]; then
    for alt in c8g.xlarge c7g.xlarge c9g.xlarge; do
        [[ $alt == "$type" ]] && continue
        fallback_shape_ok "$alt" || continue
        hourly=$(ondemand_hourly "$alt") || continue
        log "on-demand $type did not launch; trying $alt (estimated \$$hourly/h)"
        lt_version=$(set_lt_type "$alt") || continue
        type=$alt
        seek_launch && break
    done
fi
[[ -n $launched ]] || die "could not place 2 instances in $region on $market after searching every enabled region"
read -r ida idb <<<"$(tr '\n' ' ' <<<"$ids")"
log "nodes $ida (a) $idb (b) $market in $launched; waiting for them to run"
ec2 wait instance-running --instance-ids "$ida" "$idb" || die "the nodes did not start"
{
    echo "cluster $N kubernetes $k8s"
    echo "type $type zone $launched market $market strategy $spot_strategy"
    echo "ami $ami"
    echo "images $registry/$N/{shared-cpus,workload}:$tag"
    echo "a $ida"
    echo "b $idb"
    echo "reserved $reserved isolated $isolated shared $shared_cpus irq $irq_cpu hugepages $hugepages"
} >"$out/instances.txt"

nodes_ready() {
    local n
    n=$(k get nodes -l ergon.io/shared-cpus=true \
        -o jsonpath='{range .items[*]}{range .status.conditions[?(@.type=="Ready")]}{.status}{"\n"}{end}{end}' 2>/dev/null | grep -c '^True$')
    ((n == 2))
}
log "waiting for both nodes to provision, reboot and join (up to 25 minutes)"
if ! wait_for 1500 30 "2 Ready nodes labelled ergon.io/shared-cpus=true" nodes_ready; then
    for n in a b; do
        id=$ida
        [[ $n == b ]] && id=$idb
        ec2 get-console-output --instance-id "$id" --latest --output text >"$out/$n-console.log" 2>&1
    done
    die "the nodes did not join (console output: $out/a-console.log, $out/b-console.log)"
fi
node_of() { k get nodes -o jsonpath='{range .items[*]}{.metadata.name}{" "}{.spec.providerID}{"\n"}{end}' | awk -v id="$1" '$2 ~ "/" id "$" {print $1}'; }
node_a=$(node_of "$ida")
node_b=$(node_of "$idb")
[[ -n $node_a && -n $node_b ]] || die "cannot match the nodes to $ida and $idb"
log "nodes: a $node_a, b $node_b"

# ---- deploy ------------------------------------------------------------------

mkdir -p "$out/k8s"
cat >"$out/k8s/probe.yaml" <<'YAML'
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: ergon-lab-probe
  namespace: default
spec:
  selector:
    matchLabels:
      app: ergon-lab-probe
  template:
    metadata:
      labels:
        app: ergon-lab-probe
    spec:
      nodeSelector:
        ergon.io/shared-cpus: "true"
      tolerations:
        - operator: Exists
      hostPID: true
      terminationGracePeriodSeconds: 1
      containers:
        - name: probe
          image: PROBE_IMAGE
          command: ["sleep", "infinity"]
          securityContext:
            privileged: true
YAML
sed -i.bak "s|PROBE_IMAGE|$registry/$N/workload:$tag|" "$out/k8s/probe.yaml" && rm -f "$out/k8s/probe.yaml.bak"

# The shared-cpus plugin: this tree's chart (0.1.2) rendered with the book's
# EKS AL2023 install (kubelet config.json plus its config.json.d drop-ins,
# --kubelet-config-dir), the image built above from this tree and pushed to
# the run's ECR repository (never GHCR). The config for this node group:
# shared CPU 0, the lab namespaces only, annotation opt-in, and the grants the
# modes ask for (rlimits memlock, seccomp io_uring, the NIC PHC); devRoot is
# the chart's /host/dev (the host's /dev, read-only).
mkdir -p "$out/k8s"
helm template ergon-shared-cpus "$plugin_src/charts/ergon-shared-cpus" --namespace ergon-system \
    --set image.repository="$registry/$N/shared-cpus" --set image.tag="$tag" \
    --set-string config.sharedCpus="$shared_cpus" \
    --set 'config.namespaces={ergon-latency-pods,ergon-latency-pods-probe}' \
    --set config.annotationOptIn=true \
    --set-string config.seccomp.ioUring=allow --set-string config.ptp=auto --set-string config.rlimits.memlock=inf \
    --set hostContract.kubeletConfig.path=/etc/kubernetes/kubelet/config.json \
    --set hostContract.kubeletConfigDir.enabled=true \
    >"$out/k8s/plugin.yaml" 2>"$out/helm.log" || { cat "$out/helm.log" >&2; die "helm template of the shared-cpus chart"; }

# The latency namespaces and the validating admission policy: no liveness
# probes, readiness only httpGet/tcpSocket/grpc, no sidecars. The policy
# matches PODS labelled ergon.io/latency: "hot", the same label the shipped
# policy (k8s/nri-latency-pods/admission/validating-admission-policy.yaml) matches
# and the bench manifests set. make_pod sets it on every mode but M5; M5
# carries no such label and runs in ergon-latency-pods-probe on purpose, so its
# livenessProbe is admitted and the lab measures the harm the policy removes
# from a latency pod. admission_check below submits the refused case: the
# label with a livenessProbe. The namespace label ergon.io/latency: "true"
# is the mutating policy's (it reads namespaceObject), not this one's.
cat >"$out/k8s/admission.yaml" <<'YAML'
apiVersion: v1
kind: Namespace
metadata:
  name: ergon-latency-pods
  labels:
    ergon.io/latency: "true"
---
apiVersion: v1
kind: Namespace
metadata:
  name: ergon-latency-pods-probe
---
apiVersion: admissionregistration.k8s.io/v1
kind: ValidatingAdmissionPolicy
metadata:
  name: ergon-latency-pods
spec:
  failurePolicy: Fail
  matchConstraints:
    resourceRules:
      - apiGroups: [""]
        apiVersions: ["v1"]
        operations: ["CREATE", "UPDATE"]
        resources: ["pods"]
  matchConditions:
    - name: latency-pod
      # CEL has no ?? (that is TypeScript). A map index on a missing key is an
      # error, so the key is tested with `in` first; without this the policy
      # fails to compile and the run would stop at admission_check.
      expression: >-
        has(object.metadata.labels) &&
        'ergon.io/latency' in object.metadata.labels &&
        object.metadata.labels['ergon.io/latency'] == 'hot'
  validations:
    - expression: "object.spec.containers.all(c, !has(c.livenessProbe))"
      message: "latency pods take no livenessProbe (the exec lands on the hot cpuset; drop the probe or the ergon.io/latency label)"
      reason: Forbidden
    - expression: >-
        object.spec.containers.all(c, !has(c.readinessProbe) ||
        (has(c.readinessProbe.httpGet) || has(c.readinessProbe.tcpSocket) || has(c.readinessProbe.grpc)))
      message: "readinessProbe only httpGet, tcpSocket or grpc (never exec/command: those land in the container's cpuset)"
      reason: Forbidden
    - expression: "object.spec.containers.all(c, !has(c.restartPolicy) || c.restartPolicy != 'Always')"
      message: "no sidecars (restartPolicy: Always) in a latency pod"
      reason: Forbidden
---
apiVersion: admissionregistration.k8s.io/v1
kind: ValidatingAdmissionPolicyBinding
metadata:
  name: ergon-latency-pods
spec:
  policyName: ergon-latency-pods
  validationActions: ["Deny"]
YAML

# A fresh control plane can drop a TLS handshake (seen once: "TLS handshake
# timeout" fetching the OpenAPI schema); each apply gets five tries.
k_apply() { # <file> <what>
    local try
    for try in 1 2 3 4 5; do
        k apply -f "$1" >>"$out/apply.log" 2>&1 && return 0
        log "kubectl apply $2 failed (try $try); retrying"
        sleep $((try * 10))
    done
    cat "$out/apply.log" >&2
    die "kubectl apply $2"
}
: >"$out/apply.log"
k_apply "$out/k8s/admission.yaml" admission
k_apply "$out/k8s/plugin.yaml" plugin
k_apply "$out/k8s/probe.yaml" probe
# Each DaemonSet in the namespace it is deployed in: the shared-cpus plugin
# ships in ergon-system (deploy/daemonset.yaml), the probe in default. A wait
# on the wrong namespace finds nothing and the run dies before it measures.
for ds_ns in "ergon-shared-cpus ergon-system" "ergon-lab-probe default"; do
    ds=${ds_ns%% *}
    ns=${ds_ns##* }
    kubectl rollout status "ds/$ds" -n "$ns" --timeout=10m >>"$out/apply.log" 2>&1 ||
        {
            k describe ds "$ds" -n "$ns" >"$out/rollout-$ds.log" 2>&1
            k describe pods -n "$ns" -l "app=$ds" >>"$out/rollout-$ds.log" 2>&1
            k logs -n "$ns" -l "app=$ds" --tail=100 --all-containers >>"$out/rollout-$ds.log" 2>&1
            die "daemonset $ds in $ns did not roll out (see $out/rollout-$ds.log)"
        }
done
k get ds -A >>"$out/apply.log" 2>&1

pod_on() { k get pods -A -l "app=$1" --field-selector "spec.nodeName=$2" -o jsonpath='{.items[0].metadata.name}'; }
ns_of() { k get pods -A -l "app=$1" --field-selector "spec.nodeName=$2" -o jsonpath='{.items[0].metadata.namespace}'; }
for n in a b; do
    node=$(nv node "$n")
    [[ -n $node ]] || nv_set node "$n" "$( [[ $n == a ]] && echo "$node_a" || echo "$node_b" )"
    node=$(nv node "$n")
    nv_set probe "$n" "$(pod_on ergon-lab-probe "$node")"
    nv_set probe_ns "$n" "$(ns_of ergon-lab-probe "$node")"
    [[ -n $(nv probe "$n") ]] || die "$n: no probe pod on $node"
done
on_host() { local n=$1; shift; kx -i "$(nv probe "$n")" -n "$(nv probe_ns "$n")" -- nsenter -t 1 -m -u -i -n -p -- timeout 600 bash -s -- "$@"; }
in_pod() { local n=$1; shift; kx -n ergon-latency-pods "$@"; }

# The dead-man's switch: each node powers off (and so terminates with its
# disk) 30 minutes past the watchdog's limit, should this machine never
# delete it. The cluster itself waits for EKS_DELETE.
for n in a b; do
    on_host "$n" <<<"shutdown -h +$((max_minutes + 30)) 'ergon-lab deadline'" >/dev/null 2>&1 ||
        log "WARNING: $n: no shutdown deadline; only this machine's teardown deletes it"
done

# ---- node facts ---------------------------------------------------------------

mkdir -p "$out/facts"
for n in a b; do
    on_host "$n" >"$out/facts/$n.txt" 2>&1 <<'SH'
echo "== uname"; uname -a
echo "== containerd"; containerd --version 2>&1
echo "== runc"; runc --version 2>&1
echo "== containerd config dump (NRI)"; containerd config dump 2>&1 | grep -A6 -i nri
echo "== cmdline"; cat /proc/cmdline
echo "== numactl"; numactl -H 2>&1 || echo "no numactl"
echo "== ptp"; ls -l /dev/ptp* 2>&1
echo "== RT_GROUP_SCHED"; zcat /proc/config.gz 2>/dev/null | grep -E 'RT_GROUP_SCHED|PREEMPT' || echo "no /proc/config.gz"
echo "== sysctls"; sysctl net.core.rmem_max net.core.wmem_max net.core.busy_poll net.core.busy_read 2>&1
echo "== hugepages"; grep -i huge /proc/meminfo
echo "== isolated"; cat /sys/devices/system/cpu/isolated 2>&1
echo "== thp"; cat /sys/kernel/mm/transparent_hugepage/enabled
SH
    log "$n facts: $out/facts/$n.txt"
done
grep -H . "$out/facts"/*.txt | grep -E 'containerd version|runc version|Linux version|NUMA node\(s\)|ptp|RT_GROUP_SCHED|rmem_max|HugePages|isolated|thp' |
    sed 's/^/  /' >&2 || true
# Print the containerd NRI section in full: the run's one unverified claim
# is that AL2023's containerd has NRI on by default and accepts the
# validator fragment.
for n in a b; do
    log "$n: containerd NRI section:"
    sed -n '/== containerd config dump/,/== cmdline/p' "$out/facts/$n.txt" | sed 's/^/    /' >&2
done

has_ptp=1
if ! grep -q '^crw.*/dev/ptp' "$out/facts/a.txt" || ! grep -q '^crw.*/dev/ptp' "$out/facts/b.txt"; then
    has_ptp=0
    log "no /dev/ptp* on the nodes: M4 checks the refusal only, M6 runs without the PHC grant"
fi

# ---- mode manifests ----------------------------------------------------------

# make_pod <mode> <which a|b> <ping_ep> <pong_ep>: one pod YAML on stdout.
# Every pod is unprivileged (no capabilities, no privileged flag): the
# rlimits, the hugepage mount, the isolated partition and the PTP device
# come from the plugin's grants, never from the pod spec (PSS Baseline
# forbids hostPath and Unconfined). cpu: 2 Guaranteed takes the isolated
# CPUs; the shared-millicpus request is the opt-in to the plugin's shared
# CPU. SCHED_FIFO is deliberately absent (inconclusive in the one AWS A/B).
# The pod name of a role and mode: lowercase, as Kubernetes requires (M0 -> m0).
pname() { echo "ergon-lab-$1-$(tr '[:upper:]' '[:lower:]' <<<"$2")"; }
# Whether the nodes have a NIC PHC: without one, M6 is the all-on set less ptp
# (recorded), and M4 is a refusal check only. Set from the node facts above.
make_pod() {
    local mode=$1 which=$2 ping_ep=$3 pong_ep=$4
    local name ns=ergon-latency-pods
    name=$(pname "$which" "$mode")
    local node=$node_a
    [[ $which == b ]] && node=$node_b
    local ann_hugepages="" ann_partition="" ann_seccomp="" ann_devices="" ann_rlimits=""
    local huge="" aeron_dir=/data/aeron page_size=""
    [[ $mode == M5 ]] && ns=ergon-latency-pods-probe
    case $mode in
    M1 | M6)
        # The plugin mounts a sized hugetlbfs at /dev/hugepages (NRI
        # AddMount, pagesize=2M,size=N). No emptyDir medium: that has no
        # size= and Aeron refuses an unsized hugetlbfs. MEMLOCK rides
        # along so the app can mlock the pages.
        huge=1
        aeron_dir=/dev/hugepages/aeron
        page_size=2097152
        ann_hugepages=$'    hugepages.ergon.io/app: "2M"\n'
        ann_rlimits=$'    rlimits.ergon.io/app: "memlock"\n'
        ;;
    esac
    case $mode in
    M2 | M6) ann_partition=$'    cpu-partition.ergon.io/app: "isolated"\n' ;;
    esac
    case $mode in
    M3 | M6) ann_seccomp=$'    seccomp.ergon.io/app: "io-uring"\n' ;;
    esac
    case $mode in
    M4) ann_devices=$'    ptp.ergon.io/app: "auto"\n' ;;
    M6) ((has_ptp)) && ann_devices=$'    ptp.ergon.io/app: "auto"\n' ;;
    esac
    local probe="" huge_res=""
    [[ $mode == M5 ]] && probe=$(cat <<'PYAML'
      livenessProbe:
        exec:
          command: ["true"]
        periodSeconds: 1
        timeoutSeconds: 1
        failureThreshold: 1000
PYAML
)
    # Every mode but M5 is a latency pod: the validating policy matches this
    # label and would refuse M5's livenessProbe. M5 is the counterfactual, so
    # it carries no label and runs where the policy is not asked about it.
    local lat=""
    [[ $mode != M5 ]] && lat=$'    ergon.io/latency: "hot"\n'
    if [[ -n $huge ]]; then
        # k8s isolates hugepages at container scope; the plugin's mount
        # size matches this limit. 256Mi = the node's 256 x 2 MiB pages.
        huge_res=$(cat <<'PYAML'
          hugepages-2Mi: 256Mi
PYAML
)
    fi
    # The pod's command is a sleep: the script starts each rep's xpong and
    # xping inside it with kubectl exec (like the DPDK lab's driver pod), so
    # the files the clients write stay readable while the pod lives. Each
    # role is bound to the node whose IP becomes its endpoint: a generic
    # nodeSelector lets the two land anywhere, and swapped roles point the
    # RTT at the wrong process. The measured thread is moved into the
    # isolated partition and pinned by rep_cmd; an exec'd process starts on
    # the container's whole cpuset, which is why both are explicit.
    cat <<YAML
apiVersion: v1
kind: Pod
metadata:
  name: $name
  namespace: $ns
  labels:
    app: ergon-lab-$which
    ergon-lab/mode: "$mode"
$lat
  annotations:
    required-plugins.noderesource.dev: '["ergon-shared-cpus"]'
$ann_hugepages$ann_partition$ann_seccomp$ann_devices$ann_rlimits
spec:
  restartPolicy: Never
  hostNetwork: true
  nodeSelector:
    ergon.io/shared-cpus: "true"
    kubernetes.io/hostname: "$node"
  tolerations:
    - operator: Exists
  terminationGracePeriodSeconds: 1
  containers:
    - name: app
      image: $registry/$N/workload:$tag
      command: ["sleep", "infinity"]
      env:
        - name: HARNESS_DRIVER
          value: invoker
        - name: AERON_DIR
          value: $aeron_dir
        - name: AERON_FILE_PAGE_SIZE
          value: "$page_size"
        - name: AERON_TERM_BUFFER_SPARSE_FILE
          value: "false"
        - name: LABEL
          value: "$mode"
        - name: RTT_SECONDS
          value: "$rtt_seconds"
      securityContext:
        allowPrivilegeEscalation: false
        capabilities:
          drop: ["ALL"]
        seccompProfile:
          type: RuntimeDefault
      resources:
        requests:
          cpu: "$pod_cpus"
          memory: 1Gi
          ergon.io/shared-millicpus: "250"
$huge_res
        limits:
          cpu: "$pod_cpus"
          memory: 1Gi
          ergon.io/shared-millicpus: "250"
$huge_res
$probe
      volumeMounts:
        - name: work
          mountPath: /data
  volumes:
        - name: work
          emptyDir: {}
YAML
}

# The admission policy proof: a livenessProbe in the labelled namespace is
# rejected by name; the same pod in ergon-latency-pods-probe is admitted (M5 runs).
admission_check() {
    local bad
    bad=$(k apply -f - 2>&1 <<'YAML' || true
apiVersion: v1
kind: Pod
metadata:
  name: ergon-lab-admission-should-fail
  namespace: ergon-latency-pods
  labels:
    ergon.io/latency: "hot"
spec:
  restartPolicy: Never
  containers:
    - name: app
      image: busybox
      command: ["true"]
      livenessProbe:
        exec:
          command: ["true"]
YAML
)
    if [[ $bad == *livenessProbe* || $bad == *Denied* || $bad == *denied* || $bad == *Forbidden* || $bad == *ergon-latency-pods* ]]; then
        log "admission policy: a livenessProbe on a labelled pod is rejected (as designed)"
        echo "admission: rejected livenessProbe on a labelled pod: $bad" >>"$out/admission.txt"
    else
        log "ERROR: the admission policy did not reject a livenessProbe on a labelled pod: $bad"
        echo "admission: NOT rejected: $bad" >>"$out/admission.txt"
        k delete pod ergon-lab-admission-should-fail -n ergon-latency-pods --ignore-not-found >/dev/null 2>&1
        return 1
    fi
    k delete pod ergon-lab-admission-should-fail -n ergon-latency-pods --ignore-not-found >/dev/null 2>&1

    # The other side of the same rule: the same probe on a pod that carries
    # no latency label must be admitted, or M5 could not run its harm.
    local ok
    ok=$(k apply -f - 2>&1 <<'YAML' || true
apiVersion: v1
kind: Pod
metadata:
  name: ergon-lab-admission-should-pass
  namespace: ergon-latency-pods-probe
spec:
  restartPolicy: Never
  containers:
    - name: app
      image: busybox
      command: ["true"]
      livenessProbe:
        exec:
          command: ["true"]
YAML
)
    k delete pod ergon-lab-admission-should-pass -n ergon-latency-pods-probe --ignore-not-found >/dev/null 2>&1
    if [[ $ok == *denied* || $ok == *Denied* || $ok == *Forbidden* || $ok == *ergon-latency-pods* ]]; then
        log "ERROR: the admission policy refused an unlabelled pod (M5 could not run): $ok"
        echo "admission: refused an unlabelled pod: $ok" >>"$out/admission.txt"
        return 1
    fi
    log "admission policy: a livenessProbe on an unlabelled pod is admitted (M5 can run)"
    echo "admission: admitted a livenessProbe on an unlabelled pod: $ok" >>"$out/admission.txt"
    return 0
}
admission_check || die "the admission policy is not in force (see $out/admission.txt); the run would not measure extension 4"

# Whether this plugin build grants rlimits and seccomp (the plan's plugin
# extensions). Absent: the modes still run, and the run records that the
# grant was missing (prelock fails closed).
plugin_metrics() { # [which a|b]: the plugin pod's /metrics on that node, fetched from the node
    local n=${1:-a} ip
    ip=$(k get pods -n ergon-system -l app=ergon-shared-cpus --field-selector "spec.nodeName=$(nv node "$n")" \
        -o jsonpath='{.items[0].status.podIP}' 2>/dev/null)
    [[ -n $ip ]] || return 1
    on_host "$n" <<<"curl -fsS -m 10 http://$ip:9466/metrics"
}
plugin_metrics a >"$out/plugin-metrics.txt" 2>&1 || true
rlimits_metric=no seccomp_metric=no
grep -q ergon_shared_cpus_rlimits_granted "$out/plugin-metrics.txt" && rlimits_metric=yes
grep -q ergon_shared_cpus_seccomp_granted "$out/plugin-metrics.txt" && seccomp_metric=yes
log "plugin grants in this build: rlimits=$rlimits_metric seccomp=$seccomp_metric (recorded; no gate)"

# ---- measure -----------------------------------------------------------------

known=/dev/null # lib.sh's ssh options, unused here
. "$benches/lib.sh" # for loss_row
# lib.sh's loss_row appends to $results, set above

# counters and /proc/vmstat of one lab pod (ns from the mode).
pod_ns() { [[ $1 == M5 ]] && echo ergon-latency-pods-probe || echo ergon-latency-pods; }
pod_of() { k get pods -n "$2" -l "app=ergon-lab-$1" -o jsonpath='{.items[0].metadata.name}' 2>/dev/null; }
loss_now() { # <which> <ns>
    local pod
    pod=$(pod_of "$1" "$2")
    [[ -n $pod ]] || { echo "invalid=1"; return 0; }
    kx -n "$2" "$pod" -- timeout 20 counters loss 2>/dev/null | tr '\n' ';'
}
vmstat_now() { # <which> <ns> -> two lines pgfault/pgmajfault
    local pod
    pod=$(pod_of "$1" "$2")
    [[ -n $pod ]] || { echo "pgfault 0"; echo "pgmajfault 0"; return 0; }
    kx -n "$2" "$pod" -- grep -E '^(pgfault|pgmajfault) ' /proc/vmstat 2>/dev/null
}

node_ip() { k get node "$1" -o jsonpath='{.status.addresses[?(@.type=="InternalIP")].address}'; }
ip_a=$(node_ip "$node_a")
ip_b=$(node_ip "$node_b")
[[ -n $ip_a && -n $ip_b ]] || die "no internal IP for both nodes"

wait_pods_ready() { # <ns>
    local ns=$1 i ready
    for i in $(seq 20); do
        ready=$(k get pods -n "$ns" -l 'app in (ergon-lab-a,ergon-lab-b)' \
            --field-selector=status.phase=Running -o name 2>/dev/null | wc -l)
        ready=${ready// /}
        ((ready >= 2)) && return 0
        sleep 6
    done
    return 1
}

# The rep's command inside the pod: the gap spinner on the second granted
# exclusive CPU (to attribute every max), then the measured client on the
# first. Both are moved into the isolated partition first — a task left
# outside it cannot execute on those CPUs at all — and the rep refuses to
# produce numbers unless the measured task is shown to be in the child and
# on the CPU it was given. The CPUs come from the grant, never a literal.
rep_cmd() { # <mode> <which> <ping_ep> <pong_ep>
    local mode=$1 which=$2 ping_ep=$3 pong_ep=$4 pre="" client="" envs=""
    # A control mode (M0/M1/M3/M4/M5) is measured in the container cgroup on
    # purpose: it asked for no cpu-partition.ergon.io, so there is no
    # ERGON_ISOLATED_CGROUP and no isolated child to enter. Requiring one
    # here made every control rep exit before it measured anything.
    local want_partition=0
    case $mode in
    # prelock --mlock asks the target to mlockall, which the plugin's
    # RLIMIT_MEMLOCK grant (rlimits.ergon.io/app: "memlock") makes possible
    # in an unprivileged pod. prelock does not itself lock any pages.
    M1 | M6) pre="prelock --mlock --" ;;
    esac
    case $mode in
    M2 | M6) want_partition=1 ;;
    esac
    if [[ $which == a ]]; then
        client="rtt xping $ping_ep $pong_ep $rtt_messages $rtt_warmup"
        # These have to be exported, not prefixed to one command: the old
        # `VAR=... mkdir; ...; rtt` spelling set them for mkdir alone, so
        # the histogram and spike files were never written.
        envs="export RTT_HISTOGRAM=/data/rtt-histogram.csv LOAD_SPIKES=/data/load-spikes.csv LOAD_SPIKE_NS=100000 LOOP_STALLS_DIR=/data"
    else
        client="rtt xpong $ping_ep $pong_ep"
    fi
    local cmd="$client"
    [[ -n $pre ]] && cmd="$pre $client"
    # The partition block is emitted only for a mode that asked for one: a
    # control mode's script has no isolated child to find, no move and no
    # membership proof. The block is single-quoted so its `$`s reach the pod.
    local partition_enter="" partition_prove="" partition_note="no partition (control mode)"
    if [[ $want_partition == 1 ]]; then
        partition_note="partition isolated"
        partition_enter='child=${ERGON_ISOLATED_CGROUP:-}
if [ -n "$child" ] && [ ! -d "$child" ]; then
    child=/sys/fs/cgroup/$(basename "$child")
fi
[ -d "$child" ] || { echo "rep: this mode asked for a partition but no isolated cgroup is under ERGON_ISOLATED_CGROUP" >&2; exit 1; }
[ "$(cat "$child/cpuset.cpus.partition" 2>/dev/null)" = isolated ] ||
    { echo "rep: $child/cpuset.cpus.partition is not isolated" >&2; exit 1; }
# The whole process group goes into the partition before anything starts.
echo $$ >"$child/cgroup.procs" || { echo "rep: cannot move $$ into $child" >&2; exit 1; }'
        partition_prove='grep -q "^$app$" "$child/cgroup.threads" 2>/dev/null ||
    { echo "rep: pid $app is not in $child" >&2; kill $app $gap 2>/dev/null || true; exit 1; }'
    fi
    cat <<EOF
$envs
set -eu
mkdir -p "\${AERON_DIR:-/data/aeron}"
rm -f /data/gapspin-$which.log /data/load-spikes.csv /data/stalls-ping.csv /data/stalls-pong.csv /data/rtt-histogram.csv

# The exclusive CPUs the plugin granted this pod. Every mode gets these from
# ergon-shared-cpus; only the partition annotation adds an isolated child.
iso=\${ERGON_ISOLATED_CPUS:-}
[ -n "\$iso" ] || { echo "rep: ERGON_ISOLATED_CPUS is empty: no exclusive-CPU grant" >&2; exit 1; }
expand() { echo "\$1" | tr ',' '\n' | while read -r r; do case \$r in *-*) seq "\${r%%-*}" "\${r##*-}";; *) echo "\$r";; esac; done; }
set -- \$(expand "\$iso")
[ \$# -ge 1 ] || { echo "rep: ERGON_ISOLATED_CPUS=\$iso names no CPU" >&2; exit 1; }
cpu_client=\$1
cpu_gap=\${2:-}

# The isolated child. The env carries the node's path; inside the pod's own
# cgroup namespace the child is a directory of the namespace root.
$partition_enter

# the gap spinner only on a second exclusive CPU, never beside the client
gap=
if [ -n "\$cpu_gap" ]; then
    gapspin \$cpu_gap 50 > /data/gapspin-$which.log 2>&1 &
    gap=\$!
fi
$cmd \$cpu_client &
app=\$!

# Prove it before any number counts.
sleep 0.2
$partition_prove
mask=\$(taskset -pc \$app 2>/dev/null | sed 's/.*: //')
case ",\$iso," in
*,\$cpu_client,*) ;;
*) echo "rep: cpu \$cpu_client is not in ERGON_ISOLATED_CPUS=\$iso" >&2; kill \$app \$gap 2>/dev/null || true; exit 1 ;;
esac
echo "rep: $which pid \$app on cpu \$cpu_client (affinity \$mask); gapspin \$gap on \$cpu_gap; $partition_note" >&2
wait \$app
EOF
}

# What each extension actually granted, read back from inside the pods
# (the plan's functional checks). Recorded per mode, and a mode whose grant
# cannot be read back is not measured: an extension that did not fire is a
# diagnostic, never a performance comparison. Returns 0 only when every
# grant that mode asked for is present in both pods.
functional_check() { # <mode> <ns> -> 0 when every promised grant is there
    local mode=$1 ns=$2 pod out=$out/functional-$mode.txt
    local ok=0 missing=""
    : >"$out"
    for which in a b; do
        pod=$(pod_of "$which" "$ns")
        if [[ -z $pod ]]; then
            echo "$which: no pod" >>"$out"
            ok=1
            missing="$missing $which:no-pod"
            continue
        fi
        {
            echo "== $which $pod"
            kx -n "$ns" "$pod" -- sh -c '
                echo "AERON_DIR=$AERON_DIR"
                echo "ERGON_HUGEPAGES_DIR=${ERGON_HUGEPAGES_DIR:-}"
                echo "ERGON_HUGEPAGE_SIZE=${ERGON_HUGEPAGE_SIZE:-}"
                echo "ERGON_ISOLATED_CPUS=${ERGON_ISOLATED_CPUS:-}"
                echo "ERGON_SHARED_CPUS=${ERGON_SHARED_CPUS:-}"
                echo "ERGON_ISOLATED_CGROUP=${ERGON_ISOLATED_CGROUP:-}"
                echo "ERGON_PTP_DEVICE=${ERGON_PTP_DEVICE:-}"
                echo "== findmnt hugepages"; findmnt /dev/hugepages 2>&1 || true
                echo "== partition"
                child=${ERGON_ISOLATED_CGROUP:-}
                if [ -n "$child" ] && [ ! -d "$child" ]; then
                    child=/sys/fs/cgroup/$(basename "$child")
                fi
                echo "child=$child"
                if [ -n "$child" ] && [ -d "$child" ]; then
                    cat "$child/cpuset.cpus.partition" 2>&1 || true
                    echo "== child cpus"; cat "$child/cpuset.cpus" 2>&1 || true
                    echo "== child type"; cat "$child/cgroup.type" 2>&1 || true
                else
                    echo "(no isolated child)"
                fi
                echo "== parent cpuset"; cat /sys/fs/cgroup/cpuset.cpus 2>&1 || true
                echo "== ptp"; ls -l /dev/ptp* 2>&1 || true
                echo "== io_uring"; io_uring_probe 2>&1 || true
                echo "== seccomp"; grep Seccomp /proc/self/status
            ' 2>&1
        } >>"$out"
    done
    log "functional $mode: $out"
    grep -E 'findmnt|pagesize|size=|partition|child|ERGON_|ptp|io_uring|Seccomp|isolated' "$out" | sed 's/^/    /' >&2 || true

    # Enforce: the read-back has to show what this mode asked for.
    local want=""
    case $mode in
    M1 | M6) want="hugepages" ;;
    esac
    case $mode in
    M2 | M6) want="$want partition" ;;
    esac
    case $mode in
    M3 | M6) want="$want io_uring" ;;
    esac
    case $mode in
    M4) want="$want ptp" ;;
    M6) ((has_ptp)) && want="$want ptp" ;;
    esac
    local w
    for w in $want; do
        case $w in
        hugepages)
            if ! grep -q 'ERGON_HUGEPAGES_DIR=/dev/hugepages' "$out" ||
                ! grep -q 'size=' "$out"; then
                missing="$missing hugepages"
            fi
            ;;
        partition)
            if ! grep -q '^isolated$' "$out" || ! grep -q 'ERGON_ISOLATED_CPUS=.' "$out"; then
                missing="$missing partition"
            fi
            ;;
        io_uring)
            if ! grep -q 'io_uring_setup fd=' "$out"; then
                missing="$missing io_uring"
            fi
            ;;
        ptp)
            if ! grep -q 'ERGON_PTP_DEVICE=/dev/ptp\|/dev/ptp[0-9]' "$out"; then
                missing="$missing ptp"
            fi
            ;;
        esac
    done
    if [[ -n $missing ]]; then
        log "functional $mode: missing$missing"
        echo "functional $mode: missing$missing" >>"$out"
        return 1
    fi
    return 0
}


# ---- the EKS lane (k8s/nri-shared-cpus/RELEASE.md) ---------------------------

# Each item the release gate lists as unproven on EKS, read on both nodes and
# from the API, with a verdict: VERIFIED, FAILED (and why). The quiet-CPU
# boot unit's --check is read here too.
eks_lane() {
    local n f=$out/eks-lane.md v
    for n in a b; do
        on_host "$n" >"$out/eks-lane-$n.txt" 2>&1 <<'SH'
echo "== nri socket"; ls -l /var/run/nri/nri.sock 2>&1
echo "== containerd nri"; containerd config dump 2>/dev/null | awk '/io.containerd.nri.v1.nri/{p=1} p&&/^\[/&&!/nri/{p=0} p' | head -30
echo "== containerd drop-ins"; ls -l /etc/containerd/config.d/ 2>&1; grep -rn nri /etc/containerd/ 2>&1 | head -10
echo "== kubelet argv"; tr '\0' ' ' </proc/"$(pgrep -xo kubelet)"/cmdline; echo
echo "== kubelet drop-ins"; ls -l /etc/kubernetes/kubelet/config.json.d/ 2>&1; cat /etc/kubernetes/kubelet/config.json.d/*.conf 2>&1 | head -60
echo "== kubelet base reservedSystemCPUs"; grep -o '"reservedSystemCPUs":[^,]*' /etc/kubernetes/kubelet/config.json 2>&1
echo "== opt"; ls -l /opt/ergon/bin/ 2>&1; findmnt -T /opt -no SOURCE,OPTIONS 2>&1
echo "== quiet unit"; echo "enabled $(systemctl is-enabled ergon-quiet-cpus.service 2>&1) active $(systemctl is-active ergon-quiet-cpus.service 2>&1)"
/usr/local/sbin/ergon-quiet-cpus.sh --check 2>&1; echo "quiet-check rc=$?"
echo "== cmdline"; cat /proc/cmdline
echo "== isolated"; cat /sys/devices/system/cpu/isolated /sys/devices/system/cpu/nohz_full
echo "== irqbalance"; systemctl is-active irqbalance 2>&1
echo "== nic irqs (irq requested effective name)"
for i in $(awk -F: '/eth|ens|ena|Tx|Rx|mgmnt/ {gsub(/ /, "", $1); print $1}' /proc/interrupts); do
    echo "irq $i $(cat /proc/irq/$i/smp_affinity_list 2>/dev/null) $(cat /proc/irq/$i/effective_affinity_list 2>/dev/null) $(awk -v i="$i:" '$1 == i {print $NF}' /proc/interrupts)"
done
echo "== cpu_manager_state"; cat /var/lib/kubelet/cpu_manager_state 2>&1; echo
SH
    done
    k get pods -n ergon-system -l app=ergon-shared-cpus \
        -o jsonpath='{range .items[*]}{.metadata.name} {.metadata.namespace} {.spec.priorityClassName} {.status.phase} {.spec.nodeName}{"\n"}{end}' \
        >"$out/eks-lane-plugin-pods.txt" 2>&1
    k logs -n ergon-system -l app=ergon-shared-cpus --tail=200 --prefix >"$out/plugin-logs-start.txt" 2>&1
    for n in a b; do plugin_metrics "$n" >"$out/plugin-metrics-$n.txt" 2>&1 || true; done
    {
        echo "| item | node a | node b | evidence |"
        echo "|---|---|---|---|"
        lane_row() { # <item> <evidence> <test for a> <test for b>
            local ra rb
            ra=FAILED rb=FAILED
            eval "$3" && ra=VERIFIED
            eval "$4" && rb=VERIFIED
            echo "| $1 | $ra | $rb | $2 |"
        }
        for n in a b; do
            eval "m_$n=\$(cat \"$out/plugin-metrics-$n.txt\" 2>/dev/null)"
            eval "h_$n=\$(cat \"$out/eks-lane-$n.txt\" 2>/dev/null)"
        done
        lane_row "NRI socket" "/var/run/nri/nri.sock is a socket; ergon_shared_cpus_nri_connected 1" \
            '[[ $h_a == *"srw"*nri.sock* && $m_a == *"ergon_shared_cpus_nri_connected 1"* ]]' \
            '[[ $h_b == *"srw"*nri.sock* && $m_b == *"ergon_shared_cpus_nri_connected 1"* ]]'
        lane_row "nodeadm TOML merge" "containerd config dump shows the NodeConfig's NRI table (default_validator enable = true)" \
            '[[ $h_a == *default_validator* && $h_a == *"enable = true"* ]]' \
            '[[ $h_b == *default_validator* && $h_b == *"enable = true"* ]]'
        lane_row "kubelet drop-in merge (--kubelet-config-dir)" "kubelet runs with --config-dir; ergon_shared_cpus_contract_ok 1 (the merged reservedSystemCPUs and static policy pass)" \
            '[[ $h_a == *--config-dir* && $m_a == *"ergon_shared_cpus_contract_ok 1"* ]]' \
            '[[ $h_b == *--config-dir* && $m_b == *"ergon_shared_cpus_contract_ok 1"* ]]'
        lane_row "/opt writable" "the plugin installed /opt/ergon/bin/ergon-cpus on the node" \
            '[[ $h_a == *ergon-cpus* ]]' '[[ $h_b == *ergon-cpus* ]]'
        lane_row "system-node-critical outside kube-system" "the plugin pod in ergon-system with priorityClassName system-node-critical, Running" \
            "grep -q \" ergon-system system-node-critical Running $(nv node a)\" \"$out/eks-lane-plugin-pods.txt\"" \
            "grep -q \" ergon-system system-node-critical Running $(nv node b)\" \"$out/eks-lane-plugin-pods.txt\""
        lane_row "device plugin registered" "ergon_shared_cpus_device_plugin_registered 1" \
            '[[ $m_a == *"ergon_shared_cpus_device_plugin_registered 1"* ]]' '[[ $m_b == *"ergon_shared_cpus_device_plugin_registered 1"* ]]'
        lane_row "quiet-CPU boot unit" "ergon-quiet-cpus.service enabled and active; quiet-cpus.sh --check rc 0" \
            '[[ $h_a == *"enabled enabled active active"* && $h_a == *"quiet-check rc=0"* ]]' \
            '[[ $h_b == *"enabled enabled active active"* && $h_b == *"quiet-check rc=0"* ]]'
        echo
        echo "User data: $(cat "$out/userdata.log" 2>/dev/null)"
    } >"$f"
    log "EKS lane:"
    sed 's/^/    /' "$f" >&2
}
eks_lane

# The VPC CNI attaches its warm ENI after boot, and that ENA's queue IRQs came
# up with requested affinity 0-1 (irqaffinity=1 notwithstanding), which the
# plugin refuses hot pods for (run eks-nri-20261025-082023, ens35). Re-pin
# every movable IRQ to the IRQ CPU before each measurement.
repin_irqs() {
    local n
    for n in a b; do
        on_host "$n" >>"$out/repin.log" 2>&1 <<SH
c=0; for f in /proc/irq/*/smp_affinity_list; do echo $irq_cpu > \$f 2>/dev/null && c=\$((c + 1)); done
echo "\$(date +%T) $n: \$c IRQs on CPU $irq_cpu"
SH
    done
}

# ---- phase X -------------------------------------------------------------------
#
# Two latency pods (the M3 manifest: exclusive CPUs, the shared CPU, the
# io_uring seccomp grant) and, for the loaded cells, a Burstable pod outside
# the plugin's namespaces with spinners pinned to the shared CPU. kubectl exec
# starts on the whole cpuset with the kernel's 50 us slack, so every probe here
# names its CPU and slack itself.
xcsv=$out/phase-x.csv
: >"$xcsv"
x_sh() { local w=$1; shift; kx -n ergon-latency-pods "$(pname "$w" M3)" -- sh -c "$*"; }
load_on() { # <which> <spinners on the shared CPU>
    local w=$1 c=$2 node
    node=$(nv node "$w")
    k apply -f - >>"$out/apply.log" 2>&1 <<YAML
apiVersion: v1
kind: Pod
metadata:
  name: ergon-lab-load-$w
  namespace: default
  labels:
    app: ergon-lab-load
spec:
  restartPolicy: Never
  nodeSelector:
    kubernetes.io/hostname: "$node"
  tolerations:
    - operator: Exists
  terminationGracePeriodSeconds: 1
  containers:
    - name: load
      image: $registry/$N/workload:$tag
      command: ["sh", "-c", "for i in \$(seq $c); do taskset -c $shared_cpus sh -c 'while :; do :; done' & done; wait"]
      resources:
        requests:
          cpu: 100m
          memory: 32Mi
YAML
    k wait --for=condition=Ready "pod/ergon-lab-load-$w" -n default --timeout=180s >>"$out/apply.log" 2>&1
}
load_off() { k delete pod -n default -l app=ergon-lab-load --ignore-not-found --wait=true >/dev/null 2>&1; }
cpu_busy() { # <which>: the shared CPU's busy share over 2 s, from /proc/stat on the node
    on_host "$1" <<SH
a=\$(grep '^cpu$shared_cpus ' /proc/stat); sleep 2; b=\$(grep '^cpu$shared_cpus ' /proc/stat)
echo "\$a" "\$b" | awk '{t1=0; for (i=2;i<=11;i++) t1+=\$i; t2=0; for (i=13;i<=22;i++) t2+=\$i; i1=\$5+\$6; i2=\$16+\$17; printf "%.0f\n", 100*(1-(i2-i1)/(t2-t1))}'
SH
}
upp_script() { # <ping|pong> <busy|sqshared|sqexcl> <local port> <peer ip> <peer port> <seconds> <label>
    cat <<EOF
expand() { echo "\$1" | tr ',' '\n' | while read -r r; do case \$r in *-*) seq "\${r%%-*}" "\${r##*-}";; *) echo "\$r";; esac; done; }
set -- \$(expand "\$ERGON_ISOLATED_CPUS")
app=\$1; second=\${2:-}
sh0=\$(expand "\$ERGON_SHARED_CPUS" | head -1)
case $2 in
busy) mode=busy sq=- ;;
sqshared) mode=sqpoll sq=\$sh0 ;;
sqexcl) [ -n "\$second" ] || { echo "no second exclusive CPU" >&2; exit 1; }; mode=sqpoll sq=\$second ;;
esac
exec uring-pp $1 $3 $4 $5 \$app \$mode \$sq $6 10000 10000 $7
EOF
}
phase_x_run() {
    local ok=0 w cell lay load port=23100 pong_bg
    log "---- phase X: placement, wake-up wait, timer slack, pin cost, SQPOLL placement (${x_seconds}s a cell)"
    repin_irqs
    k delete pod -n ergon-latency-pods -l 'app in (ergon-lab-a,ergon-lab-b)' --ignore-not-found >/dev/null 2>&1
    make_pod M3 a 0 0 >"$out/k8s/pod-a-X.yaml"
    make_pod M3 b 0 0 >"$out/k8s/pod-b-X.yaml"
    if ! k apply -f "$out/k8s/pod-a-X.yaml" >>"$out/apply.log" 2>&1 || ! k apply -f "$out/k8s/pod-b-X.yaml" >>"$out/apply.log" 2>&1 ||
        ! wait_pods_ready ergon-latency-pods; then
        k describe pod -n ergon-latency-pods -l 'app in (ergon-lab-a,ergon-lab-b)' >"$out/X-describe.log" 2>&1
        log "WARNING: phase X: the pods did not run (see $out/X-describe.log)"
        return 1
    fi
    # placement: what the hook gave PID 1, the container's cpuset, an exec'd
    # shell, the env, and the node's view of a helper and a spinner
    for w in a b; do
        x_sh "$w" '
            echo "== env"; env | grep ^ERGON_ | sort
            echo "== pid 1 (started by the hook)"; grep Cpus_allowed_list /proc/1/status; echo "timerslack_ns $(cat /proc/1/timerslack_ns)"
            echo "== container cpuset"; cat /sys/fs/cgroup/cpuset.cpus.effective
            echo "== an exec shell"; grep Cpus_allowed_list /proc/self/status; echo "timerslack_ns $(cat /proc/self/timerslack_ns)"
            echo "== seccomp"; grep Seccomp /proc/self/status' >"$out/X-placement-$w.txt" 2>&1
        x_sh "$w" 'set -- $(echo "$ERGON_ISOLATED_CPUS" | tr "," " " | sed "s/-/ /"); (taskset -c "$1" sh -c "while :; do :; done" &) ; (taskset -c "${ERGON_SHARED_CPUS%%[,-]*}" sleep 30 &) ; sleep 1; ps -eLo pid,tid,psr,comm' \
            >>"$out/X-placement-$w.txt" 2>&1
        x_sh "$w" 'pkill -f "while :; do :; done"; pkill -f "sleep 30"' >/dev/null 2>&1 || true
        log "phase X $w placement: $(grep -A1 'pid 1' "$out/X-placement-$w.txt" | tail -1 | tr -s '\t ' ' '); cpuset $(sed -n '/container cpuset/{n;p;}' "$out/X-placement-$w.txt")"
    done
    # wake-up wait on the shared CPU: idle (A/A), then one spinner loading it
    for cell in "idle 50000 slack50us-A" "idle 1 slack1ns" "idle 50000 slack50us-B" "load 50000 load-slack50us" "load 1 load-slack1ns"; do
        set -- $cell
        if [[ $1 == load ]]; then
            [[ -n $(k get pod -n default ergon-lab-load-a -o name 2>/dev/null) ]] || load_on a 1 || log "WARNING: no load pod"
            log "phase X: shared CPU busy on a: $(cpu_busy a)%"
        fi
        x_sh a "taskset -c \"\$ERGON_SHARED_CPUS\" wakelat $x_seconds 1000 $2 $3" 2>>"$out/X-err.log" | tee -a "$xcsv" >&2
    done
    for cell in idle load; do
        if [[ $cell == idle ]]; then load_off; else load_on a 1; fi
        x_sh a "taskset -c \"\$ERGON_SHARED_CPUS\" pincost 10000" 2>>"$out/X-err.log" | sed "s/^pincost,/pincost-$cell,/" | tee -a "$xcsv" >&2
    done
    load_off
    # SQPOLL placement, a round trip between the nodes at 10,000/s: no io_uring
    # (A/A), SQPOLL on the shared CPU, SQPOLL on the second exclusive CPU; then
    # the shared CPU loaded on both nodes
    local cells=("busy - U0-busy-A" "sqshared - U1-sqpoll-shared" "busy - U0-busy-B" "sqshared - U1-sqpoll-shared-B"
        "busy load U0-busy-load" "sqshared load U1-sqpoll-shared-load")
    # SQPOLL on a second exclusive CPU needs a pod with two
    ((pod_cpus >= 2)) && cells+=("sqexcl - U2-sqpoll-exclusive" "sqexcl load U2-sqpoll-exclusive-load")
    for cell in "${cells[@]}"; do
        set -- $cell
        lay=$1 load=$2
        if [[ $load == load ]]; then
            for w in a b; do [[ -n $(k get pod -n default "ergon-lab-load-$w" -o name 2>/dev/null) ]] || load_on "$w" 1; done
        fi
        port=$((port + 2))
        kx -n ergon-latency-pods "$(pname b M3)" -- sh -c "$(upp_script pong "$lay" $((port + 1)) "$ip_a" "$port" $((x_seconds + 20)) "$3")" \
            >"$out/X-$3-pong.log" 2>&1 &
        pong_bg=$!
        sleep 2
        x_sh a "$(upp_script ping "$lay" "$port" "$ip_b" $((port + 1)) "$x_seconds" "$3")" 2>"$out/X-$3-ping.err" | tee -a "$xcsv" >&2
        wait "$pong_bg" 2>/dev/null
        kx -n ergon-latency-pods "$(pname b M3)" -- pkill uring-pp >/dev/null 2>&1 || true
        [[ -s $out/X-$3-ping.err ]] && log "phase X $3: $(tr '\n' ' ' <"$out/X-$3-ping.err")"
    done
    load_off
    k logs -n ergon-system -l app=ergon-shared-cpus --tail=300 --prefix >"$out/plugin-logs-X.txt" 2>&1
    k delete pod -n ergon-latency-pods -l 'app in (ergon-lab-a,ergon-lab-b)' --ignore-not-found >/dev/null 2>&1
    grep -c . "$xcsv" >/dev/null && ok=1
    ((ok)) && log "phase X: $(grep -c . "$xcsv") rows in $xcsv"
    return 0
}
if ((phase_x)); then
    phase_x_run || log "WARNING: phase X did not complete (the modes still run)"
fi

: >"$results"
log "modes: $modes, $reps reps of ${rtt_seconds}s each"
for mode in $modes; do
    log "---- mode $mode"
    ns=$(pod_ns "$mode")
    k delete pod -n "$ns" -l 'app in (ergon-lab-a,ergon-lab-b)' --ignore-not-found >/dev/null 2>&1
    for rep in $(seq "$reps"); do
        run=$mode-$rep
        ping_ep=$ip_b:$((22200 + 2 * rep))
        pong_ep=$ip_a:$((22201 + 2 * rep))
        repin_irqs
        make_pod "$mode" a "$ping_ep" "$pong_ep" >"$out/k8s/pod-a-$run.yaml"
        make_pod "$mode" b "$ping_ep" "$pong_ep" >"$out/k8s/pod-b-$run.yaml"
        if ! k apply -f "$out/k8s/pod-a-$run.yaml" >>"$out/apply.log" 2>&1 ||
            ! k apply -f "$out/k8s/pod-b-$run.yaml" >>"$out/apply.log" 2>&1 ||
            ! wait_pods_ready "$ns"; then
            k describe pod -n "$ns" -l 'app in (ergon-lab-a,ergon-lab-b)' >"$out/$run-describe.log" 2>&1
            k get events -n "$ns" --sort-by=.lastTimestamp >"$out/$run-events.log" 2>&1
            log "WARNING: $mode rep $rep: the pods did not run (see $out/$run-describe.log; the plugin may have refused a grant). Skipping the rest of $mode."
            echo "mode $mode: not measured (pods did not run; see $run-describe.log)" >>"$out/skipped.txt"
            [[ $mode == M0 ]] && die "the baseline M0 pods did not run ($out/$run-describe.log, $out/apply.log): nothing else can be compared"
            k delete pod -n "$ns" -l 'app in (ergon-lab-a,ergon-lab-b)' --ignore-not-found >/dev/null 2>&1
            break
        fi
        # The endpoints are the named nodes' IPs, so each role has to be on
        # its own node. Spreading alone does not say which pod runs where.
        for which in a b; do
            want=$node_a
            [[ $which == b ]] && want=$node_b
            got=$(k get pod -n "$ns" "$(pname "$which" "$mode")" -o jsonpath='{.spec.nodeName}' 2>/dev/null)
            if [[ $got != "$want" ]]; then
                echo "mode $mode rep $rep: pod $which on '$got', not '$want': the endpoints name the wrong process" >>"$out/skipped.txt"
                die "$mode rep $rep: role $which is on '$got', not '$want' (see $out/skipped.txt)"
            fi
        done
        # The grants have to be readable before the numbers count. An
        # extension that did not fire is a diagnostic, never a comparison.
        if ! functional_check "$mode" "$ns"; then
            echo "mode $mode: not measured (a grant is missing; see functional-$mode.txt)" >>"$out/skipped.txt"
            log "WARNING: $mode rep $rep: a grant is missing (functional-$mode.txt). Skipping the rest of $mode."
            k delete pod -n "$ns" -l 'app in (ergon-lab-a,ergon-lab-b)' --ignore-not-found >/dev/null 2>&1
            break
        fi
        loss_a_before=$(loss_now a "$ns")
        loss_b_before=$(loss_now b "$ns")
        vm_before=$(vmstat_now a "$ns")
        # xpong on b first, then xping on a; the rtt line is xping's stdout.
        # rep_cmd moves each measured task into the isolated partition and
        # pins it to a granted CPU; it prints nothing to stdout but the
        # client's own line, and nothing at all if the proof fails.
        kx -n "$ns" "$(pname b "$mode")" -- sh -c "$(rep_cmd "$mode" b "$ping_ep" "$pong_ep")" \
            >"$out/$run-pong.log" 2>&1 &
        pong_bg=$!
        sleep 3
        kx -n "$ns" "$(pname a "$mode")" -- sh -c "$(rep_cmd "$mode" a "$ping_ep" "$pong_ep")" \
            2>"$out/$run-ping-err.log" | sed "s/^/$rep,/" | tee -a "$results" >&2
        ping_rc=${PIPESTATUS[0]}
        if ((ping_rc != 0)); then
            die "$mode rep $rep: the measured client did not prove its placement (rc $ping_rc; $out/$run-ping-err.log)"
        fi
        kx -n "$ns" "$(pname b "$mode")" -- pkill -f '[r]tt xpong' >/dev/null 2>&1 || true
        wait "$pong_bg" 2>/dev/null
        # what the clients wrote, while the pods still live
        for which in a b; do
            pod=$(pod_of "$which" "$ns")
            kx -n "$ns" "$pod" -- sh -c \
                'echo "== gapspin"; cat /data/gapspin-'"$which"'.log 2>/dev/null; echo "== stalls"; cat /data/stalls-*.csv 2>/dev/null; echo "== spikes"; cat /data/load-spikes.csv 2>/dev/null' \
                >"$out/$run-$which-files.log" 2>/dev/null || true
            kx -n "$ns" "$pod" -- cat /data/load-spikes.csv >"$out/spikes-$run.csv" 2>/dev/null || true
            kx -n "$ns" "$pod" -- cat /data/stalls-ping.csv >"$out/stalls-ping-$run.csv" 2>/dev/null || true
            kx -n "$ns" "$pod" -- cat /data/stalls-pong.csv >"$out/stalls-pong-$run.csv" 2>/dev/null || true
            kx -n "$ns" "$pod" -- cat /data/gapspin-$which.log >"$out/gapspin-$which-$run.log" 2>/dev/null || true
        done
        vm_after=$(vmstat_now a "$ns")
        printf '%s\n' "$vm_before" >"$out/vmstat-before-$run.txt"
        printf '%s\n' "$vm_after" >"$out/vmstat-after-$run.txt"
        loss_row "$rep" "$mode" a "$loss_a_before" "$(loss_now a "$ns")"
        loss_row "$rep" "$mode" b "$loss_b_before" "$(loss_now b "$ns")"
        # attribute every spike and the rep's max now (the files are named
        # for the attribute helper and for spikes.py's conventions)
        python3 "$scripts/eks-nri-attribute.py" "$out/spikes-$run.csv" \
            --stalls "$out/stalls-ping-$run.csv" --stalls "$out/stalls-pong-$run.csv" \
            --gapspin "$out/gapspin-a-$run.log" --gapspin "$out/gapspin-b-$run.log" \
            --vmstat "$out/vmstat-before-$run.txt" "$out/vmstat-after-$run.txt" \
            $( [[ $mode == M5 ]] && echo "--probe-every 1" ) \
            --threshold-us 200 >"$out/$run-attribute.txt" 2>>"$out/$run-attribute.err" || true
        log "$mode rep $rep done: $(tail -1 "$out/$run-attribute.txt" 2>/dev/null)"
        k delete pod -n "$ns" "$(pname a "$mode")" "$(pname b "$mode")" --ignore-not-found >/dev/null 2>&1
        sleep 3
    done
done

# ---- report ------------------------------------------------------------------

grep -q ",rtt," "$results" 2>/dev/null || log "WARNING: no mode round trip was measured (logs: $out/*-ping.log, $out/apply.log)"
{
    python3 "$benches/analyse.py" "$results" 2>/dev/null || true
    python3 "$benches/spikes.py" "$out" 2>&1 || true
} >"$out/summary-raw.md"

# The per-mode table the harness asked for: p50, p99, p99.9, p99.99 and MAX
# with the loss counters and the max's attributed cause. The environment
# block first: everything the numbers depend on.
{
    echo "# NRI latency-pod extensions: EKS AL2023 lab"
    echo
    echo "Run $stamp: 2 x $type $market ($spot_strategy), Kubernetes $k8s, $region."
    echo "Node config: reservedSystemCPUs $reserved (0 = system/shared, 1 = NIC IRQs); isolated $isolated;"
    echo "sharedCpus $shared_cpus; hugepages $hugepages x 2 MiB a node; hostNetwork pods; HARNESS_DRIVER=invoker."
    echo "Plugin image $registry/$N/shared-cpus:$tag; workload $registry/$N/workload:$tag."
    echo
    echo "## Environment (as the nodes reported it)"
    echo
    for n in a b; do
        echo "### node $n"
        echo '```'
        cat "$out/facts/$n.txt" 2>/dev/null
        echo '```'
    done
    echo "## Results"
    echo
} >"$out/benchmarks.md"
python3 - "$results" "$out" "$modes" "$rlimits_metric" "$seccomp_metric" "$type" >>"$out/benchmarks.md" <<'PY'
import csv, os, re, sys
results, out, modes, rlim, secc, itype = sys.argv[1:7]
rows = []
loss = {}
with open(results) as f:
    for line in f:
        parts = line.strip().split(",")
        if len(parts) >= 12 and parts[1] == "rtt":
            rows.append(parts)
        elif len(parts) >= 5 and parts[1] == "loss":
            loss.setdefault(parts[2], []).append(parts[4])

def fnum(x):
    try:
        return float(x)
    except ValueError:
        return float("nan")

def cause_of(mode, rep):
    path = os.path.join(out, f"{mode}-{rep}-attribute.txt")
    try:
        for line in open(path):
            if line.startswith("MAX,"):
                return line.strip().split(",", 3)[3]
    except OSError:
        pass
    return "OPEN (no attribution file)"

def pgfaults_of(mode, rep):
    """pgfault/pgmajfault growth over the rep (plan: page faults around the run)."""
    def read(path):
        d = {}
        try:
            for line in open(path):
                k, _, v = line.strip().partition(" ")
                if k in ("pgfault", "pgmajfault"):
                    d[k] = int(v)
        except OSError:
            pass
        return d
    b = read(os.path.join(out, f"vmstat-before-{mode}-{rep}.txt"))
    a = read(os.path.join(out, f"vmstat-after-{mode}-{rep}.txt"))
    if not b or not a:
        return "?"
    return f"{a.get('pgfault', 0) - b.get('pgfault', 0)}/{a.get('pgmajfault', 0) - b.get('pgmajfault', 0)}"

print(f"Plugin grants in this build: rlimits={rlim}, seccomp={secc}.")
print("Latency figures are nanoseconds. Every max is investigated below; an unexplained one is OPEN, never dropped.")
print()
print("| mode | p50 | p90 | p99 | p99.9 | p99.99 | max | mean | samples | lost | pgfault | max cause |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|")
for mode in modes.split():
    rs = [r for r in rows if r[2] == mode]
    if not rs:
        print(f"| {mode} | — | — | — | — | — | — | — | — | — | not measured |")
        continue
    # the worst rep's max is the one to attribute in the table row
    worst = max(rs, key=lambda r: fnum(r[9]))
    lost = worst[12] if len(worst) > 12 else "?"
    faults = pgfaults_of(mode, worst[0])
    cells = [mode] + [f"{fnum(worst[i]):.1f}" for i in (4, 5, 6, 7, 8, 9, 10)] + [worst[11], lost, faults, cause_of(mode, worst[0])]
    print("| " + " | ".join(str(c) for c in cells) + " |")
print()
print("## Max investigations")
print()
for mode in modes.split():
    rs = [r for r in rows if r[2] == mode]
    for r in rs:
        print(f"- {mode} rep {r[0]}: max {fnum(r[9]):.0f} ns, {r[12] if len(r) > 12 else '?'} lost — {cause_of(mode, r[0])}")
for mode in modes.split():
    ns_loss = loss.get(mode, [])
    if ns_loss:
        print(f"- {mode} loss counters over each rep: {'; '.join(ns_loss)}")
print()
print("## What the extensions changed")
print()
print("- hugepages (M1): sized hugetlbfs at /dev/hugepages vs a plain emptyDir (M0); see functional-M1.txt and the page-fault columns.")
print("- partition (M2): cpuset.cpus.partition=isolated on the exclusive CPUs; the read-back is in functional-M2.txt.")
print("- seccomp (M3): RuntimeDefault + io_uring_*; no RTT change expected, recorded to prove no regression.")
print("- devices (M4): the NIC PHC granted read-only (ptp.ergon.io/auto); functional check plus no-regression RTT.")
print("- liveness probe (M5): the harm the admission policy removes; run in the unlabelled namespace on purpose.")
print("- all-on (M6): M1+M2+M3+M4 together.")
print()
print("## Not measured / open")
print()
print("- Whether %s exposes /dev/ptp* and is single-NUMA (see the node facts). A missing PHC skips M4 and M6; the other modes still run." % itype)
print("- The veth path (this lab runs hostNetwork).")
print("- The partition-alone latency effect (the boot-arg bundle is measured elsewhere; this lab isolates the knob).")
print("- Any max without a cause above stays OPEN, never dropped.")
print()
print("## Functional checks")
print()
for mode in modes.split():
    path = os.path.join(out, f"functional-{mode}.txt")
    if os.path.exists(path):
        print(f"- {mode}: recorded in functional-{mode}.txt")
    else:
        print(f"- {mode}: no functional file (mode skipped or pods did not run)")
if os.path.exists(os.path.join(out, "skipped.txt")):
    print()
    print("Skipped modes:")
    print()
    for line in open(os.path.join(out, "skipped.txt")):
        print(f"- {line.strip()}")
PY

{
    echo
    echo "## EKS lane (k8s/nri-shared-cpus/RELEASE.md)"
    echo
    cat "$out/eks-lane.md" 2>/dev/null || echo "not run"
    echo
    echo "## Phase X (microseconds)"
    echo
    python3 - "$out/phase-x.csv" <<'PY'
import sys
rows = [l.strip().split(",") for l in open(sys.argv[1]) if l.strip()]
us = lambda x: f"{int(x) / 1000:.1f}"
w = [r for r in rows if r[0] == "wakelat"]
if w:
    print("Wake-up wait of a 1 ms sleeper on the shared CPU (lateness past its deadline):")
    print()
    print("| cell | samples | p50 | p90 | p99 | p99.9 | p99.99 | max | mean | off shared | slack ns | CPUs |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for r in w:
        print("| " + " | ".join([r[1], r[2]] + [us(x) for x in r[3:9]] + [us(r[9]), r[10], r[11], r[12] if len(r) > 12 else ""]) + " |")
    print()
pc = [r for r in rows if r[0].startswith("pincost")]
if pc:
    print("Pin cost (getenv+parse ERGON_ISOLATED_CPUS; sched_setaffinity; call until running there):")
    print()
    print("| cell | what | samples | p50 | p90 | p99 | p99.9 | max | mean |")
    print("|---|---|---|---|---|---|---|---|---|")
    for r in pc:
        print("| " + " | ".join([r[0].split("-", 1)[1], r[1], r[2]] + [us(x) for x in r[3:9]]) + " |")
    print()
u = [r for r in rows if r[0] == "uringpp"]
if u:
    print("Kernel-UDP round trip between the nodes, 10,000/s, app thread on the first exclusive CPU:")
    print()
    print("| cell | mode | samples | p50 | p90 | p99 | p99.9 | p99.99 | max | mean | lost | nivcsw | app CPU | SQPOLL CPU |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for r in u:
        print("| " + " | ".join([r[1], r[2], r[3]] + [us(x) for x in r[4:11]] + r[11:15]) + " |")
PY
} >>"$out/benchmarks.md" 2>&1
log "round trip per mode (us):"
awk -F, '$2 == "rtt" {
        printf "  %s rep %s: p50 %.1f  p90 %.1f  p99 %.1f  p99.9 %.1f  p99.99 %.1f  MAX %.1f  mean %.1f  (%s samples, %s lost)\n", $3, $1, $5 / 1e3, $6 / 1e3, $7 / 1e3, $8 / 1e3, $9 / 1e3, $10 / 1e3, $11 / 1e3, $12, $13
    }' "$results" >&2
log "loss counters:"
grep ',loss,' "$results" | sed 's/^/  /' >&2
log "benchmarks.md: $out/benchmarks.md"
# A new file in the repo's results spot (never over a hand-written one): the
# main session reviews it and moves it into the book if it belongs there.
repo_bench=$repo/md-dpdk/benches/cloud/aws/benchmarks.md
if [[ ! -e $repo_bench ]]; then
    cp "$out/benchmarks.md" "$repo_bench" && log "wrote $repo_bench"
else
    log "$repo_bench exists: leaving it; this run's page is $out/benchmarks.md"
fi
cat "$out/benchmarks.md"
hold "$hold_minutes" "after the report"
