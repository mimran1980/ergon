#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The ergon-shared-cpus latency lab on ONE Azure Cobalt VM that exists only
# while this script runs. Runs on macOS or Linux with the az CLI logged in
# and an SSH public key (~/.ssh/id_ed25519.pub or id_rsa.pub).
#
#   k8s/nri-shared-cpus/lab/azure-vm.sh            the run
#   DRY_RUN=1 k8s/nri-shared-cpus/lab/azure-vm.sh  the plan, size, price and
#                                                  every az call; calls nothing
#
# Creates resource group $LAB_GROUP (refusing one that exists) in
# $LAB_REGION, every resource in it tagged owner=ergon-lab run=<stamp>: an
# NSG (SSH from this machine's address only) and one $LAB_SIZE VM ($LAB_IMAGE)
# with a system identity that is Contributor on that group alone. Then, on
# the VM (vm.sh): bootstrap (docker, kind, kubectl, helm, the boot line),
# a reboot into
#   isolcpus=managed_irq,domain,<exclusive> nohz_full=<exclusive> rcu_nocbs=<exclusive> irqaffinity=0
# where <exclusive> is CPUs 3 to the last (the ones the kind node's kubelet
# hands out: static CPU manager, reservedSystemCPUs 0-2: CPU 0 the IRQ CPU,
# CPUs 1-2 housekeeping, of which the chart's sharedCpus is 1 for e2e and
# bench and 1-2 for scale: lib.sh lab_layout), tune (every NIC IRQ and
# VMBus channel on CPU 0; with LAB_QUIET=1 the node settings of lib.sh
# lab_quiet_settings), cluster (both images built on the VM from this
# checkout, uploaded as a tarball: uncommitted changes included), e2e (a
# hot pod admitted and started, its spin on its isolated CPU, its helpers
# on the shared CPU, an exec-probe hot pod denied), scale (how much of two
# shared CPUs a pod's helpers get at 100 and 1001 millicpus, idle, and when
# pods share one: results/scale.md), bench ($LAB_REPS reps
# of each of $LAB_ARMS, then one traced rep: results/trace-cpu<N>.md) and
# report. Results go to target/nri-lab/<stamp>/ (report.md first). The
# group is deleted on every exit, then checked gone.
#
#   LAB_REGION (eastus) LAB_SIZE (Standard_D8ps_v6; Arm sizes only, 7 vCPUs
#   or more, so 8) LAB_IMAGE (Debian:debian-13:13-arm64:latest; or
#   Canonical:ubuntu-24_04-lts:server-arm64:latest) LAB_GROUP
#   (ergon-lab-nri-<stamp>) LAB_REPS (5) LAB_SECONDS (60: spin seconds per run)
#   LAB_DELAY (5: helper seconds before each spin) LAB_UDP_RATE (20000
#   datagrams/s) LAB_STRESS ("--cpu 1 --vm 1 --vm-bytes 128M")
#   LAB_ARMS ("plugin-on"; "plugin-on plugin-off" adds the plain static CPU
#   manager arm) LAB_HELPERS (process: the probe's UDP helpers in a child
#   process; thread: threads of the probe, as run 20261024212712 did)
#   LAB_BENCH (1; 0 skips bench and its trace) LAB_TRACE (1: one more
#   plugin-on rep under kernel ftrace) LAB_SCALE (1: the scale step)
#   LAB_SCALE_SECONDS (30: each scale case's window) LAB_SCALE_THREADS (4:
#   busy helper threads a scale pod) LAB_QUIET (1: the node settings)
#   LAB_MAX_HOURS (4): the deleting backstop. A systemd timer on the VM
#   (cloud-init, so from first boot, surviving the reboot) deletes the
#   group through the VM's identity at that deadline; a watchdog on this
#   machine deletes it as well; Azure's auto-shutdown deallocates the VM 30
#   minutes later (not before: a deallocated VM cannot run its timer); and
#   this script stops its run 15 minutes before the deadline.
#   LAB_DETACH (auto): a run the plan puts over 2 hours re-runs itself
#   detached (nohup, setsid, caffeinate on macOS) and returns at once;
#   LAB_DETACH=1 always, 0 never. Its log is target/nri-lab/<stamp>/run.log;
#   stop it with kill -TERM <pid> (the teardown runs).
#   LAB_PRICE_PER_HOUR (lib.sh's table): USD an hour for the plan's cost.
#   LAB_SSH_FROM (this machine's public address, api.ipify.org).
#   LAB_OUT (target/nri-lab/<stamp>).
# Test knobs (test/test-lab.sh): LAB_POLL_SECONDS (20), LAB_SSH_WAIT_SECONDS
# (900), LAB_REBOOT_SECONDS (20).
set -uo pipefail
((BASH_VERSINFO[0] >= 4)) || { echo "azure-vm.sh needs bash 4 or later (macOS: brew install bash)" >&2; exit 2; }

here=$(cd "$(dirname "$0")" && pwd)
repo=${LAB_REPO:-$(cd "$here/../../.." && pwd)}
# shellcheck source=lib.sh
. "$here/lib.sh"

stamp=${LAB_STAMP:-$(date -u +%Y%m%d%H%M%S)}
region=${LAB_REGION:-eastus}
size=${LAB_SIZE:-Standard_D8ps_v6}
image=${LAB_IMAGE:-Debian:debian-13:13-arm64:latest}
group=${LAB_GROUP:-ergon-lab-nri-$stamp}
reps=${LAB_REPS:-5}
seconds=${LAB_SECONDS:-60}
delay=${LAB_DELAY:-5}
udp_rate=${LAB_UDP_RATE:-20000}
stress=${LAB_STRESS:---cpu 1 --vm 1 --vm-bytes 128M}
arms=${LAB_ARMS-plugin-on}
helpers=${LAB_HELPERS:-process}
bench=${LAB_BENCH:-1}
trace=${LAB_TRACE:-1}
scale=${LAB_SCALE:-1}
scale_seconds=${LAB_SCALE_SECONDS:-30}
scale_threads=${LAB_SCALE_THREADS:-4}
quiet=${LAB_QUIET:-1}
max_hours=${LAB_MAX_HOURS:-4}
poll=${LAB_POLL_SECONDS:-20}
ssh_wait=${LAB_SSH_WAIT_SECONDS:-900}
reboot_wait=${LAB_REBOOT_SECONDS:-20}
out=${LAB_OUT:-$repo/target/nri-lab/$stamp}
user=azureuser
tags=(owner=ergon-lab "run=$stamp")
created=0

# the log goes to the stderr the script started with, through fd 3: a trap
# that runs inside a command whose stderr is redirected (an interrupt during
# `on true 2>/dev/null`) still logs its teardown
exec 3>&2
log() { echo "[$(date +%T)] $*" >&3; }
die() { log "ERROR: $*"; exit 1; }

# ---------------------------------------------------------------- the plan

vcpus=$(lab_size_vcpus "$size") || die "LAB_SIZE $size: not an Azure Arm (Cobalt) size such as Standard_D8ps_v6"
read -r irq_cpu shared_cpus exclusive_cpus reserved_cpus <<<"$(lab_layout "$vcpus" 1)" ||
    die "LAB_SIZE $size: $vcpus vCPUs; the lab needs 7 or more (IRQ, two shared, four exclusive)"
[[ -n $irq_cpu ]] || die "LAB_SIZE $size: $vcpus vCPUs; the lab needs 7 or more (IRQ, two shared, four exclusive)"
read -r _ scale_shared_cpus _ _ <<<"$(lab_layout "$vcpus" 2)"
for v in reps seconds delay udp_rate max_hours scale_seconds scale_threads; do
    [[ ${!v} =~ ^[0-9]+$ ]] || die "LAB_${v^^} is a whole number, not ${!v}"
done
for v in bench trace scale quiet; do
    [[ ${!v} == 0 || ${!v} == 1 ]] || die "LAB_${v^^} is 0 or 1, not ${!v}"
done
((reps >= 1)) || die "LAB_REPS is 1 or more"
((seconds >= 10)) || die "LAB_SECONDS is 10 or more"
((scale_seconds >= 10)) || die "LAB_SCALE_SECONDS is 10 or more"
[[ $helpers == process || $helpers == thread ]] || die "LAB_HELPERS: process or thread, not $helpers"
read -ra arm_list <<<"$arms"
((${#arm_list[@]})) || die "LAB_ARMS: plugin-on and/or plugin-off, not empty"
for a in "${arm_list[@]}"; do
    [[ $a == plugin-on || $a == plugin-off ]] || die "LAB_ARMS: plugin-on and/or plugin-off, not $a"
done
arms=${arm_list[*]}
((bench)) || trace=0
((max_hours >= 1 && max_hours <= 23)) || die "LAB_MAX_HOURS is 1 to 23 (auto-shutdown takes a time of day, 30 min after the deadline)"
[[ $stress =~ ^[-a-zA-Z0-9\ ]+$ ]] || die "LAB_STRESS: stress-ng flags only (letters, digits, dashes, spaces)"
boot=$(lab_boot_args "$exclusive_cpus" "$irq_cpu")
price=${LAB_PRICE_PER_HOUR:-$(lab_price_per_hour "$size")}
minutes=$(lab_estimate_minutes "$reps" "$seconds" "$delay" $((bench * ${#arm_list[@]})) "$trace" "$scale" "$scale_seconds")
((minutes <= max_hours * 60 - 15)) ||
    die "the plan takes about $minutes minutes, more than LAB_MAX_HOURS=$max_hours less 15: fewer reps or seconds, or more hours"
cost=$(lab_cost "$price" "$minutes")
cap=$(lab_cost "$price" $((max_hours * 60)))
now=$(date -u +%s)
deadline=$((now + max_hours * 3600))
deadline_utc=$(date -u -r "$deadline" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date -u -d "@$deadline" '+%Y-%m-%d %H:%M:%S')
# auto-shutdown only deallocates, and a deallocated VM cannot run its
# deleting timer: it comes 30 minutes after the timer, as a last stop
off=$((deadline + 1800))
off_hhmm=$(date -u -r "$off" +%H%M 2>/dev/null || date -u -d "@$off" +%H%M)
detach=${LAB_DETACH:-auto}
[[ $detach == auto ]] && { ((minutes > 120)) && detach=1 || detach=0; }

vm_steps="e2e"
((scale)) && vm_steps+=", scale"
((bench)) && vm_steps+=", bench"
vm_steps+=", report"
helpers_said="a child process"
[[ $helpers == thread ]] && helpers_said="threads of the probe"
if ((bench)); then
    bench_said="$reps reps x ${arms// /, }; ${seconds} s spin after ${delay} s helpers ($helpers_said); stress-ng $stress; UDP $udp_rate/s"
else
    bench_said="off (LAB_BENCH=0)"
fi
if ((trace)); then
    trace_said="one more ${seconds} s plugin-on rep under kernel ftrace, filtered to its spin CPU, after $(lab_trace_delay "$delay") s helpers (the tracefs setup done first): results/trace-cpu<N>.md"
else
    trace_said="off (LAB_TRACE=0)"
fi
if ((scale)); then
    scale_said="sharedCpus $scale_shared_cpus: bursts at 100 and 1001 millicpus, idle, contention ($scale_threads helper threads a pod, ${scale_seconds} s each), accounting: results/scale.md"
else
    scale_said="off (LAB_SCALE=0)"
fi
if ((quiet)); then quiet_said="the quiet settings (LAB_QUIET=1): $(lab_quiet_settings "$reserved_cpus" | cut -d'|' -f1 | sed 's|^/proc/sys/||; s|^/sys/||' | paste -sd, - | sed 's/,/, /g')"; else
    quiet_said="as booted (LAB_QUIET=0)"
fi

cat >&2 <<EOF
ergon-shared-cpus latency lab, run $stamp
  group     $group in $region, every resource tagged ${tags[*]}
  VM        $size ($vcpus vCPUs, Arm), $image, 64 GB OS disk, one Standard public IP
  layout    IRQ CPU $irq_cpu, shared CPU $shared_cpus (e2e, bench) or $scale_shared_cpus (scale), exclusive (isolated) $exclusive_cpus, reservedSystemCPUs $reserved_cpus
  boot line $boot
  node      $quiet_said
  bench     $bench_said
  trace     $trace_said
  scale     $scale_said
  time      about $minutes minutes$( ((detach)) && echo ", detached")
  price     about \$$price/h for the VM (+\$0.02/h IP and disk): about \$$cost for the run, at most \$$cap to the backstop
  backstop  $deadline_utc UTC (LAB_MAX_HOURS=$max_hours): the VM deletes the group; auto-shutdown deallocates it 30 min later
  results   $out
EOF

if [[ ${DRY_RUN:-} == 1 ]]; then
    cat >&2 <<EOF
DRY_RUN: the az calls a run makes, in order (nothing called):
  az account show --query id -o tsv
  az vm image show --urn $image -l $region
  az group list --query "[?tags.owner=='ergon-lab'].name" -o tsv     (leftovers from earlier runs: refused)
  az group exists -n $group
  az group create -n $group -l $region --tags ${tags[*]}
  az network nsg create -g $group -n nsg --tags ${tags[*]}
  az network nsg rule create -g $group --nsg-name nsg -n ssh --priority 100 --source-address-prefixes <this machine>/32 --destination-port-ranges 22 --protocol Tcp --access Allow
  az vm create -g $group -n vm --size $size --image $image --admin-username $user --ssh-key-values <key> --nsg nsg --public-ip-sku Standard --os-disk-size-gb 64 --accelerated-networking true --assign-identity [system] --role Contributor --scope <group id> --custom-data <backstop cloud-init> --tags ${tags[*]}
  az tag update --resource-id <each resource in $group> --operation merge --tags ${tags[*]}
  az vm auto-shutdown -g $group -n vm --time $off_hhmm
  az vm show -d -g $group -n vm --query publicIps -o tsv
  then over SSH: the backstop timer enabled and its identity able to read the group,
  vm.sh bootstrap, a reboot, vm.sh tune, cluster, $vm_steps; the results copied back
  ... every $((900 / 60)) minutes: az resource list -g $group
  on exit: az group delete -n $group --yes --no-wait; az group wait -n $group --deleted (up to 3 tries)
           az group exists -n $group; az resource list --tag run=$stamp
EOF
    exit 0
fi

key=
for k in ~/.ssh/id_ed25519.pub ~/.ssh/id_rsa.pub; do [[ -f $k ]] && { key=$k; break; }; done
[[ -n $key ]] || die "no ~/.ssh/id_ed25519.pub or ~/.ssh/id_rsa.pub"
command -v az >/dev/null || die "az is not on PATH"
mkdir -p "$out" || die "cannot create $out"

# The checkout the VM builds, uncommitted changes included; then this run
# executes from a copy of these scripts in it, so an edit to the checkout
# mid-run cannot break the run or its teardown.
if [[ -z ${LAB_SNAPSHOT:-} ]]; then
    COPYFILE_DISABLE=1 tar -czf "$out/src.tgz" -C "$repo" --exclude target --exclude .git \
        k8s/nri-shared-cpus k8s/nri-latency-pods/admission || die "cannot pack the checkout into $out/src.tgz"
    snapshot=$(mktemp -d "${TMPDIR:-/tmp}/ergon-lab.XXXXXX") || die "no snapshot directory in ${TMPDIR:-/tmp}"
    tar -xzf "$out/src.tgz" -C "$snapshot" || die "cannot unpack $out/src.tgz"
    export LAB_SNAPSHOT=$snapshot LAB_REPO=$repo LAB_STAMP=$stamp LAB_OUT=$out LAB_GROUP=$group
    self="$snapshot/k8s/nri-shared-cpus/lab/azure-vm.sh"
    if ((detach)) && [[ -z ${LAB_DETACHED:-} ]]; then
        export LAB_DETACHED=1
        wrap=()
        command -v setsid >/dev/null && wrap+=(setsid)
        command -v caffeinate >/dev/null && wrap+=(caffeinate -i)
        nohup "${wrap[@]}" bash "$self" "$@" >"$out/run.log" 2>&1 </dev/null &
        log "detached: pid $!, log $out/run.log; stop it with: kill -TERM $!"
        exit 0
    fi
    exec bash "$self" "$@"
fi

remove_snapshot() {
    local s=${LAB_SNAPSHOT:-}
    [[ -n $s && $s == "${TMPDIR:-/tmp}"/ergon-lab.* && -f $s/k8s/nri-shared-cpus/lab/azure-vm.sh ]] && rm -rf -- "$s"
}

# ---------------------------------------------------------------- teardown

teardown() {
    local rc=$?
    trap '' INT TERM HUP
    [[ -n ${watchdog:-} ]] && kill "$watchdog" 2>/dev/null
    if ((created)); then
        # three tries: a failed or timed-out delete (a throttle, a reset
        # connection) is sent again while the group still exists
        local exists left listed=1 try
        for try in 1 2 3; do
            log "deleting resource group $group (try $try)"
            if az group delete -n "$group" --yes --no-wait -o none; then
                az group wait -n "$group" --deleted --timeout 3600 -o none || log "WARNING: az group wait failed"
            else
                log "WARNING: az group delete failed"
            fi
            exists=$(az group exists -n "$group" 2>/dev/null)
            [[ $exists == true ]] || break
            sleep "$poll"
        done
        # every read must succeed for "verified": an expired login reads as
        # nothing left, and that is not a check
        left=$(az resource list --tag "run=$stamp" --query '[].id' -o tsv 2>/dev/null) || listed=0
        if [[ $exists == false && $listed == 1 && -z $left ]]; then
            log "verified: group $group is gone and no resource is tagged run=$stamp"
        else
            log "WARNING: not verified, may still bill: group exists=${exists:-unread}, tagged run=$stamp: $( ((listed)) && echo "${left:-none}" || echo unread). Check and delete by hand: az group delete -n $group --yes; az resource list --tag run=$stamp"
            ((rc)) || rc=1
        fi
    fi
    log "results: $out"
    remove_snapshot
    exit "$rc"
}
trap teardown EXIT
trap 'exit 130' INT TERM
trap 'exit 129' HUP

# ---------------------------------------------------------------- create

sub=$(az account show --query id -o tsv) || die "az is not logged in (az login)"
az vm image show --urn "$image" -l "$region" -o none || die "image $image not found in $region's catalog (LAB_IMAGE)"
leftover=$(az group list --query "[?tags.owner=='ergon-lab'].name" -o tsv) || die "az group list failed"
[[ -z $leftover ]] || die "groups tagged owner=ergon-lab exist (an earlier run's?): $leftover. Delete them first"
[[ $(az group exists -n "$group") == false ]] || die "resource group $group exists: delete it or set LAB_GROUP"
me=${LAB_SSH_FROM:-$(curl -fsS -m 10 https://api.ipify.org)} || die "cannot find this machine's public address"
[[ $me =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "this machine's address is not an IPv4 address: $me"
rgid=/subscriptions/$sub/resourceGroups/$group

log "creating $group in $region"
# set first: a create that reaches ARM and then fails or is interrupted
# still leaves a group, and the teardown deletes it (the name was free)
created=1
az group create -n "$group" -l "$region" --tags "${tags[@]}" -o none || die "group create failed"
# this machine's watchdog: the group goes at the deadline should this
# script die without its trap (the VM's timer and auto-shutdown are the rest)
nohup bash -c "trap 'kill \$s 2>/dev/null; exit 0' TERM; sleep $((deadline - $(date -u +%s))) & s=\$!; wait \$s; az group delete -n '$group' --yes --no-wait" >/dev/null 2>&1 </dev/null &
watchdog=$!
az network nsg create -g "$group" -n nsg --tags "${tags[@]}" -o none || die "nsg create failed"
az network nsg rule create -g "$group" --nsg-name nsg -n ssh --priority 100 --source-address-prefixes "$me/32" \
    --destination-port-ranges 22 --protocol Tcp --access Allow -o none || die "nsg rule create failed"

cat >"$out/cloud-init.yaml" <<EOF
#cloud-config
write_files:
  - path: /usr/local/sbin/ergon-lab-backstop
    permissions: '0755'
    content: |
      #!/bin/sh
      # The lab's deleting backstop: delete this VM's resource group with its
      # identity. "check": read the group instead (the role is in place).
      tok=\$(curl -fsS -H Metadata:true 'http://169.254.169.254/metadata/identity/oauth2/token?api-version=2018-02-01&resource=https://management.azure.com/' | sed -n 's/.*"access_token":"\\([^"]*\\)".*/\\1/p')
      [ -n "\$tok" ] || exit 1
      if [ "\${1:-}" = check ]; then
        curl -fsS -o /dev/null -H "Authorization: Bearer \$tok" 'https://management.azure.com$rgid?api-version=2021-04-01'
        exit
      fi
      curl -fsS -X DELETE -H "Authorization: Bearer \$tok" 'https://management.azure.com$rgid?api-version=2021-04-01'
  - path: /etc/systemd/system/ergon-lab-backstop.service
    content: |
      [Unit]
      Description=ergon-lab: delete resource group $group (run $stamp)
      [Service]
      Type=oneshot
      ExecStart=/usr/local/sbin/ergon-lab-backstop
      Restart=on-failure
      RestartSec=120
  - path: /etc/systemd/system/ergon-lab-backstop.timer
    content: |
      [Timer]
      OnCalendar=$deadline_utc UTC
      Persistent=true
      [Install]
      WantedBy=timers.target
runcmd:
  - [systemctl, daemon-reload]
  - [systemctl, enable, --now, ergon-lab-backstop.timer]
EOF

log "creating the VM ($size, $image)"
az vm create -g "$group" -n vm --size "$size" --image "$image" --admin-username "$user" --ssh-key-values "$key" \
    --nsg nsg --public-ip-sku Standard --os-disk-size-gb 64 --accelerated-networking true \
    --assign-identity '[system]' --role Contributor --scope "$rgid" \
    --custom-data "$out/cloud-init.yaml" --tags "${tags[@]}" -o none || die "vm create failed"
# every resource the group holds (NIC, IP, disk, VNet too), tagged
ids=$(az resource list -g "$group" --query '[].id' -o tsv) && [[ -n $ids ]] ||
    die "cannot list the resources in $group to tag them"
for id in $ids; do
    # the Tags API, not `az resource tag`: that one asks each provider for its
    # newest API version, which a region may not serve yet (eastus refused
    # publicIPAddresses at 2026-07-01)
    az tag update --resource-id "$id" --operation merge --tags "${tags[@]}" -o none || die "cannot tag $id"
done
az vm auto-shutdown -g "$group" -n vm --time "$off_hhmm" -o none ||
    log "WARNING: no auto-shutdown: the VM's timer and this machine's teardown remain"
ip=$(az vm show -d -g "$group" -n vm --query publicIps -o tsv) && [[ -n $ip ]] || die "no public IP"
log "VM at $ip"

# ---------------------------------------------------------------- the VM

ssh_opts=(-o StrictHostKeyChecking=accept-new -o "UserKnownHostsFile=$out/known_hosts" -o ConnectTimeout=10
    -o ServerAliveInterval=15 -o ServerAliveCountMax=4 -o BatchMode=yes)
on() { ssh "${ssh_opts[@]}" "$user@$ip" "$@"; }

wait_ssh() {
    local until=$(($(date +%s) + ssh_wait))
    while (($(date +%s) < until)); do
        on true 2>/dev/null && return 0
        sleep "$poll"
    done
    return 1
}

# check_resources: what the group holds now (every 15 minutes and at each step)
last_check=0
check_resources() {
    local n
    (($(date +%s) - last_check >= 900)) || [[ ${1:-} == now ]] || return 0
    last_check=$(date +%s)
    n=$(az resource list -g "$group" --query 'length(@)' -o tsv 2>/dev/null) || n='?'
    log "resources in $group: $n (deleted on exit)"
}

# step <name>: vm.sh <name> on the VM, under nohup (an SSH drop does not
# kill it), its log followed here, until it writes its exit code.
step() {
    local name=$1 off=0 rc chunk seen
    log "step $name"
    on "rm -f ~/step-$name.rc; nohup bash -c 'bash ~/lab/k8s/nri-shared-cpus/lab/vm.sh $name; echo \$? >~/step-$name.rc' >~/step-$name.log 2>&1 </dev/null &" ||
        die "step $name: cannot start it"
    seen=$(date +%s)
    while :; do
        (($(date -u +%s) < deadline - 900)) || die "step $name: 15 minutes to the backstop deadline; stopping"
        check_resources
        if chunk=$(on "tail -c +$((off + 1)) ~/step-$name.log 2>/dev/null"); then
            seen=$(date +%s)
            [[ -n $chunk ]] && {
                printf '%s\n' "$chunk" | sed "s/^/  $name| /" >&3
                off=$((off + $(printf '%s\n' "$chunk" | wc -c)))
            }
        fi
        rc=$(on "cat ~/step-$name.rc 2>/dev/null") && [[ -n $rc ]] && break
        # a VM that stops answering is not waited on until the deadline
        (($(date +%s) - seen < ssh_wait)) || die "step $name: no SSH to $ip for $ssh_wait s"
        sleep "$poll"
    done
    [[ $rc == 0 ]] || die "step $name failed (exit $rc)"
}

wait_ssh || die "no SSH to $ip in $ssh_wait s"
on "systemctl is-enabled ergon-lab-backstop.timer" >/dev/null || die "the VM's backstop timer is not enabled"
# the identity's role takes minutes to reach ARM: the backstop must be able
# to read the group before anything else runs
ok=0
until=$(($(date +%s) + 600))
while :; do
    on "sudo /usr/local/sbin/ergon-lab-backstop check" && { ok=1; break; }
    (($(date +%s) < until)) || break
    sleep "$poll"
done
((ok)) || die "the VM's identity cannot read $group: the deleting backstop would not work"
log "backstop armed on the VM: deletes $group at $deadline_utc UTC"
scp "${ssh_opts[@]}" -q "$out/src.tgz" "$user@$ip:src.tgz" || die "cannot upload the checkout"
on "mkdir -p ~/lab && tar -xzf ~/src.tgz -C ~/lab && cat >~/lab/lab.env" <<EOF || die "cannot unpack the checkout on the VM"
LAB_SIZE=$size
IRQ_CPU=$irq_cpu
SHARED_CPUS=$shared_cpus
EXCLUSIVE_CPUS=$exclusive_cpus
RESERVED_CPUS=$reserved_cpus
REPS=$reps
SECONDS_PER_RUN=$seconds
DELAY_SECONDS=$delay
UDP_RATE=$udp_rate
STRESS="$stress"
SCALE_SHARED_CPUS=$scale_shared_cpus
ARMS="$arms"
HELPERS=$helpers
TRACE=$trace
QUIET=$quiet
SCALE_SECONDS=$scale_seconds
SCALE_THREADS=$scale_threads
EOF
step bootstrap
log "rebooting into the isolation boot line"
boot0=$(on "cat /proc/sys/kernel/random/boot_id") && [[ -n $boot0 ]] || die "cannot read the VM's boot id"
on "sudo systemctl reboot" || true
sleep "$reboot_wait"
# the old boot can answer SSH for a while after the reboot is asked for:
# tune runs only once a new boot id answers
until=$(($(date +%s) + ssh_wait))
while :; do
    boot1=$(on "cat /proc/sys/kernel/random/boot_id" 2>/dev/null) && [[ -n $boot1 && $boot1 != "$boot0" ]] && break
    (($(date +%s) < until)) || die "no SSH to $ip in a new boot $ssh_wait s after the reboot"
    sleep "$poll"
done
step tune
step cluster
step e2e
scale_rc=0 bench_rc=0
if ((scale)); then ( step scale ) || scale_rc=1; fi
if ((bench)); then ( step bench ) || bench_rc=1; fi
step report
check_resources now
mkdir -p "$out/results"
scp "${ssh_opts[@]}" -q -r "$user@$ip:/var/lib/ergon-lab/results/." "$out/results/" || log "WARNING: results not copied"
scp "${ssh_opts[@]}" -q "$user@$ip:step-*.log" "$out/" || true
log "report: $out/results/report.md"
((scale_rc == 0)) || log "ERROR: scale had failures (results/scale.md)"
((bench_rc == 0)) || log "ERROR: bench had failures (results/failures.txt)"
((scale_rc == 0 && bench_rc == 0)) || exit 1
