#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Prepares an 8-CPU Linux VM (Azure D8ps_v6 Cobalt 100 or F8as_v6, no SMT)
# for the vdev-isolated overlay on kind: the boot line, hugepages, sysctls
# and IRQs. Run as root, reboot, run again: the second run only checks.
#
#   sudo samples/low-latency-k8s/node/linux-vm-prep.sh [--check]
#
# Not yet run on Azure.
#
# | setting | value | source |
# |---|---|---|
# | isolation | isolcpus=nohz,domain,managed_irq,2-7 nohz_full=2-7 rcu_nocbs=2-7 irqaffinity=0 | md-dpdk/deploy/aws-eks/node/provision-node.sh; book/src/deploy/hft.md |
# | IOMMU | x86 iommu=pt, arm64 iommu.passthrough=1 | provision-node.sh |
# | THP | transparent_hugepage=never, and never again at every boot (node/quiet-cpus.sh) | k8s/nri-shared-cpus/BENCHMARKS.md: khugepaged's LRU drain ran a kworker on an isolated CPU |
# | fair server | runtime 0 on the isolated CPUs at every boot (node/quiet-cpus.sh, kernel 6.12+, debugfs) | k8s/nri-shared-cpus/BENCHMARKS.md: its 1 s timer IPIs isolated CPUs and keeps their tick on |
# | hugepages | hugepagesz=2M hugepages=2048 | provision-node.sh |
# | page size | 4 KiB only | md-dpdk/deploy/aws-eks/README.md |
# | sysctl | kernel.numa_balancing=0, net.core.rmem_max/wmem_max=16 MiB | hft.md |
# | IRQs | irqbalance off, every IRQ on CPU 0 | hft.md |
set -euo pipefail

CHECK=0
[[ ${1:-} == --check ]] && CHECK=1
fail=0
want() { # what ok-condition message
  if eval "$2"; then echo "PASS $1"; else echo "FAIL $1: $3"; fail=1; fi
}
EXCL=2-7
case $(uname -m) in
  x86_64) IOMMU=iommu=pt ;;
  aarch64) IOMMU=iommu.passthrough=1 ;;
  *) echo "unsupported $(uname -m)"; exit 1 ;;
esac
BOOT="isolcpus=nohz,domain,managed_irq,$EXCL nohz_full=$EXCL rcu_nocbs=$EXCL irqaffinity=0 $IOMMU transparent_hugepage=never hugepagesz=2M hugepages=2048"

if [[ $CHECK == 0 ]]; then
  [[ $(id -u) == 0 ]] || { echo "run as root"; exit 1; }
  if [[ -f /etc/default/grub ]] && ! grep -q "isolcpus=" /etc/default/grub; then
    sed -i "s|^GRUB_CMDLINE_LINUX=\"\(.*\)\"|GRUB_CMDLINE_LINUX=\"\1 $BOOT\"|" /etc/default/grub
    (command -v update-grub >/dev/null && update-grub) || grub2-mkconfig -o /boot/grub2/grub.cfg
    echo "boot line set: reboot, then run this again"
  fi
  cat >/etc/sysctl.d/90-ll.conf <<SYSCTL
kernel.numa_balancing = 0
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
SYSCTL
  sysctl --system >/dev/null
  systemctl disable --now irqbalance 2>/dev/null || true
  for irq in /proc/irq/[0-9]*; do echo 0 >"$irq/smp_affinity_list" 2>/dev/null || true; done
  modprobe tun || true
  install -m 0755 "$(dirname "$0")/quiet-cpus.sh" /usr/local/sbin/ergon-quiet-cpus.sh
  install -m 0644 "$(dirname "$0")/ergon-quiet-cpus.service" /etc/systemd/system/ergon-quiet-cpus.service
  systemctl daemon-reload
  systemctl enable ergon-quiet-cpus.service
  # after the reboot: the isolated CPUs exist only once the boot line took
  if [[ -n $(cat /sys/devices/system/cpu/isolated) ]]; then systemctl restart ergon-quiet-cpus.service || true; fi
fi

want nproc '[[ $(nproc --all) -ge 8 ]]' "need 8 CPUs, have $(nproc --all)"
want smt_off '[[ $(cat /sys/devices/system/cpu/cpu0/topology/thread_siblings_list) =~ ^[0-9]+$ ]]' "SMT is on"
want page_size '[[ $(getconf PAGESIZE) == 4096 ]]' "base page size $(getconf PAGESIZE)"
want isolated '[[ $(cat /sys/devices/system/cpu/isolated) == "$EXCL" ]]' "isolated=$(cat /sys/devices/system/cpu/isolated) (the boot line did not take: reboot)"
want nohz_full '[[ $(cat /sys/devices/system/cpu/nohz_full 2>/dev/null) == "$EXCL" ]]' "nohz_full=$(cat /sys/devices/system/cpu/nohz_full 2>/dev/null) (CONFIG_NO_HZ_FULL missing?)"
want hugepages '[[ $(cat /proc/sys/vm/nr_hugepages) -ge 1024 ]]' "nr_hugepages=$(cat /proc/sys/vm/nr_hugepages)"
want thp '[[ $(cat /sys/kernel/mm/transparent_hugepage/enabled) == *"[never]"* ]]' "THP not never"
want cgroup_v2 '[[ -f /sys/fs/cgroup/cgroup.controllers ]]' "cgroup v1"
want numa_balancing '[[ $(sysctl -n kernel.numa_balancing 2>/dev/null || echo 0) == 0 ]]' "numa_balancing on"
want tun '[[ -e /dev/net/tun ]]' "no /dev/net/tun"
want quiet_unit '[[ $(systemctl is-enabled ergon-quiet-cpus.service 2>/dev/null) == enabled ]]' "ergon-quiet-cpus.service not enabled"
want quiet_cpus 'bash "$(dirname "$0")/quiet-cpus.sh" --check' "THP or the fair server: see the FAIL lines above"
exit $fail
