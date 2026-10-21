#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The cgroup kernel lane: run the partition plan against a real cgroup v2
# kernel. The unit fixtures in internal/hook manufacture cpuset.* files on
# ordinary directories, so they cannot show what the kernel decides. This can.
#
# It needs write access to the cgroup v2 root, which a plain container does
# not have, so this starts a privileged container with the host's cgroup
# namespace. scripts/check.sh runs this lane after the unit lane.
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
image=${GO_IMAGE:-golang:1.27-trixie}

if [[ "${1:-}" != "--local" ]]; then
    # --privileged --cgroupns=host: the test creates a scratch tree on the
    # real cgroup v2 hierarchy and takes it down again. Without the host
    # cgroup namespace the container only sees its own cgroup, which does not
    # delegate cpuset, and the test cannot run at all.
    # shellcheck disable=SC2086 # CHECK_DOCKER_ARGS is a list of arguments
    exec docker run --rm --name "ergon-nri-cgroup-$$" \
        --privileged --cgroupns=host \
        -v "$here":/src -w /src ${CHECK_DOCKER_ARGS:-} \
        "$image" bash scripts/check-cgroup.sh --local
fi

cd "$here"
export GOFLAGS=-buildvcs=false

echo "== cgroup kernel"
go test -count=1 -tags cgroup_kernel -v ./test/cgroup/
echo "check-cgroup: PASS"
