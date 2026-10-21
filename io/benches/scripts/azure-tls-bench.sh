#!/usr/bin/env bash
# CPU micro-benchmarks on the same VM size as the receiver in azure-run.sh: TLS
# record decryption (tls_bench) and request signing (sign_bench). SKIP_TLS=1
# runs only sign_bench. One resource group, deleted on every exit path and
# verified gone.
set -euo pipefail
cd "$(dirname "$0")/.."

STAMP=${STAMP:-$(date -u +%Y%m%dT%H%M%SZ)}
LOC=${LOC:-eastasia}
RG=ergon-lab-tlsbench-$STAMP
IMAGE=${IMAGE:-Canonical:ubuntu-24_04-lts:server:latest}
SIZE=${SIZE:-Standard_D8ds_v6}
OUT=results/tls-$STAMP
mkdir -p "$OUT/ssh"
# Snapshot the source now: later edits in the working tree must not reach this run.
# `tune` is a path dependency of `ergon-io` (and its README is `include_str!`d),
# so it has to travel with the snapshot or the build fails on the VM.
( cd ../.. && COPYFILE_DISABLE=1 tar --no-xattrs --exclude target --exclude results -czf "$OLDPWD/$OUT/src.tgz" \
    Cargo.toml rust-toolchain.toml tune io )
exec > >(trap '' INT TERM; exec tee -a "$OUT/run.log") 2>&1
log() { echo "[$(date -u +%H:%M:%S)] $*"; }

( sleep "${MAX_S:-3600}"; echo "[watchdog] stopping"; kill -TERM $$ ) &
WATCHDOG=$!
teardown() {
  local rc=$?
  set +e
  trap '' PIPE INT TERM
  trap - EXIT
  kill "$WATCHDOG" 2>/dev/null
  log "teardown: deleting $RG (exit code $rc)"
  # Always attempt the delete and retry: a failed `az group exists` must not skip it.
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

KEY=$OUT/ssh/id_ed25519
[[ -f $KEY ]] || ssh-keygen -q -t ed25519 -N '' -f "$KEY" -C "ergon-lab-$STAMP"
SSHO=(-i "$KEY" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ServerAliveInterval=30 -o ServerAliveCountMax=20 -o ConnectTimeout=15 -o ConnectionAttempts=5)
MYIP=$(curl -s https://api.ipify.org)
log "existing groups:"; az group list --query '[].name' -o tsv
TAGS=(owner=ergon-lab run="$STAMP")
az group create -n "$RG" -l "$LOC" --tags "${TAGS[@]}" -o none
az network nsg create -g "$RG" -n nsg --tags "${TAGS[@]}" -o none
az network nsg rule create -g "$RG" --nsg-name nsg -n ssh --priority 100 --source-address-prefixes "$MYIP/32" --destination-port-ranges 22 --access Allow --protocol Tcp -o none
az vm create -g "$RG" -n tls --image "$IMAGE" --size "$SIZE" --nsg nsg --accelerated-networking true \
  --admin-username azureuser --ssh-key-values "$KEY.pub" --public-ip-sku Standard \
  --os-disk-delete-option Delete --nic-delete-option Delete --tags "${TAGS[@]}" -o none
IP=$(az vm show -d -g "$RG" -n tls --query publicIps -o tsv)
r() { ssh "${SSHO[@]}" "azureuser@$IP" "$@"; }
for _ in $(seq 60); do r true 2>/dev/null && break; sleep 5; done
scp "${SSHO[@]}" "$OUT/src.tgz" "azureuser@$IP:" >/dev/null
log "building"
r 'set -e; sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq; sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq build-essential pkg-config libssl-dev cmake >/dev/null; curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal >/dev/null; . ~/.cargo/env; mkdir -p ergon; tar -xzf src.tgz -C ergon; cd ergon/io/benches; cargo build --release --bin tls_bench --bin sign_bench --features aws-lc,openssl -q'
r "uname -a; lscpu | grep -E 'Model name|^CPU\(s\)'; openssl version" | tee "$OUT/env.txt"
if [[ -z ${SKIP_TLS:-} ]]; then
  log "running tls_bench pinned to CPU 2 (sibling 3 idle)"
  r "cd ergon/io/benches && taskset -c 2 target/release/tls_bench --sizes 200,1000,4000,16000 --mib 64 --reps 5" | tee "$OUT/tls_bench.md"
fi
log "running sign_bench pinned to CPU 2"
r "cd ergon/io/benches && taskset -c 2 target/release/sign_bench --iters 200000 --reps 7" | tee "$OUT/sign_bench.md"
log "done"
