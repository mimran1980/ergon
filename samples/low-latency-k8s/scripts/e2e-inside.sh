#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The vdev e2e inside one privileged container of the sample image
# (scripts/e2e-vdev.sh --docker runs it): the media driver on two DPDK tap
# ports bridged in the container, a pong and a ping on that driver, and
# ll-probe watching every thread of the three. The ergon-shared-cpus plugin
# is not here, so this script plays its part: it sets the ERGON_* lists and
# starts each process with `taskset -c <shared>`, as the plugin's
# createRuntime hook pins a container's first thread. The driver runs under
# the real ergon-cpus and driver-entrypoint.sh.
#
# CPU plan (at least 6 CPUs): 0 probe and IRQs, 1 shared, 2-3 driver
# sender/receiver, 4 ping, 5 pong.
#
# Env: LL_E2E_SAMPLES (2000000), LL_E2E_WINDOW_S (30), LL_E2E_DURATION_S (45:
# the ping outlives the probe window),
# LL_E2E_OUT (/out: where the logs go).
set -uo pipefail

OUT=${LL_E2E_OUT:-/out}
SAMPLES=${LL_E2E_SAMPLES:-2000000}
WINDOW=${LL_E2E_WINDOW_S:-30}
DURATION=${LL_E2E_DURATION_S:-45}
mkdir -p "$OUT"
say() { echo "e2e: $*" >&2; }
die() { say "FAIL: $*"; for f in "$OUT"/*.log; do echo "--- $f" >&2; tail -40 "$f" >&2; done; exit 1; }

n=$(nproc)
(( n >= 6 )) || die "need 6 CPUs, have $n"
# this script (and its waits) on the probe's CPU, off the exclusive ones
taskset -cp 0 $$ >/dev/null
SHARED=1
export AERON_DIR=/dev/shm/aeron
COMMON=(AERON_DIR="$AERON_DIR" AERON_TERM_BUFFER_SPARSE_FILE=false RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena ERGON_HOST_CHECKS=on LL_OVERLAY=docker-vdev
  ERGON_SHARED_CPUS=$SHARED ERGON_HELPER_CPUS=$SHARED ERGON_SHARED_MILLICPUS=100)

say "$(cat /etc/ll-build.txt | tr '\n' ' ')"
say "kernel $(uname -r) $(uname -m), $n CPUs"

# ---- driver ------------------------------------------------------------------
env "${COMMON[@]}" \
  ERGON_ISOLATED_CPUS=2,3 ERGON_SHARED_MILLICPUS=500 \
  AERON_DIR_DELETE_ON_START=true RUSTERON_MEDIA_DRIVER_DIR="$AERON_DIR" \
  AERON_RCV_INITIAL_WINDOW_LENGTH=2097152 AERON_SOCKET_SO_RCVBUF=2097152 AERON_SOCKET_SO_SNDBUF=2097152 \
  AERON_RECEIVER_IO_VECTOR_CAPACITY=16 \
  RUSTERON_DPDK_TEST_VDEV=1 RUSTERON_DPDK_HOST_LOCAL=loopback \
  RUSTERON_DPDK_SENDER_PCI=net_tap0 RUSTERON_DPDK_SENDER_IPV4_CIDR=10.9.0.1/24 RUSTERON_DPDK_SENDER_GATEWAY=10.9.0.254 \
  RUSTERON_DPDK_RECEIVER_PCI=net_tap1 RUSTERON_DPDK_RECEIVER_IPV4_CIDR=10.9.0.3/24 RUSTERON_DPDK_RECEIVER_GATEWAY=10.9.0.254 \
  RUSTERON_DPDK_RX_DESCRIPTORS=1024 RUSTERON_DPDK_TX_DESCRIPTORS=1024 RUSTERON_DPDK_MBUFS_PER_PORT=8192 \
  RUSTERON_DPDK_MAX_AERON_MTU=1408 \
  LL_EAL_EXTRA="--no-huge -m 512 --no-pci" LL_VDEV_BRIDGE=1 HOSTNAME=e2e-driver \
  taskset -c "$SHARED" ergon-cpus \
    --env AERON_SENDER_CPU_AFFINITY=isolated:0 \
    --env AERON_RECEIVER_CPU_AFFINITY=isolated:1 \
    --env RUSTERON_DPDK_HOUSEKEEPING_CPU=shared:0 \
    -- driver-entrypoint.sh media_driver >"$OUT/driver.log" 2>&1 &
driver=$!
trap 'kill -TERM $driver ${pong:-} ${ping:-} 2>/dev/null' EXIT
for _ in $(seq 1 120); do
  grep -q "vdev bridge br0" "$OUT/driver.log" 2>/dev/null && [[ -e $AERON_DIR/cnc.dat ]] && break
  kill -0 "$driver" 2>/dev/null || die "the driver exited"
  sleep 0.5
done
grep -q "vdev bridge br0" "$OUT/driver.log" || die "the driver's taps were not bridged in 60 s"
say "driver up (pid $driver)"

# ---- apps --------------------------------------------------------------------
app() { # role isolated pub sub [extra env...]
  local role=$1 iso=$2 pub=$3 sub=$4
  shift 4
  exec env "${COMMON[@]}" ERGON_ISOLATED_CPUS="$iso" LL_ROLE="$role" \
    LL_PUB_CHANNEL="aeron:udp?endpoint=$pub" LL_SUB_CHANNEL="aeron:udp?endpoint=$sub" \
    LL_DRIVER_WAIT_S=60 LL_CONNECT_WAIT_S=60 LL_TERMINATION_LOG=/nonexistent "$@" \
    taskset -c "$SHARED" ll-app
}
app pong 5 10.9.0.3:40002 10.9.0.3:40001 >"$OUT/pong.log" 2>&1 &
pong=$!
app ping 4 10.9.0.3:40001 10.9.0.3:40002 LL_WARMUP=20000 LL_SAMPLES="$SAMPLES" LL_DURATION_S="$DURATION" \
  >"$OUT/ping.log" 2>&1 &
ping=$!
for _ in $(seq 1 240); do
  grep -q "ll-selfcheck RESULT" "$OUT/ping.log" && grep -q "ll-selfcheck RESULT" "$OUT/pong.log" && break
  kill -0 "$ping" 2>/dev/null || die "the ping exited before its self-check"
  kill -0 "$pong" 2>/dev/null || die "the pong exited before its self-check"
  sleep 0.5
done
grep -q "ll-selfcheck RESULT PASS" "$OUT/ping.log" || die "the ping's self-check did not pass"
grep -q "ll-selfcheck RESULT PASS" "$OUT/pong.log" || die "the pong's self-check did not pass"
say "self-checks PASS; probe for ${WINDOW} s"

# ---- probe -------------------------------------------------------------------
ll-probe watch --contract placement --window-s "$WINDOW" --interval-ms 100 --pin 0 \
  --cpuset-check off --node docker-vdev \
  --target "e2e/ll-media-driver=$driver" --target "e2e/ll-ping=$ping" --target "e2e/ll-pong=$pong" \
  >"$OUT/verify.txt" 2>"$OUT/probe.err"
probe_rc=$?

wait "$ping"
ping_rc=$?
kill -TERM "$pong" 2>/dev/null
wait "$pong"
kill -TERM "$driver" 2>/dev/null
wait "$driver"
trap - EXIT

cat "$OUT/verify.txt"
grep -h "^ll-latency" "$OUT/ping.log" "$OUT/pong.log"
grep -h "^ll-selfcheck" "$OUT/ping.log" >"$OUT/selfcheck-ping.txt"

[[ $probe_rc == 0 ]] || die "ll-probe: exit $probe_rc ($(cat "$OUT/probe.err"))"
grep -q $'^ll-verify\tRESULT\tPASS' "$OUT/verify.txt" || die "no RESULT PASS"
[[ $ping_rc == 0 ]] || die "the ping exited $ping_rc"
for field in v1 rtt_ns rtt_max_top10 loop_gap_ns hot log; do
  grep -q $'^ll-latency\t'"$field"$'\t' "$OUT/ping.log" || die "the ping printed no ll-latency $field line"
done
grep -q $'^ll-latency\tv1\t.*\tlost=0$' "$OUT/ping.log" || die "the ping lost echoes"
grep -q $'^ll-latency\tlog\tdropped_lines=0\t' "$OUT/ping.log" || die "the ping dropped log lines"
say "PASS"
