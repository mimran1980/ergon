#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# netns-sysctls.sh -- write the node's network settings for latency pods.
#
# Candidate 7 of the latency-pod shortlist. The four names people ask for are
# kernel-blocked in a pod netns, so this script writes them where the kernel
# allows (the node) and refuses to pretend a pod can. Run it from user data or
# a systemd unit BEFORE the kubelet starts.
#
#   node/netns-sysctls.sh            write the node sysctl.d and the netdev attrs
#   node/netns-sysctls.sh --check    report only, exit 1 if anything is wrong
#   node/netns-sysctls.sh --refuse NAME
#                                    print the refusal for a pod-level request
#
# Env knobs (all optional):
#   RMEM_MAX / WMEM_MAX     default 16777216 (16 MiB)
#   NETDEV                  default eth0
#   GRO_FLUSH_TIMEOUT       default 200000 (ns)
#   NAPI_DEFER_HARD_IRQS    default 1
#   BUSY_POLL               default 0 (keep off on Graviton)
set -euo pipefail

rmem_max=${RMEM_MAX:-16777216}
wmem_max=${WMEM_MAX:-16777216}
netdev=${NETDEV:-eth0}
gro_flush_timeout=${GRO_FLUSH_TIMEOUT:-200000}
napi_defer=${NAPI_DEFER_HARD_IRQS:-1}
busy_poll=${BUSY_POLL:-0}

say() { printf '%s\n' "$*"; }

refuse() {
    local name=$1
    case "$name" in
        net.core.rmem_max|net.core.wmem_max|net.core.rmem_default|net.core.wmem_default)
            say "$name is a node-wide sysctl: write it in the node's sysctl.d before the kubelet starts (see nri-latency-pods/node-checks.md)"
            ;;
        net.core.busy_poll|net.core.busy_read)
            say "$name is registered only for init_net (host-wide); set it on the node, and keep it 0 on Graviton (see nri-latency-pods/node-checks.md)"
            ;;
        net.core.gro_flush_timeout|net.core.napi_defer_hard_irqs)
            say "$name is a per-netdev sysfs attribute, not a sysctl; write /sys/class/net/<dev>/$name on the node (see nri-latency-pods/node-checks.md)"
            ;;
        *)
            say "$name is not a known node-only setting; check nri-latency-pods/node-checks.md"
            ;;
    esac
}

if [[ "${1:-}" == "--refuse" ]]; then
    refuse "${2:?--refuse needs a sysctl name}"
    exit 0
fi

check_only=0
[[ "${1:-}" == "--check" ]] && check_only=1

failed=0
check() { # check NAME ACTUAL WANT
    if [[ "$2" != "$3" ]]; then
        say "FAIL $1: $2, want $3"
        failed=1
    else
        say "ok   $1: $2"
    fi
}

# --- sysctl.d (the only place these are writable) ---
if [[ $check_only -eq 0 ]]; then
    conf=/etc/sysctl.d/99-ergon-latency.conf
    say "writing $conf"
    cat >"$conf" <<EOF
# ergon-latency-pods latency node. Written by nri-latency-pods/node/netns-sysctls.sh.
# A pod cannot set these: rmem/wmem_max and friends are read-only outside
# init_net, busy_* is host-wide. Do not ask a pod to.
net.core.rmem_max = $rmem_max
net.core.wmem_max = $wmem_max
net.core.busy_poll = $busy_poll
net.core.busy_read = $busy_poll
kernel.numa_balancing = 0
EOF
    sysctl --system >/dev/null
fi

cur_rmem=$(sysctl -n net.core.rmem_max 2>/dev/null || echo missing)
cur_wmem=$(sysctl -n net.core.wmem_max 2>/dev/null || echo missing)
cur_busy=$(sysctl -n net.core.busy_poll 2>/dev/null || echo missing)
check net.core.rmem_max "$cur_rmem" "$rmem_max"
check net.core.wmem_max "$cur_wmem" "$wmem_max"
check net.core.busy_poll "$cur_busy" "$busy_poll"

# --- per-netdev sysfs attributes (not sysctls) ---
for f in gro_flush_timeout napi_defer_hard_irqs; do
    path=/sys/class/net/$netdev/$f
    want=$gro_flush_timeout
    [[ "$f" == napi_defer_hard_irqs ]] && want=$napi_defer
    if [[ ! -e "$path" ]]; then
        say "FAIL $path: absent (device $netdev or kernel without the attribute)"
        failed=1
        continue
    fi
    if [[ $check_only -eq 0 ]]; then
        echo "$want" >"$path" || true
    fi
    check "$path" "$(cat "$path")" "$want"
done

if [[ $failed -ne 0 ]]; then
    say "netns-sysctls: FAILED"
    exit 1
fi
say "netns-sysctls: PASS"
