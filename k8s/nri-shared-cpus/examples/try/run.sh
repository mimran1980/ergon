#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Throwaway kind cluster. Installs the public ergon-shared-cpus chart 0.1.0,
# applies the latency-pod admission policy from this repo, runs one hot pod,
# and shows that an exec probe is refused.
#
# Needs: docker (at least 6 CPUs), helm, kubectl, kind.
# Uses KUBECONFIG under /tmp. Does not read or write ~/.kube/config.
# Deletes the cluster only after this run's kind create succeeds, including
# on a later failure or interrupt. A preflight refusal does not delete a
# cluster that already exists. No cloud.
#
#   examples/try/run.sh
#
# KIND=/path/to/kind examples/try/run.sh
set -uo pipefail

TRY=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$TRY/../../../.." && pwd)
ADMISSION="$ROOT/k8s/nri-latency-pods/admission"
CLUSTER=ergon-shared-cpus-try
KUBECONFIG=${KUBECONFIG:-/tmp/ergon-shared-cpus-try.kubeconfig}
export KUBECONFIG
KIND=${KIND:-kind}
LOG=/tmp/ergon-shared-cpus-try.log

say() { printf '%s\n' "$*" | tee -a "$LOG"; }

# Docker Desktop can restart containerd in this kind node and then fail every
# start with "failed to get metadata for stored sandbox ... not found".
# ctr can answer for a moment before that loop begins. Wipe the node's
# containerd state only when it is down and the journal shows that fatal,
# and only once: wiping a healthy node causes the same fatal. Images on the
# node are pulled again. No other cluster is touched.
containerd_recovered=0
containerd_up() {
  # grep runs in the node. A host pipeline under pipefail treats SIGPIPE
  # from grep -q as "containerd is down", and a leftover journal fatal
  # would then wipe a healthy node.
  docker exec "${CLUSTER}-control-plane" sh -c 'ctr version 2>/dev/null | grep -q "Server:"'
}
containerd_sandbox_fatal() {
  docker exec "${CLUSTER}-control-plane" sh -c 'journalctl -u containerd --no-pager 2>/dev/null | grep -q "failed to get metadata for stored sandbox"'
}
recover_containerd() {
  [[ "$containerd_recovered" -eq 1 ]] && return 0
  if containerd_up; then
    return 0
  fi
  containerd_sandbox_fatal || return 0
  containerd_recovered=1
  say "containerd is crash-looping on stale sandbox metadata; wiping its state"
  # killall by exact name. A pkill -f pattern that appears in this script
  # would kill the shell that runs it.
  docker exec "${CLUSTER}-control-plane" sh -c '
    systemctl stop kubelet >/dev/null 2>&1
    systemctl stop containerd >/dev/null 2>&1
    sleep 1
    killall -9 containerd-shim-runc-v2 containerd 2>/dev/null || true
    rm -rf /var/lib/containerd /run/containerd 2>/dev/null || true
    mkdir -p /run/containerd
    systemctl start containerd
    sleep 1
    systemctl start kubelet
  ' >>"$LOG" 2>&1 || true
}
wait_containerd() {
  local i stable=0
  local need=${CONTAINERD_STABLE_NEED:-5}
  local limit=${CONTAINERD_WAIT_LIMIT:-45}
  for i in $(seq 1 "$limit"); do
    if containerd_up; then
      stable=$((stable + 1))
      if [[ "$stable" -ge "$need" ]]; then
        return 0
      fi
    else
      stable=0
      recover_containerd
    fi
    sleep 1
  done
  return 1
}
wait_plugin() {
  local i ready limit=${PLUGIN_WAIT_LIMIT:-150}
  for i in $(seq 1 "$limit"); do
    if ! containerd_up; then
      recover_containerd
      sleep 2
      continue
    fi
    ready=$(kubectl -n ergon-system get ds ergon-shared-cpus -o jsonpath='{.status.numberReady}' 2>/dev/null || echo 0)
    if [[ "$ready" == "1" ]]; then
      return 0
    fi
    sleep 2
  done
  return 1
}

# TRY_LIB_ONLY=1 sources the helpers and stops before any cluster.
if [[ "${TRY_LIB_ONLY:-}" == 1 ]]; then
  return 0 2>/dev/null || exit 0
fi

: >"$LOG"

if [[ "${KUBECONFIG}" != /tmp/ergon-shared-cpus-try.kubeconfig ]]; then
  say "refusing: KUBECONFIG must stay /tmp/ergon-shared-cpus-try.kubeconfig"
  exit 1
fi

created=0
cleanup() {
  if [[ "$created" -ne 1 ]]; then
    return
  fi
  say "deleting kind cluster ${CLUSTER}"
  "$KIND" delete cluster --name "$CLUSTER" >>"$LOG" 2>&1 || true
  rm -f "$KUBECONFIG"
}
trap cleanup EXIT

fail() {
  say "FAIL: $*"
  {
    echo "=== nodes ==="
    kubectl get nodes -o wide 2>&1 || true
    echo "=== pods ==="
    kubectl get pods -A -o wide 2>&1 || true
    echo "=== plugin logs ==="
    kubectl -n ergon-system logs -l app=ergon-shared-cpus --tail=200 2>&1 || true
    echo "=== trader ==="
    kubectl -n trading describe pod trader 2>&1 || true
    kubectl -n trading logs trader 2>&1 || true
  } >>"$LOG"
  say "log: $LOG"
  exit 1
}

command -v docker >/dev/null || fail "docker is not on PATH"
command -v helm >/dev/null || fail "helm is not on PATH"
command -v kubectl >/dev/null || fail "kubectl is not on PATH"
command -v "$KIND" >/dev/null || fail "kind is not on PATH (set KIND= or install kind v0.33.0)"
[[ -f "$ADMISSION/priority-class.yaml" ]] || fail "admission YAML missing at $ADMISSION"

ncpu=$(docker info --format '{{.NCPU}}' 2>/dev/null || echo 0)
say "docker CPUs: $ncpu"
[[ "$ncpu" -ge 6 ]] || fail "Docker has ${ncpu} CPUs; this sample needs at least 6"

if "$KIND" get clusters 2>/dev/null | grep -qx "$CLUSTER"; then
  fail "cluster ${CLUSTER} already exists; delete it before running this sample"
fi

say "creating ${CLUSTER} (kindest/node:v1.33.0)"
"$KIND" create cluster \
  --name "$CLUSTER" \
  --config "$TRY/kind.yaml" \
  --kubeconfig "$KUBECONFIG" \
  --wait 300s >>"$LOG" 2>&1 || fail "kind create failed; see $LOG. This script did not delete a cluster."
created=1

kubectl get nodes -o wide >>"$LOG" || fail "kubectl cannot reach the cluster"
node=$(kubectl get nodes -o jsonpath='{.items[0].metadata.name}')
kubectl label node "$node" ergon.io/shared-cpus=true --overwrite >>"$LOG" || fail "node label failed"
say "node: $node"
wait_containerd || fail "containerd did not stay up on ${node}; see $LOG"

install_chart() {
  helm install ergon-shared-cpus oci://ghcr.io/mimran1980/charts/ergon-shared-cpus \
    --version 0.1.0 \
    --namespace ergon-system --create-namespace \
    --set config.sharedCpus=0 \
    --set 'config.namespaces={trading}' \
    >>"$LOG" 2>&1
}

say "helm install oci://ghcr.io/mimran1980/charts/ergon-shared-cpus --version 0.1.0"
if ! install_chart; then
  say "helm install failed; checking containerd and retrying once"
  wait_containerd || fail "containerd did not stay up on ${node}; see $LOG"
  helm uninstall ergon-shared-cpus -n ergon-system >>"$LOG" 2>&1 || true
  install_chart || fail "helm install failed; see $LOG"
fi
wait_plugin || fail "the ergon-shared-cpus DaemonSet did not become ready; see $LOG"
say "chart 0.1.0 notes still say it is not released. That sentence is stale. This install is the public chart."

say "waiting for ergon.io/shared-millicpus"
ok=0
i=0
while [[ "$i" -lt 60 ]]; do
  alloc=$(kubectl get node "$node" -o go-template='{{index .status.allocatable "ergon.io/shared-millicpus"}}' 2>/dev/null || true)
  if [[ -n "$alloc" && "$alloc" != "<no value>" ]]; then
    say "allocatable ergon.io/shared-millicpus=${alloc}"
    ok=1
    break
  fi
  i=$((i + 1))
  sleep 2
done
[[ "$ok" -eq 1 ]] || fail "the device plugin did not publish ergon.io/shared-millicpus"

kubectl create namespace trading >>"$LOG" 2>&1 || fail "namespace trading already exists or could not be created"
kubectl apply -f "$ADMISSION/priority-class.yaml" >>"$LOG" || fail "priority classes failed"
kubectl apply -f "$ADMISSION/validating-admission-policy.yaml" >>"$LOG" || fail "admission policy failed"

say "applying the hot pod"
kubectl apply -f "$TRY/hot-pod.yaml" >>"$LOG" || fail "hot pod was refused"
kubectl wait -n trading --for=condition=Ready pod/trader --timeout=180s >>"$LOG" 2>&1 || fail "trader did not become Ready"

# The container prints once, then sleeps. Give the log a moment.
sleep 2
grant=$(kubectl -n trading logs trader 2>>"$LOG" || true)
printf '%s\n' "$grant" | tee -a "$LOG"
printf '%s\n' "$grant" | grep -q '^ERGON_SHARED_CPUS=0$' || fail "ERGON_SHARED_CPUS is not 0"
printf '%s\n' "$grant" | grep -q '^ERGON_ISOLATED_CPUS=..*' || fail "ERGON_ISOLATED_CPUS is empty"
printf '%s\n' "$grant" | grep -q '^Cpus_allowed_list:[[:space:]]*0$' || fail "PID 1 Cpus_allowed_list is not the shared CPU 0"
printf '%s\n' "$grant" | grep -q '^nproc=1$' || fail "nproc is not 1; PID 1 can see more than the shared CPU"

say "applying the exec-probe pod (expect deny)"
deny_status=0
deny=$(kubectl apply -f "$TRY/bad-probe.yaml" 2>&1) || deny_status=$?
printf '%s\n' "$deny" | tee -a "$LOG"
[[ "$deny_status" -ne 0 ]] || fail "the exec probe was admitted (kubectl exited 0)"
printf '%s\n' "$deny" | grep -Eqi 'denied|forbidden' || fail "the exec probe was not an admission denial"
printf '%s\n' "$deny" | grep -q 'livenessProbe must not be exec' || fail "the exec probe was not denied for the probe rule"
if kubectl -n trading get pod trader-bad-probe >/dev/null 2>&1; then
  fail "trader-bad-probe exists; the policy did not deny it"
fi

say "PASS"
say "log: $LOG"
