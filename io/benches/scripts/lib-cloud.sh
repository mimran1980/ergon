# shellcheck shell=bash
# Host access and preparation shared by azure-flags.sh and aws-flags.sh
# (sourced once RCV, SND, SND_PRIV, RUSER, SSHO, OUT and log are set).
#
# Run settings (environment):
#   NIC=stock|tuned   tuned: both hosts' NICs with adaptive receive moderation
#                     off, held there for the whole run (vm-setup.sh pin); for
#                     comparisons between instance types, set it the same way
#                     for every pair
#   RCV_BOOT=full     the receiver's boot profile (vm-setup.sh), e.g. full-nohlt
#                     on Graviton; MODE=fair: full-pollidle where the host
#                     allows it (FAIR_IDLE=poll, the default), else full
#   BUILD_RUSTFLAGS   RUSTFLAGS for the benchmark build on every host; default
#                     per host (build_rustflags)
# The hosts may differ in architecture (a Graviton receiver against the x86
# sender rig): the sender's programs are then built on the sender.
NIC=${NIC:-stock}
# (Set once: a run sources this file twice, the second time its snapshot.)
RCV_BOOT_GIVEN=${RCV_BOOT_GIVEN-${RCV_BOOT:-}}
RCV_BOOT=${RCV_BOOT:-full}
FAIR_IDLE=${FAIR_IDLE:-poll}

rssh() { ssh "${SSHO[@]}" "$RUSER@$RCV" "$@"; }
sssh() { ssh "${SSHO[@]}" "$RUSER@$SND" "$@"; }
# Copies host1:path1 to host2:path2 through this machine.
relay() {
  local tmp="$OUT/relay.$$"
  scp "${SSHO[@]}" "$RUSER@$1:$2" "$tmp" > /dev/null && scp "${SSHO[@]}" "$tmp" "$RUSER@$3:$4" > /dev/null && rm -f "$tmp"
}
# A detached job on a host (ssh function $1 runs command $2), which a dropped
# session cannot kill: hstart starts it and prints its id; hwait polls it with
# short ssh calls, prints its output and returns its exit code. Two hosts'
# jobs run at once by starting both before waiting for either, never by a
# local background poller: one would outlive a run that stops early and poll
# the deleted host for ever.
hstart() {
  local id
  id="job$(date +%s)$RANDOM"
  "$1" "mkdir -p ~/jobs; nohup bash -c $(printf '%q' "($2); echo \$? > ~/jobs/$id.rc") > ~/jobs/$id.log 2>&1 < /dev/null &" >&2
  echo "$id"
}
hwait() {
  local rc=""
  until [[ -n $rc ]]; do
    sleep 3
    rc=$("$1" "cat ~/jobs/$2.rc 2>/dev/null" 2>/dev/null || true)
  done
  until "$1" "cat ~/jobs/$2.log"; do sleep 5; done
  return "$rc"
}
# A detached job on the receiver, waited for.
rrun() { hwait rssh "$(hstart rssh "$1")"; }
# `vm-setup.sh tune`'s argument for this run's NIC setting.
tune_arg() { [[ $NIC == tuned ]] && echo nic || true; }
wait_ssh() { for _ in $(seq 90); do ssh "${SSHO[@]}" "$RUSER@$1" true 2>/dev/null && return 0; sleep 5; done; return 1; }

# CPUs of the receiver's topology ($TOPO: `lscpu -p=CPU,CORE` lines).
cpu_of_core() { echo "$TOPO" | awk -F, -v c="$1" '$2==c {print $1; exit}'; }
siblings_of_core() { echo "$TOPO" | awk -F, -v c="$1" '$2==c {print $1}' | paste -sd, -; }

# The receiver's CPU roles from $TOPO and its architecture $RARCH: HK
# (housekeeping: every IRQ, the kernel's own work, ssh), ISO (isolated), CPU_A
# (the loop), CPU_SIB (its hyperthread sibling, idle; empty without SMT),
# CPU_SPARE (an idle isolated core for the spare-core probe and a second
# loop), CPU_SQ (the SQPOLL thread), SMT (yes/no). Housekeeping is core 0; on
# Graviton ("vCPUs map 1:1 to physical cores", aws-graviton-getting-started,
# perfrunbook/configuring_your_sut.md) core 0 is a single CPU, so core 1 joins
# it when at least three cores stay isolated: two housekeeping CPUs, as c7i's
# core 0 gives with its two hyperthreads. x86 keeps the layout its published
# runs had, with or without SMT (c7a: core 0 alone). The loop, spare and
# SQPOLL CPUs are the first CPUs of the first three isolated cores; cpu_map
# may reorder them.
cpu_roles() {
  local cores hk_cores c
  mapfile -t cores < <(echo "$TOPO" | awk -F, '{print $2}' | sort -un)
  SMT=no
  [[ $(echo "$TOPO" | awk -F, '{print $2}' | sort -n | uniq -d | head -1) == "" ]] || SMT=yes
  hk_cores=1
  [[ ${RARCH:-} == aarch64 && $SMT == no && ${#cores[@]} -ge 5 ]] && hk_cores=2
  HK=$(for c in "${cores[@]:0:hk_cores}"; do siblings_of_core "$c"; done | paste -sd, -)
  local iso_cores=("${cores[@]:hk_cores}")
  ISO=$(for c in "${iso_cores[@]}"; do siblings_of_core "$c"; done | paste -sd, -)
  CPU_A=$(cpu_of_core "${iso_cores[0]:-}")
  CPU_SIB=$(echo "$TOPO" | awk -F, -v a="$CPU_A" -v c="${iso_cores[0]:-}" '$2==c && $1!=a {print $1; exit}')
  CPU_SPARE=$(cpu_of_core "${iso_cores[1]:-}")
  CPU_SQ=$(cpu_of_core "${iso_cores[2]:-}")
}

# The fair mode's receiver roles (MODE=fair): housekeeping, the NIC
# interrupts, the loop and the SQPOLL thread each on a physical core of their
# own: HK core 0, CPU_IRQ core 1, CPU_A core 2, CPU_SQ core 3, CPU_SPARE core 4
# when there is one (LAYOUT=own-cores). Every CPU but housekeeping's is
# isolated, so only the NIC's interrupts run on the IRQ core; SMT siblings stay
# idle. Two cores with SMT (the x86 fallback, Standard_D4ds_v6) cannot do
# that: the IRQs take housekeeping's sibling and the SQPOLL thread the loop's
# (LAYOUT=smt-shared, as uring-sqpoll-sib measured it in 122127Z). Anything
# smaller fails.
fair_roles() {
  local cores c0 c1
  mapfile -t cores < <(echo "$TOPO" | awk -F, '{print $2}' | sort -un)
  SMT=no
  [[ $(echo "$TOPO" | awk -F, '{print $2}' | sort -n | uniq -d | head -1) == "" ]] || SMT=yes
  CPU_SPARE=""
  if (( ${#cores[@]} >= 4 )); then
    LAYOUT=own-cores
    HK=$(siblings_of_core "${cores[0]}")
    ISO=$(for c in "${cores[@]:1}"; do siblings_of_core "$c"; done | paste -sd, -)
    CPU_IRQ=$(cpu_of_core "${cores[1]}")
    CPU_A=$(cpu_of_core "${cores[2]}")
    CPU_SQ=$(cpu_of_core "${cores[3]}")
    CPU_SPARE=$(cpu_of_core "${cores[4]:-}")
    CPU_SIB=$(echo "$TOPO" | awk -F, -v a="$CPU_A" -v c="${cores[2]}" '$2==c && $1!=a {print $1; exit}')
  elif (( ${#cores[@]} == 2 )) && [[ $SMT == yes ]]; then
    LAYOUT=smt-shared
    mapfile -t c0 < <(echo "$TOPO" | awk -F, -v c="${cores[0]}" '$2==c {print $1}')
    mapfile -t c1 < <(echo "$TOPO" | awk -F, -v c="${cores[1]}" '$2==c {print $1}')
    HK=${c0[0]} CPU_IRQ=${c0[1]} CPU_A=${c1[0]} CPU_SQ=${c1[1]} CPU_SIB=${c1[1]}
    ISO="$CPU_IRQ,$CPU_A,$CPU_SQ"
  else
    echo "the fair mode needs four cores, or two with SMT; this receiver has ${#cores[@]} (SMT $SMT)" >&2
    return 1
  fi
}

# The fair mode's boot profile for a host with SMT $1 (yes/no) and
# architecture $2: idle CPUs poll (full-pollidle) where that spins no
# sibling and FAIR_IDLE=poll; else full.
fair_boot() {
  if [[ $FAIR_IDLE == poll && ($1 == no || $2 == aarch64) ]]; then echo full-pollidle; else echo full; fi
}

# RUSTFLAGS for the benchmark build on a host of architecture $1 whose
# MIDR_EL1 is $2 (`-` when unknown). BUILD_RUSTFLAGS overrides. On Graviton,
# AWS recommends targeting the core ("-Ctarget-cpu=neoverse-n1" for Graviton2
# or newer: aws-graviton-getting-started, rust.md; per generation, c-c++.md:
# Graviton2 neoverse-n1, Graviton3 neoverse-v1, Graviton4 neoverse-v2,
# Graviton5 neoverse-v3), so each Arm Neoverse core (implementer 0x41) gets its
# own; any other aarch64 core is built for itself. x86 stays generic, as
# every published x86 run was built.
build_rustflags() {
  if [[ -n ${BUILD_RUSTFLAGS+set} ]]; then echo "$BUILD_RUSTFLAGS"; return; fi
  [[ $1 == aarch64 ]] || { echo ""; return; }
  local cpu=native
  if [[ $2 =~ ^0x[0-9a-fA-F]+$ ]] && (( ($2 >> 24 & 0xff) == 0x41 )); then
    case $(printf '%03x' $(( $2 >> 4 & 0xfff ))) in
      d0c) cpu=neoverse-n1 ;;
      d40) cpu=neoverse-v1 ;;
      d4f) cpu=neoverse-v2 ;;
      d84) cpu=neoverse-v3 ;;
    esac
  fi
  echo "-Ctarget-cpu=$cpu"
}

# One host's architecture, MIDR_EL1 (aarch64; `-` elsewhere) and CPU model,
# space-separated (the model last), through ssh function $1.
host_facts() {
  "$1" "echo \$(uname -m) \$(cat /sys/devices/system/cpu/cpu0/regs/identification/midr_el1 2>/dev/null || echo -) \$(lscpu | sed -n 's/^Model name: *//p' | head -1)"
}

# The benchmark build as a shell command for rrun/srun: `cargo build
# --release $1`, with RUSTFLAGS $3, then the binaries $2 (space-separated)
# copied to ~/bin; a non-empty $4 also runs the tests.
build_cmd() {
  local b cp=""
  for b in $2; do cp+="target/release/$b "; done
  echo "set -e; exec > build.log 2>&1
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq build-essential pkg-config >/dev/null
    command -v cargo >/dev/null || (curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal >/dev/null); . ~/.cargo/env
    mkdir -p ergon bin; tar -xzf src.tgz -C ergon; cd ergon/io/benches
    export RUSTFLAGS='$3'
    cargo build --release -q $1
    ${4:+cargo test --release -q}
    cp $cp~/bin/"
}

# CPU layout, packages, build on the receiver, full isolation, reboot, tune.
prepare_hosts() {
  wait_ssh "$RCV"; wait_ssh "$SND"
  TOPO=$(rssh "lscpu -p=CPU,CORE | grep -v '^#'")
  local rf sf
  rf=$(host_facts rssh); sf=$(host_facts sssh)
  RARCH=${rf%% *}; SARCH=${sf%% *}
  if [[ ${MODE:-} == fair ]]; then
    fair_roles || exit 1
    [[ -n $RCV_BOOT_GIVEN ]] || RCV_BOOT=$(fair_boot "$SMT" "$RARCH")
  else
    cpu_roles
  fi
  local rmidr smidr
  rmidr=$(echo "$rf" | awk '{print $2}'); smidr=$(echo "$sf" | awk '{print $2}')
  RFLAGS=$(build_rustflags "$RARCH" "$rmidr"); SFLAGS=$(build_rustflags "$SARCH" "$smidr")
  log "receiver: $rf; sender: $sf"
  log "receiver build RUSTFLAGS='$RFLAGS'; sender '$SFLAGS'"
  [[ $SMT == yes ]] || log "receiver: no SMT (a vCPU is a core): no hyperthread sibling, housekeeping $HK"
  STOPO=$(sssh "lscpu -p=CPU,CORE | grep -v '^#'")
  if echo "$STOPO" | awk -F, '$2==1 {f=1} END {exit !f}'; then
    # Two or more cores: the server loop gets core 1 to itself (its
    # hyperthread twin, if any, is isolated and idle).
    S_CPU=$(echo "$STOPO" | awk -F, '$2==1 {print $1; exit}')
    S_ISO=$(echo "$STOPO" | awk -F, '$2==1 {print $1}' | paste -sd, -)
    S_HK=$(echo "$STOPO" | awk -F, '$2!=1 {print $1}' | paste -sd, -)
  else
    # One core: the loop on its second hyperthread.
    S_CPU=$(echo "$STOPO" | awk -F, 'NR==2 {print $1}'); S_HK=$(echo "$STOPO" | awk -F, 'NR==1 {print $1}')
    [[ -n $S_CPU ]] || S_CPU=$S_HK
    S_ISO=$S_CPU
  fi
  log "receiver: housekeeping $HK, isolated $ISO, ${CPU_IRQ:+NIC IRQs $CPU_IRQ ($LAYOUT), }loop CPU $CPU_A (sibling ${CPU_SIB:-none}), SQPOLL $CPU_SQ, spare ${CPU_SPARE:-none}; sender CPU $S_CPU"

  # The sender is the rig, not what is measured, and must never be slow to
  # wake (lab notes: an echo server waiting on a CPU in C6, 190 µs exit on
  # c7i, made 0–16% of round trips 150–250 µs slower). x86: idle states
  # deeper than C1 are disabled after the reboot. Graviton has no idle states
  # the OS controls (fixed frequency, WFI only), so its idle CPUs poll instead
  # (nohlt, boot profile full-nohlt).
  local s_boot=full s_smt=no
  [[ $SARCH == aarch64 ]] && s_boot=full-nohlt
  [[ $(echo "$STOPO" | awk -F, '{print $2}' | sort -n | uniq -d | head -1) == "" ]] || s_smt=yes
  # The fair mode: the sender's idle CPUs poll too where no sibling spins.
  [[ ${MODE:-} == fair ]] && s_boot=$(fair_boot "$s_smt" "$SARCH")
  for h in "$RCV" "$SND"; do scp "${SSHO[@]}" "${SCRIPTS:-scripts}/vm-setup.sh" "$RUSER@$h:" > /dev/null; done
  rssh "sudo bash vm-setup.sh install $HK $ISO $RCV_BOOT" | while read -r l; do log "receiver: $l"; done
  sssh "sudo bash vm-setup.sh install $S_HK $S_ISO $s_boot" | while read -r l; do log "sender: $l"; done

  # The receiver builds everything; the sender's two programs are copied
  # when both hosts would build the same code, else built there (a Graviton
  # receiver against the x86 rig, or two Arm generations).
  local rkey="$RARCH $RFLAGS" skey="$SARCH $SFLAGS" sbuild=""
  [[ $RFLAGS == *native* ]] && rkey="$rkey $rf"
  [[ $SFLAGS == *native* ]] && skey="$skey $sf"
  [[ $rkey == "$skey" ]] || sbuild=1
  scp "${SSHO[@]}" "$OUT/src.tgz" "$RUSER@$RCV:" > /dev/null
  local sjob=""
  if [[ -n $sbuild ]]; then
    log "building on the receiver and, for its own architecture or core, the sender"
    scp "${SSHO[@]}" "$OUT/src.tgz" "$RUSER@$SND:" > /dev/null
    sjob=$(hstart sssh "$(build_cmd "--bin replay_server --bin noise_probe" "replay_server noise_probe" "$SFLAGS")")
  else
    log "building on the receiver"
  fi
  rrun "$(build_cmd --bins "io_bench http_bench replay_server noise_probe analyse mio_bench" "$RFLAGS" test)" \
    || { rssh "tail -40 build.log"; exit 1; }
  rssh "tail -3 build.log"
  if [[ -n $sjob ]]; then
    hwait sssh "$sjob" > "$OUT/build-snd.txt" 2>&1 || { sssh "tail -40 build.log"; log "sender build failed"; exit 1; }
  else
    for b in replay_server noise_probe; do
      scp "${SSHO[@]}" "$RUSER@$RCV:bin/$b" "$OUT/$b" > /dev/null
    done
    sssh "mkdir -p bin"
    scp "${SSHO[@]}" "$OUT/replay_server" "$OUT/noise_probe" "$RUSER@$SND:bin/" > /dev/null
    rm -f "$OUT/replay_server" "$OUT/noise_probe"
  fi

  log "rebooting for CPU isolation"
  rssh "sudo reboot" || true; sssh "sudo reboot" || true
  sleep 45; wait_ssh "$RCV"; wait_ssh "$SND"
  rssh "sudo bash vm-setup.sh tune $(tune_arg)" > "$OUT/env-rcv.txt"
  sssh "sudo bash vm-setup.sh tune $(tune_arg)" > "$OUT/env-snd.txt"
  if [[ $SARCH == aarch64 ]]; then
    if sssh "grep -qw nohlt /proc/cmdline"; then
      log "sender: idle CPUs poll (nohlt): never in WFI"
    else
      log "!!! sender: nohlt is not on its kernel command line: its idle CPUs execute WFI"
    fi
  else
    sssh "sudo bash vm-setup.sh cond cstate on" >> "$OUT/env-snd.txt" 2>&1 || log "sender: capping idle states failed"
    log "sender: idle states deeper than C1 disabled ($(grep -c WARN "$OUT/env-snd.txt" || true) warnings)"
  fi
  if [[ ${MODE:-} == fair ]]; then
    # Every backend's NIC and kernel settings, held for the whole run and
    # checked before and after every point (lib-matrix.sh fair_point).
    local h out
    for h in receiver sender; do
      if [[ $h == receiver ]]; then
        out=$(rssh "sudo bash vm-setup.sh fair $CPU_IRQ; sudo bash vm-setup.sh check" 2>&1) || true
        printf '%s\n' "$out" >> "$OUT/env-rcv.txt"
      else
        out=$(sssh "sudo bash vm-setup.sh fair; sudo bash vm-setup.sh check" 2>&1) || true
        printf '%s\n' "$out" >> "$OUT/env-snd.txt"
      fi
      { grep -E '^(fair|check|WARN)' <<< "$out" || true; } | while read -r l; do log "$h: $l"; done
      # Settings that cannot be held would fail every point: stop now.
      if ! grep -q '^check: ok' <<< "$out"; then
        log "!!! $h: the fair mode's settings are not as wanted after applying them; stopping before any point"
        exit 1
      fi
    done
  fi
  # Whether the receiver has idle states to cap (the cstate condition).
  # The remote `|| true` matters: an arm64 `nohlt` boot registers no cpuidle
  # states, the glob matches nothing, and `cat` would return 1 — which this
  # script's `set -e -o pipefail` turns into an abort mid-run. No states is
  # the answer "no", not an error.
  RCV_DEEP_IDLE=$(rssh "cat /sys/devices/system/cpu/cpu0/cpuidle/state*/latency 2>/dev/null || true" | awk '$1 > 1 {n++} END {print n ? "yes" : "no"}')
  log "receiver: arch $RARCH, boot profile $RCV_BOOT, NIC $NIC, idle states deeper than 1 µs: $RCV_DEEP_IDLE"
  log "receiver isolated CPUs: $(rssh cat /sys/devices/system/cpu/isolated)"
  cpu_map
}

# Gap map: a counter-only spinner (noise_probe) on every isolated CPU of both
# hosts at once for CPUMAP_S seconds (at least two minute marks), so the CPUs
# the once-a-minute stall reaches are known before anything is measured.
# Writes res/cpumap-receiver and (merged later) srvres/cpumap-sender, logs
# each CPU's longest gap and its gaps of 100 µs or more by second of the
# minute. PICK_CORE=quiet (the default) moves the loop to the isolated core
# whose worst gap is smallest, the SQPOLL thread to the next and the spare
# probe to the third; PICK_CORE=noisy (the default with MODE=diagnose) does
# the reverse, so the diagnosis watches a CPU the stall reaches; fixed keeps
# cores 1, 3 and 2.
cpu_map() {
  local secs=${CPUMAP_S:-130} summary
  summary='for d in DIR/cpu*; do awk -v cpu="${d##*/cpu}" '"'"'
      /^#/ { for (i = 2; i <= NF; i++) if ($i ~ /^max_ns=/) max = substr($i, 8) + 0; next }
      { if ($1 > max) max = $1; if ($1 >= 100000) { n++; s[int(($3 / 1e9) % 60)]++ } }
      END { at = ""; for (k in s) at = at sprintf(" :%02d x%d", k, s[k]); printf "%s %d %d%s\n", cpu, max / 1000, n, at }'"'"' "$d/stalls-0.txt"; done'
  log "gap map: a spinner on every isolated CPU of both hosts for $secs s"
  rssh "mkdir -p res; for c in ${ISO//,/ }; do sudo bin/noise_probe --cpu \$c --duration $secs --stall-us 20 --out res/cpumap-receiver/cpu\$c > /dev/null 2>&1 & done; wait" &
  local r=$!
  sssh "mkdir -p srvres; for c in ${S_ISO//,/ }; do sudo bin/noise_probe --cpu \$c --duration $secs --stall-us 20 --out srvres/cpumap-sender/cpu\$c > /dev/null 2>&1 & done; wait" &
  local s=$!
  wait "$r" "$s" || log "gap map: a spinner failed"
  rssh "sudo chown -R \$(id -un) res; ${summary//DIR/res/cpumap-receiver}" > "$OUT/cpumap-receiver.txt" || true
  sssh "sudo chown -R \$(id -un) srvres; ${summary//DIR/srvres/cpumap-sender}" > "$OUT/cpumap-sender.txt" || true
  local f
  for f in receiver sender; do
    while read -r cpu worst n at; do
      log "gap map $f CPU $cpu: longest ${worst} µs, $n gaps of 100 µs or more${at:+ at$at}"
    done < "$OUT/cpumap-$f.txt"
  done
  local pick=${PICK_CORE:-quiet}
  [[ -n ${PICK_CORE:-} || ${MODE:-} != diagnose ]] || pick=noisy
  [[ $pick == quiet || $pick == noisy ]] || return 0
  # The fair mode's IRQ core keeps its role; two cores with SMT have no
  # choice to make.
  if [[ ${LAYOUT:-} == smt-shared ]]; then
    log "gap map: two cores with SMT: CPU roles fixed"
    return 0
  fi
  local irq_core="" need=3
  if [[ -n ${CPU_IRQ:-} ]]; then
    irq_core=$(echo "$TOPO" | awk -F, -v c="$CPU_IRQ" '$1==c {print $2; exit}')
    need=2
  fi
  # Cores 1.. ranked by the worst gap of their CPUs.
  local ranked
  ranked=$(echo "$TOPO" | awk -F, -v map="$OUT/cpumap-receiver.txt" -v skip="$irq_core" '
    BEGIN { while ((getline l < map) > 0) { split(l, f, " "); w[f[1]] = f[2] } }
    $2 != 0 && $2 != skip && ($1 in w) { if (!($2 in c) || w[$1] > c[$2]) c[$2] = w[$1] }
    END { for (k in c) print c[k], k }' | if [[ $pick == noisy ]]; then sort -rn; else sort -n; fi | awk '{print $2}')
  local cores=($ranked)
  if (( ${#cores[@]} < need )); then
    log "gap map: fewer than $need isolated cores mapped; CPU roles unchanged"
    return 0
  fi
  CPU_A=$(cpu_of_core "${cores[0]}")
  CPU_SIB=$(echo "$TOPO" | awk -F, -v a="$CPU_A" -v c="${cores[0]}" '$2==c && $1!=a {print $1; exit}')
  CPU_SQ=$(cpu_of_core "${cores[1]}")
  CPU_SPARE=$(cpu_of_core "${cores[2]:-}")
  log "gap map ($pick core first): loop CPU $CPU_A (core ${cores[0]}, sibling ${CPU_SIB:-none}), SQPOLL $CPU_SQ, spare ${CPU_SPARE:-none}${CPU_IRQ:+, NIC IRQs $CPU_IRQ (kept)}"
}
