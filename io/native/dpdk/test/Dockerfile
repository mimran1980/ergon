# Linux environment for ergon-io's `smoltcp-dpdk` feature: Rust 1.99.0 with
# the components rust-toolchain.toml names, libdpdk >= 23.11 (Ubuntu 24.04's), and what the vdev
# lane (scripts/test-io-smoltcp-vdev.sh) needs: iproute2 (veth, tc netem),
# ethtool (the kernel side's checksum offload off), ping. Nothing in it names
# an architecture: linux/arm64 (Graviton's) and linux/amd64 both build.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential pkg-config curl ca-certificates git \
        libdpdk-dev \
        iproute2 iputils-ping ethtool util-linux \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal \
        --default-toolchain 1.99.0 --component clippy,rustfmt,rust-analyzer \
    && chmod -R a+rwX /usr/local/cargo /usr/local/rustup

WORKDIR /src
