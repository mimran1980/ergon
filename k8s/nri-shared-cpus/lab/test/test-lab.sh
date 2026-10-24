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
    "IRQ CPU 0, shared CPU 1, exclusive (isolated) 2-7, reservedSystemCPUs 0-1" \
    "boot line isolcpus=managed_irq,domain,2-7 nohz_full=2-7 rcu_nocbs=2-7 irqaffinity=0" \
    "5 reps x plugin-on, plugin-off; 60 s spin" "about 71 minutes" "about \$0.330/h" "about \$0.41 for the run" \
    "at most \$1.40 to the backstop" "(LAB_MAX_HOURS=4)" "tagged owner=ergon-lab run=20261024000000" \
    "az group create -n ergon-lab-nri-20261024000000 -l eastus --tags owner=ergon-lab run=20261024000000" \
    "--assign-identity [system] --role Contributor --scope <group id>" "az vm auto-shutdown" \
    "az group delete -n ergon-lab-nri-20261024000000 --yes --no-wait" "az resource list --tag run=20261024000000"; do
    check "plan names: $want" has "$d/out" "$want"
done

echo "== refusals before anything"
for c in "LAB_SIZE=Standard_D8s_v6|not an Azure Arm" "LAB_SIZE=Standard_D2ps_v6|the lab needs 4 or more" \
    "LAB_REPS=50|more than LAB_MAX_HOURS" "LAB_SECONDS=abc|LAB_SECONDS is a whole number" \
    "LAB_MAX_HOURS=30|LAB_MAX_HOURS is 1 to 24" "LAB_STRESS=--cpu;rm|stress-ng flags only"; do
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
check "steps in order" test "$(tr '\n' ' ' <"$work/ok/fake/steps.log")" = "bootstrap reboot tune cluster e2e bench report "
check "group create before the VM" test "$(line_of ok 'group create')" -lt "$(line_of ok 'vm create')"
check "group created with the run's tags" has "$work/ok/fake/az.log" "az group create -n ergon-lab-nri-20261024000000 -l eastus --tags owner=ergon-lab run=20261024000000"
check "the VM: identity on the group alone" has "$work/ok/fake/az.log" "--assign-identity [system] --role Contributor --scope /subscriptions/00000000-0000-0000-0000-00000000cafe/resourceGroups/ergon-lab-nri-20261024000000 "
check "the VM: Arm size, image, cloud-init, tags" has "$work/ok/fake/az.log" "--size Standard_D8ps_v6 --image Debian:debian-13:13-arm64:latest"
check "the VM carries the backstop cloud-init" has "$work/ok/fake/az.log" "--custom-data $work/ok/out-dir/cloud-init.yaml --tags owner=ergon-lab run=20261024000000"
check "SSH only from this machine" has "$work/ok/fake/az.log" "--source-address-prefixes 198.51.100.7/32"
check "every resource tagged" test "$(grep -c 'az resource tag --ids /fake/.* --tags owner=ergon-lab run=20261024000000 -i' "$work/ok/fake/az.log")" -eq 3
check "auto-shutdown set" has "$work/ok/fake/az.log" "az vm auto-shutdown -g ergon-lab-nri-20261024000000 -n vm --time"
ci=$work/ok/out-dir/cloud-init.yaml
check "backstop: a timer at the deadline" grep -qE 'OnCalendar=20[0-9]{2}-[0-9]{2}-[0-9]{2} [0-9:]{8} UTC' "$ci"
check "backstop: deletes this group" has "$ci" "curl -fsS -X DELETE -H \"Authorization: Bearer \$tok\" 'https://management.azure.com/subscriptions/00000000-0000-0000-0000-00000000cafe/resourceGroups/ergon-lab-nri-20261024000000?api-version=2021-04-01'"
check "backstop: enabled from first boot" has "$ci" "[systemctl, enable, --now, ergon-lab-backstop.timer]"
check "backstop: checked before the steps" has "$work/ok/fake/ssh.log" "sudo /usr/local/sbin/ergon-lab-backstop check"
for want in IRQ_CPU=0 SHARED_CPUS=1 EXCLUSIVE_CPUS=2-7 RESERVED_CPUS=0-1 REPS=5 SECONDS_PER_RUN=60 'STRESS="--cpu 1 --vm 1 --vm-bytes 128M"'; do
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

echo "== teardown on a failed step"
run_lab stepfail FAKE_FAIL_STEP=e2e LAB_SSH_WAIT_SECONDS=5
check "fails" test "$(rc_of stepfail)" -ne 0
check "stops at the failed step" test "$(tr '\n' ' ' <"$work/stepfail/fake/steps.log")" = "bootstrap reboot tune cluster e2e "
check "names it" has "$work/stepfail/out" "step e2e failed (exit 1)"
check "group deleted and verified" has "$work/stepfail/out" "verified: group"

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

echo "== nothing outlives the runs"
sleep 1
check "no watchdog left" bash -c "! pgrep -f 'ergon-lab-nri-20261024000000' >/dev/null"
check "no snapshot left" test -z "$(ls "$work"/*/tmp 2>/dev/null | grep -v ':$' | grep .)"

echo "== lib.sh"
# shellcheck source=../lib.sh
. "$lab/lib.sh"
check "D8ps_v6 has 8 vCPUs" test "$(lab_size_vcpus Standard_D8ps_v6)" = 8
check "D16pds_v6 has 16" test "$(lab_size_vcpus Standard_D16pds_v6)" = 16
check "D8s_v6 is not Arm" bash -c ". '$lab/lib.sh'; ! lab_size_vcpus Standard_D8s_v6"
check "layout of 8" test "$(lab_layout 8)" = "0 1 2-7 0-1"
check "layout of 4" test "$(lab_layout 4)" = "0 1 2-3 0-1"
check "no layout under 4" bash -c ". '$lab/lib.sh'; ! lab_layout 2"
check "boot line" test "$(lab_boot_args 2-7 0)" = "isolcpus=managed_irq,domain,2-7 nohz_full=2-7 rcu_nocbs=2-7 irqaffinity=0"
check "estimate" test "$(lab_estimate_minutes 5 60 5)" = 71
check "cost" test "$(lab_cost 0.330 60)" = 0.35
check "unknown price" test "$(lab_cost '?' 60)" = "?"
check "first CPU" test "$(lab_first_cpu 3,5)$(lab_first_cpu 2-7)" = 32
yaml() { ruby -ryaml -rjson -e 'puts JSON.generate(YAML.safe_load(STDIN.read))'; }
kc=$(lab_kind_config 0-1 | yaml)
check "kind config parses" test -n "$kc"
check "kind: reservedSystemCPUs 0-1, static" bash -c "jq -r '.nodes[0].kubeadmConfigPatches[1]' <<<'$kc' | ruby -ryaml -rjson -e 'y = YAML.safe_load(STDIN.read); exit(y[\"reservedSystemCPUs\"] == \"0-1\" && y[\"cpuManagerPolicy\"] == \"static\")'"
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
check "both: spin on --cpu auto with the delay" bash -c "jq -r '.spec.containers[0].args[0]' <<<'$on' | grep -q -- '--seconds 60 --delay-seconds 5 --udp-rate 20000 --cpu auto --out /results'"
check "an unknown arm is refused" bash -c ". '$lab/lib.sh'; ! lab_pod plugin-x a 1 1 1 1 x >/dev/null"

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

echo
if ((fails)); then
    echo "test-lab: $fails FAILED"
    exit 1
fi
echo "test-lab: PASS"
