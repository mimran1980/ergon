#!/usr/bin/env bash
# Runs as root on each VM (Azure or AWS; x86_64 or aarch64, e.g. Graviton).
#   vm-setup.sh install <housekeeping-cpus> <isolated-cpus> [profile]
#         packages + isolation (boot profile, default full; reboot afterwards)
#   vm-setup.sh boot <profile> <housekeeping-cpus> <isolated-cpus> [tick-cpus]
#         kernel command line for the next boot (reboot afterwards); profiles:
#           full          isolcpus=managed_irq,domain + nohz_full + rcu_nocbs + irqaffinity, irqbalance off
#           full-sqtick   full, but <tick-cpus> (the SQPOLL CPU) left out of nohz_full: a kernel
#                         thread that never leaves the kernel keeps its tick instead of having it
#                         restarted by an IPI for every RCU grace period
#           full-nomitig  full + mitigations=off
#           full-nohlt    full + nohlt (arm64 only): idle CPUs poll instead of executing WFI.
#                         Graviton gives the OS no idle states to cap ("AWS Graviton processors
#                         have built-in power saving modes and operate at a fixed frequency.
#                         Therefore, they do not provide the ability for the operating system to
#                         control C-states and P-states": EC2 User Guide, processor state
#                         control), so polling is the only idle setting there is. It goes
#                         further than x86's C1 cap, which still halts as WFI does; with a
#                         fixed frequency it costs no turbo headroom. nohlt needs
#                         CONFIG_GENERIC_IDLE_POLL_SETUP, which arm64 selects (arch/arm64/Kconfig;
#                         Documentation/admin-guide/kernel-parameters.txt). x86 has no nohlt (its
#                         idle=poll would spin the loop's hyperthread sibling): refused there.
#           full-pollidle full, and idle CPUs poll instead of halting (arm64: nohlt; x86: idle=poll,
#                         refused on a host with SMT, where it would spin the loop's sibling): the CPU
#                         that takes the NIC interrupts is never asleep when one arrives, so the
#                         interrupt-driven backends do not pay a wake-up (the fair mode's setting)
#           isolcpus      isolcpus=managed_irq,domain + irqaffinity only, irqbalance off
#           stock         nothing added, irqbalance on (the VM as it comes)
#   vm-setup.sh bootline <profile> <housekeeping-cpus> <isolated-cpus> [tick-cpus]
#         prints the line `boot` would add, changing nothing
#   vm-setup.sh tune [stock|nic]    per-boot settings (stock: none; nic: also the NIC settings the
#         AWS Tokyo runs found best, adaptive receive moderation off), prints the environment:
#         architecture, counter, idle states, SMT, isolation as applied, and every x86-only step
#         skipped on this host with its reason
#   vm-setup.sh pin                 every movable IRQ back on the housekeeping CPUs (the NIC
#         queues on the loop CPU while the irqlocal condition is on) and the NIC's interrupt
#         moderation back to what the run set (tune nic, nodim, nocoalesce); run after every NIC
#         change and before every measurement, because the ENA driver re-applies its own
#         per-queue affinity, and turns adaptive moderation back on, whenever it brings the
#         interface up
#   vm-setup.sh fair [nic-irq-cpu]  the fair mode's NIC and kernel settings, held for the whole run
#         (pin): net.core.busy_poll and busy_read 0 (an interrupt-driven backend is purely
#         interrupt driven; recv-busy and uring-sqpoll set their own busy polling per socket or
#         ring), interrupt moderation off where the NIC has it (adaptive-rx off, rx-usecs 0, RX CQE
#         coalescing off), napi_defer_hard_irqs and gro_flush_timeout 0, and every NIC queue
#         interrupt on <nic-irq-cpu> (a core of its own; housekeeping when not given). Prints
#         what each NIC allows (a NIC without a moderation setting says so)
#   vm-setup.sh check               every setting the fair mode holds, one `key=value` line each, then
#         `check: ok` or `check: DRIFT ...` naming each setting not as wanted (exit 3); the runs
#         check before and after every point and fail a point whose settings drifted
#   vm-setup.sh cond <name> on|off [cpu]   one runtime setting under test:
#           housekeeping  workqueues on housekeeping CPUs, NMI and soft-lockup watchdogs off, vm.stat_interval=120
#           prefer        napi_defer_hard_irqs=100, gro_flush_timeout=200000 on every NIC (for SO_PREFER_BUSY_POLL)
#           noquickack    routes without quickack (rto_min kept)
#           nogro         GRO off on every NIC
#           nocoalesce    interrupt moderation off (adaptive-rx off, rx-usecs 0) where the NIC allows it
#           irqlocal      every NIC queue interrupt on <cpu> (the loop's CPU)
#           rt            kernel.sched_rt_runtime_us=-1 (lets a SCHED_FIFO loop run unthrottled)
#           noagent       the cloud guest agents stopped (Azure walinuxagent and hv-kvp-daemon; AWS
#                         amazon-ssm-agent and the CloudWatch agent), restarted by `off`
#           nodim         adaptive interrupt moderation off (ethtool adaptive-rx off), nothing else:
#                         ENA's DIM work otherwise runs as a kworker on the CPU that polls the
#                         queue, pre-empting the busy-polling loop
#           onequeue      every NIC down to one combined queue: the busy-polled queue then also
#                         carries the flow's transmit completions, and no other queue interrupts
#           noautocork    net.ipv4.tcp_autocorking=0: a send never waits for the previous one to
#                         leave the qdisc
#           cstate        every CPU idle state deeper than C1 disabled (exit latency above 1 µs,
#                         e.g. C6's 190 µs on c7i), so an idling core wakes for an interrupt at once;
#                         a Graviton VM has none (WFI only): there the boot profile full-nohlt is
#                         the setting to measure
#   vm-setup.sh trace start <cpu-list> [noirq] | trace stop <file>   kernel events on those CPUs, mono
#         clock; noirq leaves out the interrupt and softirq events (the fair mode's irqlocal points)
#   vm-setup.sh fgraph start <cpu> <us> | fgraph stop <file>  kernel functions slower than <us> on
#         <cpu> (function_graph with tracing_thresh; heavy: diagnosis runs only)
#   vm-setup.sh slowsys start <cpu> <us> <file> | slowsys stop   syscalls slower than <us> on <cpu>
#         (bpftrace, one line each: end mono ns, syscall number, duration ns)
#   vm-setup.sh hwlat start <cpus> <us> | hwlat stop <file>   the kernel's hardware-latency
#         detector on each of <cpus> at once: gaps longer than <us> with interrupts off are the hypervisor or firmware
#   vm-setup.sh netstats <file>     snapshot NIC (ethtool -S) and TCP (nstat) counters
#   vm-setup.sh irqs <file>         snapshot /proc/interrupts (plus each CPU's SMI count on bare metal)
#   vm-setup.sh clock <file>        chrony's view of this host's clock (`chronyc -n tracking` and
#         `sources`), kept with each run: one-way figures compare two hosts' clocks, and analyse
#         bounds their offset with |System time| + 1/2 root delay + root dispersion per host
set -euo pipefail
cmd=${1:?install|boot|bootline|tune|pin|cond|trace|irqs}
# VM_SETUP_ROOT (tests): a directory standing in for / for the state directory,
# the NICs under /sys/class/net and the /proc files the IRQ commands use.
R=${VM_SETUP_ROOT:-}
# The architecture whose settings apply: this host's (tests set another).
ARCH=${VM_SETUP_ARCH:-$(uname -m)}
STATE=$R/var/lib/ergon-lab
mkdir -p "$STATE"

# Hex mask for a CPU list such as "2,3,6-7" (CPUs below 64).
mask() {
  local m=0 part a b c
  IFS=, read -ra parts <<< "$1"
  for part in "${parts[@]}"; do
    a=${part%-*}; b=${part#*-}
    for ((c = a; c <= b; c++)); do m=$((m | (1 << c))); done
  done
  printf '%x' "$m"
}
# NICs: interfaces backed by a device (netvsc, the MANA VF, ENA), not lo,
# tunnels or plain files such as bonding_masters.
nics() {
  local n
  for n in "$R"/sys/class/net/*; do
    [[ -e $n/device ]] && basename "$n"
  done
  return 0
}
nic_irqs() { awk -F: '/mana|ena|ens|eth|enp|mlx|virtio/ && $1 ~ /^ *[0-9]+$/ {gsub(/ /, "", $1); print $1}' "$R/proc/interrupts"; }
online() { cat /sys/devices/system/cpu/online; }
hk_cpus() { sed -n 's/.*irqaffinity=\([^ ]*\).*/\1/p' "$R/proc/cmdline"; }

# The CPUs of list $1 not in list $2, comma-separated (lists may hold ranges).
cpus_minus() {
  local c out=""
  for c in $(expand_cpus "$1"); do
    [[ ,$(expand_cpus "$2" | tr ' ' ,), == *,$c,* ]] || out+=${out:+,}$c
  done
  echo "$out"
}
expand_cpus() {
  local part
  for part in ${1//,/ }; do
    if [[ $part == *-* ]]; then seq "${part%-*}" "${part#*-}"; else echo "$part"; fi
  done | tr '\n' ' '
}

# The kernel command line a boot profile adds (see the usage above).
boot_line() {
  local profile=$1 hk=$2 iso=$3 full
  full="isolcpus=managed_irq,domain,$iso nohz_full=$iso rcu_nocbs=$iso irqaffinity=$hk"
  case $profile in
    full) echo "$full" ;;
    full-sqtick)
      local nohz
      nohz=$(cpus_minus "$iso" "${4:?tick cpus}")
      echo "isolcpus=managed_irq,domain,$iso nohz_full=$nohz rcu_nocbs=$iso irqaffinity=$hk" ;;
    full-nomitig) echo "$full mitigations=off" ;;
    full-nohlt)
      if [[ $ARCH != aarch64 ]]; then
        echo "boot profile full-nohlt is arm64 only: $ARCH has no nohlt (idle=poll there would spin the loop's hyperthread sibling)" >&2
        return 2
      fi
      echo "$full nohlt" ;;
    full-pollidle)
      if [[ $ARCH == aarch64 ]]; then
        echo "$full nohlt"
      elif [[ $(cat "$R/sys/devices/system/cpu/smt/active" 2>/dev/null || echo 0) == 1 ]]; then
        echo "boot profile full-pollidle: this $ARCH host has SMT, and idle=poll would spin the loop's hyperthread sibling: use full" >&2
        return 2
      else
        echo "$full idle=poll"
      fi ;;
    isolcpus) echo "isolcpus=managed_irq,domain,$iso irqaffinity=$hk" ;;
    stock) echo "" ;;
    *) echo "unknown boot profile $profile" >&2; return 2 ;;
  esac
}

boot() {
  local profile=$1 line
  line=$(boot_line "$@") || exit 2
  local f=/etc/default/grub.d/99-iobench.cfg
  if [[ -n $line ]]; then
    printf 'GRUB_CMDLINE_LINUX_DEFAULT="$GRUB_CMDLINE_LINUX_DEFAULT %s"\n' "$line" > "$f"
    systemctl disable --now irqbalance 2>/dev/null || true
  else
    rm -f "$f"
    systemctl enable irqbalance 2>/dev/null || true
  fi
  update-grub >/dev/null 2>&1 || grub2-mkconfig -o /boot/grub2/grub.cfg >/dev/null 2>&1
  echo "$profile" > "$STATE/boot-profile"
  echo "boot profile $profile set; reboot to apply: ${line:-<none>}"
}

routes() {
  # Re-add the default and subnet routes with the given options, keeping every
  # existing attribute (proto dhcp, src, metric); quickack and rto_min (shown
  # as "rto_min lock 5ms") are stripped first so the options can be switched
  # both ways.
  local opts=$1 dev def net line
  dev=$(ip route show default | awk '{print $5; exit}')
  def=$(ip -4 route show default | sed -n '1{s/ quickack [0-9]*//; s/ rto_min \(lock \)\{0,1\}[^ ]*//; p}')
  # shellcheck disable=SC2086
  ip route replace $def $opts || echo "WARN: default route: $def"
  for net in $(ip -4 route show dev "$dev" scope link | awk '{print $1}'); do
    line=$(ip -4 route show "$net" dev "$dev" scope link | sed -n '1{s/ quickack [0-9]*//; s/ rto_min \(lock \)\{0,1\}[^ ]*//; p}')
    # shellcheck disable=SC2086
    ip route replace $net dev "$dev" ${line#"$net"} $opts || echo "WARN: route: $line"
  done
}

# The CPU the NIC queue interrupts belong on: the loop CPU while the irqlocal
# condition is on, else the fair mode's IRQ core (`fair <cpu>`), else
# housekeeping ($1).
nic_irq_want() {
  if [[ -s $STATE/irqlocal-cpu ]]; then
    cat "$STATE/irqlocal-cpu"
  elif [[ -s $STATE/nic-irq-cpu ]]; then
    cat "$STATE/nic-irq-cpu"
  else
    echo "$1"
  fi
}

# Every movable IRQ on the housekeeping CPUs (this boot's irqaffinity=), or,
# while the irqlocal condition is on, the NIC's on the loop CPU (in the fair
# mode, on its IRQ core). Each time the
# ENA driver brings the interface up (ena_up in
# drivers/net/ethernet/amazon/ena/ena_netdev.c: after ethtool -L or -G, an XDP
# change or a device reset) it gives queue i the affinity hint CPU i mod nCPUs
# and applies it (ena_setup_io_intr, ena_request_io_irq: irq_set_affinity_hint),
# so every queue's interrupt lands on the isolated CPUs again. So this runs at
# boot, after every NIC change and before every measurement. An IRQ already
# where it belongs is not written again (on arm64 a write can move it between
# the housekeeping CPUs). Prints one line.
pin_irqs() {
  local hk loc="" nic f q want moved=0 total=0 stuck=0
  hk=$(hk_cpus)
  if [[ -z $hk ]]; then
    echo "pin: no irqaffinity= on the kernel command line (boot profile stock): IRQs left as they are"
    return 0
  fi
  loc=$(nic_irq_want "$hk")
  [[ $loc == "$hk" ]] && loc=""
  nic=" $(nic_irqs | tr '\n' ' ') "
  for f in "$R"/proc/irq/[0-9]*/smp_affinity_list; do
    [[ -e $f ]] || continue
    q=${f%/smp_affinity_list}; q=${q##*/}
    want=$hk
    [[ -n $loc && $nic == *" $q "* ]] && want=$loc
    total=$((total + 1))
    [[ $(expand_cpus "$(cat "$f" 2>/dev/null)") == "$(expand_cpus "$want")" ]] && continue
    if echo "$want" > "$f" 2>/dev/null; then moved=$((moved + 1)); else stuck=$((stuck + 1)); fi
  done
  local why="irqlocal on"
  [[ -s $STATE/irqlocal-cpu ]] || why="the fair mode's IRQ core"
  echo "pin: $moved of $total IRQs moved to $hk${loc:+ (NIC queues to $loc: $why)}; $stuck cannot move (per-CPU or kernel-managed)"
}

# Interrupt moderation the run set and holds: adaptive receive moderation off
# (tune nic, or the nodim or nocoalesce condition) and rx-usecs 0
# (nocoalesce). ena_up turns adaptive moderation back on whenever the
# interface comes up ("Enabling DIM needs to happen before enabling IRQs":
# ena_com_enable_adaptive_moderation), so it is checked and set again here.
# AWS recommends it off for latency-sensitive work on Graviton ("Set to
# ethtool -C ethN adaptive-rx off for a latency sensitive workload":
# aws-graviton-getting-started, perfrunbook/optimization_recommendation.md);
# the AWS Tokyo runs found it the best NIC setting (BENCHMARK.md, host and NIC
# settings on AWS).
pin_nic() {
  local off="" usecs="" cqe="" d a u c
  [[ -e $STATE/want-tune-nic || -e $STATE/want-cond-nodim || -e $STATE/want-cond-nocoalesce || -e $STATE/want-fair ]] && off=1
  [[ -e $STATE/want-cond-nocoalesce || -e $STATE/want-fair ]] && usecs=0
  # The fair mode: MANA's RX CQE coalescing (several packets per completion,
  # held up to its timeout) off too, where the driver and ethtool show it.
  [[ -e $STATE/want-fair ]] && cqe=1
  [[ -n $off ]] || return 0
  for d in $(nics); do
    a=$(ethtool -c "$d" 2>/dev/null | awk '/Adaptive RX/ {print $3}' || true)
    u=$(ethtool -c "$d" 2>/dev/null | awk '/^rx-usecs:/ {print $2}' || true)
    c=$(ethtool -c "$d" 2>/dev/null | awk '/^rx-cqe-frames:/ {print $2}' || true)
    if [[ -n $cqe && $c =~ ^[0-9]+$ && $c != 1 ]]; then
      ethtool -C "$d" rx-cqe-frames 1 2>/dev/null && echo "pin: $d rx-cqe-frames was $c, set 1 again" \
        || echo "WARN: $d rx-cqe-frames 1 unsupported"
    fi
    [[ -n $a ]] || continue
    if [[ $a != off ]]; then
      ethtool -C "$d" adaptive-rx off 2>/dev/null && echo "pin: $d adaptive-rx was $a, set off again" \
        || echo "WARN: $d adaptive-rx off unsupported"
    fi
    if [[ -n $usecs && -n $u && $u != "$usecs" ]]; then
      ethtool -C "$d" rx-usecs "$usecs" 2>/dev/null && echo "pin: $d rx-usecs was $u, set $usecs again" \
        || echo "WARN: $d rx-usecs $usecs unsupported"
    fi
  done
  return 0
}

if [[ $cmd == install ]]; then
  hk=${2:?housekeeping cpus}; iso=${3:?isolated cpus}
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq
  # Parallel copies (several SSH connections at once) must not be refused.
  echo "MaxStartups 100:30:200" > /etc/ssh/sshd_config.d/99-lab.conf
  systemctl reload ssh 2> /dev/null || true
  pkgs=(build-essential pkg-config ethtool chrony jq)
  # msr-tools reads x86 model-specific registers (the SMI count, `irqs`):
  # there is no package, nor anything to read, elsewhere.
  if [[ $ARCH == x86_64 ]]; then
    pkgs+=(msr-tools)
  else
    echo "skip msr-tools: x86 only (no MSRs on $ARCH)"
  fi
  apt-get install -y -qq "${pkgs[@]}" linux-tools-common "linux-tools-$(uname -r)" >/dev/null 2>&1 \
    || apt-get install -y -qq "${pkgs[@]}" >/dev/null
  boot "${4:-full}" "$hk" "$iso"
  exit 0
fi

if [[ $cmd == boot ]]; then
  boot "${2:?profile}" "${3:?housekeeping cpus}" "${4:?isolated cpus}" "${5:-}"
  exit 0
fi

if [[ $cmd == bootline ]]; then
  boot_line "${2:?profile}" "${3:?housekeeping cpus}" "${4:?isolated cpus}" "${5:-}" || exit 2
  exit 0
fi

# sysctls through /proc/sys (under VM_SETUP_ROOT in the tests).
sysctl_get() { cat "$R/proc/sys/${1//.//}" 2> /dev/null || echo "?"; }
sysctl_set() { echo "$2" > "$R/proc/sys/${1//.//}"; }
# The fair mode: no global busy polling, set again if anything changed it.
pin_sysctls() {
  [[ -e $STATE/want-fair ]] || return 0
  local k v
  for k in net.core.busy_poll net.core.busy_read; do
    v=$(sysctl_get "$k")
    if [[ $v != 0 ]]; then
      sysctl_set "$k" 0 && echo "pin: $k was $v, set 0 again" || echo "WARN: $k 0 not set"
    fi
  done
}

if [[ $cmd == pin ]]; then
  pin_irqs
  pin_nic
  pin_sysctls
  exit 0
fi
nic_driver() { ethtool -i "$1" 2> /dev/null | awk '/^driver:/ {print $2}' || true; }
# One NIC's interrupt moderation as `ethtool -c` shows it: adaptive RX,
# rx-usecs, rx-frames and RX CQE coalescing, `-` for each it does not show.
nic_moderation() {
  local out
  out=$(ethtool -c "$1" 2> /dev/null) || { echo "unsupported"; return 0; }
  awk '/Adaptive RX/ {a = $3} /^rx-usecs:/ {u = $2} /^rx-frames:/ {f = $2} /^rx-cqe-frames:/ {c = $2}
    END {printf "%s %s %s %s\n", (a == "" ? "-" : a), (u == "" ? "-" : u), (f == "" ? "-" : f), (c == "" ? "-" : c)}' <<< "$out"
}

if [[ $cmd == fair ]]; then
  : > "$STATE/want-fair"
  if [[ -n ${2:-} ]]; then echo "$2" > "$STATE/nic-irq-cpu"; else rm -f "$STATE/nic-irq-cpu"; fi
  for k in net.core.busy_poll net.core.busy_read; do
    sysctl_set "$k" 0 || echo "WARN: $k 0 not set"
  done
  for d in $(nics); do
    for f in napi_defer_hard_irqs gro_flush_timeout; do
      if [[ -e $R/sys/class/net/$d/$f ]]; then echo 0 > "$R/sys/class/net/$d/$f" || echo "WARN: $d $f 0 not set"; fi
    done
    m=$(nic_moderation "$d")
    if [[ $m == unsupported ]]; then
      echo "fair: $d ($(nic_driver "$d")): no interrupt moderation setting (ethtool -c unsupported): nothing to turn off"
    else
      read -r a u f c <<< "$m"
      echo "fair: $d ($(nic_driver "$d")): interrupt moderation adaptive-rx $a, rx-usecs $u, rx-frames $f, rx-cqe-frames $c (before)"
    fi
  done
  pin_irqs
  pin_nic
  echo "fair: net.core.busy_poll $(sysctl_get net.core.busy_poll), busy_read $(sysctl_get net.core.busy_read); NIC queue IRQs on $(nic_irq_want "$(hk_cpus)")"
  exit 0
fi

# Every setting the fair mode holds, and whether it is as wanted (see usage).
if [[ $cmd == check ]]; then
  drift=()
  # setting <key> <value> [wanted]: one line; a wanted value it differs from is drift.
  setting() {
    echo "$1=$2"
    if [[ -n ${3:-} && $2 != "$3" ]]; then drift+=("$1 want $3 have $2"); fi
  }
  fair=""; [[ -e $STATE/want-fair ]] && fair=1
  off=""; [[ -n $fair || -e $STATE/want-tune-nic || -e $STATE/want-cond-nodim || -e $STATE/want-cond-nocoalesce ]] && off=1
  zero=""; [[ -n $fair || -e $STATE/want-cond-nocoalesce ]] && zero=1
  hk=$(hk_cpus)
  state=off; [[ -n $fair ]] && state=on
  setting fair "$state"
  bp=$(cat "$STATE/boot-profile" 2> /dev/null || echo unknown)
  setting boot-profile "$bp"
  idle=halt
  grep -qw nohlt "$R/proc/cmdline" 2> /dev/null && idle=poll-nohlt
  grep -qw 'idle=poll' "$R/proc/cmdline" 2> /dev/null && idle=poll-idle=poll
  # A profile whose idle CPUs poll, booted without it (the kernel line did
  # not take): the IRQ core would wake from sleep for every interrupt.
  w=""
  [[ $idle == halt && ($bp == full-pollidle || $bp == full-nohlt) ]] && w=poll
  setting idle "$idle" "$w"
  setting housekeeping "${hk:-none}"
  setting sysctl.net.core.busy_poll "$(sysctl_get net.core.busy_poll)" "${fair:+0}"
  setting sysctl.net.core.busy_read "$(sysctl_get net.core.busy_read)" "${fair:+0}"
  for d in $(nics); do
    setting "nic.$d.driver" "$(nic_driver "$d")"
    m=$(nic_moderation "$d")
    if [[ $m == unsupported ]]; then
      setting "nic.$d.moderation" "none (ethtool -c unsupported)"
    else
      read -r a u f c <<< "$m"
      w=""; [[ -n $off && $a != - ]] && w=off
      setting "nic.$d.adaptive-rx" "$a" "$w"
      w=""; [[ -n $zero && $u != - ]] && w=0
      setting "nic.$d.rx-usecs" "$u" "$w"
      setting "nic.$d.rx-frames" "$f"
      w=""; [[ -n $fair && $c != - ]] && w=1
      setting "nic.$d.rx-cqe-frames" "$c" "$w"
    fi
    for f in napi_defer_hard_irqs gro_flush_timeout; do
      [[ -e $R/sys/class/net/$d/$f ]] && setting "nic.$d.$f" "$(cat "$R/sys/class/net/$d/$f")" "${fair:+0}"
    done
  done
  if [[ -n $hk ]]; then
    want=$(nic_irq_want "$hk")
    for q in $(nic_irqs); do
      name=$(awk -v q="$q:" '$1 == q {print $NF; exit}' "$R/proc/interrupts")
      f=$R/proc/irq/$q/effective_affinity_list
      [[ -s $f ]] || f=$R/proc/irq/$q/smp_affinity_list
      have=$(cat "$f" 2> /dev/null || echo "?")
      # Where the kernel delivers it must be within the CPUs wanted.
      ok=1
      for c in $(expand_cpus "$have"); do [[ " $(expand_cpus "$want") " == *" $c "* ]] || ok=""; done
      if [[ -n $ok ]]; then setting "irq.$q.$name" "$have"; else setting "irq.$q.$name" "$have" "$want"; fi
    done
  fi
  if [[ -z $R ]] && command -v systemctl > /dev/null; then
    setting irqbalance "$(systemctl is-active irqbalance 2> /dev/null || true)" "$([[ -n $hk ]] && echo inactive)"
  fi
  if (( ${#drift[@]} )); then
    echo "check: DRIFT ${#drift[@]}: $(IFS=';'; echo "${drift[*]}")"
    exit 3
  fi
  echo "check: ok"
  exit 0
fi

# Trace events `trace start` asks for that this kernel lacks, with why.
missing_trace_events() {
  local T=/sys/kernel/tracing e
  [[ -d $T/events ]] || mount -t tracefs nodev "$T" 2>/dev/null || true
  [[ -d $T/events ]] || T=/sys/kernel/debug/tracing
  [[ -d $T/events ]] || { echo "tracefs not available: the runs' kernel traces will be empty"; return 0; }
  for e in irq_vectors exceptions/page_fault_user exceptions/page_fault_kernel; do
    [[ -e $T/events/$e ]] && continue
    case $e in
      irq_vectors) echo "$e: x86 interrupt vectors (arm64 traces the tick as irq arch_timer and IPIs as ipi events)" ;;
      exceptions/*) echo "$e: an x86-only tracepoint (page faults are not attributed here; the loops mlock and pre-fault)" ;;
    esac
  done
}

if [[ $cmd == tune ]]; then
  pinned=""
  rm -f "$STATE/want-tune-nic"
  if [[ ${2:-} != stock ]]; then
    # Busy polling is opted into per variant (SO_BUSY_POLL, EPIOCSPARAMS, NAPI
    # registration), never through these global defaults.
    sysctl -qw net.core.busy_read=0 net.core.busy_poll=0
    routes "quickack 1 rto_min 5ms"
    # nic: adaptive receive moderation off, held for the whole run (pin_nic).
    [[ ${2:-} == nic ]] && : > "$STATE/want-tune-nic"
    # Every movable IRQ to the housekeeping CPUs, the NIC as the run wants it.
    pinned=$(pin_irqs; pin_nic)
  fi
  # A report: an unsupported query (ethtool on a synthetic NIC) must not stop it.
  set +e +o pipefail
  echo "== arch";         echo "$ARCH, MIDR_EL1 $(cat /sys/devices/system/cpu/cpu0/regs/identification/midr_el1 2>/dev/null || echo -)"
  echo "== uname";        uname -a
  echo "== cmdline";      cat /proc/cmdline
  echo "== boot profile"; cat "$STATE/boot-profile" 2>/dev/null || echo unknown
  echo "== isolated";     cat /sys/devices/system/cpu/isolated
  echo "== clocksource";  cat /sys/devices/system/clocksource/clocksource0/current_clocksource
  echo "== mitigations";  grep -r . /sys/devices/system/cpu/vulnerabilities/ 2>/dev/null | sed 's|.*/||' | head -20
  echo "== cpuidle";      echo "driver $(cat /sys/devices/system/cpu/cpuidle/current_driver 2>/dev/null)"
  for st in /sys/devices/system/cpu/cpu0/cpuidle/state[0-9]*; do
    [[ -e $st/name ]] && echo "$(cat "$st/name") latency $(cat "$st/latency") µs disabled $(cat "$st/disable")"
  done
  if [[ $ARCH == aarch64 ]]; then
    echo "$ARCH: WFI is the idle instruction (Graviton offers the OS no idle or frequency states: EC2 User Guide, processor state control)"
  fi
  if grep -qw nohlt /proc/cmdline; then echo "nohlt: idle CPUs poll"; else echo "nohlt: no"; fi
  echo "== counter"
  if [[ $ARCH == aarch64 ]]; then
    dmesg 2>/dev/null | grep -m3 -iE 'arch_timer' || echo "no arch_timer line left in dmesg"
  else
    dmesg 2>/dev/null | grep -m3 -iE 'tsc: (Detected|Refined)|clocksource.*tsc' || echo "no tsc line left in dmesg"
  fi
  echo "== smt";          cat /sys/devices/system/cpu/smt/active 2>/dev/null || echo "no SMT control ($ARCH: a vCPU is a core)"
  echo "== nohz_full";    cat /sys/devices/system/cpu/nohz_full 2>/dev/null || echo "absent: kernel built without NO_HZ_FULL"
  echo "== memory";       echo "THP $(cat /sys/kernel/mm/transparent_hugepage/enabled 2>/dev/null), defrag $(cat /sys/kernel/mm/transparent_hugepage/defrag 2>/dev/null), page size $(getconf PAGESIZE)"
  echo "== cpu";          lscpu | grep -E 'Model name|^CPU\(s\)|Thread|Core|Socket|NUMA node0|Hypervisor'
  echo "== topology";     lscpu -p=CPU,CORE | grep -v '^#'
  echo "== routes";       ip route show
  echo "== links";        ip -br link
  for i in $(nics); do
    echo "== ethtool -i $i"; ethtool -i "$i" 2>/dev/null | head -3
    echo "== ethtool -l $i"; ethtool -l "$i" 2>/dev/null | tail -5
    # Every moderation setting the NIC shows (MANA: RX CQE coalescing too),
    # or that it has none.
    echo "== ethtool -c $i"; ethtool -c "$i" 2>&1 | grep -E 'Adaptive|rx-usecs|tx-usecs|rx-frames|cqe|not supported' | head -8
    echo "== ethtool -T $i"; ethtool -T "$i" 2>/dev/null | head -20
  done
  echo "== nic irqs";     for q in $(nic_irqs); do echo "$q $(cat /proc/irq/$q/smp_affinity_list 2>/dev/null) $(grep -E "^ *$q:" /proc/interrupts | awk '{print $NF}')"; done
  echo "== chrony";       chronyc tracking 2>/dev/null || true
  echo "== chrony sources"; chronyc sources 2>/dev/null || true
  echo "== io_uring";     sysctl kernel.io_uring_disabled 2>/dev/null || echo "kernel.io_uring_disabled absent"
  echo "== nic held";     if [[ -e $STATE/want-fair ]]; then echo "fair: moderation off where the NIC has it"; elif [[ -e $STATE/want-tune-nic ]]; then echo "adaptive-rx off (tune nic)"; else echo "driver defaults"; fi
  echo "== pin";          echo "${pinned:-not run (boot profile stock)}"
  echo "== skipped on $ARCH"
  if [[ $ARCH == x86_64 ]]; then
    echo "boot profile full-nohlt: arm64 only"
  else
    echo "msr-tools and the SMI count (irqs): x86 MSRs"
    echo "cstate cap (cond cstate): no OS-controlled idle states; boot profile full-nohlt instead"
  fi
  missing_trace_events
  exit 0
fi

if [[ $cmd == cond ]]; then
  name=${2:?condition}; state=${3:?on|off}; cpu=${4:-}
  saved=$STATE/cond-$name
  case $name:$state in
    housekeeping:on)
      {
        echo "wq=$(cat /sys/devices/virtual/workqueue/cpumask 2>/dev/null)"
        echo "nmi=$(sysctl -n kernel.nmi_watchdog 2>/dev/null || echo 0)"
        echo "wd=$(sysctl -n kernel.watchdog 2>/dev/null || echo 1)"
        echo "stat=$(sysctl -n vm.stat_interval)"
      } > "$saved"
      hk=$(hk_cpus); [[ -n $hk ]] || hk=0
      echo "$(mask "$hk")" > /sys/devices/virtual/workqueue/cpumask || echo "WARN: workqueue cpumask"
      sysctl -qw kernel.nmi_watchdog=0 kernel.watchdog=0 vm.stat_interval=120 || echo "WARN: sysctl"
      ;;
    housekeeping:off)
      # shellcheck disable=SC1090
      source "$saved"
      [[ -n ${wq:-} ]] && echo "$wq" > /sys/devices/virtual/workqueue/cpumask
      sysctl -qw kernel.nmi_watchdog="$nmi" kernel.watchdog="$wd" vm.stat_interval="$stat" || true
      ;;
    prefer:on)
      : > "$saved"
      for d in $(nics); do
        [[ -e /sys/class/net/$d/napi_defer_hard_irqs ]] || continue
        echo "$d $(cat /sys/class/net/$d/napi_defer_hard_irqs) $(cat /sys/class/net/$d/gro_flush_timeout)" >> "$saved"
        echo 100 > "/sys/class/net/$d/napi_defer_hard_irqs"
        echo 200000 > "/sys/class/net/$d/gro_flush_timeout"
      done
      ;;
    prefer:off)
      while read -r d defer flush; do
        echo "$defer" > "/sys/class/net/$d/napi_defer_hard_irqs"
        echo "$flush" > "/sys/class/net/$d/gro_flush_timeout"
      done < "$saved"
      ;;
    noquickack:on) routes "rto_min 5ms" ;;
    noquickack:off) routes "quickack 1 rto_min 5ms" ;;
    nogro:on) for d in $(nics); do ethtool -K "$d" gro off 2>/dev/null || echo "WARN: $d gro off unsupported"; done ;;
    nogro:off) for d in $(nics); do ethtool -K "$d" gro on 2>/dev/null || true; done ;;
    nocoalesce:on)
      : > "$saved"
      : > "$STATE/want-cond-nocoalesce"
      for d in $(nics); do
        a=$(ethtool -c "$d" 2>/dev/null | awk '/Adaptive RX/ {print $3}' || true)
        u=$(ethtool -c "$d" 2>/dev/null | awk '/^rx-usecs:/ {print $2}' || true)
        [[ -n $u ]] || { echo "WARN: $d reports no interrupt moderation"; continue; }
        echo "$d ${a:-off} $u" >> "$saved"
        ethtool -C "$d" adaptive-rx off rx-usecs 0 2>/dev/null || echo "WARN: $d rx-usecs 0 unsupported"
      done
      ;;
    nocoalesce:off)
      rm -f "$STATE/want-cond-nocoalesce"
      while read -r d a u; do ethtool -C "$d" adaptive-rx "$a" rx-usecs "$u" 2>/dev/null || true; done < "$saved"
      ;;
    irqlocal:on)
      : > "$saved"
      # `pin` keeps the NIC queues here while this file exists.
      echo "${cpu:?cpu}" > "$STATE/irqlocal-cpu"
      for q in $(nic_irqs); do
        echo "$q $(cat "$R/proc/irq/$q/smp_affinity_list")" >> "$saved"
        echo "$cpu" > "$R/proc/irq/$q/smp_affinity_list" 2>/dev/null || echo "WARN: irq $q cannot move (managed)"
      done
      ;;
    irqlocal:off)
      rm -f "$STATE/irqlocal-cpu"
      while read -r q aff; do echo "$aff" > "$R/proc/irq/$q/smp_affinity_list" 2>/dev/null || true; done < "$saved"
      ;;
    cstate:on)
      : > "$saved"
      for st in /sys/devices/system/cpu/cpu[0-9]*/cpuidle/state[0-9]*; do
        [[ -e $st/latency ]] || continue
        if (( $(cat "$st/latency") > 1 )); then
          echo "$st $(cat "$st/disable")" >> "$saved"
          echo 1 > "$st/disable"
        fi
      done
      if [[ ! -s $saved ]]; then
        why=$(cat /sys/devices/system/cpu/cpuidle/current_driver 2>/dev/null || echo no cpuidle driver)
        [[ $ARCH == aarch64 ]] && why+="; $ARCH: WFI only, boot profile full-nohlt keeps idle CPUs polling"
        echo "WARN: no idle state deeper than C1 to disable ($why)"
      fi
      ;;
    cstate:off)
      while read -r st dis; do echo "$dis" > "$st/disable"; done < "$saved"
      ;;
    noagent:on)
      : > "$saved"
      for u in walinuxagent hv-kvp-daemon snap.amazon-ssm-agent.amazon-ssm-agent amazon-ssm-agent amazon-cloudwatch-agent; do
        if systemctl is-active --quiet "$u"; then
          echo "$u" >> "$saved"
          systemctl stop "$u"
        fi
      done
      [[ -s $saved ]] || echo "WARN: no cloud guest agent was running"
      ;;
    noagent:off)
      while read -r u; do systemctl start "$u" || echo "WARN: $u did not restart"; done < "$saved"
      ;;
    nodim:on)
      : > "$saved"
      : > "$STATE/want-cond-nodim"
      for d in $(nics); do
        a=$(ethtool -c "$d" 2>/dev/null | awk '/Adaptive RX/ {print $3}' || true)
        [[ -n $a ]] || { echo "WARN: $d reports no adaptive moderation"; continue; }
        echo "$d $a" >> "$saved"
        ethtool -C "$d" adaptive-rx off 2>/dev/null || echo "WARN: $d adaptive-rx off unsupported"
      done
      ;;
    nodim:off)
      rm -f "$STATE/want-cond-nodim"
      while read -r d a; do ethtool -C "$d" adaptive-rx "$a" 2>/dev/null || true; done < "$saved"
      ;;
    onequeue:on)
      : > "$saved"
      for d in $(nics); do
        n=$(ethtool -l "$d" 2>/dev/null | awk '/^Current hardware settings/ {c = 1} c && /^Combined:/ {print $2; exit}')
        [[ -n $n ]] || { echo "WARN: $d reports no channels"; continue; }
        echo "$d $n" >> "$saved"
        ethtool -L "$d" combined 1 2>/dev/null || echo "WARN: $d combined 1 unsupported"
      done
      sleep 3
      ;;
    onequeue:off)
      while read -r d n; do ethtool -L "$d" combined "$n" 2>/dev/null || echo "WARN: $d combined $n not restored"; done < "$saved"
      sleep 3
      ;;
    noautocork:on)
      sysctl -n net.ipv4.tcp_autocorking > "$saved"
      sysctl -qw net.ipv4.tcp_autocorking=0
      ;;
    noautocork:off) sysctl -qw net.ipv4.tcp_autocorking="$(cat "$saved")" ;;
    rt:on) sysctl -qw kernel.sched_rt_runtime_us=-1 ;;
    rt:off) sysctl -qw kernel.sched_rt_runtime_us=950000 ;;
    *) echo "unknown condition $name:$state" >&2; exit 2 ;;
  esac
  # An ethtool change may have brought the interface down and up (-L does):
  # the driver's IRQ affinity and adaptive moderation are back, so pin again.
  case $name in nogro|nocoalesce|nodim|onequeue) pin_irqs; pin_nic ;; esac
  echo "cond $name $state"
  exit 0
fi

if [[ $cmd == trace ]]; then
  T=$R/sys/kernel/tracing
  [[ -d $T/events || -n $R ]] || mount -t tracefs nodev "$T" 2>/dev/null || true
  [[ -d $T/events ]] || T=$R/sys/kernel/debug/tracing
  case ${2:?start|stop} in
    start)
      # noirq (the fair mode's irqlocal points): no interrupt or softirq
      # events. There every packet's interrupt lands on the traced loop CPU,
      # so these events would add the tracer's cost to every packet of the
      # interrupt-driven backends alone. /proc/interrupts still counts them.
      noirq=""
      [[ ${4:-} == noirq ]] && noirq=1
      echo 0 > "$T/tracing_on"
      echo > "$T/trace"
      echo mono > "$T/trace_clock"
      echo "$(mask "${3:?cpu list}")" > "$T/tracing_cpumask"
      echo 32768 > "$T/buffer_size_kb"
      for e in irq/irq_handler_entry irq/irq_handler_exit irq/softirq_entry irq/softirq_exit \
               irq_vectors ipi sched/sched_switch timer/hrtimer_expire_entry timer/hrtimer_expire_exit \
               workqueue/workqueue_execute_start workqueue/workqueue_execute_end \
               exceptions/page_fault_user exceptions/page_fault_kernel; do
        if [[ -n $noirq && $e == irq/* ]]; then
          continue
        fi
        if [[ -e $T/events/$e/enable ]]; then
          echo 1 > "$T/events/$e/enable"
        else
          echo "trace: $e not on this kernel ($ARCH)" >&2
        fi
      done
      echo 1 > "$T/tracing_on"
      ;;
    stop)
      echo 0 > "$T/tracing_on"
      cat "$T/trace" > "${3:?file}"
      echo 0 > "$T/events/enable"
      echo "$(mask "$(online)")" > "$T/tracing_cpumask"
      echo > "$T/trace"
      ;;
  esac
  exit 0
fi

if [[ $cmd == fgraph ]]; then
  T=/sys/kernel/tracing
  [[ -d $T/events ]] || mount -t tracefs nodev "$T" 2>/dev/null || true
  [[ -d $T/events ]] || T=/sys/kernel/debug/tracing
  case ${2:?start|stop} in
    start)
      echo 0 > "$T/tracing_on"
      echo > "$T/trace"
      echo mono > "$T/trace_clock"
      echo "$(mask "${3:?cpu}")" > "$T/tracing_cpumask"
      echo 65536 > "$T/buffer_size_kb"
      echo "${4:?microseconds}" > "$T/tracing_thresh"
      echo function_graph > "$T/current_tracer"
      echo 1 > "$T/options/funcgraph-abstime" 2>/dev/null || true
      echo 1 > "$T/options/funcgraph-proc" 2>/dev/null || true
      echo 1 > "$T/tracing_on"
      ;;
    stop)
      echo 0 > "$T/tracing_on"
      cat "$T/trace" > "${3:?file}"
      echo nop > "$T/current_tracer"
      echo 0 > "$T/tracing_thresh"
      echo "$(mask "$(online)")" > "$T/tracing_cpumask"
      echo > "$T/trace"
      ;;
  esac
  exit 0
fi

if [[ $cmd == hwlat ]]; then
  T=/sys/kernel/tracing
  [[ -d $T/events ]] || mount -t tracefs nodev "$T" 2>/dev/null || true
  [[ -d $T/events ]] || T=/sys/kernel/debug/tracing
  case ${2:?start|stop} in
    start)
      echo 0 > "$T/tracing_on"
      echo > "$T/trace"
      echo mono > "$T/trace_clock"
      echo "$(mask "${3:?cpu list}")" > "$T/tracing_cpumask"
      echo "${4:?microseconds}" > "$T/tracing_thresh"
      # Sample 950 ms of every second, interrupts off, on every listed CPU at
      # once (per-cpu, kernel 5.14+), else one at a time.
      echo 1000000 > "$T/hwlat_detector/window"
      echo 950000 > "$T/hwlat_detector/width"
      echo per-cpu > "$T/hwlat_detector/mode" 2>/dev/null \
        || echo round-robin > "$T/hwlat_detector/mode" 2>/dev/null || true
      echo hwlat > "$T/current_tracer"
      echo 1 > "$T/tracing_on"
      ;;
    stop)
      echo 0 > "$T/tracing_on"
      cat "$T/trace" > "${3:?file}"
      echo nop > "$T/current_tracer"
      echo 0 > "$T/tracing_thresh"
      echo "$(mask "$(online)")" > "$T/tracing_cpumask"
      echo > "$T/trace"
      ;;
  esac
  exit 0
fi

if [[ $cmd == slowsys ]]; then
  case ${2:?start|stop} in
    start)
      cpu=${3:?cpu}; ns=$(( ${4:?microseconds} * 1000 )); out=${5:?file}
      command -v bpftrace > /dev/null || DEBIAN_FRONTEND=noninteractive apt-get install -y -qq bpftrace > /dev/null 2>&1
      # Kernel-side filter: only the loop CPU's syscalls are timed, and only
      # the slow ones are printed.
      nohup bpftrace -e "
        tracepoint:raw_syscalls:sys_enter /cpu == $cpu/ { @s[tid] = nsecs; @n[tid] = args->id; }
        tracepoint:raw_syscalls:sys_exit /cpu == $cpu && @s[tid]/ {
          \$d = nsecs - @s[tid];
          if (\$d > $ns) { printf(\"%llu %d %llu\\n\", nsecs, @n[tid], \$d); }
          delete(@s[tid]);
        }" > "$out" 2> "$out.err" < /dev/null &
      echo $! > "$STATE/slowsys.pid"
      sleep 3
      ;;
    stop)
      kill "$(cat "$STATE/slowsys.pid")" 2>/dev/null || true
      sleep 1
      ;;
  esac
  exit 0
fi

if [[ $cmd == netstats ]]; then
  # NIC and TCP counters: drops, the cloud's allowance counters (ENA:
  # pps/bw/conntrack_allowance_exceeded), retransmissions and timeouts.
  {
    for d in $(nics); do echo "== ethtool -S $d"; ethtool -S "$d" 2>/dev/null; done
    echo "== nstat"; nstat -az 2>/dev/null
  } > "${2:?file}"
  exit 0
fi

if [[ $cmd == clock ]]; then
  {
    echo "== time"; date -u +%Y-%m-%dT%H:%M:%S.%NZ
    if command -v chronyc > /dev/null; then
      echo "== tracking"; chronyc -n tracking 2>&1 || true
      echo "== sources"; chronyc -n sources 2>&1 || true
    else
      echo "== chronyc not installed"
    fi
  } > "${2:?file}"
  exit 0
fi

if [[ $cmd == irqs ]]; then
  cat /proc/interrupts > "${2:?file}"
  # Bare-metal Intel: each CPU's SMI count (MSR 0x34) as one more row, so
  # firmware pauses show next to the interrupts. VMs do not expose it, nor
  # does any other architecture (tune lists it as skipped).
  if [[ $ARCH == x86_64 ]] && { [[ -e /dev/cpu/0/msr ]] || modprobe msr 2> /dev/null; }; then
    if command -v rdmsr > /dev/null && smi=$(rdmsr -a -d 0x34 2> /dev/null); then
      echo " SMI: $(echo $smi)   System management interrupts" >> "$2"
    fi
  fi
  exit 0
fi
echo "unknown command $cmd" >&2; exit 2
