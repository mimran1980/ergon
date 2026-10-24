#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Builds the sample image for one tag, loaded into the local Docker:
#
#   samples/low-latency-k8s/docker/build.sh <tag> [image-name]   (default name: ll)
#
# | tag           | platform    | RUST_TARGET_CPU | RUSTERON_C_MARCH | nodes |
# |---------------|-------------|-----------------|------------------|-------|
# | amd64-generic | linux/amd64 | (generic)       | x86-64-v2        | vdev, any x86 |
# | arm64-generic | linux/arm64 | neoverse-n1     | armv8.2-a        | vdev, Apple silicon, Graviton2+ |
# | arm64-v3      | linux/arm64 | neoverse-v3     | armv8.2-a        | c9g (Graviton5) |
# | arm64-n2      | linux/arm64 | neoverse-n2     | armv8.2-a        | Azure Cobalt 100 |
#
# A CPU tag runs only on that CPU family (SIGILL elsewhere): select its nodes
# by instance family. md-dpdk's build.rs stops when AL2023's gcc does not know
# the -mcpu name. Built and run so far: arm64-generic only (README).
set -euo pipefail

TAG=${1:?usage: build.sh <amd64-generic|arm64-generic|arm64-v3|arm64-n2> [image-name]}
NAME=${2:-ll}
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
case $TAG in
  amd64-generic) PLATFORM=linux/amd64 CPU= MARCH=x86-64-v2 ;;
  arm64-generic) PLATFORM=linux/arm64 CPU=neoverse-n1 MARCH=armv8.2-a ;;
  arm64-v3) PLATFORM=linux/arm64 CPU=neoverse-v3 MARCH=armv8.2-a ;;
  arm64-n2) PLATFORM=linux/arm64 CPU=neoverse-n2 MARCH=armv8.2-a ;;
  *) echo "build.sh: unknown tag $TAG" >&2; exit 2 ;;
esac
docker buildx build --platform "$PLATFORM" --load \
  -f "$ROOT/samples/low-latency-k8s/docker/Dockerfile" \
  --build-arg "RUST_TARGET_CPU=$CPU" \
  --build-arg "RUSTERON_C_MARCH=$MARCH" \
  --build-arg "LL_CPU_BUILD=$TAG" \
  ${CARGO_BUILD_JOBS:+--build-arg "CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS"} \
  -t "$NAME:$TAG" "$ROOT"
docker run --rm "$NAME:$TAG" cat /etc/ll-build.txt
