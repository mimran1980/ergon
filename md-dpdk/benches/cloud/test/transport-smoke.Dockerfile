# The transport suite's local smoke (test/transport-smoke-docker.sh): Rust
# 1.99.0 on Debian bookworm, the Aeron C build's needs (clang, CMake >= 3.30
# from pip: bookworm's is 3.25), and what the runner calls on a host
# (iproute2's tc, procps's ps, sudo, gawk). No JDK: no Aeron jar is built.
FROM rust:1.99.0-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
        clang libclang-dev uuid-dev libbsd-dev iproute2 procps sudo python3 python3-pip gawk \
    && rm -rf /var/lib/apt/lists/* \
    && python3 -m pip install --break-system-packages --no-cache-dir cmake
