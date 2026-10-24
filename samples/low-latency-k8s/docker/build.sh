#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Builds the sample image for one tag:
#
#   samples/low-latency-k8s/docker/build.sh [--push <registry>] [--name ll] <tag>
#
# Without --push the image is loaded into the local Docker as <name>:<tag>
# (`baseline` loads both platforms: Docker's containerd image store holds a
# multi-platform image). With --push it goes to <registry>/<name>:<tag>, a
# manifest list for `baseline`.
#
# | tag      | platforms               | -Ctarget-cpu (Rust, md-dpdk C) | Aeron C -march | nodes |
# |----------|-------------------------|--------------------------------|----------------|-------|
# | baseline | linux/amd64,linux/arm64 | x86-64-v2 / neoverse-n1        | x86-64-v2 / armv8.2-a | any x86-64-v2 or Neoverse-N1-or-later arm64: vdev, kind, Apple silicon, Graviton2+, Cobalt, c7i |
# | amd64-v3 | linux/amd64             | x86-64-v3                      | x86-64-v3      | c7i (Sapphire Rapids), AMD Zen 4 (F8as_v6) |
# | arm64-n2 | linux/arm64             | neoverse-n2                    | armv8.5-a      | Azure Cobalt 100 (Neoverse N2, Armv9.0 = v8.5 base) |
# | arm64-v3 | linux/arm64             | neoverse-v3                    | armv8.6-a      | c9g (Graviton5, Neoverse V3, Armv9.2 >= v8.7) |
#
# LL_PLATFORMS=linux/arm64 (or linux/amd64) builds one half of `baseline`
# with the same flags: a machine short of disk builds and loads the halves
# one at a time (both at once need room for two builder stages).
#
# Aeron C takes only -march (rusteron's RUSTERON_C_MARCH): the arm64 levels
# are the highest AL2023's gcc 11.5 names that the CPU implements (gcc 11
# has no armv9-a). The levels are compile choices; no tag but `baseline`
# has been run, and no CPU tag has been measured against `baseline`.
# A CPU tag runs only on that CPU family (SIGILL elsewhere): its overlay
# selects the nodes by instance type and kubernetes.io/arch.
set -euo pipefail

PUSH=""
NAME=ll
while [[ $# -gt 1 ]]; do
  case $1 in
    --push) PUSH=$2; shift 2 ;;
    --name) NAME=$2; shift 2 ;;
    *) echo "build.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done
TAG=${1:?usage: build.sh [--push <registry>] [--name ll] <baseline|amd64-v3|arm64-n2|arm64-v3>}
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
AMD_CPU='' AMD_MARCH='' ARM_CPU='' ARM_MARCH=''
case $TAG in
  baseline) PLATFORM=linux/amd64,linux/arm64 AMD_CPU=x86-64-v2 AMD_MARCH=x86-64-v2 ARM_CPU=neoverse-n1 ARM_MARCH=armv8.2-a ;;
  amd64-v3) PLATFORM=linux/amd64 AMD_CPU=x86-64-v3 AMD_MARCH=x86-64-v3 ;;
  arm64-n2) PLATFORM=linux/arm64 ARM_CPU=neoverse-n2 ARM_MARCH=armv8.5-a ;;
  arm64-v3) PLATFORM=linux/arm64 ARM_CPU=neoverse-v3 ARM_MARCH=armv8.6-a ;;
  *) echo "build.sh: unknown tag $TAG" >&2; exit 2 ;;
esac
PLATFORM=${LL_PLATFORMS:-$PLATFORM}
REF=$NAME:$TAG
OUTPUT=(--load)
if [[ -n $PUSH ]]; then
  REF=$PUSH/$NAME:$TAG
  OUTPUT=(--push)
fi
docker buildx build --platform "$PLATFORM" "${OUTPUT[@]}" \
  -f "$ROOT/samples/low-latency-k8s/docker/Dockerfile" \
  --build-arg "RUST_TARGET_CPU_AMD64=$AMD_CPU" --build-arg "RUSTERON_C_MARCH_AMD64=$AMD_MARCH" \
  --build-arg "RUST_TARGET_CPU_ARM64=$ARM_CPU" --build-arg "RUSTERON_C_MARCH_ARM64=$ARM_MARCH" \
  --build-arg "LL_CPU_BUILD=$TAG" \
  ${CARGO_BUILD_JOBS:+--build-arg "CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS"} \
  -t "$REF" "$ROOT"
if [[ -z $PUSH ]]; then
  for p in ${PLATFORM//,/ }; do
    echo "== $REF $p"
    docker run --rm --platform "$p" "$REF" cat /etc/ll-build.txt
  done
fi
