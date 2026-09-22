#!/usr/bin/env bash
# Build the local images for the sample stack.
#
# Two classes of image:
#   * the Rust images (ingester, archive-agent) COPY binaries that must be
#     **Linux** executables, and the ingester also needs the Aeron shared
#     objects rusteron builds into the cargo tree;
#   * the recorder image compiles its PyO3 wheel on the host over a bind mount
#     (so it needs neither a host binary nor a Linux host), and COPYs it in.
#
# There is no cross-compilation here, so the Rust images fail loudly on a
# non-Linux host rather than producing images that pull and then CrashLoop with
# "exec format error".
set -euo pipefail
# `ROOT` is the sample; the recorder context also needs the repository root,
# because the sample's Cargo manifest path-depends on `../../sbe`.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"
IMG="${1:?usage: build.sh <image>}"
log() { printf '[images] %s\n' "$*"; }

BIN_DIR="${ERGO_BIN_DIR:-$ROOT/target/release}"
# The Dockerfiles below are fed on stdin with $ROOT as the build context, so
# the paths they COPY are relative to it.
BIN_REL="${BIN_DIR#"$ROOT"/}"

require_linux_host() {
    if [[ "$(uname -s)" != "Linux" && "${ERGO_ALLOW_CROSS_IMAGES:-}" != "1" ]]; then
        cat >&2 <<'MSG'
[images] this image COPYs binaries built on this host, and it needs linux/arm64
[images] or linux/amd64 ones. Either:
[images]   1. run this script on a Linux build host, or
[images]   2. build for the container platform and re-run with the binaries in
[images]      target/<triple>/release, then set ERGO_ALLOW_CROSS_IMAGES=1.
[images] Refusing to build an image from macOS binaries: it would pull and then
[images] CrashLoop, which reads as a stack failure rather than a build one.
[images]
[images] There is no cross-compilation here, so from macOS the binary has to be
[images] built inside a Linux container over a bind mount. Four things bite, all
[images] of them found by running it:
[images]   * `bash -lc` re-sources the login profile and loses the Rust image's
[images]     PATH, so cargo is "command not found". Use `bash -c` with PATH set.
[images]   * rusteron builds Aeron from source and needs CMake >= 3.30; Debian's
[images]     is 3.25. Fetch the Kitware tarball (see recorder.Dockerfile).
[images]   * rusteron-archive needs a JDK >= 17 (`find_package(Java)`), so
[images]     default-jdk-headless has to be installed as well.
[images]   * the container's CARGO_HOME is discarded with --rm, re-downloading
[images]     ~250MB of crates every run. Mount a registry cache if iterating.
[images] The working invocation is recorded in PLAN.md ("Building the Linux
[images] images from macOS") rather than duplicated here.
MSG
        exit 1
    fi
}

require_binary() {
    [[ -x "$BIN_DIR/$1" ]] \
        || { echo "[images] missing $BIN_DIR/$1 — build it first" >&2; exit 1; }
}

# rusteron builds Aeron into the cargo tree rather than installing it, so the
# shared objects the ingester links against are in no system path. Both the
# client and the archive C libraries are needed: the archive one is discovered
# only at run time, so a missing copy fails right after libaeron.so resolves.
aeron_lib_dirs() {
    local dirs out=""
    dirs="$(find "$ROOT/target/release/build" -name 'libaeron.so' -exec dirname {} \; 2>/dev/null | sort -u)"
    [[ -n "$dirs" ]] \
        || { echo "[images] libaeron.so not found under target/release/build" >&2; exit 1; }
    for d in $dirs; do out="$out ${d#"$ROOT"/}"; done
    printf '%s' "$out"
}

# The recorder image stages its own context so the multi-gigabyte cargo
# `target/` trees never reach the daemon.
recorder_context() {
    local stage src
    stage="$(mktemp -d)"
    mkdir -p "$stage/samples/clickhouse"
    # Fail here, naming the path, rather than at the daemon with
    # "/samples/clickhouse/apps: not found".
    stage_path() {
        src="$1"
        [[ -e "$src" ]] || { echo "[images] missing source for the context: $src" >&2; exit 1; }
    }
    # `sbe/Cargo.toml` inherits `workspace.package.edition` from the repository
    # root, so that manifest must be staged too or `cargo metadata` fails with
    # "failed to find a workspace root". The sample's own workspace stays
    # separate — the root excludes `samples`.
    # `apps/` and `crates/` also carry dev-dependencies on `tests/support`.
    stage_path "$REPO/Cargo.toml"
    stage_path "$REPO/Cargo.lock"
    stage_path "$REPO/sbe"
    for d in apps crates fixtures tests; do stage_path "$ROOT/$d"; done
    stage_path "$ROOT/Cargo.toml"
    stage_path "$ROOT/Cargo.lock"
    cp "$REPO/Cargo.toml" "$REPO/Cargo.lock" "$stage/"
    cp -R "$REPO/sbe" "$stage/sbe"
    for d in apps crates fixtures tests; do
        cp -R "$ROOT/$d" "$stage/samples/clickhouse/$d"
    done
    cp "$ROOT/Cargo.toml" "$ROOT/Cargo.lock" "$stage/samples/clickhouse/"
    printf '%s' "$stage"
}

cd "$ROOT"

case "$IMG" in
    ergo/archive-agent:local)
        require_linux_host
        require_binary archive-agent
        docker build -q -f - -t "$IMG" . <<DOCKER
FROM debian:bookworm-slim
COPY $BIN_REL/archive-agent /usr/local/bin/archive-agent
ENTRYPOINT ["/usr/local/bin/archive-agent"]
DOCKER
        ;;
    ergo/ingester:local)
        require_linux_host
        require_binary ingester
        AERON_RELS="$(aeron_lib_dirs)"
        log "aeron libs:${AERON_RELS}"
        docker build -q -f - -t "$IMG" . <<DOCKER
FROM debian:bookworm-slim
# The binary links libbsd (rusteron's native build uses it), which the slim
# base does not carry.
RUN apt-get update \
 && apt-get install -y --no-install-recommends libbsd0 \
 && rm -rf /var/lib/apt/lists/*
COPY $BIN_REL/ingester /usr/local/bin/ingester
COPY $AERON_RELS/ /usr/local/lib/
RUN ldconfig
ENV LD_LIBRARY_PATH=/usr/local/lib
ENTRYPOINT ["/usr/local/bin/ingester"]
DOCKER
        ;;
    ergo/market-recorder:local)
        log "building $IMG (wheel built on the host over a bind mount)"
        stage="$(recorder_context)"
        trap 'rm -rf "$stage"' EXIT
        # The wheel's toolchain, built as a standalone image so the compile
        # below can run *outside* the image. Cached after the first build, so
        # this is a no-op on every later one.
        docker build -q --target wheel-toolchain \
            -t ergo/recorder-wheel-toolchain:local \
            -f "$ROOT/deploy/images/recorder.Dockerfile" "$stage" >/dev/null
        # Compile the wheel over a bind mount. It writes into the host's
        # `target/` — where the Linux artifacts the ingester image left behind
        # already are — rather than several GB of `target/` inside the Docker
        # VM, which is what filled the daemon's disk and wedged it four times
        # on 2026-09-20. `bash -c`, not `-lc`: the login profile drops the
        # image's PATH.
        log "disk before: $(df -h / | awk 'NR==2 {print $4}') free"
        mkdir -p /tmp/ergo-cargo-registry
        docker run --rm \
            -v "$REPO":/src \
            -v /tmp/ergo-cargo-registry:/root/.cargo/registry \
            -w /src \
            ergo/recorder-wheel-toolchain:local \
            maturin build --release \
                --interpreter python3 \
                --manifest-path samples/clickhouse/crates/python-bridge/Cargo.toml \
                --features pyo3/extension-module \
                --out samples/clickhouse/target/wheels
        mkdir -p "$stage/wheels"
        cp "$ROOT"/target/wheels/*.whl "$stage/wheels/"
        docker build -q \
            -f "$ROOT/deploy/images/recorder.Dockerfile" \
            -t "$IMG" "$stage"
        log "disk after: $(df -h / | awk 'NR==2 {print $4}') free"
        ;;
    ergo/archive-driver:local)
        # Latest Temurin JRE (25). Not the `-alpine` variant: it publishes no
        # arm64 manifest, so the image cannot be built on Apple Silicon at all.
        # The Ubuntu-based tag has amd64, arm and arm64.
        docker build -q -f - -t "$IMG" . <<'DOCKER'
FROM eclipse-temurin:25-jre
WORKDIR /opt/aeron
RUN apt-get update && apt-get install -y --no-install-recommends wget \
 && rm -rf /var/lib/apt/lists/* \
 && wget -q https://repo1.maven.org/maven2/io/aeron/aeron-all/1.53.2/aeron-all-1.53.2.jar
ENV AERON_DIR=/dev/shm/aeron
CMD ["java", "-cp", "aeron-all-1.53.2.jar", "io.aeron.archive.ArchivingMediaDriver"]
DOCKER
        ;;
    ergo/jupyter-lab:local)
        # KNOWN BROKEN IN KIND — see PLAN.md, "JupyterLab cannot run on this
        # host". Any image built from a Jupyter base runs fine on the host but
        # every exec inside a node fails with "exec format error", `/bin/sh`
        # included. The cause is NOT size: a `minimal-notebook` build was tried
        # at 2.35GB (against `scipy-notebook`'s 5.43GB) with 6.2GB free, and it
        # failed identically. Nor is it the platform, the transport (`kind load`
        # and a registry pull both fail), or stale snapshots — each was ruled
        # out by running it. Do not treat `kind load` succeeding as evidence
        # that this image is usable.
        docker build -q -f - -t "$IMG" . <<'DOCKER'
FROM jupyter/scipy-notebook:latest
RUN pip install --no-cache-dir clickhouse-connect nbclient nbformat nbconvert matplotlib
DOCKER
        ;;
    *)
        echo "unknown image: $IMG" >&2
        exit 1
        ;;
esac
log "built $IMG"
