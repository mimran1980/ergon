#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Proves the sample's thread layout on a running cluster:
#
#   samples/low-latency-k8s/scripts/verify.sh [--contract placement|full] [--seconds 30]
#       [--image <ref>] [--irq-cpu 0] [--tick-limit 2]
#
# 1. finds every running ll-media-driver pod (trading-driver; a DaemonSet,
#    one per node) and the ll-pong and ll-ping pods (trading), their UIDs and
#    nodes, and fails unless each node that runs a ping or a pong runs
#    exactly one driver and no driver runs on a node without an app;
# 2. waits for each app's "ll-selfcheck RESULT PASS" in kubectl logs (a
#    RESULT FAIL fails here);
# 3. starts one probe pod per node (k8s/verify/probe-pod.yaml) with the image
#    the ll-ping pod runs (--image overrides), which samples every thread of
#    that node's pods for --seconds and prints the ll-verify lines;
# 4. prints every probe's output, deletes the probes (also on failure or
#    interrupt), and exits 0 only when every probe printed RESULT PASS.
#
# The ping must still be measuring when the probe starts (a probe that finds
# no process fails): the overlays bound the ping by LL_DURATION_S=90, which
# outlasts the default window plus the waits here (tests/manifests.rs).
#
# Fails closed: a missing pod, a node without its driver, a probe that does
# not finish, or a probe without a RESULT line is a FAIL. It never runs
# kubectl exec into a hot pod. Runs on bash 3.2 (macOS's /bin/bash) and
# later.
set -uo pipefail

HERE=$(cd "$(dirname "$0")/.." && pwd)
CONTRACT=placement
SECONDS_=30
IMAGE=""
IRQ_CPU=0
TICK=2
POLLS=${LL_VERIFY_POLLS:-90}
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
fail() { say "FAIL: $*"; printf 'll-verify\tRESULT\tFAIL\tchecks=0\tfailed=1\n'; exit 1; }
WORK=$(mktemp -d)
probes=""
cleanup() {
  for p in $probes; do kubectl -n ll-verify delete pod "$p" --wait=false >/dev/null 2>&1; done
  rm -rf "$WORK"
}
trap cleanup EXIT

# "<name> <uid> <node>" for every running pod of app=$2 in namespace $1
pods_of() {
  kubectl -n "$1" get pods -l "app=$2" --field-selector=status.phase=Running \
    -o jsonpath='{range .items[*]}{.metadata.name} {.metadata.uid} {.spec.nodeName}{"\n"}{end}' 2>/dev/null \
    | grep -v '^[[:space:]]*$'
}
# waits for at least one running pod of app=$2 in $1; prints them
wait_pods() {
  local out="" i
  for ((i = 0; i < POLLS; i++)); do
    out=$(pods_of "$1" "$2") && [[ -n $out ]] && break
    sleep 2
  done
  [[ -n $out ]] || return 1
  echo "$out"
}

# node<TAB>arg lines, one per pod the probe on that node watches
targets=$WORK/targets
: >"$targets"
app_nodes=$WORK/app-nodes
: >"$app_nodes"
for spec in trading/ll-pong trading/ll-ping; do
  ns=${spec%/*} app=${spec#*/}
  lines=$(wait_pods "$ns" "$app") || fail "no running pod for app=$app in $ns"
  while read -r name uid node; do
    ok=""
    for ((i = 0; i < POLLS; i++)); do
      logs=$(kubectl -n "$ns" logs "$name" 2>/dev/null)
      if grep -q "ll-selfcheck RESULT PASS" <<<"$logs"; then ok=1; break; fi
      if grep -q "ll-selfcheck RESULT FAIL" <<<"$logs"; then
        grep "ll-selfcheck" <<<"$logs" >&2
        fail "$ns/$name: self-check FAIL"
      fi
      sleep 2
    done
    [[ -n $ok ]] || fail "$ns/$name: no ll-selfcheck RESULT in $((POLLS * 2)) s"
    say "$ns/$name on $node: ll-selfcheck RESULT PASS"
    printf '%s\t--pod %s/%s=%s\n' "$node" "$ns" "$name" "$uid" >>"$targets"
    echo "$node" >>"$app_nodes"
  done <<<"$lines"
done
lines=$(wait_pods trading-driver ll-media-driver) || fail "no running ll-media-driver pod in trading-driver"
driver_nodes=$WORK/driver-nodes
: >"$driver_nodes"
while read -r name uid node; do
  printf '%s\t--pod trading-driver/%s=%s\n' "$node" "$name" "$uid" >>"$targets"
  echo "$node" >>"$driver_nodes"
done <<<"$lines"
# one driver on each app node, none elsewhere
dup=$(sort "$driver_nodes" | uniq -d)
[[ -z $dup ]] || fail "more than one ll-media-driver on: $(echo $dup)"
want=$(sort -u "$app_nodes" | tr '\n' ' ')
have=$(sort -u "$driver_nodes" | tr '\n' ' ')
[[ $want == "$have" ]] || fail "app pods run on [ $want] but ll-media-driver pods on [ $have]"
say "$(sort -u "$app_nodes" | wc -l | tr -d ' ') node(s), each with one driver: $have"

if [[ -z $IMAGE ]]; then
  IMAGE=$(kubectl -n trading get pods -l app=ll-ping -o jsonpath='{.items[0].spec.containers[0].image}' 2>/dev/null)
  [[ -n $IMAGE ]] || fail "cannot read the ll-ping pod's image (pass --image)"
  say "probe image $IMAGE (the ll-ping pod's)"
fi

out=$WORK/out
: >"$out"
for node in $(sort -u "$app_nodes"); do
  name="ll-probe-$(echo "$node" | tr -c 'a-z0-9' '-' | cut -c1-40)$RANDOM"
  cmd="[\"ll-probe\",\"watch\",\"--contract\",\"$CONTRACT\",\"--window-s\",\"$SECONDS_\",\"--pin\",\"$IRQ_CPU\",\"--tick-limit\",\"$TICK\",\"--node\",\"$node\""
  while IFS=$'\t' read -r n arg; do
    [[ $n == "$node" ]] || continue
    for a in $arg; do cmd+=",\"$a\""; done
  done <"$targets"
  cmd+="]"
  sed -e "s|__NAME__|$name|" -e "s|__NODE__|$node|" -e "s|__IMAGE__|$IMAGE|" -e "s|__COMMAND__|$cmd|" \
    "$HERE/k8s/verify/probe-pod.yaml" | kubectl apply -f - >/dev/null || fail "cannot start the probe on $node"
  probes+=" $name"
  say "probe $name on $node for ${SECONDS_} s"
done
deadline=$((SECONDS + SECONDS_ + 180))
passed=0 total=0
for p in $probes; do
  total=$((total + 1))
  phase=""
  while (( SECONDS < deadline )); do
    phase=$(kubectl -n ll-verify get pod "$p" -o jsonpath='{.status.phase}' 2>/dev/null)
    [[ $phase == Succeeded || $phase == Failed ]] && break
    sleep 3
  done
  log=$(kubectl -n ll-verify logs "$p" 2>&1)
  echo "$log" >>"$out"
  [[ $phase == Succeeded ]] && grep -q $'^ll-verify\tRESULT\tPASS' <<<"$log" && passed=$((passed + 1))
done
cat "$out"
(( passed == total && passed > 0 )) || fail "$passed of $total probes passed"
say "PASS on $total node(s)"
