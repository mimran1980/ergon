#!/usr/bin/env bash
# The lab's k3s nodes as Azure VMs, one per region, all in one resource group so
# one delete removes everything that costs money. The regions' networks are
# peered: nodes, k3s and Aeron talk on private addresses. Only ssh, the
# Kubernetes API and the NodePorts are open, and only to this machine's address.
#
#   scripts/azure.sh up       create the group, networks and VMs, then provision
#                             k3s (scripts/k3s.sh); print the .env lines. Run it
#                             again after a failure: what exists is kept
#   scripts/azure.sh down     delete the group and everything in it, after
#                             confirmation, and check the subscription is empty
#   scripts/azure.sh stop     deallocate every VM: no compute charge, the cluster
#                             kept (disks and addresses still cost a few cents an hour)
#   scripts/azure.sh start    start them again
#   scripts/azure.sh status   every resource group and resource in the subscription
#
# Settings, usually in samples/clickhouse/.env (gitignored):
#   LAB_AZ_GROUP    resource group (default ergon-lab)
#   LAB_AZ_REGIONS  azure-region:lab-region[:vm-size] per node, the k3s server first
#                   (default: japaneast:an1 southeastasia:as1 uksouth:ew2 eastus:us1).
#                   A trial subscription may not get every size in every region:
#                   `az vm list-skus -l <region>` shows which it can use.
#   LAB_AZ_SIZE     VM size where a node names none (default Standard_D4s_v5:
#                   4 dedicated vCPUs, 16 GiB)
#   LAB_AZ_DISK_GB  OS disk size (default 64)
set -euo pipefail

export PYTHONWARNINGS=ignore::SyntaxWarning

# az 2.90 on Python 3.14 sometimes dies importing `requests` on two threads at
# once (`_DeadlockError`) before it sends anything; the same call then succeeds.
# Only that failure is retried: any other is the real answer.
az_bin=$(type -P az) || { echo "az is not on PATH" >&2; exit 1; }
az() {
    local err rc attempt
    err=$(mktemp)
    for attempt in 1 2 3 4 5; do
        # By path, not `command az`: macOS's bash 3.2 execs a command run
        # through `command` in place of a background subshell, so `up`'s
        # nodes (`node ... &`) ended at their first failing az, silently.
        if "$az_bin" "$@" 2>"$err"; then rc=0; else rc=$?; fi
        if ((rc == 0)); then
            cat "$err" >&2
            rm -f "$err"
            return 0
        fi
        grep -q _DeadlockError "$err" || break
        echo "az: import deadlock (attempt $attempt), retrying: az $1 $2" >&2
    done
    cat "$err" >&2
    rm -f "$err"
    return "$rc"
}

group=${LAB_AZ_GROUP:-ergon-lab}
read -ra pairs <<<"${LAB_AZ_REGIONS:-japaneast:an1 southeastasia:as1 uksouth:ew2 eastus:us1}"
size=${LAB_AZ_SIZE:-Standard_D4s_v5}
disk_gb=${LAB_AZ_DISK_GB:-64}
image=Debian:debian-13:13-gen2:latest

region() { cut -d: -f1 <<<"${pairs[$1]}"; }
label() { cut -d: -f2 <<<"${pairs[$1]}"; }
vm_size() { local s; s=$(cut -s -d: -f3 <<<"${pairs[$1]}"); echo "${s:-$size}"; }

cloud_init() {
    cat <<EOF
#cloud-config
runcmd:
  - install -d -o $(id -un) -g $(id -un) /srv/ergon
  - ln -sfn /srv/ergon/samples/clickhouse /lab
EOF
}

# One region's network, firewall and VM.
node() {
    local k=$1 key=$2 mine=$3 init=$4
    local region label name
    region=$(region "$k") label=$(label "$k") name=lab-$(label "$k")
    if az vm show -g "$group" -n "$name" -o none 2>/dev/null; then return; fi
    az network nsg create -g "$group" -n "$name-nsg" -l "$region" -o none
    az network nsg rule create -g "$group" --nsg-name "$name-nsg" -n from-operator --priority 100 \
        --source-address-prefixes "$mine" --destination-port-ranges 22 6443 30000-32767 \
        --protocol Tcp --access Allow -o none
    az network vnet create -g "$group" -n "$name-vnet" -l "$region" \
        --address-prefixes "10.$((k + 1)).0.0/16" --subnet-name nodes \
        --subnet-prefixes "10.$((k + 1)).0.0/24" --network-security-group "$name-nsg" -o none
    az vm create -g "$group" -n "$name" -l "$region" --size "$(vm_size "$k")" --image "$image" \
        --admin-username "$(id -un)" --ssh-key-values "$key" \
        --vnet-name "$name-vnet" --subnet nodes --nsg "" \
        --public-ip-address "$name-ip" --public-ip-sku Standard \
        --accelerated-networking true --os-disk-size-gb "$disk_gb" --storage-sku Premium_LRS \
        --os-disk-delete-option Delete --nic-delete-option Delete \
        --custom-data "$init" -o none
}

peer() {
    az network vnet peering show -g "$group" -n "$1-to-$2" --vnet-name "$1-vnet" -o none 2>/dev/null \
        || az network vnet peering create -g "$group" -n "$1-to-$2" --vnet-name "$1-vnet" \
            --remote-vnet "$2-vnet" --allow-vnet-access -o none
}

up() {
    local key_file=$HOME/.ssh/id_ed25519.pub mine init k i j a b
    [[ -f $key_file ]] || key_file=$HOME/.ssh/id_rsa.pub
    mine=$(curl -fsS https://api.ipify.org)/32
    init=$(mktemp)
    trap 'rm -f "$init"' RETURN
    cloud_init > "$init"
    az group create -n "$group" -l "$(region 0)" -o none
    local pids=()
    for k in "${!pairs[@]}"; do
        node "$k" "$(cat "$key_file")" "$mine" "$init" &
        pids+=($!)
    done
    local failed=0
    for k in "${pids[@]}"; do wait "$k" || failed=1; done
    ((failed == 0)) || { echo "a node failed: fix it, then run up again (what exists is kept)" >&2; exit 1; }
    for ((i = 0; i < ${#pairs[@]}; i++)); do
        for ((j = i + 1; j < ${#pairs[@]}; j++)); do
            a=lab-$(label "$i") b=lab-$(label "$j")
            peer "$a" "$b"
            peer "$b" "$a"
        done
    done
    local public=() private=() labels=()
    for k in "${!pairs[@]}"; do
        public+=("$(az vm show -d -g "$group" -n "lab-$(label "$k")" --query publicIps -o tsv)")
        private+=("$(az vm show -d -g "$group" -n "lab-$(label "$k")" --query privateIps -o tsv)")
        labels+=("$(label "$k")")
    done
    LAB_VM_IPS="${public[*]}" LAB_VM_NODE_IPS="${private[*]}" LAB_VM_REGIONS="${labels[*]}" \
        "$(dirname "$0")/k3s.sh" provision
    echo "Up. Put these in .env:"
    echo "LAB_VM_IPS=\"${public[*]}\""
    echo "LAB_VM_NODE_IPS=\"${private[*]}\""
    echo "LAB_VM_REGIONS=\"${labels[*]}\""
}

down() {
    if [[ $(az group exists -n "$group") == true ]]; then
        echo "This deletes resource group $group and everything in it:"
        az resource list -g "$group" --query '[].[type, name, location]' -o tsv | sed 's/^/  /'
        read -rp 'Type "destroy" to continue: ' answer
        [[ $answer == destroy ]] || { echo 'Nothing deleted.'; exit 1; }
        az group delete -n "$group" --yes
    fi
    # Azure makes NetworkWatcherRG with a watcher per region on the first
    # network there; delete it while it holds nothing else.
    if [[ $(az group exists -n NetworkWatcherRG) == true ]] \
        && [[ $(az resource list -g NetworkWatcherRG \
            --query "length([?type!='Microsoft.Network/networkWatchers'])") == 0 ]]; then
        az group delete -n NetworkWatcherRG --yes
    fi
    status
    if [[ $(az resource list --query 'length(@)') != 0 ]]; then
        echo "resources remain in the subscription: see above" >&2
        exit 1
    fi
    echo "Subscription empty: nothing left to bill."
}

vms() { az vm list -g "$group" --query '[].id' -o tsv; }

stop() {
    local ids
    ids=$(vms)
    [[ -z $ids ]] || az vm deallocate --ids $ids -o none
    az vm list -g "$group" -d --query '[].[name, powerState]' -o tsv | sed 's/^/  /'
}

start() {
    local ids
    ids=$(vms)
    [[ -z $ids ]] || az vm start --ids $ids -o none
    az vm list -g "$group" -d --query '[].[name, powerState, publicIps]' -o tsv | sed 's/^/  /'
}

status() {
    echo "resource groups:"
    az group list --query '[].[name, location]' -o tsv | sed 's/^/  /'
    echo "resources:"
    az resource list --query '[].[resourceGroup, type, name]' -o tsv | sed 's/^/  /'
}

case "${1:-}" in
    up | down | stop | start | status) "$1" ;;
    *) echo "usage: $0 up|down|stop|start|status" >&2; exit 2 ;;
esac
