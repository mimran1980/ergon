#!/usr/bin/env bash
# The settings matrix on AWS (scripts/lib-matrix.sh), Tokyo by default: every
# host, kernel, NIC and socket setting under test, with platform noise, round
# trips, HTTP and a market-data burst per condition, each run traced so every
# stall can be attributed.
#   MODE=matrix (default) | base | confirm | diagnose | sqtick | http | md | many | base-many   base: the baseline set only
#   (instance-type comparisons); confirm: stage 2, CONFIRM="base;<spec>..." at 1,000/s,
#   15 minutes each; diagnose: hwlat, slow syscalls, slow kernel functions and the
#   guest-agent A/B, with the loop on a CPU the gap map saw stall; sqtick: the SQPOLL CPU with and
#   without nohz_full (boot profiles full and full-sqtick), A/B/A/B; http: ergon-io (every backend),
#   libcurl (distro, source) and reqwest; md: market data, two backends paired per message;
#   many: 100 connections with mixed trickle and bursts, each backend alone; fair: every
#   backend with its own best tuning on the same hosts (lib-matrix.sh run_fair; the plan and
#   estimate are Azure's: scripts/azure-flags.sh MODE=fair DRY_RUN=1)
#   RCV_TYPE=c7i.2xlarge SND_TYPE=c7i.xlarge AZ_ID=apne1-az4 REGION=ap-northeast-1
#   NIC=stock|tuned RCV_BOOT=full BUILD_RUSTFLAGS=<per host> (scripts/lib-cloud.sh)
#
# Graviton: RCV_TYPE (or SND_TYPE) may be an Arm type such as c7g.2xlarge or
# c8g.2xlarge; each host boots Ubuntu for its own architecture, and the host
# set-up applies Graviton's own settings (core layout without SMT, idle CPUs
# polling on the sender, Rust built for its Neoverse core). The sender may
# stay the x86 rig: its programs are then built on it. See BENCHMARK.md,
# "Graviton".
#
# Every resource carries the tags owner=ergon-lab and run=<stamp>: a key pair,
# a security group, a cluster placement group and two instances (their root
# volumes and network interfaces are deleted with them). The EXIT trap
# terminates and deletes them all and verifies nothing tagged remains; what is
# running is listed every 15 minutes. Results go to results/flags-aws-<stamp>/.
set -euo pipefail
# Run from a private copy: editing this file while a run is going cannot
# reach the run (bash reads a script as it executes it).
if [[ -z ${ERGON_LAB_HOME:-} ]]; then
  ERGON_LAB_HOME=$(cd "$(dirname "$0")/.." && pwd)
  export ERGON_LAB_HOME
  snap=$(mktemp "${TMPDIR:-/tmp}/ergon-lab-$(basename "$0").XXXXXX")
  cp "$0" "$snap"
  exec bash "$snap" "$@"
fi
cd "$ERGON_LAB_HOME"

STAMP=${STAMP:-$(date -u +%Y%m%dT%H%M%SZ)}
REGION=${REGION:-ap-northeast-1}
AZ_ID=${AZ_ID:-apne1-az4}
RCV_TYPE=${RCV_TYPE:-c7i.2xlarge}
SND_TYPE=${SND_TYPE:-c7i.xlarge}
MODE=${MODE:-matrix}
NAME=ergon-lab-$STAMP
OUT=results/flags-aws-$STAMP
RUSER=ubuntu
mkdir -p "$OUT/ssh"
# The results are analysed here at the end (collect_results): build the
# analysis first, so a broken build stops the run before anything is created.
ANALYSE=$PWD/target/release/analyse
cargo build --release -q --bin analyse || { echo "analyse does not build; nothing created"; exit 1; }
cp "$ANALYSE" "$OUT/analyse.bin" && ANALYSE=$PWD/$OUT/analyse.bin
# Snapshot the source now: later edits in the working tree must not reach this run.
# `tune` is a path dependency of `ergon-io` (and its README is `include_str!`d),
# so it has to travel with the snapshot or the build fails on the VM.
( cd ../.. && COPYFILE_DISABLE=1 tar --no-xattrs --exclude target --exclude results -czf "$OLDPWD/$OUT/src.tgz" \
    Cargo.toml rust-toolchain.toml tune io )
# The shared scripts too: this run sources and copies its own snapshot.
mkdir -p "$OUT/scripts"
cp scripts/lib-cloud.sh scripts/lib-matrix.sh scripts/vm-setup.sh "$OUT/scripts/"
LOG=$OUT/run.log
exec > >(trap '' INT TERM; exec tee -a "$LOG") 2>&1
MAX_S=${MAX_S:-36000}
RUN_START=$(date +%s)
( sleep "$MAX_S"; echo "[watchdog] ${MAX_S}s reached, stopping"; kill -TERM $$ ) &
WATCHDOG=$!

log() { echo "[$(date -u +%H:%M:%S)] $*"; }
# `aws login` refreshes its token into a cache file, and two refreshes at once
# (several runs, or a run and its billing loop) corrupt it: every later call
# fails, teardown included. Every call takes one lock shared by all runs on
# this machine ($^F keeps the lock open across exec, for the call's life).
AWS_LOCK=${TMPDIR:-/tmp}/ergon-lab-aws.lock
aws() {
  perl -MFcntl=:flock -e '$^F = 255; open(my $l, ">>", shift) or die "lock: $!"; flock($l, LOCK_EX) or die "lock: $!"; exec @ARGV or die "exec: $!"' \
    "$AWS_LOCK" aws "$@"
}
ec2() { aws --region "$REGION" ec2 "$@"; }
tags() { echo "ResourceType=$1,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$STAMP},{Key=Name,Value=$NAME${2:+-$2}}]"; }
live_instances() {
  ec2 describe-instances --filters "Name=tag:run,Values=$STAMP" \
    Name=instance-state-name,Values=pending,running,stopping,stopped,shutting-down \
    --query 'Reservations[].Instances[].InstanceId' --output text
}

teardown() {
  local rc=$?
  set +e
  trap '' PIPE INT TERM
  trap - EXIT
  kill "$WATCHDOG" "${BILLING:-}" 2>/dev/null
  if [[ $rc -ne 0 && -n ${RCV:-} ]]; then
    log "teardown: salvaging partial results"
    collect_results 2>/dev/null
  fi
  log "teardown: terminating everything tagged run=$STAMP (exit code $rc)"
  local ids="" try
  # A failed call must not read as "nothing to terminate": retry.
  for try in 1 2 3 4 5 6 7 8; do
    if ids=$(live_instances); then
      [[ -z $ids ]] && break
      # shellcheck disable=SC2086
      if ec2 terminate-instances --instance-ids $ids > /dev/null; then
        # shellcheck disable=SC2086
        ec2 wait instance-terminated --instance-ids $ids
        break
      fi
    fi
    log "teardown: AWS call failed (attempt $try); retrying in 30 s"
    sleep 30
  done
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    ec2 delete-placement-group --group-name "$NAME" 2>/dev/null
    [[ -n ${SG:-} ]] && ec2 delete-security-group --group-id "$SG" 2>/dev/null
    local pg sg
    pg=$(ec2 describe-placement-groups --filters "Name=tag:run,Values=$STAMP" --query 'length(PlacementGroups)' --output text 2>/dev/null)
    sg=$(ec2 describe-security-groups --filters "Name=tag:run,Values=$STAMP" --query 'length(SecurityGroups)' --output text 2>/dev/null)
    [[ $pg == 0 && $sg == 0 ]] && break
    sleep 15
  done
  ec2 delete-key-pair --key-name "$NAME" > /dev/null 2>&1
  local left_i left_v left_e left_s left_p left_k
  left_i=$(live_instances | wc -w | tr -d ' ')
  left_v=$(ec2 describe-volumes --filters "Name=tag:run,Values=$STAMP" --query 'length(Volumes)' --output text)
  left_e=$(ec2 describe-network-interfaces --filters "Name=tag:run,Values=$STAMP" --query 'length(NetworkInterfaces)' --output text)
  left_s=$(ec2 describe-security-groups --filters "Name=tag:run,Values=$STAMP" --query 'length(SecurityGroups)' --output text)
  left_p=$(ec2 describe-placement-groups --filters "Name=tag:run,Values=$STAMP" --query 'length(PlacementGroups)' --output text)
  left_k=$(ec2 describe-key-pairs --filters "Name=tag:run,Values=$STAMP" --query 'length(KeyPairs)' --output text)
  log "verify ($REGION, tag run=$STAMP): instances $left_i, volumes $left_v, interfaces $left_e, security groups $left_s, placement groups $left_p, key pairs $left_k"
  if [[ "$left_i$left_v$left_e$left_s$left_p$left_k" != 000000 ]]; then
    log "!!! RESOURCES LEFT FOR run=$STAMP: delete them by hand"
    rc=1
  fi
  exit "$rc"
}
trap teardown EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'echo "[err] line $LINENO: \"$BASH_COMMAND\" exited $?"' ERR
set -E

KEY=$OUT/ssh/id_ed25519
[[ -f $KEY ]] || ssh-keygen -q -t ed25519 -N '' -f "$KEY" -C "$NAME"
SSHO=(-i "$KEY" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ServerAliveInterval=30 -o ServerAliveCountMax=20 -o ConnectTimeout=15 -o ConnectionAttempts=5)
MYIP=$(curl -s https://api.ipify.org)

log "existing in $REGION before create: instances $(ec2 describe-instances --filters Name=instance-state-name,Values=pending,running,stopping,stopped --query 'length(Reservations[].Instances[])' --output text), volumes $(ec2 describe-volumes --query 'length(Volumes)' --output text), lab-tagged $(aws --region "$REGION" resourcegroupstaggingapi get-resources --tag-filters Key=owner,Values=ergon-lab --query 'length(ResourceTagMappingList)' --output text | awk '{n += $1} END {print n + 0}') (the tagging API lists deleted resources for a while)"
log "creating $NAME in $REGION $AZ_ID (receiver $RCV_TYPE, sender $SND_TYPE, mode $MODE)"
# Every 15 minutes while anything exists: what is running.
( while sleep 900; do
    log "[billing] run=$STAMP instances: $(ec2 describe-instances --filters "Name=tag:run,Values=$STAMP" --query 'Reservations[].Instances[].[InstanceType,State.Name]' --output text 2>/dev/null | tr '\n\t' '; ')"
  done ) &
BILLING=$!

ec2 import-key-pair --key-name "$NAME" --public-key-material "fileb://$KEY.pub" --tag-specifications "$(tags key-pair)" > /dev/null
VPC=$(ec2 describe-vpcs --filters Name=isDefault,Values=true --query 'Vpcs[0].VpcId' --output text)
SG=$(ec2 create-security-group --group-name "$NAME" --description "ergon-lab io benchmark $STAMP" --vpc-id "$VPC" \
  --tag-specifications "$(tags security-group)" --query GroupId --output text)
ec2 authorize-security-group-ingress --group-id "$SG" --protocol tcp --port 22 --cidr "$MYIP/32" > /dev/null
ec2 authorize-security-group-ingress --group-id "$SG" --protocol -1 --source-group "$SG" > /dev/null
ec2 create-placement-group --group-name "$NAME" --strategy cluster --tag-specifications "$(tags placement-group)" > /dev/null

ami_for() {
  local arch
  arch=$(ec2 describe-instance-types --instance-types "$1" --query 'InstanceTypes[0].ProcessorInfo.SupportedArchitectures[0]' --output text)
  [[ $arch == arm64 ]] && arch=arm64 || arch=amd64
  aws --region "$REGION" ssm get-parameter --name "/aws/service/canonical/ubuntu/server/24.04/stable/current/$arch/hvm/ebs-gp3/ami-id" \
    --query Parameter.Value --output text
}
launch() {
  local role=$1 type=$2
  ec2 run-instances --image-id "$(ami_for "$type")" --instance-type "$type" --key-name "$NAME" \
    --subnet-id "$SUBNET" --security-group-ids "$SG" --placement "GroupName=$NAME" \
    --instance-initiated-shutdown-behavior terminate \
    --block-device-mappings 'DeviceName=/dev/sda1,Ebs={VolumeSize=128,VolumeType=gp3,DeleteOnTermination=true}' \
    --tag-specifications "$(tags instance "$role")" "$(tags volume "$role")" "$(tags network-interface "$role")" \
    --query 'Instances[0].InstanceId' --output text
}
# Capacity is per zone: try AZ_ID, then the region's other zones; a pair that
# launched only halfway is terminated (and gone, so the cluster placement
# group is free to move) before the next zone.
RCV_ID="" SND_ID=""
zones="$AZ_ID $(ec2 describe-availability-zones --query 'AvailabilityZones[?State==`available`].ZoneId' --output text \
  | tr '\t' '\n' | grep -vx "$AZ_ID" | tr '\n' ' ')"
for az in $zones; do
  SUBNET=$(ec2 describe-subnets --filters "Name=vpc-id,Values=$VPC" "Name=availability-zone-id,Values=$az" Name=default-for-az,Values=true \
    --query 'Subnets[0].SubnetId' --output text)
  if RCV_ID=$(launch rcv "$RCV_TYPE") && SND_ID=$(launch snd "$SND_TYPE"); then
    AZ_ID=$az
    break
  fi
  log "no capacity for the pair in $az"
  if [[ -n $RCV_ID ]]; then
    ec2 terminate-instances --instance-ids "$RCV_ID" > /dev/null
    ec2 wait instance-terminated --instance-ids "$RCV_ID"
  fi
  RCV_ID="" SND_ID=""
done
[[ -n $RCV_ID && -n $SND_ID ]] || { log "no zone had capacity for $RCV_TYPE + $SND_TYPE"; exit 1; }
log "launched in $AZ_ID"
ec2 wait instance-running --instance-ids "$RCV_ID" "$SND_ID"
ip_of() { ec2 describe-instances --instance-ids "$1" --query "Reservations[0].Instances[0].$2" --output text; }
RCV=$(ip_of "$RCV_ID" PublicIpAddress)
SND=$(ip_of "$SND_ID" PublicIpAddress)
SND_PRIV=$(ip_of "$SND_ID" PrivateIpAddress)
log "rcv $RCV ($RCV_ID)  snd $SND ($SND_ID, private $SND_PRIV)"

SCRIPTS=$OUT/scripts
# shellcheck source=lib-cloud.sh
source "$OUT/scripts/lib-cloud.sh"
# shellcheck source=lib-matrix.sh
source "$OUT/scripts/lib-matrix.sh"
# Dead-man switch: the hosts power off (and so terminate) 15 minutes after the
# watchdog, even if this machine loses its AWS credentials or its connection.
# Re-armed after every reboot (reboot_into calls it).
deadman() {
  local m=$(( (RUN_START + MAX_S - $(date +%s)) / 60 + 15 ))
  rssh "sudo shutdown -P +$m 'ergon-lab dead-man switch'" > /dev/null 2>&1 || log "dead-man switch not set on the receiver"
  sssh "sudo shutdown -P +$m 'ergon-lab dead-man switch'" > /dev/null 2>&1 || log "dead-man switch not set on the sender"
  log "dead-man switch: both hosts power off in $m minutes"
}
prepare_hosts
deadman
if [[ $MODE == fair ]]; then
  # The fair report's heading: the run id and the environment.
  FAIR_ENV="run=$STAMP
cloud=aws
region=$REGION $AZ_ID
arch=$RARCH
priority=on-demand
receiver=$RCV_TYPE
sender=$SND_TYPE
image=Ubuntu 24.04 (Canonical, SSM parameter)
receiver_cpu=$(host_facts rssh | cut -d' ' -f3-)
kernel=$(rssh uname -r)"
fi
case $MODE in
  base) run_base_only ;;
  base-many) run_base_only; run_many ;;
  confirm) run_confirm ;;
  diagnose) run_diagnose ;;
  sqtick) run_sqtick ;;
  http) run_http ;;
  md) run_md ;;
  many) run_many ;;
  fair) run_fair ;;
  *) run_matrix ;;
esac
collect_results
log "done; tables in $OUT/tables.md"
