#!/usr/bin/env bash
# eks-nri.sh's own logic against a fake aws CLI (fake-aws-nri.py): the cost
# gate, the minute cap, EKS_CHECK_ONLY, the EKS_DELETE stamp check and the
# teardown/verify pass. No cloud, no docker, no cluster. The full lab run is
# not exercised here (it needs AWS, docker buildx and two nodes); only the
# paths that must refuse or must not create anything.
#
#   md-dpdk/benches/cloud/aws/test/test-eks-nri.sh
#
# Exit 0: every check passed.
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
aws_dir=$(cd "$here/.." && pwd)
script=$aws_dir/eks-nri.sh
fake=$here/fake-aws-nri.py

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
bin=$tmp/bin
mkdir -p "$bin" "$tmp/repo"

pass=0
fail=0

ok() { echo "PASS  $*"; pass=$((pass + 1)); }
bad() { echo "FAIL  $*"; fail=$((fail + 1)); }

# The shell's python3 may be a venv that no longer exists; put a working one
# first on PATH so eks-nri.sh's price math runs.
for p in /usr/bin/python3 /usr/local/bin/python3 /opt/homebrew/bin/python3; do
    [ -x "$p" ] && "$p" -c 'pass' >/dev/null 2>&1 && { ln -sf "$p" "$bin/python3"; break; }
done
[ -e "$bin/python3" ] || { echo "no working python3 on this host" >&2; exit 1; }
ln -sf "$fake" "$bin/aws"
chmod +x "$fake"

# kubectl and docker must be on PATH (eks-nri.sh checks); these paths must
# not call them, and if one is called the stub fails loudly.
cat >"$bin/kubectl" <<'EOF'
#!/bin/sh
echo "fake kubectl: not expected in these tests: $*" >&2
exit 1
EOF
cat >"$bin/docker" <<'EOF'
#!/bin/sh
case "$1" in
logout | info) exit 0 ;;
buildx) exit 0 ;;
esac
echo "fake docker: not expected in these tests: $*" >&2
exit 1
EOF
chmod +x "$bin/kubectl" "$bin/docker"

export PATH="$bin:$PATH"
export AWS_PAGER=""

# Run eks-nri.sh once. The rest of the arguments are NAME=VALUE environment
# settings for it (not script arguments). Sets: rc, err, calls, state.
run_lab() {
    local name=$1
    shift
    rc=0 err=$tmp/err.$name calls=$tmp/calls.$name state=$tmp/state.$name
    : >"$calls"
    [ -f "$state" ] || echo '{}' >"$state"
    env AWS_CALLS=$calls FAKE_AWS_STATE=$state \
        EKS_REPO=$tmp/repo EKS_REGION=us-east-1 \
        "$@" bash "$script" >"$tmp/out.$name" 2>"$err" || rc=$?
}

state_seed() { # <name> <json>
    echo "$2" >"$tmp/state.$1"
}

calls_have() { grep -Eq "$2" "$tmp/calls.$1"; }
err_has() { grep -q "$2" "$tmp/err.$1"; }

no_creates() { # <name>
    if calls_have "$1" 'create-|run-instances|allocate-address|import-key-pair'; then
        bad "$1: created something (see $tmp/calls.$1)"
        sed 's/^/      /' "$tmp/calls.$1"
    else
        ok "$1: created nothing"
    fi
}

# ---- 1. the minute cap refuses before any call -------------------------------
run_lab minutes EKS_MAX_MINUTES=181
if ((rc == 2)) && err_has minutes 'cap is 180 minutes'; then
    ok "EKS_MAX_MINUTES=181 refused (exit 2, names the 180-minute cap)"
else
    bad "EKS_MAX_MINUTES=181: rc=$rc, expected 2 and the cap message"
    sed 's/^/      /' "$tmp/err.minutes"
fi
if [ -s "$tmp/calls.minutes" ]; then
    bad "EKS_MAX_MINUTES=181: called the aws CLI before refusing"
    sed 's/^/      /' "$tmp/calls.minutes"
else
    ok "EKS_MAX_MINUTES=181: no aws call at all"
fi

# ---- 2. spot above the gate refuses -----------------------------------------
run_lab spot_high FAKE_SPOT_PRICES="0.12" EKS_CHECK_ONLY=1
if ((rc == 1)) && err_has spot_high '\$0\.30/h gate'; then
    ok "spot \$0.12/h refused at the \$0.30/h gate (2*0.12+0.12=0.36)"
else
    bad "spot \$0.12/h: rc=$rc, expected 1 and the \$0.30/h gate message"
    sed 's/^/      /' "$tmp/err.spot_high"
fi
no_creates spot_high

# ---- 3. no spot history: the on-demand floor is also gated -------------------
run_lab od_only FAKE_NO_SPOT=1
if ((rc == 1)) && err_has od_only '\$0\.30/h gate'; then
    ok "no spot history, on-demand 0.1361: refused (2*0.1361+0.12=0.392)"
else
    bad "no spot history: rc=$rc, expected 1 and the \$0.30/h gate message"
    sed 's/^/      /' "$tmp/err.od_only"
fi
no_creates od_only

# ---- 4. a good estimate is printed before anything, and CHECK_ONLY stops ----
run_lab check_only FAKE_SPOT_PRICES="0.0456 0.0312" EKS_CHECK_ONLY=1
if ((rc == 0)); then
    ok "EKS_CHECK_ONLY=1 with spot \$0.0456/h exits 0"
else
    bad "EKS_CHECK_ONLY=1: rc=$rc, expected 0"
    sed 's/^/      /' "$tmp/err.check_only"
fi
if err_has check_only 'cost estimate BEFORE anything is created' &&
    err_has check_only 'estimated \$0\.211/h' &&
    err_has check_only 'EKS_CHECK_ONLY=1: nothing created'; then
    ok "estimate printed before any create, at \$0.211/h (2*0.0456+0.12)"
else
    bad "EKS_CHECK_ONLY=1: the estimate line is missing or wrong"
    sed 's/^/      /' "$tmp/err.check_only"
fi
no_creates check_only

# ---- 5. EKS_DELETE stamps ---------------------------------------------------
for stamp in garbage eks-20260101-000000 eks-nri-20260101 eks-nri-20260101-0000; do
    run_lab "stamp_${stamp//[^a-zA-Z0-9]/_}" EKS_DELETE="$stamp"
    if ((rc == 2)); then
        ok "EKS_DELETE=$stamp refused (exit 2)"
    else
        bad "EKS_DELETE=$stamp: rc=$rc, expected 2"
        sed 's/^/      /' "$tmp/err.stamp_${stamp//[^a-zA-Z0-9]/_}"
    fi
done

# ---- 6. EKS_DELETE on an empty account: teardown verifies clean -------------
run_lab delete_empty EKS_DELETE=eks-nri-20260101-000000
if ((rc == 0)) && err_has delete_empty 'verified: nothing of run eks-nri-20260101-000000'; then
    ok "EKS_DELETE of an absent run verifies clean (exit 0)"
else
    bad "EKS_DELETE empty: rc=$rc, expected 0 and a clean verify"
    sed 's/^/      /' "$tmp/err.delete_empty"
fi
no_creates delete_empty

# ---- 7. EKS_DELETE deletes a leftover cluster, then verifies ----------------
state_seed delete_cluster '{"clusters": {"ergon-lab-eks-nri-20260101-000000": "ACTIVE"}}'
run_lab delete_cluster EKS_DELETE=eks-nri-20260101-000000
if calls_have delete_cluster 'delete-cluster' && ((rc == 0)) &&
    err_has delete_cluster 'verified: nothing'; then
    ok "EKS_DELETE removes a leftover cluster and verifies clean"
else
    bad "EKS_DELETE with a cluster: rc=$rc (want 0, delete-cluster called, clean verify)"
    sed 's/^/      /' "$tmp/err.delete_cluster"
    sed 's/^/      /' "$tmp/calls.delete_cluster"
fi

# ---- 8. a deletion AWS refuses is reported LEFT -----------------------------
state_seed sticky '{"roles": ["ergon-lab-eks-nri-20260101-000000-cluster"]}'
run_lab sticky FAKE_STICKY_ROLE=1 EKS_DELETE=eks-nri-20260101-000000
if ((rc == 1)) && err_has sticky 'LEFT IAM role' && err_has sticky 'WARNING: still present'; then
    ok "a role that will not delete is reported LEFT (exit 1)"
else
    bad "sticky role: rc=$rc, expected 1 and a LEFT report"
    sed 's/^/      /' "$tmp/err.sticky"
fi

# ---- 9. a non-Graviton type is refused --------------------------------------
run_lab x86 FAKE_ARCH=x86_64 EKS_CHECK_ONLY=1
if ((rc == 1)) && err_has x86 'Graviton'; then
    ok "an x86_64 instance type is refused"
else
    bad "x86 type: rc=$rc, expected 1 and a Graviton message"
    sed 's/^/      /' "$tmp/err.x86"
fi
no_creates x86

# ---- 10. too few vCPUs is refused -------------------------------------------
run_lab small FAKE_VCPUS=2 EKS_CHECK_ONLY=1
if ((rc == 1)) && err_has small 'vCPU'; then
    ok "a 2-vCPU instance type is refused (the node needs 4)"
else
    bad "2 vCPU: rc=$rc, expected 1"
    sed 's/^/      /' "$tmp/err.small"
fi

# ---- 11. the plan's static contract (no cloud, no fake needed) ---------------
# SCHED_FIFO is not built (inconclusive in the one AWS A/B); the hugepage mount
# is the plugin's sized hugetlbfs, never an emptyDir medium (no size=); the
# node never gets a node-wide cpuCFSQuota:false; the cost gate is $0.30/h and
# 180 minutes; on-demand fallback is off.
contract() { # <name> <ok-if-this-regex-is-absent> <why>
    local name=$1 re=$2 why=$3
    if grep -Eq "$re" "$script" "$aws_dir/eks-nri-userdata.sh" 2>/dev/null; then
        bad "$name: $why"
    else
        ok "$name: $why"
    fi
}
contract no-fifo '--fifo|sched_setscheduler|SCHED_FIFO mode' "no SCHED_FIFO path (plan: inconclusive A/B)"
contract no-emptydir-hugepage 'medium:[[:space:]]*HugePages' "no emptyDir hugepage volume (Aeron needs a sized mount)"
contract no-cfs-quota-false 'cpuCFSQuota:[[:space:]]*false|cpuCFSQuota=false' "no node-wide cpuCFSQuota:false"
if grep -q 'OnDemandTargetCapacity=0' "$script"; then
    ok "spot-only: OnDemandTargetCapacity=0 (never silently on-demand)"
else
    bad "spot-only: OnDemandTargetCapacity=0 is missing"
fi
if grep -q '0\.30/h gate' "$script" && grep -q 'cap is 180 minutes' "$script"; then
    ok "cost gate text: \$0.30/h and 180 minutes"
else
    bad "cost gate text missing"
fi
for ann in 'hugepages.ergon.io' 'cpu-partition.ergon.io' 'seccomp.ergon.io' 'ptp.ergon.io'; do
    if grep -q "$ann" "$script"; then
        ok "plan annotation $ann is in the mode manifests"
    else
        bad "plan annotation $ann is missing"
    fi
done

# ---- 12. the admission contract --------------------------------------------
# The validating policy matches a POD label, the same one the shipped policy
# and the bench manifests use. make_pod sets it on every mode but M5 (M5 is
# the counterfactual and must keep its livenessProbe). The policy check is a
# gate: a lab that cannot show extension 4 refusing a probe must not run.
if grep -q 'ergon.io/latency: "hot"' "$script" &&
    grep -q 'mode != M5' "$script"; then
    ok "admission label: make_pod sets ergon.io/latency: \"hot\" on every mode but M5"
else
    bad "admission label: make_pod does not set ergon.io/latency: \"hot\" except on M5"
fi
if grep -q "object.metadata.labels\['ergon.io/latency'\] == 'hot'" "$script" &&
    grep -q "'ergon.io/latency' in object.metadata.labels" "$script"; then
    ok "admission match: the policy matches the pod label == \"hot\" (not any value)"
else
    bad "admission match: the policy does not match the pod label == \"hot\""
fi
if grep -v '^\s*#' "$script" | grep -q '??'; then
    bad "admission match: the CEL uses ?? (not valid CEL; the policy would not compile)"
else
    ok "admission match: the CEL has no ?? operator"
fi
if grep -q 'admission_check || die' "$script"; then
    ok "admission gate: a policy that is not in force fails the run"
else
    bad "admission gate: admission_check is not a gate (a wasted run would pass)"
fi

# ---- 13. the lab proves what it measures ------------------------------------
# Source-level guarantees for the defects a live run would only show as
# numbers that mean nothing: the plugin waited for in the wrong namespace,
# a config that silently disables two extensions, roles that can land on
# either node, a measured task outside its isolated partition, and a CPU
# number that does not come from the grant.
if grep -q 'ergon-shared-cpus ergon-system' "$script" &&
    grep -q 'ergon-lab-probe default' "$script" &&
    grep -q 'rollout status "ds/$ds" -n "$ns"' "$script"; then
    ok "ds namespace: each DaemonSet is waited for in the namespace it is deployed in"
else
    bad "ds namespace: the rollout wait does not pair each DaemonSet with its own namespace"
fi
if grep -q 'devRoot: /host/dev' "$script" &&
    grep -q 'ioUring: allow' "$script"; then
    ok "plugin config: devRoot and seccomp.ioUring are set (PHC discovery and M3/M6)"
else
    bad "plugin config: devRoot and/or seccomp.ioUring are missing from the generated ConfigMap"
fi
if grep -q 'kubernetes.io/hostname' "$script" &&
    grep -q "role \$which is on" "$script"; then
    ok "placement: each role is bound to its named node and a mismatch fails the run"
else
    bad "placement: roles are not bound to the nodes whose IPs become the endpoints"
fi
if grep -q 'cgroup.procs' "$script" &&
    grep -q 'cgroup.threads' "$script" &&
    grep -qF 'pid $app is not in' "$script"; then
    ok "partition: the measured task is moved into the isolated child and proved there"
else
    bad "partition: the measured task is not moved into the isolated child, or not proved"
fi
if grep -qF 'ERGON_ISOLATED_CPUS=\$iso names no CPU' "$script" &&
    grep -qF 'cpu \$cpu_client is not in ERGON_ISOLATED_CPUS' "$script"; then
    ok "cpus: the measured CPU comes from the grant, and is checked against it"
else
    bad "cpus: the rep does not read ERGON_ISOLATED_CPUS (a literal CPU number can leave the partition)"
fi
if grep -q 'if ! functional_check' "$script" &&
    grep -q 'not measured (a grant is missing' "$script"; then
    ok "grant gate: a mode whose grant cannot be read back is not measured"
else
    bad "grant gate: functional_check does not gate the measurement"
fi
if grep -q 'export RTT_HISTOGRAM=' "$script" &&
    ! grep -Eq 'LOAD_SPIKES=[^ ]+ mkdir' "$script"; then
    ok "env: the client's environment is exported, not prefixed to one command"
else
    bad "env: the client's RTT_HISTOGRAM/LOAD_SPIKES are not exported (they would not reach rtt)"
fi

# ---- 14. a control mode does not require the partition it never asked for ----
# Only M2/M6 carry cpu-partition.ergon.io, so only they have an isolated
# child. rep_cmd's in-pod script used to demand one for every mode, so every
# control rep exited before it measured anything and the lab died at M0.
# Generate the two scripts and read what they ask for.
rep_fn=$tmp/rep_cmd.sh
sed -n '/^rep_cmd()/,/^}/p' "$script" >"$rep_fn"
gen_rep() { # <mode> -> the in-pod script on stdout
    bash -c 'source "$1"; rtt_messages=1000 rtt_warmup=10; rep_cmd "$2" a 10.0.0.1:1 10.0.0.2:1' \
        _ "$rep_fn" "$1"
}
gen_rep M0 >"$tmp/rep-m0.sh"
gen_rep M2 >"$tmp/rep-m2.sh"
if grep -q 'cgroup.procs' "$tmp/rep-m0.sh"; then
    bad "rep_cmd M0: a control mode writes cgroup.procs (it was never granted a partition)"
else
    ok "rep_cmd M0: a control mode stays in the container cgroup"
fi
if grep -q 'cgroup.procs' "$tmp/rep-m2.sh" &&
    grep -q 'cgroup.threads' "$tmp/rep-m2.sh" &&
    grep -q 'partition isolated' "$tmp/rep-m2.sh"; then
    ok "rep_cmd M2: the partition mode still moves in and proves it"
else
    bad "rep_cmd M2: the partition mode lost its move or its proof"
fi
if grep -q 'no partition (control mode)' "$tmp/rep-m0.sh" &&
    ! grep -q 'ERGON_ISOLATED_CGROUP' "$tmp/rep-m0.sh"; then
    ok "rep_cmd: a control mode's script names its mode and never looks for a child"
else
    bad "rep_cmd: a control mode's script still looks for an isolated child"
fi

echo
echo "passed $pass, failed $fail"
if ((fail)); then
    echo "workdir (kept only on failure): $tmp"
    trap - EXIT
    exit 1
fi
exit 0
