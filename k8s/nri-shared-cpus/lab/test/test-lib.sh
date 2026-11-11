#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# lib.sh's functions against fixtures. An external AWS lab sources lib.sh, so
# this pins the shape of what it reads: "path|value|why" lines, the
# fair-server lines per exclusive CPU only, and the housekeeping masks.
set -uo pipefail

lab=$(cd "$(dirname "$0")/.." && pwd)
fail=0
check() {
    local name=$1
    shift
    if "$@" >/dev/null 2>&1; then echo "ok:   $name"; else echo "FAIL: $name"; fail=1; fi
}

# shellcheck source=../lib.sh
. "$lab/lib.sh"

qs=$(lab_quiet_settings 0-2)
qx=$(lab_quiet_settings 0-2 3-4)
check "cpus: ranges and lists" test "$(lab_cpus 1,3-5 | tr '\n' ' ')" = "1 3 4 5 "
check "cpumask: 0-2 is 7" test "$(lab_cpumask_hex 0-2)" = 7
check "cpumask: 0,3-4 is 19" test "$(lab_cpumask_hex 0,3-4)" = 19
check "quiet: watchdog on the housekeeping CPUs" grep -qF '/proc/sys/kernel/watchdog_cpumask|0-2|' <<<"$qs"
check "quiet: writeback work on them" grep -qF '/sys/bus/workqueue/devices/writeback/cpumask|7|' <<<"$qs"
check "quiet: no THP collapse daemon" grep -qF '/sys/kernel/mm/transparent_hugepage/enabled|never|' <<<"$qs"
check "quiet: no fair-server line without exclusive CPUs" test "$(grep -c fair_server <<<"$qs")" = 0
check "quiet: fair server off on exclusive CPU 3" grep -qF '/sys/kernel/debug/sched/fair_server/cpu3/runtime|0|' <<<"$qx"
check "quiet: fair server off on exclusive CPU 4" grep -qF '/sys/kernel/debug/sched/fair_server/cpu4/runtime|0|' <<<"$qx"
check "quiet: fair server kept on the housekeeping CPUs" test "$(grep -c 'fair_server/cpu[0-2]/' <<<"$qx")" = 0
check "quiet: every line is path|value|why" test "$(awk -F'|' 'NF >= 3 && $1 ~ /^\/(proc|sys)\// && $3 != ""' <<<"$qx" | wc -l | tr -d ' ')" = "$(wc -l <<<"$qx" | tr -d ' ')"

exit "$fail"
