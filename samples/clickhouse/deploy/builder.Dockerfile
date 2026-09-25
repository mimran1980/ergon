# Builds the Linux binaries. rusteron compiles Aeron's C client from source:
# it needs CMake >= 3.30 (newer than Debian's, so from pip), clang for
# bindgen, libbsd and libuuid, rustfmt, and a JDK for the archive codecs.
FROM rust:1.98.1-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
        clang libclang-dev libbsd-dev uuid-dev pkg-config python3-pip default-jdk-headless \
    && pip install --no-cache-dir --break-system-packages cmake \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add rustfmt
