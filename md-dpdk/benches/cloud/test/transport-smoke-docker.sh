#!/usr/bin/env bash
# transport-smoke-docker.sh [<out dir>]: the transport suite end to end on
# one Linux box, in a container (transport.sh smoke): no cloud, no ssh. It
# builds the bench binaries by vm.sh transport-build's own script
# (transport-build.sh, features static as vm.sh gives a host without DPDK)
# and the media driver, runs every arm of every set but DPDK's
# (transport-smoke.tsv: 1 rep, short durations), lays the session out (transport_collect.py), and
# runs fairness.py and analyse.py --transport on it. <out dir> (default
# target/transport-smoke) gets the session (run/), fairness.txt, tables.md
# and summary.txt. Exit 0 when every point gave its rows, no run failed, the
# collector found nothing missing and the tables rendered; fairness.py's
# violations are listed by rule in summary.txt (the box isolates nothing).
#
# --privileged --network host: netem on lo (tc) and the TCP and busy-poll
# sysctls (busy_read and busy_poll exist only in the first network namespace).
# They are the Docker VM's, so the knobs the smoke sets are read first and
# put back as it ends; netem and the hosts' addresses (local-vm.sh) leave lo
# with it.
# SMOKE_IMAGE (ergon-transport-smoke:local, built from
# transport-smoke.Dockerfile when missing), SMOKE_TARGET_VOLUME
# (ergon-transport-smoke-target) and SMOKE_REGISTRY_VOLUME
# (ergon-transport-smoke-registry): Docker volumes, kept between runs;
# remove them with `docker volume rm` when done. Needs eight CPUs.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cloud=$(dirname "$here")
repo=$(cd "$cloud/../../.." && pwd)
out=${1:-$repo/target/transport-smoke}
mkdir -p "$out"
out=$(cd "$out" && pwd)
image=${SMOKE_IMAGE:-ergon-transport-smoke:local}
if ! docker image inspect "$image" >/dev/null 2>&1; then
    docker build -t "$image" -f "$here/transport-smoke.Dockerfile" "$here"
fi
# shellcheck disable=SC2016
inner='
set -euo pipefail
# a copy: io/benches has no lock file of its own, and cargo writes one
rm -rf /work && mkdir /work
(cd /src && tar --exclude ./target -cf /tmp/src.tar .)
tar -xf /tmp/src.tar -C /work && rm /tmp/src.tar
cd /work
cargo build --locked --release -p ergon-md-dpdk --features static --bin media_driver
# the bench binaries as vm.sh transport-build builds them
bash md-dpdk/benches/cloud/transport-build.sh /work /target static
cloud=/work/md-dpdk/benches/cloud
# the Docker VM'"'"'s knobs as they were, put back on any exit
saved=$(. "$cloud/transport-settings.env"; . "$cloud/host.sh"; transport_settings_read | grep -v "=absent$")
restore() {
    while IFS="=" read -r k v; do sysctl -qw "$k=$v" || true; done <<<"$saved"
    tc qdisc del dev lo root 2>/dev/null || true
    for ip in 127.0.1.1 127.0.1.2 127.0.2.1 127.0.2.2; do ip addr del "$ip/32" dev lo 2>/dev/null || true; done
}
trap restore EXIT
rm -rf /out/smoke && bash "$cloud/transport.sh" smoke /out/smoke
'
docker run --rm --privileged --network host --shm-size=2g \
    -v "$repo:/src:ro" -v "$out:/out" \
    -v "${SMOKE_TARGET_VOLUME:-ergon-transport-smoke-target}:/target" \
    -v "${SMOKE_REGISTRY_VOLUME:-ergon-transport-smoke-registry}:/usr/local/cargo/registry" \
    -e CARGO_TARGET_DIR=/target -e LAB_SEED="${LAB_SEED:-}" \
    "$image" bash -c "$inner"
