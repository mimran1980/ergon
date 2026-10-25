#!/usr/bin/env bash
# transport-build.sh <checkout> <target dir> [<harness features>]: the
# transport suite's bench binaries (transport-cli.md) into
# <target dir>/release, as vm.sh transport-build and the local smoke
# (test/transport-smoke-docker.sh) both build them; 1 unless every one is
# there after.
#   aeron_lat, hostcheck  the root workspace's ergon-md-dpdk-harness, with
#                         io/benches's release profile (panic abort, debug
#                         info), so every arm's measuring code is built alike
#   mpsc_lat, udp_lat, tcp_lat  io/benches, a workspace of its own (the root
#                         one excludes it), by its manifest
set -euo pipefail
repo=${1:?checkout} target=${2:?target dir} features=${3:-}
cd "$repo"
CARGO_PROFILE_RELEASE_PANIC=abort CARGO_PROFILE_RELEASE_DEBUG=1 \
    cargo build --release --target-dir "$target" -p ergon-md-dpdk-harness ${features:+--features "$features"} \
    --bin aeron_lat --bin hostcheck
cargo build --release --manifest-path io/benches/Cargo.toml --target-dir "$target" --bin mpsc_lat --bin udp_lat --bin tcp_lat
missing=
for b in aeron_lat hostcheck mpsc_lat udp_lat tcp_lat; do [[ -x $target/release/$b ]] || missing+=" $b"; done
[[ -z $missing ]] || { echo "transport-build: not in $target/release:$missing" >&2; exit 1; }
echo "transport-build: aeron_lat hostcheck mpsc_lat udp_lat tcp_lat in $target/release"
