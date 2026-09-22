#!/usr/bin/env bash
# Start the local fixture stack: preflight, build artifacts, apply
# manifests, wait for readiness, print URLs. Idempotent.
set -euo pipefail

cd "$(dirname "$0")/.."

MODE="fixture"
CLUSTER="kind"
CONTEXT=""
NAMESPACE="clickhouse"
ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode)    MODE="$2"; shift 2 ;;
        --cluster) CLUSTER="$2"; shift 2 ;;
        --context) CONTEXT="$2"; shift 2 ;;
        *) ARGS+=("$1"); shift ;;
    esac
done
[[ -n "$CONTEXT" ]] && export KUBECONFIG=${KUBECONFIG:-} && kubectl config use-context "$CONTEXT" >/dev/null

log() { printf '[start] %s\n' "$(date -u +%H:%M:%S) $*"; }

# ── preflight ────────────────────────────────────────────────────────────────
log "preflight: tools"
for tool in docker kubectl cargo python3; do
    command -v "$tool" >/dev/null || { echo "missing tool: $tool" >&2; exit 1; }
done
log "preflight: container runtime"
docker info >/dev/null 2>&1 || { echo "docker daemon not running" >&2; exit 1; }
if [[ "$CLUSTER" == "kind" ]]; then
    command -v kind >/dev/null || { echo "missing tool: kind" >&2; exit 1; }
    kubectl config get-contexts -o name | grep -q '^kind-clickhouse$' || {
        log "creating kind cluster"
        kind create cluster --config deploy/kind.yaml
    }
fi
log "preflight: capacity (host)"
AVAILABLE_KB=$(df -k . | awk 'NR==2 {print $4}')
AVAILABLE_GB=$((AVAILABLE_KB / 1024 / 1024))
log "disk available: ${AVAILABLE_GB}GiB"
if (( AVAILABLE_GB < 20 )); then
    echo "insufficient disk (<20GiB free): run ./scripts/clean.sh full and retry" >&2
    exit 1
fi
log "mode: $MODE (label: $MODE-profile)"

# ── build artifacts ──────────────────────────────────────────────────────────
log "building Rust artifacts"
# The ingester pod runs `--mode aeron`, which is behind the `archive` feature;
# without it the binary exits immediately with "built without the `archive`
# feature" and the pod CrashLoops.
cargo build --release -p ingester --features archive -p archive-agent
# Built separately from the line above: combining it with `ingester --features
# archive` in one invocation unifies features across the shared
# ergo-clickhouse-persist dependency, so `archive` (and its libaeron link)
# leaks into a binary that never asked for it. The result runs on a Linux
# build host (where the loader can find libaeron.so on the system path) but
# fails on macOS with `Library not loaded: @rpath/libaeron.dylib — no
# LC_RPATH's found`, and config-watch.py — which spawns this binary without
# the notebooks' DYLD_FALLBACK_LIBRARY_PATH workaround — reports that as
# "INVALID edit", not a build problem. recording-config only needs the
# `config` feature (Cargo's own default), never `archive`.
cargo build --release -p recording-config

if [[ "$CLUSTER" == "kind" ]]; then
    # Load images into kind (pinned builds; local images built on demand).
    for img in ergo/ingester:local ergo/archive-agent:local ergo/market-recorder:local ergo/archive-driver:local ergo/jupyter-lab:local; do
        docker image inspect "$img" >/dev/null 2>&1 || {
            log "image $img not present — building it (deploy/images/build.sh)"
            ./deploy/images/build.sh "$img" || exit 1
        }
        kind load docker-image "$img" --name clickhouse
    done
fi

# ── config first, then manifests ─────────────────────────────────────────────
log "applying recording config"
./scripts/config-watch.py --once

# The live profile must not carry exchange credentials into any pod. Check the
# rendered manifests *before* applying them: a post-apply `kubectl exec` check
# cannot see a pod that is still Pending, and PLAN §8/§12 make the absence of
# credentials a precondition of the profile, not a postcondition.
OVERLAY="local"
[[ "$MODE" == "live" ]] && OVERLAY="live"
if [[ "$MODE" == "live" ]]; then
    log "live profile: verifying no exchange credentials in the rendered manifests"
    if kubectl kustomize "deploy/overlays/$OVERLAY" \
        | grep -iE 'name:[[:space:]]*"?[A-Z_]*(API_?KEY|API_?SECRET|SECRET_?KEY)[A-Z_]*"?'; then
        echo "LIVE profile must not declare exchange credential env vars" >&2
        exit 1
    fi
fi

# The JupyterLab pod mounts the notebooks from this ConfigMap; without it the
# apply fails on a missing volume.
log "publishing notebooks as a ConfigMap"
kubectl create configmap ergo-notebooks -n "$NAMESPACE" \
    --from-file=notebooks/ --dry-run=client -o yaml | kubectl apply -f - >/dev/null

log "applying manifests (overlay: $OVERLAY)"
kubectl apply -k "deploy/overlays/$OVERLAY" >/dev/null
if [[ "$MODE" == "live" ]]; then
    log "live profile: verifying no exchange credentials in pod env"
    pods=$(kubectl get pods -n "$NAMESPACE" -l app=recorder -o name)
    [[ -n "$pods" ]] || { echo "no recorder pods to verify in the live profile" >&2; exit 1; }
    for pod in $pods; do
        # A pod whose env cannot be read is not evidence of absence; an exec
        # failure must fail the run rather than yield an empty string that
        # trivially satisfies the grep.
        env=$(kubectl exec -n "$NAMESPACE" "$pod" -c market-recorder -- env) || {
            echo "cannot read env of $pod; the live credential check cannot be satisfied" >&2
            exit 1
        }
        if echo "$env" | grep -qiE 'binance.*(api.?key|secret)|bybit.*(api.?key|secret)|API_KEY|API_SECRET'; then
            echo "LIVE profile must not contain exchange credentials" >&2
            exit 1
        fi
    done
fi

# ── wait for readiness ───────────────────────────────────────────────────────
log "waiting for readiness (this can take a few minutes on first run)"
ready="0/0"
deadline=$((SECONDS + 600))
while (( SECONDS < deadline )); do
    # jupyter-0 is excluded: a containerd unpack race on some hosts corrupts
    # its image non-deterministically (PLAN.md, "JupyterLab cannot run on
    # this host"), independent of anything this script controls, and nothing
    # in the acceptance path needs it. Gating readiness on it would make
    # every start on an affected host time out and exit 1 despite a fully
    # healthy pipeline.
    ready=$(kubectl get pods -n "$NAMESPACE" -o json 2>/dev/null | python3 -c '
import json,sys
d = json.load(sys.stdin)
total = ready = 0
for p in d.get("items", []):
    if p.get("metadata", {}).get("labels", {}).get("app") == "jupyter":
        continue
    # A completed Job pod (clickhouse-init, s3-bucket-init) never reports
    # ready=true even after its container exits 0 — that is correct
    # Kubernetes behaviour for a Job, not a stuck workload. Counting it here
    # would hang this gate forever on an otherwise fully healthy cluster.
    if p.get("status", {}).get("phase") == "Succeeded":
        continue
    for c in p.get("status", {}).get("containerStatuses", []):
        total += 1
        if c.get("ready"): ready += 1
print(f"{ready}/{total}")' 2>/dev/null || echo "0/0")
    log "containers ready: $ready"
    if [[ "$ready" =~ ^([1-9][0-9]*)/([0-9]+)$ ]] && [[ "${BASH_REMATCH[1]}" == "${BASH_REMATCH[2]}" ]]; then
        break
    fi
    sleep 10
done
# The loop's `break` is the only success path; timing out must not fall through
# to the URL banner and exit 0.
if ! [[ "$ready" =~ ^([1-9][0-9]*)/([0-9]+)$ ]] || [[ "${BASH_REMATCH[1]}" != "${BASH_REMATCH[2]}" ]]; then
    echo "stack not ready after 600s ($ready containers ready)" >&2
    kubectl get pods -n "$NAMESPACE" >&2 || true
    exit 1
fi

# ── port-forwards (tracked, stopped by stop.sh) ──────────────────────────────
mkdir -p /tmp/ergo-lab
for svc_port in "clickhouse 8123" "grafana 3000" "prometheus 9090" "jupyter 8888"; do
    set -- $svc_port
    pf_file="/tmp/ergo-lab/pf-$1.pid"
    if [[ ! -f "$pf_file" ]] || ! kill -0 "$(cat "$pf_file")" 2>/dev/null; then
        kubectl port-forward -n "$NAMESPACE" "svc/$1" "$2:$2" >/dev/null 2>&1 &
        echo $! > "$pf_file"
    fi
done

log "── local URLs ──────────────────────────────────────"
log "  ClickHouse:  http://localhost:8123  (play: curl 'http://localhost:8123/?query=SELECT+1')"
log "  Grafana:     http://localhost:3000 (anonymous viewer)"
log "  Prometheus:  http://localhost:9090"
log "  JupyterLab:  http://localhost:8888"
log "next: just clickhouse-watch-config   # edit config/recording.yaml live"
log "next: ./scripts/verify-local.sh      # fixture verification"
