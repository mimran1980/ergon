#!/usr/bin/env bash
# Compile-verify and test the `dpdk` feature of ergon-md-dpdk on Linux, in
# Docker: x86_64 (amd64 emulation on an arm64 host) or, with --arm64, aarch64
# (Graviton's architecture; native on Apple silicon). Builds the environment
# image once, then runs the feature build and tests against the workspace
# mounted read-only.
#
# Usage: ./verify-dpdk.sh [--arm64] [--build] [--all | --rootless | test_filter...]
#   --arm64      linux/arm64, in image ergon-md-dpdk-verify-arm64 with its own
#                target volume (ergon-md-dpdk-target-arm64); else linux/amd64
#                in ergon-md-dpdk-verify with ergon-md-dpdk-target
#   --build      (re)build that image from this directory's Dockerfile
#   --all        `just check-dpdk`: clippy, the tests with `dpdk` and with
#                `static,dpdk`, and the lab clients' tests
#   --rootless   every vdev scenario over the real tap PMD, as an unprivileged
#                user (scripts/test-dpdk-vdev-rootless.sh): the container gets
#                --security-opt seccomp=unconfined and apparmor=unconfined (for
#                unshare(CLONE_NEWUSER)) and --device /dev/net/tun, not
#                --privileged; root only builds the harness
#   test_filter  optional cargo test filter, default `--test dpdk_abi`
# The options may come in any order; every other argument is the filter.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
PLATFORM=linux/amd64
IMAGE="ergon-md-dpdk-verify"
TARGET_VOLUME="ergon-md-dpdk-target"
BUILD=0
ALL=0
ROOTLESS=0
FILTER=()
while [[ $# -gt 0 ]]; do
  case $1 in
  --arm64)
    PLATFORM=linux/arm64
    IMAGE="ergon-md-dpdk-verify-arm64"
    TARGET_VOLUME="ergon-md-dpdk-target-arm64"
    ;;
  --build) BUILD=1 ;;
  --all) ALL=1 ;;
  --rootless) ROOTLESS=1 ;;
  *) FILTER+=("$1") ;;
  esac
  shift
done

RUN_FLAGS=()
if [[ $ROOTLESS == 1 ]]; then
  if [[ $ALL == 1 || ${#FILTER[@]} -gt 0 ]]; then
    echo "verify-dpdk.sh: --rootless runs the vdev scenarios alone; drop --all and the filter" >&2
    exit 2
  fi
  RUN_FLAGS=(--security-opt seccomp=unconfined --security-opt apparmor=unconfined --device /dev/net/tun --tmpfs /tmp:exec)
  # Root builds the harness into the target volume; uid 1000 (ubuntu) runs
  # the scenarios through the rootless runner.
  COMMANDS="command -v ip >/dev/null && command -v ping >/dev/null || { echo 'the image lacks iproute2 or ping: rebuild it with --build' >&2; exit 1; } \
    && cargo build -p ergon-md-dpdk-harness --features dpdk \
    && chmod -R a+rX /target \
    && setpriv --reuid=1000 --regid=1000 --clear-groups env HOME=/tmp CARGO_TARGET_DIR=/target RUSTERON_HARNESS_BIN=/target/debug/dpdk-harness bash scripts/test-dpdk-vdev-rootless.sh"
elif [[ $ALL == 1 ]]; then
  if [[ ${#FILTER[@]} -gt 0 ]]; then
    echo "verify-dpdk.sh: --all runs every test; drop the filter (${FILTER[*]})" >&2
    exit 2
  fi
  # The commands of just/dpdk.just's check-dpdk, in order.
  COMMANDS="cargo clippy -p ergon-md-dpdk -p ergon-md-dpdk-harness --all-targets --features ergon-md-dpdk/dpdk,ergon-md-dpdk-harness/dpdk -- -D warnings \
    && cargo test -p ergon-md-dpdk --features dpdk -- --test-threads=1 \
    && cargo test -p ergon-md-dpdk --features static,dpdk -- --test-threads=1 \
    && cargo test -p ergon-md-dpdk-harness -- --test-threads=1"
else
  [[ ${#FILTER[@]} -gt 0 ]] || FILTER=(--test dpdk_abi)
  COMMANDS="cargo test -p ergon-md-dpdk --features dpdk ${FILTER[*]} -- --test-threads=1"
fi

if [[ $BUILD == 1 ]]; then
  docker build --platform "$PLATFORM" -t "$IMAGE" "$HERE"
fi

# dpdk_config mutates process-global env vars, so its tests must run
# single-threaded; a single-binary run is unaffected.
# --shm-size=512m is required: the Aeron driver's IPC publication allocates a
# ~200MB log in /dev/shm, and docker's default 64MB shm fails it with
# "(-12) insufficient storage space" (aeron_driver_context_run_storage_checks).
# The target dir and cargo registry live in named volumes, so the host tree is
# untouched and later runs rebuild incrementally; each architecture has its
# own target volume, and they share the registry (sources only), mounted where
# this Dockerfile's CARGO_HOME keeps it and where images built before it did.
docker run --rm --platform "$PLATFORM" \
  --shm-size=512m \
  ${RUN_FLAGS[@]+"${RUN_FLAGS[@]}"} \
  -v "$REPO:/src:ro" \
  -v "$TARGET_VOLUME:/target" \
  -v ergon-md-dpdk-cargo-registry:/usr/local/cargo/registry \
  -v ergon-md-dpdk-cargo-registry:/root/.cargo/registry \
  -e CARGO_TARGET_DIR=/target \
  -w /src \
  "$IMAGE" \
  bash -c "set -o pipefail; uname -m; uname -r; { $COMMANDS; } 2>&1 | tail -80"
