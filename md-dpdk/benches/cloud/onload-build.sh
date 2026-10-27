#!/usr/bin/env bash
# onload-build.sh <source dir> <repository> <commit> <patch>: OpenOnload for
# the transport suite's Onload arms (setup onload; vm.sh transport-build
# onload), AF_XDP only: <repository> at exactly <commit> in <source dir>, the
# patch applied, built for the running kernel (onload_build --no-sfc) and
# installed (onload_install --nobuild --no-sfc --nomodulesloadd: the modules
# load only when vm.sh onload-up loads them, never at boot); kept between
# runs (a stamp of commit, patch and kernel). 1, saying why, unless the
# `onload` launcher, its library and the three modules are there after.
#
# The patch (onload-kernel-7.2.patch) carries what the pinned tree needs on
# the lab's kernel (Debian 13's trixie-backports 7.2) and on AArch64: a
# strncpy() and an rdmsrq() where Linux 7.2 has none, tcp_set_state() no
# longer exported, efrm_syscall_table_found on AArch64, the kernel's own
# aarch64_insn_is_bti, and three x86-only spots in the user-level build
# (emmintrin.h, BMI2's pdep, eflatency's clock). Checked by building the
# commit with it against Debian's linux-headers-7.2.9+deb13-cloud-{amd64,arm64}
# (and unpatched, x86_64, against 6.12.113+deb13-cloud-amd64).
set -euo pipefail
src=${1:?source dir} repo=${2:?repository} commit=${3:?commit} patch=${4:?patch}
[[ $commit =~ ^[0-9a-f]{40}$ ]] || { echo "onload-build: commit $commit is not a full SHA-1" >&2; exit 2; }
[[ -r $patch ]] || { echo "onload-build: no patch $patch" >&2; exit 2; }
kver=$(uname -r)
stamp="$commit $(sha256sum "$patch" | cut -d' ' -f1) $kver"
installed() {
    command -v onload >/dev/null && [[ -n $(ls /usr/lib/*/libonload.so* /usr/lib64/libonload.so* /usr/lib/libonload.so* 2>/dev/null) ]] &&
        modinfo -k "$kver" onload sfc_resource sfc_char >/dev/null 2>&1
}
if [[ $(cat "$src/.ergon-onload-stamp" 2>/dev/null) == "$stamp" ]] && installed; then
    echo "onload-build: $commit with $(basename "$patch") already built and installed for $kver"
    exit 0
fi
sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq build-essential git gawk kmod libcap-dev libmnl-dev libpcap-dev \
    "linux-headers-$kver" >/dev/null
if [[ ! -d $src/.git ]]; then
    rm -rf "$src"
    git clone -q --filter=blob:none "$repo" "$src"
fi
cd "$src"
git fetch -q origin "$commit" 2>/dev/null || git fetch -q origin
git checkout -q --force "$commit"
git clean -qfdx
[[ $(git rev-parse HEAD) == "$commit" ]] || { echo "onload-build: HEAD is $(git rev-parse HEAD), not $commit" >&2; exit 1; }
git apply --check "$patch" || { echo "onload-build: $(basename "$patch") does not apply to $commit" >&2; exit 1; }
git apply "$patch"
./scripts/onload_build --no-sfc --kernelver "$kver"
sudo ./scripts/onload_install --nobuild --no-sfc --nomodulesloadd --kernelver "$kver"
sudo depmod "$kver"
installed || { echo "onload-build: onload, libonload or the onload, sfc_resource and sfc_char modules missing after the install" >&2; exit 1; }
echo "$stamp" >"$src/.ergon-onload-stamp"
echo "onload-build: $(onload --version 2>&1 | head -1) ($commit with $(basename "$patch")) installed for $kver"
