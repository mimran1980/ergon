#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The vdev end-to-end run: the DPDK media driver on two tap ports, a pong
# and a ping through it, the thread layout proven by ll-probe, the latency
# table printed.
#
#   samples/low-latency-k8s/scripts/e2e-vdev.sh [--docker | --kind] [--tag baseline]
#       [--overlay vdev-local|vdev-isolated] [--local-chart] [--capture]
#
# --tag: an image tag of docker/build.sh (built when missing). `baseline` is
#   multi-arch; a CPU tag (amd64-v3, arm64-n2, arm64-v3) must match this
#   machine's architecture, or the run stops before starting anything.
#
# --docker (default; the lane that has run): one privileged container of the
#   sample image runs scripts/e2e-inside.sh. No Kubernetes and no plugin: the
#   script sets ERGON_* and pins each process's first thread to the shared
#   CPU as the plugin's hook would. Needs Docker with >= 6 CPUs.
# --kind (written, not yet run): a one-node kind cluster from
#   node/kind-ll.yaml, the ergon-shared-cpus chart (GHCR 0.1.1, or the chart
#   in git with --local-chart), the admission policy, the overlay with its
#   image set to ll:<tag> (and, for a CPU tag, kubernetes.io/arch), then
#   scripts/verify.sh. Needs kind, helm, kubectl, Docker with >= 8 CPUs, no
#   SMT, and containerd >= 2.2 in the node image. The cluster is deleted on
#   success, failure and interrupt.
# --capture writes tests/fixtures/e2e-<mode>-<tag>.txt from the run.
set -uo pipefail

HERE=$(cd "$(dirname "$0")/.." && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
MODE=docker
TAG=baseline
OVERLAY=vdev-local
LOCAL_CHART=0
CAPTURE=0
while [[ $# -gt 0 ]]; do
  case $1 in
    --docker) MODE=docker; shift ;;
    --kind) MODE=kind; shift ;;
    --tag) TAG=$2; shift 2 ;;
    --overlay) OVERLAY=$2; shift 2 ;;
    --local-chart) LOCAL_CHART=1; shift ;;
    --capture) CAPTURE=1; shift ;;
    *) echo "e2e-vdev.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done
IMAGE=ll:$TAG
say() { echo "e2e-vdev: $*" >&2; }
die() { say "FAIL: $*"; exit 1; }
command -v docker >/dev/null || die "docker is not on PATH"
case $(docker info --format '{{.Architecture}}') in
  x86_64 | amd64) ARCH=amd64 ;;
  aarch64 | arm64) ARCH=arm64 ;;
  *) die "Docker's architecture $(docker info --format '{{.Architecture}}'): amd64 or arm64" ;;
esac
case $TAG in
  baseline) ;;
  "$ARCH"-*) ;;
  *) die "tag $TAG is not for this $ARCH machine: use baseline or an $ARCH-* tag" ;;
esac
docker image inspect "$IMAGE" >/dev/null 2>&1 || bash "$HERE/docker/build.sh" "$TAG" || die "cannot build $IMAGE"
OUT=$(mktemp -d)

if [[ $MODE == docker ]]; then
  ncpu=$(docker info --format '{{.NCPU}}')
  (( ncpu >= 6 )) || die "Docker has $ncpu CPUs; the Docker lane needs 6"
  name=ll-e2e-$$
  trap 'docker rm -f "$name" >/dev/null 2>&1; rm -rf "$OUT"' EXIT
  docker run --name "$name" --rm --privileged --shm-size=1g \
    -v "$HERE/scripts:/e2e:ro" -v "$OUT:/out" \
    -e LL_E2E_SAMPLES -e LL_E2E_WINDOW_S -e LL_E2E_DURATION_S \
    "$IMAGE" bash /e2e/e2e-inside.sh | tee "$OUT/e2e.txt"
  rc=${PIPESTATUS[0]}
  if [[ $CAPTURE == 1 && $rc == 0 ]]; then
    { echo "# scripts/e2e-vdev.sh --docker --tag $TAG, $(date -u +%Y-%m-%dT%H:%MZ)"; grep -E $'^(ll-verify|ll-latency)\t' "$OUT/e2e.txt"; grep -h "^ll-selfcheck" "$OUT/selfcheck-ping.txt"; } \
      >"$HERE/tests/fixtures/e2e-docker-$TAG.txt"
    say "captured tests/fixtures/e2e-docker-$TAG.txt"
  fi
  exit "$rc"
fi

# ---- kind --------------------------------------------------------------------
KIND=${KIND:-kind}
CLUSTER=ll-vdev
export KUBECONFIG=/tmp/ll-vdev.kubeconfig
for t in "$KIND" helm kubectl; do command -v "$t" >/dev/null || die "$t is not on PATH"; done
ncpu=$(docker info --format '{{.NCPU}}')
(( ncpu >= 8 )) || die "Docker has $ncpu CPUs: one kind node needs 8 (allocatable = capacity - reservedSystemCPUs 0-1; the driver takes 2 whole CPUs, ping and pong 1 each)"
tpc=$(docker run --rm alpine cat /sys/devices/system/cpu/cpu0/topology/thread_siblings_list)
[[ $tpc =~ ^[0-9]+$ ]] || die "SMT is on (cpu0 siblings $tpc): full-pcpus-only refuses cpu: \"1\""
"$KIND" get clusters 2>/dev/null | grep -qx "$CLUSTER" && die "cluster $CLUSTER exists; delete it first"
created=0
trap '[[ $created == 1 ]] && "$KIND" delete cluster --name "$CLUSTER" >/dev/null 2>&1; rm -rf "$OUT" "$KUBECONFIG" "$HERE/k8s/.e2e-$$"' EXIT
"$KIND" create cluster --name "$CLUSTER" --config "$HERE/node/kind-ll.yaml" --wait 300s || die "kind create failed"
created=1
node=$CLUSTER-control-plane
cv=$(docker exec "$node" containerd --version | grep -oE ' v?[0-9]+\.[0-9]+' | head -1 | tr -d ' v')
[[ $(printf '%s\n2.2\n' "$cv" | sort -V | head -1) == 2.2 ]] || die "containerd $cv in the node: required-plugins needs >= 2.2"
docker exec "$node" mkdir -p /mnt/huge-aeron
if [[ $OVERLAY == vdev-local ]]; then
  docker exec "$node" mount -t tmpfs -o size=1g tmpfs /mnt/huge-aeron || die "tmpfs in the node"
else
  docker exec "$node" sh -c 'mkdir -p /dev/hugepages && mount -t hugetlbfs nodev /dev/hugepages && mount -t hugetlbfs -o pagesize=2M,size=1G nodev /mnt/huge-aeron' \
    || die "hugetlbfs in the node (the host needs vm.nr_hugepages: node/linux-vm-prep.sh)"
fi
"$KIND" load docker-image "$IMAGE" --name "$CLUSTER" || die "kind load"
if [[ $LOCAL_CHART == 1 ]]; then
  docker build -t ergon-shared-cpus:dev "$ROOT/k8s/nri-shared-cpus" && "$KIND" load docker-image ergon-shared-cpus:dev --name "$CLUSTER" || die "local plugin image"
  chart=("$ROOT/k8s/nri-shared-cpus/charts/ergon-shared-cpus" --set image.repository=ergon-shared-cpus --set image.tag=dev)
else
  chart=(oci://ghcr.io/mimran1980/charts/ergon-shared-cpus --version 0.1.1)
fi
helm install ergon-shared-cpus "${chart[@]}" -n ergon-system --create-namespace \
  -f "$HERE/helm/values-kind.yaml" --wait --timeout 5m || die "helm install"
kubectl apply -f "$ROOT/k8s/nri-latency-pods/admission/priority-class.yaml" \
  -f "$ROOT/k8s/nri-latency-pods/admission/validating-admission-policy.yaml" || die "admission"
kubectl label node "$node" ergon.io/shared-cpus=true ergon.io/ll-driver=true ergon.io/ll-app=true --overwrite
# the overlay with the image set to this run's tag (kustomize takes no
# absolute resource path: a scratch dir beside the overlays, removed on exit)
deploy=$HERE/k8s/.e2e-$$
mkdir -p "$deploy"
{
  echo "apiVersion: kustomize.config.k8s.io/v1beta1"
  echo "kind: Kustomization"
  echo "resources: [../overlays/$OVERLAY]"
  echo "images: [{name: ll, newName: ll, newTag: $TAG}]"
  if [[ $TAG != baseline ]]; then
    echo "patches:"
    echo "  - target: {kind: DaemonSet|Deployment|Job}"
    echo "    patch: '[{\"op\": \"add\", \"path\": \"/spec/template/spec/nodeSelector/kubernetes.io~1arch\", \"value\": \"$ARCH\"}]'"
  fi
} >"$deploy/kustomization.yaml"
kubectl apply -k "$deploy" || die "apply $OVERLAY with $IMAGE"
contract=placement
[[ $OVERLAY == vdev-isolated ]] && contract=full
bash "$HERE/scripts/verify.sh" --contract "$contract" --image "$IMAGE" | tee "$OUT/verify.txt"
rc=${PIPESTATUS[0]}
kubectl -n trading wait --for=condition=complete job/ll-ping --timeout=300s || rc=1
kubectl -n trading logs job/ll-ping | grep -E $'^ll-latency\t' | tee "$OUT/latency.txt"
grep -q $'\tlost=0$' "$OUT/latency.txt" || rc=1
grep -q $'^ll-latency\tlog\tdropped_lines=0\twrite_errors=0\t' "$OUT/latency.txt" || rc=1
if [[ $CAPTURE == 1 && $rc == 0 ]]; then
  cat "$OUT/verify.txt" "$OUT/latency.txt" >"$HERE/tests/fixtures/e2e-kind-$OVERLAY-$TAG.txt"
fi
exit "$rc"
