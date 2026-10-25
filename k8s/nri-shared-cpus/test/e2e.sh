#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# End-to-end test for k8s/nri-shared-cpus on kind.
#
# Everything runs in Docker containers: nothing is installed on the host.
# Every container, cluster, image and volume this script creates is deleted in
# the trap on exit, error or interrupt, then listed again to prove it. Images
# and containers that existed before the run are kept (other workflows use
# them). No cloud.
#
#   test/e2e.sh
#
# Needs: docker, bash. Fails (does not skip) if the Docker daemon has fewer
# than 6 CPUs.
#
# Note on errexit: bash turns -e off for the body of a function invoked as an
# `if` condition, so step functions must not rely on it. They use `must` for
# commands that have to work and `check` for assertions, and end with
# `return $step_failed`.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
E2E="$ROOT/test/e2e"
MANIFESTS="$E2E/manifests"

CLUSTER=ergon-nri-e2e
NODE=${CLUSTER}-control-plane
GO_IMAGE=golang:1.27-trixie
PLUGIN_IMAGE=ergon-shared-cpus:dev
TEST_IMAGE=ergon-nri-pin-test:e2e
GOMOD_VOL=ergon-nri-gomod
TOOLS_VOL=ergon-nri-tools
KIND_VERSION=v0.33.0
NS=ergon-e2e

say() { printf '%s\n' "$*"; }

# ---------------------------------------------------------------- tools

# kind runs in a container with the Docker socket. It shells out to the docker
# CLI for `docker info`, so the image must have one: docker:cli. Nothing is
# installed on the host.
KIND_RUN_IMAGE=docker:cli
kind() {
    docker run --rm --name "ergon-nri-kind-$$" \
        -v /var/run/docker.sock:/var/run/docker.sock \
        -v "$TOOLS_VOL":/tools:ro \
        -v "$E2E":/e2e:ro \
        -w /tmp \
        --entrypoint /tools/kind \
        "$KIND_RUN_IMAGE" "$@"
}

kube() {
    docker exec "$NODE" kubectl --kubeconfig /etc/kubernetes/admin.conf "$@"
}

node_exec() { docker exec "$NODE" "$@"; }

plugin_logs() {
    kube logs -n ergon-system -l app=ergon-shared-cpus --tail=-1 2>/dev/null || true
}

pod_phase() {
    kube get pod "$1" -n "$2" -o jsonpath='{.status.phase}' 2>/dev/null || echo Missing
}

probe() { kube exec "$1" -n "$NS" -- pin-test probe 2>/dev/null; }

kv() { # kv <key>  (stdin: probe output)
    awk -F= -v k="$1" '$1==k{print substr($0, length(k)+2); exit}'
}

# Expand a kernel cpulist ("0-1,5") or comma list to sorted numbers, one/line.
cpus_of() {
    awk -F',' '{
        for (i=1;i<=NF;i++) {
            n=$i
            gsub(/^[ \t]+|[ \t]+$/, "", n)
            if (n=="") continue
            if (index(n,"-")) { split(n,a,"-"); for(c=a[1]+0;c<=a[2]+0;c++) print c }
            else print n+0
        }
    }' | sort -n
}

need_layout() {
    # Step 4 writes /tmp/e.txt and friends. Later steps read them; without
    # them the run would only produce cascading "No such file" noise.
    if [[ -f /tmp/e.txt && -f /tmp/s.txt && -f /tmp/eu.txt ]]; then
        return 0
    fi
    say "  FAIL: no layout files from step 4 (pod A never came up)"
    step_failed=1
    return 1
}

qty() { # Kubernetes prints 4k for 4000 and 100k for 100000. Plain decimal out.
    local v=${1:-0}
    case $v in
    *M) echo $(( ${v%M} * 1000000 )) ;;
    *k) echo $(( ${v%k} * 1000 )) ;;
    *) echo "$v" ;;
    esac
}

drop_pods() { # free CPU: delete pods this run created and no longer needs
    local p
    for p in "$@"; do
        kube delete pod "$p" -n "$NS" --ignore-not-found --wait=true >/dev/null 2>&1 || true
    done
}

alloc_val() { # node allocatable for our resource, as a plain decimal
    qty "$(kube get node "$NODE" -o go-template='{{index .status.allocatable "ergon.io/shared-millicpus"}}' 2>/dev/null || echo 0)"
}

alloc_is() { test "$(alloc_val)" = "$1"; }

wait_allocatable() { # wait_allocatable <want> <seconds> <what>
    local want=$1 t=$2 what=$3 now
    wait_for "$t" "$what (want $want, now $(alloc_val))" alloc_is "$want"
    now=$(alloc_val)
    say "  allocatable $what = $now"
    check "$what" test "$now" = "$want"
}

wait_container_reason() { # wait_container_reason <pod> <ns> <reason> <seconds>
    local p=$1 n=$2 want=$3 t=$4
    wait_for "$t" "pod $n/$p container -> $want (now $(kube get pod "$p" -n "$n" -o jsonpath='{.status.containerStatuses[0].state.waiting.reason}' 2>/dev/null || echo none))" \
        bash -c "test \"\$(docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf get pod $p -n $n -o jsonpath='{.status.containerStatuses[0].state.waiting.reason}' 2>/dev/null)\" = $want"
}

wait_for() { # wait_for <seconds> <description> <command...>
    local t=$1 desc=$2 i=0
    shift 2
    while (( i < t * 2 )); do
        if "$@" >/dev/null 2>&1; then return 0; fi
        sleep 0.5
        i=$((i + 1))
    done
    say "  TIMEOUT after ${t}s: $desc"
    return 1
}

wait_phase() { # wait_phase <pod> <ns> <phase> <seconds>
    local p=$1 n=$2 want=$3 t=$4
    wait_for "$t" "pod $n/$p -> $want (now $(pod_phase "$p" "$n"))" \
        bash -c "test \"\$(docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf get pod $p -n $n -o jsonpath='{.status.phase}' 2>/dev/null)\" = $want"
}

restart_kubelet() {
    if node_exec systemctl restart kubelet >/dev/null 2>&1; then
        say "  kubelet restarted (systemctl)"
    else
        node_exec sh -c 'kill $(pidof kubelet) 2>/dev/null' || true
        say "  kubelet killed (kill)"
    fi
}

# ---------------------------------------------------------------- checks

step_failed=0

# must: run a command that has to work. On failure record it and return 1 so
# the step can stop with `|| return 1`.
must() {
    if "$@"; then return 0; fi
    say "  FAIL: $*"
    step_failed=1
    return 1
}

# check: assert something. Records a failure and keeps going.
check() {
    local desc=$1
    shift
    if "$@"; then
        say "  ok:   $desc"
    else
        say "  FAIL: $desc"
        step_failed=1
    fi
}

# ---------------------------------------------------------------- teardown

pre_images=""
pre_containers=""
pre_volumes=""

cleanup() {
    local rc=$?
    set +e
    say ""
    say "########## step 18: cleanup ##########"
    say "deleting everything this run created (script exit rc=$rc)..."

    if docker volume inspect "$TOOLS_VOL" >/dev/null 2>&1; then
        kind delete cluster --name "$CLUSTER" >/dev/null 2>&1
        kind delete cluster --name "$CLUSTER" >/dev/null 2>&1
    fi
    docker rm -f "$NODE" >/dev/null 2>&1

    local c
    while read -r c; do
        [[ -z "$c" ]] && continue
        if ! grep -qxF "$c" <<<"$pre_containers"; then
            case "$c" in
            ergon-nri* | "$CLUSTER" | "$NODE")
                say "  rm container $c"
                docker rm -f "$c" >/dev/null 2>&1
                ;;
            esac
        fi
    done < <(docker ps -a --format '{{.Names}}')

    local v
    while read -r v; do
        [[ -z "$v" ]] && continue
        # Only volumes this run created. A pre-existing ergon-* volume is
        # another workflow's cache: keep it. The kind tools volume is a run
        # artifact and is always ours.
        if ! grep -qxF "$v" <<<"$pre_volumes" || [[ "$v" == "$TOOLS_VOL" ]]; then
            say "  rm volume $v"
            docker volume rm -f "$v" >/dev/null 2>&1
        fi
    done < <(docker volume ls --format '{{.Name}}')

    local img
    while read -r img; do
        [[ -z "$img" || "$img" == "<none>:<none>" ]] && continue
        # Only images this run built or loaded. Never match on a shared name
        # like kindest/node or another workflow's stamped ergon-* image: those
        # belong to whoever pulled them, and deleting them breaks their run.
        if ! grep -qxF "$img" <<<"$pre_images" \
            || [[ "$img" == ergon-nri* || "$img" == "$PLUGIN_IMAGE" || "$img" == "$TEST_IMAGE" ]]; then
            say "  rmi $img"
            docker rmi -f "$img" >/dev/null 2>&1
        fi
    done < <(docker images --format '{{.Repository}}:{{.Tag}}')

    say ""
    say "=== what is left ==="
    say "--- kind clusters ---"
    if docker volume inspect "$TOOLS_VOL" >/dev/null 2>&1; then
        kind get clusters 2>/dev/null || echo "(none)"
    else
        echo "(kind tools volume gone)"
    fi
    say "--- docker ps -a ---"
    docker ps -a --format '{{.Names}}\t{{.Image}}\t{{.Status}}'
    say "--- docker images ---"
    docker images --format '{{.Repository}}:{{.Tag}}\t{{.Size}}'
    say "--- docker volume ls ---"
    docker volume ls --format '{{.Name}}'
    say ""
    say "--- pre-run snapshot (kept) ---"
    say "images:"
    printf '%s\n' "$pre_images" | sed '/^$/d' | sed 's/^/  /'
    say "containers:"
    printf '%s\n' "$pre_containers" | sed '/^$/d' | sed 's/^/  /'
    say "volumes:"
    printf '%s\n' "$pre_volumes" | sed '/^$/d' | sed 's/^/  /'
    exit $rc
}
trap cleanup EXIT INT TERM

# ---------------------------------------------------------------- results

results=()
fails=0

run_step() {
    local n=$1 name=$2 rc=0
    shift 2
    say ""
    say "########## step $n: $name ##########"
    set +e
    "$@"
    rc=$?
    set -e
    if (( rc == 0 )); then
        results+=("step $n: PASS  $name")
        say "step $n: PASS"
    else
        results+=("step $n: FAIL  $name")
        say "step $n: FAIL"
        fails=$((fails + 1))
    fi
}

# ================================================================ steps

step_preflight() {
    step_failed=0
    local ncpu
    ncpu=$(docker info --format '{{.NCPU}}' 2>/dev/null || echo 0)
    say "docker daemon CPUs: $ncpu"
    check "at least 6 CPUs in the Docker daemon" test "$ncpu" -ge 6
    docker version --format 'docker server {{.Server.Version}}' || step_failed=1
    return $step_failed
}

step1_build_load() {
    step_failed=0
    say "== unit tests, vet, gofmt, static build =="
    # --cap-add=SYS_NICE: SetTimerSlack writes another task's
    # /proc/PID/timerslack_ns, which the kernel refuses without CAP_SYS_NICE.
    # The OCI hook has the capability; the tests run where the write can work.
    # The admission policy the unit tests read lives in the sibling
    # nri-latency-pods module, mounted where scripts/check.sh mounts it.
    must docker run --rm --name "ergon-nri-check-e2e" --cap-add=SYS_NICE \
        -v "$ROOT":/src -v "$ROOT/../nri-latency-pods/admission":/admission:ro -w /src \
        -v "$GOMOD_VOL":/go/pkg/mod \
        "$GO_IMAGE" bash scripts/check.sh --local || return 1

    say "== build images =="
    must docker build -t "$PLUGIN_IMAGE" "$ROOT" || return 1
    must docker build -f "$E2E/Dockerfile" -t "$TEST_IMAGE" "$ROOT" || return 1
    say "plugin image size: $(docker image inspect "$PLUGIN_IMAGE" --format '{{.Size}}') bytes"
    say "test image size:   $(docker image inspect "$TEST_IMAGE" --format '{{.Size}}') bytes"

    say "== build kind $KIND_VERSION =="
    must docker volume create "$TOOLS_VOL" || return 1
    must docker run --rm --name "ergon-nri-kindbuild" \
        -v "$TOOLS_VOL":/out \
        -v "$GOMOD_VOL":/go/pkg/mod \
        "$GO_IMAGE" bash -c "CGO_ENABLED=0 GOBIN=/out go install sigs.k8s.io/kind@$KIND_VERSION" || return 1

    say "== kind create cluster =="
    must kind create cluster \
        --name "$CLUSTER" \
        --config /e2e/kind.yaml \
        --wait 180s || return 1

    say "== kind load images =="
    must kind load docker-image "$PLUGIN_IMAGE" "$TEST_IMAGE" --name "$CLUSTER" || return 1

    say "== node =="
    node_exec uname -r || step_failed=1
    node_exec containerd --version || step_failed=1
    node_exec runc --version || step_failed=1
    kube get nodes -o wide || step_failed=1
    return $step_failed
}

step2_deploy() {
    step_failed=0
    must docker cp "$ROOT/deploy/namespace.yaml" "$NODE:/root/namespace.yaml" || return 1
    must docker cp "$ROOT/deploy/daemonset.yaml" "$NODE:/root/daemonset.yaml" || return 1
    must docker cp "$MANIFESTS/configmap-e2e.yaml" "$NODE:/root/configmap.yaml" || return 1
    must docker cp "$MANIFESTS/ns.yaml" "$NODE:/root/ns.yaml" || return 1

    must kube label node "$NODE" ergon.io/shared-cpus=true --overwrite || return 1
    must kube apply -f /root/namespace.yaml || return 1
    must kube apply -f /root/ns.yaml || return 1
    must kube apply -f /root/configmap.yaml || return 1
    must kube apply -f /root/daemonset.yaml || return 1

    must wait_for 120 "DaemonSet pod Ready" \
        bash -c "docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf -n ergon-system get pods -l app=ergon-shared-cpus -o jsonpath='{.items[0].status.containerStatuses[0].ready}' | grep -q true" || return 1

    say "== plugin pod =="
    kube get pods -n ergon-system -o wide
    check "plugin container is ready" bash -c \
        "docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf -n ergon-system get pods -l app=ergon-shared-cpus -o jsonpath='{.items[0].status.containerStatuses[0].ready}' | grep -q true"

    say "== containerd NRI config =="
    local dump
    dump=$(node_exec containerd config dump) || return 1
    printf '%s\n' "$dump" | awk '/io.containerd.nri.v1.nri/,/^$/' | head -20 | sed 's/^/  | /'
    check "containerd NRI is enabled" bash -c "grep -q 'disable = false' <<<\"$dump\""
    check "containerd default_validator section present" bash -c "grep -q 'default_validator' <<<\"$dump\""
    return $step_failed
}

step3_allocatable() {
    step_failed=0
    # The device plugin can be registered while the kubelet has not yet
    # published the new extended resource on the node. Poll.
    wait_allocatable 4000 60 "allocatable = 4000 (sharedCpus 2 x oversubscription 2 x 1000)"
    return $step_failed
}

step4_pod_a() {
    step_failed=0
    must docker cp "$MANIFESTS/pod-a.yaml" "$NODE:/root/pod-a.yaml" || return 1
    must kube apply -f /root/pod-a.yaml || return 1
    must wait_phase a "$NS" Running 90 || return 1
    sleep 1

    local out
    out=$(probe a) || { say "  FAIL: pin-test probe did not run"; return 1; }
    printf '%s\n' "$out" | sed 's/^/  | /'
    printf '%s\n' "$out" >/tmp/ergon-nri-probe-a.txt

    local iso shared milli helper ois osh cores eff qmax
    iso=$(printf '%s\n' "$out" | kv ERGON_ISOLATED_CPUS)
    shared=$(printf '%s\n' "$out" | kv ERGON_SHARED_CPUS)
    cores=$(printf '%s\n' "$out" | kv ERGON_ISOLATED_CORES)
    milli=$(printf '%s\n' "$out" | kv ERGON_SHARED_MILLICPUS)
    helper=$(printf '%s\n' "$out" | kv ERGON_HELPER_CPUS)
    ois=$(printf '%s\n' "$out" | kv OPENSHIFT_ISOLATED_CPUS)
    osh=$(printf '%s\n' "$out" | kv OPENSHIFT_SHARED_CPUS)
    eff=$(printf '%s\n' "$out" | kv cpuset.cpus.effective)
    qmax=$(printf '%s\n' "$out" | kv cpu.max)
    say "  E=$iso  S=$shared  cores=$cores  M=$milli  eff=$eff"

    printf '%s\n' "$iso" >/tmp/e.txt
    printf '%s\n' "$shared" >/tmp/s.txt
    printf '%s\n' "$eff" >/tmp/eff.txt
    cat /tmp/e.txt /tmp/s.txt | cpus_of >/tmp/eu.txt

    check "cpuset.cpus.effective == E union S" \
        cmp -s <(cpus_of </tmp/eff.txt) /tmp/eu.txt
    check "|E| = 2 (cpu: 2)" test "$(cpus_of </tmp/e.txt | wc -l)" -eq 2
    check "|S| = 1 (250 millicpus)" test "$(cpus_of </tmp/s.txt | wc -l)" -eq 1
    check "E and S are disjoint" \
        test "$(comm -12 <(cpus_of </tmp/e.txt) <(cpus_of </tmp/s.txt) | wc -l)" -eq 0

    check "ERGON_SHARED_MILLICPUS = 250" test "$milli" = "250"
    check "ERGON_HELPER_CPUS = ERGON_SHARED_CPUS" test "$helper" = "$shared"
    check "OPENSHIFT_ISOLATED_CPUS is E as a cpulist" \
        cmp -s <(printf '%s\n' "$ois" | cpus_of) <(cpus_of </tmp/e.txt)
    check "OPENSHIFT_SHARED_CPUS is S as a cpulist" \
        cmp -s <(printf '%s\n' "$osh" | cpus_of) <(cpus_of </tmp/s.txt)
    check "ERGON_ISOLATED_CPUS is a comma list (no ranges)" bash -c "! grep -q -- '-' <<<\"$iso\""
    check "ERGON_SHARED_CPUS is a comma list (no ranges)" bash -c "! grep -q -- '-' <<<\"$shared\""

    printf '%s\n' "$cores" >/tmp/cores.txt
    tr ';' '\n' </tmp/cores.txt | tr ',' '\n' | cpus_of >/tmp/core-cpus.txt
    check "ERGON_ISOLATED_CORES groups partition E" \
        cmp -s /tmp/core-cpus.txt <(cpus_of </tmp/e.txt)

    local g ok=1 first sibs
    while IFS= read -r g; do
        [[ -z "$g" ]] && continue
        first=$(printf '%s\n' "$g" | cpus_of | head -1)
        sibs=$(kube exec a -n "$NS" -- cat "/sys/devices/system/cpu/cpu${first}/topology/thread_siblings_list" | tr -d '\r\n') || { ok=0; continue; }
        if ! cmp -s <(printf '%s\n' "$g" | cpus_of) \
            <(printf '%s\n' "$sibs" | cpus_of | comm -12 - <(cpus_of </tmp/e.txt)); then
            say "  FAIL: core group $g is not the sysfs siblings of cpu $first ($sibs) intersected with E"
            ok=0
        else
            say "  ok:   core group $g matches sysfs siblings of cpu $first ($sibs)"
        fi
    done < <(tr ';' '\n' </tmp/cores.txt)
    [[ $ok -eq 1 ]] || step_failed=1

    check "cpu.max = max 100000 (no CFS quota)" test "$qmax" = "max 100000"
    return $step_failed
}

step5_hook_effect() {
    step_failed=0
    need_layout || return 1
    must wait_for 30 "hold startup lines" \
        bash -c "docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf logs a -n $NS --tail=-1 | grep -q '^hotchild.affinity='" || return 1
    local logs init pthread samples s
    logs=$(kube logs a -n "$NS" --tail=-1) || return 1
    printf '%s\n' "$logs" | sed 's/^/  | /'
    init=$(printf '%s\n' "$logs" | awk -F= '/^init\.affinity=/{print $2; exit}')
    pthread=$(printf '%s\n' "$logs" | awk -F= '/^pthread\.affinity=/{print $2; exit}')
    samples=$(printf '%s\n' "$logs" | awk '/^samples=/{print; exit}')
    s=$(cpus_of </tmp/s.txt | paste -sd, -)
    say "  S=$s  init.affinity=$init  pthread.affinity=$pthread  $samples"
    check "PID 1 affinity = S ($s)" test "$init" = "$s"
    check "new pthread affinity = S" test "$pthread" = "$s"
    check "1000 sched_getcpu() samples all on S" bash -c "grep -q 'off_shared=0' <<<\"$samples\""

    local helper_aff hotchild e0
    helper_aff=$(printf '%s\n' "$logs" | awk -F= '/^helper.affinity=/{print $2; exit}')
    hotchild=$(printf '%s\n' "$logs" | awk -F= '/^hotchild.affinity=/{print $2; exit}')
    e0=$(cpus_of </tmp/e.txt | sed -n 1p)
    say "  helper.affinity=$helper_aff  hotchild.affinity=$hotchild  hot=$e0"
    check "helper created before the hot pin keeps S" test "$helper_aff" = "$s"
    check "helper created by the pinned hot thread inherits the hot mask" test "$hotchild" = "$e0"

    local tself tpthread
    tself=$(printf '%s\n' "$logs" | awk -F= '/^timerslack\.self=/{print $2; exit}')
    tpthread=$(printf '%s\n' "$logs" | awk -F= '/^timerslack\.pthread=/{print $2; exit}')
    say "  timerslack.self=$tself  timerslack.pthread=$tpthread"
    check "timer slack = 1 ns on PID 1" test "$tself" = "1"
    check "timer slack = 1 ns on a new pthread (inherited)" test "$tpthread" = "1"
    return $step_failed
}

step6_pin_by_index() {
    step_failed=0
    need_layout || return 1
    local e0 e1 out
    e0=$(cpus_of </tmp/e.txt | sed -n 1p)
    e1=$(cpus_of </tmp/e.txt | sed -n 2p)
    say "  E[0]=$e0 E[1]=$e1"

    out=$(kube exec a -n "$NS" -- pin-test sample 0 2) || return 1
    say "  | $out"
    check "isolated:0 held 2 s on cpu $e0" bash -c "grep -q 'cpu=$e0' <<<\"$out\" && grep -q 'bad=0' <<<\"$out\""

    out=$(kube exec a -n "$NS" -- pin-test sample 1 2) || return 1
    say "  | $out"
    check "isolated:1 held 2 s on cpu $e1" bash -c "grep -q 'cpu=$e1' <<<\"$out\" && grep -q 'bad=0' <<<\"$out\""

    out=$(kube exec a -n "$NS" -- pin) || return 1
    printf '%s\n' "$out" | sed 's/^/  | pin: /'
    check "examples/pin.c pins index 0 to cpu $e0" bash -c "grep -q 'isolated index 0 = cpu $e0' <<<\"$out\""
    check "examples/pin.c pins index 1 to cpu $e1" bash -c "grep -q 'isolated index 1 = cpu $e1' <<<\"$out\""

    out=$(kube exec a -n "$NS" -- pin-rs) || return 1
    printf '%s\n' "$out" | sed 's/^/  | pin-rs: /'
    check "examples/pin.rs pins index 0 to cpu $e0" bash -c "grep -q 'isolated index 0 = cpu $e0' <<<\"$out\""
    check "examples/pin.rs pins index 1 to cpu $e1" bash -c "grep -q 'isolated index 1 = cpu $e1' <<<\"$out\""
    return $step_failed
}

step7_ergon_cpus() {
    step_failed=0
    need_layout || return 1
    local e0 e1 s0 out rc
    e0=$(cpus_of </tmp/e.txt | sed -n 1p)
    e1=$(cpus_of </tmp/e.txt | sed -n 2p)
    s0=$(cpus_of </tmp/s.txt | sed -n 1p)

    out=$(kube exec a -n "$NS" -- ergon-cpus \
        --env AERON_RECEIVER_CPU_AFFINITY=isolated:0 \
        --env AERON_SENDER_CPU_AFFINITY=isolated:1 \
        --env AERON_CONDUCTOR_CPU_AFFINITY=shared:0 \
        -- sh -c 'echo RECV=$AERON_RECEIVER_CPU_AFFINITY SEND=$AERON_SENDER_CPU_AFFINITY COND=$AERON_CONDUCTOR_CPU_AFFINITY') || return 1
    say "  | $out"
    check "Aeron vars become real CPUs RECV=$e0 SEND=$e1 COND=$s0" \
        bash -c "grep -q 'RECV=$e0 SEND=$e1 COND=$s0' <<<\"$out\""

    rc=0
    out=$(kube exec a -n "$NS" -- ergon-cpus --env AERON_RECEIVER_CPU_AFFINITY=isolated:2 -- true 2>&1) || rc=$?
    say "  | isolated:2 -> rc=$rc  $out"
    check "isolated:2 exits 1" test "$rc" -eq 1
    check "isolated:2 says out of range" bash -c "grep -q 'out of range' <<<\"$out\""

    rc=0
    out=$(kube exec a -n "$NS" -- ergon-cpus --env AERON_RECEIVER_CPU_AFFINITY=bad:0 -- true 2>&1) || rc=$?
    say "  | bad:0 -> rc=$rc  $out"
    check "bad:0 exits 1" test "$rc" -eq 1
    check "bad:0 says unknown spec kind" bash -c "grep -q 'unknown spec kind' <<<\"$out\""

    rc=0
    out=$(kube exec a -n "$NS" -- sh -c 'AERON_DRIVER_CPUSET_AFFINITY=true ergon-cpus --env AERON_RECEIVER_CPU_AFFINITY=isolated:0 -- true' 2>&1) || rc=$?
    say "  | AERON_DRIVER_CPUSET_AFFINITY=true -> rc=$rc  $out"
    check "cpuset-affinity mode exits 1" test "$rc" -eq 1
    check "cpuset-affinity mode names the variable" \
        bash -c "grep -q 'AERON_DRIVER_CPUSET_AFFINITY' <<<\"$out\""
    return $step_failed
}

step8_kubelet_restart() {
    step_failed=0
    need_layout || return 1
    local before after out eff
    before=$(plugin_logs | grep -c 'ergon-e2e/a' || true)
    restart_kubelet
    say "waiting 30 s for the kubelet to re-write cpusets..."
    sleep 30
    must wait_for 60 "node Ready after kubelet restart" \
        bash -c "docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf get node $NODE -o jsonpath='{.status.conditions[?(@.type==\"Ready\")].status}' | grep -q True" || return 1

    after=$(plugin_logs | grep -c 'ergon-e2e/a' || true)
    say "  log lines naming ergon-e2e/a: $before -> $after"
    check "plugin logged an update for A" test "$after" -gt "$before"
    plugin_logs | grep 'ergon-e2e/a' | tail -3 | sed 's/^/  | /'

    out=$(probe a) || true
    eff=$(printf '%s\n' "$out" | kv cpuset.cpus.effective)
    say "  cpuset.cpus.effective after restart = $eff"
    check "cpuset still E union S" cmp -s <(printf '%s\n' "$eff" | cpus_of) /tmp/eu.txt

    check "no VIOLATION lines so far" bash -c \
        "! docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf logs a -n $NS --tail=-1 | grep -q VIOLATION"
    return $step_failed
}

step9_plugin_down() {
    step_failed=0
    need_layout || return 1
    # Demonstrated limitation, not a passing isolation claim. While the
    # plugin is down the kubelet rewrites the cpuset to the exclusive CPUs
    # and helpers run there. Recovery does not prove the contract held
    # during the outage. required-plugins protects creation, not a pod that
    # is already running.
    # Park the DaemonSet on a nodeSelector that matches nothing. The node
    # label stays on, so the test pods below still schedule; only the plugin
    # goes away. (Removing the node label would unschedulable every pod that
    # carries nodeSelector ergon.io/shared-cpus=true.)
    must kube -n ergon-system patch ds ergon-shared-cpus --type=json \
        -p '[{"op":"replace","path":"/spec/template/spec/nodeSelector","value":{"ergon.io/shared-cpus":"off"}}]' >/dev/null || return 1
    say "DaemonSet parked; waiting for the plugin pod to go away"
    must wait_for 60 "plugin pod gone" \
        bash -c "! docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf -n ergon-system get pods -l app=ergon-shared-cpus -o name | grep -q ." || return 1

    restart_kubelet
    say "waiting 30 s for the kubelet to re-write cpusets without the plugin..."
    sleep 30

    local out eff logs
    out=$(probe a) || true
    eff=$(printf '%s\n' "$out" | kv cpuset.cpus.effective)
    say "  cpuset.cpus.effective while down = $eff"
    check "cpuset shrank to E" cmp -s <(printf '%s\n' "$eff" | cpus_of) <(cpus_of </tmp/e.txt)

    logs=$(kube logs a -n "$NS" --tail=-1 || true)
    check "helper reported violations (the documented harm)" \
        bash -c "grep -q 'VIOLATION helper' <<<\"$logs\""
    check "hot thread did not move" \
        bash -c "! grep -q 'VIOLATION hot' <<<\"$logs\""
    say "  VIOLATION lines so far: $(printf '%s\n' "$logs" | grep -c VIOLATION || true)"
    printf '%s\n' "$logs" | grep VIOLATION | head -3 | sed 's/^/  | /'

    must docker cp "$MANIFESTS/pod-b.yaml" "$NODE:/root/pod-b.yaml" || return 1
    must docker cp "$MANIFESTS/pod-c.yaml" "$NODE:/root/pod-c.yaml" || return 1
    must docker cp "$MANIFESTS/pod-d.yaml" "$NODE:/root/pod-d.yaml" || return 1
    kube apply -f /root/pod-b.yaml >/dev/null || true
    kube apply -f /root/pod-c.yaml >/dev/null || true
    kube apply -f /root/pod-d.yaml >/dev/null || true
    # After two kubelet restarts these take a while to settle. Wait for the
    # state instead of sleeping: B is refused by containerd's default
    # validator, C is plain and must start.
    must wait_for 90 "pod B is CreateContainerError" \
        bash -c "test \"\$(docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf get pod b -n $NS -o jsonpath='{.status.containerStatuses[0].state.waiting.reason}' 2>/dev/null)\" = CreateContainerError" || return 1
    must wait_phase c "$NS" Running 90 || return 1

    local breason bmsg
    breason=$(kube get pod b -n "$NS" -o jsonpath='{.status.containerStatuses[0].state.waiting.reason}' 2>/dev/null || echo Missing)
    bmsg=$(kube get pod b -n "$NS" -o jsonpath='{.status.containerStatuses[0].state.waiting.message}' 2>/dev/null || true)
    say "  pod B: reason=$breason"
    say "    message: $bmsg"
    check "pod B is CreateContainerError" test "$breason" = "CreateContainerError"
    check "pod B message names the missing plugin" bash -c "grep -qi 'ergon-shared-cpus' <<<\"$bmsg\""

    say "  pod C: phase=$(pod_phase c "$NS")"
    check "pod C (plain) starts" test "$(pod_phase c "$NS")" = "Running"

    say "  pod D (asks the resource): phase=$(pod_phase d "$NS")"
    kube describe pod d -n "$NS" 2>/dev/null | tail -15 | sed 's/^/  | /'
    kube get events -n "$NS" --field-selector involvedObject.name=d 2>/dev/null | tail -8 | sed 's/^/  | /' || true
    check "pod D is not Running (the resource cannot allocate)" \
        bash -c "! test \"\$(docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf get pod d -n $NS -o jsonpath='{.status.phase}' 2>/dev/null)\" = Running"
    check "pod D event says Insufficient ergon.io/shared-millicpus" \
        bash -c "docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf get events -n $NS --field-selector involvedObject.name=d 2>/dev/null | grep -qi 'Insufficient ergon.io/shared-millicpus'"

    # Forge pods: a pod author can write env and the opt-in annotation. They
    # must never make the plugin widen a cpuset onto another pod's exclusive
    # CPUs. Created here, while the plugin is down, and checked in step 10.
    local e0
    e0=$(cpus_of </tmp/e.txt | sed -n 1p)
    say "  forging pods that claim pod A's exclusive CPU $e0"
    must sed "s/@@E0@@/$e0/g" "$MANIFESTS/forge-env.yaml.tmpl" >"$E2E/forge-env.yaml" || return 1
    must sed "s/@@E0@@/$e0/g" "$MANIFESTS/forge-annotation.yaml.tmpl" >"$E2E/forge-annotation.yaml" || return 1
    must docker cp "$E2E/forge-env.yaml" "$NODE:/root/forge-env.yaml" || return 1
    must docker cp "$E2E/forge-annotation.yaml" "$NODE:/root/forge-annotation.yaml" || return 1
    kube apply -f /root/forge-env.yaml >/dev/null || true
    kube apply -f /root/forge-annotation.yaml >/dev/null || true
    must wait_phase forge-env "$NS" Running 90 || return 1
    must wait_phase forge-annotation "$NS" Running 90 || return 1
    say "  forge pods: phase=$(pod_phase forge-env "$NS") / $(pod_phase forge-annotation "$NS") (plain Guaranteed pods; each took one exclusive CPU)"
    return $step_failed
}

step10_plugin_back() {
    step_failed=0
    need_layout || return 1
    must kube -n ergon-system patch ds ergon-shared-cpus --type=json \
        -p '[{"op":"replace","path":"/spec/template/spec/nodeSelector","value":{"ergon.io/shared-cpus":"true"}}]' >/dev/null || return 1
    say "DaemonSet restored; waiting for the plugin"
    must wait_for 120 "plugin pod Ready" \
        bash -c "docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf -n ergon-system get pods -l app=ergon-shared-cpus -o jsonpath='{.items[0].status.containerStatuses[0].ready}' | grep -q true" || return 1
    sleep 8

    local out eff logs v1 v2
    out=$(probe a) || true
    eff=$(printf '%s\n' "$out" | kv cpuset.cpus.effective)
    say "  cpuset.cpus.effective after restore = $eff"
    check "Synchronize re-widened A to E union S" cmp -s <(printf '%s\n' "$eff" | cpus_of) /tmp/eu.txt

    logs=$(kube logs a -n "$NS" --tail=-1 || true)
    check "hot thread never moved" bash -c "! grep -q 'VIOLATION hot' <<<\"$logs\""
    v1=$(printf '%s\n' "$logs" | grep -c VIOLATION || true)
    sleep 3
    logs=$(kube logs a -n "$NS" --tail=-1 || true)
    v2=$(printf '%s\n' "$logs" | grep -c VIOLATION || true)
    say "  VIOLATION count after restore: $v1 -> $v2"
    check "helper is back on S (no new violations)" test "$v2" -eq "$v1"

    must wait_phase b "$NS" Running 90 || return 1
    check "pod B starts once the plugin is back" test "$(pod_phase b "$NS")" = "Running"

    # The forge pods from step 9 claimed pod A's exclusive CPU through env (and
    # one of them through the opt-in annotation too). Synchronize must not have
    # widened either of them: the union only ever adds CPUs from sharedCpus.
    local e0
    e0=$(cpus_of </tmp/e.txt | sed -n 1p)
    for f in forge-env forge-annotation; do
        eff=$(probe "$f" | kv cpuset.cpus.effective)
        say "  $f: cpuset.cpus.effective=$eff (claimed $e0)"
        # cpus_of is a shell function, so these must not go through `bash -c`
        # (a subshell cannot see it and the check would pass without testing).
        check "$f keeps only its own exclusive CPU (1 CPU)" \
            test "$(printf '%s\n' "$eff" | cpus_of | wc -l)" -eq 1
        check "$f was not widened onto pod A's CPU $e0" \
            test "$(printf '%s\n' "$eff" | cpus_of | grep -cx "$e0")" -eq 0
        check "$f holds no shared CPU" \
            test "$(printf '%s\n' "$eff" | cpus_of | grep -cxE '0|1')" -eq 0
    done
    plugin_logs | grep -iE 'forge|not re-widen|unhandled|outside sharedCpus' | tail -6 | sed 's/^/  | /' || true

    plugin_logs | grep -E 'sync|update' | tail -5 | sed 's/^/  | /'
    return $step_failed
}

step11_balance() {
    step_failed=0
    need_layout || return 1
    # Only A may hold shared CPUs here. D asked for 250 millicpus while the
    # plugin was down and took a shared CPU when it came back; drop it and C,
    # and the forge pods from step 9 (each holds one exclusive CPU).
    drop_pods c d forge-env forge-annotation
    must docker cp "$MANIFESTS/balance-e.yaml" "$NODE:/root/e.yaml" || return 1
    must docker cp "$MANIFESTS/balance-f.yaml" "$NODE:/root/f.yaml" || return 1
    must kube apply -f /root/e.yaml || return 1
    must kube apply -f /root/f.yaml || return 1
    must wait_phase e "$NS" Running 90 || return 1
    must wait_phase f "$NS" Running 90 || return 1
    sleep 1

    local se sf
    se=$(probe e | kv ERGON_SHARED_CPUS)
    sf=$(probe f | kv ERGON_SHARED_CPUS)
    say "  A holds S=$(cpus_of </tmp/s.txt | paste -sd, -); E -> S=$se; F -> S=$sf"
    # The plugin balances over sharedCpus: each pod gets exactly one shared
    # CPU, both from sharedCpus, and the two land on different ones (the
    # less-loaded CPU is chosen first). Which pod gets which CPU is the
    # container create order, which is not the plugin's to fix.
    check "E got exactly one shared CPU" test "$(printf '%s\n' "$se" | cpus_of | wc -l)" -eq 1
    check "F got exactly one shared CPU" test "$(printf '%s\n' "$sf" | cpus_of | wc -l)" -eq 1
    check "E's shared CPU is in sharedCpus (0,1)" test "$(printf '%s\n' "$se" | cpus_of | grep -cxE '0|1')" -eq 1
    check "F's shared CPU is in sharedCpus (0,1)" test "$(printf '%s\n' "$sf" | cpus_of | grep -cxE '0|1')" -eq 1
    check "E and F landed on different shared CPUs (balanced)" test "$se" != "$sf"
    return $step_failed
}

step12_refusals() {
    step_failed=0
    local f ns got msg
    # A holds two CPUs and B finished its job in step 10. Drop everything this
    # step does not need: the refusal pods must be scheduled to reach the
    # plugin, and one node is tight. refuse-too-many is created first and alone
    # because it is the one that has to get a node before the plugin can say no;
    # the other four ask for little.
    drop_pods b e f
    # refuse-too-many first and alone: it needs a whole CPU and has to reach
    # the plugin before the other four take the rest of the node.
    must docker cp "$MANIFESTS/refuse-too-many.yaml" "$NODE:/root/refuse-too-many.yaml" || return 1
    must kube apply -f /root/refuse-too-many.yaml || return 1
    wait_container_reason refuse-too-many "$NS" CreateContainerError 60 || true
    for f in refuse-burstable refuse-frac refuse-ns refuse-env; do
        must docker cp "$MANIFESTS/$f.yaml" "$NODE:/root/$f.yaml" || return 1
        must kube apply -f "/root/$f.yaml" || return 1
    done

    for f in refuse-too-many refuse-burstable refuse-frac refuse-ns refuse-env; do
        ns=$NS
        [[ $f == refuse-ns ]] && ns=default
        # Five pods at once: the last one can still be waiting for the
        # scheduler or for Allocate when the first ones are already refused.
        wait_container_reason "$f" "$ns" CreateContainerError 60 || true
        got=$(kube get pod "$f" -n "$ns" -o jsonpath='{.status.containerStatuses[0].state.waiting.reason}' 2>/dev/null || echo Missing)
        msg=$(kube get pod "$f" -n "$ns" -o jsonpath='{.status.containerStatuses[0].state.waiting.message}' 2>/dev/null || true)
        say "  $f: reason=$got"
        say "    message: $msg"
        if [[ "$got" != "CreateContainerError" ]]; then
            say "  $f: phase=$(pod_phase "$f" "$ns") (expected a refusal)"
            kube get pod "$f" -n "$ns" -o yaml 2>/dev/null | sed -n '/status:/,$p' | head -40 | sed 's/^/  | /' || true
        fi
        check "$f is CreateContainerError" test "$got" = "CreateContainerError"
        [[ "$got" == "CreateContainerError" ]] || continue
        check "$f message comes from the plugin" bash -c "grep -q 'ergon-shared-cpus:' <<<\"$msg\""
        case $f in
        refuse-ns)
            check "$f rule 1 text" bash -c "grep -q 'namespace default may not use shared CPUs' <<<\"$msg\"" ;;
        refuse-too-many)
            check "$f rule 6 text" bash -c "grep -q 'millicpus need' <<<\"$msg\"" ;;
        refuse-env)
            check "$f rule 7 text" bash -c "grep -q 'is set by hand' <<<\"$msg\"" ;;
        refuse-burstable | refuse-frac)
            check "$f is a 4.3 rule text" bash -c \
                "grep -qE 'no cpuset from the kubelet|CPU quota is set|cpuset is not exclusive' <<<\"$msg\"" ;;
        esac
    done
    return $step_failed
}

step13_scheduler() {
    step_failed=0
    must docker cp "$MANIFESTS/sched-too-big.yaml" "$NODE:/root/sched.yaml" || return 1
    must kube apply -f /root/sched.yaml || return 1
    sleep 10
    local phase ev
    phase=$(pod_phase sched-too-big "$NS")
    say "  phase = $phase"
    check "pod is Pending" test "$phase" = "Pending"
    ev=$(kube describe pod sched-too-big -n "$NS" | grep -iE 'Insufficient|FailedScheduling' || true)
    say "  | $ev"
    check "event says Insufficient ergon.io/shared-millicpus" \
        bash -c "grep -qi 'Insufficient ergon.io/shared-millicpus' <<<\"$ev\""
    return $step_failed
}

step14_api_integer() {
    step_failed=0
    must docker cp "$MANIFESTS/api-fractional.yaml" "$NODE:/root/api.yaml" || return 1
    local out rc=0
    out=$(kube_in "$MANIFESTS/api-fractional.yaml" apply -f - 2>&1) || rc=$?
    say "  rc=$rc"
    printf '%s\n' "$out" | sed 's/^/  | /'
    check "apply fails" test "$rc" -ne 0
    check "error says must be an integer" bash -c "grep -qi 'must be an integer' <<<\"$out\""
    return $step_failed
}

step15_opt_out() {
    step_failed=0
    drop_pods refuse-burstable refuse-frac refuse-ns refuse-env refuse-too-many
    must docker cp "$MANIFESTS/pod-optout.yaml" "$NODE:/root/optout.yaml" || return 1
    must kube apply -f /root/optout.yaml || return 1
    must wait_phase optout "$NS" Running 90 || return 1
    must wait_for 30 "optout startup lines" \
        bash -c "docker exec $NODE kubectl --kubeconfig /etc/kubernetes/admin.conf logs optout -n $NS --tail=-1 | grep -q '^init.affinity='" || return 1
    local out init eff
    out=$(kube logs optout -n "$NS" --tail=-1) || return 1
    printf '%s\n' "$out" | head -5 | sed 's/^/  | /'
    init=$(printf '%s\n' "$out" | awk -F= '/^init\.affinity=/{print $2; exit}')
    eff=$(probe optout | kv cpuset.cpus.effective)
    say "  init.affinity=$init  cpuset.cpus.effective=$eff"
    check "PID 1 affinity is the whole cpuset" \
        cmp -s <(printf '%s\n' "$init" | cpus_of) <(printf '%s\n' "$eff" | cpus_of)
    return $step_failed
}

step16_fallback() {
    step_failed=0
    drop_pods optout
    must docker cp "$MANIFESTS/pod-fallback.yaml" "$NODE:/root/fallback.yaml" || return 1
    must kube apply -f /root/fallback.yaml || return 1
    must wait_phase fallback "$NS" Running 90 || return 1
    sleep 1
    local out iso shared eff milli
    out=$(probe fallback) || true
    printf '%s\n' "$out" | sed 's/^/  | /'
    iso=$(printf '%s\n' "$out" | kv ERGON_ISOLATED_CPUS)
    shared=$(printf '%s\n' "$out" | kv ERGON_SHARED_CPUS)
    eff=$(printf '%s\n' "$out" | kv cpuset.cpus.effective)
    milli=$(printf '%s\n' "$out" | kv ERGON_SHARED_MILLICPUS)
    printf '%s\n' "$iso" >/tmp/e2.txt
    printf '%s\n' "$shared" >/tmp/s2.txt
    check "|E| = 2" test "$(cpus_of </tmp/e2.txt | wc -l)" -eq 2
    check "|S| = 1" test "$(cpus_of </tmp/s2.txt | wc -l)" -eq 1
    check "cpuset.cpus.effective = E union S" \
        cmp -s <(printf '%s\n' "$eff" | cpus_of) <(cat /tmp/e2.txt /tmp/s2.txt | cpus_of)
    check "ERGON_SHARED_MILLICPUS = 250 from the annotation" test "$milli" = "250"
    return $step_failed
}

step17_capacity() {
    step_failed=0
    drop_pods fallback
    must docker cp "$MANIFESTS/configmap-cap50.yaml" "$NODE:/root/configmap-cap50.yaml" || return 1
    must kube apply -f /root/configmap-cap50.yaml || return 1
    must kube rollout restart ds/ergon-shared-cpus -n ergon-system || return 1
    must kube rollout status ds/ergon-shared-cpus -n ergon-system --timeout=120s || return 1
    # The new plugin registers a different device count for the same resource
    # name; the kubelet publishes that on the node in its own time.
    wait_allocatable 100000 90 "allocatable with oversubscription 50"

    must docker cp "$MANIFESTS/cap-check.yaml" "$NODE:/root/cap.yaml" || return 1
    must kube apply -f /root/cap.yaml || return 1
    must wait_phase cap-check "$NS" Running 90 || return 1
    check "a 250-millicpus pod runs at 100000 capacity" test "$(pod_phase cap-check "$NS")" = "Running"

    must kube apply -f /root/configmap.yaml || return 1
    must kube rollout restart ds/ergon-shared-cpus -n ergon-system || return 1
    must kube rollout status ds/ergon-shared-cpus -n ergon-system --timeout=120s || return 1
    wait_allocatable 4000 90 "allocatable back to 4000"
    return $step_failed
}

# kube_in is used by step 14: pipe a host file into kubectl on the node.
kube_in() {
    local file=$1
    shift
    docker exec -i "$NODE" kubectl --kubeconfig /etc/kubernetes/admin.conf "$@" <"$file"
}

# ================================================================ main

say "ergon-shared-cpus e2e on kind"
say "module root: $ROOT"
say "docker server: $(docker version --format '{{.Server.Version}}' 2>/dev/null || echo '?')"

pre_images=$(docker images --format '{{.Repository}}:{{.Tag}}' | sort)
pre_containers=$(docker ps -a --format '{{.Names}}' | sort)
pre_volumes=$(docker volume ls --format '{{.Name}}' | sort)

run_step 0 "preflight: at least 6 CPUs" step_preflight
run_step 1 "unit tests, build images, kind load" step1_build_load
run_step 2 "deploy namespace, ConfigMap, DaemonSet" step2_deploy
run_step 3 "node allocatable = 4000" step3_allocatable
run_step 4 "pod A layout and env" step4_pod_a
run_step 5 "hook effect: PID 1, pthread, 1000 samples" step5_hook_effect
run_step 6 "pin by index (C and Rust)" step6_pin_by_index
run_step 7 "ergon-cpus --env translation and refusals" step7_ergon_cpus
run_step 8 "kubelet restart keeps the union" step8_kubelet_restart
run_step 9 "plugin down: harm and fail closed" step9_plugin_down
run_step 10 "plugin back: Synchronize re-widens" step10_plugin_back
run_step 11 "balance over shared CPUs" step11_balance
run_step 12 "admission refusals" step12_refusals
run_step 13 "scheduler: 4001 is Pending" step13_scheduler
run_step 14 "API: 500m is not an integer" step14_api_integer
run_step 15 "opt-out keeps the whole cpuset" step15_opt_out
run_step 16 "annotation fallback" step16_fallback
run_step 17 "capacity: oversubscription 50 = 100000" step17_capacity

say ""
say "================ summary ================"
printf '%s\n' "${results[@]}"
say "fails: $fails"
say "plugin image size: $(docker image inspect "$PLUGIN_IMAGE" --format '{{.Size}}' 2>/dev/null || echo n/a) bytes"
say "test image size:   $(docker image inspect "$TEST_IMAGE" --format '{{.Size}}' 2>/dev/null || echo n/a) bytes"
if (( fails > 0 )); then
    say "e2e: FAIL"
    exit 1
fi
say "e2e: PASS"
