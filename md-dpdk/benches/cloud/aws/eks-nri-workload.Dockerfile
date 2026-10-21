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
# build stage carries JDK 25 (Aeron's Gradle 9.7.1 fails on 27 with an
# unsupported class file version), cmake and clang. Debian 13 (trixie)
# packages openjdk-25 and cmake 3.31.6, which meets Aeron's CMakeLists
# minimum of 3.30. The runtime image matches the build's glibc (rtt is
# dynamically linked). The pods run `prelock --mlock -- rtt ...` (on
# the modes with the RLIMIT_MEMLOCK grant) and kubectl exec runs `counters`
# and the probes.

FROM rust:1.99.0-slim-trixie AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
        make gcc g++ libc6-dev pkg-config \
        clang libclang-dev \
        openjdk-25-jdk-headless \
        cmake \
        uuid-dev libbsd-dev \
        ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
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

FROM debian:trixie-slim
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
