#!/usr/bin/env bash
# docker-test.sh [--arch amd64|arm64] [--in-container]: ergon-io's
# fstack-spin end to end (io/tests/fstack.rs) without a NIC. On the host it
# runs itself in io/benches/fstack's image (F-Stack and its DPDK under
# /opt/fstack, built by build-fstack.sh), the repo mounted at /w; IMAGE
# names a prebuilt one instead of building it. Inside: builds the test with
# F-Stack linked, joins two F-Stack processes by a veth pair (DPDK's
# af_packet PMD, no hugepages), starts the peer (FSTACK_ROLE=server) and
# runs the client against it; both must pass.
#
# amd64 on an Apple Silicon host runs emulated: the latencies it prints
# mean nothing there, only that the path works. CARGO_TARGET_DIR and
# CARGO_HOME are passed through (one cache volume: CACHE_VOLUME).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
arch=amd64
inside=false
while [[ $# -gt 0 ]]; do
    case $1 in
    --arch) arch=$2; shift 2 ;;
    --in-container) inside=true; shift ;;
    *) echo "usage: $0 [--arch amd64|arm64] [--in-container]" >&2; exit 2 ;;
    esac
done

if ! $inside; then
    image=${IMAGE:-ergon-fstack-test:$arch}
    if [[ -z ${IMAGE:-} ]]; then
        docker build --platform "linux/$arch" -t "$image" "$repo/io/benches/fstack"
    fi
    vol=()
    if [[ -n ${CACHE_VOLUME:-} ]]; then
        vol=(-v "$CACHE_VOLUME":/cache -e CARGO_TARGET_DIR=/cache/target-$arch -e CARGO_HOME=/cache/cargo)
    fi
    exec docker run --rm --platform "linux/$arch" --privileged "${vol[@]}" -v "$repo":/w "$image" \
        bash /w/io/tests/fstack/docker-test.sh --arch "$arch" --in-container
fi

export PATH=/usr/local/cargo/bin:$PATH
cd /w
bin=$(FSTACK_PREFIX=/opt/fstack cargo test -p ergon-io --features fstack --test fstack --no-run \
    --message-format=json | python3 -c '
import json, sys
for line in sys.stdin:
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m["target"]["name"] == "fstack" and m.get("executable"):
        print(m["executable"])')
[[ -x $bin ]] || { echo "docker-test: no test binary" >&2; exit 1; }
out=$(mktemp -d)
ip link del va 2>/dev/null || true
ip link add va type veth peer name vb
ip link set va up
ip link set vb up
sysctl -qw net.ipv6.conf.va.disable_ipv6=1 net.ipv6.conf.vb.disable_ipv6=1
server_pid=
cleanup() {
    [[ -z $server_pid ]] || kill -9 "$server_pid" 2>/dev/null || true
    ip link del va 2>/dev/null || true
}
trap cleanup EXIT
fail() {
    echo "docker-test: FAIL: $*" >&2
    for f in server client; do echo "--- $f.log" >&2; grep -v "^\[" "$out/$f.log" | tail -60 >&2 || true; done
    exit 1
}
run() { # role iface addr lcore
    FSTACK_ROLE=$1 FSTACK_IFACE=$2 FSTACK_ADDR=$3 FSTACK_LCORE=$4 FSTACK_SHARE=$out \
        ERGON_HOST_CHECKS=off "$bin" --exact fstack_end_to_end --nocapture --test-threads=1
}
run server vb 10.9.0.2 1 >"$out/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 120); do
    grep -qx 'fstack-test: ready' "$out/server.log" && break
    kill -0 "$server_pid" 2>/dev/null || fail "the server exited before ready"
    sleep 1
done
grep -qx 'fstack-test: ready' "$out/server.log" || fail "the server was not ready in 120 s"
timeout 600 bash -c "$(declare -f run); bin=$bin out=$out; run client va 10.9.0.1 2" \
    >"$out/client.log" 2>&1 || fail "the client failed"
wait "$server_pid" || fail "the server failed"
server_pid=
grep -h '^fstack-test:' "$out/server.log" "$out/client.log"
grep -q 'fstack-test: client ok' "$out/client.log" || fail "no client ok line"
echo "docker-test: ok ($arch)"
