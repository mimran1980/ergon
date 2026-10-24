#!/usr/bin/env bash
# purefloor-docker.sh <image> <out dir>: purefloor.c compiled with -Wall
# -Wextra -Werror and run end to end in a Linux container, its ping and pong
# over a memif pair (no NIC, no hugepages, no privileges), as vm.sh client
# runs them: the driver.env variables, LABEL, RTT_HISTOGRAM, LOAD_SPIKES,
# LOOP_STALLS_DIR. <image> has gcc, pkg-config and libdpdk-dev (Debian
# trixie: apt-get install gcc libc6-dev pkg-config libdpdk-dev dpdk). The
# ping's result line, histogram, spikes and both stalls files land in <out
# dir> as lib.sh names them, for analyse.py and spikes.py. Fails unless the
# ping measured every round trip it sent, none lost, and the histogram
# counts them all.
set -euo pipefail
image=${1:?image with gcc, pkg-config and libdpdk-dev}
out=${2:?out dir}
here=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$out"
out=$(cd "$out" && pwd)
docker run --rm --tmpfs /tmp:exec,size=3g -v "$here:/src:ro" -v "$out:/out" "$image" bash -euc '
gcc -O2 -Wall -Wextra -Werror $(pkg-config --cflags libdpdk) -o /tmp/purefloor /src/purefloor.c $(pkg-config --libs libdpdk)
export RUSTERON_DPDK_EAL_ARGS="-l 0 --no-huge -m 256 --no-pci" LOOP_STALLS_DIR=/out LOOP_STALL_NS=10000
RUSTERON_DPDK_FILE_PREFIX=pong RUSTERON_DPDK_RECEIVER_PCI=net_memif0,role=server,socket=/tmp/memif.sock \
    RUSTERON_DPDK_RECEIVER_IPV4_CIDR=10.0.0.2/24 RUSTERON_DPDK_RECEIVER_GATEWAY=10.0.0.254 \
    /tmp/purefloor xpong 10.0.0.2:20123 10.0.0.1:20124 1 2>/out/pong.log &
pong=$!
sleep 1
RUSTERON_DPDK_FILE_PREFIX=ping RUSTERON_DPDK_RECEIVER_PCI=net_memif0,role=client,socket=/tmp/memif.sock \
    RUSTERON_DPDK_RECEIVER_IPV4_CIDR=10.0.0.1/24 RUSTERON_DPDK_RECEIVER_GATEWAY=10.0.0.254 \
    LABEL=docker:dpdk-pure RTT_SECONDS=5 RTT_HISTOGRAM=/out/histogram.csv LOAD_SPIKES=/out/spikes.csv LOAD_SPIKE_NS=30000 \
    /tmp/purefloor xping 10.0.0.2:20123 10.0.0.1:20124 20000 1000 2 >/out/rtt.csv 2>/out/ping.log
kill -TERM $pong
wait $pong
'
cat "$out/rtt.csv"
IFS=, read -r _ label _ _ _ _ _ _ _ _ samples lost <"$out/rtt.csv"
counted=$(awk -F, 'NR > 1 {n += $2} END {print n + 0}' "$out/histogram.csv")
if [[ $label != docker:dpdk-pure || $samples != 20000 || $lost != 0 || $counted != 20000 ]] ||
    ! grep -q '^# spikes_total=' "$out/spikes.csv" || [[ ! -s $out/stalls-ping.csv || ! -s $out/stalls-pong.csv ]] ||
    ! grep -q 'pong ended: SIGTERM' "$out/pong.log"; then
    echo "purefloor-docker: FAILED (samples $samples lost $lost histogram $counted)" >&2
    exit 1
fi
echo "purefloor-docker: passed"
