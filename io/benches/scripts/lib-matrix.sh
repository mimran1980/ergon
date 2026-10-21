# shellcheck shell=bash
# The settings matrix, shared by azure-flags.sh and aws-flags.sh (sourced).
#
# The caller provides: log, rssh, sssh, rrun, relay, wait_ssh; and RCV, SND,
# SND_PRIV, RUSER (the VMs' login), SSHO (ssh options), CPU_A (the loop CPU),
# CPU_SQ (the SQPOLL CPU), CPU_SIB (CPU_A's hyperthread sibling, empty without
# SMT, as on Graviton), CPU_SPARE (an idle isolated CPU, empty if there is
# none), HK, ISO, S_CPU, OUT, RARCH (the receiver's architecture),
# RCV_DEEP_IDLE (yes when it has idle states deeper than 1 µs), RFLAGS.
#
# Before every run both hosts are pinned again (vm-setup.sh pin): IRQs on
# their housekeeping CPUs and the NIC's interrupt moderation as the run set
# it, since the ENA driver resets both whenever the interface comes up.
#
# During every timed run the noise probe spins on CPU_SPARE (traced like the
# loop CPU): a loop stall that the spare core shares is the whole VM paused by
# the hypervisor, not the loop or its kernel work.
#
# Around every run both hosts' chrony state is kept with the run's results
# (`vm-setup.sh clock`: res/<run>/clock-{before,after}.txt and
# clock-{before,after}-sender.txt): one-way figures compare the two hosts'
# clocks, and analyse bounds their offset (and the slack for matching the
# server's stalls) from these.
#
# Every condition is measured the same way (`measure`): platform noise on the
# loop CPU, ping-pong round trips paced and back to back per backend, HTTP, and
# a market-data burst receive. Each run captures a kernel trace of the loop CPU
# (and of the server's CPU), /proc/interrupts before and after, and the loops'
# stall logs, so every slow iteration can be attributed.
#
# Durations give every percentile up to p99.99 about a million samples or
# more: 10,000 round trips/s for 120 s (1.2 M), back to back for 60 s (about
# 1.5 M), HTTP at 10,000/s for 120 s, the burst for 60 s (3 M messages). A
# maximum stays one event: stage 2 (`run_confirm`) measures the baseline and
# the chosen settings at the real order rate for 15 minutes each, A/B/A/B.
#
# Stage 1 order: baseline, two conditions, baseline, ... then the boot
# profiles; ROUNDS=2 repeats the conditions in reverse.

PACED_RATE=${PACED_RATE:-10000}
PACED_S=${PACED_S:-120}
B2B_S=${B2B_S:-60}
HTTP_S=${HTTP_S:-120}
NOISE_S=${NOISE_S:-120}
MD_S=${MD_S:-60}
ROUNDS=${ROUNDS:-1}
# mio (mio + rustls, the usual Rust way) runs wherever epoll-spin does.
ALL_BACKENDS=${ALL_BACKENDS:-recv-busy uring-sqpoll uring-spin epoll-spin mio}

# name(s, joined by +) | backends | extra bench flags | command prefix | needs
# (smt: a hyperthread sibling; cstates: idle states deeper than 1 µs)
CONDITIONS=(
  "housekeeping|recv-busy uring-sqpoll uring-spin|||"
  "prefer|recv-busy uring-sqpoll|--prefer-busy-poll||"
  "noquickack|recv-busy uring-spin|||"
  "nogro|recv-busy uring-spin|||"
  "nocoalesce|recv-busy uring-spin epoll-spin mio|||"
  "irqlocal|recv-busy uring-spin epoll-spin mio|||"
  "rt|recv-busy uring-spin||chrt -f 50|"
  "sibling|recv-busy uring-sqpoll uring-spin|||smt"
  "busypoll50|recv-busy|--busy-poll 50||"
  "budget64|recv-busy|--busy-poll-budget 64||"
  "cstate|recv-busy uring-sqpoll uring-spin epoll-spin mio|||cstates"
  "noagent|recv-busy uring-sqpoll uring-spin|||"
  "nodim|recv-busy uring-sqpoll uring-spin epoll-spin mio|||"
  "onequeue|recv-busy uring-sqpoll uring-spin|||"
  "noautocork|recv-busy uring-sqpoll|||"
)
# COND_ONLY="name ..." measures only those conditions (with their baselines).
if [[ -n ${COND_ONLY:-} ]]; then
  _keep=()
  for _c in "${CONDITIONS[@]}"; do
    [[ " $COND_ONLY " == *" ${_c%%|*} "* ]] && _keep+=("$_c")
  done
  CONDITIONS=("${_keep[@]}")
  unset _keep _c
fi

start_servers() {
  sssh "sudo pkill -x replay_server || true; mkdir -p srvstalls srvres; rm -f srvstalls/*
    nohup sudo bin/replay_server --listen 0.0.0.0:9444 --cert-out cert-echo.pem --mode echo --cpu $S_CPU --busy-poll 50 --stalls-dir \$HOME/srvstalls > srv-echo.log 2>&1 < /dev/null &
    nohup sudo bin/replay_server --listen 0.0.0.0:9445 --cert-out cert-http.pem --mode http --cpu $S_CPU --busy-poll 50 --stalls-dir \$HOME/srvstalls > srv-http.log 2>&1 < /dev/null &"
  sleep 3
  relay "$SND" cert-echo.pem "$RCV" cert-echo.pem
  relay "$SND" cert-http.pem "$RCV" cert-http.pem
}

# One run on the receiver with the loop CPU traced; with `server`, the server's
# CPU is traced too and its stall log for this connection is kept on the
# sender under srvres/<dir> (merged into the receiver's results at the end).
# With `secs`, the noise probe spins on CPU_SPARE for the run (res/<dir>/spare).
# TRACE_EXTRA (set by the caller) adds CPUs to the trace. TRACE_NOIRQ=1 (the
# fair mode's irqlocal points) leaves the interrupt and softirq events out of
# it (vm-setup.sh trace start ... noirq).
# FAIR_CHECK=1 (the fair mode): after each host's pin, and again after the
# run, `vm-setup.sh check` keeps every setting the mode holds
# (res/<dir>/settings-{before,after}.txt and settings-*-sender.txt). Settings
# not as wanted before the run skip it; settings that drifted during it, or a
# run that failed, fail the point (fail_point: res/<dir>/failed.txt, which
# the analysis leaves out and lists).
POINT_FAILED=""
traced_run() {
  local dir=$1 cmd=$2 server=${3:-} secs=${4:-}
  local cpus=$CPU_A${TRACE_EXTRA:+,$TRACE_EXTRA} spare_on="" spare_off=""
  local s_before="" s_after="" r_before="" r_after="" safter="" rc=0
  POINT_FAILED=""
  if [[ -n ${FAIR_CHECK:-} ]]; then
    s_before="sudo bash vm-setup.sh check > srvres/$dir/settings-before-sender.txt; echo \"sender-before-\$(tail -1 srvres/$dir/settings-before-sender.txt)\""
    s_after="sudo bash vm-setup.sh check > srvres/$dir/settings-after-sender.txt; echo \"sender-after-\$(tail -1 srvres/$dir/settings-after-sender.txt)\""
    r_before="sudo bash vm-setup.sh check > res/$dir/settings-before.txt; ok=\$?; echo \"receiver-before-\$(tail -1 res/$dir/settings-before.txt)\""
    r_after="sudo bash vm-setup.sh check > res/$dir/settings-after.txt; echo \"receiver-after-\$(tail -1 res/$dir/settings-after.txt)\""
  fi
  if [[ -n $secs && -n ${CPU_SPARE:-} ]]; then
    cpus="$cpus,$CPU_SPARE"
    spare_on="sudo bin/noise_probe --cpu $CPU_SPARE --duration $((secs + 8)) --out res/$dir/spare > /dev/null 2>&1 & spare=\$!"
    spare_off="wait \$spare"
  fi
  if [[ -n $server ]]; then
    # Trace events this kernel lacks are listed once, in env-snd.txt.
    sssh "sudo bash vm-setup.sh trace start $S_CPU" > /dev/null 2>&1 || log "run $dir: sender trace did not start"
  fi
  local runlog spin
  runlog="$OUT/logs/$(echo "$dir" | tr / _).log"
  spin=$(sssh "mkdir -p srvres/$dir; sudo bash vm-setup.sh pin | tee srvres/$dir/pin-sender.txt
    $s_before
    sudo bash vm-setup.sh netstats srvres/$dir/net-before-sender.txt > /dev/null 2>&1
    sudo bash vm-setup.sh clock srvres/$dir/clock-before-sender.txt > /dev/null 2>&1" 2> /dev/null) || true
  if grep -qE '^pin: ([1-9][0-9]* of|.* again$)' <<< "$spin"; then
    log "run $dir: the sender's IRQs or NIC moderation had drifted and were pinned again: $(grep -E '^pin: ' <<< "$spin" | paste -sd ';' -)"
  fi
  rrun "mkdir -p res/$dir; sudo bash vm-setup.sh pin | tee res/$dir/pin.txt
    ok=0; $r_before
    sudo bash vm-setup.sh netstats res/$dir/net-before.txt; sudo bash vm-setup.sh irqs res/$dir/irq-before.txt
    sudo bash vm-setup.sh clock res/$dir/clock-before.txt; sudo bash vm-setup.sh trace start $cpus${TRACE_NOIRQ:+ noirq}
    $spare_on
    if [[ \$ok == 0 ]]; then
    $cmd; rc=\$?
    else echo 'not measured: the settings were not as wanted'; rc=97; fi
    $spare_off
    sudo bash vm-setup.sh trace stop res/$dir/trace.txt; sudo bash vm-setup.sh irqs res/$dir/irq-after.txt
    $r_after
    sudo bash vm-setup.sh clock res/$dir/clock-after.txt
    sudo bash vm-setup.sh netstats res/$dir/net-after.txt; sudo chown -R \$(id -un) res; exit \$rc" \
    > "$runlog" 2>&1 || rc=$?
  (( rc == 0 )) || log "run $dir failed (exit $rc, see logs)"
  if grep -qE '^pin: ([1-9][0-9]* of|.* again$)' "$runlog"; then
    log "run $dir: the receiver's IRQs or NIC moderation had drifted and were pinned again: $(grep -E '^pin: ' "$runlog" | paste -sd ';' -)"
  fi
  safter=$(sssh "sudo bash vm-setup.sh netstats srvres/$dir/net-after-sender.txt
    sudo bash vm-setup.sh clock srvres/$dir/clock-after-sender.txt
    $s_after
    sudo chown -R \$(id -un) srvres" 2> /dev/null) || true
  if [[ -n ${FAIR_CHECK:-} ]]; then
    local why="" k
    for k in receiver-before receiver-after; do
      grep -q "^$k-check: ok" "$runlog" || why+="$k: $(grep -m1 "^$k-check" "$runlog" || echo "no check"); "
    done
    grep -q '^sender-before-check: ok' <<< "$spin" || why+="sender-before: $(grep -m1 '^sender-before-check' <<< "$spin" || echo "no check"); "
    grep -q '^sender-after-check: ok' <<< "$safter" || why+="sender-after: $(grep -m1 '^sender-after-check' <<< "$safter" || echo "no check"); "
    (( rc == 0 )) || why+="run exit $rc; "
    [[ -z $why ]] || fail_point "$dir" "${why%; }"
  fi
  if [[ -n $server ]]; then
    sleep 1
    sssh "mkdir -p srvres/$dir; sudo bash vm-setup.sh trace stop srvres/$dir/server-trace.txt
      f=\$(ls -t srvstalls/server-$server-*.txt 2>/dev/null | head -1); [[ -n \$f ]] && sudo mv \$f srvres/$dir/; sudo chown -R \$(id -un) srvres" > /dev/null || true
  fi
}

# Marks point $1 failed for reason $2: logged, listed in failed-points.txt and
# kept with the run (res/<dir>/failed.txt), so the analysis leaves it out.
fail_point() {
  POINT_FAILED=$2
  log "!!! point $1 FAILED: $2"
  echo "$1: $2" >> "$OUT/failed-points.txt"
  rssh "mkdir -p res/$1; printf '%s\n' $(printf '%q' "$2") > res/$1/failed.txt" > /dev/null 2>&1 \
    || log "point $1: failed.txt not written"
}

# Market data: 50-message bursts every millisecond, one receiving loop.
burst_run() {
  local tag=$1 flags=$2 prefix=$3 secs=$4
  # Separate calls: a shell whose command line holds the start command would
  # match the pattern and kill itself.
  sssh "sudo pkill -f 'replay_serve[r] --listen 0.0.0.0:9446' || true" > /dev/null || true
  sssh "nohup sudo bin/replay_server --listen 0.0.0.0:9446 \
    --cert-out cert-burst.pem --clients 1 --mode burst --burst 50 --gap-us 1000 --cpu $S_CPU --duration $((secs + 30)) \
    > srv-burst.log 2>&1 < /dev/null &" || log "$tag: burst sender did not start"
  sleep 2
  relay "$SND" cert-burst.pem "$RCV" cert-burst.pem || log "$tag: burst certificate not copied"
  traced_run "$tag/md-recv-busy" "sudo $prefix bin/io_bench lab --server $SND_PRIV:9446 --ca cert-burst.pem --sni lab.local \
    --variants recv-busy --cpus $CPU_A --busy-poll 1 --wait-us 0 $flags --duration $secs --out res/$tag/md-recv-busy" "" "$secs"
  sssh "sudo pkill -f 'replay_serve[r] --listen 0.0.0.0:9446' || true" > /dev/null || true
}

pp_run() {
  local tag=$1 b=$2 rate=$3 secs=$4 flags=$5 prefix=$6
  # The SQPOLL kernel thread does the receiving and sending: trace its CPU too.
  local TRACE_EXTRA=""
  if [[ $b == uring-sqpoll ]]; then TRACE_EXTRA=$CPU_SQ; fi
  # recv-busy-pq: recv-busy reading one socket per NIC queue (Config::per_queue).
  if [[ $b == recv-busy-pq ]]; then
    traced_run "$tag/pp-$b-$rate" "sudo $prefix bin/io_bench pingpong --server $SND_PRIV:9444 --ca cert-echo.pem --sni lab.local \
      --variant recv-busy --per-queue --cpu $CPU_A --busy-poll 1 --wait-us 0 $flags --rate $rate --duration $secs \
      --out res/$tag/pp-$b-$rate" echo "$secs"
    return
  fi
  # uring-sqpoll-sib: the SQPOLL thread on the loop's hyperthread sibling.
  if [[ $b == uring-sqpoll-sib ]]; then
    if [[ -z ${CPU_SIB:-} ]]; then
      log "skip $tag $b: no hyperthread sibling (no SMT: a second vCPU is a whole core, which is uring-sqpoll)"
      return
    fi
    TRACE_EXTRA=$CPU_SIB
    traced_run "$tag/pp-$b-$rate" "sudo $prefix bin/io_bench pingpong --server $SND_PRIV:9444 --ca cert-echo.pem --sni lab.local \
      --variant uring-sqpoll --cpu $CPU_A --sqpoll-cpu $CPU_SIB --busy-poll 1 --wait-us 0 $flags --rate $rate --duration $secs \
      --out res/$tag/pp-$b-$rate" echo "$secs"
    return
  fi
  # mio: the usual Rust way (mio + rustls), its own client.
  if [[ $b == mio ]]; then
    traced_run "$tag/pp-$b-$rate" "sudo $prefix bin/mio_bench pingpong --server $SND_PRIV:9444 --ca cert-echo.pem --sni lab.local \
      --cpu $CPU_A --rate $rate --duration $secs --out res/$tag/pp-$b-$rate" echo "$secs"
    return
  fi
  traced_run "$tag/pp-$b-$rate" "sudo $prefix bin/io_bench pingpong --server $SND_PRIV:9444 --ca cert-echo.pem --sni lab.local \
    --variant $b --cpu $CPU_A --sqpoll-cpu $CPU_SQ --busy-poll 1 --wait-us 0 $flags --rate $rate --duration $secs \
    --out res/$tag/pp-$b-$rate" echo "$secs"
}

http_run() {
  local tag=$1 rate=$2 secs=$3 flags=$4 prefix=$5
  traced_run "$tag/http-recv-busy-$rate" "sudo $prefix bin/http_bench --client ergon --backend recv-busy --server $SND_PRIV:9445 \
    --ca cert-http.pem --sni lab.local --cpu $CPU_A $flags --rate $rate --duration $secs --out res/$tag/http-recv-busy-$rate" http "$secs"
}

# measure <tag> <backends> <flags> <prefix>
measure() {
  local tag=$1 backends=$2 flags=$3 prefix=$4 b
  log "measure $tag: noise, ping-pong ($backends), HTTP, burst"
  traced_run "$tag/noise" "sudo $prefix bin/noise_probe --cpu $CPU_A --duration $NOISE_S --out res/$tag/noise"
  for b in $backends; do
    pp_run "$tag" "$b" "$PACED_RATE" "$PACED_S" "$flags" "$prefix"
    pp_run "$tag" "$b" 0 "$B2B_S" "$flags" "$prefix"
  done
  if [[ $backends == *recv-busy* ]]; then
    http_run "$tag" "$PACED_RATE" "$HTTP_S" "$flags" "$prefix"
    burst_run "$tag" "$flags" "$prefix" "$MD_S"
  fi
}

# Applies (on) or removes (off) every setting in a +-joined name; the busy
# sibling is a load, not a setting.
apply_conds() {
  local names=$1 state=$2 tag=$3 n
  IFS='+' read -ra parts <<< "$names"
  for n in "${parts[@]}"; do
    # Client flags only (the condition's flags column): nothing on the host.
    case $n in busypoll*|budget*) continue ;; esac
    if [[ $n == sibling ]]; then
      if [[ $state == on ]]; then
        rssh "nohup sudo bin/noise_probe --cpu $CPU_SIB --duration 100000 --load > /dev/null 2>&1 < /dev/null &"
      else
        rssh "sudo pkill -x noise_probe || true"
      fi
      continue
    fi
    rssh "sudo bash vm-setup.sh cond $n $state $CPU_A" > "$OUT/logs/cond-$tag-$n-$state.log" 2>&1 \
      || log "cond $n $state failed: $(tail -1 "$OUT/logs/cond-$tag-$n-$state.log")"
    # The sender's idle states stay capped at C1 for the whole run
    # (prepare_hosts), so cstate tests the receiver alone.
    grep -h WARN "$OUT/logs/cond-$tag-$n-$state.log" | sed "s/^/[$tag] /" >> "$OUT/cond-warnings.txt" || true
  done
}

# condition <round> <spec>
condition() {
  local round=$1 name backends flags prefix needs
  IFS='|' read -r name backends flags prefix needs <<< "$2"
  if [[ $needs == smt && -z ${CPU_SIB:-} ]]; then
    log "skip $name: no hyperthread sibling on this instance type"
    return
  fi
  if [[ $needs == cstates && ${RCV_DEEP_IDLE:-yes} != yes ]]; then
    log "skip $name: the receiver has no idle state deeper than 1 µs to cap (${RARCH:-}; on Graviton the boot profile full-nohlt is the counterpart)"
    return
  fi
  local tag=r$round-$name
  apply_conds "$name" on "$tag"
  measure "$tag" "$backends" "$flags" "$prefix"
  apply_conds "$name" off "$tag"
}

baseline() { measure "r$1-base-$2" "$ALL_BACKENDS" "" ""; }

# reboot_into <profile>: reboot the receiver into it and tune it.
reboot_into() {
  local p=$1
  log "receiver boot profile: $p"
  rssh "sudo bash vm-setup.sh boot $p $HK $ISO $CPU_SQ" | tail -1 || log "boot $p: profile not set"
  rssh "sudo reboot" || true
  sleep 45
  wait_ssh "$RCV"
  if [[ $p == stock ]]; then
    rssh "sudo bash vm-setup.sh tune stock" > "$OUT/env-rcv-$p.txt" || log "boot $p: tune failed"
  else
    rssh "sudo bash vm-setup.sh tune $(tune_arg)" > "$OUT/env-rcv-$p.txt" || log "boot $p: tune failed"
  fi
  # A reboot cancels a scheduled power-off.
  if declare -F deadman > /dev/null; then deadman; fi
}

# boot_profile <profile>: reboot the receiver into it and measure the baseline set.
boot_profile() {
  reboot_into "$1"
  measure "boot-$1" "$ALL_BACKENDS" "" ""
}

# The SQPOLL CPU with and without nohz_full (boot profiles full and
# full-sqtick), SQTICK_REPEATS times A/B, each after a fresh boot: round trips
# paced and back to back for uring-sqpoll and recv-busy (the control), with
# the SQPOLL CPU traced and /proc/interrupts kept per run.
SQTICK_REPEATS=${SQTICK_REPEATS:-2}
run_sqtick() {
  mkdir -p "$OUT/logs"
  rssh "mkdir -p res"
  start_servers
  local k p b
  for ((k = 1; k <= SQTICK_REPEATS; k++)); do
    for p in full full-sqtick; do
      reboot_into "$p"
      log "sqtick $k: $p"
      for b in uring-sqpoll recv-busy; do
        pp_run "s$k-$p" "$b" "$PACED_RATE" "$PACED_S" "" ""
        pp_run "s$k-$p" "$b" 0 "$B2B_S" "" ""
      done
    done
  done
}

# Stage 1: every setting once (ROUNDS=2: again, in reverse), with baselines
# bracketing every two settings, then the boot profiles.
run_matrix() {
  mkdir -p "$OUT/logs"
  rssh "mkdir -p res"
  start_servers
  local i k round
  for ((round = 1; round <= ROUNDS; round++)); do
    log "round $round"
    k=0
    baseline "$round" 0
    for i in "${!CONDITIONS[@]}"; do
      if (( round % 2 == 0 )); then i=$((${#CONDITIONS[@]} - 1 - i)); fi
      condition "$round" "${CONDITIONS[$i]}"
      k=$((k + 1))
      if (( k % 2 == 0 )); then baseline "$round" "$k"; fi
    done
    if (( k % 2 == 1 )); then baseline "$round" "$k"; fi
    if (( round == 1 )) && [[ ${SKIP_BOOT:-} != 1 ]]; then
      boot_profile stock
      boot_profile isolcpus
      boot_profile full-nomitig
      # Idle CPUs polling instead of WFI: Graviton's only idle-state lever.
      if [[ ${RARCH:-} == aarch64 ]]; then
        boot_profile full-nohlt
      else
        log "skip boot profile full-nohlt: arm64 only (x86 has no nohlt)"
      fi
      boot_profile full
    fi
  done
}

# Stage 2: the real order rate for long enough that the maximum and p99.99
# mean something. CONFIRM is a ;-separated list of specs (as in CONDITIONS,
# "base" for the baseline); each runs CONFIRM_REPEATS times, interleaved
# A/B/A/B: round trips at CONFIRM_RATE (1,000/s) for CONFIRM_S per backend, the burst and the
# noise probe for a third of that.
CONFIRM_S=${CONFIRM_S:-900}
# The round-trip rate; a higher one finds rare tail events in fewer minutes.
CONFIRM_RATE=${CONFIRM_RATE:-1000}
CONFIRM_REPEATS=${CONFIRM_REPEATS:-2}
CONFIRM_BACKENDS=${CONFIRM_BACKENDS:-recv-busy uring-sqpoll}
run_confirm() {
  mkdir -p "$OUT/logs"
  rssh "mkdir -p res"
  start_servers
  local specs rep spec name backends flags prefix needs b
  IFS=';' read -ra specs <<< "${CONFIRM:?CONFIRM=<specs>}"
  for ((rep = 1; rep <= CONFIRM_REPEATS; rep++)); do
    for spec in "${specs[@]}"; do
      if [[ $spec == base ]]; then spec="base|$CONFIRM_BACKENDS|||"; fi
      IFS='|' read -r name backends flags prefix needs <<< "$spec"
      local tag=c$rep-$name
      [[ $name == base ]] || apply_conds "$name" on "$tag"
      log "confirm $tag: ${CONFIRM_S}s per backend at ${CONFIRM_RATE}/s"
      traced_run "$tag/noise" "sudo $prefix bin/noise_probe --cpu $CPU_A --duration $((CONFIRM_S / 3)) --out res/$tag/noise"
      for b in ${backends:-$CONFIRM_BACKENDS}; do
        pp_run "$tag" "$b" "$CONFIRM_RATE" "$CONFIRM_S" "$flags" "$prefix"
      done
      http_run "$tag" "$CONFIRM_RATE" "$CONFIRM_S" "$flags" "$prefix"
      burst_run "$tag" "$flags" "$prefix" "$((CONFIRM_S / 3))"
      [[ $name == base ]] || apply_conds "$name" off "$tag"
    done
  done
}

# Diagnosis of the stalls the timed runs cannot explain (DIAG_S seconds each,
# not timed comparisons: the tracing slows the loop):
#   hwlat: the kernel's hardware-latency detector on the loop CPU (interrupts
#     off; any gap is the hypervisor or firmware);
#   per backend, ping-pong at the paced rate with every syscall over 100 µs on
#     the loop CPU recorded (bpftrace), next to the usual stall log and trace;
#   per backend, the same with function_graph showing every kernel function
#     over 50 µs on the loop CPU (heavy).
DIAG_S=${DIAG_S:-120}
DIAG_BACKENDS=${DIAG_BACKENDS:-recv-busy uring-sqpoll uring-spin}
run_diagnose() {
  mkdir -p "$OUT/logs"
  rssh "mkdir -p res"
  start_servers
  local b
  log "diagnose: hardware latency on CPUs $ISO"
  rrun "mkdir -p res/diag/hwlat && printf 'mode=hwlat\ncpu=$ISO\nduration_s=$DIAG_S\n' > res/diag/hwlat/meta.txt
    sudo bash vm-setup.sh pin; sudo bash vm-setup.sh hwlat start $ISO 10; sleep $DIAG_S; sudo bash vm-setup.sh hwlat stop res/diag/hwlat/hwlat.txt; sudo chown -R \$(id -un) res" \
    > "$OUT/logs/diag-hwlat.log" 2>&1 || log "diagnose: hwlat failed"
  for b in $DIAG_BACKENDS; do
    log "diagnose: slow syscalls, $b"
    traced_run "diag/syscalls-pp-$b" "sudo bash vm-setup.sh slowsys start $CPU_A 100 res/diag/syscalls-pp-$b/slowsys.txt
      sudo bin/io_bench pingpong --server $SND_PRIV:9444 --ca cert-echo.pem --sni lab.local --variant $b --cpu $CPU_A \
        --sqpoll-cpu $CPU_SQ --busy-poll 1 --wait-us 0 --rate $PACED_RATE --duration $DIAG_S --out res/diag/syscalls-pp-$b
      sudo bash vm-setup.sh slowsys stop" echo
    log "diagnose: slow kernel functions, $b"
    rrun "mkdir -p res/diag/fgraph-pp-$b; sudo bash vm-setup.sh pin; sudo bash vm-setup.sh fgraph start $CPU_A 50
      sudo bin/io_bench pingpong --server $SND_PRIV:9444 --ca cert-echo.pem --sni lab.local --variant $b --cpu $CPU_A \
        --sqpoll-cpu $CPU_SQ --busy-poll 1 --wait-us 0 --rate $PACED_RATE --duration $((DIAG_S / 2)) --out res/diag/fgraph-pp-$b
      sudo bash vm-setup.sh fgraph stop res/diag/fgraph-pp-$b/fgraph.txt; sudo chown -R \$(id -un) res" \
      > "$OUT/logs/diag-fgraph-$b.log" 2>&1 || log "diagnose: fgraph $b failed"
  done
  # The once-a-minute stall: cloud guest agents running against stopped,
  # A/B/A/B, three minutes each (three occurrences), traced as usual, with
  # the loop on a CPU the gap map saw stall (PICK_CORE=noisy).
  local k state
  for k in 1 2; do
    for state in on off; do
      if [[ $state == off ]]; then apply_conds noagent on diag; fi
      log "diagnose: guest agents $state ($k)"
      pp_run "diag/agents-$state-$k" recv-busy "$PACED_RATE" 180 "" ""
      if [[ $state == off ]]; then apply_conds noagent off diag; fi
    done
  done
}

# HTTP clients (MODE=http): ergon-io's client on every backend and through
# the pooled HttpClient API, libcurl (the distro library and a source build
# with c-ares, -O3 and the build host's CPU: -march=native on x86,
# -mcpu=native on aarch64, where -march sets the architecture only and
# -mcpu also tunes for the core, GCC's AArch64 options; AWS's Graviton C/C++
# guide tunes with -mcpu) and reqwest, paced and back to back, HTTP_ROUNDS
# times with the client order reversed every other round.
HTTP_ROUNDS=${HTTP_ROUNDS:-2}
CURL_SRC=${CURL_SRC:-8.23.0}
build_http_clients() {
  log "building the HTTP clients on the receiver (reqwest; libcurl distro and $CURL_SRC from source)"
  rrun "set -e; exec > build-http.log 2>&1
    sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq libssl-dev libcurl4-openssl-dev libc-ares-dev cmake perl xz-utils zlib1g-dev > /dev/null
    . ~/.cargo/env; cd ergon/io/benches
    export RUSTFLAGS='${RFLAGS:-}'
    case \$(uname -m) in aarch64) native=-mcpu=native ;; *) native=-march=native ;; esac
    cargo build --release --bin http_bench --features reqwest --target-dir t-reqwest -q
    cp t-reqwest/release/http_bench ~/bin/http_bench_reqwest
    cargo build --release --bin http_bench --features curl --target-dir t-distro -q
    cp t-distro/release/http_bench ~/bin/http_bench_curl_distro
    cd ~; (curl -sSfLO https://curl.se/download/curl-$CURL_SRC.tar.xz || curl -sSfLO https://curl.se/download/curl-8.22.0.tar.xz)
    tar -xJf curl-*.tar.xz; cd curl-*/
    cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_FLAGS=\"-O3 \$native\" -DCMAKE_INSTALL_PREFIX=/opt/curl \\
      -DENABLE_ARES=ON -DENABLE_THREADED_RESOLVER=OFF -DHTTP_ONLY=ON -DCURL_USE_OPENSSL=ON -DUSE_NGHTTP2=OFF \\
      -DCURL_USE_LIBPSL=OFF -DCURL_USE_LIBSSH2=OFF -DCURL_BROTLI=OFF -DCURL_ZSTD=OFF -DUSE_LIBIDN2=OFF \\
      -DBUILD_CURL_EXE=OFF -DBUILD_SHARED_LIBS=ON -DBUILD_TESTING=OFF > /dev/null
    cmake --build build -j4 > /dev/null; sudo cmake --install build > /dev/null
    cd ~/ergon/io/benches
    PKG_CONFIG_PATH=/opt/curl/lib/pkgconfig cargo build --release --bin http_bench --features curl --target-dir t-source -q
    cp t-source/release/http_bench ~/bin/http_bench_curl_source" \
    || { rssh "tail -40 build-http.log"; return 1; }
}

run_http() {
  mkdir -p "$OUT/logs"
  rssh "mkdir -p res"
  build_http_clients || { log "HTTP client build failed"; exit 1; }
  start_servers
  local -A client=(
    [ergon-recv-busy]="bin/http_bench --client ergon --backend recv-busy"
    [ergon-uring-sqpoll]="bin/http_bench --client ergon --backend uring-sqpoll --sqpoll-cpu $CPU_SQ"
    [ergon-uring-spin]="bin/http_bench --client ergon --backend uring-spin"
    [ergon-epoll-spin]="bin/http_bench --client ergon --backend epoll-spin"
    [mio]="bin/mio_bench http --pool 1"
    [api-native-recv-busy]="bin/http_bench --client client --impl native --backend recv-busy"
    [curl-distro]="bin/http_bench_curl_distro --client curl --curl-label distro"
    [curl-source]="env LD_LIBRARY_PATH=/opt/curl/lib bin/http_bench_curl_source --client curl --curl-label source"
    [reqwest]="bin/http_bench_reqwest --client reqwest"
  )
  local order=(ergon-recv-busy ergon-uring-sqpoll ergon-uring-spin ergon-epoll-spin mio api-native-recv-busy curl-distro curl-source reqwest)
  local ok=() c round rate i TRACE_EXTRA
  log "http: smoke test, every client for 3 s (a client that fails is left out)"
  for c in "${order[@]}"; do
    if rssh "sudo ${client[$c]} --server $SND_PRIV:9445 --ca cert-http.pem --sni lab.local --cpu $CPU_A --rate 1000 --duration 3 --out res/smoke/http-$c" > "$OUT/logs/smoke-http-$c.log" 2>&1; then
      ok+=("$c")
    else
      log "http: leaving out $c (smoke test failed, see logs/smoke-http-$c.log)"
    fi
  done
  for ((round = 1; round <= HTTP_ROUNDS; round++)); do
    for rate in "$PACED_RATE" 0; do
      log "http round $round, ${rate}/s (0: back to back): ${ok[*]}"
      for i in "${!ok[@]}"; do
        if (( round % 2 == 0 )); then c=${ok[$((${#ok[@]} - 1 - i))]}; else c=${ok[$i]}; fi
        TRACE_EXTRA=""
        if [[ $c == *sqpoll* ]]; then TRACE_EXTRA=$CPU_SQ; fi
        traced_run "h$round/http-$c-$rate" "sudo ${client[$c]} --server $SND_PRIV:9445 --ca cert-http.pem --sni lab.local \
          --cpu $CPU_A --rate $rate --duration $HTTP_S --out res/h$round/http-$c-$rate" http "$HTTP_S"
      done
    done
  done
}

# Market data (MODE=md): two loops receive the same stream at once, each on
# its own core and connection, and every message is paired (which loop decoded
# it first, and by how much: no cross-host clock needed). Each pair of MD_PAIRS
# runs at 50-message bursts every millisecond and at one message every
# millisecond (where the interrupt path's wake-up shows), MD_ROUNDS times with
# the pairs in reverse order every other round. An A/A pair measures the bias
# of the two cores and connections themselves.
MD_PAIRS=${MD_PAIRS:-recv-busy,uring-sqpoll recv-busy,uring-spin recv-busy,epoll-spin uring-sqpoll,recv-busy recv-busy,recv-busy}
MD_ROUNDS=${MD_ROUNDS:-2}
MD_PAIR_S=${MD_PAIR_S:-90}
run_md() {
  mkdir -p "$OUT/logs"
  rssh "mkdir -p res"
  [[ -n ${CPU_SPARE:-} ]] || { log "md: no spare core for the second loop"; exit 1; }
  local pairs round pair i shape burst gap tag TRACE_EXTRA
  read -ra pairs <<< "$MD_PAIRS"
  for ((round = 1; round <= MD_ROUNDS; round++)); do
    for i in "${!pairs[@]}"; do
      if (( round % 2 == 0 )); then pair=${pairs[$((${#pairs[@]} - 1 - i))]}; else pair=${pairs[$i]}; fi
      for shape in burst50 single; do
        if [[ $shape == burst50 ]]; then burst=50; else burst=1; fi
        gap=1000
        tag=md$round/lab-${pair/,/-vs-}-$shape
        log "md round $round: $pair, $shape"
        sssh "sudo pkill -f 'replay_serve[r] --listen 0.0.0.0:9446' || true" > /dev/null || true
        sssh "nohup sudo bin/replay_server --listen 0.0.0.0:9446 --cert-out cert-burst.pem --clients 2 --mode burst \
          --burst $burst --gap-us $gap --cpu $S_CPU --duration $((MD_PAIR_S + 30)) > srv-burst.log 2>&1 < /dev/null &" \
          || log "md: burst sender did not start"
        sleep 2
        relay "$SND" cert-burst.pem "$RCV" cert-burst.pem || log "md: burst certificate not copied"
        TRACE_EXTRA="$CPU_SPARE,$CPU_SQ"
        traced_run "$tag" "sudo bin/io_bench lab --server $SND_PRIV:9446 --ca cert-burst.pem --sni lab.local \
          --variants $pair --cpus $CPU_A,$CPU_SPARE --sqpoll-cpu $CPU_SQ --busy-poll 1 --wait-us 0 \
          --duration $MD_PAIR_S --out res/$tag"
      done
    done
  done
  sssh "sudo pkill -f 'replay_serve[r] --listen 0.0.0.0:9446' || true" > /dev/null || true
}

# Many connections (MODE=many): MANY_CONNS (each count in the list) TLS WebSocket connections on one
# loop, each its own stream (replay_server --mode mixed: a trickle at each
# connection's own random interval of 1-10 ms, and every 2 ms a burst of 10
# messages to each of 10 random connections; about 68,000 messages a second).
# Each backend runs alone with the same traffic (a fixed seed), MANY_ROUNDS
# times in alternating order. The one-way numbers (sender clock -> decoded)
# include the hosts' clock offset: relative only. Both hosts' chrony state is
# kept with every run (traced_run), and analyse labels the one-way tables
# with the bound on that offset.
# MANY_CONNS is a list: the whole set runs per connection count.
MANY_CONNS=${MANY_CONNS:-1 10 100}
MANY_S=${MANY_S:-60}
MANY_ROUNDS=${MANY_ROUNDS:-3}
MANY_BACKENDS=${MANY_BACKENDS:-recv-busy recv-busy-pq uring-sqpoll uring-spin epoll-spin mio}
# MANY_STREAMS: streams sharing the connections, as on combined-stream
# sockets (empty: one stream per connection).
MANY_STREAMS=${MANY_STREAMS:-}
MANY_SERVER_FLAGS=${MANY_SERVER_FLAGS:---trickle-min-us 1000 --trickle-max-us 10000 --burst-every-us 2000 --burst-slots 10 --burst 10 --seed 42}
run_many() {
  mkdir -p "$OUT/logs"
  rssh "mkdir -p res"
  local backends round i b tag TRACE_EXTRA conns
  read -ra backends <<< "$MANY_BACKENDS"
  for ((round = 1; round <= MANY_ROUNDS; round++)); do
   for conns in $MANY_CONNS; do
    for i in "${!backends[@]}"; do
      if (( round % 2 == 0 )); then b=${backends[$((${#backends[@]} - 1 - i))]}; else b=${backends[$i]}; fi
      tag=m$round/lab-$b-c$conns${MANY_STREAMS:+-s$MANY_STREAMS}
      log "many round $round: $b, $conns connections"
      sssh "sudo pkill -f 'replay_serve[r] --listen 0.0.0.0:9447' || true" > /dev/null || true
      sssh "nohup sudo bin/replay_server --listen 0.0.0.0:9447 --cert-out cert-many.pem --clients $conns --mode mixed \
        $MANY_SERVER_FLAGS ${MANY_STREAMS:+--streams $MANY_STREAMS} --cpu $S_CPU --duration $((MANY_S + 60)) \
        > srv-many.log 2>&1 < /dev/null &" \
        || log "many: sender did not start"
      sleep 2
      relay "$SND" cert-many.pem "$RCV" cert-many.pem || log "many: certificate not copied"
      # Both hosts' chrony state is kept with the run itself (traced_run).
      TRACE_EXTRA=""
      if [[ $b == uring-sqpoll ]]; then TRACE_EXTRA=$CPU_SQ; fi
      if [[ $b == uring-sqpoll-sib ]]; then
        if [[ -z ${CPU_SIB:-} ]]; then
          log "skip $tag: no hyperthread sibling (no SMT: a second vCPU is a whole core, which is uring-sqpoll)"
          continue
        fi
        TRACE_EXTRA=$CPU_SIB
      fi
      local lab_cmd="bin/io_bench lab --variants $b --cpus $CPU_A --sqpoll-cpu $CPU_SQ --busy-poll 1 --wait-us 0"
      if [[ $b == mio ]]; then lab_cmd="bin/mio_bench lab --cpu $CPU_A"; fi
      # uring-sqpoll-sib: the SQPOLL thread on the loop's hyperthread sibling
      # (one physical core in all, as a pod with two vCPUs gets).
      if [[ $b == uring-sqpoll-sib ]]; then
        lab_cmd="bin/io_bench lab --variants uring-sqpoll --cpus $CPU_A --sqpoll-cpu $CPU_SIB --busy-poll 1 --wait-us 0"
      fi
      if [[ $b == recv-busy-pq ]]; then
        lab_cmd="bin/io_bench lab --variants recv-busy --per-queue --cpus $CPU_A --busy-poll 1 --wait-us 0"
      fi
      traced_run "$tag" "sudo $lab_cmd --server $SND_PRIV:9447 --ca cert-many.pem --sni lab.local \
        --conns $conns --capacity 8000000 --duration $MANY_S --out res/$tag" "" "$MANY_S"
      sssh "tail -2 srv-many.log" > "$OUT/logs/srv-many-$round-$b-c$conns.txt" 2>&1 || true
    done
   done
  done
  sssh "sudo pkill -f 'replay_serve[r] --listen 0.0.0.0:9447' || true" > /dev/null || true
}

# The fair comparison (MODE=fair): every backend on the same hosts, each
# with its own best tuning, the host settings held and checked before and
# after every point (FAIR_CHECK, traced_run), a point whose settings drifted
# failed and measured again (FAIR_RETRIES). The receiver's roles come from
# fair_roles (lib-cloud.sh): housekeeping, the NIC interrupts, the loop and
# the SQPOLL thread each on a core of its own. Conditions (FAIR_CONDS):
#   irqcore   every NIC queue interrupt on the IRQ core, every backend
#   irqlocal  every NIC queue interrupt on the loop's own CPU, the
#             interrupt-driven backends only (FAIR_IRQLOCAL_BACKENDS)
# Points, per round and condition (FAIR_ROUNDS; the backend order and the
# condition order reverse every other round):
#   md    market data on FAIR_MD_CONNS connections, FAIR_MD_STREAMS streams
#         over them (replay_server --mode mixed: the same traffic for every
#         connection count), FAIR_MD_S seconds; analysed as sender -> decoded
#         (the whole path: the only cross-family comparison) and, within a
#         family only, kernel arrival -> decoded
#   rt    the WebSocket order round trip on 1 connection at FAIR_RT_RATES while
#         N-1 more connections on the same loop carry the market data
#         (N in FAIR_RT_CONNS; one server thread answers the order connection
#         between any two feed sends), FAIR_RT_S seconds
#   http  the HTTP order round trip through a pool of P connections (P in
#         FAIR_HTTP_POOLS; requests take the pool's connections in turn), the
#         server answering every connection from one thread, FAIR_HTTP_S
#         seconds
FAIR_ROUNDS=${FAIR_ROUNDS:-2}
FAIR_CONDS=${FAIR_CONDS:-irqcore irqlocal}
FAIR_BACKENDS=${FAIR_BACKENDS:-epoll-spin mio uring-spin recv-busy recv-busy-pq uring-sqpoll}
FAIR_IRQLOCAL_BACKENDS=${FAIR_IRQLOCAL_BACKENDS:-epoll-spin mio uring-spin}
FAIR_KINDS=${FAIR_KINDS:-md rt http}
FAIR_MD_CONNS=${FAIR_MD_CONNS:-1 4 16 64 256}
FAIR_MD_STREAMS=${FAIR_MD_STREAMS:-1024}
# The market data's schedule (replay_server --mode mixed): each of the 1024
# streams a trickle at its own mean interval of 10-100 ms (about 26,000
# messages a second in all) and every 2 ms 5 messages to each of 10 random
# streams (25,000 a second): about 51,000 a second whatever the connection
# count, well within what the sender holds (it logs how late it ran:
# fair_point warns when over 1% of its sends began 100 µs late). MANY_SERVER_FLAGS'
# 1-10 ms trickle over 1024 streams asks for about 310,000 a second, more than
# a sender holds, so its rate followed the sender and the receiver instead.
FAIR_MD_SERVER_FLAGS=${FAIR_MD_SERVER_FLAGS:---trickle-min-us 10000 --trickle-max-us 100000 --burst-every-us 2000 --burst-slots 10 --burst 5 --seed 42}
FAIR_MD_S=${FAIR_MD_S:-60}
FAIR_RT_CONNS=${FAIR_RT_CONNS:-1 4 16 64}
FAIR_RT_RATES=${FAIR_RT_RATES:-10000}
FAIR_RT_S=${FAIR_RT_S:-60}
FAIR_HTTP_POOLS=${FAIR_HTTP_POOLS:-1 20}
FAIR_HTTP_RATE=${FAIR_HTTP_RATE:-10000}
FAIR_HTTP_S=${FAIR_HTTP_S:-60}
FAIR_RETRIES=${FAIR_RETRIES:-1}
# Set-up and copying around each point (server start, certificate relay,
# pins, checks, traces, polling): for the time estimate only.
FAIR_POINT_OVERHEAD_S=${FAIR_POINT_OVERHEAD_S:-35}

# polling or interrupt: the family of backend $1.
fair_family() {
  case $1 in
    recv-busy | recv-busy-pq | uring-sqpoll) echo polling ;;
    *) echo interrupt ;;
  esac
}

# What backend $1 sets for itself (every other setting is the host's, the
# same for all).
fair_backend_desc() {
  case $1 in
    epoll-spin) echo "ergon-io: epoll_wait(0) in a spin loop, SO_BUSY_POLL 0 on its sockets: purely interrupt driven" ;;
    mio) echo "mio + rustls (the usual Rust way): Poll::poll(0) (epoll) in a spin loop, no busy polling: purely interrupt driven" ;;
    uring-spin) echo "ergon-io: io_uring submitted and reaped without waiting, no NAPI registration: interrupt driven" ;;
    recv-busy) echo "ergon-io: a non-blocking recvmsg on every socket every poll, SO_BUSY_POLL 1 µs: each empty read polls the NIC queue once" ;;
    recv-busy-pq) echo "ergon-io: recv-busy reading one socket per NIC queue, then epoll_wait(0) for the rest (Config::per_queue)" ;;
    uring-sqpoll) echo "ergon-io: io_uring with its SQPOLL kernel thread on its own core (sqpoll_cpu), NAPI busy poll 1 µs registered for its sockets' queues" ;;
    *) echo "unknown backend" ;;
  esac
}

# The host settings every backend shares in the fair mode.
fair_settings_desc() {
  cat << 'EOS'
net.core.busy_poll 0, net.core.busy_read 0 (no global busy polling: the interrupt-driven backends are purely interrupt driven; recv-busy and uring-sqpoll set their own per socket or ring)
NIC interrupt moderation off where the NIC has it: adaptive-rx off, rx-usecs 0, RX CQE coalescing off (rx-cqe-frames 1); a NIC without the setting is recorded as such
napi_defer_hard_irqs 0, gro_flush_timeout 0 on every NIC
every NIC queue interrupt on the IRQ core (irqcore), or on the loop's CPU (irqlocal); every other movable interrupt on housekeeping; irqbalance off
idle CPUs poll instead of halting where no SMT sibling would spin (boot profile full-pollidle: x86 idle=poll, arm64 nohlt), so the IRQ core never wakes from sleep for an interrupt; else halt (recorded)
isolcpus, nohz_full and rcu_nocbs on every CPU but housekeeping; routes with quickack 1, rto_min 5ms
the sender: the same settings, its NIC interrupts on its housekeeping CPU, its server loop on the other CPU (the round-trip and HTTP servers busy poll their sockets; the market-data sender only sends)
kernel trace of the loop CPU (and the SQPOLL CPU) for the cause of each stall; in irqlocal without interrupt and softirq events, which there fire on every packet of the interrupt-driven backends only (/proc/interrupts still counts them)
EOS
}

# The fair points in run order, one per line:
#   <round> <condition> <kind> <backend> <connections> <rate|-> <seconds>
# (connections: md the connections, rt N = 1 order + N-1 feeds, http the pool).
fair_points() {
  local round cond kind b n rate list conds backends i
  for ((round = 1; round <= FAIR_ROUNDS; round++)); do
    read -ra conds <<< "$FAIR_CONDS"
    if (( round % 2 == 0 )); then
      local rev=()
      for ((i = ${#conds[@]} - 1; i >= 0; i--)); do rev+=("${conds[$i]}"); done
      conds=("${rev[@]}")
    fi
    for cond in "${conds[@]}"; do
      list=$FAIR_BACKENDS
      if [[ $cond == irqlocal ]]; then
        list=""
        for b in $FAIR_BACKENDS; do [[ " $FAIR_IRQLOCAL_BACKENDS " == *" $b "* ]] && list+="$b "; done
      fi
      read -ra backends <<< "$list"
      if (( round % 2 == 0 )); then
        local rb=()
        for ((i = ${#backends[@]} - 1; i >= 0; i--)); do rb+=("${backends[$i]}"); done
        backends=("${rb[@]}")
      fi
      for kind in $FAIR_KINDS; do
        case $kind in
          md) for n in $FAIR_MD_CONNS; do for b in "${backends[@]}"; do echo "$round $cond md $b $n - $FAIR_MD_S"; done; done ;;
          rt) for n in $FAIR_RT_CONNS; do for rate in $FAIR_RT_RATES; do for b in "${backends[@]}"; do echo "$round $cond rt $b $n $rate $FAIR_RT_S"; done; done; done ;;
          http) for n in $FAIR_HTTP_POOLS; do for b in "${backends[@]}"; do echo "$round $cond http $b $n $FAIR_HTTP_RATE $FAIR_HTTP_S"; done; done ;;
        esac
      done
    done
  done
}

# Seconds the fair points take: measuring, and with FAIR_POINT_OVERHEAD_S each.
fair_measure_s() { fair_points | awk '{s += $7} END {print s + 0}'; }
fair_points_s() { fair_points | awk -v o="$FAIR_POINT_OVERHEAD_S" '{s += $7 + o} END {print s + 0}'; }

# The plan: the points by kind and condition, the settings and each
# backend's own, then every point. No host is needed.
fair_plan() {
  local n
  n=$(fair_points | wc -l | tr -d ' ')
  echo "== points: $n ($FAIR_ROUNDS rounds; conditions $FAIR_CONDS)"
  fair_points | awk '{k = $2 " " $3; c[k]++; s[k] += $7} END {for (k in c) printf "  %-16s %4d points, %6d s measuring\n", k, c[k], s[k]}' | sort
  echo "  market data: connections $FAIR_MD_CONNS, $FAIR_MD_STREAMS streams over them (about 51,000 messages a second: $FAIR_MD_SERVER_FLAGS), ${FAIR_MD_S} s each; metric across families: sender -> decoded; kernel arrival -> decoded within a family only"
  echo "  order round trip: N = $FAIR_RT_CONNS (1 order connection + N-1 market-data connections on the same loop), ${FAIR_RT_RATES// /, } round trips a second, ${FAIR_RT_S} s each"
  echo "  HTTP: pools of ${FAIR_HTTP_POOLS// /, } connections (one request in flight, the pool's connections in turn), ${FAIR_HTTP_RATE}/s, ${FAIR_HTTP_S} s each; the server answers every connection from one thread"
  echo "  a point whose settings drift fails and is measured again up to $FAIR_RETRIES time(s)"
  echo "== settings every backend shares (vm-setup.sh fair, held by pin, checked before and after every point)"
  fair_settings_desc | sed 's/^/  /'
  echo "== what each backend sets for itself"
  local b
  for b in $FAIR_BACKENDS; do printf '  %-13s %-9s %s\n' "$b" "$(fair_family "$b")" "$(fair_backend_desc "$b")"; done
  echo "== every point: round condition kind backend connections rate seconds"
  fair_points | sed 's/^/  /'
}

# The fair run's environment for the report (res/fair-env.txt), one
# key=value a line: FAIR_ENV (set by the caller: run, cloud, region, sizes,
# priority) first.
fair_env() {
  printf '%s\n' "${FAIR_ENV:-}"
  printf '%s\n' "layout=${LAYOUT:-}" "housekeeping_cpu=$HK" "irq_cpu=${CPU_IRQ:-}" "loop_cpu=$CPU_A" \
    "sqpoll_cpu=$CPU_SQ" "spare_cpu=${CPU_SPARE:-none}" "receiver_boot=$RCV_BOOT"
  local b
  for b in $FAIR_BACKENDS; do echo "backend.$b=$(fair_family "$b"): $(fair_backend_desc "$b")"; done
  fair_settings_desc | awk '{printf "setting.%d=%s\n", NR, $0}'
}

# A replay_server on port $1 writing certificate $2, with arguments $3,
# then its certificate on the receiver.
fair_server() {
  # Separate calls: a shell whose command line holds the start command would
  # match the pattern and kill itself.
  sssh "sudo pkill -f 'replay_serve[r] --listen 0.0.0.0:$1' || true" > /dev/null || true
  sssh "mkdir -p srvstalls; nohup sudo bin/replay_server --listen 0.0.0.0:$1 --cert-out $2 $3 --cpu $S_CPU \
    > srv-fair.log 2>&1 < /dev/null &" || return 1
  sleep 2
  relay "$SND" "$2" "$RCV" "$2"
}

# The client command of one fair point: kind, backend, server, certificate,
# connections (md), N (rt) or pool (http), rate, seconds, output directory.
fair_client() {
  local kind=$1 b=$2 common="--server $3 --ca $4 --sni lab.local" n=$5 rate=$6 secs=$7 out=$8 v=$2 extra=""
  [[ $b == recv-busy-pq ]] && v=recv-busy extra=--per-queue
  [[ $b == uring-sqpoll ]] && extra="--sqpoll-cpu $CPU_SQ"
  case $kind:$b in
    md:mio) echo "bin/mio_bench lab $common --cpu $CPU_A --conns $n --capacity 8000000 --duration $secs --out $out" ;;
    md:*) echo "bin/io_bench lab $common --variants $v $extra --cpus $CPU_A --busy-poll 1 --wait-us 0 --conns $n --capacity 8000000 --duration $secs --out $out" ;;
    rt:mio) echo "bin/mio_bench pingpong $common --cpu $CPU_A --feeds $((n - 1)) --rate $rate --duration $secs --out $out" ;;
    rt:*) echo "bin/io_bench pingpong $common --variant $v $extra --cpu $CPU_A --busy-poll 1 --wait-us 0 --feeds $((n - 1)) --rate $rate --duration $secs --out $out" ;;
    http:mio) echo "bin/mio_bench http $common --cpu $CPU_A --pool $n --rate $rate --duration $secs --out $out" ;;
    http:*) echo "bin/http_bench --client client --impl native --backend $v $extra --pool $n $common --cpu $CPU_A --rate $rate --duration $secs --out $out" ;;
  esac
}

# One fair point (fair_points' fields); a failed one is measured again up to
# FAIR_RETRIES times, the failed attempt kept as <run>.attempt<k>.
fair_point() {
  local round=$1 cond=$2 kind=$3 b=$4 n=$5 rate=$6 secs=$7 try dir run port cert server args TRACE_EXTRA=""
  local tag=f$round-$cond at=""
  [[ $rate != - ]] && at=", $rate/s"
  case $kind in
    md) run=md-$b-c$n port=9447 cert=cert-many.pem server=""
      args="--clients $n --mode mixed $FAIR_MD_SERVER_FLAGS --streams $FAIR_MD_STREAMS --duration $((secs + 60))" ;;
    rt) run=rt-$b-n$n-r$rate port=9448 cert=cert-rt.pem server=echo
      args="--clients $n --mode mixed $FAIR_MD_SERVER_FLAGS --busy-poll 50 --stalls-dir \$HOME/srvstalls --duration $((secs + 60))"
      (( n > 1 )) && args+=" --streams $FAIR_MD_STREAMS" ;;
    http) run=http-$b-p$n-r$rate port=9445 cert=cert-http.pem server=http
      args="--mode http --threads one --busy-poll 50 --stalls-dir \$HOME/srvstalls --duration $((secs + 60))" ;;
    *) log "fair: unknown kind $kind"; return 1 ;;
  esac
  dir=$tag/$run
  [[ $b == uring-sqpoll ]] && TRACE_EXTRA=$CPU_SQ
  # irqlocal: every packet's interrupt is on the traced loop CPU; tracing it
  # would slow the interrupt-driven backends alone (vm-setup.sh trace noirq).
  local TRACE_NOIRQ=""
  [[ $cond == irqlocal ]] && TRACE_NOIRQ=1
  for ((try = 0; try <= FAIR_RETRIES; try++)); do
    log "fair $tag: $kind $b, $n connection(s)$at"
    if fair_server "$port" "$cert" "$args"; then
      traced_run "$dir" "sudo $(fair_client "$kind" "$b" "$SND_PRIV:$port" "$cert" "$n" "$rate" "$secs" "res/$dir")" "$server" "$secs"
    else
      fail_point "$dir" "the server did not start"
    fi
    local srvlog
    srvlog="$OUT/logs/srv-$(echo "$dir" | tr / _).txt"
    sleep 1
    sssh "tail -4 srv-fair.log" > "$srvlog" 2>&1 || true
    # schedule: sent=N batches=B late_100us=K late_max_us=M (market data and
    # feeds; K of the B due times began over 100 µs late).
    awk '/^schedule: / { for (i = 2; i <= NF; i++) { split($i, kv, "="); v[kv[1]] = kv[2] }
        if (v["batches"] > 0 && v["late_100us"] * 100 > v["batches"])
          printf "%d of %d sends began over 100 µs late, the latest %d µs\n", v["late_100us"], v["batches"], v["late_max_us"] }' "$srvlog" \
      | while read -r l; do log "!!! point $dir: the sender fell behind its schedule ($l): its traffic was not the planned traffic"; done
    # The same account belongs beside the work counts in the report. `work.late`
    # counts the paced order slots filled when the next was already due and
    # cannot say this: a point reports 2 of those and 39 464 late batches at
    # once, and only the second is the traffic not being the planned traffic.
    # Folded into the point's meta.txt here, where the receiver still has it,
    # so the analysis reports it and a table cannot read a slipped sender as a
    # clean run.
    local sch
    sch=$(awk '/^schedule: / { for (i = 2; i <= NF; i++) { split($i, kv, "="); v[kv[1]] = kv[2] }
        if (v["batches"] != "")
          printf "send.sent=%s send.batches=%s send.late_100us=%s send.late_max_us=%s",
                 v["sent"], v["batches"], v["late_100us"], v["late_max_us"] }' "$srvlog")
    if [[ -n $sch ]]; then
      rssh "for kv in $sch; do grep -qxF \"\$kv\" res/$dir/meta.txt 2>/dev/null || echo \"\$kv\" >> res/$dir/meta.txt; done" \
        || log "point $dir: the sender schedule was not folded into meta.txt"
    fi
    [[ -z $POINT_FAILED ]] && return 0
    if (( try < FAIR_RETRIES )); then
      log "point $dir: measuring again ($((try + 1)) of $FAIR_RETRIES); the failed attempt is kept as $dir.attempt$((try + 1))"
      rssh "rm -rf res/$dir.attempt$((try + 1)); mv res/$dir res/$dir.attempt$((try + 1))" || true
    fi
  done
}

run_fair() {
  mkdir -p "$OUT/logs"
  rssh "mkdir -p res"
  FAIR_CHECK=1
  local points p cur="" round cond kind b n rate secs
  # The environment and every held setting as the run starts, for the report.
  local env
  env="$(fair_env)
$(rssh "sudo bash vm-setup.sh check" | { grep -E '^(idle|boot-profile|sysctl\.|nic\.)' || true; } | sed 's/^/receiver./')
$(sssh "sudo bash vm-setup.sh check" | { grep -E '^(idle|boot-profile|sysctl\.|nic\.)' || true; } | sed 's/^/sender./')"
  printf '%s\n' "$env" | rssh "cat > res/fair-env.txt"
  # Read first: the ssh calls inside the loop would read its input.
  mapfile -t points < <(fair_points)
  log "fair: ${#points[@]} points, about $(( $(fair_points_s) / 60 )) minutes"
  for p in "${points[@]}"; do
    read -r round cond kind b n rate secs <<< "$p"
    if [[ $cond != "$cur" ]]; then
      [[ $cur == irqlocal ]] && apply_conds irqlocal off "fair-$round"
      [[ $cond == irqlocal ]] && apply_conds irqlocal on "fair-$round"
      cur=$cond
      log "fair round $round: condition $cond"
    fi
    fair_point "$round" "$cond" "$kind" "$b" "$n" "$rate" "$secs"
  done
  [[ $cur == irqlocal ]] && apply_conds irqlocal off "fair-end"
  sssh "sudo pkill -x replay_server || true" > /dev/null || true
  local failed=0
  [[ -s $OUT/failed-points.txt ]] && failed=$(wc -l < "$OUT/failed-points.txt" | tr -d ' ')
  log "fair: done; $failed failed attempt(s) (failed-points.txt)"
}

# The baseline set only (instance-type comparisons).
run_base_only() {
  mkdir -p "$OUT/logs"
  rssh "mkdir -p res"
  start_servers
  baseline 1 0
  baseline 1 1
}

# Merges the sender's files into the receiver's results, analyses there, and
# brings the tables, stall logs, traces and interrupt counts home, then the raw
# records (rsync, compressed; KEEP_RECORDS=0 leaves them on the VM), so the
# run can be analysed again later without re-running it.
# The results come back as tables and logs (a few MB): the analysis runs on
# the receiver once the measuring is done. KEEP_RECORDS=1 also copies the raw
# records and kernel traces (compressed, about 6x; gigabytes for a long run,
# so data transfer out of the cloud) and analyses them here instead.
collect_results() {
  sssh "tar -czf srvres.tgz srvres" > /dev/null && relay "$SND" srvres.tgz "$RCV" srvres.tgz \
    && rssh "mkdir -p srvx && tar -xzf srvres.tgz -C srvx && (cd srvx/srvres && find . -type f) | while read -r f; do
         mkdir -p res/\$(dirname \$f); mv srvx/srvres/\$f res/\$f; done" \
    || log "collect: sender files not merged"
  rssh "sudo chown -R \$(id -un) res"
  if [[ ${KEEP_RECORDS:-0} != 1 ]]; then
    log "collect: analysing on the receiver"
    # Every CPU may run it now (the isolated ones are idle).
    rssh "cd res && taskset -c 0-\$((\$(nproc) - 1)) ~/bin/analyse --warmup-s 2 --matrix . > ../tables.md 2> ../analyse.err; true"
    # The fair mode's own report: each backend against the others, per
    # connection count, with the run id and the environment.
    [[ ${MODE:-} != fair ]] || rssh "cd res && taskset -c 0-\$((\$(nproc) - 1)) ~/bin/analyse --warmup-s 2 --fair . > ../fair.md 2>> ../analyse.err; true"
    rssh "touch fair.md; tar -czf res.tgz --exclude='records.bin*' --exclude='trace.txt*' --exclude='server-trace.txt*' --exclude='fgraph.txt' \
      res tables.md fair.md analyse.err" && scp "${SSHO[@]}" "$RUSER@$RCV:res.tgz" "$OUT/" > /dev/null
    tar -xzf "$OUT/res.tgz" -C "$OUT" && rm -f "$OUT/res.tgz"
    log "collect: tables and logs copied ($(du -sh "$OUT/res" | cut -f1)); raw records and traces left on the VM"
    return
  fi
  # Records and traces compress about 6x: compressed here they copy back in a
  # fraction of the time and disk, and the analysis reads the .zst copies.
  rssh "command -v zstd > /dev/null || sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq zstd > /dev/null
    find res \\( -name records.bin -o -name trace.txt -o -name server-trace.txt \\) -size +1M -print0 | xargs -0 -r zstd -q -T0 -10 --rm" \
    || log "collect: compression failed (copying what is there)"
  rssh "tar -czf res.tgz --exclude='records.bin*' res" && scp "${SSHO[@]}" "$RUSER@$RCV:res.tgz" "$OUT/" > /dev/null
  tar -xzf "$OUT/res.tgz" -C "$OUT" && rm -f "$OUT/res.tgz"
  log "collect: copying the raw records"
  # One rsync per run, four at a time (a single SSH stream to a distant region
  # manages well under 1 MB/s). A failed copy is retried and never stops the
  # others (xargs gives up on an exit status of 255); the count is checked.
  local want got
  want=$(rssh "cd res && find . -name 'records.bin*' | wc -l" | tr -d ' ')
  rssh "cd res && find . -name 'records.bin*' -printf '%h\\n'" \
    | xargs -P 4 -I{} sh -c 'mkdir -p "$1/res/$2" && for t in 1 2 3; do
        rsync -az -e "$3" "$4:res/$2/records.bin*" "$1/res/$2/" && exit 0; sleep 5; done; exit 1' _ \
      "$OUT" {} "ssh ${SSHO[*]}" "$RUSER@$RCV" || true
  got=$(find "$OUT/res" -name 'records.bin*' | wc -l | tr -d ' ')
  log "collect: $got of $want record files, $(du -sh "$OUT/res" | cut -f1)"
  [[ $got == "$want" ]] || log "!!! collect: $((want - got)) record files missing"
  log "collect: analysing here"
  ANALYSE=${ANALYSE:-target/release/analyse}
  "$ANALYSE" --warmup-s 2 --matrix "$OUT/res" > "$OUT/tables.md" 2> "$OUT/analyse.err" \
    || log "collect: analysis failed (see analyse.err)"
  if [[ ${MODE:-} == fair ]]; then
    "$ANALYSE" --warmup-s 2 --fair "$OUT/res" > "$OUT/fair.md" 2>> "$OUT/analyse.err" \
      || log "collect: fair report failed (see analyse.err)"
  fi
}
