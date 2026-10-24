#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# scripts/verify.sh against a fake kubectl: one probe per node with that
# node's driver, pong and ping; every driver of a two-node DaemonSet probed;
# a node without its driver, two drivers on one node and a failing probe
# refused; the probe image read from the ll-ping pod. Runs verify.sh under
# each bash given (default: bash on PATH, and /bin/bash when it differs,
# which is 3.2 on macOS).
#
#   samples/low-latency-k8s/scripts/test-verify.sh [bash...]
set -euo pipefail

HERE=$(cd "$(dirname "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/bin"
# The fake reads its pods from $FAKE/pods-<ns>-<app>, records applied
# manifests in $FAKE/applied-*.yaml, and answers a probe's logs from
# $FAKE/probe-result (PASS or FAIL).
cat >"$WORK/bin/kubectl" <<'EOF'
#!/usr/bin/env bash
ns="" app="" args=("$@")
while [[ $# -gt 0 ]]; do
  case $1 in
    -n) ns=$2; shift 2 ;;
    -l) app=${2#app=}; shift 2 ;;
    *) shift ;;
  esac
done
set -- "${args[@]}"
case " $* " in
  *" apply -f - "*) n=$(ls "$FAKE"/applied-*.yaml 2>/dev/null | wc -l | tr -d ' '); cat >"$FAKE/applied-$n.yaml" ;;
  *" delete pod "*) ;;
  *"{.items[0].spec.containers[0].image}"*) echo "registry.test/ll:baseline" ;;
  *" get pods "*) cat "$FAKE/pods-$ns-$app" 2>/dev/null ;;
  *"{.status.phase}"*) echo Succeeded ;;
  *" logs "*)
    if [[ $ns == ll-verify ]]; then
      printf 'll-verify\tv1\tcontract=placement\n'
      printf 'll-verify\tRESULT\t%s\tchecks=9\tfailed=0\n' "$(cat "$FAKE/probe-result")"
    else
      echo "ll-selfcheck RESULT PASS"
    fi ;;
  *) echo "fake kubectl: unhandled: $*" >&2; exit 1 ;;
esac
EOF
chmod +x "$WORK/bin/kubectl"

shells=("$@")
if [[ ${#shells[@]} == 0 ]]; then
  shells=(bash)
  [[ -x /bin/bash && $(/bin/bash -c 'echo $BASH_VERSION') != "$(bash -c 'echo $BASH_VERSION')" ]] && shells+=(/bin/bash)
fi
pass=0 fail=0

# case <name> <want: ok|refuse> <text> ; pods come from the caller's files
run() {
  local name=$1 want=$2 text=$3 sh out rc
  for sh in "${shells[@]}"; do
    rm -f "$FAKE"/applied-*.yaml
    set +e
    out=$(PATH="$WORK/bin:$PATH" FAKE=$FAKE LL_VERIFY_POLLS=1 "$sh" "$HERE/scripts/verify.sh" --seconds 1 2>&1)
    rc=$?
    set -e
    if { [[ $want == ok && $rc == 0 ]] || [[ $want == refuse && $rc != 0 ]]; } && [[ "$out" == *"$text"* ]]; then
      pass=$((pass + 1))
      echo "PASS $name ($("$sh" -c 'echo bash $BASH_VERSION'))"
    else
      fail=$((fail + 1))
      echo "FAIL $name ($sh, rc=$rc, wanted $want with \"$text\"):"
      echo "$out" | sed 's/^/    /'
    fi
  done
}
pods() { # ns app lines...
  local f="$FAKE/pods-$1-$2"
  shift 2
  printf '%s\n' "$@" >"$f"
}
applied_has() { grep -l -- "$1" "$FAKE"/applied-*.yaml 2>/dev/null | wc -l | tr -d ' '; }

FAKE=$WORK/fake
mkdir -p "$FAKE"
echo PASS >"$FAKE/probe-result"

# two nodes: ping on a, pong on b, the DaemonSet's driver on each
pods trading ll-ping "ping-1 uid-ping node-a"
pods trading ll-pong "pong-1 uid-pong node-b"
pods trading-driver ll-media-driver "drv-a uid-da node-a" "drv-b uid-db node-b"
run two_nodes ok "PASS on 2 node(s)"
# each probe watches its own node's driver and app
for sh in "${shells[@]}"; do
  rm -f "$FAKE"/applied-*.yaml
  PATH="$WORK/bin:$PATH" FAKE=$FAKE LL_VERIFY_POLLS=1 "$sh" "$HERE/scripts/verify.sh" --seconds 1 >/dev/null 2>&1 || true
  a=$(grep -l 'nodeName: node-a' "$FAKE"/applied-*.yaml)
  b=$(grep -l 'nodeName: node-b' "$FAKE"/applied-*.yaml)
  if grep -q 'trading-driver/drv-a=uid-da' "$a" && grep -q 'trading/ll-ping\|trading/ping-1=uid-ping' "$a" \
    && grep -q 'trading-driver/drv-b=uid-db' "$b" && grep -q 'trading/pong-1=uid-pong' "$b" \
    && ! grep -q 'drv-b' "$a" && ! grep -q 'drv-a' "$b" \
    && [[ $(applied_has 'image: registry.test/ll:baseline') == 2 ]]; then
    pass=$((pass + 1))
    echo "PASS per_node_targets_and_image ($sh)"
  else
    fail=$((fail + 1))
    echo "FAIL per_node_targets_and_image ($sh):"
    cat "$FAKE"/applied-*.yaml | sed 's/^/    /'
  fi
done

# one node: everything on it
pods trading ll-pong "pong-1 uid-pong node-a"
pods trading-driver ll-media-driver "drv-a uid-da node-a"
run one_node ok "PASS on 1 node(s)"

# the pong's node has no driver
pods trading ll-pong "pong-1 uid-pong node-b"
pods trading-driver ll-media-driver "drv-a uid-da node-a"
run missing_driver refuse "but ll-media-driver pods on"

# a driver on a node with no app
pods trading ll-pong "pong-1 uid-pong node-a"
pods trading-driver ll-media-driver "drv-a uid-da node-a" "drv-c uid-dc node-c"
run stray_driver refuse "but ll-media-driver pods on"

# two drivers on one node
pods trading-driver ll-media-driver "drv-a uid-da node-a" "drv-a2 uid-da2 node-a"
run two_drivers_one_node refuse "more than one ll-media-driver on: node-a"

# no driver at all
pods trading-driver ll-media-driver
run no_driver refuse "no running ll-media-driver pod"

# a probe that fails
pods trading-driver ll-media-driver "drv-a uid-da node-a"
echo FAIL >"$FAKE/probe-result"
run probe_fails refuse "0 of 1 probes passed"

echo "verify.sh fixtures: $pass passed, $fail failed"
[[ $fail == 0 ]]
