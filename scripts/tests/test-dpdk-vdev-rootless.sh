#!/usr/bin/env bash
# Self-test of scripts/test-dpdk-vdev-rootless.sh's failure paths (Linux, run
# unprivileged): a refused user namespace fails the run and names the AppArmor
# sysctl that refuses it on Ubuntu 23.10 and later, never a skip or a pass;
# root is turned away to the sudo lane; a namespace without /dev/net/tun or a
# tap fails with the reason. `unshare`, `id` and `ip` are stubbed on PATH, so
# nothing here needs a namespace, a tap or DPDK.
#
# Run from anywhere:  bash scripts/tests/test-dpdk-vdev-rootless.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUNNER="$HERE/../test-dpdk-vdev-rootless.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
die() { echo "FAIL: $*" >&2; exit 1; }

# stub <name> <body>: a command on the stub PATH.
stub() {
    mkdir -p "$TMP/bin"
    printf '#!/usr/bin/env bash\n%s\n' "$2" > "$TMP/bin/$1"
    chmod +x "$TMP/bin/$1"
}

# expect_fail <label> <needle> <command...>: must exit non-zero, never print
# a skip, and say <needle> on stderr.
expect_fail() {
    local label="$1" needle="$2"
    shift 2
    if "$@" >"$TMP/out" 2>"$TMP/err"; then
        cat "$TMP/out" "$TMP/err" >&2
        die "$label: exited 0"
    fi
    grep -qF -- "$needle" "$TMP/err" || { cat "$TMP/err" >&2; die "$label: stderr lacks '$needle'"; }
    ! grep -qi 'skip' "$TMP/out" "$TMP/err" || die "$label: printed a skip"
    echo "$label: failed as expected"
}

# The stubs must run, or every case below tests the real commands (a noexec
# TMPDIR would do that silently).
stub id 'echo stubbed'
[[ "$(env PATH="$TMP/bin:$PATH" id)" == stubbed ]] || die "the stubs in $TMP/bin do not run (noexec TMPDIR?)"

# 1. The user namespace refused (Ubuntu 24.04's AppArmor default, or a
#    container's seccomp profile): fails, naming the sysctl.
stub unshare 'echo "unshare: write failed /proc/self/uid_map: Operation not permitted" >&2; exit 1'
stub id 'echo 1001'
expect_fail "userns refused" "kernel.apparmor_restrict_unprivileged_userns" \
    env PATH="$TMP/bin:$PATH" bash "$RUNNER"
grep -qF "Operation not permitted" "$TMP/err" || die "userns refused: unshare's own error is not passed on"
grep -qF -- "--security-opt seccomp=unconfined" "$TMP/err" || die "userns refused: the Docker hint is missing"

# 2. The self-test mode fails the same way: a probe, not a skip.
expect_fail "self-test, userns refused" "kernel.apparmor_restrict_unprivileged_userns" \
    env PATH="$TMP/bin:$PATH" bash "$RUNNER" --self-test

# 3. Root: this runner is the unprivileged lane; root uses test-dpdk-vdev.sh.
stub id 'echo 0'
expect_fail "run as root" "unprivileged" env PATH="$TMP/bin:$PATH" bash "$RUNNER"

# 4. Inside the namespace without a usable /dev/net/tun: fails with the
#    reason (Docker: --device /dev/net/tun).
stub id 'echo 0'
expect_fail "no /dev/net/tun" "/dev/net/tun" \
    env PATH="$TMP/bin:$PATH" RUSTERON_ROOTLESS_INNER=1 RUSTERON_ROOTLESS_TUN="$TMP/no-tun" bash "$RUNNER" --self-test

# 5. A tap the namespace cannot create: fails, saying net_tap is unusable.
touch "$TMP/tun"
stub ip 'if [[ "$1 $2" == "tuntap add" ]]; then echo "ioctl(TUNSETIFF): Operation not permitted" >&2; exit 1; fi; exit 0'
stub mount 'exit 0'
expect_fail "tap refused" "net_tap" \
    env PATH="$TMP/bin:$PATH" RUSTERON_ROOTLESS_INNER=1 RUSTERON_ROOTLESS_TUN="$TMP/tun" bash "$RUNNER" --self-test

echo "test-dpdk-vdev-rootless.sh self-test PASSED"
