# fstack_lat's local test (docker-test.sh): Rust 1.99.0 on Debian 13, F-Stack
# and its bundled DPDK built by build-fstack.sh (generic platform), and what
# the test needs (iproute2 for the veth pair).
FROM rust:1.99.0-trixie
RUN apt-get update && apt-get install -y --no-install-recommends \
        meson ninja-build python3-pyelftools libnuma-dev libssl-dev zlib1g-dev pkg-config make gcc gawk \
        iproute2 git ca-certificates python3 \
    && rm -rf /var/lib/apt/lists/*
COPY build-fstack.sh ff-run-once.patch /opt/fstack-build/
RUN bash /opt/fstack-build/build-fstack.sh /opt/fstack generic
