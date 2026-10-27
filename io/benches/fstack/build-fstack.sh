#!/usr/bin/env bash
# build-fstack.sh <prefix> [native|generic]: F-Stack, pinned, with
# ff-run-once.patch, and the DPDK it bundles, both static, under <prefix>
# (fstack_lat's build.rs reads FSTACK_PREFIX=<prefix>).
#
# F-Stack does not build against a stock DPDK: its lib calls
# rte_timer_meta_init, which only its bundled DPDK (24.11.8 plus a few
# patches: EAL cleanup, rte_timer, i40e, mlx5; ENA's PMD is upstream's) has.
# So this DPDK lives under <prefix> alone, static, and is linked only into
# fstack_lat: the media driver's DPDK (Debian's 24.11, or vm.sh build-dpdk's
# /opt/dpdk-25.11) is untouched. Never on PKG_CONFIG_PATH outside this
# script.
#
# platform: native (the lab VM's own CPU, as every other lab build) or
# generic (Docker under emulation, CI).
set -euo pipefail

prefix=${1:?usage: build-fstack.sh <prefix> [native|generic]}
# x86_64 only. On arm64 (Graviton) neither v2.0 nor v1.25 builds with
# Debian 13's GCC 14: F-Stack's arm64 per-CPU glue declares pcpup as a
# global register variable in x18 (lib/include/machine/pcpu.h), refused as
# call-clobbered (v1.25) or clashing with its __thread pcpup (v2.0, which
# also lists libkern fls.c/flsl.c that FreeBSD 15 removed). Checked
# 2026-10-26 in Docker (arm64 native).
case $(uname -m) in
x86_64) ;;
*) echo "build-fstack: F-Stack builds on x86_64 only (not $(uname -m)): see this script's header" >&2; exit 2 ;;
esac
platform=${2:-native}
here=$(cd "$(dirname "$0")" && pwd)
# F-Stack v2.0 (2026-10-20): DPDK 24.11.8, FreeBSD 15.0
FSTACK_TAG=v2.0
FSTACK_COMMIT=6a520b386d3f15265e54d87fe0e9fdc83d4da794
# what F-Stack's lib links: the ENA (AWS), af_packet (the Docker test's
# veth pair), bonding and virtio (F-Stack's lib references both), ring
DRIVERS=bus/pci,bus/vdev,mempool/ring,net/ena,net/af_packet,net/bonding,net/virtio,net/ring

src=$prefix/src/f-stack
mkdir -p "$prefix/src"
if [[ ! -d $src/.git ]]; then
    git -c advice.detachedHead=false clone -q --depth 1 --branch "$FSTACK_TAG" https://github.com/F-Stack/f-stack.git "$src"
fi
got=$(git -C "$src" rev-parse HEAD)
[[ $got == "$FSTACK_COMMIT" ]] || { echo "build-fstack: $FSTACK_TAG is $got, not $FSTACK_COMMIT" >&2; exit 1; }
# ff-run-once.patch: ff_run_once(), one pass of ff_run's loop, so the bench's
# loop owns the thread (as ergon's runtime would) instead of ff_run.
if ! grep -q ff_run_once "$src/lib/ff_api.h"; then
    git -C "$src" apply "$here/ff-run-once.patch"
fi
# F-Stack's DPDK builds its igb_uio on Linux whatever enable_kmods says
# (it comments out upstream's `if is_freebsd`), which needs the running
# kernel's headers and installs under /lib/modules. The lab loads the
# igb_uio vm.sh builds from dpdk-kmods (wc_activate=1), so upstream's guard
# goes back: no module here.
sed -i 's/^#if is_freebsd$/if is_freebsd/; s/^#endif$/endif/' "$src/dpdk/kernel/meson.build"

dpdk=$prefix/dpdk
if [[ ! -f $dpdk/.built ]]; then
    (cd "$src/dpdk" &&
        meson setup --wipe build-ergon -Dprefix="$dpdk" -Dlibdir=lib -Ddefault_library=static \
            -Dplatform="$platform" -Denable_drivers="$DRIVERS" -Ddisable_apps='*' -Dtests=false \
            -Denable_kmods=false -Dexamples= -Dc_args=-fPIC &&
        ninja -C build-ergon && ninja -C build-ergon install) >"$prefix/dpdk-build.log" 2>&1 ||
        { tail -30 "$prefix/dpdk-build.log" >&2; exit 1; }
    touch "$dpdk/.built"
fi
export PKG_CONFIG_PATH=$dpdk/lib/pkgconfig
echo "build-fstack: DPDK $(pkg-config --modversion libdpdk) at $dpdk"
make -C "$src/lib" -j"$(nproc)" FF_PATH="$src" >"$prefix/fstack-build.log" 2>&1 ||
    { tail -30 "$prefix/fstack-build.log" >&2; exit 1; }
mkdir -p "$prefix/lib" "$prefix/include"
cp "$src/lib/libfstack.a" "$prefix/lib/"
cp "$src/lib/ff_api.h" "$src/lib/ff_event.h" "$src/lib/ff_errno.h" "$src/lib/ff_epoll.h" "$prefix/include/"
{
    echo "fstack_tag=$FSTACK_TAG"
    echo "fstack_commit=$FSTACK_COMMIT"
    echo "fstack_patch_sha256=$(sha256sum "$here/ff-run-once.patch" | cut -d' ' -f1)"
    echo "dpdk=$(pkg-config --modversion libdpdk)"
    echo "platform=$platform"
} >"$prefix/BUILD"
echo "build-fstack: $prefix/lib/libfstack.a ($FSTACK_TAG, patched)"
