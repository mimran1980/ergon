# SPDX-License-Identifier: Apache-2.0
#
# The lab's probe image: gapprobe, stress-ng (the helper load) and procps
# (ps -L for the placement check). Built on the lab VM, loaded into kind,
# never pushed.
FROM rust:1.99.0-trixie AS build
WORKDIR /src
COPY Cargo.toml ./
COPY src src
# no dependencies: nothing to lock or fetch
RUN cargo build --release && cargo test --release

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends stress-ng procps \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/gapprobe /usr/local/bin/gapprobe
