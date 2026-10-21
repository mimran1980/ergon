#!/usr/bin/env bash
# The settings matrix on Azure (scripts/lib-matrix.sh): every host, kernel, NIC
# and socket setting under test, with platform noise, round trips, HTTP and a
# market-data burst per condition, each run traced so every stall can be
# attributed. Run it twice at once (two STAMPs) to replicate across VM instances.
#   MODE=matrix (default) | base | confirm | diagnose | sqtick | http | md | many   (base: the baseline set only;
#   confirm: stage 2, CONFIRM="base;<spec>..." at 1,000/s, 15 minutes each;
#   diagnose: hwlat, slow syscalls, slow kernel functions and the guest-agent
#   A/B, with the loop on a CPU the gap map saw stall; sqtick: the SQPOLL CPU with and
#   without nohz_full, boot profiles full and full-sqtick, A/B/A/B; http: ergon-io (every backend),
#   libcurl (distro, source) and reqwest; md: market data, two backends paired per message;
#   many: 100 connections with mixed trickle and bursts, each backend alone;
#   fair: every backend with its own best tuning on the same hosts, see below)
#
# One resource group per run (ergon-lab-ioflags-<stamp>), deleted on every exit
# path and verified gone; the group is listed every 15 minutes while it exists.
# Results go to results/flags-azure-<stamp>/.
#
# MODE=fair (lib-matrix.sh run_fair: market data at 1-256 connections, the
# order round trip with N-1 feed connections on the loop, HTTP with a pool of
# 1 and 20; the NIC and kernel settings held and checked around every point):
#   ARCH=x86|arm         x86: Standard_F4as_v6 receiver (AMD Genoa, 4 cores, no
#                        SMT) and Standard_F2as_v6 sender; arm: Standard_D4ps_v6
#                        and Standard_D2ps_v6 (Cobalt 100, no SMT); Ubuntu 24.04
#                        for the architecture. RCV_SIZE, SND_SIZE and IMAGE
#                        override (x86 fallback: Standard_D4ds_v6 +
#                        Standard_D2ds_v6, two cores with SMT: the IRQs share
#                        housekeeping's core and SQPOLL the loop's)
#   PRIORITY=Spot|Regular  Spot (the default in this mode): eviction policy
#                        Delete; an eviction ends the platform: teardown
#                        deletes the group
#   DRY_RUN=1            print the plan and the time and cost estimate (Azure's
#                        public retail prices) and stop: no cloud call at all
#   DETACH=auto|yes|no   auto: a run whose deadline (MAX_S) is over 2 h
#                        detaches (nohup) and prints where its log is
# The resource group is ergon-lab-io-fair-<arch>-<stamp>, every resource tagged
# owner=ergon-lab run=<stamp> platform=<arch>; results go to
# results/fair-azure-<arch>-<stamp>/ (fair.md: the comparison). Both platforms
# at once: scripts/azure-fair.sh.
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
LOC=${LOC:-eastasia}
MODE=${MODE:-matrix}
RG=ergon-lab-ioflags-$STAMP
# What the caller set, before the defaults (the fair mode has its own).
IMAGE_GIVEN=${IMAGE:-} RCV_SIZE_GIVEN=${RCV_SIZE:-} SND_SIZE_GIVEN=${SND_SIZE:-} PRIORITY_GIVEN=${PRIORITY:-}
IMAGE=${IMAGE:-Canonical:ubuntu-24_04-lts:server:latest}
RCV_SIZE=${RCV_SIZE:-Standard_D8ds_v6}
SND_SIZE=${SND_SIZE:-Standard_D2ds_v6}
PRIORITY=${PRIORITY:-Regular}
OUT=results/flags-azure-$STAMP
TAGS=(owner=ergon-lab run="$STAMP")
if [[ $MODE == fair ]]; then
  # The mode's own defaults; what the caller set wins.
  ARCH=${ARCH:-x86}
  case $ARCH in
    x86) d_rcv=Standard_F4as_v6 d_snd=Standard_F2as_v6 d_image=Canonical:ubuntu-24_04-lts:server:latest ;;
    arm) d_rcv=Standard_D4ps_v6 d_snd=Standard_D2ps_v6 d_image=Canonical:ubuntu-24_04-lts:server-arm64:latest ;;
    *) echo "ARCH=$ARCH: x86 or arm"; exit 2 ;;
  esac
  RCV_SIZE=${RCV_SIZE_GIVEN:-$d_rcv} SND_SIZE=${SND_SIZE_GIVEN:-$d_snd}
  IMAGE=${IMAGE_GIVEN:-$d_image} PRIORITY=${PRIORITY_GIVEN:-Spot}
  RG=ergon-lab-io-fair-$ARCH-$STAMP
  OUT=results/fair-azure-$ARCH-$STAMP
  TAGS+=(platform="$ARCH")
fi
case $PRIORITY in Spot | Regular) ;; *) echo "PRIORITY=$PRIORITY: Spot or Regular"; exit 2 ;; esac
RUSER=azureuser

if [[ $MODE == fair ]]; then
  # shellcheck source=lib-cloud.sh
  source scripts/lib-cloud.sh
  # shellcheck source=lib-matrix.sh
  source scripts/lib-matrix.sh
  # shellcheck source=azure-plan.sh
  source scripts/azure-plan.sh
  EST_S=$(( $(fair_points_s) + FAIR_SETUP_S + FAIR_FINISH_S ))
  # The deadline: half as long again as the estimate (MAX_S overrides).
  MAX_S=${MAX_S:-$(( EST_S * 3 / 2 ))}
  if [[ ${DRY_RUN:-} == 1 ]]; then
    azure_fair_plan
    exit 0
  fi
  if [[ -z ${ERGON_LAB_DETACHED:-} && ${DETACH:-auto} != no ]] && { [[ ${DETACH:-auto} == yes ]] || (( MAX_S > 7200 )); }; then
    # It may run longer than 2 h (its deadline is): detached, so nothing that
    # ends this shell ends the run or its teardown.
    export ERGON_LAB_DETACHED=1 STAMP ARCH MODE PRIORITY RCV_SIZE SND_SIZE IMAGE
    mkdir -p "$OUT"
    nohup bash "$0" "$@" > "$OUT/detached.log" 2>&1 < /dev/null &
    echo "detached (estimate $(( EST_S / 60 )) min): pid $!, log $OUT/run.log (resource group $RG)"
    exit 0
  fi
fi
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
SCRIPTS=$OUT/scripts
LOG=$OUT/run.log
exec > >(trap '' INT TERM; exec tee -a "$LOG") 2>&1
MAX_S=${MAX_S:-36000}
( sleep "$MAX_S"; echo "[watchdog] ${MAX_S}s reached, stopping"; kill -TERM $$ ) &
WATCHDOG=$!

log() { echo "[$(date -u +%H:%M:%S)] $*"; }

teardown() {
  local rc=$?
  set +e
  trap '' PIPE INT TERM
  trap - EXIT
  kill "$WATCHDOG" "${BILLING:-}" "${EVICTION:-}" 2>/dev/null
  # An evicted receiver is gone with its results: nothing to salvage.
  if [[ $rc -ne 0 && -n ${RCV:-} && ! -e $OUT/evicted-rcv ]]; then
    log "teardown: salvaging partial results"
    collect_results 2>/dev/null
  fi
  log "teardown: deleting resource group $RG (exit code $rc)"
  local gone=""
  for _ in 1 2 3 4 5 6; do
    az group delete -n "$RG" --yes 2>/dev/null
    [[ $(az group exists -n "$RG" 2>/dev/null) == false ]] && { gone=1; break; }
    sleep 30
  done
  if [[ -z $gone ]]; then log "!!! RESOURCE GROUP $RG STILL EXISTS: delete it by hand"; rc=1; fi
  log "verify: group exists = $(az group exists -n "$RG")"
  if [[ $MODE == fair ]]; then
    # Both platforms of a run share its stamp: count this platform's only.
    local left
    left=$(az resource list --tag run="$STAMP" --query "[?tags.platform=='$ARCH'] | length(@)" -o tsv)
    log "verify: resources tagged run=$STAMP platform=$ARCH: $left"
    if [[ $left != 0 ]]; then log "!!! RESOURCES LEFT FOR run=$STAMP platform=$ARCH: delete them by hand"; rc=1; fi
    # The backstop deleter is not needed once the group is gone.
    [[ -n $gone ]] && kill "${REAPER:-}" 2> /dev/null
  else
    log "verify: resources tagged run=$STAMP: $(az resource list --tag run="$STAMP" --query 'length(@)' -o tsv)"
  fi
  exit "$rc"
}
trap teardown EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'echo "[err] line $LINENO: \"$BASH_COMMAND\" exited $?"' ERR
set -E

KEY=$OUT/ssh/id_ed25519
[[ -f $KEY ]] || ssh-keygen -q -t ed25519 -N '' -f "$KEY" -C "ergon-lab-$STAMP"
SSHO=(-i "$KEY" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ServerAliveInterval=30 -o ServerAliveCountMax=20 -o ConnectTimeout=15 -o ConnectionAttempts=5)
MYIP=$(curl -s https://api.ipify.org)

log "existing groups before create:"; az group list --query '[].name' -o tsv
if [[ $MODE == fair ]]; then
  # Enough quota for both VMs, or nothing is created.
  azure_quota_check "$(( $(azure_vcpus "$RCV_SIZE") + $(azure_vcpus "$SND_SIZE") ))" || exit 1
  # A backstop that outlives this script (a kill -9, a crash): it deletes the
  # group 30 minutes after the deadline; teardown stops it once the group is
  # verified gone. It needs this machine on and logged in to Azure.
  nohup bash -c "sleep $((MAX_S + 1800)); az group delete -n '$RG' --yes --no-wait" > "$OUT/reaper.log" 2>&1 < /dev/null &
  REAPER=$!
  log "backstop: resource group $RG deleted at the latest $(( (MAX_S + 1800) / 60 )) minutes from now (pid $REAPER)"
fi
log "creating $RG in $LOC (receiver $RCV_SIZE, sender $SND_SIZE, mode $MODE, priority $PRIORITY)"
az group create -n "$RG" -l "$LOC" --tags "${TAGS[@]}" -o none
# Every 15 minutes while the group exists: what is running. Like every
# watcher here, it stops when this script is gone (a kill -9), rather than
# polling for ever.
( while sleep 900 && kill -0 $$ 2> /dev/null; do
    log "[billing] $RG: $(az vm list -g "$RG" -d --query '[].{n:name,s:powerState}' -o tsv 2>/dev/null | tr '\n\t' '; ')"
  done ) &
BILLING=$!
az ppg create -g "$RG" -n ppg -t standard --tags "${TAGS[@]}" -o none
az network vnet create -g "$RG" -n vnet --address-prefix 10.23.0.0/16 --subnet-name s --subnet-prefix 10.23.1.0/24 --tags "${TAGS[@]}" -o none
az network nsg create -g "$RG" -n nsg --tags "${TAGS[@]}" -o none
az network nsg rule create -g "$RG" --nsg-name nsg -n ssh --priority 100 --source-address-prefixes "$MYIP/32" --destination-port-ranges 22 --access Allow --protocol Tcp -o none
# Spot: deleted (never stopped) when Azure evicts it, at any price.
SPOT=()
[[ $PRIORITY == Spot ]] && SPOT=(--priority Spot --eviction-policy Delete --max-price -1)
for vm in rcv snd; do
  size=$RCV_SIZE; [[ $vm == snd ]] && size=$SND_SIZE
  disk=128
  # The fair mode: the sender keeps only the system and its logs
  # (azure-plan.sh FAIR_SND_DISK_GB, the image's own 30 GB).
  if [[ $MODE == fair && $vm == snd ]]; then disk=$FAIR_SND_DISK_GB; fi
  if [[ $MODE == fair && $vm == rcv ]]; then disk=$FAIR_RCV_DISK_GB; fi
  az vm create -g "$RG" -n "$vm" --image "$IMAGE" --size "$size" --ppg ppg --vnet-name vnet --subnet s --nsg nsg \
    --accelerated-networking true --os-disk-size-gb "$disk" --admin-username "$RUSER" --ssh-key-values "$KEY.pub" --public-ip-sku Standard \
    --os-disk-delete-option Delete --nic-delete-option Delete --data-disk-delete-option Delete "${SPOT[@]}" --tags "${TAGS[@]}" -o none
done
if [[ $PRIORITY == Spot ]]; then
  # Every minute: both VMs still there. An evicted spot VM is deleted, so
  # its absence (ResourceNotFound, not a failed call) ends this platform:
  # teardown deletes the group and verifies it.
  ( while sleep 60 && kill -0 $$ 2> /dev/null; do
      for vm in rcv snd; do
        if err=$(az vm show -g "$RG" -n "$vm" --query name -o tsv 2>&1 > /dev/null) || true; [[ $err == *ResourceNotFound* || $err == *"was not found"* ]]; then
          : > "$OUT/evicted-$vm"
          log "!!! spot eviction: $vm ($([[ $vm == rcv ]] && echo "$RCV_SIZE" || echo "$SND_SIZE")) was evicted and deleted by Azure; ending the ${ARCH:-} platform: teardown deletes $RG"
          kill -TERM $$
          exit 0
        fi
      done
    done ) &
  EVICTION=$!
fi
RCV=$(az vm show -d -g "$RG" -n rcv --query publicIps -o tsv)
SND=$(az vm show -d -g "$RG" -n snd --query publicIps -o tsv)
SND_PRIV=$(az vm show -d -g "$RG" -n snd --query privateIps -o tsv)
log "rcv $RCV  snd $SND (private $SND_PRIV)"

# shellcheck source=lib-cloud.sh
source "$OUT/scripts/lib-cloud.sh"
# shellcheck source=lib-matrix.sh
source "$OUT/scripts/lib-matrix.sh"
prepare_hosts
if [[ $MODE == fair ]]; then
  # The report's heading: the run id and the environment.
  FAIR_ENV="run=$STAMP
cloud=azure
region=$LOC
arch=$ARCH
priority=$PRIORITY
receiver=$RCV_SIZE
sender=$SND_SIZE
image=$IMAGE
receiver_cpu=$(host_facts rssh | cut -d' ' -f3-)
kernel=$(rssh uname -r)"
  azure_fair_plan > "$OUT/fair-plan.txt" 2>&1 || true
fi
case $MODE in
  base) run_base_only ;;
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
