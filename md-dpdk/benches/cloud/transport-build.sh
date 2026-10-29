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
#                         the media driver's loss counters with it), and floor
#                         (the dpdk-raw arm's raw floor)
#   mpsc_lat, udp_lat, tcp_lat, wstls_lat
#                         io/benches, a workspace of its own (the root
#                         one excludes it), by its manifest. wstls_lat is the
#                         TCP and WebSocket-over-TLS echo.
#   fstack_lat            with TR_FSTACK_PREFIX (vm.sh transport-build fstack):
#                         F-Stack and its DPDK built there first
#                         (io/benches/fstack/build-fstack.sh, kept between
#                         runs when <prefix>/BUILD exists), then io/benches
#                         with --features fstack
#   fstack_spin_lat       the same prefix, its own invocation with
#                         --features fstack-spin (ergon-io links F-Stack),
#                         together with wstls_fstack_lat (WebSocket on that
#                         backend). Not combined with --features fstack: two DPDKs.
#   smoltcp               the same prefix: a separate cargo invocation with
#                         --features smoltcp-dpdk for wstls_lat, udp_lat and
#                         tcp_lat. fstack and smoltcp-dpdk are not one
#                         invocation: each links a DPDK, and io/build.rs
#                         refuses both in one binary.
set -euo pipefail
repo=${1:?checkout} target=${2:?target dir} features=${3:-}
cd "$repo"
CARGO_PROFILE_RELEASE_PANIC=abort CARGO_PROFILE_RELEASE_DEBUG=1 \
    cargo build --release --target-dir "$target" -p ergon-md-dpdk-harness ${features:+--features "$features"} \
    --bin aeron_lat --bin hostcheck --bin counters --bin floor
cargo build --release --manifest-path io/benches/Cargo.toml --target-dir "$target" \
    --bin mpsc_lat --bin udp_lat --bin tcp_lat --bin wstls_lat
bins="aeron_lat hostcheck counters floor mpsc_lat udp_lat tcp_lat wstls_lat"
if [[ -n ${TR_FSTACK_PREFIX:-} ]]; then
    if [[ ! -e $TR_FSTACK_PREFIX/BUILD ]]; then
        bash io/benches/fstack/build-fstack.sh "$TR_FSTACK_PREFIX" native
    fi
    FSTACK_PREFIX=$TR_FSTACK_PREFIX cargo build --release --manifest-path io/benches/Cargo.toml --target-dir "$target" \
        --features fstack --bin fstack_lat
    FSTACK_PREFIX=$TR_FSTACK_PREFIX cargo build --release --manifest-path io/benches/Cargo.toml --target-dir "$target" \
        --features fstack-spin --bin fstack_spin_lat --bin wstls_fstack_lat
    cargo build --release --manifest-path io/benches/Cargo.toml --target-dir "$target" \
        --features smoltcp-dpdk --bin wstls_lat --bin udp_lat --bin tcp_lat
    bins+=" fstack_lat fstack_spin_lat wstls_fstack_lat"
fi
missing=
for b in $bins; do [[ -x $target/release/$b ]] || missing+=" $b"; done
[[ -z $missing ]] || { echo "transport-build: not in $target/release:$missing" >&2; exit 1; }
echo "transport-build: $bins in $target/release"
