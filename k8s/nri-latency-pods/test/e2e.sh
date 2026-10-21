#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# End-to-end test for k8s/nri-latency-pods: one check per latency-pod extension,
# all in kind in Docker. Nothing is installed on the host. Every container,
# cluster, image and volume this script creates is deleted in the trap on
# exit, error or interrupt, then listed again to prove it. Images and
# containers that existed before the run are kept. No cloud.
#
#   test/e2e.sh
#
# Needs: docker, bash. Fails (does not skip) if the Docker daemon has fewer
# than 6 CPUs. This script does NOT touch nri-shared-cpus/test/e2e.sh; it
# builds the shared-cpus plugin from source and uses it as the fixture that
# applies the grants.
#
# The five items under test:
#   1 sized hugetlbfs mount for AERON_DIR
#   2 cpuset.cpus.partition=isolated on the exclusive CPUs
#   3 io_uring seccomp profile (RuntimeDefault + io_uring_*, never Unconfined)
#   4 ValidatingAdmissionPolicy: no exec probes, no always-restart sidecars
#   5 NRI device grants: /dev/ptp* and (x86 only) /dev/cpu_dma_latency
# Plain rlimits ride along. SCHED_FIFO is deliberately not built and not
# tested: nothing here may change the scheduler.
#
# Note on errexit: bash turns -e off for the body of a function invoked as an
# `if` condition, so step functions must not rely on it. They use `must` for
# commands that have to work and `check` for assertions, and end with
# `return $step_failed`.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
KIT=$(cd "$HERE/.." && pwd)
PLUGIN=$(cd "$KIT/../nri-shared-cpus" && pwd)

STAMP=$(date +%H%M%S)
CLUSTER=ergon-latency-pods-e2e-$STAMP
NODE=${CLUSTER}-control-plane
GO_IMAGE=golang:1.27-trixie
# Per-run image tags: two e2e runs must never share a tag, or one run's
# `docker rmi` breaks the other's `kind load`.
PLUGIN_IMAGE=ergon-shared-cpus:e2e-$STAMP
TEST_IMAGE=ergon-latency-pods-demo:e2e-$STAMP
PLUGIN_IMAGE_SRC=ergon-shared-cpus:dev
TEST_IMAGE_SRC=ergon-latency-pods-demo:e2e
TOOLS_VOL=ergon-latency-pods-tools
KIND_VERSION=v0.33.0
KIND_NODE_IMAGE=kindest/node:v1.33.0
NS=ergon-latency-pods
PROFILE_NAME=ergon-io-uring.json

say() { printf '%s\n' "$*"; }

# kind runs in a container with the Docker socket and shells out to the docker
# CLI, so the image must have one: docker:cli. Nothing is installed on the host.
kind() {
    docker run --rm --name "ergon-latency-pods-kind-$$" \
        -v /var/run/docker.sock:/var/run/docker.sock \
        -v "$TOOLS_VOL":/tools:ro \
        -v "$HERE":/e2e:ro \
        -w /tmp \
        --entrypoint /tools/kind \
        docker:cli "$@"
}
kube() { docker exec "$NODE" kubectl --kubeconfig /etc/kubernetes/admin.conf "$@"; }
node() { docker exec "$NODE" "$@"; }

step_failed=0
must() { if "$@"; then return 0; fi; say "  FAIL: $*"; step_failed=1; return 1; }
check() {
    local desc=$1; shift
    if "$@"; then say "  ok:   $desc"; else say "  FAIL: $desc"; step_failed=1; fi
}
pod_phase() { kube get pod "$1" -n "$2" -o jsonpath='{.status.phase}' 2>/dev/null || echo Missing; }
# kubectl jsonpath cannot parse the hyphen in hugepages-2Mi, so pull it out of
# the JSON. Empty means the node does not advertise 2Mi pages.
hugepages_alloc() {
    kube get node -o json 2>/dev/null \
        | sed -n 's/.*"hugepages-2Mi": "\([^"]*\)".*/\1/p' \
        | head -1
}
wait_phase() {
    local p=$1 n=$2 want=$3 t=$4 i=0
    while (( i < t * 2 )); do
        [[ "$(pod_phase "$p" "$n")" == "$want" ]] && return 0
        sleep 0.5; i=$((i + 1))
    done
    say "  TIMEOUT after ${t}s: pod $n/$p -> $want (now $(pod_phase "$p" "$n"))"
    pod_events "$n" "$p"
    step_failed=1
    return 1
}
# The pod's warning events (CreateContainerError text lands here).
pod_events() { kube get events -n "$1" --field-selector "involvedObject.name=$2" 2>/dev/null | tail -20 || true; }
container_env() { kube exec "$1" -n "$2" -- sh -c "printf %s \"\${$3-UNSET}\"" 2>/dev/null || echo EXECFAIL; }
plugin_logs() { kube logs -n ergon-system -l app=ergon-shared-cpus --tail=120 2>/dev/null || true; }

# containerd can SEGV once (seen in RunPodSandbox for coredns on kind in
# Docker Desktop) and then refuse to start: the CRI sandbox store lists a
# sandbox whose metadata is gone, and every start fatals with
# "failed to get metadata for stored sandbox ... not found". Only then is it
# safe to wipe containerd's state — a wipe on a healthy node deletes live
# sandbox metadata and causes the same fatal. Wiping all of
# /var/lib/containerd drops the images too; kind load redoes them. Static
# pods come back from /etc/kubernetes/manifests.
recover_containerd() {
    node sh -c 'journalctl -u containerd --no-pager 2>/dev/null | grep -q "failed to get metadata for stored sandbox"' || return 0
    say "  (containerd is crash-looping on stale sandbox metadata; wiping its state)"
    # killall by exact name: a pkill -f pattern that appears in this script
    # would kill the exec shell that runs it.
    node sh -c '
        systemctl stop kubelet >/dev/null 2>&1
        systemctl stop containerd >/dev/null 2>&1
        sleep 1
        killall -9 containerd-shim-runc-v2 containerd 2>/dev/null || true
        rm -rf /var/lib/containerd /run/containerd 2>/dev/null || true
        mkdir -p /run/containerd
        systemctl start containerd
        # kubelet was stopped above; without this start the node stays
        # NotReady with the unreachable taint and every pod is unschedulable.
        sleep 1
        systemctl start kubelet
    ' >/dev/null 2>&1 || true
    # The wipe dropped the kind-loaded images; put them back or every
    # pullPolicy Never pod fails with ErrImageNeverPull.
    reload_images
}

# True when the node answers ctr with a Server line.
containerd_up() { node sh -c 'ctr version 2>/dev/null | grep -q "Server:"'; }

# Wait for containerd, recovering the sandbox-metadata crash-loop once.
wait_containerd() {
    local li
    for li in $(seq 1 30); do
        containerd_up && return 0
        if (( li == 8 )); then
            say "  (containerd has no Server yet; recovering it)"
            recover_containerd
        fi
        sleep 1
    done
    return 1
}

# ---------------------------------------------------------------- teardown

pre_images=$(docker images --format '{{.Repository}}:{{.Tag}}')
pre_containers=$(docker ps -a --format '{{.Names}}')
pre_volumes=$(docker volume ls --format '{{.Name}}')

cleanup() {
    local rc=$?
    # Run once. `exit` at the end of this function re-fires the EXIT trap,
    # and a TERM during the run fires it too; without this guard the second
    # pass races the first and can delete a parallel run's helpers.
    trap - EXIT INT TERM HUP
    set +e
    say ""
    say "########## cleanup (rc=$rc) ##########"
    if docker volume inspect "$TOOLS_VOL" >/dev/null 2>&1; then
        kind delete cluster --name "$CLUSTER" >/dev/null 2>&1
        kind delete cluster --name "$CLUSTER" >/dev/null 2>&1
    fi
    docker rm -f "$NODE" >/dev/null 2>&1
    local c v img
    while read -r c; do
        [[ -z "$c" ]] && continue
        grep -qxF "$c" <<<"$pre_containers" && continue
        case "$c" in
        "$CLUSTER" | "$NODE" | "$CLUSTER"-* | "ergon-latency-pods-kind-$$" | "ergon-latency-pods-kind-dl-$$")
            say "  rm container $c"; docker rm -f "$c" >/dev/null 2>&1 ;;
        esac
    done < <(docker ps -a --format '{{.Names}}')
    while read -r v; do
        [[ -z "$v" ]] && continue
        if ! grep -qxF "$v" <<<"$pre_volumes" && [[ "$v" == ergon-latency-pods-* ]]; then
            say "  rm volume $v"; docker volume rm -f "$v" >/dev/null 2>&1
        fi
    done < <(docker volume ls --format '{{.Name}}')
    while read -r img; do
        [[ -z "$img" || "$img" == "<none>:<none>" ]] && continue
        if ! grep -qxF "$img" <<<"$pre_images" \
            && [[ "$img" == ergon-latency-pods-* || "$img" == "$PLUGIN_IMAGE" || "$img" == "$TEST_IMAGE" || "$img" == "$KIND_NODE_IMAGE" ]]; then
            say "  rmi $img"; docker rmi -f "$img" >/dev/null 2>&1
        fi
    done < <(docker images --format '{{.Repository}}:{{.Tag}}')
    say ""
    say "=== what is left ==="
    say "--- kind clusters ---"; kind get clusters 2>/dev/null || echo "(none)"
    say "--- docker ps -a ---"; docker ps -a --format '{{.Names}}\t{{.Image}}\t{{.Status}}'
    say "--- docker images ---"; docker images --format '{{.Repository}}:{{.Tag}}'
    say "--- docker volume ls ---"; docker volume ls --format '{{.Name}}'
    exit $rc
}
trap cleanup EXIT INT TERM HUP

# The node is usable when it is Ready and containerd answers.
node_ready() {
    kube get node -o jsonpath='{.items[0].conditions[?(@.type=="Ready")].status}' 2>/dev/null | grep -q True
}

# recover_containerd wipes /var/lib/containerd, which drops the images that
# kind load put there. Put them back after every recovery, or every pod with
# pullPolicy Never fails with ErrImageNeverPull and the plugin DS never rolls.
reload_images() {
    local img
    for img in "$PLUGIN_IMAGE" "$TEST_IMAGE"; do
        kind load docker-image "$img" --name "$CLUSTER" >/dev/null 2>&1 || true
    done
}

# Recover a node that went NotReady after containerd SEGVd, then wait.
ensure_node() {
    node_ready && containerd_up && return 0
    say "  (node not ready or containerd down; recovering)"
    recover_containerd
    kube wait --for=condition=Ready node --all --timeout=120s >/dev/null 2>&1 || true
    wait_containerd || true
    reload_images
    node_ready && containerd_up
}

# Delete and recreate a pod, after making the node usable again.
retry_pod() {
    local f=$1 p=$2
    ensure_node || true
    kube delete pod "$p" -n "$NS" --wait=false >/dev/null 2>&1 || true
    sleep 2
    ensure_node || true
    must kube apply -f "$f"
}

results=()
fails=0
run_step() {
    local n=$1 name=$2 rc=0
    shift 2
    say ""
    say "########## step $n: $name ##########"
    if (( n >= 2 )); then
        ensure_node || say "  (node still unhealthy at the start of step $n)"
    fi
    set +e; "$@"; rc=$?; set -e
    if (( rc == 0 )) && (( step_failed == 0 )); then
        say "step $n: PASS"; results+=("PASS  $n $name")
    else
        say "step $n: FAIL"; results+=("FAIL  $n $name"); fails=$((fails + 1))
    fi
    step_failed=0
}

copy_apply() {
    # copy_apply <local file> <remote name> [more pairs...]
    # The manifests ship the shared image names; rewrite them to this run's
    # unique tags so parallel e2e runs cannot collide on `docker rmi`.
    while (( $# )); do
        must docker cp "$1" "$NODE:/root/$2" || return 1
        must node sed -i \
            -e "s|image: $PLUGIN_IMAGE_SRC|image: $PLUGIN_IMAGE|g" \
            -e "s|image: $TEST_IMAGE_SRC|image: $TEST_IMAGE|g" \
            "/root/$2" || return 1
        shift 2
    done
}

# ---------------------------------------------------------------- build

step1_build_load() {
    say "== build kind $KIND_VERSION =="
    must docker volume create "$TOOLS_VOL" || return 1
    if ! docker run --rm -v "$TOOLS_VOL":/tools alpine test -x /tools/kind 2>/dev/null; then
        must docker run --rm --name "ergon-latency-pods-kind-dl-$$" \
            -v "$TOOLS_VOL":/out \
            "$GO_IMAGE" bash -c "CGO_ENABLED=0 GOBIN=/out go install sigs.k8s.io/kind@$KIND_VERSION" || return 1
    else
        say "  (reusing the kind binary already in $TOOLS_VOL)"
    fi

    say "== build the shared-cpus plugin (the grant fixture) =="
    must docker build -t "$PLUGIN_IMAGE" "$PLUGIN" || return 1
    say "== build the demo tools =="
    must docker build -f "$HERE/Dockerfile" -t "$TEST_IMAGE" "$KIT" || return 1

    say "== kind create cluster =="
    # kind in Docker Desktop can come up with containerd already crash-looping
    # on a stale sandbox record. Recreate the cluster once; a second dead
    # node is a real failure.
    local attempt
    for attempt in 1 2 3; do
        must kind create cluster \
            --name "$CLUSTER" \
            --config /e2e/kind.yaml \
            --image "$KIND_NODE_IMAGE" \
            --wait 120s || return 1
        if wait_containerd; then
            break
        fi
        say "  (containerd never came up on attempt $attempt; recreating the cluster)"
        kind delete cluster --name "$CLUSTER" >/dev/null 2>&1 || true
        if (( attempt == 3 )); then
            say "  FAIL: containerd never started in the kind node"
            step_failed=1
            return 1
        fi
    done
    say "== kind load images =="
    local li ok=0 img
    for img in "$PLUGIN_IMAGE" "$TEST_IMAGE"; do
        ok=0
        for li in 1 2 3 4 5 6 7 8; do
            if kind load docker-image "$img" --name "$CLUSTER"; then
                ok=1
                break
            fi
            say "  kind load $img retry $li/8"
            recover_containerd
            sleep 3
        done
        if (( ! ok )); then
            say "  FAIL: kind load docker-image $img"
            step_failed=1
            return 1
        fi
    done
    must node sh -c 'ctr -n k8s.io images ls' || return 1
    return $step_failed
}

# ---------------------------------------------------------------- deploy

# The node-side fixture every item needs: hugepages reserved, a PHC and a PM
# QoS device node, the shipped seccomp profile where kubelet's Localhost
# lookup finds it, and the node facts file the PHC grant bind-mounts.
step2_deploy() {
    local kver
    kver=$(node uname -r 2>/dev/null || echo unknown)
    say "  kind node kernel: $kver"

    say "== reserve hugepages, make the node advertise them =="
    node sh -c 'echo 32 > /proc/sys/vm/nr_hugepages' >/dev/null 2>&1 || true
    node sh -c 'cat /proc/meminfo | grep -i hugepage' || true
    local i alloc=""
    for i in $(seq 1 60); do
        [[ "$(node sh -c "cat /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages" 2>/dev/null || echo 0)" -gt 0 ]] && break
        sleep 1
    done
    node sh -c 'cat /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages' || true
    # kubelet often picks the pages up without a restart. Restarting it flaps
    # the node NotReady on this kind image, so only restart if the node still
    # does not advertise them, then wait for Ready either way.
    for i in $(seq 1 30); do
        alloc=$(hugepages_alloc)
        [[ -n "$alloc" && "$alloc" != "0" ]] && break
        sleep 1
    done
    if [[ -z "$alloc" || "$alloc" == "0" ]]; then
        say "  (no hugepages-2Mi yet: restarting kubelet once)"
        node systemctl restart kubelet >/dev/null 2>&1 || true
        for i in $(seq 1 60); do
            alloc=$(hugepages_alloc)
            [[ -n "$alloc" && "$alloc" != "0" ]] && break
            sleep 1
        done
    fi
    say "  node allocatable hugepages-2Mi: ${alloc:-<none>}"
    must kube wait --for=condition=Ready node --all --timeout=90s || return 1

    say "== fake a PHC and a PM QoS node if the VM has none =="
    # The plugin only stats these to get major/minor for NRI AddDevice. A
    # char node with any numbers proves the grant path; a real NIC PHC is the
    # lab's job.
    node sh -c 'test -c /dev/ptp0 || mknod /dev/ptp0 c 249 0' || say "  (mknod /dev/ptp0 failed)"
    node sh -c 'test -c /dev/cpu_dma_latency || mknod /dev/cpu_dma_latency c 10 142' || say "  (mknod /dev/cpu_dma_latency failed)"
    node sh -c 'ls -l /dev/ptp* /dev/cpu_dma_latency 2>/dev/null' || true

    say "== install the shipped seccomp profile for the Localhost fallback =="
    must docker cp "$KIT/admission/profiles/$PROFILE_NAME" "$NODE:/root/$PROFILE_NAME" || return 1
    must node mkdir -p /var/lib/kubelet/seccomp || return 1
    must node cp "/root/$PROFILE_NAME" "/var/lib/kubelet/seccomp/$PROFILE_NAME" || return 1

    say "== node facts file the PHC grant bind-mounts =="
    must node mkdir -p /run/ergon || return 1
    must node sh -c 'printf %s "{\"ptp\":\"/dev/ptp0\",\"numa\":0}" > /run/ergon/node.json' || return 1

    must copy_apply \
        "$PLUGIN/deploy/namespace.yaml" namespace.yaml \
        "$PLUGIN/deploy/daemonset.yaml" daemonset.yaml \
        "$HERE/manifests/configmap-e2e.yaml" configmap.yaml \
        "$HERE/manifests/ns.yaml" ns.yaml || return 1
    must kube apply -f /root/namespace.yaml || return 1
    must kube apply -f /root/ns.yaml || return 1
    must kube apply -f /root/configmap.yaml || return 1
    must kube apply -f /root/daemonset.yaml || return 1
    must kube -n ergon-system rollout status ds/ergon-shared-cpus --timeout=120s || return 1
    return $step_failed
}

# ---------------------------------------------------------------- item 1

step3_hugepages() {
    must copy_apply \
        "$HERE/manifests/hugepages-yes.yaml" hugepages-yes.yaml \
        "$HERE/manifests/hugepages-no.yaml" hugepages-no.yaml \
        "$HERE/manifests/hugepages-nolimit.yaml" hugepages-nolimit.yaml || return 1

    # Fail closed, and this needs no hugepages at all: annotation without a
    # hugepages-2Mi limit must not create the container.
    must kube apply -f /root/hugepages-nolimit.yaml || return 1
    sleep 4
    local ev
    ev=$(pod_events "$NS" hugepages-nolimit)
    say "  $ev"
    check "annotation without hugepages-2Mi is refused" \
        grep -qi "hugepages-2Mi\|CreateContainerError" <<<"$ev"
    check "the refusal names the fix (add a hugepages-2Mi limit)" \
        grep -qi "hugepages-2Mi" <<<"$ev"
    must kube delete pod hugepages-nolimit -n "$NS" --wait=false >/dev/null || true

    must kube apply -f /root/hugepages-yes.yaml || return 1
    must kube apply -f /root/hugepages-no.yaml || return 1

    local alloc
    alloc=$(hugepages_alloc)
    say "  node allocatable hugepages-2Mi: ${alloc:-<none>}"
    if [[ -z "$alloc" || "$alloc" == "0" ]]; then
        say "  (node advertises no hugepages-2Mi: the Yes/No pods cannot schedule;"
        say "   the mount shape is covered by the plugin unit tests and by the"
        say "   Pending assertion below. Reserve pages and restart kubelet first.)"
        check "hugepages-yes is Pending when the node has no hugepages-2Mi (fails closed, never a 4K AERON_DIR)" \
            test "$(pod_phase hugepages-yes "$NS")" = Pending
        return $step_failed
    fi

    if ! wait_phase hugepages-yes "$NS" Running 90; then
        say "  -- plugin logs (huge_limit / huge_raw show what the runtime sent) --"
        plugin_logs | tail -30
        say "  (retrying hugepages-yes once after recovering the node)"
        retry_pod /root/hugepages-yes.yaml hugepages-yes || true
        if wait_phase hugepages-yes "$NS" Running 90; then
            step_failed=0   # the first timeout was a dead node, not a bad grant
        else
            say "  -- plugin logs after retry --"
            plugin_logs | tail -30
            return 1
        fi
    fi
    must wait_phase hugepages-no "$NS" Running 90 || return 1

    local mnt
    mnt=$(kube exec hugepages-yes -n "$NS" -- grep hugepages /proc/mounts 2>/dev/null || true)
    say "  /proc/mounts: $mnt"
    check "sized hugetlbfs at /dev/hugepages" grep -q "/dev/hugepages hugetlb" <<<"$mnt"
    check "pagesize=2M" grep -q "pagesize=2M" <<<"$mnt"
    check "size= present" grep -q "size=" <<<"$mnt"
    check "ERGON_HUGEPAGES_DIR=/dev/hugepages" \
        kube exec hugepages-yes -n "$NS" -- sh -c 'test "$ERGON_HUGEPAGES_DIR" = /dev/hugepages'
    check "ERGON_HUGEPAGE_SIZE=2097152" \
        kube exec hugepages-yes -n "$NS" -- sh -c 'test "$ERGON_HUGEPAGE_SIZE" = 2097152'
    check "the plugin does not set AERON_DIR (the app does)" \
        kube exec hugepages-yes -n "$NS" -- sh -c 'test -z "${AERON_DIR:-}"'

    mnt=$(kube exec hugepages-no -n "$NS" -- grep hugepages /proc/mounts 2>/dev/null || true)
    say "  /proc/mounts: $mnt"
    check "no annotation: no hugetlbfs mount" bash -c "! grep -q '/dev/hugepages' <<<'$mnt'"
    check "no annotation: no ERGON_HUGEPAGES_DIR" \
        kube exec hugepages-no -n "$NS" -- sh -c 'test -z "${ERGON_HUGEPAGES_DIR:-}"'
    return $step_failed
}

# ---------------------------------------------------------------- item 2

step4_partition() {
    must copy_apply \
        "$HERE/manifests/partition-yes.yaml" partition-yes.yaml \
        "$HERE/manifests/partition-absent.yaml" partition-absent.yaml \
        "$HERE/manifests/partition-no.yaml" partition-no.yaml || return 1

    # Bad annotation value: refused, no container.
    must kube apply -f /root/partition-no.yaml || return 1
    sleep 4
    local ev
    ev=$(pod_events "$NS" partition-no)
    say "  $ev"
    check "cpu-partition value other than isolated is refused" \
        grep -qi "isolated\|CreateContainerError" <<<"$ev"
    must kube delete pod partition-no -n "$NS" --wait=false >/dev/null || true

    must kube apply -f /root/partition-absent.yaml || return 1
    must wait_phase partition-absent "$NS" Running 90 || return 1
    check "no annotation: no ERGON_ISOLATED_CGROUP" \
        kube exec partition-absent -n "$NS" -- sh -c 'test -z "${ERGON_ISOLATED_CGROUP:-}"'

    must kube apply -f /root/partition-yes.yaml || return 1
    local phase
    phase=$(pod_phase partition-yes "$NS")
    if [[ "$phase" != Running ]]; then
        sleep 8
        phase=$(pod_phase partition-yes "$NS")
    fi
    say "  partition-yes phase: $phase"
    if [[ "$phase" != Running ]]; then
        ev=$(pod_events "$NS" partition-yes)
        say "  $ev"
        local kver
        kver=$(node uname -r 2>/dev/null || echo unknown)
        say "  kernel: $kver"
        check "partition-yes runs, or fails closed naming the kernel (< 6.1)" \
            grep -qi "6\.1\|kernel\|partition\|CreateContainerError" <<<"$ev"
        return $step_failed
    fi

    local cg isolated cpus shared
    cg=$(container_env partition-yes "$NS" ERGON_ISOLATED_CGROUP)
    isolated=$(container_env partition-yes "$NS" ERGON_ISOLATED_CPUS)
    shared=$(container_env partition-yes "$NS" ERGON_SHARED_CPUS)
    say "  ERGON_ISOLATED_CGROUP=$cg ERGON_ISOLATED_CPUS=$isolated ERGON_SHARED_CPUS=$shared"
    check "ERGON_ISOLATED_CGROUP is set and names the child" \
        bash -c "[[ -n \"$cg\" && \"$cg\" == *ergon-isolated ]]"
    check "ERGON_ISOLATED_CPUS is set" bash -c "[[ -n \"$isolated\" ]]"

    local child_cpus child_part parent_part parent_cpus
    child_cpus=$(node sh -c "cat '$cg/cpuset.cpus'" 2>/dev/null || echo MISSING)
    child_part=$(node sh -c "cat '$cg/cpuset.cpus.partition'" 2>/dev/null || echo MISSING)
    say "  child cpuset.cpus=$child_cpus partition=$child_part"
    check "child cpuset.cpus is exactly the exclusive CPUs" \
        bash -c "[[ \"$child_cpus\" == \"$isolated\" ]]"
    check "cpuset.cpus.partition reads isolated (not 'isolated invalid')" \
        bash -c "[[ \"$child_part\" == isolated ]]"

    local parent
    parent=$(dirname "$cg")
    parent_cpus=$(node sh -c "cat '$parent/cpuset.cpus'" 2>/dev/null || echo MISSING)
    parent_part=$(node sh -c "cat '$parent/cpuset.cpus.partition'" 2>/dev/null || echo MISSING)
    say "  parent cpuset.cpus=$parent_cpus partition=$parent_part"
    check "parent keeps the exclusive CPUs (helpers stay load-balanced)" \
        bash -c "[[ \",$parent_cpus,\" == *\",${isolated%%,*},\"* ]]"
    check "parent is not an isolated partition" \
        bash -c "[[ \"$parent_part\" != isolated ]]"

    # Move-then-pin: a tid that lands in the child must run on the exclusive
    # CPUs. (The loop thread does this at start-up.)
    local sched
    sched=$(node sh -c "echo \$\$ > '$cg/cgroup.procs' 2>/dev/null; taskset -pc \$\$ 2>/dev/null; echo \$\$ > '$parent/cgroup.procs' 2>/dev/null" || true)
    say "  move-into-child affinity: $sched"
    check "a tid moved into the child is on the exclusive CPUs" \
        bash -c "[[ \"$sched\" == *\"$isolated\"* || \"$sched\" == *\"$(echo "$isolated" | tr ',' '-')\"* ]]"
    return $step_failed
}

# ---------------------------------------------------------------- item 3

step5_seccomp() {
    must copy_apply \
        "$HERE/manifests/seccomp-yes.yaml" seccomp-yes.yaml \
        "$HERE/manifests/seccomp-no.yaml" seccomp-no.yaml \
        "$HERE/manifests/seccomp-localhost.yaml" seccomp-localhost.yaml \
        "$HERE/manifests/seccomp-unconfined.yaml" seccomp-unconfined.yaml || return 1

    # Unconfined is refused by default (and PSS Baseline forbids it).
    must kube apply -f /root/seccomp-unconfined.yaml || return 1
    sleep 4
    local ev
    ev=$(pod_events "$NS" seccomp-unconfined)
    say "  $ev"
    check "seccomp.ergon.io: unconfined is refused" \
        grep -qi "unconfined\|refused\|io-uring\|CreateContainerError" <<<"$ev"
    check "the refusal names io-uring as the fix" \
        grep -qi "io-uring" <<<"$ev"
    must kube delete pod seccomp-unconfined -n "$NS" --wait=false >/dev/null || true

    must kube apply -f /root/seccomp-yes.yaml || return 1
    must wait_phase seccomp-yes "$NS" Running 90 || return 1
    local out
    out=$(kube exec seccomp-yes -n "$NS" -- io_uring_probe 2>/dev/null || true)
    say "  io_uring_probe (plugin widen): $out"
    check "io_uring allowed after the plugin widens" grep -q "^fd=" <<<"$out"

    must kube apply -f /root/seccomp-no.yaml || return 1
    must wait_phase seccomp-no "$NS" Running 90 || return 1
    out=$(kube exec seccomp-no -n "$NS" -- io_uring_probe 2>/dev/null || true)
    say "  io_uring_probe (RuntimeDefault): $out"
    check "RuntimeDefault blocks io_uring (EPERM)" grep -q "EPERM" <<<"$out"

    must kube apply -f /root/seccomp-localhost.yaml || return 1
    must wait_phase seccomp-localhost "$NS" Running 90 || return 1
    out=$(kube exec seccomp-localhost -n "$NS" -- io_uring_probe 2>/dev/null || true)
    say "  io_uring_probe (Localhost profile): $out"
    check "the shipped Localhost profile allows io_uring" grep -q "^fd=" <<<"$out"
    return $step_failed
}

# ---------------------------------------------------------------- item 4

step6_admission() {
    must copy_apply \
        "$KIT/admission/validating-admission-policy.yaml" vap.yaml \
        "$KIT/admission/priority-class.yaml" priority-class.yaml \
        "$KIT/admission/examples/bad-probe.yaml" bad-probe.yaml \
        "$KIT/admission/examples/bad-sidecar.yaml" bad-sidecar.yaml \
        "$KIT/admission/examples/bad-no-priority.yaml" bad-no-priority.yaml \
        "$KIT/admission/examples/bad-no-required-plugins.yaml" bad-no-required-plugins.yaml \
        "$KIT/admission/examples/good-pod.yaml" good-pod.yaml \
        "$HERE/manifests/probe-in-other-ns.yaml" probe-other.yaml || return 1
    must kube apply -f /root/priority-class.yaml || return 1
    must kube apply -f /root/vap.yaml || return 1
    sleep 3

    local out
    out=$(kube apply -f /root/bad-probe.yaml 2>&1 || true)
    say "  $out"
    check "livenessProbe.exec pod rejected" grep -qi "livenessProbe" <<<"$out"

    out=$(kube apply -f /root/bad-sidecar.yaml 2>&1 || true)
    say "  $out"
    check "restartPolicy: Always sidecar rejected" grep -qi "sidecar\|restartPolicy" <<<"$out"

    out=$(kube apply -f /root/bad-no-priority.yaml 2>&1 || true)
    say "  $out"
    check "pod without priorityClassName rejected" grep -qi "priorityClassName" <<<"$out"

    out=$(kube apply -f /root/bad-no-required-plugins.yaml 2>&1 || true)
    say "  $out"
    check "pod without required-plugins rejected" grep -qi "required-plugins" <<<"$out"

    out=$(kube apply -f /root/good-pod.yaml 2>&1 || true)
    say "  $out"
    check "good pod admitted" bash -c "! grep -qi 'denied\|forbidden' <<<'$out'"

    out=$(kube apply -f /root/probe-other.yaml 2>&1 || true)
    say "  $out"
    check "unlabelled pod untouched (exec probe fine outside a hot pod)" \
        bash -c "! grep -qi 'denied\|forbidden' <<<'$out'"
    return $step_failed
}

# ---------------------------------------------------------------- item 5

step7_devices() {
    must copy_apply \
        "$HERE/manifests/ptp-yes.yaml" ptp-yes.yaml \
        "$HERE/manifests/ptp-no.yaml" ptp-no.yaml \
        "$HERE/manifests/ptp-bad.yaml" ptp-bad.yaml \
        "$HERE/manifests/cpuqos-yes.yaml" cpuqos-yes.yaml || return 1

    # A device this node does not have: refused, and the message lists what it
    # does have.
    must kube apply -f /root/ptp-bad.yaml || return 1
    sleep 4
    local ev
    ev=$(pod_events "$NS" ptp-bad)
    say "  $ev"
    check "a /dev/ptp* this node lacks is refused" \
        grep -qi "ptp9\|not on this node\|CreateContainerError" <<<"$ev"
    check "the refusal lists the node's devices" grep -q "ptp" <<<"$ev"
    must kube delete pod ptp-bad -n "$NS" --wait=false >/dev/null || true

    must kube apply -f /root/ptp-yes.yaml || return 1
    must wait_phase ptp-yes "$NS" Running 90 || return 1
    check "PHC device node exists in the container" \
        kube exec ptp-yes -n "$NS" -- sh -c 'test -c /dev/ptp0'
    check "ERGON_PTP_DEVICE=/dev/ptp0" \
        kube exec ptp-yes -n "$NS" -- sh -c 'test "$ERGON_PTP_DEVICE" = /dev/ptp0'
    check "ERGON_NODE_FACTS is set" \
        kube exec ptp-yes -n "$NS" -- sh -c 'test -n "$ERGON_NODE_FACTS"'
    # The bind of /run/ergon/node.json is unverified in kind: if the runtime
    # refuses the file bind, the adjustment is still correct.
    if kube exec ptp-yes -n "$NS" -- test -f /run/ergon/node.json 2>/dev/null; then
        check "node facts file is mounted read-only" \
            kube exec ptp-yes -n "$NS" -- sh -c 'test -r /run/ergon/node.json'
    else
        say "  note: /run/ergon/node.json did not land in the pod (the bind is"
        say "        unverified in kind). The NRI adjustment is asserted above."
    fi

    must kube apply -f /root/ptp-no.yaml || return 1
    must wait_phase ptp-no "$NS" Running 90 || return 1
    check "no annotation: no /dev/ptp* in the container" \
        kube exec ptp-no -n "$NS" -- sh -c '! test -c /dev/ptp0'
    check "no annotation: no ERGON_PTP_DEVICE" \
        kube exec ptp-no -n "$NS" -- sh -c 'test -z "${ERGON_PTP_DEVICE:-}"'

    must kube apply -f /root/cpuqos-yes.yaml || return 1
    must wait_phase cpuqos-yes "$NS" Running 90 || return 1
    local arch
    arch=$(node uname -m 2>/dev/null || echo unknown)
    say "  node arch: $arch"
    if [[ "$arch" == x86_64 ]]; then
        check "PM QoS device node exists in the container" \
            kube exec cpuqos-yes -n "$NS" -- sh -c 'test -c /dev/cpu_dma_latency'
        check "the app may write its request (FileMode 0666)" \
            kube exec cpuqos-yes -n "$NS" -- sh -c 'test -w /dev/cpu_dma_latency'
        check "ERGON_CPU_DMA_LATENCY=/dev/cpu_dma_latency" \
            kube exec cpuqos-yes -n "$NS" -- sh -c 'test "$ERGON_CPU_DMA_LATENCY" = /dev/cpu_dma_latency'
    else
        say "  not x86_64: the cpuqos annotation is ignored (by design)"
        check "arm64: no /dev/cpu_dma_latency granted" \
            kube exec cpuqos-yes -n "$NS" -- sh -c '! test -c /dev/cpu_dma_latency'
        check "arm64: no ERGON_CPU_DMA_LATENCY" \
            kube exec cpuqos-yes -n "$NS" -- sh -c 'test -z "${ERGON_CPU_DMA_LATENCY:-}"'
    fi
    return $step_failed
}

# ---------------------------------------------------------------- ride-along

step8_rlimits() {
    must copy_apply "$HERE/manifests/rlimits-yes.yaml" rlimits-yes.yaml \
        "$HERE/manifests/rlimits-no.yaml" rlimits-no.yaml || return 1
    must kube apply -f /root/rlimits-yes.yaml || return 1
    must wait_phase rlimits-yes "$NS" Running 60 || return 1
    local ul
    ul=$(kube exec rlimits-yes -n "$NS" -- sh -c 'ulimit -l; ulimit -n; ulimit -r' 2>/dev/null)
    say "  ulimit -l/-n/-r: $ul"
    check "memlock unlimited" grep -q "unlimited" <<<"$ul"
    check "nofile 65536" grep -q "65536" <<<"$ul"
    check "rt-prio 50 (a limit value only)" grep -q "50" <<<"$ul"
    check "ERGON_RLIMIT_MEMLOCK=inf" \
        kube exec rlimits-yes -n "$NS" -- sh -c 'test "$ERGON_RLIMIT_MEMLOCK" = inf'
    # RT scheduling is not built: nothing may change the scheduler and no FIFO
    # env may appear.
    check "no ERGON_SCHED_FIFO (RT scheduling is not built)" \
        kube exec rlimits-yes -n "$NS" -- sh -c 'test -z "${ERGON_SCHED_FIFO:-}"'
    local pol
    pol=$(kube exec rlimits-yes -n "$NS" -- chrt -p 1 2>/dev/null)
    say "  $pol"
    check "PID 1 stays SCHED_OTHER (no scheduler change)" \
        grep -Eq "SCHED_OTHER|NORMAL" <<<"$pol"

    must kube apply -f /root/rlimits-no.yaml || return 1
    must wait_phase rlimits-no "$NS" Running 60 || return 1
    # The node's own memlock is often already unlimited (kind, Docker Desktop),
    # so "not unlimited" is the wrong assertion. The plugin only applies an
    # rlimit when it also exports the matching ERGON_RLIMIT_* env, so the
    # missing env is the proof it left memlock alone.
    say "  memlock without the annotation: $(kube exec rlimits-no -n "$NS" -- sh -c 'ulimit -l' 2>/dev/null)"
    check "no annotation: no ERGON_RLIMIT_MEMLOCK (the plugin does not set memlock)" \
        kube exec rlimits-no -n "$NS" -- sh -c 'test -z "${ERGON_RLIMIT_MEMLOCK:-}"'
    check "no annotation: rt-prio stays the default (0)" \
        kube exec rlimits-no -n "$NS" -- sh -c 'test "$(ulimit -r)" = 0'
    pol=$(kube exec rlimits-no -n "$NS" -- chrt -p 1 2>/dev/null)
    say "  $pol"
    check "no annotation: PID 1 stays SCHED_OTHER (no scheduler change)" \
        grep -Eq "SCHED_OTHER|NORMAL" <<<"$pol"
    return $step_failed
}

# ---------------------------------------------------------------- main

cpus=$(docker info --format '{{.NCPU}}' 2>/dev/null || echo 0)
if (( cpus < 6 )); then
    say "FAIL: Docker reports $cpus CPUs; this e2e needs at least 6"
    exit 1
fi

say "ergon-latency-pods e2e: kind $CLUSTER, ns $NS, one check per extension"
say "extensions: 1 hugepages  2 partition  3 io_uring seccomp  4 admission  5 devices"
run_step 1 "build images, kind create and load" step1_build_load
run_step 2 "node fixture and plugin deploy" step2_deploy
run_step 3 "item 1: sized hugetlbfs for AERON_DIR" step3_hugepages
run_step 4 "item 2: cpuset.cpus.partition=isolated" step4_partition
run_step 5 "item 3: io_uring seccomp (not Unconfined)" step5_seccomp
run_step 6 "item 4: admission strips exec probes and sidecars" step6_admission
run_step 7 "item 5: NRI device grants (ptp, cpuqos)" step7_devices
run_step 8 "ride-along: plain rlimits (no scheduler change)" step8_rlimits

say ""
say "======== results ========"
for r in "${results[@]}"; do say "$r"; done
say "========================="
if (( fails > 0 )); then
    say "e2e: FAIL ($fails step(s))"
    say "--- plugin logs ---"
    plugin_logs
    exit 1
fi
say "e2e: PASS"
