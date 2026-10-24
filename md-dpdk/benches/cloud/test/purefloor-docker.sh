#!/usr/bin/env bash
# purefloor-docker.sh <image> <out dir>: purefloor.c compiled with -Wall
# -Wextra -Werror and run end to end in a Linux container, its ping and pong
# as vm.sh client runs them (the driver.env variables, LABEL, RTT_HISTOGRAM,
# LOAD_SPIKES, LOOP_STALLS_DIR), over two wires:
#   memif  a net_memif pair (no NIC, no hugepages, no privileges): no
#          checksum offloads or link-change interrupts, so both are off
#   tap    two net_tap ports joined in the container's kernel by tc mirred
#          (NET_ADMIN, /dev/net/tun): the PMD offers IPv4 and UDP checksum offloads and
#          link-change interrupts, so both are on, as the transport sets
#          them, and every datagram's checksums are checked on receipt
# <image> has gcc, pkg-config, libdpdk-dev and iproute2 (Debian trixie:
# apt-get install gcc libc6-dev pkg-config libdpdk-dev dpdk iproute2). Each
# wire's result line, histogram, spikes and both stalls files land in
# <out dir>/<wire> as lib.sh names them, for analyse.py and spikes.py.
# Fails unless, on each wire, the ping measured every round trip it sent,
# none lost, the histogram counts them all, both sides set the port up as
# the transport would on that NIC, and no datagram came with a bad checksum.
set -euo pipefail
image=${1:?image with gcc, pkg-config, libdpdk-dev and iproute2}
out=${2:?out dir}
here=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$out/memif" "$out/tap"
out=$(cd "$out" && pwd)
# shellcheck disable=SC2016
inner='
gcc -O2 -Wall -Wextra -Werror $(pkg-config --cflags libdpdk) -o /tmp/purefloor /src/purefloor.c $(pkg-config --libs libdpdk)
export RUSTERON_DPDK_EAL_ARGS="-l 0 --no-huge -m 256 --no-pci" LOOP_STALLS_DIR=/out LOOP_STALL_NS=10000
if [[ $WIRE == memif ]]; then
    pong_dev=net_memif0,role=server,socket=/tmp/memif.sock ping_dev=net_memif0,role=client,socket=/tmp/memif.sock
else
    pong_dev=net_tap0,iface=dtap1 ping_dev=net_tap1,iface=dtap0
    # once both PMDs have made their taps, the ingress of each (the
    # frames its port sends) redirected out of the other (a bridge would
    # pass them through arptables); the ping retries ARP meanwhile
    (for _ in $(seq 600); do
        ip link show dtap0 >/dev/null 2>&1 && ip link show dtap1 >/dev/null 2>&1 && break
        sleep 0.1
    done
    for pair in "dtap0 dtap1" "dtap1 dtap0"; do
        set -- $pair
        tc qdisc add dev "$1" ingress 2>/dev/null || true
        tc filter add dev "$1" ingress matchall action mirred egress redirect dev "$2"
    done) &
fi
RUSTERON_DPDK_FILE_PREFIX=pong RUSTERON_DPDK_RECEIVER_PCI=$pong_dev \
    RUSTERON_DPDK_RECEIVER_IPV4_CIDR=10.0.0.2/24 RUSTERON_DPDK_RECEIVER_GATEWAY=10.0.0.254 \
    /tmp/purefloor xpong 10.0.0.2:20123 10.0.0.1:20124 1 2>/out/pong.log &
pong=$!
sleep 1
RUSTERON_DPDK_FILE_PREFIX=ping RUSTERON_DPDK_RECEIVER_PCI=$ping_dev \
    RUSTERON_DPDK_RECEIVER_IPV4_CIDR=10.0.0.1/24 RUSTERON_DPDK_RECEIVER_GATEWAY=10.0.0.254 \
    LABEL=docker:dpdk-pure RTT_SECONDS=5 RTT_HISTOGRAM=/out/histogram.csv LOAD_SPIKES=/out/spikes.csv LOAD_SPIKE_NS=30000 \
    /tmp/purefloor xping 10.0.0.2:20123 10.0.0.1:20124 20000 1000 2 >/out/rtt.csv 2>/out/ping.log
kill -TERM $pong
wait $pong
'
docker run --rm --tmpfs /tmp:exec,size=3g -v "$here:/src:ro" -v "$out/memif:/out" -e WIRE=memif "$image" bash -euc "$inner"
docker run --rm --tmpfs /tmp:exec,size=3g --cap-add NET_ADMIN --device /dev/net/tun -v "$here:/src:ro" -v "$out/tap:/out" \
    -e WIRE=tap "$image" bash -euc "$inner"
failed=0
for wire in memif tap; do
    o=$out/$wire
    cat "$o/rtt.csv"
    IFS=, read -r _ label _ _ _ _ _ _ _ _ samples lost <"$o/rtt.csv"
    counted=$(awk -F, 'NR > 1 {n += $2} END {print n + 0}' "$o/histogram.csv")
    if [[ $wire == memif ]]; then port="checksum offloads off (in software), link-change interrupts off"; else port="checksum offloads on, link-change interrupts on"; fi
    if [[ $label != docker:dpdk-pure || $samples != 20000 || $lost != 0 || $counted != 20000 ]] ||
        ! grep -q '^# spikes_total=' "$o/spikes.csv" || [[ ! -s $o/stalls-ping.csv || ! -s $o/stalls-pong.csv ]] ||
        ! grep -q 'pong ended: SIGTERM' "$o/pong.log" ||
        ! grep -q "$port" "$o/ping.log" || ! grep -q "$port" "$o/pong.log" ||
        ! grep -q ' 0 datagrams with a bad checksum dropped' "$o/ping.log" ||
        ! grep -q ' 0 datagrams with a bad checksum dropped' "$o/pong.log"; then
        echo "purefloor-docker: $wire FAILED (samples $samples lost $lost histogram $counted; want: $port, no bad checksums)" >&2
        tail -3 "$o/ping.log" "$o/pong.log" >&2
        failed=1
    fi
done
((failed == 0)) || exit 1
echo "purefloor-docker: passed"
