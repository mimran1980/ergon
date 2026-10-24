#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The sample's Linux lanes in Docker: fmt, clippy -D warnings, every test
# (unit, logger, placement, manifests excluded: they need kubectl, helm and
# kubeconform, and run on the host) and the bench smoke, on linux/arm64
# (native on Apple silicon) or linux/amd64 (emulated there).
#
#   samples/low-latency-k8s/scripts/test-docker.sh [--arch arm64|amd64] [--build]
#
# The image is md-dpdk's verification environment
# (md-dpdk/native/dpdk/test/Dockerfile: Ubuntu 24.04, Rust 1.99.0, CMake,
# clang), tagged ll-sample-test:<arch>. --build (re)builds it. The worktree
# is mounted read-only; the target dir and the cargo registry live in named
# volumes (ll-sample-target-<arch>, ll-sample-rustup-<arch> for rustfmt and
# clippy, ll-sample-cargo-registry), which
# --clean removes along with the image.
set -euo pipefail

HERE=$(cd "$(dirname "$0")/.." && pwd)
REPO=$(cd "$HERE/../.." && pwd)
ARCH=arm64
BUILD=0
CLEAN=0
while [[ $# -gt 0 ]]; do
  case $1 in
    --arch) ARCH=$2; shift 2 ;;
    --build) BUILD=1; shift ;;
    --clean) CLEAN=1; shift ;;
    *) echo "test-docker.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done
[[ $ARCH == arm64 || $ARCH == amd64 ]] || { echo "test-docker.sh: --arch arm64 or amd64" >&2; exit 2; }
IMAGE=ll-sample-test:$ARCH
if [[ $CLEAN == 1 ]]; then
  docker image rm -f "$IMAGE" >/dev/null 2>&1 || true
  docker volume rm -f "ll-sample-target-$ARCH" "ll-sample-rustup-$ARCH" ll-sample-cargo-registry >/dev/null 2>&1 || true
  echo "test-docker.sh: removed $IMAGE and its volumes"
  exit 0
fi
if [[ $BUILD == 1 ]] || ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  docker build --platform "linux/$ARCH" -t "$IMAGE" "$REPO/md-dpdk/native/dpdk/test"
fi
# --shm-size: the embedded driver's IPC logs live in /dev/shm.
# --cpus is not set: the placement tests need two CPUs, better three.
docker run --rm --platform "linux/$ARCH" \
  --shm-size=1g \
  -v "$REPO:/src:ro" \
  -v "ll-sample-target-$ARCH:/target" \
  -v ll-sample-cargo-registry:/usr/local/cargo/registry \
  -v "ll-sample-rustup-$ARCH:/usr/local/rustup" \
  -e CARGO_TARGET_DIR=/target \
  -w /src/samples/low-latency-k8s \
  "$IMAGE" \
  bash -c 'set -euo pipefail
    echo "== $(uname -m) $(uname -r) nproc=$(nproc)"
    cargo fmt --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked --lib -- --test-threads=1
    cargo test --locked --test logger_nonblocking --test logger_alloc -- --test-threads=1 --nocapture
    cargo test --locked --test placement_threads --test placement_aeron -- --test-threads=1 --nocapture
    cargo bench --locked --bench log_event -- --smoke
    echo "== PASS $(uname -m)"'
