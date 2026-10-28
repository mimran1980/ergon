#!/usr/bin/env bash
# ergon-io's `smoltcp-dpdk` feature on Linux, in Docker: linux/arm64
# (Graviton's architecture; native on Apple silicon) or, with --amd64,
# linux/amd64 (emulated there). The workspace is mounted read-only; the
# target directory and the cargo registry live in named volumes.
#
# Usage: io/native/dpdk/test/verify-smoltcp.sh [--amd64] [--build] [--check | --vdev [filter...]]
#   --build   (re)build the image from this directory's Dockerfile
#   --check   fmt, clippy with and without the feature, rustdoc, and the
#             crate's tests without and with it (the default)
#   --vdev    the vdev lane: scripts/test-io-smoltcp-vdev.sh as a non-root
#             user (root builds the binaries); --security-opt
#             seccomp=unconfined and apparmor=unconfined for unshare, not
#             --privileged
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
PLATFORM=linux/arm64
TAG=arm64
BUILD=0
MODE=check
FILTER=()
while [[ $# -gt 0 ]]; do
    case $1 in
    --amd64) PLATFORM=linux/amd64 TAG=amd64 ;;
    --build) BUILD=1 ;;
    --check) MODE=check ;;
    --vdev) MODE=vdev ;;
    *) FILTER+=("$1") ;;
    esac
    shift
done
IMAGE="ergon-io-smoltcp:$TAG"
VOLUME="ergon-io-smoltcp-target-$TAG"

if [[ $BUILD == 1 ]]; then
    docker build --platform "$PLATFORM" -t "$IMAGE" "$HERE"
fi

# seccomp unconfined: Docker's default profile refuses io_uring (the
# uring-sqpoll tests) and unshare (the vdev lane).
FLAGS=(--security-opt seccomp=unconfined)
if [[ $MODE == vdev ]]; then
    FLAGS+=(--security-opt apparmor=unconfined)
    COMMANDS="cargo build --locked -p ergon-io --features smoltcp-dpdk --test smoltcp_vdev --example smoltcp_peer \
        --message-format=json-render-diagnostics > /target/vdev-build.json \
        && chmod -R a+rX /target \
        && t=\$(sed -n 's/.*\"executable\":\"\\([^\"]*smoltcp_vdev[^\"]*\\)\".*/\\1/p' /target/vdev-build.json | tail -1) \
        && p=\$(sed -n 's/.*\"executable\":\"\\([^\"]*smoltcp_peer[^\"]*\\)\".*/\\1/p' /target/vdev-build.json | tail -1) \
        && setpriv --reuid=1000 --regid=1000 --clear-groups env HOME=/tmp ERGON_SMOLTCP_TEST_BIN=\$t ERGON_SMOLTCP_PEER_BIN=\$p \
            bash scripts/test-io-smoltcp-vdev.sh ${FILTER[*]:-}"
else
    COMMANDS="cargo fmt --all --check \
        && cargo clippy --locked -p ergon-io --all-targets -- -D warnings \
        && cargo clippy --locked -p ergon-io --all-targets --features smoltcp-dpdk -- -D warnings \
        && cargo clippy --locked -p ergon-io --features smoltcp-dpdk --test smoltcp_vdev --example smoltcp_peer -- -D warnings \
        && RUSTDOCFLAGS='-D warnings' cargo doc --locked -p ergon-io --features smoltcp-dpdk,webpki-roots --no-deps \
        && cargo test --locked -p ergon-io -- --test-threads=1 \
        && cargo test --locked -p ergon-io --features smoltcp-dpdk -- --test-threads=1"
fi

docker run --rm --platform "$PLATFORM" \
    ${FLAGS[@]+"${FLAGS[@]}"} \
    -v "$REPO:/src:ro" \
    -v "$VOLUME:/target" \
    -v ergon-io-smoltcp-registry:/usr/local/cargo/registry \
    -e CARGO_TARGET_DIR=/target \
    -w /src \
    "$IMAGE" \
    bash -c "set -o pipefail; uname -m; uname -r; $COMMANDS"
