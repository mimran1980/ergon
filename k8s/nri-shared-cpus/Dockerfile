# SPDX-License-Identifier: Apache-2.0
#
# Build both binaries statically, then ship them in a scratch image.
# The DaemonSet copies ergon-cpus to the node; the NRI plugin runs from here.
#
#   docker build -t ergon-shared-cpus:dev .
#   docker buildx build --platform linux/amd64,linux/arm64 -t ergon-shared-cpus:dev .
#
# Published as ghcr.io/mimran1980/ergon-shared-cpus:0.1.1.
# See RELEASE.md for what is still not verified.

FROM golang:1.27-trixie AS build
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
ARG VERSION=dev
ARG GIT_DESCRIBE=""
RUN mkdir -p /out \
 && CGO_ENABLED=0 GOOS=linux go build \
      -ldflags "-s -w -X main.version=${VERSION} -X main.gitDescribe=${GIT_DESCRIBE}" \
      -o /out/ergon-shared-cpus ./cmd/ergon-shared-cpus \
 && CGO_ENABLED=0 GOOS=linux go build \
      -ldflags "-s -w -X main.version=${VERSION}" \
      -o /out/ergon-cpus ./cmd/ergon-cpus

FROM scratch
ARG VERSION=dev
ARG GIT_DESCRIBE=""
# OCI image labels. VERSION and GIT_DESCRIBE are redeclared here because ARG
# does not cross a stage boundary.
LABEL org.opencontainers.image.title="ergon-shared-cpus" \
      org.opencontainers.image.description="NRI plugin and kubelet device plugin that give a pod its exclusive CPUs and the node's shared housekeeping CPUs" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/mimran1980/ergon" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${GIT_DESCRIBE}"
COPY --from=build /out/ergon-shared-cpus /ergon-shared-cpus
COPY --from=build /out/ergon-cpus /ergon-cpus
# The hook binary must be at the path the config names; the DaemonSet mounts
# /opt/ergon/bin from the host and the daemon installs ergon-cpus there.
USER 0:0
ENTRYPOINT ["/ergon-shared-cpus"]
