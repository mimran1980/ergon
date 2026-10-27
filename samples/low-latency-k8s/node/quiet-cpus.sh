#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Quiets the node's isolated CPUs (/sys/devices/system/cpu/isolated) with
# the two settings the nri-shared-cpus latency lab traced to an isolated
# spin CPU (k8s/nri-shared-cpus/BENCHMARKS.md, run 20261024234841):
#
# | setting | value | why |
# |---|---|---|
# | /sys/kernel/mm/transparent_hugepage/enabled | never | khugepaged calls lru_add_drain_all, which runs a kworker on every CPU with queued pages |
# | /sys/kernel/debug/sched/fair_server/cpu<N>/runtime, each isolated N | 0 | the fair server's 1 s timer (kernel 6.12+) sends a nohz kick and a reschedule IPI each second and keeps the tick on |
#
# The fair server keeps real-time tasks from starving normal ones: 0 is
# safe only where nothing real-time runs on those CPUs. It is in debugfs and
# does not survive a reboot, so ergon-quiet-cpus.service runs this at every
# boot, after debugfs is mounted. A kernel without the fair server (before
# 6.12) passes. Checked offline by tests/manifests.rs.
#
#   quiet-cpus.sh           apply, then check
#   quiet-cpus.sh --check   check only, read-only
#
# LL_SYS_ROOT (default /sys) points it at a fake tree.
set -euo pipefail

sys=${LL_SYS_ROOT:-/sys}
check_only=0
case ${1:-} in
  --check) check_only=1 ;;
  "") ;;
  *) echo "usage: $0 [--check]" >&2; exit 2 ;;
esac

isolated=$(cat "$sys/devices/system/cpu/isolated" 2>/dev/null || true)
if [[ ! $isolated =~ ^[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*$ ]]; then
  echo "FAIL isolated: no isolated CPUs ('$isolated'): boot with isolcpus" >&2
  exit 1
fi
cpus=()
IFS=, read -ra parts <<<"$isolated"
for p in "${parts[@]}"; do
  for ((c = ${p%-*}; c <= ${p#*-}; c++)); do cpus+=("$c"); done
done

thp=$sys/kernel/mm/transparent_hugepage/enabled
sched=$sys/kernel/debug/sched
fair=$sched/fair_server
if ((check_only == 0)); then
  echo never >"$thp"
  if [[ -d $fair ]]; then
    for c in "${cpus[@]}"; do echo 0 >"$fair/cpu$c/runtime"; done
  fi
fi

fail=0
now=$(cat "$thp" 2>/dev/null || echo missing)
if [[ $now == *"[never]"* ]]; then
  echo "PASS thp never"
else
  echo "FAIL thp: $now, want [never]"
  fail=1
fi
if [[ ! -d $sched ]]; then
  echo "FAIL fair_server: no $sched: debugfs not mounted"
  fail=1
elif [[ ! -d $fair ]]; then
  echo "PASS fair_server: this kernel has none (before 6.12)"
else
  for c in "${cpus[@]}"; do
    v=$(cat "$fair/cpu$c/runtime" 2>/dev/null || echo missing)
    if [[ $v == 0 ]]; then
      echo "PASS fair_server cpu$c runtime 0"
    else
      echo "FAIL fair_server cpu$c runtime $v, want 0"
      fail=1
    fi
  done
fi
exit "$fail"
