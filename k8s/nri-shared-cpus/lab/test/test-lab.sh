#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The latency lab without a cloud or a VM: azure-vm.sh against fake az, ssh,
# scp and curl (test/fakes), and lib.sh's and vm.sh's functions against
# fixtures. Checks that:
#   * DRY_RUN=1 prints the plan (layout, boot line, size, price, cost, the
#     backstop's deadline) and every az call, and calls nothing;
#   * an x86 size, a size under 4 vCPUs, a plan longer than LAB_MAX_HOURS
#     and a non-number are refused before anything;
#   * a run creates the group first and tags it, gives the VM its identity
#     on the group alone and the backstop cloud-init (a timer at the
#     deadline deleting the group), tags every resource, sets auto-shutdown,
#     runs the VM steps in order (bootstrap, reboot, tune, cluster, e2e,
#     bench, report), writes lab.env, then deletes the group and checks it
#     gone and nothing tagged with the run left ("verified");
#   * the teardown runs on a failure (no SSH), a failed step (no later step
#     runs) and an interrupt (TERM), and says "not verified" and fails when
#     the group outlives the delete or the check cannot read (expired login);
#   * a group that exists already, or one an earlier run left, is refused
#     and nothing is created or deleted;
#   * the group is deleted when its create fails after reaching ARM, a
#     failed delete is retried, a failed resource listing stops the run,
#     a hangup cannot cut the teardown short, auto-shutdown comes 30 min
#     after the VM's deleting timer, tune waits for a new boot, and a step
#     whose VM stops answering SSH is given up on;
#   * no snapshot directory and no watchdog process outlive a run;
#   * lib.sh: the size's vCPUs, the layout, the boot line, the estimate and
#     cost; the kind config and both pods (parsed as YAML: ruby, jq); the
#     placement check on ps fixtures;
#   * vm.sh: the NIC check on a fake /sys and /proc (an MSI IRQ and VMBus
#     channels on the IRQ CPU pass; a channel elsewhere, an unreadable one,
#     no NIC fail), cpumask, cpu_count.
# Needs bash 4+, ruby and jq.
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
lab=$(cd "$here/.." && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/nri-lab-test.XXXXXX")
trap '[[ -n ${KEEP:-} ]] && echo "kept $work" || rm -rf "$work"' EXIT
fails=0
pass() { echo "ok:   $*"; }
bad() { echo "FAIL: $*"; fails=$((fails + 1)); }
check() { # check <description> <command...>
    local d=$1
    shift
    if "$@"; then pass "$d"; else bad "$d"; fi
}
has() { grep -qF -- "$2" "$1"; }

# run_lab <name> [VAR=value...]: azure-vm.sh with the fakes first on PATH,
# HOME with a key, its own TMPDIR and output; its stderr in $work/<name>/out,
# its exit code in $work/<name>/rc.
run_lab() {
    local name=$1 d=$work/$1
    shift
    mkdir -p "$d/home/.ssh" "$d/tmp" "$d/fake"
    echo "ssh-ed25519 AAAA test" >"$d/home/.ssh/id_ed25519.pub"
    env -i PATH="$here/fakes:$PATH" HOME="$d/home" TMPDIR="$d/tmp" FAKE_DIR="$d/fake" \
        LAB_OUT="$d/out-dir" LAB_STAMP=20261024000000 LAB_SSH_FROM=198.51.100.7 \
        LAB_POLL_SECONDS=0 LAB_SSH_WAIT_SECONDS=1 LAB_REBOOT_SECONDS=0 LAB_DETACH=0 \
        "$@" bash "$lab/azure-vm.sh" >"$d/stdout" 2>"$d/out"
    echo $? >"$d/rc"
}
rc_of() { cat "$work/$1/rc"; }
azlog() { cat "$work/$1/fake/az.log" 2>/dev/null; }
line_of() { grep -nF -- "$2" "$work/$1/fake/az.log" | head -1 | cut -d: -f1; }

echo "== syntax"
for f in "$lab"/*.sh "$here"/*.sh "$here"/fakes/*; do
    check "bash -n ${f#"$lab"/}" bash -n "$f"
done

echo "== dry plan"
mkdir -p "$work/noaz"
printf '#!/bin/sh\necho "az $*" >>"%s/noaz/called"\nexit 99\n' "$work" >"$work/noaz/az"
chmod +x "$work/noaz/az"
d=$work/dry
mkdir -p "$d"
env -i PATH="$work/noaz:$PATH" HOME="$d" DRY_RUN=1 LAB_STAMP=20261024000000 bash "$lab/azure-vm.sh" >"$d/stdout" 2>"$d/out"
check "dry run exits 0" test $? -eq 0
check "dry run calls no az" test ! -e "$work/noaz/called"
for want in "Standard_D8ps_v6 (8 vCPUs, Arm)" "Debian:debian-13:13-arm64:latest" \
    "IRQ CPU 0, shared CPU 1 (e2e, bench) or 1-2 (scale), exclusive (isolated) 3-7, reservedSystemCPUs 0-2" \
    "boot line isolcpus=managed_irq,domain,3-7 nohz_full=3-7 rcu_nocbs=3-7 irqaffinity=0" \
    "bench     5 reps x plugin-on; 60 s spin after 5 s helpers (a child process)" \
    "trace     one more 60 s plugin-on rep under kernel ftrace, filtered to its spin CPU, after 25 s helpers (the tracefs setup done first)" \
    "scale     sharedCpus 1-2: bursts at 100 and 1001 millicpus, idle, contention (4 helper threads a pod, 30 s each), accounting" \
    "node      the quiet settings (LAB_QUIET=1)" \
    "about 75 minutes" "about \$0.330/h" "about \$0.44 for the run" \
    "at most \$1.40 to the backstop" "(LAB_MAX_HOURS=4)" "tagged owner=ergon-lab run=20261024000000" \
    "az group create -n ergon-lab-nri-20261024000000 -l eastus --tags owner=ergon-lab run=20261024000000" \
    "--assign-identity [system] --role Contributor --scope <group id>" "az vm auto-shutdown" \
    "vm.sh bootstrap, a reboot, vm.sh tune, cluster, e2e, scale, bench, report" \
    "az group delete -n ergon-lab-nri-20261024000000 --yes --no-wait" "az resource list --tag run=20261024000000"; do
    check "plan names: $want" has "$d/out" "$want"
done
check "plan: no plugin-off arm by default" bash -c "! grep -q 'plugin-off' '$d/out'"

dryplan() { # dryplan <name> [VAR=value...]: the dry plan's stderr in $work/<name>.plan
    local name=$1
    shift
    env -i PATH="$work/noaz:$PATH" HOME="$d" DRY_RUN=1 LAB_STAMP=20261024000000 "$@" bash "$lab/azure-vm.sh" >/dev/null 2>"$work/$name.plan"
}
dryplan both "LAB_ARMS=plugin-on plugin-off"
check "LAB_ARMS both: two arms" has "$work/both.plan" "bench     5 reps x plugin-on, plugin-off; 60 s spin"
check "LAB_ARMS both: 88 minutes" has "$work/both.plan" "about 88 minutes"
dryplan lean LAB_TRACE=0 LAB_SCALE=0 LAB_HELPERS=thread LAB_QUIET=0
check "no trace, no scale: 58 minutes" has "$work/lean.plan" "about 58 minutes"
check "no trace: said" has "$work/lean.plan" "trace     off (LAB_TRACE=0)"
check "no scale: said" has "$work/lean.plan" "scale     off (LAB_SCALE=0)"
check "helpers as threads: said" has "$work/lean.plan" "after 5 s helpers (threads of the probe)"
check "no quiet settings: said" has "$work/lean.plan" "node      as booted (LAB_QUIET=0)"
check "no scale step in the SSH steps" has "$work/lean.plan" "vm.sh bootstrap, a reboot, vm.sh tune, cluster, e2e, bench, report"
dryplan nobench LAB_BENCH=0
check "LAB_BENCH=0: no bench step, no trace" has "$work/nobench.plan" "vm.sh bootstrap, a reboot, vm.sh tune, cluster, e2e, scale, report"
check "LAB_BENCH=0: bench off" has "$work/nobench.plan" "bench     off (LAB_BENCH=0)"
check "LAB_BENCH=0: 58 minutes" has "$work/nobench.plan" "about 58 minutes"

echo "== refusals before anything"
for c in "LAB_SIZE=Standard_D8s_v6|not an Azure Arm" "LAB_SIZE=Standard_D2ps_v6|the lab needs 7 or more" \
    "LAB_SIZE=Standard_D4ps_v6|the lab needs 7 or more" \
    "LAB_REPS=80|more than LAB_MAX_HOURS" "LAB_SECONDS=abc|LAB_SECONDS is a whole number" \
    "LAB_MAX_HOURS=30|LAB_MAX_HOURS is 1 to 23" "LAB_MAX_HOURS=24|LAB_MAX_HOURS is 1 to 23" "LAB_STRESS=--cpu;rm|stress-ng flags only" \
    "LAB_ARMS=plugin-x|LAB_ARMS: plugin-on and/or plugin-off" "LAB_ARMS=|LAB_ARMS: plugin-on and/or plugin-off" \
    "LAB_HELPERS=fork|LAB_HELPERS: process or thread" "LAB_TRACE=2|LAB_TRACE is 0 or 1" "LAB_SCALE=yes|LAB_SCALE is 0 or 1" \
    "LAB_BENCH=2|LAB_BENCH is 0 or 1" "LAB_QUIET=x|LAB_QUIET is 0 or 1" "LAB_SCALE_SECONDS=5|LAB_SCALE_SECONDS is 10 or more"; do
    var=${c%%|*} want=${c#*|}
    rm -f "$work/noaz/called"
    env -i PATH="$work/noaz:$PATH" HOME="$d" DRY_RUN=1 "$var" bash "$lab/azure-vm.sh" >/dev/null 2>"$d/refuse"
    rc=$?
    check "$var refused" test "$rc" -ne 0
    check "$var names why: $want" has "$d/refuse" "$want"
    check "$var calls no az" test ! -e "$work/noaz/called"
done

echo "== a run, start to end"
run_lab ok LAB_SSH_WAIT_SECONDS=5
check "run exits 0" test "$(rc_of ok)" -eq 0
check "steps in order" test "$(tr '\n' ' ' <"$work/ok/fake/steps.log")" = "bootstrap reboot tune cluster e2e scale bench report "
check "group create before the VM" test "$(line_of ok 'group create')" -lt "$(line_of ok 'vm create')"
check "group created with the run's tags" has "$work/ok/fake/az.log" "az group create -n ergon-lab-nri-20261024000000 -l eastus --tags owner=ergon-lab run=20261024000000"
check "the VM: identity on the group alone" has "$work/ok/fake/az.log" "--assign-identity [system] --role Contributor --scope /subscriptions/00000000-0000-0000-0000-00000000cafe/resourceGroups/ergon-lab-nri-20261024000000 "
check "the VM: Arm size, image, cloud-init, tags" has "$work/ok/fake/az.log" "--size Standard_D8ps_v6 --image Debian:debian-13:13-arm64:latest"
check "the VM carries the backstop cloud-init" has "$work/ok/fake/az.log" "--custom-data $work/ok/out-dir/cloud-init.yaml --tags owner=ergon-lab run=20261024000000"
check "SSH only from this machine" has "$work/ok/fake/az.log" "--source-address-prefixes 198.51.100.7/32"
check "every resource tagged" test "$(grep -c 'az tag update --resource-id /fake/.* --operation merge --tags owner=ergon-lab run=20261024000000' "$work/ok/fake/az.log")" -eq 3
check "auto-shutdown set" has "$work/ok/fake/az.log" "az vm auto-shutdown -g ergon-lab-nri-20261024000000 -n vm --time"
ci=$work/ok/out-dir/cloud-init.yaml
check "backstop: a timer at the deadline" grep -qE 'OnCalendar=20[0-9]{2}-[0-9]{2}-[0-9]{2} [0-9:]{8} UTC' "$ci"
check "backstop: deletes this group" has "$ci" "curl -fsS -X DELETE -H \"Authorization: Bearer \$tok\" 'https://management.azure.com/subscriptions/00000000-0000-0000-0000-00000000cafe/resourceGroups/ergon-lab-nri-20261024000000?api-version=2021-04-01'"
check "backstop: enabled from first boot" has "$ci" "[systemctl, enable, --now, ergon-lab-backstop.timer]"
check "backstop: checked before the steps" has "$work/ok/fake/ssh.log" "sudo /usr/local/sbin/ergon-lab-backstop check"
for want in IRQ_CPU=0 SHARED_CPUS=1 SCALE_SHARED_CPUS=1-2 EXCLUSIVE_CPUS=3-7 RESERVED_CPUS=0-2 REPS=5 SECONDS_PER_RUN=60 \
    'STRESS="--cpu 1 --vm 1 --vm-bytes 128M"' 'ARMS="plugin-on"' TRACE=1 HELPERS=process QUIET=1 SCALE_SECONDS=30 SCALE_THREADS=4; do
    check "lab.env: $want" grep -qxF "$want" "$work/ok/fake/lab.env"
done
check "group deleted" test "$(line_of ok 'group delete -n ergon-lab-nri-20261024000000 --yes --no-wait')" -gt "$(line_of ok 'vm create')"
check "delete waited for" has "$work/ok/fake/az.log" "az group wait -n ergon-lab-nri-20261024000000 --deleted"
check "teardown checks the tag in every group" has "$work/ok/fake/az.log" "az resource list --tag run=20261024000000"
check "teardown says verified" has "$work/ok/out" "verified: group ergon-lab-nri-20261024000000 is gone and no resource is tagged run=20261024000000"
check "resources counted at the end" has "$work/ok/fake/az.log" "az resource list -g ergon-lab-nri-20261024000000 --query length(@) -o tsv"
check "results copied back" test -s "$work/ok/out-dir/results/report.md"
check "checkout packed with the lab" bash -c "tar -tzf '$work/ok/out-dir/src.tgz' | grep -q 'k8s/nri-shared-cpus/lab/vm.sh' && tar -tzf '$work/ok/out-dir/src.tgz' | grep -q 'k8s/nri-latency-pods/admission/validating-admission-policy.yaml'"
check "no snapshot left" test -z "$(ls "$work/ok/tmp")"

echo "== teardown on failure: no SSH"
run_lab nossh FAKE_SSH=down
check "fails" test "$(rc_of nossh)" -ne 0
check "says why" has "$work/nossh/out" "no SSH to 203.0.113.10"
check "group deleted" has "$work/nossh/fake/az.log" "az group delete -n ergon-lab-nri-20261024000000 --yes --no-wait"
check "verified" has "$work/nossh/out" "verified: group"

echo "== sshd answers before cloud-init has enabled the backstop timer"
run_lab cloudinit FAKE_CLOUD_INIT_LAG=2 LAB_SSH_WAIT_SECONDS=5
check "waits for cloud-init, then runs" test "$(rc_of cloudinit)" -eq 0
check "checks the timer after cloud-init" test "$(grep -n 'cloud-init status --wait' "$work/cloudinit/fake/ssh.log" | tail -1 | cut -d: -f1)" -lt "$(grep -n 'is-enabled ergon-lab-backstop' "$work/cloudinit/fake/ssh.log" | head -1 | cut -d: -f1)"

echo "== teardown on a failed step"
run_lab stepfail FAKE_FAIL_STEP=e2e LAB_SSH_WAIT_SECONDS=5
check "fails" test "$(rc_of stepfail)" -ne 0
check "stops at the failed step" test "$(tr '\n' ' ' <"$work/stepfail/fake/steps.log")" = "bootstrap reboot tune cluster e2e "
check "names it" has "$work/stepfail/out" "step e2e failed (exit 1)"
check "group deleted and verified" has "$work/stepfail/out" "verified: group"

echo "== a failed scale step: bench and report still run"
run_lab scalefail FAKE_FAIL_STEP=scale LAB_SSH_WAIT_SECONDS=5
check "fails" test "$(rc_of scalefail)" -ne 0
check "bench and report run after it" test "$(tr '\n' ' ' <"$work/scalefail/fake/steps.log")" = "bootstrap reboot tune cluster e2e scale bench report "
check "names it" has "$work/scalefail/out" "scale had failures (results/scale.md)"
check "results copied back" test -s "$work/scalefail/out-dir/results/report.md"
check "group deleted and verified" has "$work/scalefail/out" "verified: group"

echo "== LAB_SCALE=0 and LAB_BENCH=0 drop their steps"
run_lab noscale LAB_SCALE=0 LAB_SSH_WAIT_SECONDS=5
check "no scale step" test "$(tr '\n' ' ' <"$work/noscale/fake/steps.log")" = "bootstrap reboot tune cluster e2e bench report "
run_lab nobench LAB_BENCH=0 LAB_SSH_WAIT_SECONDS=5
check "no bench step" test "$(tr '\n' ' ' <"$work/nobench/fake/steps.log")" = "bootstrap reboot tune cluster e2e scale report "
check "lab.env: TRACE=0 without the bench" grep -qxF TRACE=0 "$work/nobench/fake/lab.env"

echo "== teardown on interrupt"
d=$work/term
mkdir -p "$d/home/.ssh" "$d/tmp" "$d/fake"
echo "ssh-ed25519 AAAA test" >"$d/home/.ssh/id_ed25519.pub"
env -i PATH="$here/fakes:$PATH" HOME="$d/home" TMPDIR="$d/tmp" FAKE_DIR="$d/fake" FAKE_SSH=hang \
    LAB_OUT="$d/out-dir" LAB_STAMP=20261024000000 LAB_SSH_FROM=198.51.100.7 LAB_POLL_SECONDS=0 \
    LAB_SSH_WAIT_SECONDS=600 LAB_DETACH=0 bash "$lab/azure-vm.sh" >"$d/stdout" 2>"$d/out" &
pid=$!
for _ in $(seq 1 100); do [[ -s $d/fake/ssh.log ]] && break; sleep 0.1; done
kill -TERM "$pid"
wait "$pid"
rc=$?
check "TERM exits 130" test "$rc" -eq 130
check "group deleted" has "$d/fake/az.log" "az group delete -n ergon-lab-nri-20261024000000 --yes --no-wait"
check "verified" has "$d/out" "verified: group"

echo "== the group outlives the delete"
run_lab still FAKE_SSH=down FAKE_STILL_EXISTS=1
check "fails" test "$(rc_of still)" -ne 0
check "says it may still bill, with the commands" has "$work/still/out" "WARNING: not verified, may still bill: group exists=true"
check "no verified line" bash -c "! grep -q 'verified:' '$work/still/out'"

echo "== the teardown's listing fails (an expired login)"
run_lab listfail FAKE_SSH=down FAKE_LIST_FAIL=1
check "fails" test "$(rc_of listfail)" -ne 0
check "says unread, never verified" has "$work/listfail/out" "tagged run=20261024000000: unread"
check "no verified line" bash -c "! grep -q 'verified:' '$work/listfail/out'"

echo "== refusals at the cloud"
run_lab exists FAKE_EXISTS=1
check "an existing group is refused" has "$work/exists/out" "resource group ergon-lab-nri-20261024000000 exists"
check "nothing created" bash -c "! grep -q 'group create' '$work/exists/fake/az.log'"
check "nothing deleted" bash -c "! grep -q 'group delete' '$work/exists/fake/az.log'"
run_lab leftover FAKE_LEFTOVER=ergon-lab-nri-20261023000000
check "an earlier run's group is refused" has "$work/leftover/out" "groups tagged owner=ergon-lab exist (an earlier run's?): ergon-lab-nri-20261023000000"
check "nothing created" bash -c "! grep -q 'group create' '$work/leftover/fake/az.log'"

echo "== the group create fails after reaching ARM"
run_lab createfail FAKE_CREATE_FAIL=1
check "fails" test "$(rc_of createfail)" -ne 0
check "says why" has "$work/createfail/out" "group create failed"
check "the group is deleted all the same" has "$work/createfail/fake/az.log" "az group delete -n ergon-lab-nri-20261024000000 --yes --no-wait"
check "verified" has "$work/createfail/out" "verified: group"

echo "== auto-shutdown only after the VM's deleting timer"
# a deallocated VM cannot run its timer: the shutdown comes 30 min later
t=$(sed -n 's/^ *OnCalendar=[0-9-]* \([0-9][0-9]\):\([0-9][0-9]\):.*/\1\2/p' "$work/ok/out-dir/cloud-init.yaml")
s=$(sed -n 's/.*az vm auto-shutdown .* --time \([0-9]\{4\}\).*/\1/p' "$work/ok/fake/az.log")
want=$(printf '%02d%02d' $(((10#${t:0:2} * 60 + 10#${t:2:2} + 30) / 60 % 24)) $(((10#${t:0:2} * 60 + 10#${t:2:2} + 30) % 60)))
check "auto-shutdown $s is the timer $t plus 30 min ($want)" test -n "$t" -a "$s" = "$want"

echo "== the first delete fails"
run_lab delonce FAKE_SSH=down FAKE_DELETE_FAIL_ONCE=1
check "the delete is retried" test "$(grep -c 'az group delete -n ergon-lab-nri-20261024000000' "$work/delonce/fake/az.log")" -ge 2
check "verified" has "$work/delonce/out" "verified: group"

echo "== listing the group's resources to tag them fails"
run_lab rglistfail FAKE_RG_LIST_FAIL=1
check "fails" test "$(rc_of rglistfail)" -ne 0
check "says why" has "$work/rglistfail/out" "cannot list the resources in"
check "no step runs" test ! -e "$work/rglistfail/fake/steps.log"
check "verified" has "$work/rglistfail/out" "verified: group"

echo "== the old boot still answers after the reboot"
run_lab lag FAKE_REBOOT_LAG=3 LAB_SSH_WAIT_SECONDS=5
check "run exits 0" test "$(rc_of lag)" -eq 0
check "tune runs in the new boot" grep -qx 'tune@boot-2' "$work/lag/fake/steps-boot.log"

echo "== SSH lost mid-step"
d=$work/lost
mkdir -p "$d/home/.ssh" "$d/tmp" "$d/fake"
echo "ssh-ed25519 AAAA test" >"$d/home/.ssh/id_ed25519.pub"
env -i PATH="$here/fakes:$PATH" HOME="$d/home" TMPDIR="$d/tmp" FAKE_DIR="$d/fake" FAKE_SSH_DOWN_AT=cluster \
    LAB_OUT="$d/out-dir" LAB_STAMP=20261024000000 LAB_SSH_FROM=198.51.100.7 LAB_POLL_SECONDS=0 \
    LAB_SSH_WAIT_SECONDS=2 LAB_REBOOT_SECONDS=0 LAB_DETACH=0 bash "$lab/azure-vm.sh" >"$d/stdout" 2>"$d/out" &
pid=$!
for _ in $(seq 1 300); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
if kill -0 "$pid" 2>/dev/null; then
    kill -TERM "$pid"
    bad "gives up within 30 s of losing SSH (still polling)"
else
    pass "gives up within 30 s of losing SSH"
fi
wait "$pid"
check "says why" has "$d/out" "step cluster: no SSH to 203.0.113.10 for 2 s"
check "verified" has "$d/out" "verified: group"

echo "== a hangup during the teardown"
d=$work/hup
mkdir -p "$d/home/.ssh" "$d/tmp" "$d/fake"
echo "ssh-ed25519 AAAA test" >"$d/home/.ssh/id_ed25519.pub"
env -i PATH="$here/fakes:$PATH" HOME="$d/home" TMPDIR="$d/tmp" FAKE_DIR="$d/fake" FAKE_SSH=down FAKE_WAIT_SECONDS=2 \
    LAB_OUT="$d/out-dir" LAB_STAMP=20261024000000 LAB_SSH_FROM=198.51.100.7 LAB_POLL_SECONDS=0 \
    LAB_SSH_WAIT_SECONDS=1 LAB_DETACH=0 bash "$lab/azure-vm.sh" >"$d/stdout" 2>"$d/out" &
pid=$!
for _ in $(seq 1 100); do grep -q 'group wait' "$d/fake/az.log" 2>/dev/null && break; sleep 0.1; done
kill -HUP "$pid"
wait "$pid"
check "the teardown finishes: verified" has "$d/out" "verified: group"

echo "== nothing outlives the runs"
# A watchdog exits within its poll after its run ends; a slow runner needs
# more than a second, and one still there after 20 s is a leak. The [e]
# keeps pgrep -f from matching the command line that runs it (Linux).
for _ in $(seq 1 200); do pgrep -f '[e]rgon-lab-nri-20261024000000' >/dev/null || break; sleep 0.1; done
pgrep -fl '[e]rgon-lab-nri-20261024000000' || true
check "no watchdog left" bash -c "! pgrep -f '[e]rgon-lab-nri-20261024000000' >/dev/null"
check "no snapshot left" test -z "$(find "$work"/*/tmp -mindepth 1 -maxdepth 1 2>/dev/null)"

echo "== lib.sh"
# shellcheck source=../lib.sh
. "$lab/lib.sh"
check "D8ps_v6 has 8 vCPUs" test "$(lab_size_vcpus Standard_D8ps_v6)" = 8
check "D16pds_v6 has 16" test "$(lab_size_vcpus Standard_D16pds_v6)" = 16
check "D8s_v6 is not Arm" bash -c ". '$lab/lib.sh'; ! lab_size_vcpus Standard_D8s_v6"
check "layout of 8: one shared CPU" test "$(lab_layout 8)" = "0 1 3-7 0-2"
check "layout of 8: two shared CPUs, the same reserved and exclusive" test "$(lab_layout 8 2)" = "0 1-2 3-7 0-2"
check "layout of 7" test "$(lab_layout 7)" = "0 1 3-6 0-2"
check "layout of 16, two shared" test "$(lab_layout 16 2)" = "0 1-2 3-15 0-2"
check "no layout under 7" bash -c ". '$lab/lib.sh'; ! lab_layout 6"
check "no layout with 3 shared" bash -c ". '$lab/lib.sh'; ! lab_layout 8 3"
check "boot line" test "$(lab_boot_args 3-7 0)" = "isolcpus=managed_irq,domain,3-7 nohz_full=3-7 rcu_nocbs=3-7 irqaffinity=0"
check "scale spin: one pod" test "$(lab_scale_spin 30 1)" = 40
check "scale spin: three pods started in turn, 30 s more each after the first" test "$(lab_scale_spin 30 3)" = 100
check "scale step seconds" test "$(lab_scale_seconds 30)" = 780
check "trace delay: 20 s more than the bench's, for the tracefs setup" test "$(lab_trace_delay 5)" = 25
check "estimate: one arm, trace, scale" test "$(lab_estimate_minutes 5 60 5)" = 75
check "estimate: both arms" test "$(lab_estimate_minutes 5 60 5 2 1 1)" = 88
check "estimate: one arm alone" test "$(lab_estimate_minutes 5 60 5 1 0 0)" = 58
check "cost" test "$(lab_cost 0.330 60)" = 0.35
check "unknown price" test "$(lab_cost '?' 60)" = "?"
check "first CPU" test "$(lab_first_cpu 3,5)$(lab_first_cpu 2-7)" = 32
yaml() { ruby -ryaml -rjson -e 'puts JSON.generate(YAML.safe_load(STDIN.read))'; }
kc=$(lab_kind_config 0-2 | yaml)
check "kind config parses" test -n "$kc"
check "kind: reservedSystemCPUs 0-2, static" bash -c "jq -r '.nodes[0].kubeadmConfigPatches[1]' <<<'$kc' | ruby -ryaml -rjson -e 'y = YAML.safe_load(STDIN.read); exit(y[\"reservedSystemCPUs\"] == \"0-2\" && y[\"cpuManagerPolicy\"] == \"static\")'"
check "kind: the VM's /sys at /vm-sys, read-only" test "$(jq -c '.nodes[0].extraMounts[0]' <<<"$kc")" = '{"hostPath":"/sys","containerPath":"/vm-sys","readOnly":true}'
check "kind: NRI on" bash -c "jq -r '.containerdConfigPatches[1]' <<<'$kc' | grep -q 'disable = false'"
on=$(lab_pod plugin-on plugin-on-2 2 60 5 20000 "--cpu 1" | yaml)
off=$(lab_pod plugin-off plugin-off-2 2 60 5 20000 "--cpu 1" | yaml)
check "plugin-on pod parses" test -n "$on"
check "plugin-off pod parses" test -n "$off"
check "on: hot label" test "$(jq -r '.metadata.labels["ergon.io/latency"]' <<<"$on")" = hot
check "on: the plugin token" test "$(jq -r '.metadata.annotations["required-plugins.noderesource.dev"]' <<<"$on")" = '["ergon-shared-cpus"]'
check "on: priority class" test "$(jq -r '.spec.priorityClassName' <<<"$on")" = ergon-hot
check "on: one shared CPU, Guaranteed" test "$(jq -c '.spec.containers[0].resources | [.requests == .limits, .limits.cpu, .limits["ergon.io/shared-millicpus"]]' <<<"$on")" = '[true,"2","1000"]'
check "on: resizePolicy RestartContainer" test "$(jq -c '.spec.containers[0].resizePolicy' <<<"$on")" = '[{"resourceName":"cpu","restartPolicy":"RestartContainer"}]'
check "on: restartPolicy OnFailure" test "$(jq -r '.spec.restartPolicy' <<<"$on")" = OnFailure
check "off: no label, token, class, shared CPU or resize" test "$(jq -c '[.metadata.labels["ergon.io/latency"], .metadata.annotations, .spec.priorityClassName, .spec.containers[0].resources.limits["ergon.io/shared-millicpus"], .spec.containers[0].resizePolicy]' <<<"$off")" = '[null,null,null,null,null]'
check "off: Guaranteed, 2 CPUs" test "$(jq -c '.spec.containers[0].resources | [.requests == .limits, .limits.cpu]' <<<"$off")" = '[true,"2"]'
check "both: the same command but the arm" test "$(jq -r '.spec.containers[0].args[0]' <<<"$on" | sed 's/plugin-on/ARM/g')" = "$(jq -r '.spec.containers[0].args[0]' <<<"$off" | sed 's/plugin-off/ARM/g')"
check "both: spin on --cpu auto with the delay, helpers in a child process" bash -c "jq -r '.spec.containers[0].args[0]' <<<'$on' | grep -q -- '--seconds 60 --delay-seconds 5 --udp-rate 20000 --helpers process --cpu auto --out /results'"
check "an unknown arm is refused" bash -c ". '$lab/lib.sh'; ! lab_pod plugin-x a 1 1 1 1 x >/dev/null"
check "unknown helpers are refused" bash -c ". '$lab/lib.sh'; ! lab_pod plugin-on a 1 1 1 1 x fork >/dev/null"
th=$(lab_pod plugin-on t 1 60 5 20000 "--cpu 1" thread | yaml)
check "helpers as threads" bash -c "jq -r '.spec.containers[0].args[0]' <<<'$th' | grep -q -- '--helpers thread --cpu auto'"
tr_=$(lab_pod trace trace-1 1 60 5 20000 "--cpu 1" | yaml)
check "trace pod: hot, one shared CPU, its own arm" test "$(jq -c '[.metadata.labels["ergon.io/latency"], .spec.containers[0].resources.limits["ergon.io/shared-millicpus"], (.spec.containers[0].args[0] | test("--label trace-1 --arm trace "))]' <<<"$tr_")" = '["hot","1000",true]'
sc=$(lab_scale_pod scale-a a-burst 1 100 4 100 40 5 | yaml)
check "scale pod parses" test -n "$sc"
check "scale pod: Guaranteed, cpu 1, 100 shared millicpus" test "$(jq -c '.spec.containers[0].resources | [.requests == .limits, .limits.cpu, .limits["ergon.io/shared-millicpus"]]' <<<"$sc")" = '[true,"1","100"]'
check "scale pod: hot label, token, class, resizePolicy, OnFailure" test "$(jq -c '[.metadata.labels["ergon.io/latency"], .metadata.annotations["required-plugins.noderesource.dev"], .spec.priorityClassName, .spec.containers[0].resizePolicy[0].restartPolicy, .spec.restartPolicy]' <<<"$sc")" = '["hot","[\"ergon-shared-cpus\"]","ergon-hot","RestartContainer","OnFailure"]'
check "scale pod: burn helpers for the delay, spin and 5 s" bash -c "jq -r '.spec.containers[0].args[0]' <<<'$sc' | grep -q -- 'gapprobe burn --threads 4 --duty 100 --seconds 50 &'"
check "scale pod: spin into the case directory, no UDP helpers" bash -c "jq -r '.spec.containers[0].args[0]' <<<'$sc' | grep -q -- 'gapprobe spin --label scale-a --arm a-burst --rep 1 --seconds 40 --delay-seconds 5 --udp-rate 0 --cpu auto --out /results/scale/a-burst'"
check "scale pod: a bad cpu is refused" bash -c ". '$lab/lib.sh'; ! lab_scale_pod x a 0 100 4 100 40 5 >/dev/null"
check "scale pod: a bad case name is refused" bash -c ". '$lab/lib.sh'; ! lab_scale_pod x 'a b' 1 100 4 100 40 5 >/dev/null"
ev=$(lab_trace_events 3)
check "trace events: 21" test "$(wc -l <<<"$ev" | tr -d ' ')" = 21
check "trace: switches on the CPU" grep -qxF 'sched:sched_switch;CPU == 3;' <<<"$ev"
check "trace: wakeups to it, with the waker's stack" grep -qxF 'sched:sched_wakeup;target_cpu == 3;stack' <<<"$ev"
check "trace: IPIs to it by cpumask, with the sender's stack" grep -qxF 'ipi:ipi_send_cpumask;cpumask & CPUS{3};stack' <<<"$ev"
check "trace: migrations to or from it, the filter whole" grep -qxF 'sched:sched_migrate_task;orig_cpu == 3 || dest_cpu == 3;stack' <<<"$ev"
check "trace: work queued for it" grep -qxF 'workqueue:workqueue_queue_work;cpu == 3;stack' <<<"$ev"
qs=$(lab_quiet_settings 0-2)
check "quiet: watchdog on the housekeeping CPUs" grep -qF '/proc/sys/kernel/watchdog_cpumask|0-2|' <<<"$qs"
check "quiet: writeback work on them" grep -qF '/sys/bus/workqueue/devices/writeback/cpumask|7|' <<<"$qs"
check "quiet: each setting says why" test "$(awk -F'|' 'NF == 3 && $3 != ""' <<<"$qs" | wc -l | tr -d ' ')" = "$(wc -l <<<"$qs" | tr -d ' ')"

placement() { lab_placement_check "$@" >"$work/placement" 2>&1; }
good="11 1 1 sh
12 1 1 stress-ng
13 1 1 stress-ng-cpu
20 1 1 gapprobe
21 1 1 gp-udp-sink
22 1 1 gp-udp-src
23 2 2 gp-hot"
check "placement: plugin-on as it should be" placement plugin-on 2 1 <<<"$good"
check "placement: every line ok" test "$(grep -c '^ok:' "$work/placement")" -eq 7
check "placement: hot on another CPU fails" bash -c ". '$lab/lib.sh'; ! lab_placement_check plugin-on 2 1 <<<'${good/23 2 2/23 3 3}' >/dev/null"
check "placement: hot allowed more fails" bash -c ". '$lab/lib.sh'; ! lab_placement_check plugin-on 2 1 <<<'${good/23 2 2/23 2 1-3}' >/dev/null"
check "placement: a helper allowed the exclusive CPUs fails" bash -c ". '$lab/lib.sh'; ! lab_placement_check plugin-on 2 1 <<<'${good/12 1 1/12 1 1-3}' >/dev/null"
check "placement: no hot thread fails" bash -c ". '$lab/lib.sh'; ! lab_placement_check plugin-on 2 1 <<<'11 1 1 sh' >/dev/null"
check "placement: plugin-off records helpers anywhere" placement plugin-off 2 1 <<<"${good//1 1 /3 2-3 }"
check "placement: plugin-off still checks the hot thread" bash -c ". '$lab/lib.sh'; ! lab_placement_check plugin-off 2 1 <<<'${good/23 2 2/23 3 2-3}' >/dev/null"
check "placement: the trace arm checks helpers as plugin-on" bash -c ". '$lab/lib.sh'; ! lab_placement_check trace 2 1 <<<'${good/12 1 1/12 1 1-3}' >/dev/null"
check "placement: two shared CPUs" placement plugin-on 3 1-2 <<<"11 2 1-2 sh
21 1 1-2 gp-burn
23 3 3 gp-hot"

echo "== vm.sh"
VM_LIB_ONLY=1 . "$lab/vm.sh"
fake_tree() { # fake_tree <dir> <msi irq effective> <channel cpus...>
    local t=$1 eff=$2 i=14 c
    shift 2
    mkdir -p "$t/sys/class/net/lo" "$t/sys/class/net/docker0" "$t/sys/class/net/enP1s1/device/msi_irqs/30" "$t/proc/irq/30"
    echo "$eff" >"$t/proc/irq/30/effective_affinity_list"
    for c in "$@"; do
        mkdir -p "$t/sys/class/net/eth0/device/channels/$i"
        [[ $c == - ]] || echo "$c" >"$t/sys/class/net/eth0/device/channels/$i/cpu"
        i=$((i + 1))
    done
}
nic() { LAB_SYS_ROOT=$1/sys LAB_PROC_ROOT=$1/proc nic_check 0 >"$work/nic" 2>&1; }
fake_tree "$work/t1" 0 0 0
check "NIC check: MSI and channels on CPU 0 pass" nic "$work/t1"
check "NIC check: three lines ok" test "$(grep -c '^ok:' "$work/nic")" -eq 3
fake_tree "$work/t2" 0 0 3
nic "$work/t2"
check "NIC check: a channel on CPU 3 fails, named" bash -c "grep -q 'FAIL: eth0 vmbus channel 15 on CPU 3, want 0' '$work/nic'"
check "NIC check: its exit code" bash -c "VM_LIB_ONLY=1 . '$lab/vm.sh'; ! LAB_SYS_ROOT='$work/t2/sys' LAB_PROC_ROOT='$work/t2/proc' nic_check 0 >/dev/null"
fake_tree "$work/t3" 1 0
nic "$work/t3"
check "NIC check: an MSI IRQ on CPU 1 fails, named" has "$work/nic" "FAIL: enP1s1 irq 30 on CPU 1, want 0"
fake_tree "$work/t4" 0 -
nic "$work/t4"
check "NIC check: an unreadable channel fails" has "$work/nic" "FAIL: eth0 vmbus channel 14 on CPU ?, want 0"
mkdir -p "$work/t5/sys/class/net/lo"
check "NIC check: no NIC fails" bash -c "VM_LIB_ONLY=1 . '$lab/vm.sh'; ! LAB_SYS_ROOT='$work/t5/sys' LAB_PROC_ROOT='$work/t5/proc' nic_check 0 >/dev/null"
check "cpumask 0-1" test "$(cpumask 0-1)" = 3
check "cpumask 0,2-3" test "$(cpumask 0,2-3)" = d
check "cpu_count 1,4-6" test "$(cpu_count 1,4-6)" = 4
check "qty 1k" test "$(qty 1k) $(qty 2M) $(qty 250)" = "1000 2000000 250"
check "field" test "$(field 'gapprobe: label=x cpu=2 ERGON_SHARED_CPUS=1 main_affinity=[0, 1]' cpu)" = 2

# from here the fakes are first on PATH: sudo runs as this user
PATH="$lab/test/fakes:$PATH" # (vm.sh, sourced above, set $here to its own directory)
hash -r

# a tracefs of plain files: the events this kernel "has", a trace, per-CPU stats
tfs=$work/tracefs
mkdir -p "$tfs/per_cpu/cpu0" "$tfs/per_cpu/cpu3"
for f in tracing_on trace_clock buffer_size_kb current_tracer set_event; do echo x >"$tfs/$f"; done
printf 'entries: 9\noverrun: 0\n' >"$tfs/per_cpu/cpu0/stats"
printf 'entries: 4\noverrun: 2\n' >"$tfs/per_cpu/cpu3/stats"
for e in sched/sched_switch sched/sched_wakeup ipi/ipi_send_cpu ipi/ipi_send_cpumask irq/irq_handler_entry workqueue/workqueue_queue_work; do
    mkdir -p "$tfs/events/$e"
    for f in enable filter trigger; do echo 0 >"$tfs/events/$e/$f"; done
done
printf '#!/bin/sh\n[ "$1" = clocks ] && echo "1700000000000000500 1000"\n' >"$work/fake-gapprobe"
chmod +x "$work/fake-gapprobe"
tv() { LAB_TRACEFS=$tfs LAB_GAPPROBE=$work/fake-gapprobe "$@"; }
echo "line one" >"$tfs/trace"
tv trace_start 3 "$work/tr" >"$work/tr.out" 2>&1
check "trace_start succeeds" test $? -eq 0
check "trace: mono clock" test "$(cat "$tfs/trace_clock")" = mono
check "trace: on" test "$(cat "$tfs/tracing_on")" = 1
check "trace: a big buffer" test "$(cat "$tfs/buffer_size_kb")" = 16384
check "trace: the switch filter" test "$(cat "$tfs/events/sched/sched_switch/filter")" = "CPU == 3"
check "trace: the switch enabled" test "$(cat "$tfs/events/sched/sched_switch/enable")" = 1
check "trace: the sender's stack on IPIs" test "$(cat "$tfs/events/ipi/ipi_send_cpu/trigger")" = "stacktrace if cpu == 3"
check "trace: no stack on the switch" test "$(cat "$tfs/events/sched/sched_switch/trigger")" = 0
check "trace: enabled events listed" grep -qxF "sched:sched_switch if CPU == 3" "$work/tr.events"
check "trace: stacks listed" grep -qxF "ipi:ipi_send_cpumask if cpumask & CPUS{3}, with the sender's stack" "$work/tr.events"
check "trace: missing events listed" grep -qxF "csd:csd_queue_cpu" "$work/tr.missing"
check "trace: 6 enabled, 15 missing" test "$(wc -l <"$work/tr.events" | tr -d ' ') $(wc -l <"$work/tr.missing" | tr -d ' ')" = "6 15"
check "trace: realtime - mono" test "$(cat "$work/tr.offset")" = 1699999999999999500
check "trace: the buffer cleared first" bash -c "! grep -q 'line one' '$tfs/trace'"
echo "the trace text" >"$tfs/trace"
tv trace_stop "$work/tr" >"$work/tr.out" 2>&1
check "trace_stop succeeds" test $? -eq 0
check "trace: off" test "$(cat "$tfs/tracing_on")" = 0
check "trace: copied" test "$(cat "$work/tr.trace.txt")" = "the trace text"
check "trace: the offset read again at the end" test "$(cat "$work/tr.offset.end")" = 1699999999999999500
check "trace: overruns kept per CPU" grep -qxF "cpu3 overrun: 2" "$work/tr.stats"
check "trace: events disabled" test "$(cat "$tfs/events/sched/sched_switch/enable")" = 0
check "trace: filters cleared" test "$(cat "$tfs/events/sched/sched_switch/filter")" = 0
check "trace: triggers removed" test "$(cat "$tfs/events/ipi/ipi_send_cpu/trigger")" = "!stacktrace if cpu == 3"
check "trace: the clock back to local" test "$(cat "$tfs/trace_clock")" = local
check "trace_start fails without tracefs" bash -c "VM_LIB_ONLY=1 . '$lab/vm.sh'; ! LAB_TRACEFS='$work/none' LAB_GAPPROBE=true trace_start 3 '$work/tr2' >/dev/null 2>&1"

# the quiet settings on a fake /proc/sys and /sys
q=$work/quiet
mkdir -p "$q/proc/sys/vm" "$q/proc/sys/kernel" "$q/sys/bus/workqueue/devices/writeback"
echo 1 >"$q/proc/sys/vm/stat_interval"
echo 0-7 >"$q/proc/sys/kernel/watchdog_cpumask"
echo 1 >"$q/proc/sys/kernel/nmi_watchdog"
echo 20 >"$q/proc/sys/vm/compaction_proactiveness"
echo 1 >"$q/proc/sys/kernel/timer_migration"
echo ff >"$q/sys/bus/workqueue/devices/writeback/cpumask"
LAB_PROC_ROOT=$q/proc LAB_SYS_ROOT=$q/sys quiet_node 0-2 >"$work/quiet.out" 2>&1
check "quiet: stat_interval 1 -> 120" grep -qF "vm/stat_interval: 1 -> 120" "$work/quiet.out"
check "quiet: written" test "$(cat "$q/proc/sys/vm/stat_interval")" = 120
check "quiet: the writeback cpumask" test "$(cat "$q/sys/bus/workqueue/devices/writeback/cpumask")" = 7
check "quiet: a setting this kernel lacks is skipped, named" grep -qF "skip: /proc/sys/kernel/numa_balancing (not on this kernel)" "$work/quiet.out"

# a probe's container cgroup, from /proc/<pid>/cgroup
mkdir -p "$work/cg/proc/42" "$work/cg/fs/kubelet.slice/pod1.slice/cri-1.scope"
echo "0::/kubelet.slice/pod1.slice/cri-1.scope" >"$work/cg/proc/42/cgroup"
check "cgroup_dir" test "$(LAB_PROC_ROOT=$work/cg/proc LAB_CGROUP_ROOT=$work/cg/fs cgroup_dir 42)" = "$work/cg/fs/kubelet.slice/pod1.slice/cri-1.scope"
check "cgroup_dir: none" bash -c "VM_LIB_ONLY=1 . '$lab/vm.sh'; ! LAB_PROC_ROOT='$work/cg/proc' cgroup_dir 43 >/dev/null 2>&1"

# a scale snapshot: /proc/stat, the container's and the pod's cpu.stat, the spin's schedstat
ss=$work/ss
mkdir -p "$ss/proc/42/task/43" "$ss/fs/pod1.slice/cri-1.scope" "$ss/out"
echo "cpu1 1 0 0 9 0 0 0 0 0 0" >"$ss/proc/stat"
echo "usage_usec 5" >"$ss/fs/pod1.slice/cri-1.scope/cpu.stat"
echo "usage_usec 7" >"$ss/fs/pod1.slice/cpu.stat"
echo "100 0 0" >"$ss/proc/42/task/43/schedstat"
(
    declare -A scale_cg=([p]=$ss/fs/pod1.slice/cri-1.scope) scale_pid=([p]=42) scale_tid=([p]=43)
    LAB_PROC_ROOT=$ss/proc scale_snapshot "$ss/out" before
)
check "scale snapshot: /proc/stat" test -s "$ss/out/proc.stat.before"
check "scale snapshot: the container's cpu.stat" test "$(cat "$ss/out/p.cpu.stat.before")" = "usage_usec 5"
check "scale snapshot: the pod's cpu.stat (a pod-level quota throttles there)" test "$(cat "$ss/out/p.pod.cpu.stat.before" 2>/dev/null)" = "usage_usec 7"
check "scale snapshot: the spin's schedstat" test "$(cat "$ss/out/p.hot.schedstat.before")" = "100 0 0"

# the traced rep's tracefs must be armed before its spin starts
printf 'label\tarm\trep\tcpu\tsamples\tp50_ns\tp90_ns\tp99_ns\tp999_ns\tp9999_ns\tmax_ns\tmax_end_ns\tgaps\tdropped\tnvcsw\tnivcsw\tstart_ns\tend_ns\tminflt\tmajflt\nt\ttrace\t1\t3\t1\t1\t1\t1\t1\t1\t1\t1\t0\t0\t0\t0\t2000\t9000\t0\t0\n' >"$work/armed.summary.tsv"
check "armed before the spin: ok" bash -c "VM_LIB_ONLY=1 . '$lab/vm.sh'; armed_before_spin 1999 '$work/armed.summary.tsv'"
check "armed after the spin started: refused" bash -c "VM_LIB_ONLY=1 . '$lab/vm.sh'; ! armed_before_spin 2001 '$work/armed.summary.tsv'"
check "armed, no summary: refused" bash -c "VM_LIB_ONLY=1 . '$lab/vm.sh'; ! armed_before_spin 1 '$work/none.tsv'"

echo
if ((fails)); then
    echo "test-lab: $fails FAILED"
    exit 1
fi
echo "test-lab: PASS"
