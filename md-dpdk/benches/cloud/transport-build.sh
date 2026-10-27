#!/usr/bin/env bash
# transport-build.sh <checkout> <target dir> [<harness features>]: the
# transport suite's bench binaries (transport-cli.md) into
# <target dir>/release, as vm.sh transport-build and the local smoke
# (test/transport-smoke-docker.sh) both build them; 1 unless every one is
# there after.
#   aeron_lat, hostcheck  the root workspace's ergon-md-dpdk-harness, with
#                         io/benches's release profile (panic abort, debug
#                         info), so every arm's measuring code is built alike;
#                         counters too (the per-point events' sampler reads
#                         the media driver's loss counters with it)
#   mpsc_lat, udp_lat, tcp_lat  io/benches, a workspace of its own (the root
#                         one excludes it), by its manifest
#   fstack_lat            with TR_FSTACK_PREFIX (vm.sh transport-build fstack):
#                         F-Stack and its DPDK built there first
#                         (io/benches/fstack/build-fstack.sh, kept between
#                         runs), then io/benches with --features fstack
set -euo pipefail
repo=${1:?checkout} target=${2:?target dir} features=${3:-}
cd "$repo"
CARGO_PROFILE_RELEASE_PANIC=abort CARGO_PROFILE_RELEASE_DEBUG=1 \
    cargo build --release --target-dir "$target" -p ergon-md-dpdk-harness ${features:+--features "$features"} \
    --bin aeron_lat --bin hostcheck --bin counters
cargo build --release --manifest-path io/benches/Cargo.toml --target-dir "$target" --bin mpsc_lat --bin udp_lat --bin tcp_lat
bins="aeron_lat hostcheck counters mpsc_lat udp_lat tcp_lat"
if [[ -n ${TR_FSTACK_PREFIX:-} ]]; then
    bash io/benches/fstack/build-fstack.sh "$TR_FSTACK_PREFIX" native
    FSTACK_PREFIX=$TR_FSTACK_PREFIX cargo build --release --manifest-path io/benches/Cargo.toml --target-dir "$target" \
        --features fstack --bin fstack_lat
    bins+=" fstack_lat"
fi
missing=
for b in $bins; do [[ -x $target/release/$b ]] || missing+=" $b"; done
[[ -z $missing ]] || { echo "transport-build: not in $target/release:$missing" >&2; exit 1; }
echo "transport-build: $bins in $target/release"
