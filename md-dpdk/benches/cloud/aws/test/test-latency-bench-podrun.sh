#!/usr/bin/env bash
# latency-bench-podrun.sh's placement contract, executed against a fake cgroup
# tree and a fake taskset. The three helpers it sources are the whole of what
# makes a number mean something: where the isolated child is, that the
# measuring process is inside it and in an isolated partition, and that it is
# on the CPU the grant named. A warn-and-continue here is a run that measures
# outside the partition, so every failure path must exit.
#
#   md-dpdk/benches/cloud/aws/test/test-latency-bench-podrun.sh
#
# Exit 0: every check passed.
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
script=$(cd "$here/.." && pwd)/../latency-bench-podrun.sh
[ -f "$script" ] || { echo "no latency-bench-podrun.sh at $script" >&2; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0
ok() { echo "PASS  $*"; pass=$((pass + 1)); }
bad() { echo "FAIL  $*"; fail=$((fail + 1)); }

# A taskset stub: prints the mask the test set in $tmp/mask, or fails when
# $tmp/no-taskset exists. Enough to exercise the affinity proof without Linux.
mkdir -p "$tmp/bin"
cat >"$tmp/bin/taskset" <<EOF
#!/bin/sh
[ -e "$tmp/no-taskset" ] && { echo "taskset: cannot access" >&2; exit 1; }
echo "pid \$1's current affinity list: \$(cat "$tmp/mask")"
EOF
chmod +x "$tmp/bin/taskset"
export PATH="$tmp/bin:$PATH"

# The script's helpers, with none of the run's side effects. The cgroup root
# is read when the script is sourced, so it is set first.
root=$tmp/root
export LATENCY_CGROUP_ROOT=$root/sys/fs/cgroup
export LATENCY_PODRUN_LIB=1
# shellcheck source=/dev/null
. "$script" || { echo "cannot source $script" >&2; exit 1; }
unset LATENCY_PODRUN_LIB
role=test
out=$tmp/out
mkdir -p "$out"

# A cgroup tree whose child is only reachable by name from inside the pod.
host_child=$root/host/kubepods/burstable/podx/ctrx/ergon-isolated
pod_child=$root/sys/fs/cgroup/ergon-isolated
new_child() { # <dir> <partition> [threads...]
    local dir=$1 part=$2
    shift 2
    mkdir -p "$dir"
    : >"$dir/cgroup.procs"
    printf '%s\n' "$part" >"$dir/cpuset.cpus.partition"
    if (($#)); then
        printf '%s\n' "$@" >"$dir/cgroup.threads"
    else
        : >"$dir/cgroup.threads"
    fi
}
new_child "$pod_child" isolated 1

# The helpers report failure by exiting; run each case in a subshell and read
# its message. Sets: rc, msg.
try() {
    local fn=$1
    shift
    msg=$tmp/msg
    rc=0
    ( "$@" ) >"$tmp/stdout" 2>"$msg" || rc=$?
    out_path=$tmp/stdout
}

# ---- resolve_child --------------------------------------------------------
: >"$tmp/mask"
unset ERGON_ISOLATED_CGROUP
try resolve resolve_child
if ((rc != 0)); then ok "resolve_child fails with no ERGON_ISOLATED_CGROUP"; else bad "resolve_child accepted an unset grant"; fi

export ERGON_ISOLATED_CGROUP=$pod_child
try resolve resolve_child
if ((rc == 0)) && [ "$(cat "$tmp/stdout")" = "$pod_child" ]; then
    ok "resolve_child takes the path when it is already a directory"
else
    bad "resolve_child did not return $pod_child (rc=$rc, got $(cat "$tmp/stdout" 2>/dev/null))"
fi

# The node's path does not exist inside the pod's cgroup namespace; the child
# is a directory of the namespace root with the same name.
export ERGON_ISOLATED_CGROUP=/sys/fs/cgroup/kubepods/burstable/podx/ctrx/ergon-isolated
try resolve resolve_child
if ((rc == 0)) && [ "$(cat "$tmp/stdout")" = "$pod_child" ]; then
    ok "resolve_child falls back to the cgroup root by name"
else
    bad "resolve_child did not fall back (rc=$rc, got $(cat "$tmp/stdout" 2>/dev/null))"
fi

export ERGON_ISOLATED_CGROUP=$root/host/nothing/ergon-missing
try resolve resolve_child
if ((rc != 0)); then ok "resolve_child fails when neither path exists"; else bad "resolve_child invented a child"; fi

# ---- cpu_in_mask ----------------------------------------------------------
cpu_in_mask 2 "2" && ok "cpu_in_mask: 2 in 2" || bad "cpu_in_mask: 2 in 2"
cpu_in_mask 2 "2,3" && ok "cpu_in_mask: 2 in 2,3" || bad "cpu_in_mask: 2 in 2,3"
cpu_in_mask 3 "0-3,8-11" && ok "cpu_in_mask: 3 in 0-3,8-11" || bad "cpu_in_mask: 3 in 0-3,8-11"
cpu_in_mask 4 "0-3" && bad "cpu_in_mask: 4 in 0-3" || ok "cpu_in_mask: 4 not in 0-3"
cpu_in_mask 2 "12-20" && bad "cpu_in_mask: 2 in 12-20" || ok "cpu_in_mask: 2 not in 12-20"

# ---- enter_partition ------------------------------------------------------
# No grant, and the mode did not ask for one: a control run, recorded.
unset ERGON_ISOLATED_CGROUP LATENCY_REQUIRE_PARTITION
rm -f "$out/meta.txt"
try enter enter_partition
if ((rc == 0)) && grep -q 'partition=none' "$out/meta.txt" 2>/dev/null; then
    ok "enter_partition allows a mode that asked for no partition"
else
    bad "enter_partition refused a control run (rc=$rc, $(cat "$msg"))"
fi

# The same mode, but it asked for cpu-partition.ergon.io: missing is fatal.
export LATENCY_REQUIRE_PARTITION=1
try enter enter_partition
if ((rc != 0)) && grep -q 'LATENCY_REQUIRE_PARTITION=1 but ERGON_ISOLATED_CGROUP is unset' "$msg"; then
    ok "enter_partition fails closed when a partition was promised and not granted"
else
    bad "enter_partition measured without a promised partition (rc=$rc, $(cat "$msg"))"
fi

# Granted, resolvable by name, isolated: moves in and returns the child.
export ERGON_ISOLATED_CGROUP=/sys/fs/cgroup/ergon-isolated
rm -f "$out/meta.txt"
try enter enter_partition
if ((rc == 0)) && [ "$(cat "$tmp/stdout")" = "$pod_child" ] &&
    grep -q "partition=isolated child=$pod_child" "$out/meta.txt"; then
    ok "enter_partition moves into the child and records an isolated partition"
else
    bad "enter_partition did not land in $pod_child (rc=$rc, $(cat "$msg"), $(cat "$out/meta.txt" 2>/dev/null))"
fi

# The child exists but is not an isolated partition: the numbers would mean
# nothing, so the run must not produce them.
new_child "$pod_child" root
try enter enter_partition
if ((rc != 0)) && grep -q "expected 'isolated'" "$msg"; then
    ok "enter_partition refuses a child that is not an isolated partition"
else
    bad "enter_partition accepted partition=root (rc=$rc, $(cat "$msg"))"
fi
new_child "$pod_child" isolated 1

# The host path is gone and no child sits under the cgroup root either.
export ERGON_ISOLATED_CGROUP=/sys/fs/cgroup/gone/ergon-missing
try enter enter_partition
if ((rc != 0)) && grep -q 'no isolated cgroup under ERGON_ISOLATED_CGROUP' "$msg"; then
    ok "enter_partition fails closed when the child cannot be found"
else
    bad "enter_partition accepted a missing child (rc=$rc, $(cat "$msg"))"
fi

# cgroup.procs not writable: the move would silently not happen.
export ERGON_ISOLATED_CGROUP=$pod_child
chmod a-w "$pod_child/cgroup.procs"
try enter enter_partition
if ((rc != 0)) && grep -q 'cgroup.procs is not writable' "$msg"; then
    ok "enter_partition fails closed when the move cannot be written"
else
    bad "enter_partition continued past an unwritable cgroup.procs (rc=$rc, $(cat "$msg"))"
fi
chmod u+w "$pod_child/cgroup.procs"

# The child is set for the caller as well as printed: prove_placed needs the
# path, and a run that failed to enter must not reach it with an empty one.
export ERGON_ISOLATED_CGROUP=$pod_child
new_child "$pod_child" isolated 1
child=unset
enter_partition >/dev/null
if [[ $child == "$pod_child" ]]; then
    ok "enter_partition sets child for the calling shell"
else
    bad "enter_partition left child=[$child]; the runner would skip the membership proof"
fi

# The runner must call it in the current shell. `child=$(enter_partition)`
# runs it in a subshell, whose `die` exits only that subshell: the run then
# measures outside the partition and `prove_placed` skips its one cgroup
# check, because the check is on `child` being non-empty. Comment lines are
# ignored: the warning above says the same words.
if grep -nE '\$\(enter_partition' "$script" | grep -vE '^[0-9]+:[[:space:]]*#' >/dev/null; then
    bad "the runner captures enter_partition in a command substitution (die is swallowed)"
else
    ok "the runner calls enter_partition in the current shell"
fi

# ---- prove_placed ---------------------------------------------------------
pid=$$
echo "$pid" >"$pod_child/cgroup.threads"
echo 2 >"$tmp/mask"
try place prove_placed "$pid" 2 "$pod_child"
if ((rc == 0)); then ok "prove_placed accepts a task in the child on the granted CPU"; else bad "prove_placed rejected a good placement ($(cat "$msg"))"; fi

echo 99 >"$pod_child/cgroup.threads"
try place prove_placed "$pid" 2 "$pod_child"
if ((rc != 0)) && grep -q "pid $pid is not in" "$msg"; then
    ok "prove_placed fails closed when the task is outside the child"
else
    bad "prove_placed accepted a task outside the child (rc=$rc, $(cat "$msg"))"
fi
echo "$pid" >"$pod_child/cgroup.threads"

echo 5 >"$tmp/mask"
try place prove_placed "$pid" 2 "$pod_child"
if ((rc != 0)) && grep -q "is not on CPU 2" "$msg"; then
    ok "prove_placed fails closed when the task is on the wrong CPU"
else
    bad "prove_placed accepted the wrong CPU (rc=$rc, $(cat "$msg"))"
fi

# A mask printed as a range counts as covering the CPU.
echo 0-3 >"$tmp/mask"
try place prove_placed "$pid" 2 "$pod_child"
if ((rc == 0)); then ok "prove_placed reads a ranged affinity mask"; else bad "prove_placed rejected 0-3 for CPU 2 ($(cat "$msg"))"; fi

# ---- the manifests ask for what they measure ------------------------------
d=$(cd "$here/.." && pwd)/latency-bench.d
for m in partition all; do
    if grep -q 'cpu-partition.ergon.io' "$d/$m.yaml" &&
        grep -q 'LATENCY_REQUIRE_PARTITION' "$d/$m.yaml"; then
        ok "$m.yaml: asks for cpu-partition and requires the grant"
    else
        bad "$m.yaml: does not require the partition it asked for"
    fi
done
for m in baseline hugepages devices seccomp probe; do
    if grep -q 'cpu-partition.ergon.io' "$d/$m.yaml"; then
        bad "$m.yaml: asks for cpu-partition without LATENCY_REQUIRE_PARTITION"
    else
        ok "$m.yaml: no partition asked for (a control mode)"
    fi
done

echo
echo "passed $pass, failed $fail"
if ((fail)); then
    echo "workdir (kept only on failure): $tmp"
    trap - EXIT
    exit 1
fi
exit 0
