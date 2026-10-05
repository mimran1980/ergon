#!/usr/bin/env bash
# k3s on the lab's VMs (scripts/azure.sh makes them): the first node is the
# k3s server and the place to build, test, benchmark and run `just`; the
# others join it as agents, each labelled with its region.
#
#   scripts/k3s.sh provision  install k3s and the first node's tools, write
#                             ~/.kube/lab-vms.yaml; safe to repeat after a failure
#   scripts/k3s.sh stop       stop k3s and every pod on every node, keeping data
#   scripts/k3s.sh start      start k3s again, the server first
#
# Settings (samples/clickhouse/.env; `just azure up` prints them):
#   LAB_VM_IPS       the nodes' ssh addresses, the server first
#   LAB_VM_NODE_IPS  the addresses the nodes reach each other on, in the same
#                    order (default LAB_VM_IPS)
#   LAB_VM_REGIONS   each node's region label (default: an1 an1 as1 ew2)
set -euo pipefail

: "${LAB_VM_IPS:?set LAB_VM_IPS to the ssh address of each node, the server first}"
read -ra ips <<<"$LAB_VM_IPS"
read -ra node_ips <<<"${LAB_VM_NODE_IPS:-$LAB_VM_IPS}"
read -ra regions <<<"${LAB_VM_REGIONS:-an1 an1 as1 ew2}"
((${#ips[@]} == ${#regions[@]} && ${#node_ips[@]} == ${#regions[@]})) \
    || { echo "LAB_VM_IPS, LAB_VM_NODE_IPS and LAB_VM_REGIONS need one entry per node" >&2; exit 2; }
on_vm() { ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new -o ConnectTimeout=5 "$@"; }

wait_ssh() {
    for ((i = 0; i < 100; i++)); do
        on_vm "$1" true 2>/dev/null && return
        sleep 3
    done
    echo "$1 never answered ssh" >&2
    return 1
}

provision() {
    local i ip token kubeconfig=$HOME/.kube/lab-vms.yaml
    for ip in "${ips[@]}"; do
        wait_ssh "$ip"
        on_vm "$ip" 'command -v rsync >/dev/null || { sudo apt-get update -q && sudo apt-get install -yq rsync; }'
    done
    # --tls-san: the Mac reaches the API on the ssh address, which may not be the node's own.
    on_vm "${ips[0]}" "curl -sfL https://get.k3s.io | sudo INSTALL_K3S_EXEC='server --disable traefik --disable servicelb \
        --node-ip ${node_ips[0]} --tls-san ${ips[0]} --node-label topology.kubernetes.io/region=${regions[0]} \
        --write-kubeconfig-mode 600' sh -"
    token=$(on_vm "${ips[0]}" sudo cat /var/lib/rancher/k3s/server/node-token)
    for ((i = 1; i < ${#ips[@]}; i++)); do
        on_vm "${ips[i]}" "curl -sfL https://get.k3s.io | sudo K3S_URL=https://${node_ips[0]}:6443 K3S_TOKEN='$token' \
            INSTALL_K3S_EXEC='agent --node-ip ${node_ips[i]} --node-label topology.kubernetes.io/region=${regions[i]}' sh -"
    done
    tools "${ips[0]}"
    # Every provision is a cluster with its own certificates, often at a new
    # address: the kubeconfig is written afresh, never kept from a deleted one.
    (umask 077; on_vm "${ips[0]}" "sudo cat /etc/rancher/k3s/k3s.yaml" \
        | sed -e "s/127.0.0.1/${ips[0]}/" -e 's/: default$/: lab-vms/' > "$kubeconfig.new")
    mv "$kubeconfig.new" "$kubeconfig"
    echo "VMs up. kubeconfig: $kubeconfig (context lab-vms). Next: just azure sync"
}

# The server builds the images, runs `just` and the benchmarks, and copies
# images to every node over ssh.
tools() {
    on_vm "$1" 'set -e
        sudo apt-get update -q
        sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq git rsync build-essential pkg-config cmake clang \
            valgrind just jq default-jdk-headless uuid-dev libbsd-dev python3 python3-venv
        command -v docker >/dev/null || curl -fsSL https://get.docker.com | sudo sh
        sudo usermod -aG docker "$USER"
        command -v rustup >/dev/null || [ -x ~/.cargo/bin/rustup ] \
            || curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
        # The toolchain the repo pins, and the llvm-objdump of its own LLVM for the
        # instruction probes. Linked onto the system PATH so non-login ssh finds them.
        ~/.cargo/bin/rustup toolchain install 1.95.0 --profile minimal -c llvm-tools,clippy,rustfmt
        for tool in cargo rustc rustup rustfmt cargo-clippy clippy-driver; do sudo ln -sf ~/.cargo/bin/$tool /usr/local/bin/$tool; done
        sudo ln -sf "$(~/.cargo/bin/rustc +1.95.0 --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-objdump" /usr/local/bin/llvm-objdump
        [ -f ~/.ssh/id_ed25519 ] || ssh-keygen -q -t ed25519 -N "" -f ~/.ssh/id_ed25519
        mkdir -p ~/.kube
        sudo cat /etc/rancher/k3s/k3s.yaml | sed "s/: default$/: lab-vms/" > ~/.kube/config
        chmod 600 ~/.kube/config
        grep -q ^KUBECONFIG= /etc/environment || echo "KUBECONFIG=$HOME/.kube/config" | sudo tee -a /etc/environment >/dev/null'
    # `just _load` on the server reaches each node on its node address.
    local pub i
    pub=$(on_vm "$1" cat .ssh/id_ed25519.pub)
    for i in "${!ips[@]}"; do
        on_vm "${ips[i]}" "grep -qxF '$pub' .ssh/authorized_keys || echo '$pub' >> .ssh/authorized_keys"
        on_vm "$1" "ssh-keygen -F ${node_ips[i]} >/dev/null || ssh-keyscan -q -t ed25519 ${node_ips[i]} >> .ssh/known_hosts"
    done
}

# k3s keeps its pods running when its service stops, so killall stops them too.
stop() {
    local i
    for ((i = ${#ips[@]} - 1; i >= 0; i--)); do
        on_vm "${ips[i]}" "sudo systemctl stop $(unit "$i") && sudo k3s-killall.sh >/dev/null 2>&1"
    done
}

start() {
    local i
    for i in "${!ips[@]}"; do on_vm "${ips[i]}" "sudo systemctl start $(unit "$i")"; done
}

unit() { if (($1 == 0)); then echo k3s; else echo k3s-agent; fi; }

case "${1:-}" in
    provision | start | stop) "$1" ;;
    *) echo "usage: $0 provision|start|stop" >&2; exit 2 ;;
esac
