#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The admission checks in e2e.sh must not treat a connection failure as
# success, or an unrelated validation error as the intended rejection.
# Sources the helpers and stops before any cluster.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
E2E_LIB_ONLY=1
# shellcheck disable=SC1091
source "$HERE/e2e.sh"

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

step_failed=0
admit_apply "good pod admitted" sh -c 'echo "The connection to the server 127.0.0.1 was refused - did you specify the right host or port?" >&2; exit 1'
[[ "$step_failed" -eq 1 ]] || fail "a connection failure counted as success"

step_failed=0
reject_apply "livenessProbe.exec pod rejected" "livenessProbe" \
    sh -c 'echo "The Pod \"x\" is invalid: spec.containers[0].livenessProbe.exec: Required value" >&2; exit 1'
[[ "$step_failed" -eq 1 ]] || fail "a schema error counted as the admission denial"

step_failed=0
reject_apply "livenessProbe.exec pod rejected" "livenessProbe" \
    sh -c 'echo "Error from server (Forbidden): pods \"trader-bad-probe\" is forbidden: ValidatingAdmissionPolicy denied request: livenessProbe must not be exec" >&2; exit 1'
[[ "$step_failed" -eq 0 ]] || fail "a real denial was not accepted"

step_failed=0
admit_apply "good pod admitted" sh -c 'echo "pod/trader created"'
[[ "$step_failed" -eq 0 ]] || fail "a created pod was not accepted"

step_failed=0
admit_apply "good pod admitted" sh -c 'echo "pod/trader configured"'
[[ "$step_failed" -eq 0 ]] || fail "a configured pod was not accepted"

step_failed=0
admit_apply "good pod admitted" sh -c 'echo "pod/trader created"; echo forbidden'
[[ "$step_failed" -eq 1 ]] || fail "forbidden in a zero-exit apply counted as success"

step_failed=0
reject_apply "livenessProbe.exec pod rejected" "livenessProbe" \
    sh -c 'echo "The connection to the server was refused" >&2; exit 1'
[[ "$step_failed" -eq 1 ]] || fail "a connection failure counted as a denial"

printf 'admission assert: PASS\n'
