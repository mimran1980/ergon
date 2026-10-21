#!/usr/bin/env bash
# The fair backend comparison on both platforms at once: azure-flags.sh
# MODE=fair with ARCH=x86 and ARCH=arm (ARCHS), in parallel, each in its own
# resource group (ergon-lab-io-fair-<arch>-<stamp>), each detached (nohup) and
# each deleting and verifying its own group on every exit. One stamp for both,
# so `az resource list --tag run=<stamp>` lists everything this run made.
#   DRY_RUN=1   both plans and estimates, and their total; no cloud call
#   X86_RCV_SIZE/X86_SND_SIZE (default Standard_F4as_v6/Standard_F2as_v6; the
#   fallback Standard_D4ds_v6/Standard_D2ds_v6), ARM_RCV_SIZE/ARM_SND_SIZE
#   (Standard_D4ps_v6/Standard_D2ps_v6): each platform's sizes
#   PRIORITY, FAIR_* and the rest pass through to azure-flags.sh.
# Before starting either, the regional quota must cover both (read-only
# check). A watcher waits for both platforms to end, then lists what is left
# with the run's tag: results/fair-azure-<stamp>-verify.log.
set -euo pipefail
cd "$(dirname "$0")/.."
STAMP=${STAMP:-$(date -u +%Y%m%dT%H%M%SZ)}
ARCHS=${ARCHS:-x86 arm}
export STAMP MODE=fair
# Each platform's sizes and image are its own: none leaks from one to the other.
unset RCV_SIZE SND_SIZE IMAGE
sizes() {
  case $1 in
    x86) echo "${X86_RCV_SIZE:-Standard_F4as_v6} ${X86_SND_SIZE:-Standard_F2as_v6}" ;;
    arm) echo "${ARM_RCV_SIZE:-Standard_D4ps_v6} ${ARM_SND_SIZE:-Standard_D2ps_v6}" ;;
    *) echo "ARCHS: x86 and/or arm" >&2; return 2 ;;
  esac
}
platform() { # <arch> <command...>: the command with that platform's settings
  local a=$1 r s
  shift
  read -r r s < <(sizes "$a")
  ARCH=$a RCV_SIZE=$r SND_SIZE=$s "$@"
}

if [[ ${DRY_RUN:-} == 1 ]]; then
  total=""
  for a in $ARCHS; do
    out=$(DRY_RUN=1 platform "$a" bash scripts/azure-flags.sh)
    echo "$out"
    echo
    total+="$(grep -E '^  cost for' <<< "$out" | sed "s/^/$a: /")"$'\n'
  done
  echo "# Both platforms in parallel (the run takes as long as the longer)"
  printf '%s' "$total" | sed 's/^/  /'
  printf '%s' "$total" | awk '{ for (i = 1; i <= NF; i++) { if ($i == "spot") { gsub(/[^0-9.]/, "", $(i + 1)); s += $(i + 1) } if ($i == "regular") { gsub(/[^0-9.]/, "", $(i + 1)); r += $(i + 1) } } }
    END { printf "  together: spot %.2f USD, regular %.2f USD\n", s, r }'
  exit 0
fi

log() { echo "[$(date -u +%H:%M:%S)] $*"; }
LOC=${LOC:-eastasia}
PRIORITY=${PRIORITY:-Spot}
# The combined need against the regional quota, before anything is created.
# shellcheck source=lib-cloud.sh
source scripts/lib-cloud.sh
# shellcheck source=lib-matrix.sh
source scripts/lib-matrix.sh
# shellcheck source=azure-plan.sh
source scripts/azure-plan.sh
need=0
for a in $ARCHS; do
  read -r r s < <(sizes "$a")
  need=$(( need + $(azure_vcpus "$r") + $(azure_vcpus "$s") ))
done
azure_quota_check "$need" || exit 1

pids=()
for a in $ARCHS; do
  line=$(DETACH=yes platform "$a" bash scripts/azure-flags.sh | tail -1)
  log "$a: $line"
  pid=$(sed -n 's/.*: pid \([0-9]*\),.*/\1/p' <<< "$line")
  [[ -n $pid ]] && pids+=("$pid")
done
mkdir -p results
verify=results/fair-azure-$STAMP-verify.log
nohup bash -c "
  while :; do
    alive=0
    for p in ${pids[*]}; do kill -0 \$p 2> /dev/null && alive=1; done
    (( alive )) || break
    sleep 60
  done
  echo \"[\$(date -u +%H:%M:%S)] both platforms ended\"
  echo \"groups of this run: \$(az group list --query \"[?starts_with(name, 'ergon-lab-io-fair-') && ends_with(name, '-$STAMP')].name\" -o tsv | tr '\n' ' ')\"
  echo \"resources tagged run=$STAMP: \$(az resource list --tag run=$STAMP --query 'length(@)' -o tsv)\"
" > "$verify" 2>&1 < /dev/null &
log "both platforms started (stamp $STAMP); logs: results/fair-azure-<arch>-$STAMP/run.log; final check: $verify"
