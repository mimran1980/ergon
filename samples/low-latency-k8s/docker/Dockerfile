# SPDX-License-Identifier: Apache-2.0
#
# One image for the sample's three containers: the DPDK media driver
# (ergon-md-dpdk's media_driver behind docker/driver-entrypoint.sh), the app
# (ll-app) and the probe (ll-probe), plus ergon-cpus from the published
# ergon-shared-cpus image. Amazon Linux 2023, as md-dpdk's EKS image.
#
# Build from the ergon repository root (docker/build.sh picks the flags per
# tag):
#   docker buildx build --platform linux/arm64 --load \
#     -f samples/low-latency-k8s/docker/Dockerfile \
#     --build-arg RUSTERON_C_MARCH=armv8.2-a --build-arg RUST_TARGET_CPU=neoverse-n1 \
#     --build-arg LL_CPU_BUILD=arm64-generic -t ll:arm64-generic .
#
# Build arguments:
#   RUST_TARGET_CPU    empty: rustc's generic CPU. A name becomes -Ctarget-cpu,
#                      and md-dpdk's build.rs gives its C the same -mcpu/-march
#                      (it stops when gcc does not know the name).
#   RUSTERON_C_MARCH   -march for rusteron's Aeron C. Always set: unset, rusteron
#                      builds it -march=native for the builder's CPU (or the
#                      emulator's under buildx).
#   LL_CPU_BUILD       the tag's name, recorded in /etc/ll-build.txt and in
#                      every ll-latency report (LL_CPU_BUILD).
#   CARGO_BUILD_JOBS   empty: cargo's own; a number caps the parallel jobs.

# ergon-cpus: the same version as the plugin's hook (k8s/nri-shared-cpus).
FROM ghcr.io/mimran1980/ergon-shared-cpus:0.1.1@sha256:69893e0c23ec6c3656053c0d6c4a06c676e33b726d56f362a30ce760908796c0 AS ergon-cpus

FROM public.ecr.aws/amazonlinux/amazonlinux:2023 AS builder
RUN dnf install -y \
        gcc gcc-c++ make clang \
        pkgconf-pkg-config \
        libbsd-devel libuuid-devel openssl-devel zlib-devel \
        numactl-devel \
        meson ninja-build tar xz \
        python3-pip \
    && dnf clean all \
    && python3 -m pip install --no-cache-dir cmake pyelftools

# DPDK 24.11.4, as md-dpdk's EKS image, plus net/tap for the vdev overlays.
# -Dplatform=generic: tuned to no builder CPU.
ARG DPDK_VERSION=24.11.4
ARG DPDK_SHA256=5c387f19f0df9d38d417f7bd0e7d10920d432a14f2414914854a9f2e781d3453
RUN curl -fsSLO "https://fast.dpdk.org/rel/dpdk-${DPDK_VERSION}.tar.xz" \
    && echo "${DPDK_SHA256}  dpdk-${DPDK_VERSION}.tar.xz" | sha256sum -c - \
    && mkdir dpdk \
    && tar -xJf "dpdk-${DPDK_VERSION}.tar.xz" -C dpdk --strip-components=1 \
    && cd dpdk \
    && meson setup build -Dplatform=generic -Dtests=false -Ddisable_apps='*' \
        -Denable_drivers=bus/pci,bus/vdev,mempool/ring,net/ena,net/tap \
    && ninja -C build \
    && meson install -C build \
    && cd / && rm -rf dpdk "dpdk-${DPDK_VERSION}.tar.xz"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain 1.99.0 --profile minimal
ENV PATH="/root/.cargo/bin:$PATH" \
    PKG_CONFIG_PATH="/usr/local/lib64/pkgconfig"
# AL2023's gcc does not default to PIE: position-independent Aeron C, so the
# PIE Rust binaries link (as md-dpdk/deploy/aws-eks/Dockerfile.media-driver).
RUN echo 'set(CMAKE_POSITION_INDEPENDENT_CODE ON)' > /usr/local/share/rusteron-pic.cmake
ENV CMAKE_TOOLCHAIN_FILE="/usr/local/share/rusteron-pic.cmake"

WORKDIR /src
COPY . .
ARG RUST_TARGET_CPU=
ARG RUSTERON_C_MARCH=
ARG LL_CPU_BUILD=unset
ARG CARGO_BUILD_JOBS=
# One layer that builds, copies the binaries out and drops the target dirs.
RUN test -n "$RUSTERON_C_MARCH" || { echo "RUSTERON_C_MARCH must be set (docker/build.sh)" >&2; exit 1; } \
    && export RUSTERON_C_MARCH RUSTFLAGS="${RUST_TARGET_CPU:+-Ctarget-cpu=$RUST_TARGET_CPU}" \
    && cargo build ${CARGO_BUILD_JOBS:+-j $CARGO_BUILD_JOBS} --locked -p ergon-md-dpdk --release --features static,dpdk --bin media_driver \
    && (cd samples/low-latency-k8s && cargo build ${CARGO_BUILD_JOBS:+-j $CARGO_BUILD_JOBS} --locked --release --bins --target-dir /src/target) \
    && mkdir -p /out \
    && cp target/release/media_driver target/release/ll-app target/release/ll-probe /out/ \
    && { \
        echo "ll-cpu-build: ${LL_CPU_BUILD}"; \
        echo "arch: $(uname -m)"; \
        echo "rust-target-cpu: ${RUST_TARGET_CPU:-generic}"; \
        echo "rusteron-c-march: ${RUSTERON_C_MARCH}"; \
        echo "dpdk: $(pkg-config --modversion libdpdk) (dpdk-${DPDK_VERSION}, platform generic, bus/pci bus/vdev mempool/ring net/ena net/tap)"; \
        echo "distro: $(cat /etc/system-release)"; \
        echo "gcc: $(gcc -dumpfullversion)"; \
    } > /out/ll-build.txt \
    && rm -rf target samples/low-latency-k8s/target /root/.cargo/registry

FROM public.ecr.aws/amazonlinux/amazonlinux:2023
RUN dnf install -y numactl-libs libbsd libuuid util-linux procps-ng iproute \
    && dnf clean all
COPY --from=builder /usr/local/lib64 /usr/local/lib64
RUN echo "/usr/local/lib64" > /etc/ld.so.conf.d/ll-dpdk.conf && ldconfig
COPY --from=ergon-cpus /ergon-cpus /usr/local/bin/ergon-cpus
COPY --from=builder /out/media_driver /out/ll-app /out/ll-probe /usr/local/bin/
COPY --from=builder /out/ll-build.txt /etc/ll-build.txt
COPY samples/low-latency-k8s/docker/driver-entrypoint.sh /usr/local/bin/driver-entrypoint.sh
ARG LL_CPU_BUILD=unset
ENV LL_CPU_BUILD=${LL_CPU_BUILD}
RUN chmod +x /usr/local/bin/driver-entrypoint.sh \
    && /usr/local/bin/ergon-cpus --version \
    && ls /usr/local/lib64/dpdk/pmds-*/librte_net_tap.so >/dev/null
CMD ["/usr/local/bin/ll-app"]
