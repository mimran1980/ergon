#!/usr/bin/env bash
# The kind lab's four nodes and three regions as Debian VMs on a libvirt host,
# joined into one k3s cluster. The first VM is the server and the place to
# build, test and run `just`; the VMs sit on the host's LAN through macvtap.
#
#   scripts/vms.sh create    make the VMs, then provision them
#   scripts/vms.sh provision install k3s and the server's tools, write ~/.kube/lab-vms.yaml;
#                            safe to repeat after a failure
#   scripts/vms.sh stop      stop k3s and every pod on every VM, keeping data
#   scripts/vms.sh start     start k3s again, the server first
#   scripts/vms.sh destroy   delete the VMs and their disks, after confirmation
#
# Settings, usually in samples/clickhouse/.env (gitignored):
#   LAB_VM_HOST    ssh name of the libvirt host; you must be in its libvirt group
#   LAB_VM_IPS     four free addresses on the host's LAN, the server first
#   LAB_VM_DIR     directory on the host for the disks, readable by libvirt-qemu
#   LAB_VM_CPUSET  host CPUs the VMs may use, e.g. 2-5 (default: any)
#   LAB_VM_NIC     host interface the VMs share (default eth0)
set -euo pipefail

: "${LAB_VM_IPS:?set LAB_VM_IPS to four free LAN addresses, the server first}"
read -ra ips <<<"$LAB_VM_IPS"
((${#ips[@]} == 4)) || { echo "LAB_VM_IPS needs four addresses" >&2; exit 2; }
names=(lab-an1a lab-an1b lab-as1 lab-ew2)
regions=(an1 an1 as1 ew2)
vcpus=(4 2 2 2)
memory=(7168 3072 3072 3072)
disk_gb=(120 30 30 30)
image=https://cloud.debian.org/images/cloud/trixie/latest/debian-13-genericcloud-amd64.qcow2

on_vm() { ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new -o ConnectTimeout=5 "$@"; }

host_settings() {
    : "${LAB_VM_HOST:?set LAB_VM_HOST to the ssh name of the libvirt host}"
    : "${LAB_VM_DIR:?set LAB_VM_DIR to a directory on the libvirt host}"
}
on_host() { ssh -o BatchMode=yes "$LAB_VM_HOST" "$@"; }
virsh() { on_host virsh -c qemu:///system "$@"; }

user_data() {
    cat <<EOF
#cloud-config
hostname: $1
users:
  - name: $(id -un)
    groups: [sudo]
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
    ssh_authorized_keys: ["$2"]
runcmd:
  - install -d -o $(id -un) -g $(id -un) /srv/ergon
  - ln -sfn /srv/ergon/samples/clickhouse /lab
EOF
}

network_config() {
    cat <<EOF
version: 2
ethernets:
  lan:
    match: { macaddress: "$1" }
    addresses: [$2/${LAB_VM_PREFIX:-24}]
    routes: [{ to: default, via: ${LAB_VM_GATEWAY:-${2%.*}.1} }]
    nameservers: { addresses: [${LAB_VM_GATEWAY:-${2%.*}.1}] }
EOF
}

mac_for() { printf '52:54:00:1a:b0:%02x' "${1##*.}"; }

wait_ssh() {
    for ((i = 0; i < 100; i++)); do
        on_vm "$1" true 2>/dev/null && return
        sleep 3
    done
    echo "$1 never answered ssh" >&2
    return 1
}

create() {
    host_settings
    local key_file=${LAB_VM_SSH_KEY:-$HOME/.ssh/id_ed25519.pub}
    [[ -f $key_file ]] || key_file=$HOME/.ssh/id_rsa.pub
    local key i name dir=$LAB_VM_DIR seed
    key=$(cat "$key_file")
    # Unreachable libvirt must not read as "no such VM".
    virsh list >/dev/null
    for i in "${!names[@]}"; do
        name=${names[i]}
        if virsh dominfo "$name" >/dev/null 2>&1 || on_host test -e "$dir/$name.qcow2"; then
            echo "$name already exists on $LAB_VM_HOST; refusing to overwrite it" >&2
            exit 1
        fi
    done
    on_host "test -f '$dir/debian-13.qcow2' || { curl -fsSLo '$dir/debian-13.qcow2.part' '$image' && mv '$dir/debian-13.qcow2.part' '$dir/debian-13.qcow2'; }"

    seed=$(mktemp -d)
    trap 'rm -rf "$seed"' RETURN
    for i in "${!names[@]}"; do
        name=${names[i]}
        mkdir "$seed/$name"
        user_data "$name" "$key" > "$seed/$name/user-data"
        printf 'instance-id: %s\nlocal-hostname: %s\n' "$name" "$name" > "$seed/$name/meta-data"
        network_config "$(mac_for "${ips[i]}")" "${ips[i]}" > "$seed/$name/network-config"
        COPYFILE_DISABLE=1 tar -C "$seed" -cf - "$name" | on_host "tar -C '$dir' -xf - && mv '$dir/$name' '$dir/$name-seed' \
            && genisoimage -quiet -o '$dir/$name-seed.iso' -V cidata -J -r '$dir/$name-seed' \
            && qemu-img create -q -f qcow2 -F qcow2 -b '$dir/debian-13.qcow2' '$dir/$name.qcow2' ${disk_gb[i]}G"
        # --video vga: with no video device the Debian cloud image resets in a loop
        # under KVM, right after GRUB (it boots under TCG, which hides it).
        local virsh_cpuset=${LAB_VM_CPUSET:+,cpuset=$LAB_VM_CPUSET}
        on_host virt-install --connect qemu:///system --name "$name" --osinfo generic \
            --memory "${memory[i]}" --vcpus "${vcpus[i]}$virsh_cpuset" --cpu host-passthrough \
            --disk "path=$dir/$name.qcow2,bus=virtio" --disk "path=$dir/$name-seed.iso,device=cdrom" \
            --network "type=direct,source=${LAB_VM_NIC:-eth0},source_mode=bridge,model=virtio,mac=$(mac_for "${ips[i]}")" \
            --graphics none --video vga --import --noautoconsole --autostart
    done
    provision
}

provision() {
    local i ip token kubeconfig=$HOME/.kube/lab-vms.yaml
    for ip in "${ips[@]}"; do
        wait_ssh "$ip"
        on_vm "$ip" 'command -v rsync >/dev/null || { sudo apt-get update -q && sudo apt-get install -yq rsync; }'
    done
    on_vm "${ips[0]}" "curl -sfL https://get.k3s.io | sudo INSTALL_K3S_EXEC='server --disable traefik --disable servicelb \
        --node-ip ${ips[0]} --node-label topology.kubernetes.io/region=${regions[0]} --write-kubeconfig-mode 600' sh -"
    token=$(on_vm "${ips[0]}" sudo cat /var/lib/rancher/k3s/server/node-token)
    for i in 1 2 3; do
        on_vm "${ips[i]}" "curl -sfL https://get.k3s.io | sudo K3S_URL=https://${ips[0]}:6443 K3S_TOKEN='$token' \
            INSTALL_K3S_EXEC='agent --node-ip ${ips[i]} --node-label topology.kubernetes.io/region=${regions[i]}' sh -"
    done
    tools "${ips[0]}"
    if [[ ! -e $kubeconfig ]]; then
        (umask 077; on_vm "${ips[0]}" "sudo cat /etc/rancher/k3s/k3s.yaml" \
            | sed -e "s/127.0.0.1/${ips[0]}/" -e 's/: default$/: lab-vms/' > "$kubeconfig")
    fi
    echo "VMs up. kubeconfig: $kubeconfig (context lab-vms). Next: just sync"
}

# The server builds the images, runs `just` and the benchmarks, and copies
# images to every node over ssh.
tools() {
    on_vm "$1" 'set -e
        sudo apt-get update -q
        sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq git rsync build-essential pkg-config cmake clang \
            valgrind just jq default-jre-headless python3 python3-venv
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
    local pub ip
    pub=$(on_vm "$1" cat .ssh/id_ed25519.pub)
    for ip in "${ips[@]}"; do
        on_vm "$ip" "grep -qxF '$pub' .ssh/authorized_keys || echo '$pub' >> .ssh/authorized_keys"
        on_vm "$1" "ssh-keygen -F $ip >/dev/null || ssh-keyscan -q -t ed25519 $ip >> .ssh/known_hosts"
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

destroy() {
    host_settings
    local name dir=$LAB_VM_DIR
    echo "This deletes, on $LAB_VM_HOST, the VMs and everything recorded in them:"
    for name in "${names[@]}"; do echo "  $name   $dir/$name.qcow2  $dir/$name-seed.iso  $dir/$name-seed/"; done
    read -rp 'Type "destroy" to continue: ' answer
    [[ $answer == destroy ]] || { echo 'Nothing deleted.'; exit 1; }
    for name in "${names[@]}"; do
        virsh destroy "$name" 2>/dev/null || true
        virsh undefine "$name" || true
        on_host "rm -f '$dir/$name.qcow2' '$dir/$name-seed.iso' && rm -rf '$dir/$name-seed'"
    done
    echo "Deleted. The base image $dir/debian-13.qcow2 and ~/.kube/lab-vms.yaml remain."
}

case "${1:-}" in
    create | provision | start | stop | destroy) "$1" ;;
    *) echo "usage: $0 create|provision|start|stop|destroy" >&2; exit 2 ;;
esac
