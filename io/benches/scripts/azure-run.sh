#!/usr/bin/env bash
# Two-VM epoll vs io_uring benchmark (busy-spin loops only; ONLY=sensitivity
# reruns the older poll-then-sleep pair) on x86 in Azure East Asia (Hong Kong), the
# closest region to Binance (AWS Tokyo) where this subscription can run x86 v6.
#
# Creates one resource group (ergon-lab-iobench-<stamp>), runs everything, copies
# results to results/<stamp>/, and deletes the resource group on every exit path
# (success, failure, Ctrl-C, SIGTERM), then verifies it is gone.
#
# Quota note: 10 vCPUs in the region: receiver D8ds_v6 (4 Intel cores with SMT,
# MANA NIC) and sender D2ds_v6. Variants run in pairs within one NIC-polling
# regime (plus A/A controls): core 0 housekeeping, core 1 variant A, core 2
# variant B, core 3 the SQPOLL kernel thread; cores 1..3 (both hyperthreads) are
# isolated and the sibling threads stay idle.
set -euo pipefail
cd "$(dirname "$0")/.."

STAMP=${STAMP:-$(date -u +%Y%m%dT%H%M%SZ)}
LOC=${LOC:-eastasia}
RG=ergon-lab-iobench-$STAMP
IMAGE=${IMAGE:-Canonical:ubuntu-24_04-lts:server:latest}
RCV_SIZE=${RCV_SIZE:-Standard_D8ds_v6}
SND_SIZE=${SND_SIZE:-Standard_D2ds_v6}
LAB_S=${LAB_S:-360}        # replay 1x pair runs
LAB_SHORT_S=${LAB_SHORT_S:-240}
LIVE_S=${LIVE_S:-300}
PP_S=${PP_S:-30}
CAPTURE_S=${CAPTURE_S:-300}
OUT=results/$STAMP
mkdir -p "$OUT/ssh"
LOG=$OUT/run.log
exec > >(trap '' INT TERM; exec tee -a "$LOG") 2>&1
MAX_S=${MAX_S:-18000}
( sleep "$MAX_S"; echo "[watchdog] ${MAX_S}s reached, stopping"; kill -TERM $$ ) &
WATCHDOG=$!

log() { echo "[$(date -u +%H:%M:%S)] $*"; }

teardown() {
  local rc=$?
  set +e
  trap '' PIPE INT TERM
  trap - EXIT
  kill "$WATCHDOG" 2>/dev/null
  if [[ $rc -ne 0 && -n ${RCV:-} ]]; then
    log "teardown: salvaging partial results"
    ssh "${SSHO[@]}" "azureuser@$RCV" "sudo chown -R azureuser res 2>/dev/null; tar -czf res.tgz --exclude res/capture.bin --exclude 'res/lab-burst-*/records.bin' res" \
      && scp "${SSHO[@]}" "azureuser@$RCV:res.tgz" "$OUT/" >/dev/null \
      && tar -xzf "$OUT/res.tgz" -C "$OUT"
  fi
  log "teardown: deleting resource group $RG (exit code $rc)"
  # Always attempt the delete and retry: a failed `az group exists` (expired
  # token, throttling) must not skip it.
  local gone=""
  for _ in 1 2 3 4 5 6; do
    az group delete -n "$RG" --yes 2>/dev/null
    [[ $(az group exists -n "$RG" 2>/dev/null) == false ]] && { gone=1; break; }
    sleep 30
  done
  if [[ -z $gone ]]; then log "!!! RESOURCE GROUP $RG STILL EXISTS: delete it by hand"; rc=1; fi
  log "verify: group exists = $(az group exists -n "$RG")"
  log "verify: resources tagged run=$STAMP: $(az resource list --tag run="$STAMP" --query 'length(@)' -o tsv)"
  exit "$rc"
}
trap teardown EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'echo "[err] line $LINENO: \"$BASH_COMMAND\" exited $?"' ERR
set -E

KEY=$OUT/ssh/id_ed25519
[[ -f $KEY ]] || ssh-keygen -q -t ed25519 -N '' -f "$KEY" -C "ergon-lab-$STAMP"
SSHO=(-i "$KEY" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ServerAliveInterval=30 -o ServerAliveCountMax=20 -o ConnectTimeout=15 -o ConnectionAttempts=5)
MYIP=$(curl -s https://api.ipify.org)

log "existing groups before create:"; az group list --query '[].name' -o tsv
log "creating $RG in $LOC (receiver $RCV_SIZE, sender $SND_SIZE, image $IMAGE)"
TAGS=(owner=ergon-lab run="$STAMP")
az group create -n "$RG" -l "$LOC" --tags "${TAGS[@]}" -o none
az ppg create -g "$RG" -n ppg -t standard --tags "${TAGS[@]}" -o none
az network vnet create -g "$RG" -n vnet --address-prefix 10.20.0.0/16 --subnet-name s --subnet-prefix 10.20.1.0/24 --tags "${TAGS[@]}" -o none
az network nsg create -g "$RG" -n nsg --tags "${TAGS[@]}" -o none
az network nsg rule create -g "$RG" --nsg-name nsg -n ssh --priority 100 --source-address-prefixes "$MYIP/32" --destination-port-ranges 22 --access Allow --protocol Tcp -o none
for vm in rcv snd; do
  size=$RCV_SIZE; [[ $vm == snd ]] && size=$SND_SIZE
  az vm create -g "$RG" -n "$vm" --image "$IMAGE" --size "$size" --ppg ppg --vnet-name vnet --subnet s --nsg nsg \
    --accelerated-networking true --admin-username azureuser --ssh-key-values "$KEY.pub" --public-ip-sku Standard \
    --os-disk-delete-option Delete --nic-delete-option Delete --data-disk-delete-option Delete --tags "${TAGS[@]}" -o none
done
RCV=$(az vm show -d -g "$RG" -n rcv --query publicIps -o tsv)
SND=$(az vm show -d -g "$RG" -n snd --query publicIps -o tsv)
SND_PRIV=$(az vm show -d -g "$RG" -n snd --query privateIps -o tsv)
log "rcv $RCV  snd $SND (private $SND_PRIV)"
rssh() { ssh "${SSHO[@]}" "azureuser@$RCV" "$@"; }
sssh() { ssh "${SSHO[@]}" "azureuser@$SND" "$@"; }
# Copies a file between the VMs through this machine (scp, no ssh-to-ssh pipes).
relay() { # src_host src_path dst_host dst_path
  local tmp="$OUT/relay.$$"
  scp -o LogLevel=INFO "${SSHO[@]}" "azureuser@$1:$2" "$tmp" >/dev/null || { log "relay: fetch $1:$2 failed"; return 1; }
  scp -o LogLevel=INFO "${SSHO[@]}" "$tmp" "azureuser@$3:$4" >/dev/null || { log "relay: push to $3:$4 failed"; return 1; }
  rm -f "$tmp"
  log "relay: $1:$2 -> $3:$4 ok"
}
# Runs a long command on the receiver detached (nohup) and polls for its exit
# status with short ssh calls, so a dropped ssh session cannot kill the run.
# Prints the command's output; returns its exit status.
rrun() {
  local id="job$(date +%s)$RANDOM" rc=""
  rssh "mkdir -p ~/jobs; nohup bash -c $(printf '%q' "($1); echo \$? > ~/jobs/$id.rc") > ~/jobs/$id.log 2>&1 < /dev/null &"
  until [[ -n $rc ]]; do
    sleep 10
    rc=$(rssh "cat ~/jobs/$id.rc 2>/dev/null" 2>/dev/null || true)
  done
  until rssh "cat ~/jobs/$id.log"; do sleep 5; done
  log "job $id exit $rc"
  return "$rc"
}
wait_ssh() { for _ in $(seq 60); do ssh "${SSHO[@]}" "azureuser@$1" true 2>/dev/null && return 0; sleep 5; done; return 1; }
wait_ssh "$RCV"; wait_ssh "$SND"

# Receiver: core 0 housekeeping; one CPU from each of cores 1..3 for A, B and SQPOLL.
TOPO=$(rssh "lscpu -p=CPU,CORE | grep -v '^#'")
cpu_of_core() { echo "$TOPO" | awk -F, -v c="$1" '$2==c {print $1; exit}'; }
siblings_of_core() { echo "$TOPO" | awk -F, -v c="$1" '$2==c {print $1}' | paste -sd, -; }
CPU_A=$(cpu_of_core 1); CPU_B=$(cpu_of_core 2); CPU_SQ=$(cpu_of_core 3)
HK=$(siblings_of_core 0)
NCORES=$(echo "$TOPO" | awk -F, '{print $2}' | sort -un | wc -l | tr -d ' ')
ISO=$(for c in $(seq 1 $((NCORES - 1))); do siblings_of_core "$c"; done | paste -sd, -)
log "receiver topology: housekeeping $HK, isolated $ISO, A=$CPU_A B=$CPU_B SQPOLL=$CPU_SQ"
STOPO=$(sssh "lscpu -p=CPU,CORE | grep -v '^#'")
S_CPU=$(echo "$STOPO" | awk -F, 'NR==2 {print $1}'); S_HK=$(echo "$STOPO" | awk -F, 'NR==1 {print $1}')
[[ -n $S_CPU ]] || S_CPU=$S_HK

for h in "$RCV" "$SND"; do scp "${SSHO[@]}" scripts/vm-setup.sh "azureuser@$h:" >/dev/null; done
rssh "sudo bash vm-setup.sh install $HK $ISO"
sssh "sudo bash vm-setup.sh install $S_HK $S_CPU"

log "building on the receiver"
# The bench depends on ../ (ergon-io), which inherits from the workspace root
# manifest; ship exactly those files.
# `tune` is a path dependency of `ergon-io` (and its README is `include_str!`d),
# so it has to travel with the snapshot or the build fails on the VM.
( cd ../.. && COPYFILE_DISABLE=1 tar --no-xattrs --exclude target --exclude results -czf "$OLDPWD/$OUT/src.tgz" \
    Cargo.toml rust-toolchain.toml tune io )
scp "${SSHO[@]}" "$OUT/src.tgz" "azureuser@$RCV:" >/dev/null
rssh 'set -e; command -v cargo >/dev/null || (curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal >/dev/null); . ~/.cargo/env; mkdir -p ergon bin; tar -xzf src.tgz -C ergon; cd ergon/io/benches; cargo build --release --bins -q; cargo test --release -q 2>&1 | grep "test result"; cp target/release/{io_bench,replay_server,analyse} ~/bin/'
mkdir -p "$OUT/bin"
scp "${SSHO[@]}" "azureuser@$RCV:bin/*" "$OUT/bin/" >/dev/null
sssh "mkdir -p bin"; scp "${SSHO[@]}" "$OUT/bin/replay_server" "$OUT/bin/io_bench" "azureuser@$SND:bin/" >/dev/null

log "rebooting for CPU isolation"
rssh "sudo reboot" || true; sssh "sudo reboot" || true
sleep 30; wait_ssh "$RCV"; wait_ssh "$SND"
rssh "sudo bash vm-setup.sh tune" > "$OUT/env-rcv.txt"
sssh "sudo bash vm-setup.sh tune" > "$OUT/env-snd.txt"
log "receiver isolated CPUs: $(rssh cat /sys/devices/system/cpu/isolated)"
rssh "mkdir -p res; ~/bin/io_bench clockcost" | tee "$OUT/clockcost.txt"

RB="sudo /home/azureuser/bin/io_bench"
# Binance geo-restricts some regions; probe from the receiver and fall back to
# the public market-data host (spot only) if needed.
SPOT_CODE=$(rssh "curl -s -o /dev/null -w '%{http_code}' https://api.binance.com/api/v3/ping" || echo 000)
FUT_CODE=$(rssh "curl -s -o /dev/null -w '%{http_code}' https://fapi.binance.com/fapi/v1/ping" || echo 000)
log "binance reachability: spot api $SPOT_CODE, futures api $FUT_CODE"
BIN_ARGS=""
if [[ $SPOT_CODE != 200 ]]; then BIN_ARGS="--spot-host data-stream.binance.vision --spot-port 443"; fi
if [[ $FUT_CODE != 200 ]]; then BIN_ARGS="$BIN_ARGS --fut-public '' --fut-market ''"; fi
echo "binance_args=$BIN_ARGS" > "$OUT/binance-args.txt"

# Every variant busy-spins and returns to the application each iteration; none
# waits in the kernel (wait_us = 0, so uring-napi makes one poll pass per enter).
#   spin, interrupt-driven:          epoll-spin vs uring-spin
#   NIC polled vs interrupt, 1 core: recv-busy  vs uring-spin
#   NIC polled, io_uring +1 core:    recv-busy  vs uring-sqpoll
#   controls (A/A):                  epoll-spin vs epoll-spin, recv-busy vs recv-busy
# uring-napi (wait 0) sleeps on every idle enter (schedule() on an expired
# hrtimer), so it is only in the smoke and ping-pong runs, as evidence.
PAIRS=("epoll-spin,uring-spin" "recv-busy,uring-spin" "recv-busy,uring-sqpoll")
LIVE_PAIRS=("epoll-spin,uring-spin" "recv-busy,uring-spin" "recv-busy,uring-sqpoll")
AAS=("epoll-spin,epoll-spin" "recv-busy,recv-busy")
# busy_poll 1 µs: recv-busy and the SQPOLL thread only need it non-zero (one
# NAPI pass per call); uring-napi registers budget 0 when wait_us is 0.
SPIN="--busy-poll ${BUSY_US:-1} --wait-us 0"
pair_args() { echo "--variants $1 --cpus $CPU_A,$CPU_B --sqpoll-cpu $CPU_SQ $SPIN"; }
swap() { echo "${1#*,},${1%,*}"; }
tag() { echo "${1/,/-vs-}"; }

log "smoke: 20 s live, every pair"
for p in "${PAIRS[@]}" "recv-busy,uring-napi"; do
  rrun "$RB binance $BIN_ARGS $(pair_args "$p") --duration 20 --out res/smoke-live-$(tag "$p")" | grep -E 'loop\.[01]\.(variant|messages|syscalls|napi|vol_ctx|gap_max)|busy_poll_rx'
done

log "capture ${CAPTURE_S}s of live traffic for the lab replay"
rrun "$RB capture $BIN_ARGS --cpu $CPU_A --duration $CAPTURE_S --out res/capture.bin"
relay "$RCV" res/capture.bin "$SND" capture.bin

lab_run() { # name, client args, server mode args, duration
  local name=$1 cargs=$2 sargs=$3 dur=$4
  sssh "sudo pkill -x replay_server || true; nohup sudo bin/replay_server --listen 0.0.0.0:9443 --cert-out cert.pem --clients 2 --cpu $S_CPU $sargs --duration $((dur + 60)) > srv-$name.log 2>&1 < /dev/null &"
  sleep 3
  relay "$SND" cert.pem "$RCV" cert.pem
  log "lab $name ($dur s)"
  rrun "$RB lab --server $SND_PRIV:9443 --ca cert.pem $cargs --duration $dur --out res/$name" > "$OUT/job-$name.log"
  sleep 2
  sssh "cat srv-$name.log" | tail -2
}

REPLAY="--mode replay --capture capture.bin --speed 1"
if [[ ${ONLY:-} == sensitivity ]]; then
  # Busy-poll budget equal to the wait timeout, so both kernel busy-poll-wait
  # variants poll continuously instead of polling part of each wait and sleeping.
  SENS="--busy-poll ${SENS_US:-200} --wait-us ${SENS_US:-200}"
  for order in "epoll-busy,uring-napi" "uring-napi,epoll-busy"; do
    lab_run "sens-lab-replay1x-$(tag "$order")" "$(pair_args "$order") $SENS" "$REPLAY" "$LAB_S"
  done
  lab_run "sens-lab-replay1x-aa-epoll-busy" "$(pair_args epoll-busy,epoll-busy) $SENS" "$REPLAY" "$LAB_S"
  sssh "sudo pkill -x replay_server || true; nohup sudo bin/replay_server --listen 0.0.0.0:9444 --cert-out cert-echo.pem --mode echo --cpu $S_CPU > srv-echo.log 2>&1 < /dev/null &"
  sleep 3; relay "$SND" cert-echo.pem "$RCV" cert-echo.pem
  for rate in 1000 10000; do
    for v in epoll-busy uring-napi; do
      rrun "$RB pingpong --server $SND_PRIV:9444 --ca cert-echo.pem --variant $v --cpu $CPU_A $SENS --rate $rate --duration $PP_S --out res/sens-pp-$v-$rate" > "$OUT/job-sens-pp-$v-$rate.log"
    done
  done
  sssh "sudo pkill -x replay_server || true"
else
BURST="--mode burst --capture capture.bin --burst 50 --gap-us 1000"
BURST_CAP="--capacity $(( 50000 * (LAB_SHORT_S + 30) ))"
for p in "${PAIRS[@]}"; do
  for order in "$p" "$(swap "$p")"; do
    lab_run "lab-replay1x-$(tag "$order")" "$(pair_args "$order")" "$REPLAY" "$LAB_S"
    lab_run "lab-burst-$(tag "$order")" "$(pair_args "$order") $BURST_CAP" "$BURST" "$LAB_SHORT_S"
  done
done
for p in "${AAS[@]}"; do
  lab_run "lab-replay1x-aa-${p%,*}" "$(pair_args "$p")" "$REPLAY" "$LAB_S"
  lab_run "lab-burst-aa-${p%,*}" "$(pair_args "$p") $BURST_CAP" "$BURST" "$LAB_SHORT_S"
done

log "ping-pong sweep (variants interleaved within each rate)"
sssh "sudo pkill -x replay_server || true; nohup sudo bin/replay_server --listen 0.0.0.0:9444 --cert-out cert-echo.pem --mode echo --cpu $S_CPU > srv-echo.log 2>&1 < /dev/null &"
sleep 3; relay "$SND" cert-echo.pem "$RCV" cert-echo.pem
for rate in 100 1000 10000 0; do
  for v in epoll-spin uring-spin recv-busy uring-napi uring-sqpoll; do
    rrun "$RB pingpong --server $SND_PRIV:9444 --ca cert-echo.pem --variant $v --cpu $CPU_A --sqpoll-cpu $CPU_SQ $SPIN --rate $rate --duration $PP_S --out res/pp-$v-$rate" > "$OUT/job-pp-$v-$rate.log"
  done
done
sssh "sudo pkill -x replay_server || true"

log "live Binance pairs"
for p in "${LIVE_PAIRS[@]}"; do
  for order in "$p" "$(swap "$p")"; do
    rrun "$RB binance $BIN_ARGS $(pair_args "$order") --duration $LIVE_S --out res/live-$(tag "$order")" > "$OUT/job-live-$(tag "$order").log"
  done
done
rrun "$RB binance $BIN_ARGS $(pair_args recv-busy,recv-busy) --duration $LIVE_S --out res/live-aa-recv-busy" > "$OUT/job-live-aa.log"

if rssh "command -v perf >/dev/null && sudo perf --version" >/dev/null 2>&1; then
  log "syscall census (perf trace -s, 10 s, separate from the timed runs)"
  for p in "${LIVE_PAIRS[@]}"; do
    rssh "$RB binance $BIN_ARGS $(pair_args "$p") --duration 15 --out res/perf-$(tag "$p") > /dev/null & sleep 3; sudo perf trace -s -p \$(pgrep -n io_bench) -- sleep 10 2>&1 | tail -60" > "$OUT/perf-trace-$(tag "$p").txt" || true
  done
fi

fi

log "analysing on the receiver (burst raw records stay there)"
rrun "sudo chown -R azureuser ~/res; cd ~/res && ~/bin/analyse \$(ls -d lab-* live-* pp-* sens-* 2>/dev/null) > tables.md 2> analyse.err; for d in */; do [ -f \$d/meta.txt ] && ~/bin/analyse \$d > \$d/tables.md 2>/dev/null; done; true" > /dev/null
log "collecting results"
rssh "sudo chown -R azureuser res; tar -czf res.tgz --exclude res/capture.bin --exclude 'res/lab-burst-*/records.bin' res"
scp "${SSHO[@]}" "azureuser@$RCV:res.tgz" "$OUT/" >/dev/null
tar -xzf "$OUT/res.tgz" -C "$OUT"
log "done; results in $OUT"
