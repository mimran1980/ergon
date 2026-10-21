# Verification environment for the ergon-md-dpdk `dpdk` feature.
# Builds for linux/amd64 (emulated on an arm64 host) or linux/arm64 (Graviton's
# architecture, native on Apple silicon; verify-dpdk.sh --arm64): nothing in it
# names an architecture.
#
# Hosts the full `cargo test -p ergon-md-dpdk --features dpdk` path:
# Rust 1.99.0 (pinned), Aeron C build-from-source deps (cmake/clang/Java),
# and libdpdk >= 23.11 from Ubuntu 24.04. Run verify-dpdk.sh from anywhere.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential cmake clang pkg-config make gcc g++ \
        curl ca-certificates git uuid-dev libbsd-dev \
        openjdk-17-jre-headless \
        python3-pip \
        libdpdk-dev \
    && rm -rf /var/lib/apt/lists/*

# The vendored Aeron requires CMake >= 3.30 (ubuntu 24.04 apt ships 3.28).
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/bin:$PATH \
    CARGO_TARGET_DIR=/tmp/target

RUN python3 -m pip install --break-system-packages --no-cache-dir cmake

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.99.0

WORKDIR /src
