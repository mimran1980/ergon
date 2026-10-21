#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# gofmt, go vet, go test -race and a static build for amd64 and arm64, then
# the cgroup kernel lane. Both run in a golang container, so nothing is
# installed on the host.
#   scripts/check.sh            both lanes in Docker (needs only docker)
#   scripts/check.sh --local    the unit lane with the go on PATH (CI)
#   scripts/check-cgroup.sh     the cgroup kernel lane on its own
# Extra `docker run` arguments, e.g. a module cache volume:
#   CHECK_DOCKER_ARGS="-v my-gomod:/go/pkg/mod" scripts/check.sh
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
image=${GO_IMAGE:-golang:1.27-trixie}

if [[ "${1:-}" != "--local" ]]; then
    # --cap-add=SYS_NICE: SetTimerSlack writes another process's
    # /proc/PID/timerslack_ns, which the kernel refuses without CAP_SYS_NICE
    # (timerslack_ns_write, `task != current`). The OCI hook has the
    # capability, because containerd runs hooks as root with full
    # capabilities. The tests must run where the write can succeed.
    # shellcheck disable=SC2086 # CHECK_DOCKER_ARGS is a list of arguments
    docker run --rm --name "ergon-nri-check-$$" --cap-add=SYS_NICE \
        -v "$here":/src -w /src ${CHECK_DOCKER_ARGS:-} \
        "$image" bash scripts/check.sh --local
    # The cgroup kernel lane needs a privileged container with the host's
    # cgroup namespace; see scripts/check-cgroup.sh.
    exec "$here/scripts/check-cgroup.sh"
fi

cd "$here"
export GOFLAGS=-buildvcs=false

echo "== gofmt"
unformatted=$(gofmt -l .)
if [[ -n "$unformatted" ]]; then
    echo "gofmt: these files need formatting:" >&2
    echo "$unformatted" >&2
    exit 1
fi

echo "== go vet"
go vet ./...

echo "== go test -race"
go test -race -count=1 ./...

echo "== static build, amd64 and arm64"
out=$(mktemp -d)
for arch in amd64 arm64; do
    CGO_ENABLED=0 GOOS=linux GOARCH=$arch go build -o "$out/$arch/" ./cmd/...
done
rm -rf "$out"
echo "check: PASS"
