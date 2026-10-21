#!/usr/bin/env bash
# The DPDK media driver on Amazon EKS: a cluster and two self-managed nodes
# that exist only while this script runs, a round trip between them over their
# DPDK ENAs, then everything deleted and the deletion verified. Runs on macOS
# or Linux with the aws CLI signed in (`aws login`), kubectl, docker (buildx)
# and python3; no eksctl or helm.
#
#   md-dpdk/benches/cloud/aws/eks.sh                          2 x c7i.4xlarge, Kubernetes 1.36
#   EKS_KEEP=1 EKS_REPS=0 md-dpdk/benches/cloud/aws/eks.sh     deploy and check, then keep it for work by hand
#   EKS_DELETE=eks-<stamp> md-dpdk/benches/cloud/aws/eks.sh    delete a run (a kept one, or one whose
#                                                              teardown never ran: the laptop slept)
#
# Before anything is created: the two images (docker buildx, linux/amd64, from
# md-dpdk/deploy/aws-eks/Dockerfile.*; reused while the checkout is unchanged),
# a sweep of every region for ergon-lab leftovers, and the on-demand standard
# vCPU quota (L-1216C47A), which must hold both nodes on top of what runs.
#
# Then, in $EKS_REGION, everything tagged owner=ergon-lab and run=<stamp> and
# named ergon-lab-<stamp> (the cluster's name): two IAM roles (cluster, node)
# and the node's instance profile; a VPC (10.31.0.0/16) with an internet
# gateway and public subnets only, no NAT (the nodes reach ECR, the AMI's
# repositories and dpdk.org through their public addresses; nothing is allowed
# in from outside the VPC but this machine, to the cluster's API endpoint, and
# there is no SSH or key pair); two ECR
# repositories; the EKS cluster (control-plane logging off, API access entries,
# the caller admin through bootstrapClusterCreatorAdminPermissions); a launch
# template whose user data is node/userdata.sh's (first boot: kernel command
# line, igb_uio, the per-boot DPDK unit, one reboot; kubelet with the static
# CPU manager, its CPU quota setting left on, so every pod's CPU limit holds);
# a cluster placement group; and two $EKS_TYPE nodes in it, each
# with two more ENIs on the data subnet 10.31.1.0/24 (device 1 the driver's
# sender port, 2 its receiver: a .4/.5, b .6/.7), tagged
# node.k8s.amazonaws.com/no_manage=true so the VPC CNI leaves them alone.
#
# It deploys the device plugin and the media driver (md-dpdk/deploy/aws-eks,
# images rewritten to ECR, the driver pod on 4 whole cores: housekeeping and
# conductor, sender, receiver, and the client's), plus a privileged probe pod
# per node for host commands. Every node and pod setting the latency depends on
# is checked before a measurement (fatal unless marked WARN; lines marked info
# only report, such as vm.min_free_kbytes and the primary ENA's coalescing,
# which the node keeps at the kernel's defaults), then $EKS_REPS
# round trips (rtt ping on a, pong on b) report p50..p99.99 and MAX with the
# loss counters, and spikes.py attributes each max to a cause.
#
# Everything is deleted on any exit (EXIT, INT, TERM, HUP; a watchdog sends
# TERM after EKS_MAX_MINUTES), in dependency order, and verified by tag and by
# name for what carries no tag: the ENIs the VPC CNI and EKS leave, the
# security group EKS creates, IAM, the OIDC provider (none is created), ECR,
# CloudWatch log groups (logging is off), the launch template.
#
#   EKS_REGION (us-east-1) EKS_TYPE (c7i.4xlarge; x86, 5+ cores; *.metal too, e.g.
#   c7i.metal-24xl: igb_uio through iommu=pt, a longer join wait)
#   EKS_K8S (1.36; 1.36 or later: the kubelet sets no CPU quota on a container
#   with exclusive CPUs from 1.33, and 1.36 is the floor for latency nodes)
#   EKS_REPS (5) EKS_MAX_MINUTES (150) EKS_HUGEPAGES (2048 2 MiB pages a node)
#   EKS_POD_CPUS (4: the driver pod's CPUs, 4 physical cores; SMT is off on
#   every latency node, so one CPU is one core)
#   EKS_KEEP=1 keeps everything (delete it with EKS_DELETE=<stamp> eks.sh);
#   EKS_REPS=0 with EKS_KEEP=1 stops after the checks, for work by hand.
#   LAB_KEEP and LAB_DELETE are not read: one exported for lab.sh must never
#   keep a billing cluster.
#   RTT_MESSAGES (300000) RTT_WARMUP (50000) RTT_SECONDS (20)
set -uo pipefail
export AWS_PAGER=""

here=$(cd "$(dirname "$0")" && pwd)
repo=${EKS_REPO:-$(cd "$here/../../../.." && pwd)}
# Run from a snapshot of these scripts and the manifests: bash reads a script
# as it runs, so an edit to the checkout mid-run would otherwise break this
# run (and its teardown).
if [[ -z ${EKS_SNAPSHOT:-} ]]; then
    snapshot=$(mktemp -d)
    { cp -R "$here/.." "$snapshot/cloud" && cp -R "$repo/md-dpdk/deploy/aws-eks" "$snapshot/aws-eks"; } ||
        { echo "cannot snapshot the scripts into $snapshot" >&2; exit 1; }
    EKS_SNAPSHOT=$snapshot EKS_REPO=$repo exec bash "$snapshot/cloud/${here##*/}/${0##*/}" "$@"
fi
deploy=$EKS_SNAPSHOT/aws-eks
self=$repo/md-dpdk/benches/cloud/aws/eks.sh # for messages: the checkout's path, not the snapshot's
region=${EKS_REGION:-us-east-1}
type=${EKS_TYPE:-c7i.4xlarge}
k8s=${EKS_K8S:-1.36}
reps=${EKS_REPS:-5}
max_minutes=${EKS_MAX_MINUTES:-150}
hugepages=${EKS_HUGEPAGES:-2048}
pod_cpus=${EKS_POD_CPUS:-4}
keep=${EKS_KEEP:-}
delete=${EKS_DELETE:-}
rtt_messages=${RTT_MESSAGES:-300000}
rtt_warmup=${RTT_WARMUP:-50000}
rtt_seconds=${RTT_SECONDS:-20}
# x86 only: production runs on x86. Graviton families carry a `g` after the
# generation (c7g, c7gn, m8g, ...). Bare metal runs the same node path: its
# IOMMU passes igb_uio's DMA through (iommu=pt), as in the metal lab.
if [[ ${type%%.*} =~ ^[a-z]+[0-9]+g ]]; then
    echo "EKS_TYPE $type is a Graviton (Arm) type: the benchmarks run on x86 only" >&2
    exit 2
fi
# Kubernetes 1.36 or later. The node keeps kubelet's CPU quota setting on, so
# every pod's CPU limit holds; the driver gets no quota only because kubelet
# 1.33+ sets none on a container with exclusive CPUs. A delete runs whatever
# EKS_K8S says.
if [[ -z $delete ]] && { [[ ! $k8s =~ ^1\.([0-9]+)$ ]] || ((BASH_REMATCH[1] < 36)); }; then
    echo "EKS_K8S $k8s: want Kubernetes 1.36 or later (1.MINOR), the floor for latency nodes" >&2
    exit 2
fi
# An eks- stamp, never a lab.sh one (both tag owner=ergon-lab, run=<stamp>):
# EKS_DELETE can then only ever select an EKS run, and no wildcard reaches the
# tag filters.
stamp=${delete:-eks-$(date +%Y%m%d-%H%M%S)}
[[ $stamp =~ ^eks-[0-9]{8}-[0-9]{6}$ ]] || { echo "EKS_DELETE=$stamp: an eks.sh stamp (eks-YYYYmmdd-HHMMSS)" >&2; exit 2; }
N=ergon-lab-$stamp
out=$repo/target/md-dpdk-bench/$stamp
results=$out/eks/results.csv
# Names, so a teardown (EKS_DELETE too) finds everything without state.
cluster_role=$N-cluster
node_role=$N-node
profile=$N-node
lt=$N-node
pg=$N
repos="$N/media-driver $N/device-plugin"
cluster_policies="arn:aws:iam::aws:policy/AmazonEKSClusterPolicy"
node_policies="arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryPullOnly arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
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
# kubectl, every request bounded; exec (kx) is bounded by a `timeout` in the
# command instead, since a request timeout would cut a long exec short
k() { kubectl --request-timeout=60s "$@"; }
kx() { kubectl exec "$@"; }

# wait_for <seconds> <interval> <what> <command...>: until the command succeeds
wait_for() {
    local limit=$1 interval=$2 what=$3 deadline
    shift 3
    deadline=$((SECONDS + limit))
    until "$@"; do
        ((SECONDS >= deadline)) && { log "timed out after ${limit}s waiting for $what"; return 1; }
        sleep "$interval"
    done
}

# Per-node values, a and b: nv <name> <a|b> reads, nv_set <name> <a|b> <value> writes.
nv() { local v="$1_$2"; echo "${!v:-}"; }
nv_set() { printf -v "$1_$2" '%s' "$3"; }

for t in aws kubectl docker python3; do command -v "$t" >/dev/null || die "$t is not on PATH"; done
ident=$(aws sts get-caller-identity --query '[Account,Arn]' --output text 2>/dev/null) || die "aws is not signed in (aws login)"
read -r account caller <<<"$ident"
registry=$account.dkr.ecr.$region.amazonaws.com
mkdir -p "$out/eks"

# ---- teardown ----------------------------------------------------------------

# cluster_status: the cluster's status, NONE once it is gone, UNKNOWN if unreadable
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

# One pass over the VPC's dependants, then the VPC; succeeds once it is gone.
vpc_delete_pass() {
    local vpc=$1 id res
    for id in $(ec2 describe-internet-gateways --filters "Name=attachment.vpc-id,Values=$vpc" --query 'InternetGateways[].InternetGatewayId' --output text 2>/dev/null); do
        ec2 detach-internet-gateway --internet-gateway-id "$id" --vpc-id "$vpc" 2>/dev/null
        ec2 delete-internet-gateway --internet-gateway-id "$id" 2>/dev/null
    done
    for id in $(ec2 describe-subnets --filters "Name=vpc-id,Values=$vpc" --query 'Subnets[].SubnetId' --output text 2>/dev/null); do
        ec2 delete-subnet --subnet-id "$id" 2>/dev/null
    done
    # every route table but the main one, which goes with the VPC
    for id in $(ec2 describe-route-tables --filters "Name=vpc-id,Values=$vpc" \
        --query 'RouteTables[].[RouteTableId,length(Associations[?Main])]' --output text 2>/dev/null | awk '$2 == 0 {print $1}'); do
        ec2 delete-route-table --route-table-id "$id" 2>/dev/null
    done
    # this run's group and any EKS left (the cluster's own goes with the cluster)
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

# iam_left <role|instance-profile> <name>: prints the name if it still exists
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
    # none is created; with the cluster's issuer known, any provider for it is
    # this run's. Unknown ("-" when the cluster had none), the cluster never
    # reached ACTIVE, so no provider can name it: nothing to look for.
    [[ -n $id && $id != - && $id != None ]] || return 0
    all=$(aws iam list-open-id-connect-providers --query 'OpenIDConnectProviderList[].Arn' --output text) || return 1
    tr '\t' '\n' <<<"$all" | grep -F "/id/$id" || true
}
cluster_left() { eks list-clusters --query "clusters[?@=='$N']" --output text; }
repos_left() { ecr describe-repositories --query "repositories[?starts_with(repositoryName, '$N/')].repositoryName" --output text; }
log_groups_left() {
    aws logs describe-log-groups --region "$region" --log-group-name-prefix "/aws/eks/$N/" --query 'logGroups[].logGroupName' --output text
}

# check <what> <command...>: the command prints what is left of <what>
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
    check "EKS cluster $N" cluster_left
    check "ECR repositories $N/*" repos_left
    check "IAM role $cluster_role" iam_left role "$cluster_role"
    check "IAM role $node_role" iam_left role "$node_role"
    check "IAM instance profile $profile" iam_left instance-profile "$profile"
    check "IAM OIDC provider for $N" oidc_left
    check "CloudWatch log groups /aws/eks/$N/" log_groups_left
    # a soft check: the tagging index lags deletions by minutes
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

# Delete everything of this run, in dependency order, each step retried, then
# verify. EKS_DELETE runs this alone.
teardown_run() {
    log "deleting run $stamp ($N) in $region"
    local ids id vpc deadline res
    # 1. the nodes: their data ENIs, root volumes and the CNI's ENIs go with
    # them. A failed listing (throttling, an expired token) is retried: an
    # empty answer from a failed call would leave billing nodes behind.
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
    # 2. the cluster (it cannot be deleted while creating); its access entries,
    # its security group and its ENIs ("Amazon EKS $N") go with it
    if [[ $(cluster_status) == CREATING ]]; then
        log "cluster $N is still creating: waiting for it to finish"
        wait_for 1200 30 "cluster $N to finish creating" cluster_not_creating
    fi
    if [[ $(cluster_status) != NONE ]]; then
        # its OIDC issuer, for verify_run, when this run never recorded it
        # (stopped just after ACTIVE, or EKS_DELETE from another checkout)
        if [[ ! -s $out/oidc-id ]]; then
            mkdir -p "$out"
            id=$(eks describe-cluster --name "$N" --query 'cluster.identity.oidc.issuer' --output text 2>/dev/null)
            [[ -n $id && $id != None ]] && echo "${id##*/}" >"$out/oidc-id"
        fi
        wait_for 300 30 "delete-cluster $N to be accepted" delete_cluster_once
        wait_for 1500 30 "cluster $N to be deleted" cluster_gone && log "deleted cluster $N"
    fi
    # 3. ENIs: whatever is available in the VPC, tagged or not (the CNI's carry
    # no run tag), until none is left
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
    # 4. ECR
    for id in $repos; do
        ecr describe-repositories --repository-names "$id" >/dev/null 2>&1 &&
            ecr delete-repository --repository-name "$id" --force >/dev/null && log "deleted ECR repository $id"
    done
    docker logout "$registry" >/dev/null 2>&1
    # 5. IAM: the instance profile, then the roles
    if aws iam get-instance-profile --instance-profile-name "$profile" >/dev/null 2>&1; then
        aws iam remove-role-from-instance-profile --instance-profile-name "$profile" --role-name "$node_role" 2>/dev/null
        aws iam delete-instance-profile --instance-profile-name "$profile" && log "deleted instance profile $profile"
    fi
    iam_delete_role "$node_role"
    iam_delete_role "$cluster_role"
    # 6. the launch template
    for id in $(ec2 describe-launch-templates --filters "$(tagged)" --query 'LaunchTemplates[].LaunchTemplateId' --output text 2>/dev/null); do
        ec2 delete-launch-template --launch-template-id "$id" >/dev/null && log "deleted launch template $id"
    done
    # 7. the VPC, each pass retried on DependencyViolation for up to 10 minutes
    if [[ -n $vpc ]]; then
        deadline=$((SECONDS + 600))
        until vpc_delete_pass "$vpc"; do
            ((SECONDS >= deadline)) && { log "VPC $vpc not deleted after 10 minutes"; break; }
            sleep 30
        done
    fi
    # 8. the placement group, once its instances are gone
    if ec2 describe-placement-groups --group-names "$pg" >/dev/null 2>&1; then
        wait_for 300 30 "placement group $pg to be deleted" ec2 delete-placement-group --group-name "$pg" && log "deleted placement group $pg"
    fi
    verify_run
}

if [[ -n $delete ]]; then
    teardown_run
    exit $?
fi

# kill_tree <pid>: the process and all its descendants, with KILL
kill_tree() {
    local c
    for c in $(pgrep -P "$1"); do kill_tree "$c"; done
    kill -KILL "$1" 2>/dev/null
    return 0
}
# bounded <seconds> <command...>: the command (a function too), killed with
# all its descendants after <seconds>; macOS has no timeout(1). KILL, not
# TERM: the teardown ignores TERM, and its children inherit that.
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

# Logs from the cluster and both hosts into $out/logs, once: after the
# measurement, or from the teardown when a run fails before it. Bounded, so a
# hung API server or exec never holds the teardown back.
collect_logs() {
    [[ -n $collected || ! -s $out/kubeconfig ]] && return 0
    collected=1
    mkdir -p "$out/logs"
    log "fetching logs into $out/logs (at most 5 minutes)"
    bounded 300 fetch_logs || log "WARNING: log fetch failed or was stopped after 5 minutes (partial logs in $out/logs)"
}
fetch_logs() {
    k get nodes -o wide >"$out/logs/nodes.txt" 2>&1
    k get pods -A -o wide >"$out/logs/pods.txt" 2>&1
    k describe nodes >"$out/logs/describe-nodes.txt" 2>&1
    k describe ds,pods >"$out/logs/describe-pods.txt" 2>&1
    k get events -A --sort-by=.lastTimestamp >"$out/logs/events.txt" 2>&1
    k logs -l app=rusteron-media-driver --all-containers --prefix --tail=-1 >"$out/logs/media-driver.log" 2>&1
    k logs -l app=rusteron-dpdk-device-plugin --all-containers --prefix --tail=-1 >"$out/logs/device-plugin.log" 2>&1
    local n
    for n in a b; do
        [[ -n $(nv probe "$n") ]] || continue
        bounded 120 on_host "$n" >"$out/logs/$n-host.log" 2>&1 <<'SH'
for c in 'cat /proc/cmdline' 'uname -r' 'cat /etc/rusteron-dpdk/node.env' 'cat /var/lib/rusteron-dpdk/ena-pairs.json' \
    'journalctl -b --no-pager -u rusteron-dpdk-ena' 'journalctl -b -1 --no-pager -u cloud-final' \
    'tail -n 500 /var/log/cloud-init-output.log' 'journalctl -b --no-pager -u kubelet | tail -n 3000' 'dmesg -T | tail -n 1000'; do
    echo "== $c"
    bash -c "$c" 2>&1
done
SH
    done
}

teardown() {
    local rc=$?
    # the teardown must finish: a second Ctrl-C would leave the run billing
    trap '' INT TERM HUP
    [[ -n $watchdog_pid ]] && kill "$watchdog_pid" 2>/dev/null
    jobs -p | xargs kill 2>/dev/null
    if ((created)); then
        collect_logs
        if [[ -n $keep && ! -f $out/watchdog-fired ]]; then
            log "EKS_KEEP: run $stamp kept, and billing (the cluster, both nodes); delete it with: EKS_DELETE=$stamp $self"
            log "  KUBECONFIG=$out/kubeconfig k9s"
        else
            if ! teardown_run && ((rc == 0)); then rc=1; fi
        fi
    fi
    log "results: $out"
    exit "$rc"
}

# ---- phase 0: free, before anything is created --------------------------------

log "run $stamp: EKS cluster $N, 2 x $type, Kubernetes $k8s, $region"
log "  kubeconfig once the cluster is up: aws eks update-kubeconfig --region $region --name $N --kubeconfig $out/kubeconfig"
log "  then:                              KUBECONFIG=$out/kubeconfig k9s"
read -r vcpus cores tpc arch <<<"$(ec2 describe-instance-types --instance-types "$type" --query \
    'InstanceTypes[0].[VCpuInfo.DefaultVCpus,VCpuInfo.DefaultCores,VCpuInfo.DefaultThreadsPerCore,ProcessorInfo.SupportedArchitectures[0]]' \
    --output text 2>/dev/null)"
[[ -n ${arch:-} && $arch != None ]] || die "cannot describe $type in $region"
[[ $arch == x86_64 ]] || die "$type is $arch: the benchmarks run on x86 only"
# SMT (hyper-threading) is off on every latency node: CpuOptions
# ThreadsPerCore=1 in the launch template (node/launch-template.py) where EC2
# accepts it, and nosmt with provision-node.sh on bare metal (no CPU options
# there). One Kubernetes CPU is then one physical core, the same as on
# Graviton, and the node has $cores CPUs numbered 0..cores-1.
# provision-node.sh refuses a housekeeping CPU that will not run with SMT
# off, so other numbering stops the node joining, never a measurement.
# Threads per core the nodes get: 1 where EC2 takes the change (and where the
# type already has 1), the type's default only on bare metal.
if [[ $type == *metal* ]] || ((tpc < 2)); then
    node_tpc=$tpc
else
    node_tpc=1
fi
# a reserved core, then the driver pod's: housekeeping and conductor, sender, receiver, the client
((cores >= 5)) || die "$type has $cores cores; the node needs 5 or more (system, housekeeping, sender, receiver, app)"
((pod_cpus >= 4 && pod_cpus < cores)) ||
    die "EKS_POD_CPUS=$pod_cpus: the driver pod needs at least 4 CPUs and fewer than the node's $cores"
reserved="0"
docker info >/dev/null 2>&1 || die "docker is not running"
docker buildx version >/dev/null 2>&1 || die "docker buildx is not installed"

# Leftovers anywhere: what this account has billing under ergon-lab before
# this run adds to it. Another lab's live run only warns; an ergon-lab EKS
# cluster refuses (an earlier run of this script whose teardown never ran).
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
    if grep -q ' eks-cluster ' "$out/leftovers.txt"; then
        die "an ergon-lab EKS cluster exists (above): delete it first with EKS_DELETE=<its stamp> $self"
    fi
}
sweep_leftovers

# The on-demand standard vCPU quota (A, C, D, H, I, M, R, T, Z) must hold both
# nodes on top of what already runs in the region.
quota=$(aws service-quotas get-service-quota --region "$region" --service-code ec2 --quota-code L-1216C47A \
    --query Quota.Value --output text 2>&1) || die "cannot read the on-demand standard vCPU quota (L-1216C47A): $quota"
quota=${quota%.*}
used=$(ec2 describe-instances --filters Name=instance-state-name,Values=pending,running \
    --query 'Reservations[].Instances[].[InstanceType,CpuOptions.CoreCount,CpuOptions.ThreadsPerCore]' --output text |
    awk '$1 ~ /^([acdhmrtz]|i|im|is)[0-9]/ {s += $2 * $3} END {print s + 0}') || die "cannot count the running vCPUs"
need=$((2 * cores * node_tpc))
if ((used + need > quota)); then
    die "the on-demand standard vCPU quota (L-1216C47A) in $region is $quota, $used in use: 2 x $type needs $need more. Stop what runs or raise the quota (Service Quotas)."
fi
log "vCPU quota $quota, $used in use, this run $need"

ami=$(aws ssm get-parameter --region "$region" \
    --name "/aws/service/eks/optimized-ami/$k8s/amazon-linux-2023/x86_64/standard/recommended/image_id" \
    --query Parameter.Value --output text 2>/dev/null) || die "no EKS AL2023 x86_64 AMI for Kubernetes $k8s in $region"
[[ -f $deploy/node/userdata.sh ]] || die "no $deploy/node/userdata.sh"

# The images, tagged by commit and a hash of the uncommitted changes, built
# once per checkout state (amd64: under emulation on an Arm Mac this is slow).
commit=$(git -C "$repo" rev-parse --short=12 HEAD) || die "$repo is not a git checkout"
dirty=$(cd "$repo" && { git diff HEAD; git ls-files -o --exclude-standard; git ls-files -o --exclude-standard -z | xargs -0 cat 2>/dev/null; } | sha)
tag=$commit-$dirty
git -C "$repo" rev-parse HEAD >"$out/commit"
git -C "$repo" status --porcelain >"$out/dirty"
build_image() { # <name> <Dockerfile>
    local img=ergon-md-dpdk/$1:$tag
    if docker image inspect "$img" >/dev/null 2>&1; then
        log "image $img: built already"
        return 0
    fi
    log "building $img (linux/amd64), log $out/build-$1.log"
    docker buildx build --platform linux/amd64 --load -f "$repo/md-dpdk/deploy/aws-eks/$2" -t "$img" "$repo" \
        >"$out/build-$1.log" 2>&1 || { tail -30 "$out/build-$1.log" >&2; die "image $1 did not build"; }
}
build_image media-driver Dockerfile.media-driver
build_image device-plugin Dockerfile.device-plugin
# the driver image also serves the clients and the host probe
docker run --rm --platform linux/amd64 --entrypoint bash "ergon-md-dpdk/media-driver:$tag" -c \
    'for t in media_driver rtt counters taskset nsenter pkill timeout sleep; do command -v $t >/dev/null || { echo "missing $t"; exit 1; }; done' ||
    die "the media-driver image lacks a tool the run needs (above)"

# What this costs: on-demand Linux, from the price list.
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
node_price=$(price "$type")
hourly=$(python3 -c 'import sys
p = sys.argv[1]
print("?" if p == "?" else f"{2 * float(p) + 0.10 + 0.01:.2f}")' "$node_price")
log "creates: IAM roles and an instance profile, a VPC (public subnets, no NAT), 2 ECR repositories, an EKS"
log "  cluster (\$0.10/h), a launch template, a placement group, 2 x $type (\$$node_price/h each) with 40 GiB gp3"
log "  and public IPv4s: about \$$hourly/h, deleted at exit; at most EKS_MAX_MINUTES=$max_minutes minutes"

# ---- create ------------------------------------------------------------------

trap teardown EXIT
trap 'exit 130' INT TERM
trap 'exit 129' HUP

# The watchdog: every 15 minutes what this run has running (also to
# watch.log); after EKS_MAX_MINUTES a TERM to this script and its children, so
# the teardown runs even if a step hangs.
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

# IAM (global, so eventually consistent: the cluster and the launch retry until it has propagated)
iam_tags=(--tags Key=owner,Value=ergon-lab "Key=run,Value=$stamp")
trust() { printf '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"%s"},"Action":["sts:AssumeRole"%s]}]}' "$1" "${2:-}"; }
cluster_role_arn=$(aws iam create-role --role-name "$cluster_role" --assume-role-policy-document "$(trust eks.amazonaws.com ',"sts:TagSession"')" \
    "${iam_tags[@]}" --query Role.Arn --output text) || die "IAM role $cluster_role"
for p in $cluster_policies; do aws iam attach-role-policy --role-name "$cluster_role" --policy-arn "$p" || die "attach $p"; done
node_role_arn=$(aws iam create-role --role-name "$node_role" --assume-role-policy-document "$(trust ec2.amazonaws.com)" \
    "${iam_tags[@]}" --query Role.Arn --output text) || die "IAM role $node_role"
for p in $node_policies; do aws iam attach-role-policy --role-name "$node_role" --policy-arn "$p" || die "attach $p"; done
profile_arn=$(aws iam create-instance-profile --instance-profile-name "$profile" "${iam_tags[@]}" \
    --query InstanceProfile.Arn --output text) || die "instance profile $profile"
aws iam add-role-to-instance-profile --instance-profile-name "$profile" --role-name "$node_role" || die "instance profile role"

# Network: public subnets only; DNS on (EKS needs both attributes).
vpc=$(ec2 create-vpc --cidr-block 10.31.0.0/16 --tag-specifications "$(tags vpc vpc)" --query Vpc.VpcId --output text) || die "vpc"
ec2 modify-vpc-attribute --vpc-id "$vpc" --enable-dns-support '{"Value":true}' || die "vpc dns support"
ec2 modify-vpc-attribute --vpc-id "$vpc" --enable-dns-hostnames '{"Value":true}' || die "vpc dns hostnames"
igw=$(ec2 create-internet-gateway --tag-specifications "$(tags internet-gateway igw)" \
    --query InternetGateway.InternetGatewayId --output text) || die "igw"
ec2 attach-internet-gateway --internet-gateway-id "$igw" --vpc-id "$vpc" || die "igw attach"
rt=$(ec2 create-route-table --vpc-id "$vpc" --tag-specifications "$(tags route-table rt)" --query RouteTable.RouteTableId --output text) || die "rt"
ec2 create-route --route-table-id "$rt" --destination-cidr-block 0.0.0.0/0 --gateway-id "$igw" >/dev/null || die "route"
sg=$(ec2 create-security-group --vpc-id "$vpc" --group-name "$N-nodes" --description "ergon md-dpdk EKS $stamp" \
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
ec2 create-placement-group --group-name "$pg" --strategy cluster --tag-specifications "$(tags placement-group pg)" >/dev/null ||
    die "placement group"
for r in $repos; do
    ecr create-repository --repository-name "$r" --tags Key=owner,Value=ergon-lab "Key=run,Value=$stamp" >/dev/null || die "ECR $r"
done

# The cluster: API access entries only, the caller admin, control-plane logging off.
create_cluster() {
    case $(cluster_status) in
    NONE) ;;
    UNKNOWN) return 1 ;;
    *) return 0 ;;
    esac
    # the public API endpoint answers this machine only; the nodes use the private one
    local me
    me=$(curl -fsS -m 10 https://checkip.amazonaws.com | tr -d '[:space:]') || return 1
    [[ $me =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] || { log "cannot tell this machine's public IPv4 ($me)"; return 1; }
    eks create-cluster --name "$N" --kubernetes-version "$k8s" --role-arn "$cluster_role_arn" \
        --resources-vpc-config "{\"subnetIds\":[$cluster_subnets],\"endpointPublicAccess\":true,\"publicAccessCidrs\":[\"$me/32\"],\"endpointPrivateAccess\":true}" \
        --access-config authenticationMode=API,bootstrapClusterCreatorAdminPermissions=true \
        --upgrade-policy supportType=STANDARD \
        --logging '{"clusterLogging":[{"types":["api","audit","authenticator","controllerManager","scheduler"],"enabled":false}]}' \
        --tags "owner=ergon-lab,run=$stamp" >/dev/null 2>>"$out/create-cluster.log"
}
wait_for 180 15 "create-cluster to accept the new role" create_cluster || { tail -3 "$out/create-cluster.log" >&2; die "create-cluster"; }
log "cluster $N creating; pushing the images meanwhile"
aws ecr get-login-password --region "$region" | docker login --username AWS --password-stdin "$registry" >/dev/null 2>&1 ||
    die "docker login to $registry"
for name in media-driver device-plugin; do
    docker tag "ergon-md-dpdk/$name:$tag" "$registry/$N/$name:$tag" || die "docker tag $name"
    docker push "$registry/$N/$name:$tag" >>"$out/push.log" 2>&1 || { tail -5 "$out/push.log" >&2; die "push $name"; }
done
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

# The nodes: user data from node/userdata.sh, in a launch template.
RUSTERON_DPDK_DEVICE_NUMBERS="1 2" RUSTERON_HUGEPAGES=$hugepages \
    bash "$deploy/node/userdata.sh" "$N" "$endpoint" "$ca" "$svc" "$reserved" >"$out/userdata.gz" ||
    die "node/userdata.sh failed (over the 16 KiB user-data limit?)"
# The launch template: CpuOptions ThreadsPerCore=1 on x86 where EC2 accepts
# it (node/launch-template.py turns SMT off, so one CPU is one physical core).
python3 "$deploy/node/launch-template.py" "$ami" "$type" "$profile_arn" "$out/userdata.gz" "$stamp" "$N" \
    "$vcpus" "$cores" "$tpc" "$arch" >"$out/launch-template.json" || die "launch template data"
grep -q '"ThreadsPerCore": 1' "$out/launch-template.json" &&
    log "launch template: CpuOptions CoreCount=$cores ThreadsPerCore=1 (SMT off)" ||
    log "launch template: no CpuOptions ($type takes none: SMT is off already or bare metal turns it off at boot)"
ec2 create-launch-template --launch-template-name "$lt" --launch-template-data "file://$out/launch-template.json" \
    --tag-specifications "$(tags launch-template lt)" >/dev/null || die "launch template"

# Both nodes in one zone, in one request (the placement group's capacity is
# all or nothing), trying each zone that offers the type.
launch() { # <node subnet> <zone>: both instance ids
    local try res
    for try in $(seq 12); do
        res=$(ec2 run-instances --launch-template "LaunchTemplateName=$lt,Version=1" --count 2 --subnet-id "$1" \
            --security-group-ids "$csg" "$sg" --placement "GroupName=$pg" \
            --query 'Instances[].InstanceId' --output text 2>&1) && { echo "$res"; return 0; }
        echo "$res" >>"$out/run-instances-$2.log"
        # a new instance profile takes a while to be usable; anything else: the next zone
        [[ $res == *[Ii]nstance[Pp]rofile* ]] || return 1
        sleep 10
    done
    return 1
}
zones=$(ec2 describe-instance-type-offerings --location-type availability-zone --filters "Name=instance-type,Values=$type" \
    --query 'InstanceTypeOfferings[].Location' --output text | tr '\t' '\n' | sort)
[[ -n $zones ]] || die "$type is not offered in $region"
launched=
for zone in $zones; do
    nodes_subnet=$(ec2 create-subnet --vpc-id "$vpc" --cidr-block 10.31.0.0/24 --availability-zone "$zone" \
        --tag-specifications "$(tags subnet nodes)" --query Subnet.SubnetId --output text) || die "node subnet"
    ec2 modify-subnet-attribute --subnet-id "$nodes_subnet" --map-public-ip-on-launch || die "node subnet public IPs"
    ec2 associate-route-table --route-table-id "$rt" --subnet-id "$nodes_subnet" >/dev/null || die "node subnet route"
    data_subnet=$(ec2 create-subnet --vpc-id "$vpc" --cidr-block 10.31.1.0/24 --availability-zone "$zone" \
        --tag-specifications "$(tags subnet data)" --query Subnet.SubnetId --output text) || die "data subnet"
    if ids=$(launch "$nodes_subnet" "$zone"); then
        launched=$zone
        break
    fi
    log "no capacity for 2 x $type in $zone: $(tail -1 "$out/run-instances-$zone.log" 2>/dev/null)"
    ec2 delete-subnet --subnet-id "$nodes_subnet"
    ec2 delete-subnet --subnet-id "$data_subnet"
done
[[ -n $launched ]] || die "no zone had capacity for 2 x $type"
read -r ida idb <<<"$ids"
log "nodes $ida (a) $idb (b) in $launched; waiting for them to run"
ec2 wait instance-running --instance-ids "$ida" "$idb" || die "the nodes did not start"

# Two data ENIs each, at device indexes 1 (sender) and 2 (receiver), deleted
# with the instance, tagged so the VPC CNI neither configures nor uses them.
# provision-node.sh waits for them before the node's first kubelet start.
attach() { # instance device-index ip
    local eni att
    eni=$(ec2 create-network-interface --subnet-id "$data_subnet" --groups "$sg" --private-ip-address "$3" \
        --tag-specifications "ResourceType=network-interface,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$stamp},{Key=Name,Value=$N-data-eni},{Key=node.k8s.amazonaws.com/no_manage,Value=true}]" \
        --query NetworkInterface.NetworkInterfaceId --output text) || return 1
    att=$(ec2 attach-network-interface --network-interface-id "$eni" --instance-id "$1" --device-index "$2" \
        --query AttachmentId --output text) || return 1
    ec2 modify-network-interface-attribute --network-interface-id "$eni" \
        --attachment "AttachmentId=$att,DeleteOnTermination=true" || return 1
}
attach "$ida" 1 10.31.1.4 && attach "$ida" 2 10.31.1.5 && attach "$idb" 1 10.31.1.6 && attach "$idb" 2 10.31.1.7 ||
    die "data ENIs"
ip_snd_a=10.31.1.4 ip_rcv_a=10.31.1.5 ip_snd_b=10.31.1.6 ip_rcv_b=10.31.1.7
{
    echo "cluster $N kubernetes $k8s"
    echo "type $type zone $launched placement $pg"
    echo "ami $ami"
    echo "images $registry/$N/{media-driver,device-plugin}:$tag"
    echo "a $ida sender $ip_snd_a receiver $ip_rcv_a"
    echo "b $idb sender $ip_snd_b receiver $ip_rcv_b"
} >"$out/instances.txt"

# First boot provisions and reboots once, then the node joins. (CoreDNS stays
# Pending: both nodes carry the DPDK taint, and nothing here needs cluster DNS.)
nodes_ready() {
    local n
    n=$(k get nodes -l rusteron-dpdk/ena=true \
        -o jsonpath='{range .items[*]}{range .status.conditions[?(@.type=="Ready")]}{.status}{"\n"}{end}{end}' 2>/dev/null | grep -c '^True$')
    ((n == 2))
}
# Bare metal boots, and so reboots, for several minutes more than a VM (the
# metal lab allowed 10 minutes a reboot).
join_minutes=25
[[ $type == *metal* ]] && join_minutes=45
log "waiting for both nodes to provision, reboot and join (up to $join_minutes minutes)"
if ! wait_for $((join_minutes * 60)) 30 "2 Ready nodes labelled rusteron-dpdk/ena=true" nodes_ready; then
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

# The repo's manifests as the base (the lab's mbuf count included); on top:
# the images in ECR, the driver pod on 4 whole cores (housekeeping and
# conductor, sender, receiver, and the client's), and a privileged probe per
# node for host commands (hostPID, nsenter into PID 1's namespaces).
mkdir -p "$out/k8s"
cp -R "$deploy" "$out/k8s/base"
cat >"$out/k8s/probe.yaml" <<'YAML'
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: ergon-lab-probe
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
        rusteron-dpdk/ena: "true"
      tolerations:
        - key: rusteron-dpdk
          operator: Exists
          effect: NoSchedule
      hostPID: true
      terminationGracePeriodSeconds: 1
      containers:
        - name: probe
          image: rusteron-media-driver
          command: ["sleep", "infinity"]
          securityContext:
            privileged: true
YAML
cat >"$out/k8s/kustomization.yaml" <<YAML
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:
  - base
  - probe.yaml
images:
  - name: rusteron-media-driver
    newName: $registry/$N/media-driver
    newTag: "$tag"
  - name: rusteron-dpdk-device-plugin
    newName: $registry/$N/device-plugin
    newTag: "$tag"
patches:
  - target:
      kind: DaemonSet
      name: rusteron-media-driver
    patch: |-
      - op: replace
        path: /spec/template/spec/containers/0/resources/requests/cpu
        value: "$pod_cpus"
      - op: replace
        path: /spec/template/spec/containers/0/resources/limits/cpu
        value: "$pod_cpus"
YAML
k apply -k "$out/k8s" >"$out/apply.log" 2>&1 || { cat "$out/apply.log" >&2; die "kubectl apply -k $out/k8s"; }
for ds in rusteron-dpdk-device-plugin rusteron-media-driver ergon-lab-probe; do
    kubectl rollout status "ds/$ds" --timeout=10m >>"$out/apply.log" 2>&1 || die "daemonset $ds did not roll out (logs follow in $out/logs)"
done
pod_on() { k get pods -l "app=$1" --field-selector "spec.nodeName=$2" -o jsonpath='{.items[0].metadata.name}'; }
for n in a b; do
    node=$(nv node "$n")
    nv_set drv "$n" "$(pod_on rusteron-media-driver "$node")"
    nv_set probe "$n" "$(pod_on ergon-lab-probe "$node")"
    [[ -n $(nv drv "$n") && -n $(nv probe "$n") ]] || die "$n: no driver or probe pod on $node"
done

in_driver() { local n=$1; shift; kx "$(nv drv "$n")" -- "$@"; }
# driver_cpus <a|b>: the CPUs a process in the driver container may run on
# (Cpus_allowed_list, the container's cpuset: what the entrypoint reads).
driver_cpus() { in_driver "$1" awk '$1 == "Cpus_allowed_list:" {print $2}' /proc/self/status; }
# driver_cgroup <a|b> <file>: <file> of the driver container's own cgroup.
# Not /sys/fs/cgroup/<file>: containerd gives a privileged container (the
# driver) no cgroup namespace, so that is the host's root cgroup, with every
# online CPU and no cpu.max. /proc/self/cgroup names the container's cgroup
# relative to the mount's root, with a namespace ("/") or without one.
# Run as: bash -c "$cgroup_file_sh" _ <proc cgroup> <cgroup mount> <file>
cgroup_file_sh='p=; while IFS= read -r l; do [[ $l == 0::* ]] && p=${l#0::}; done <"$1"; [[ -n $p ]] || { echo "no cgroup v2 entry in $1" >&2; exit 1; }; cat "$2${p%/}/$3"'
driver_cgroup() { in_driver "$1" bash -c "$cgroup_file_sh" _ /proc/self/cgroup /sys/fs/cgroup "$2"; }
# on_host <a|b> [args]: the script on stdin, as root in the host's namespaces
on_host() { local n=$1; shift; kx -i "$(nv probe "$n")" -- nsenter -t 1 -m -u -i -n -p -- timeout 600 bash -s -- "$@"; }

# The dead-man's switch: each node powers off, and so terminates with its
# disk (InstanceInitiatedShutdownBehavior), 30 minutes after the watchdog's
# limit, should this machine never delete it (asleep, offline). The cluster
# itself ($0.10/h) waits for EKS_DELETE.
for n in a b; do
    on_host "$n" <<<"shutdown -h +$((max_minutes + 30)) 'ergon-lab deadline'" >/dev/null 2>&1 ||
        log "WARNING: $n: no shutdown deadline; only this machine's teardown deletes it"
done

driver_started() { k logs "$(nv drv "$1")" 2>/dev/null | grep -q "media driver started"; }
field() { sed -n "s/.* $1=\([^ ]*\).*/\1/p" <<<"$2" | head -1; }
for n in a b; do
    wait_for 120 5 "$n: media driver started" driver_started "$n" ||
        { k logs "$(nv drv "$n")" >"$out/$n-driver.log" 2>&1; tail -20 "$out/$n-driver.log" >&2; die "$n: the media driver did not start"; }
    k logs "$(nv drv "$n")" >"$out/$n-driver.log" 2>&1
    grep "transport backend" "$out/$n-driver.log" >&2 || die "$n: no transport backend line in the driver log"
    # the entrypoint's layout: housekeeping (and conductor), sender, receiver, spare cores (the client's)
    line=$(grep -m1 'rusteron entrypoint:' "$out/$n-driver.log")
    nv_set hk "$n" "$(field housekeeping "$line")"
    nv_set snd "$n" "$(field sender "$line")"
    nv_set rcv "$n" "$(field receiver "$line")"
    nv_set app "$n" "$(field spare "$line" | tr ',' ' ' | awk '{print $1}')"
    nv_set cpuset "$n" "$(field cpuset "$line")"
    [[ -n $(nv hk "$n") && -n $(nv snd "$n") && -n $(nv rcv "$n") ]] || die "$n: cannot read the entrypoint's layout: $line"
    [[ -n $(nv app "$n") ]] || die "$n: the driver pod has no spare core for the client ($line)"
    log "$n: housekeeping $(nv hk "$n"), sender $(nv snd "$n"), receiver $(nv rcv "$n"), client $(nv app "$n") (cpuset $(nv cpuset "$n"))"
done
aeron_dir=$(in_driver a printenv AERON_DIR) || die "no AERON_DIR in the driver pod's environment"

# ---- checks ------------------------------------------------------------------

# On the host: what provisioning and the per-boot unit set, and write-combining
# on both data BARs (while the driver has them mapped). Prints ok/WARN/FAIL lines.
host_checks() {
    cat <<'SH'
hk=$1
shift
bdfs=$*
ok() { echo "ok   $*"; }
bad() { echo "FAIL $*"; }
warn() { echo "WARN $*"; }
info() { echo "info $*"; }
expand() {
    local r c
    for r in ${1//,/ }; do
        if [[ $r == *-* ]]; then for ((c = ${r%-*}; c <= ${r#*-}; c++)); do echo $c; done; else echo $r; fi
    done | sort -n | tr '\n' ' '
}
hkset=" $(expand "$hk")"
iso=
for c in $(expand "$(cat /sys/devices/system/cpu/online)"); do [[ $hkset == *" $c "* ]] || iso+="$c "; done
cmdline=" $(cat /proc/cmdline) "
# SMT (hyper-threading) off: one Kubernetes CPU is one physical core, the same
# as on Graviton (CpuOptions ThreadsPerCore=1 in the launch template; nosmt
# and provision-node.sh on bare metal).
v=$(cat /sys/devices/system/cpu/smt/active 2>/dev/null || echo 0)
if [[ $v == 0 ]]; then ok "SMT off (smt/active=0)"; else bad "smt/active=$v (want 0: one CPU is one physical core)"; fi
# no C-state cap on any node (provision-node.sh): no committed A/B showed a gain
args="nosmt isolcpus= nohz_full= rcu_nocbs= irqaffinity= iommu=pt transparent_hugepage=never hugepagesz=2M hugepages="
for arg in $args; do
    if [[ $cmdline == *" $arg"* ]]; then ok "cmdline has $arg"; else bad "cmdline lacks $arg"; fi
done
for arg in intel_idle.max_cstate= processor.max_cstate=; do
    if [[ $cmdline == *" $arg"* ]]; then bad "cmdline has $arg (provision-node.sh sets no C-state cap)"; else ok "no $arg"; fi
done
for f in isolated nohz_full; do
    v=$(cat /sys/devices/system/cpu/$f 2>/dev/null)
    if [[ -n $v && $(expand "$v") == "$iso" ]]; then ok "$f $v"; else bad "$f is '$v', not the CPUs $iso(empty nohz_full: no CONFIG_NO_HZ_FULL)"; fi
done
names=$(cat /sys/devices/system/cpu/cpu0/cpuidle/state*/name 2>/dev/null | tr '\n' ' ')
info "no C-state cap: cpuidle states ${names:-none} ($(cat /sys/devices/system/cpu/cpuidle/current_driver 2>/dev/null))"
# igb_uio programs no IOMMU: on metal every group must be in passthrough
groups=$(find /sys/kernel/iommu_groups -mindepth 1 -maxdepth 1 2>/dev/null | wc -l)
if ((groups == 0)); then
    ok "no IOMMU groups"
else
    translating=$(for g in /sys/kernel/iommu_groups/*/; do [[ $(cat "${g}type" 2>/dev/null) == identity ]] || echo "${g%/}"; done | wc -l)
    if ((translating == 0)); then ok "IOMMU passthrough: all $groups groups identity"; else bad "$translating of $groups IOMMU groups translate (igb_uio needs iommu=pt)"; fi
fi
v=$(cat /sys/devices/system/clocksource/clocksource0/current_clocksource)
if [[ $v == tsc ]]; then ok "clocksource tsc"; else bad "clocksource $v"; fi
v=$(cat /sys/kernel/mm/transparent_hugepage/enabled)
if [[ $v == *"[never]"* ]]; then ok "THP never"; else bad "THP $v"; fi
v=$(cat /sys/kernel/mm/ksm/run 2>/dev/null || echo 0)
if [[ $v == 0 ]]; then ok "ksm off"; else bad "ksm run=$v"; fi
v=$(cat /proc/sys/kernel/numa_balancing 2>/dev/null || echo 0)
if [[ $v == 0 ]]; then ok "numa_balancing off"; else bad "numa_balancing $v"; fi
# reported, not judged: the kernel's reserve unless RUSTERON_MIN_FREE_PCT is set
mem=$(awk '$1 == "MemTotal:" {print $2}' /proc/meminfo)
info "min_free_kbytes $(cat /proc/sys/vm/min_free_kbytes) of $mem kB; busy_poll $(cat /proc/sys/net/core/busy_poll) busy_read $(cat /proc/sys/net/core/busy_read)"
# every IRQ on housekeeping: one elsewhere that can be moved, bootstrap missed;
# one that cannot is kernel-managed (isolcpus=managed_irq keeps it off when it can)
for d in /proc/irq/[0-9]*; do
    v=$(cat $d/smp_affinity_list 2>/dev/null) || continue
    out=
    for c in $(expand "$v"); do [[ $hkset == *" $c "* ]] || out=1; done
    [[ -z $out ]] && continue
    if { echo "$hk" >$d/smp_affinity_list; } 2>/dev/null; then bad "irq ${d##*/} was on $v, not housekeeping $hk"; else warn "irq ${d##*/} on $v is kernel-managed"; fi
done
ok "IRQ affinity checked"
# reported, not judged: the primary ENA keeps its driver's coalescing
primary=$(ip -o -4 route show default | awk '{print $5; exit}')
co=$(ethtool -c "$primary" 2>/dev/null)
info "$primary coalescing: $(grep -E '^(Adaptive|rx-usecs|tx-usecs)' <<<"$co" | tr '\n\t' '  ')"
mountpoint -q /sys/kernel/debug || mount -t debugfs none /sys/kernel/debug 2>/dev/null
for bdf in $bdfs; do
    drv=$(basename "$(readlink /sys/bus/pci/devices/$bdf/driver 2>/dev/null)")
    if [[ $drv == igb_uio ]]; then ok "$bdf on igb_uio"; else bad "$bdf on '${drv}', not igb_uio"; fi
    start=$(sed -n 3p /sys/bus/pci/devices/$bdf/resource | awk '{print $1}')
    if grep -F "[mem $start-" /sys/kernel/debug/x86/pat_memtype_list 2>/dev/null | grep -q write-combining; then
        ok "$bdf BAR2 $start write-combining"
    else
        bad "$bdf BAR2 $start not write-combining ($(grep -F "[mem $start-" /sys/kernel/debug/x86/pat_memtype_list 2>&1 | head -1))"
    fi
done
echo "done host checks"
SH
}

# Thread placement: "<tid>|<comm>|<allowed CPUs>" of each media driver thread.
driver_threads() {
    on_host "$1" <<'SH'
pid=$(pgrep -x media_driver | head -1)
[[ -n $pid ]] || exit 1
for t in /proc/$pid/task/*; do
    echo "${t##*/}|$(cat $t/comm)|$(awk '$1 == "Cpus_allowed_list:" {print $2}' $t/status)"
done
SH
}

# pod_cpu_max <a|b> <pod> <container>: "pod <cpu.max>" and "container <cpu.max>"
# of the pod's cgroup and the container's, read on the host (a container with
# its own cgroup namespace cannot see its pod's cgroup). The systemd cgroup
# driver: kubepods.slice/kubepods[-<qos>]-pod<uid, _ for ->.slice, and in it
# cri-containerd-<id>.scope.
pod_cpu_max() {
    local uid id
    uid=$(k get pod "$2" -o jsonpath='{.metadata.uid}')
    id=$(k get pod "$2" -o jsonpath="{.status.containerStatuses[?(@.name==\"$3\")].containerID}")
    [[ -n $uid && -n $id ]] || { echo "no uid or container id for $2/$3"; return 1; }
    on_host "$1" "$uid" "${id#containerd://}" <<'SH'
pod=$(find /sys/fs/cgroup/kubepods.slice -maxdepth 2 -type d -name "kubepods*-pod${1//-/_}.slice" | head -1)
[[ -n $pod ]] || { echo "no cgroup for pod $1 under /sys/fs/cgroup/kubepods.slice"; exit 1; }
echo "pod $(cat "$pod/cpu.max")"
echo "container $(cat "$pod/cri-containerd-$2.scope/cpu.max")"
SH
}

# Kubelet, node and pod: the CPU manager's settings, what the node offers, and
# where the driver's threads run. The CPU quota: kubelet's setting stays on, so
# every pod's CPU limit holds (the device plugin's 100m limit is the witness),
# and the driver, a Guaranteed pod on whole CPUs, gets none because kubelet
# 1.33+ sets none on a container with exclusive CPUs, nor on its pod
# (DisableCPUQuotaWithExclusiveCPUs). Prints ok/FAIL lines.
cluster_checks() {
    local n=$1 node plugin
    node=$(nv node "$n")
    plugin=$(pod_on rusteron-dpdk-device-plugin "$node")
    k get --raw "/api/v1/nodes/$node/proxy/configz" >"$out/$n-configz.json" 2>&1
    k get --raw "/api/v1/nodes/$node/proxy/metrics" >"$out/$n-kubelet-metrics.txt" 2>&1 ||
        echo "# kubectl get --raw .../metrics failed" >>"$out/$n-kubelet-metrics.txt"
    k get node "$node" -o json >"$out/$n-node.json" 2>&1
    k get pod "$(nv drv "$n")" -o json >"$out/$n-driver-pod.json" 2>&1
    k get pod "$plugin" -o json >"$out/$n-plugin-pod.json" 2>&1
    driver_cpus "$n" >"$out/$n-cpuset.txt" 2>&1
    driver_cgroup "$n" cpu.max >"$out/$n-cpu.max" 2>&1
    pod_cpu_max "$n" "$(nv drv "$n")" media-driver >"$out/$n-driver-cgroups.txt" 2>&1
    pod_cpu_max "$n" "$plugin" device-plugin >"$out/$n-plugin-cgroups.txt" 2>&1
    driver_threads "$n" >"$out/$n-threads.txt" 2>&1
    python3 - "$out" "$n" "$reserved" "$((cores - 1))" "$hugepages" "$cores" "$pod_cpus" \
        "$(nv hk "$n")" "$(nv snd "$n")" "$(nv rcv "$n")" <<'PY'
import json, os, re, sys
out, n, reserved, cpus, pages, cores, pod_cpus, hk, snd, rcv = sys.argv[1:11]
cores, pod_cpus = int(cores), int(pod_cpus)
def check(good, what): print(("ok   " if good else "FAIL ") + what)
def cpuset(s):
    got = set()
    for part in s.strip().split(","):
        if part:
            lo, _, hi = part.partition("-")
            got.update(range(int(lo), int(hi or lo) + 1))
    return got
def mib(q):
    for unit, scale in (("Ki", 1 / 1024), ("Mi", 1), ("Gi", 1024)):
        if q.endswith(unit):
            return float(q[:-2]) * scale
    return float(q or 0) / 2**20
def load(name):
    try:
        return json.load(open(os.path.join(out, f"{n}-{name}")))
    except (OSError, ValueError) as e:
        check(False, f"{name}: {e}")
        return None
cz = load("configz.json")
if cz:
    k = cz["kubeletconfig"]
    check(k.get("cpuManagerPolicy") == "static", f"kubelet cpuManagerPolicy {k.get('cpuManagerPolicy')}")
    opts = k.get("cpuManagerPolicyOptions") or {}
    check(opts.get("full-pcpus-only") == "true", f"kubelet cpuManagerPolicyOptions {opts}")
    check(cpuset(k.get("reservedSystemCPUs", "")) == cpuset(reserved),
          f"kubelet reservedSystemCPUs {k.get('reservedSystemCPUs')!r} (want {reserved})")
    check(k.get("topologyManagerPolicy") == "single-numa-node", f"kubelet topologyManagerPolicy {k.get('topologyManagerPolicy')}")
    check(k.get("cpuCFSQuota", True) is True,
          f"kubelet cpuCFSQuota {k.get('cpuCFSQuota')} (want true or absent, the default: false removes every pod's CPU limit)")
    check(not k.get("kubeReservedCgroup") and not k.get("systemReservedCgroup"),
          f"kubelet reserved cgroups {k.get('kubeReservedCgroup')!r} {k.get('systemReservedCgroup')!r} (want none)")
node = load("node.json")
ver = None
if node:
    kv = node["status"].get("nodeInfo", {}).get("kubeletVersion", "")
    m = re.match(r"v?(\d+)\.(\d+)", kv)
    ver = (int(m[1]), int(m[2])) if m else None
    check(ver is not None and ver >= (1, 36), f"kubelet version {kv!r} (want 1.36 or later)")
    a = node["status"]["allocatable"]
    check(a.get("cpu") == cpus, f"allocatable cpu {a.get('cpu')} (want {cpus})")
    check(mib(a.get("hugepages-2Mi", "0")) == 2 * int(pages), f"allocatable hugepages-2Mi {a.get('hugepages-2Mi')} (want {2 * int(pages)}Mi)")
    check(a.get("rusteron.io/dpdk-ena-pair") == "1", f"allocatable rusteron.io/dpdk-ena-pair {a.get('rusteron.io/dpdk-ena-pair')}")
# DisableCPUQuotaWithExclusiveCPUs: its kubernetes_feature_enabled line where
# kubelet has one (1 = on), else the version (on by default from 1.33, locked
# on in 1.37, removed with the behaviour kept in 1.38).
gate = "DisableCPUQuotaWithExclusiveCPUs"
metrics = open(os.path.join(out, f"{n}-kubelet-metrics.txt")).read()
lines = [l for l in metrics.splitlines() if l.startswith("kubernetes_feature_enabled{") and f'name="{gate}"' in l]
if "# kubectl get --raw" in metrics:
    check(False, f"kubelet metrics unreadable ({n}-kubelet-metrics.txt)")
elif lines:
    check(lines[0].split()[-1] in ("1", "1.0"), f"kubelet feature gate {gate}: {lines[0]} (want 1)")
else:
    check(ver is not None and ver >= (1, 33), f"kubelet feature gate {gate}: no metric line, kubelet {ver} (on from 1.33)")
pod = load("driver-pod.json")
if pod:
    check(pod["status"].get("qosClass") == "Guaranteed", f"driver pod QoS {pod['status'].get('qosClass')}")
    lim = [c["resources"]["limits"].get("cpu", "") for c in pod["spec"]["containers"] if c["name"] == "media-driver"]
    check(lim == [str(pod_cpus)], f"driver CPU limit {lim} (want a whole {pod_cpus}: exclusive CPUs)")
def cgroups(name):
    got = {}
    for l in open(os.path.join(out, f"{n}-{name}-cgroups.txt")):
        w = l.split()
        if len(w) >= 2 and w[0] in ("pod", "container"):
            got[w[0]] = w[1:]
    return got
def milli(q):
    return int(q[:-1]) if q.endswith("m") else int(float(q) * 1000)
cg = cgroups("driver")
check(cg.get("pod", [""])[0] == "max" and cg.get("container", [""])[0] == "max",
      f"driver cgroups on the host: pod cpu.max {cg.get('pod')}, container {cg.get('container')} (want max: exclusive CPUs, no quota)")
# The witness: a pod with a fractional CPU limit on the same node keeps its quota.
plugin = load("plugin-pod.json")
if plugin:
    lim = [c["resources"].get("limits", {}).get("cpu") for c in plugin["spec"]["containers"] if c["name"] == "device-plugin"]
    cg = cgroups("plugin")
    period = cg.get("container", [])[1:2]
    want = str(milli(lim[0]) * int(period[0]) // 1000) if lim and lim[0] and period and period[0].isdigit() else None
    check(want is not None and cg.get("container", [""])[0] == want and cg.get("pod", [""])[0] == want,
          f"device plugin (CPU limit {lim}) cgroups on the host: pod cpu.max {cg.get('pod')}, container {cg.get('container')} "
          f"(want quota {want}: a pod's CPU limit holds)")
cs = cpuset(open(os.path.join(out, f"{n}-cpuset.txt")).read())
# SMT is off: every CPU is one physical core, so whole cores come for free.
# The pod needs its CPUs, under the node's cores, and none of the reserved one.
whole = (len(cs) == pod_cpus and not cs & cpuset(reserved)
         and all(0 <= c < cores for c in cs))
check(whole, f"driver cpuset {sorted(cs)}: {pod_cpus} CPUs under {cores}, none reserved")
cpumax = open(os.path.join(out, f"{n}-cpu.max")).read().split()
check(cpumax[:1] == ["max"], f"driver cpu.max {' '.join(cpumax)} (no CFS quota)")
threads = [l.strip().split("|") for l in open(os.path.join(out, f"{n}-threads.txt")) if l.count("|") == 2]
check(bool(threads), "driver threads readable")
agents = {"sender": snd, "receiver": rcv}
for role, cpu in agents.items():
    mine = [t for t in threads if t[1].startswith(role)]
    check(len(mine) == 1 and cpuset(mine[0][2]) == {int(cpu)}, f"{role} thread alone on CPU {cpu}: {mine}")
others = [t for t in threads if not t[1].startswith(tuple(agents))]
stray = [t for t in others if cpuset(t[2]) != {int(hk)}]
check(not stray, f"{len(others)} other driver threads on housekeeping CPU {hk}" + (f"; not: {stray}" if stray else ""))
print("done cluster checks")
PY
}

failed=
for n in a b; do
    on_host "$n" >"$out/$n-pairs.json" 2>&1 <<<'cat /var/lib/rusteron-dpdk/ena-pairs.json'
    bdfs=$(python3 -c '
import json, sys
p = json.load(open(sys.argv[1]))["pairs"][0]
print(p["sender"]["pci"], p["receiver"]["pci"])' "$out/$n-pairs.json" 2>/dev/null) || { log "$n: ena-pairs.json is not valid"; failed+=" $n"; }
    {
        [[ -n $bdfs ]] && echo "ok   ena-pairs.json: $bdfs" || echo "FAIL ena-pairs.json unreadable"
        host_checks | on_host "$n" "$reserved" $bdfs
        cluster_checks "$n"
    } >"$out/$n-checks.log" 2>&1
    for what in host cluster; do
        grep -q "^done $what checks$" "$out/$n-checks.log" || echo "FAIL the $what checks did not finish" >>"$out/$n-checks.log"
    done
    grep -E '^(FAIL|WARN|info)' "$out/$n-checks.log" | sed "s/^/  $n: /" >&2
    grep -q '^FAIL' "$out/$n-checks.log" && failed+=" $n"
    log "$n: $(grep -c '^ok' "$out/$n-checks.log") checks ok ($out/$n-checks.log)"
done
groups=$(ec2 describe-instances --instance-ids "$ida" "$idb" --query 'Reservations[].Instances[].Placement.GroupName' --output text)
[[ $(echo $groups) == "$pg $pg" ]] || { log "FAIL the nodes are not both in placement group $pg: $groups"; failed+=" placement"; }
[[ -z $failed ]] || die "checks failed on:$failed (before any measurement)"
{
    cat "$out/instances.txt"
    for n in a b; do
        echo "$n kernel $(on_host "$n" <<<'uname -r')"
        echo "$n cpu $(on_host "$n" <<<"lscpu | sed -n 's/^Model name: *//p'")"
        echo "$n layout housekeeping $(nv hk "$n") sender $(nv snd "$n") receiver $(nv rcv "$n") client $(nv app "$n")"
    done
    in_driver a cat /etc/rusteron-dpdk-versions.txt
    echo "comparability with the VM lab (lab.sh): the instance size, AL2023's kernel against Debian's,"
    echo "and housekeeping and the conductor on an isolated core (here) against core 0 with the IRQs (VM)"
} >"$out/env.txt" 2>&1

if ((reps == 0)); then
    log "EKS_REPS=0: deployed and checked, no measurement (KUBECONFIG=$out/kubeconfig)"
    exit 0
fi

# ---- measure -----------------------------------------------------------------

# jitter <a|b>: each driver thread's CPUs and context switches, the agents'
# poll gaps, and on each of the pod's CPUs the IRQs, timer ticks and steal
# (vm.sh jitter's format, for spikes.py)
jitter() {
    local n=$1
    on_host "$n" "$(nv hk "$n")" "$(nv app "$n")" "$(nv snd "$n")" "$(nv rcv "$n")" <<'SH'
pid=$(pgrep -x media_driver | head -1)
if [[ -n $pid ]]; then
    for t in /proc/$pid/task/*; do
        awk -v comm="$(cat $t/comm 2>/dev/null)" -v tid="${t##*/}" '
            /^Cpus_allowed_list:/ {c = $2} /^voluntary_ctxt_switches:/ {v = $2} /^nonvoluntary_ctxt_switches:/ {n = $2}
            END {printf "thread %s/%s cpus=%s vcsw=%s nvcsw=%s\n", comm, tid, c, v, n}' $t/status 2>/dev/null
    done
fi
for cpu in "$@"; do
    irqs=$(awk -v col=$((cpu + 2)) 'NR > 1 && NF >= col {s += $col} END {print s + 0}' /proc/interrupts)
    loc=$(awk -v col=$((cpu + 2)) '$1 == "LOC:" {print $col}' /proc/interrupts)
    steal=$(awk -v c="cpu$cpu" '$1 == c {print $9}' /proc/stat)
    echo "cpu $cpu irqs=$irqs timer=$loc steal=$steal"
done
SH
    in_driver "$n" env AERON_DIR="$aeron_dir" timeout 20 counters -gap 2>/dev/null | sed 's/^/counter /'
}
loss_now() { in_driver "$1" env AERON_DIR="$aeron_dir" timeout 20 counters loss 2>/dev/null; }
validity() { in_driver "$1" env AERON_DIR="$aeron_dir" timeout 20 counters _ 2>/dev/null | grep -E ' (rx|tx)_good_packets$|allowance_exceeded$'; }
allowance() { awk '/allowance_exceeded$/ {s += $1} END {print s + 0}' "$1"; }

# For the whole run, on each host: the stall spinner on a core outside the
# driver pod, and a trace of the client, agent and spinner CPUs (vm.sh
# diag-start), so each spike can be told apart.
diag_start() {
    local n=$1
    {
        echo "mkdir -p /tmp/ergon-diag && cat >/tmp/ergon-diag/gapspin.c <<'CSRC'"
        cat "$here/../gapspin.c"
        echo "CSRC"
        cat <<'SH'
cd /tmp/ergon-diag || exit 1
gcc -O2 -o gapspin gapspin.c || exit 1
app=$1 snd=$2 rcv=$3 hk=$4 pod=$5
used=" ${hk//,/ } $app $snd $rcv "
for r in ${pod//,/ }; do
    if [[ $r == *-* ]]; then used+="$(seq -s ' ' "${r%-*}" "${r#*-}") "; else used+="$r "; fi
done
spare=
for cpu in $(seq 1 $(($(nproc --all) - 1))); do
    first=$(sed 's/[,-].*//' /sys/devices/system/cpu/cpu$cpu/topology/thread_siblings_list)
    [[ $first == "$cpu" && $used != *" $cpu "* ]] && { spare=$cpu; break; }
done
[[ -n $spare ]] || spare=$(sed 's/.*[,-]//' /sys/devices/system/cpu/cpu0/topology/thread_siblings_list)
pkill -x gapspin 2>/dev/null
setsid nohup ./gapspin "$spare" 50 >diag-gapspin.log 2>&1 </dev/null &
mask=0
for cpu in $app $snd $rcv $spare; do mask=$((mask | (1 << cpu))); done
mountpoint -q /sys/kernel/tracing || mount -t tracefs nodev /sys/kernel/tracing
cd /sys/kernel/tracing && echo 0 > tracing_on && echo > trace && printf '%x' $mask > tracing_cpumask &&
    echo 8192 > buffer_size_kb && echo 1 > events/sched/sched_switch/enable &&
    echo 1 > events/workqueue/workqueue_execute_start/enable && echo 1 > events/irq/irq_handler_entry/enable &&
    { grep -q drainall kprobe_events || echo 'p:drainall lru_add_drain_all' >> kprobe_events; } &&
    mkdir -p instances/drain && echo 1 > instances/drain/events/kprobes/drainall/enable && echo 1 > tracing_on || exit 1
echo "diag spinner cpu $spare, traced cpus $(printf '%x' $mask)"
SH
    } | on_host "$n" "$(nv app "$n")" "$(nv snd "$n")" "$(nv rcv "$n")" "$reserved" "$(nv cpuset "$n")"
}
diag_fetch() {
    on_host "$1" <<'SH'
echo "== gapspin"
cat /tmp/ergon-diag/diag-gapspin.log 2>/dev/null
echo "== trace"
grep -E 'sched_switch|workqueue_execute_start|irq_handler_entry' /sys/kernel/tracing/trace
echo "UPTIME $(cut -d' ' -f1 /proc/uptime) REALTIME $(date +%s.%N)"
echo "== drains"
grep -v '^#' /sys/kernel/tracing/instances/drain/trace 2>/dev/null | grep drainall
pkill -x gapspin 2>/dev/null
echo 0 > /sys/kernel/tracing/tracing_on
SH
}

known=/dev/null # lib.sh's ssh options, unused here
. "$here/../lib.sh" # for loss_row
for n in a b; do
    diag_start "$n" >"$out/$n-diag-start.log" 2>&1 && cat "$out/$n-diag-start.log" >&2 ||
        log "WARNING: $n: diagnostics did not start ($out/$n-diag-start.log)"
done

label=eks:dpdk
client_env=(env AERON_DIR="$aeron_dir" LABEL="$label" RTT_SECONDS="$rtt_seconds" LOOP_STALLS_DIR=/tmp BENCH_CLOCK=tsc
    AERON_TERM_BUFFER_SPARSE_FILE=false AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY=true)
for rep in $(seq "$reps"); do
    run=eks-dpdk-$rep
    log "rep $rep of $reps"
    for n in a b; do
        jitter "$n" >"$out/eks/$n-jitter-before-rtt-$run.log" 2>&1
        validity "$n" >"$out/eks/$n-counters-before-$run.log" 2>&1
    done
    loss_a=$(loss_now a)
    loss_b=$(loss_now b)
    # fresh endpoints each rep: the drivers stay up, and the last rep's image
    # lingers until its liveness times out
    ping_ep=$ip_rcv_b:$((22200 + 2 * rep))
    pong_ep=$ip_rcv_a:$((22201 + 2 * rep))
    in_driver a rm -f /tmp/rtt-histogram.csv /tmp/load-spikes.csv /tmp/stalls-ping.csv
    in_driver b rm -f /tmp/stalls-pong.csv
    # round trip: b pongs, a pings (ping's endpoint on b, pong's on a); each
    # client starts on housekeeping and pins its measured thread to the client CPU
    in_driver b "${client_env[@]}" timeout $((4 * rtt_seconds + 120)) taskset -c "$(nv hk b)" \
        rtt xpong "$ping_ep" "$pong_ep" "$(nv app b)" >"$out/eks/pong-$run.log" 2>&1 &
    pong=$!
    sleep 2
    in_driver a "${client_env[@]}" RTT_HISTOGRAM=/tmp/rtt-histogram.csv LOAD_SPIKES=/tmp/load-spikes.csv LOAD_SPIKE_NS=100000 \
        timeout $((2 * rtt_seconds + 120)) taskset -c "$(nv hk a)" \
        rtt xping "$ping_ep" "$pong_ep" "$rtt_messages" "$rtt_warmup" "$(nv app a)" \
        2>"$out/eks/ping-$run.log" | sed "s/^/$rep,/" | tee -a "$results" >&2
    in_driver b pkill -f '[r]tt xpong' 2>/dev/null
    wait $pong 2>/dev/null
    in_driver a cat /tmp/rtt-histogram.csv >"$out/eks/histogram-$run.csv" 2>/dev/null
    in_driver a cat /tmp/load-spikes.csv >"$out/eks/spikes-eks-dpdk-rtt-$rep.csv" 2>/dev/null
    in_driver a cat /tmp/stalls-ping.csv >"$out/eks/stalls-ping-$run.csv" 2>/dev/null
    in_driver b cat /tmp/stalls-pong.csv >"$out/eks/stalls-pong-$run.csv" 2>/dev/null
    for n in a b; do
        jitter "$n" >"$out/eks/$n-jitter-after-rtt-$run.log" 2>&1
        validity "$n" >"$out/eks/$n-counters-after-$run.log" 2>&1
        before=$(allowance "$out/eks/$n-counters-before-$run.log")
        after=$(allowance "$out/eks/$n-counters-after-$run.log")
        ((after > before)) && log "WARNING: $n: ENA allowance_exceeded grew by $((after - before)) in rep $rep"
    done
    loss_row "$rep" "$label" a "$loss_a" "$(loss_now a)"
    loss_row "$rep" "$label" b "$loss_b" "$(loss_now b)"
done
for n in a b; do
    diag_fetch "$n" >"$out/$n-diag-eks.log" 2>&1 || log "WARNING: $n: diagnostics not fetched"
done
collect_logs

# Every rep's percentiles and MAX, the max across reps, the loss counters'
# growth, then the tables and each max's cause (spikes.py: host stall, guest
# preemption, client stall or unexplained).
grep -q '^[0-9]*,rtt,' "$results" 2>/dev/null || die "no round trip was measured (logs: $out/eks/ping-*.log, $out/logs)"
{
    python3 "$here/../analyse.py" "$results"
    python3 "$here/../spikes.py" "$out/eks" 2>&1
} >"$out/summary.md"
log "round trip, $label (us):"
awk -F, '$2 == "rtt" {
        printf "  rep %s: p50 %.1f  p90 %.1f  p99 %.1f  p99.9 %.1f  p99.99 %.1f  MAX %.1f  (%s samples)\n", $1, $5 / 1e3, $6 / 1e3, $7 / 1e3, $8 / 1e3, $9 / 1e3, $10 / 1e3, $12
        if ($10 > m) m = $10
    }
    $2 == "loss" {loss = loss sprintf("  rep %s %s: %s\n", $1, $4, $5)}
    END {printf "  MAX across reps: %.1f\nloss counters (growth over each rep):\n%s", m / 1e3, loss}' "$results" >&2
cat "$out/summary.md"
