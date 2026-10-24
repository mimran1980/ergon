#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Proves the sample's thread layout on a running cluster:
#
#   samples/low-latency-k8s/scripts/verify.sh [--contract placement|full] [--seconds 30]
#       [--image ll:arm64-generic] [--irq-cpu 0] [--tick-limit 2]
#
# 1. finds ll-media-driver (trading-driver), ll-pong and ll-ping (trading),
#    their UIDs and nodes;
# 2. waits for each app's "ll-selfcheck RESULT PASS" in kubectl logs (a
#    RESULT FAIL fails here);
# 3. starts one probe pod per node (k8s/verify/probe-pod.yaml), which samples
#    every thread of those pods for --seconds and prints the ll-verify lines;
# 4. prints every probe's output, deletes the probes (also on failure or
#    interrupt), and exits 0 only when every probe printed RESULT PASS.
#
# Fails closed: a missing pod, a probe that does not finish, or a probe
# without a RESULT line is a FAIL. It never runs kubectl exec into a hot pod.
set -uo pipefail

HERE=$(cd "$(dirname "$0")/.." && pwd)
CONTRACT=placement
SECONDS_=30
IMAGE=ll:arm64-generic
IRQ_CPU=0
TICK=2
while [[ $# -gt 0 ]]; do
  case $1 in
    --contract) CONTRACT=$2; shift 2 ;;
    --seconds) SECONDS_=$2; shift 2 ;;
    --image) IMAGE=$2; shift 2 ;;
    --irq-cpu) IRQ_CPU=$2; shift 2 ;;
    --tick-limit) TICK=$2; shift 2 ;;
    *) echo "verify.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done
say() { echo "verify: $*" >&2; }
fail() { say "FAIL: $*"; echo -e "ll-verify\tRESULT\tFAIL\tchecks=0\tfailed=1"; exit 1; }
probes=()
cleanup() { for p in "${probes[@]}"; do kubectl -n ll-verify delete pod "$p" --wait=false >/dev/null 2>&1; done; }
trap cleanup EXIT

pod_of() { # namespace app
  kubectl -n "$1" get pods -l "app=$2" --field-selector=status.phase=Running \
    -o jsonpath='{range .items[*]}{.metadata.name} {.metadata.uid} {.spec.nodeName}{"\n"}{end}' 2>/dev/null | head -1
}
declare -A args_of=()
for spec in trading-driver/ll-media-driver trading/ll-pong trading/ll-ping; do
  ns=${spec%/*}; app=${spec#*/}
  line=""
  for _ in $(seq 1 90); do line=$(pod_of "$ns" "$app"); [[ -n $line ]] && break; sleep 2; done
  [[ -n $line ]] || fail "no running pod for app=$app in $ns"
  read -r name uid node <<<"$line"
  if [[ $app != ll-media-driver ]]; then
    ok=""
    for _ in $(seq 1 90); do
      logs=$(kubectl -n "$ns" logs "$name" 2>/dev/null)
      if grep -q "ll-selfcheck RESULT PASS" <<<"$logs"; then ok=1; break; fi
      grep -q "ll-selfcheck RESULT FAIL" <<<"$logs" && { grep "ll-selfcheck" <<<"$logs" >&2; fail "$ns/$name: self-check FAIL"; }
      sleep 2
    done
    [[ -n $ok ]] || fail "$ns/$name: no ll-selfcheck RESULT in 180 s"
    say "$ns/$name: ll-selfcheck RESULT PASS"
  fi
  args_of[$node]+=" --pod $ns/$name=$uid"
done

out=$(mktemp)
for node in "${!args_of[@]}"; do
  name="ll-probe-$(echo "$node" | tr -c 'a-z0-9' '-' | cut -c1-40)$RANDOM"
  cmd="[\"ll-probe\",\"watch\",\"--contract\",\"$CONTRACT\",\"--window-s\",\"$SECONDS_\",\"--pin\",\"$IRQ_CPU\",\"--tick-limit\",\"$TICK\",\"--node\",\"$node\""
  for a in ${args_of[$node]}; do cmd+=",\"$a\""; done
  cmd+="]"
  sed -e "s|__NAME__|$name|" -e "s|__NODE__|$node|" -e "s|__IMAGE__|$IMAGE|" -e "s|__COMMAND__|$cmd|" \
    "$HERE/k8s/verify/probe-pod.yaml" | kubectl apply -f - >/dev/null || fail "cannot start the probe on $node"
  probes+=("$name")
  say "probe $name on $node for ${SECONDS_} s"
done
deadline=$((SECONDS + SECONDS_ + 180))
passed=0
for p in "${probes[@]}"; do
  phase=""
  while (( SECONDS < deadline )); do
    phase=$(kubectl -n ll-verify get pod "$p" -o jsonpath='{.status.phase}' 2>/dev/null)
    [[ $phase == Succeeded || $phase == Failed ]] && break
    sleep 3
  done
  kubectl -n ll-verify logs "$p" >>"$out" 2>&1
  [[ $phase == Succeeded ]] && grep -q $'^ll-verify\tRESULT\tPASS' <(kubectl -n ll-verify logs "$p" 2>/dev/null) && passed=$((passed + 1))
done
cat "$out"
(( passed == ${#probes[@]} && passed > 0 )) || fail "$passed of ${#probes[@]} probes passed"
say "PASS on ${#probes[@]} node(s)"
