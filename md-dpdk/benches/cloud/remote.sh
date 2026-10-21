#!/usr/bin/env bash
# Runs on VM a, started by lib.sh remote_state: one state's benchmark
# (lib.sh bench_state) against VM a itself and VM b over the private network,
# so the operator machine's connection need not hold for the run.
#
#   remote.sh <state>     reads ~/bench/remote-<state>/env, writes its results,
#                         logs and finally rc there
set -uo pipefail
state=${1:?state}
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
out=$HOME/bench/remote-$state
known=$out/known_hosts
ssh_key=$HOME/.ssh/lab
# shellcheck source=/dev/null
. "$out/env"
read -ra modes <<<"$modes"
log() { echo "[$(date +%T)] $*" >&2; }
die() { log "ERROR: $*"; exit 1; }
cd "$HOME"
. "$here/lib.sh"
# on any exit: no clients, drivers or ssh jobs left behind on either VM
trap 'rc=$?; for h in "$A" "$B"; do on "$h" "sudo pkill -x rtt; sudo pkill -x load; sudo pkill -x tput; sudo pkill -x counters; ergon/md-dpdk/benches/cloud/vm.sh stop" >/dev/null 2>&1; done; kill $(jobs -p) 2>/dev/null; echo $rc >"$out/rc"' EXIT
bench_state "$state"
