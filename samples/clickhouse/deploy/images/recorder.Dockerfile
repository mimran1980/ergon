# Multi-stage build for the recorder pod image.
#
# Build it through `deploy/images/build.sh ergo/market-recorder:local`, which
# stages the context and supplies the prebuilt wheel. The context has to be
# repository-relative: the sample's Cargo manifest path-depends on `../../sbe`
# and the bridge on `../persist`, so a context limited to `samples/clickhouse`
# cannot resolve them.
#
# Stage 1 is the wheel's toolchain, and is used as a standalone image rather
# than only as a build step: the `ergo_recorder` PyO3 wheel is compiled **on
# the host over a bind mount** and stage 2 only COPYs the result. Compiling it
# inside the image wants ~5.5GB of `target/` inside the Docker VM — more than
# this host's daemon has free — and wedged the daemon repeatedly (four times on
# 2026-09-20). Over a bind mount the same compile writes into the host's
# `target/`, which already holds the Linux artifacts the ingester image left
# there, so it is both off the VM's disk and warm. Stage 3 installs the pinned
# NautilusTrader wheel (a manylinux_2_35_aarch64 wheel exists for CPython 3.12)
# plus the app, and never carries Rust in.

# The wheel must be built against the *runtime* interpreter: bookworm's default
# `python3` is 3.11 and produced a cp311 wheel that the 3.12 runtime rejects as
# "not a supported wheel on this platform". Base this stage on the same Python
# as stage 3 and add only a toolchain.
#
FROM python:3.12-slim AS wheel-toolchain
# rusteron builds Aeron from source and needs, in order of how they failed:
# CMake >= 3.30 (Debian's is older), a C++ toolchain *including make* (the
# Kitware tarball has no generator of its own and the slim base has no `make`),
# libclang for bindgen, libbsd and libuuid for the native link, pkg-config for
# rusteron to *find* libuuid, and a JDK.
RUN apt-get update && apt-get install -y --no-install-recommends \
        curl xz-utils make gcc g++ libc6-dev ca-certificates \
        clang libclang-dev llvm-dev libbsd-dev uuid-dev pkg-config default-jdk-headless \
    && rm -rf /var/lib/apt/lists/* \
    && CM=3.30.5 \
    && curl -fsSL "https://github.com/Kitware/CMake/releases/download/v${CM}/cmake-${CM}-linux-aarch64.tar.gz" -o /tmp/cmake.tgz \
    && tar xzf /tmp/cmake.tgz -C /opt \
    && ln -sf /opt/cmake-${CM}-linux-aarch64/bin/cmake /usr/local/bin/cmake \
    && rm /tmp/cmake.tgz
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain 1.98.0
# rusteron's build script formats its generated bindings with rustfmt, which
# the minimal profile omits; without it the build panics with "rustfmt failed -
# likely syntax error in generated code".
# Absolute path: the `ENV PATH` that puts cargo on the path is set below.
RUN /root/.cargo/bin/rustup component add rustfmt
ENV PATH=/root/.cargo/bin:$PATH
# maturin bundles the shared objects the extension needs into the wheel by
# rewriting their RPATHs, which needs patchelf. Without it the build fails
# after the link with "Failed to execute 'patchelf'".
RUN pip install --no-cache-dir "maturin[patchelf]==1.15.0"

# The wheel, built by `build.sh` over a bind mount into
# `samples/clickhouse/target/wheels` and staged into the context as `wheels/`.
# The cargo *registry* is the only thing worth caching across builds; a cached
# `/src/target` grows the daemon's build cache by ~20 GB, which is enough to
# fill a default Docker Desktop disk — the failure this split exists to avoid.
FROM python:3.12-slim AS wheel
COPY wheels /wheels

FROM python:3.12-slim
# The app is run from source rather than installed, because it locates its
# fixtures relative to its own file (`node.py` -> parents[4] -> the sample
# root). Installing it into site-packages moves it and the path no longer
# resolves; keeping the source layout and `fixtures/` beside it does.
WORKDIR /src/samples/clickhouse
COPY samples/clickhouse/apps ./apps
COPY samples/clickhouse/fixtures ./fixtures
COPY --from=wheel /wheels /wheels
# maturin bundles the extension's shared objects (libaeron, libbsd, libmd) into
# the wheel and rewrites their RPATHs, so the runtime needs no LD_LIBRARY_PATH
# and no hand-copied libraries.
RUN pip install --no-cache-dir \
        "nautilus-trader==1.231.0" \
        /wheels/*.whl \
    && rm -rf /wheels
ENV PYTHONPATH=/src/samples/clickhouse/apps/market-recorder/src
# `--mode fixture` replays the baked fixtures; `--mode live` builds a data-only
# NautilusTrader node. Either way no exchange credential is passed in.
ENTRYPOINT ["python3", "-m", "market_recorder"]
