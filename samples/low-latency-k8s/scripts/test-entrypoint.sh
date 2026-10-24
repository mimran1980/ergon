#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# docker/driver-entrypoint.sh against fixtures: the good plan it prints, and
# each fail-closed refusal (ERGON_* unset, one isolated CPU, SMT siblings,
# a process not on the shared CPUs, a 64 KiB page size, a conductor
# affinity). No root, no Docker, no driver.
#
#   samples/low-latency-k8s/scripts/test-entrypoint.sh
set -euo pipefail

HERE=$(cd "$(dirname "$0")/.." && pwd)
EP="$HERE/docker/driver-entrypoint.sh"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
pass=0
fail=0

# cpu N's siblings
sys() {
  local cpu
  for cpu in "$@"; do
    mkdir -p "$WORK/sys/cpu${cpu%%:*}/topology"
    echo "${cpu#*:}" >"$WORK/sys/cpu${cpu%%:*}/topology/thread_siblings_list"
  done
}
status() { printf 'Name:\tbash\nCpus_allowed_list:\t%s\n' "$1" >"$WORK/status"; }

# run <name> <want: ok|refuse> <expected text> [VAR=value...]
run() {
  local name=$1 want=$2 text=$3 out rc
  shift 3
  set +e
  out=$(env -i PATH="$PATH" HOSTNAME=t LL_PROC_STATUS="$WORK/status" LL_SYS_CPU="$WORK/sys" LL_PAGESIZE=4096 \
    RUSTERON_ENTRYPOINT_PRINT_ONLY=1 "$@" bash "$EP" media_driver 2>&1)
  rc=$?
  set -e
  if { [[ $want == ok && $rc == 0 ]] || [[ $want == refuse && $rc != 0 ]]; } && [[ "$out" == *"$text"* ]]; then
    pass=$((pass + 1))
    echo "PASS $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name (rc=$rc, wanted $want with \"$text\"):"
    echo "$out" | sed 's/^/    /'
  fi
}

GOOD=(ERGON_ISOLATED_CPUS=2,3 ERGON_SHARED_CPUS=1 AERON_SENDER_CPU_AFFINITY=2 AERON_RECEIVER_CPU_AFFINITY=3 RUSTERON_DPDK_HOUSEKEEPING_CPU=1)
sys 2:2 3:3
status 1
run good ok "AERON_THREADING_MODE=DEDICATED" "${GOOD[@]}"
run eal_lcore ok "RUSTERON_DPDK_EAL_ARGS=-l 1 --no-huge -m 512 --no-pci" "${GOOD[@]}" "LL_EAL_EXTRA=--no-huge -m 512 --no-pci"
run keeps_a_set_value ok "AERON_MTU_LENGTH=8960" "${GOOD[@]}" AERON_MTU_LENGTH=8960
run no_conductor_affinity ok "RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena" "${GOOD[@]}"
run ergon_unset refuse "ERGON_ISOLATED_CPUS is unset" ERGON_SHARED_CPUS=1
run shared_unset refuse "ERGON_SHARED_CPUS is unset" ERGON_ISOLATED_CPUS=2,3
run one_isolated refuse "need 2 isolated CPUs" ERGON_ISOLATED_CPUS=2 ERGON_SHARED_CPUS=1 AERON_SENDER_CPU_AFFINITY=2 AERON_RECEIVER_CPU_AFFINITY=2
run unresolved_spec refuse "run under ergon-cpus" ERGON_ISOLATED_CPUS=2,3 ERGON_SHARED_CPUS=1 AERON_SENDER_CPU_AFFINITY=isolated:0 AERON_RECEIVER_CPU_AFFINITY=3
run not_isolated refuse "CPU 1 is not in ERGON_ISOLATED_CPUS" ERGON_ISOLATED_CPUS=2,3 ERGON_SHARED_CPUS=1 AERON_SENDER_CPU_AFFINITY=1 AERON_RECEIVER_CPU_AFFINITY=3
sys 2:2,3 3:2,3
run smt_siblings refuse "share a core" "${GOOD[@]}"
sys 2:2 3:3
status 0-3
run not_on_shared refuse "not the shared CPUs" "${GOOD[@]}"
status 1
run page_64k refuse "base page size 65536" "${GOOD[@]}" LL_PAGESIZE=65536
run conductor_pinned refuse "AERON_CONDUCTOR_CPU_AFFINITY is set" "${GOOD[@]}" AERON_CONDUCTOR_CPU_AFFINITY=2
run cpuset_affinity refuse "AERON_DRIVER_CPUSET_AFFINITY must stay false" "${GOOD[@]}" AERON_DRIVER_CPUSET_AFFINITY=true

echo "entrypoint fixtures: $pass passed, $fail failed"
[[ $fail == 0 ]]
