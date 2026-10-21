#!/usr/bin/env bash
# Confirmation run on the final ergon-io library (East Asia, same VM sizes as the
# main busy-spin run): WebSocket ping-pong for the four busy-spin backends, the
# HTTP clients (ergon-io against libcurl distro and source builds), and the
# reconnect test (worst poll per connect, inline against background).
# ONLY=client runs just the HttpClient comparison (native against libcurl,
# both drive modes, distro and source libcurl, plus direct baselines) and the
# reconnect test.
#
# One resource group per run (ergon-lab-ioconfirm-<stamp>), deleted on every
# exit path and verified gone. Results go to results/confirm-<stamp>/.
set -euo pipefail
cd "$(dirname "$0")/.."

STAMP=${STAMP:-$(date -u +%Y%m%dT%H%M%SZ)}
LOC=${LOC:-eastasia}
RG=ergon-lab-ioconfirm-$STAMP
IMAGE=${IMAGE:-Canonical:ubuntu-24_04-lts:server:latest}
RCV_SIZE=${RCV_SIZE:-Standard_D8ds_v6}
SND_SIZE=${SND_SIZE:-Standard_D2ds_v6}
RUN_S=${RUN_S:-30}
ROUNDS=${ROUNDS:-2}
CURL_SRC=${CURL_SRC:-8.23.0}
OUT=results/confirm-$STAMP
mkdir -p "$OUT/ssh"
# Snapshot the source now: later edits in the working tree must not reach this run.
# `tune` is a path dependency of `ergon-io` (and its README is `include_str!`d),
# so it has to travel with the snapshot or the build fails on the VM.
( cd ../.. && COPYFILE_DISABLE=1 tar --no-xattrs --exclude target --exclude results -czf "$OLDPWD/$OUT/src.tgz" \
    Cargo.toml rust-toolchain.toml tune io )
LOG=$OUT/run.log
exec > >(trap '' INT TERM; exec tee -a "$LOG") 2>&1
MAX_S=${MAX_S:-7200}
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
    ssh "${SSHO[@]}" "azureuser@$RCV" "tar -czf res.tgz res build.log 2>/dev/null" \
      && scp "${SSHO[@]}" "azureuser@$RCV:res.tgz" "$OUT/" >/dev/null \
      && tar -xzf "$OUT/res.tgz" -C "$OUT"
  fi
  log "teardown: deleting resource group $RG (exit code $rc)"
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
log "creating $RG in $LOC (receiver $RCV_SIZE, sender $SND_SIZE)"
TAGS=(owner=ergon-lab run="$STAMP")
az group create -n "$RG" -l "$LOC" --tags "${TAGS[@]}" -o none
az ppg create -g "$RG" -n ppg -t standard --tags "${TAGS[@]}" -o none
az network vnet create -g "$RG" -n vnet --address-prefix 10.22.0.0/16 --subnet-name s --subnet-prefix 10.22.1.0/24 --tags "${TAGS[@]}" -o none
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
relay() {
  local tmp="$OUT/relay.$$"
  scp "${SSHO[@]}" "azureuser@$1:$2" "$tmp" >/dev/null && scp "${SSHO[@]}" "$tmp" "azureuser@$3:$4" >/dev/null && rm -f "$tmp"
}
# Detached remote job polled with short ssh calls (a dropped session cannot kill it).
rrun() {
  local id="job$(date +%s)$RANDOM" rc=""
  rssh "mkdir -p ~/jobs; nohup bash -c $(printf '%q' "($1); echo \$? > ~/jobs/$id.rc") > ~/jobs/$id.log 2>&1 < /dev/null &"
  until [[ -n $rc ]]; do
    sleep 5
    rc=$(rssh "cat ~/jobs/$id.rc 2>/dev/null" 2>/dev/null || true)
  done
  until rssh "cat ~/jobs/$id.log"; do sleep 5; done
  log "job $id exit $rc"
  return "$rc"
}
wait_ssh() { for _ in $(seq 60); do ssh "${SSHO[@]}" "azureuser@$1" true 2>/dev/null && return 0; sleep 5; done; return 1; }
wait_ssh "$RCV"; wait_ssh "$SND"

TOPO=$(rssh "lscpu -p=CPU,CORE | grep -v '^#'")
cpu_of_core() { echo "$TOPO" | awk -F, -v c="$1" '$2==c {print $1; exit}'; }
siblings_of_core() { echo "$TOPO" | awk -F, -v c="$1" '$2==c {print $1}' | paste -sd, -; }
CPU_A=$(cpu_of_core 1); CPU_SQ=$(cpu_of_core 3)
HK=$(siblings_of_core 0)
NCORES=$(echo "$TOPO" | awk -F, '{print $2}' | sort -un | wc -l | tr -d ' ')
ISO=$(for c in $(seq 1 $((NCORES - 1))); do siblings_of_core "$c"; done | paste -sd, -)
STOPO=$(sssh "lscpu -p=CPU,CORE | grep -v '^#'")
S_CPU=$(echo "$STOPO" | awk -F, 'NR==2 {print $1}'); S_HK=$(echo "$STOPO" | awk -F, 'NR==1 {print $1}')
[[ -n $S_CPU ]] || S_CPU=$S_HK
log "receiver: housekeeping $HK, isolated $ISO, client CPU $CPU_A, SQPOLL $CPU_SQ"

for h in "$RCV" "$SND"; do scp "${SSHO[@]}" scripts/vm-setup.sh "azureuser@$h:" >/dev/null; done
rssh "sudo bash vm-setup.sh install $HK $ISO"
sssh "sudo bash vm-setup.sh install $S_HK $S_CPU"

log "building on the receiver (ergon-io, curl distro / source $CURL_SRC)"
scp "${SSHO[@]}" "$OUT/src.tgz" "azureuser@$RCV:" >/dev/null
rrun "set -e; exec > build.log 2>&1
  sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq build-essential pkg-config libssl-dev libcurl4-openssl-dev libc-ares-dev cmake perl xz-utils zlib1g-dev >/dev/null
  command -v cargo >/dev/null || (curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal >/dev/null); . ~/.cargo/env
  mkdir -p ergon bin; tar -xzf src.tgz -C ergon; cd ergon/io/benches
  cargo build --release --bins -q
  cp target/release/{http_bench,io_bench,replay_server,analyse} ~/bin/
  cargo build --release --bin http_bench --features curl --target-dir t-distro -q
  cp t-distro/release/http_bench ~/bin/http_bench_curl_distro
  cd ~; (curl -sSfLO https://curl.se/download/curl-$CURL_SRC.tar.xz || curl -sSfLO https://curl.se/download/curl-8.22.0.tar.xz)
  tar -xJf curl-*.tar.xz; cd curl-*/
  cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_FLAGS='-O3 -march=native' -DCMAKE_INSTALL_PREFIX=/opt/curl \
    -DENABLE_ARES=ON -DENABLE_THREADED_RESOLVER=OFF -DHTTP_ONLY=ON -DCURL_USE_OPENSSL=ON -DUSE_NGHTTP2=OFF \
    -DCURL_USE_LIBPSL=OFF -DCURL_USE_LIBSSH2=OFF -DCURL_BROTLI=OFF -DCURL_ZSTD=OFF -DUSE_LIBIDN2=OFF \
    -DBUILD_CURL_EXE=OFF -DBUILD_SHARED_LIBS=ON -DBUILD_TESTING=OFF > /dev/null
  cmake --build build -j4 > /dev/null; sudo cmake --install build > /dev/null
  cd ~/ergon/io/benches
  PKG_CONFIG_PATH=/opt/curl/lib/pkgconfig cargo build --release --bin http_bench --features curl --target-dir t-source -q
  cp t-source/release/http_bench ~/bin/http_bench_curl_source
  ldd ~/bin/http_bench_curl_distro | grep curl; LD_LIBRARY_PATH=/opt/curl/lib ldd ~/bin/http_bench_curl_source | grep curl" \
  || { rssh "tail -40 build.log"; exit 1; }
rssh "cat build.log | tail -5"
scp "${SSHO[@]}" "azureuser@$RCV:bin/replay_server" "$OUT/replay_server" >/dev/null
sssh "mkdir -p bin"; scp "${SSHO[@]}" "$OUT/replay_server" "azureuser@$SND:bin/" >/dev/null

log "rebooting for CPU isolation"
rssh "sudo reboot" || true; sssh "sudo reboot" || true
sleep 30; wait_ssh "$RCV"; wait_ssh "$SND"
rssh "sudo bash vm-setup.sh tune" > "$OUT/env-rcv.txt"
sssh "sudo bash vm-setup.sh tune" > "$OUT/env-snd.txt"
log "receiver isolated CPUs: $(rssh cat /sys/devices/system/cpu/isolated)"

rssh "mkdir -p res"
SPIN="--busy-poll 1 --wait-us 0"
if [[ ${ONLY:-} == client ]]; then
  log "HttpClient: native against libcurl through one API"
  sssh "sudo pkill -x replay_server || true; nohup sudo bin/replay_server --listen 0.0.0.0:9445 --cert-out cert-http.pem --mode http --cpu $S_CPU > srv-http.log 2>&1 < /dev/null &"
  sleep 3; relay "$SND" cert-http.pem "$RCV" cert-http.pem
  COMMON="--server $SND_PRIV:9445 --ca cert-http.pem --sni lab.local --cpu $CPU_A --duration $RUN_S"
  declare -A CLIENT=(
    [client-native-recv-busy]="bin/http_bench --client client --impl native --backend recv-busy"
    [client-native-uring-sqpoll]="bin/http_bench --client client --impl native --backend uring-sqpoll --sqpoll-cpu $CPU_SQ"
    [direct-ergon-recv-busy]="bin/http_bench --client ergon --backend recv-busy"
    [client-curl-perform-distro]="bin/http_bench_curl_distro --client client --impl curl --curl-drive perform --curl-label distro"
    [client-curl-socket-distro]="bin/http_bench_curl_distro --client client --impl curl --curl-drive socket-action --curl-label distro"
    [client-curl-perform-source]="env LD_LIBRARY_PATH=/opt/curl/lib bin/http_bench_curl_source --client client --impl curl --curl-drive perform --curl-label source"
    [client-curl-socket-source]="env LD_LIBRARY_PATH=/opt/curl/lib bin/http_bench_curl_source --client client --impl curl --curl-drive socket-action --curl-label source"
    [direct-curl-distro]="bin/http_bench_curl_distro --client curl --curl-label distro"
  )
  for round in 1 2; do
    for rate in 1000 0; do
      for c in client-native-recv-busy client-native-uring-sqpoll direct-ergon-recv-busy client-curl-perform-distro client-curl-socket-distro client-curl-perform-source client-curl-socket-source direct-curl-distro; do
        rrun "${CLIENT[$c]} $COMMON --rate $rate --out res/http-$c-$rate-r$round" > "$OUT/job-http-$c-$rate-r$round.log" || log "client $c failed"
      done
    done
  done
  log "reconnect test after the SQPOLL affinity fix (no sqpoll_cpu given)"
  for b in uring-sqpoll recv-busy; do
    for mode in inline background; do
      rrun "bin/http_bench --connects 50 --connect $mode --backend $b --server $SND_PRIV:9445 --ca cert-http.pem --sni lab.local --cpu $CPU_A --out res/connect-$b-$mode" | tail -2
    done
  done
  sssh "sudo pkill -x replay_server || true"
else
RB="sudo /home/azureuser/bin/io_bench"

log "WebSocket ping-pong on the library loops"
sssh "sudo pkill -x replay_server || true; nohup sudo bin/replay_server --listen 0.0.0.0:9444 --cert-out cert-echo.pem --mode echo --cpu $S_CPU > srv-echo.log 2>&1 < /dev/null &"
sleep 3; relay "$SND" cert-echo.pem "$RCV" cert-echo.pem
for rate in 100 1000 0; do
  for v in epoll-spin recv-busy uring-spin uring-sqpoll; do
    rrun "$RB pingpong --server $SND_PRIV:9444 --ca cert-echo.pem --variant $v --cpu $CPU_A --sqpoll-cpu $CPU_SQ $SPIN --rate $rate --duration $RUN_S --out res/pp-$v-$rate" > "$OUT/job-pp-$v-$rate.log"
  done
done

log "HTTP clients"
sssh "sudo pkill -x replay_server || true; nohup sudo bin/replay_server --listen 0.0.0.0:9445 --cert-out cert-http.pem --mode http --cpu $S_CPU > srv-http.log 2>&1 < /dev/null &"
sleep 3; relay "$SND" cert-http.pem "$RCV" cert-http.pem
COMMON="--server $SND_PRIV:9445 --ca cert-http.pem --sni lab.local --cpu $CPU_A --duration $RUN_S"
declare -A CLIENT=(
  [ergon-epoll-spin]="bin/http_bench --client ergon --backend epoll-spin"
  [ergon-recv-busy]="bin/http_bench --client ergon --backend recv-busy"
  [ergon-uring-spin]="bin/http_bench --client ergon --backend uring-spin"
  [ergon-uring-sqpoll]="bin/http_bench --client ergon --backend uring-sqpoll --sqpoll-cpu $CPU_SQ"
  [curl-distro]="bin/http_bench_curl_distro --client curl --curl-label distro"
  [curl-source]="env LD_LIBRARY_PATH=/opt/curl/lib bin/http_bench_curl_source --client curl --curl-label source"
)
for rate in 1000 0; do
  for c in ergon-epoll-spin ergon-recv-busy ergon-uring-spin ergon-uring-sqpoll curl-distro curl-source; do
    rrun "${CLIENT[$c]} $COMMON --rate $rate --out res/http-$c-$rate" > "$OUT/job-http-$c-$rate.log" || log "client $c failed"
  done
done

log "reconnect test: worst poll per connect"
for b in recv-busy uring-sqpoll; do
  for mode in inline background; do
    rrun "bin/http_bench --connects 50 --connect $mode --backend $b --server $SND_PRIV:9445 --ca cert-http.pem --sni lab.local --cpu $CPU_A --sqpoll-cpu $CPU_SQ --out res/connect-$b-$mode" | tail -2
  done
done
sssh "sudo pkill -x replay_server || true"
fi

log "analysing on the receiver"
rrun "sudo chown -R azureuser ~/res; cd ~/res && ~/bin/analyse \$(ls -d pp-* http-* 2>/dev/null) > tables.md 2> analyse.err; true" > /dev/null
rssh "sudo chown -R azureuser res; tar -czf res.tgz res build.log"
scp "${SSHO[@]}" "azureuser@$RCV:res.tgz" "$OUT/" >/dev/null
tar -xzf "$OUT/res.tgz" -C "$OUT"
log "done; results in $OUT"
