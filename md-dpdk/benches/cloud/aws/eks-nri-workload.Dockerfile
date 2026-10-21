# SPDX-License-Identifier: Apache-2.0
#
# The NRI lab's workload image (eks-nri.sh): the cross-host round-trip clients
# (rtt, counters) from ergon-md-dpdk-harness with `--features static`, plus the
# two tiny tools and the gap spinner. Built for linux/arm64 on any host:
#
#   docker buildx build --platform linux/arm64 --load \
#     -f md-dpdk/benches/cloud/aws/eks-nri-workload.Dockerfile \
#     -t ergon-nri/workload:<tag> .
#
# The build context is the ergon repository root. CARGO_INCREMENTAL=0. rusteron
# 0.2.11's static feature runs Aeron's Gradle and CMake from source, so the
# build stage carries JDK 17 (Java 27 fails with an unsupported class file
# version), cmake and clang. Debian bookworm: it is the only release that
# still packages openjdk-17 (trixie dropped it). Bookworm's own cmake is
# 3.25.1 and Aeron's CMakeLists requires 3.30 or higher, so cmake comes from
# cmake.org's official binary. The runtime image matches the build's glibc
# (rtt is dynamically linked). The pods run `prelock --mlock -- rtt ...` (on
# the modes with the RLIMIT_MEMLOCK grant) and kubectl exec runs `counters`
# and the probes.

FROM rust:1.99.0-slim-bookworm AS build
ARG CMAKE_VERSION=3.31.6
RUN apt-get update && apt-get install -y --no-install-recommends \
        make gcc g++ libc6-dev pkg-config \
        clang libclang-dev \
        openjdk-17-jdk-headless \
        uuid-dev libbsd-dev \
        ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && curl -fsSL "https://cmake.org/files/v${CMAKE_VERSION%.*}/cmake-${CMAKE_VERSION}-linux-$(uname -m).tar.gz" \
        | tar -xz -C /opt \
    && ln -s "/opt/cmake-${CMAKE_VERSION}-linux-$(uname -m)/bin/cmake" /usr/local/bin/cmake \
    && cmake --version | head -1
ENV CARGO_INCREMENTAL=0
# rusteron's build script runs rustfmt over the generated bindings and panics
# when the component is missing (it reports "Broken pipe", which is not the
# real cause). The slim image does not carry it.
RUN rustup component add rustfmt
WORKDIR /src
# Every workspace member directory must exist for the workspace to load; only
# the harness and its path deps are compiled. rusteron comes from crates.io.
COPY Cargo.toml Cargo.lock ./
COPY sbe ./sbe
COPY cluster ./cluster
COPY cluster-test-harness ./cluster-test-harness
COPY io ./io
COPY clock ./clock
COPY tune ./tune
COPY md-dpdk ./md-dpdk
RUN cargo build --release -p ergon-md-dpdk-harness --features static --bin rtt --bin counters
WORKDIR /src/md-dpdk/benches/cloud/aws
RUN gcc -O2 -Wall -Wextra -o /out-prelock prelock.c \
 && gcc -O2 -Wall -Wextra -o /out-io-uring-probe io-uring-probe.c \
 && gcc -O2 -Wall -Wextra -o /out-gapspin ../gapspin.c

FROM debian:bookworm-slim
# rtt is dynamically linked; it needs the same C libraries the build linked
# against. util-linux brings libuuid1; libbsd0 is explicit.
RUN apt-get update && apt-get install -y --no-install-recommends \
        coreutils procps util-linux libbsd0 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/rtt /usr/local/bin/rtt
COPY --from=build /src/target/release/counters /usr/local/bin/counters
COPY --from=build /out-prelock /usr/local/bin/prelock
COPY --from=build /out-io-uring-probe /usr/local/bin/io_uring_probe
COPY --from=build /out-gapspin /usr/local/bin/gapspin
# No entrypoint: the pod's command is `prelock ... -- rtt ...` or a probe.
