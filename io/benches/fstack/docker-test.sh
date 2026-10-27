#!/usr/bin/env bash
# docker-test.sh [--in-container]: fstack_lat end to end without a NIC. On
# the host it builds this directory's image (linux/amd64: F-Stack v2.0 does
# not build on arm64, build-fstack.sh) and runs itself inside it with the
# repo mounted; inside, it builds fstack_lat against /opt/fstack, joins two
# F-Stack processes by a veth pair (DPDK's af_packet PMD, no hugepages) and
# runs one ping/pong point at RATE msg/s, then checks what the runner relies
# on: one `ready` line from the pong, one 33-column row from the ping with
# every ping answered, both manifests, the stack's own TCP_INFO (established,
# timestamps, a sane MSS and RTT) and the config every key was applied from.
#
# Slow under emulation (an Apple Silicon host): the latencies mean nothing,
# only that the arm works. RATE (1000), DURATION (5), STREAMS (1).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)

if [[ ${1:-} != --in-container ]]; then
    docker build --platform linux/amd64 -t ergon-fstack-test "$here"
    exec docker run --rm --platform linux/amd64 --privileged -e RATE -e DURATION -e STREAMS \
        -v "$repo":/w ergon-fstack-test bash /w/io/benches/fstack/docker-test.sh --in-container
fi

export PATH=/usr/local/cargo/bin:$PATH
rate=${RATE:-1000} duration=${DURATION:-5} streams=${STREAMS:-1}
target=/opt/target
(cd /w/io/benches && FSTACK_PREFIX=/opt/fstack CARGO_TARGET_DIR=$target \
    cargo build --release --features fstack --bin fstack_lat)
bin=$target/release/fstack_lat
out=$(mktemp -d)
ip link del va 2>/dev/null || true
ip link add va type veth peer name vb
ip link set va up
ip link set vb up
sysctl -qw net.ipv6.conf.va.disable_ipv6=1 net.ipv6.conf.vb.disable_ipv6=1
common=(--prefix-len 24 --gateway 10.9.0.254 --no-huge --memory-mb 512 --rate "$rate"
    --streams "$streams" --warmup-s 2 --duration-s "$duration" --label t:fstack-tcp --connect-timeout-s 60)
pong_pid=
cleanup() {
    [[ -z $pong_pid ]] || kill -9 "$pong_pid" 2>/dev/null || true
    ip link del va 2>/dev/null || true
}
trap cleanup EXIT
fail() {
    echo "docker-test: FAIL: $*" >&2
    for f in pong ping; do echo "--- $f.log" >&2; tail -20 "$out/$f.log" >&2 || true; done
    exit 1
}

"$bin" --role pong --cpus 1 --listen 10.9.0.2:22000 --vdev net_af_packet0,iface=vb \
    --file-prefix pong --out "$out/pong" "${common[@]}" >"$out/pong.log" 2>&1 &
pong_pid=$!
for _ in $(seq 120); do
    grep -qx ready "$out/pong.log" && break
    kill -0 "$pong_pid" 2>/dev/null || fail "the pong exited before ready"
    sleep 1
done
[[ $(grep -cx ready "$out/pong.log") == 1 ]] || fail "the pong did not say ready once"

timeout $((duration + 120)) "$bin" --role ping --cpus 2 --source 10.9.0.1 --connect 10.9.0.2:22000 \
    --vdev net_af_packet0,iface=va --file-prefix ping --out "$out/ping" "${common[@]}" \
    >"$out/ping.log" 2>&1 || fail "the ping failed"
kill -TERM "$pong_pid"
for _ in $(seq 10); do kill -0 "$pong_pid" 2>/dev/null || break; sleep 1; done
wait "$pong_pid" || fail "the pong exited non-zero after SIGTERM"
pong_pid=

row=$(grep -E '^[^, ]+,' "$out/ping.log" | grep -F 't:fstack-tcp' | tail -1)
[[ -n $row ]] || fail "no row from the ping"
echo "row: $row"
python3 - "$out" "$row" "$rate" "$duration" <<'EOF'
import json, sys
out, row, rate, duration = sys.argv[1], sys.argv[2].split(","), int(sys.argv[3]), int(sys.argv[4])
def fail(m):
    print("docker-test: FAIL:", m); sys.exit(1)
if len(row) != 33:
    fail(f"row has {len(row)} columns, not 33")
ping = json.load(open(f"{out}/ping/manifest.json"))
pong = json.load(open(f"{out}/pong/manifest.json"))
s = ping.get("settings", ping)
p = pong.get("settings", pong)
def get(d, k):
    v = d.get(k)
    if v is None: fail(f"manifest has no {k}")
    return v
want = rate * duration
sent, echoes = int(get(s, "sent") if "sent" in s else want), int(get(p, "echoes"))
if echoes < want:
    fail(f"pong echoed {echoes}, fewer than the {want} measured pings")
checks = {
    "tcp_info.state": "4", "readback.nodelay": "true",
    "fstack.conf.dpdk.pkt_tx_delay": "0", "fstack.conf.dpdk.idle_sleep": "0",
    "fstack.conf.dpdk.extra_eal_args": "--no-pci --vdev=net_af_packet0,iface=va",
    "fstack.conf.port0.addr": "10.9.0.1", "fstack.build.fstack_tag": "v2.0",
}
for k, v in checks.items():
    if get(s, k) != v: fail(f"ping {k}={get(s, k)}, not {v}")
mss = int(get(s, "tcp_info.snd_mss"))
if not 500 < mss <= 9001: fail(f"tcp_info.snd_mss {mss}")
if int(get(s, "tcp_info.options")) & 1 == 0: fail("no TCP timestamps")
if int(get(s, "tcp_info.rtt_us")) <= 0: fail("tcp_info.rtt_us 0")
if get(p, "fstack.conf.port0.addr") != "10.9.0.2": fail("pong's address")
print(f"docker-test: ok: {echoes} echoes, mss {mss}, rtt {get(s, 'tcp_info.rtt_us')} us, "
      f"retrans {get(s, 'retrans') if 'retrans' in s else 'n/a'}, passes {get(s, 'stat.passes')}")
EOF
