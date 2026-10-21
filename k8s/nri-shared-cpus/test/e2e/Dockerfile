# SPDX-License-Identifier: Apache-2.0
#
# The e2e test image: pin-test (C, holds a pinned thread and a helper), the
# pinning examples and ergon-cpus, on a small Debian base.
#
# Build from the module root (the e2e script does this):
#   docker build -f test/e2e/Dockerfile -t ergon-nri-pin-test:e2e .

# The golang image has gcc, so the C programs are built here and only their
# binaries are copied out. No apt in the final image: the e2e box runs out of
# disk when a stage has to download a compiler.
FROM golang:1.27-trixie AS tools
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN mkdir -p /out \
 && CGO_ENABLED=0 go build -o /out/ergon-cpus ./cmd/ergon-cpus \
 && CGO_ENABLED=0 go build -o /out/ergon-shared-cpus ./cmd/ergon-shared-cpus \
 && gcc -O2 -pthread -o /out/pin-test test/e2e/pin-test.c \
 && gcc -O2 -o /out/pin examples/pin.c

FROM rust:1.99.0-slim-trixie AS rustbuild
WORKDIR /src
COPY examples/pin.rs /src/pin.rs
# rustfmt --check keeps the example in the repo's format. The slim image has
# no rustfmt component, so add it first.
RUN rustup component add rustfmt \
 && mkdir -p /out \
 && rustfmt --check /src/pin.rs \
 && rustc -O -o /out/pin-rs /src/pin.rs

FROM debian:trixie-slim
COPY --from=tools /out/ergon-cpus /usr/local/bin/ergon-cpus
COPY --from=tools /out/ergon-shared-cpus /usr/local/bin/ergon-shared-cpus
COPY --from=tools /out/pin-test /usr/local/bin/pin-test
COPY --from=tools /out/pin /usr/local/bin/pin
COPY --from=rustbuild /out/pin-rs /usr/local/bin/pin-rs
# The entrypoint is the in-pod helper used by the e2e script.
CMD ["pin-test","probe"]
