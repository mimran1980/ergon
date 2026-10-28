#!/usr/bin/env bash
# The transport latency suite's runner and cloud side, with no cloud and no
# VM: fakes for az, ssh, scp, sleep, curl, vm.sh, the bench binaries, and
# /sys and /proc trees. Checks that:
#   * modes.sh takes an arm's STREAMS, NETEM and AA, refuses anything else,
#     and refuses NETEM on every DPDK arm (by name or by its dpdk set-up) and
#     every Onload arm, an Onload arm or driver off AWS, F-Stack on Graviton;
#   * setup onload: transport.sh's flags (no kernel busy poll or TCP timers),
#     launcher and knobs; host.sh's Onload steps against a fake ENA (one
#     queue, Onload's MTU, the AF_XDP registration read back, undone) and its
#     stack check; the Onload smoke on fake hosts through fairness.py's onload
#     rule; the Onload session's estimate on c7i.4xlarge, c8g.2xlarge and
#     c7a.2xlarge;
#   * transport_plan.py counts the plan's points (510 for pair a, 420 for
#     b), draws each round's arm order from its seed (reproducible, the A/A
#     arm once a round, another order on another rep), and refuses a broken
#     matrix: aa first, an unknown or repeated arm, one- and two-host arms in
#     one set, two sets at once on one host, a sink arm off host a or under
#     netem; its binomial interval;
#   * host.sh against a fake D16s_v6 (16 CPUs, twins adjacent): the CPU map
#     with SMT on and off, the boot line and its check, SMT off verified (and
#     refused when the kernel keeps it on), the quiet settings, the sysctls
#     (an optional one absent, a wrong one refused), quickack routes, rings
#     and coalescing (a driver without coalescing), netem set, check and
#     counters, the placement verdict; irqs.sh: every data-NIC IRQ and VMBus
#     channel on the IRQ CPU (fail closed), the hot CPUs' IRQ deltas;
#   * az.sh: MaxNetworkInterfaces (fail closed), Spot's arguments, the
#     eviction watch, leftovers counted by pair;
#   * azure/lab.sh LAB_SUITE=transport: refusals before any az call, the
#     estimate of both pairs, the NIC preflight before the group, two pairs
#     side by side (their own groups, run and pair tags, each deleting and
#     counting only its own), Spot's arguments and an eviction ending the run
#     through its teardown, exit 75;
#   * aws/lab.sh LAB_SUITE=transport (against fake_aws.py): refusals before
#     any aws call, the AWS DPDK session's estimate (on demand and spot), the
#     ENI check before anything is created (too few refused), a session
#     tagged owner, run and pair on c7a.2xlarge in one cluster group, a TERM
#     mid-run, spot with no capacity (and LAB_SPOT_FALLBACK's on-demand
#     launch) and a detached run, each deleted and verified by tag;
#   * transport.sh run against fake hosts: every point of pair b once (rows,
#     manifests, rounds and orders recorded), netem set and cleared around
#     each lossy round and never under a DPDK arm, the binaries' flags as
#     their --help has them (transport-cli.md: addresses only for Aeron's
#     UDP, --busy-read-us for every socket arm, one CPU a busy thread), each row as
#     the binary writes it with the rep before it and its point column
#     <channel>-r<rate>, the session laid out for the analysis
#     (transport_collect.py), the ping started only once its pong says ready
#     (a pong that never does fails its point), a placement or netem verdict
#     that fails in the manifest, a failed point recorded, a hostcheck
#     warning stopping the session before its first point; a sink arm
#     (aeron_lat --mode spy-udp) on a with a sink on b for each run, beside
#     a one-host arm in its set, and fairness.py's sink rule; LAB_ARMS_FILE and
#     LAB_EXTRA_SRC's hooks with a neutral fixture (transport_prepare's steps
#     too); the hand-off arms (transport-aws-aeron-handoff.tsv): each its
#     twin's driver mode but for its threading or DRIVER=embedded, aeron_lat's
#     --driver and busy threads, an embedded driver's binary with driver.env
#     under sudo setpriv, the session on fake hosts through fairness.py's
#     declared rule (a twin chain; a driver mode differing beyond its knob);
#     the mixed-load sessions (transport-aws-aeron-mixed.tsv, both pairs):
#     each mixed arm's driver mode with the duty-cycle thresholds, two CPUs
#     a host (hot, background), its NIC counters snapshotted around each
#     point and collected, the raw DPDK floor (floor on dpdk-raw's set-up,
#     the driver's environment, sudo setpriv), fairness.py clean (its
#     mixed-load rules flagging a fan-out subscription short), analyse.py's
#     mixed table and checklist.
#
# Run from anywhere:  bash md-dpdk/benches/cloud/test/test-transport-runner.sh
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cloud=$(cd "$here/.." && pwd)
tmp=$(mktemp -d)
# (TEST_KEEP=1: kept, and named, for a look at a failure)
trap 'if [[ -n ${TEST_KEEP:-} ]]; then echo "kept $tmp" >&2; elif [[ -n ${tmp:-} && -d $tmp && ${tmp##*/} == tmp.* ]]; then rm -rf -- "$tmp"; fi' EXIT
failed=0
fail() { echo "FAIL: $*" >&2; failed=1; }
pass() { echo "ok: $*"; }
same() { # <what> <want> <got>
    if [[ $2 == "$3" ]]; then pass "$1"; else
        fail "$1"
        diff -u <(echo "$2") <(echo "$3") >&2 || true
    fi
}
plan() { python3 "$cloud/transport_plan.py" "$@"; }
matrix=$cloud/transport-matrix.tsv

# ---- modes.sh: the arms' settings ----
got=$(
    . "$cloud/modes.sh"
    TRANSPORT_ARMS=$(plan arms "$matrix")
    rc() { mode_check "$1" remote x86_64 azure '' >/dev/null 2>&1; echo -n " $?"; }
    echo -n "ok:"; for m in mio-tcp@STREAMS=8 aeron-rel@STREAMS=1@NETEM=0.01 eio-tcp@STREAMS=1@NETEM=0 mpsc@STREAMS=1@AA=1 dpdk-aeron-udp@STREAMS=128; do rc "$m"; done
    echo
    echo -n "refused:"; for m in dpdk-aeron-rel@STREAMS=1@NETEM=0.1 dpdk-aeron-self@NETEM=0 mio-tcp@FOO=1 mio-tcp@STREAMS=0 mio-tcp@STREAMS=2000 mio-udp@NETEM=101 mio-udp@NETEM=x mio-udp@AA=2 mio-udp@STREAMS=1@STREAMS=2 mio-udp+idle nosuch@STREAMS=1; do rc "$m"; done
    echo
    # an arms file's DPDK arm by its set-up, whatever its name
    TRANSPORT_ARMS+="kbx=dpdk/two "
    echo -n "by setup:"; rc kbx@NETEM=1; rc kbx@STREAMS=1
    echo
    mode_check dpdk-aeron-rel@NETEM=1 remote x86_64 azure '' 2>&1 | grep -c 'DPDK bypasses the kernel' || true
    # setup fstack: no netem, and AWS x86_64 alone (F-Stack's build, the ENA)
    TRANSPORT_ARMS+="kfs=fstack/two "
    f() { mode_check "$1" remote "$2" "$3" '' >/dev/null 2>&1; echo -n " $?"; }
    echo -n "fstack:"; f kfs@STREAMS=8 x86_64 aws; f kfs@NETEM=1 x86_64 aws; f kfs@STREAMS=1 arm64 aws; f kfs@STREAMS=1 x86_64 azure
    echo
)
same "modes.sh: an arm takes STREAMS, NETEM and AA alone; NETEM refused on every DPDK arm; an F-Stack arm on AWS x86_64 alone" "ok: 0 0 0 0 0
refused: 2 2 2 2 2 2 2 2 2 2 2
by setup: 2 0
1
fstack: 0 2 2 2" "$got"

# ---- transport_plan.py ----
got="$(plan count a 5 "$matrix") $(plan count b 5 "$matrix") $(plan count a 1 "$matrix")"
same "transport_plan.py: pair a 510 points, pair b 420 (the plan's table, an A/A arm in every set, 3c's no-qdisc round), a fifth of it at one rep" "510 420 102" "$got"
r1=$(plan rounds b 7 5 "$matrix")
r2=$(plan rounds b 7 5 "$matrix")
r3=$(plan rounds b 8 5 "$matrix")
why=
[[ $r1 == "$r2" ]] || why+=" not reproducible;"
[[ $r1 != "$r3" ]] || why+=" another seed, the same orders;"
# every round of a set with aa has its first arm twice, once @AA=1
while IFS=$'\t' read -r _ sid _ _ _ _ _ _ order; do
    case $sid in 1 | 2b | 3b) [[ $(tr ',' '\n' <<<"$order" | grep -c '@AA=1$') == 1 ]] || why+=" set $sid round without one A/A arm;" ;; esac
done <<<"$r1"
# set 2a's orders over its 5 reps are not all one
orders=$(plan rounds a 7 5 "$matrix" | awk -F'\t' '$2 == "2a" {print $9}' | sort -u | wc -l | tr -d ' ')
((orders > 1)) || why+=" one order every rep;"
# 3b: three netem levels a rep, each its own round seed
[[ $(awk -F'\t' '$2 == "3b" && $4 == 1 {print $5}' <<<"$r1" | sort | tr '\n' ' ') == "0.01 0.1 1 " ]] || why+=" 3b levels;"
[[ $(awk -F'\t' '$2 == "1" || $2 == "2b" {print $1}' <<<"$r1" | sort -u) == 0 ]] || why+=" sets 1 and 2b not one group;"
if [[ -z $why ]]; then pass "transport_plan.py: each round's order from its seed (reproducible), one A/A arm a round, orders differ by rep, netem levels a round each, sets 1 and 2b at once"; else fail "transport_plan.py rounds:$why"; fi
bad=$tmp/bad.tsv
refusals=
for case in "set z a - ab 1 1 - aa,mio-tcp|cannot be first" "set z a - ab 1 1 - nosuch|no arm nosuch" "arm mio-tcp tcp_lat sockets two|given twice" \
    "set z a - ab 1 1 - mio-tcp,mpsc|host ab takes" "set z b p1 a 1 1 - mpsc|one-host sets on different hosts"; do
    printf '%s\n' "${case%|*}" >"$bad"
    if out=$(plan modes "${case%% *}" "$matrix" "$bad" 2>&1; plan modes b "$matrix" "$bad" 2>&1); then refusals+=" accepted[${case%|*}]"; fi
    [[ $out == *"${case#*|}"* ]] || refusals+=" [${case%|*}: $out]"
done
[[ -z $refusals ]] && pass "transport_plan.py refuses aa first, an unknown or repeated arm, mixed hosts in a set, two sets at once on one host" ||
    fail "transport_plan.py refusals:$refusals"
# setup fstack (dpdk-tcp, fstack_lat): no netem on a set with a DPDK arm;
# fstack_lat's addresses as tcp_lat's, its port (PCI address, prefix,
# gateway) from the NIC files, the socket buffers its one knob; F-Stack
# built only for a session that runs it (tr_wants_fstack)
printf '%s\n' "set z a - ab 1 1 0.1 dpdk-tcp" >"$bad"
out=$(plan modes a "$matrix" "$bad" 2>&1) && fail "transport_plan.py: netem on dpdk-tcp accepted" ||
    { [[ $out == *"netem cannot reach"* ]] || fail "transport_plan.py netem on dpdk-tcp: $out"; }
got=$(
    . "$cloud/transport.sh"
    dir=$tmp/fstack
    mkdir -p "$dir"
    printf 'sender ens6 ens6 0a:00:00:00:00:01 10.30.1.11 24 0000:00:06.0\nreceiver ens7 ens7 0a:00:00:00:00:02 10.30.1.21 24 0000:00:07.0\n' >"$dir/nics-a.txt"
    printf 'sender ens6 ens6 0a:00:00:00:00:03 10.30.1.12 24 0000:00:06.0\nreceiver ens7 ens7 0a:00:00:00:00:04 10.30.1.22 24 0000:00:07.0\n' >"$dir/nics-b.txt"
    arm=dpdk-tcp bin=fstack_lat setup=fstack args=()
    echo "ping: $(tr_bin_args fstack_lat ping 10.30.1.11 10.30.1.22)"
    echo "pong: $(tr_bin_args fstack_lat pong 10.30.1.22 10.30.1.11)"
    echo "knobs: $(tr_arm_knobs | tr '\n' ' ')"
    hk=0 tr_bin_path=P
    echo "start: $(tr_start r/pong.pid r/pong.out fstack_lat --role pong)"
    setup=sockets
    echo "start sockets: $(tr_start r/pong.pid r/pong.out tcp_lat --role pong)"
    setup=fstack
    arms_file='' sets_file=$cloud/transport-aws-dpdk.tsv
    tr_wants_fstack && echo "aws-dpdk: fstack"
    sets_file=''
    tr_wants_fstack || echo "matrix: none"
)
sb_=$(. "$cloud/transport-settings.env"; echo "$TR_SOCKBUF")
same "transport.sh setup fstack: no netem with a DPDK arm; fstack_lat's addresses as tcp_lat's, its PCI port, prefix and gateway from vm.sh nics, delayed ACKs off (the kernel arms' quickack), the socket buffers; started under sudo setpriv with the EAL's capabilities ambient, under the user's own uid (no other arm is); F-Stack built only when a set runs it" \
    "ping: --source 10.30.1.11 --connect 10.30.1.22:22000 --pci 0000:00:06.0 --prefix-len 24 --gateway 10.30.1.1 --nodelay --delayed-ack false --rcvbuf $sb_ --sndbuf $sb_
pong: --listen 10.30.1.22:22000 --pci 0000:00:07.0 --prefix-len 24 --gateway 10.30.1.1 --nodelay --delayed-ack false --rcvbuf $sb_ --sndbuf $sb_
knobs: sockbuf 
start: sudo -n setpriv --reuid=\"\$(id -u)\" --regid=\"\$(id -g)\" --init-groups --reset-env --inh-caps=+sys_admin,+dac_override,+ipc_lock,+sys_rawio --ambient-caps=+sys_admin,+dac_override,+ipc_lock,+sys_rawio sh -c 'taskset -c 0 env PATH=P fstack_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
start sockets: sh -c 'taskset -c 0 env PATH=P tcp_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
aws-dpdk: fstack
matrix: none" "$got"
# setup onload (the matrix's onload-* arms: transport-aws-onload.tsv): no
# netem (its AF_XDP sends skip the qdisc); AWS only, x86_64 or Graviton;
# the kernel arm's binary and flags under onload, but no kernel busy poll
# (--busy-read-us 0) and no kernel TCP timers; aeron_lat itself not under
# onload, its media driver is (driver mode onload); OpenOnload built and the
# boot line asked for only when a set runs such an arm
printf '%s\n' "set z a - ab 1 1 0.1 onload-eio-tcp" >"$bad"
out=$(plan modes a "$matrix" "$bad" 2>&1) && fail "transport_plan.py: netem on onload-eio-tcp accepted" ||
    { [[ $out == *"run on Onload, whose AF_XDP sends netem cannot reach"* ]] || fail "transport_plan.py netem on onload-eio-tcp: $out"; }
got=$(
    . "$cloud/modes.sh"
    . "$cloud/transport-settings.env"
    TRANSPORT_ARMS=$(plan arms "$matrix")
    f() { mode_check "$1" remote "$2" "$3" '' >/dev/null 2>&1; echo -n " $?"; }
    echo -n "arms:"; f onload-eio-tcp@STREAMS=8 x86_64 aws; f onload-aeron-rel@STREAMS=1 arm64 aws; f onload-mio-udp@NETEM=1 x86_64 aws; f onload-eio-tcp@STREAMS=1 x86_64 azure
    echo
    echo -n "driver:"; f "onload$TR_AERON_MODE_SUFFIX" x86_64 aws; f "onload$TR_AERON_MODE_SUFFIX" arm64 aws; f "onload$TR_AERON_MODE_SUFFIX" x86_64 azure
    echo
    mode_check onload-mio-udp@NETEM=1 remote x86_64 aws '' 2>&1 | grep -c 'skip the kernel' || true
    # an F-Stack arm on Graviton, as aws/lab.sh now judges it (tr_arch)
    f dpdk-tcp@STREAMS=1 arm64 aws
    echo
)
same "modes.sh: an Onload arm takes STREAMS on AWS (x86_64 and Graviton), never NETEM, never Azure; the Onload driver mode AWS only; an F-Stack arm refused on Graviton" "arms: 0 0 2 2
driver: 0 0 2
1
 2" "$got"
got=$(
    . "$cloud/transport.sh"
    dir=$tmp/onl
    mkdir -p "$dir"
    # both kernels have the TCP timers: the kernel arms ask for them, Onload's never
    for h in a b; do printf 'net.ipv4.tcp_rto_min_us=8000\nnet.ipv4.tcp_delack_max_us=8000\n' >"$dir/settings-$h.txt"; done
    hk=0 tr_bin_path=P
    for a in eio-tcp onload-eio-tcp onload-mio-udp onload-eio-udp onload-aeron-rel; do
        read -r bin setup _ rest <<<"$(tr_plan arm "$a")"
        read -ra args <<<"$rest"
        arm=$a
        echo "$a: $(tr_bin_args "$bin" ping 10.30.1.11 10.30.1.22 "${args[@]}")"
        echo "  knobs: $(tr_arm_knobs | tr '\n' ' ')napi $(tr_knob napi_defer); driver: $(tr_has_driver && tr_driver_mode || echo none)"
        echo "  start: $(tr_start r/pong.pid r/pong.out "$bin" --role pong)"
    done
    arms_file='' sets_file=$cloud/transport-aws-onload.tsv
    tr_wants_setup onload && echo "aws-onload: onload"
    tr_wants_setup fstack || echo "aws-onload: no fstack"
    sets_file=$cloud/transport-aws-onload-smoke.tsv
    tr_wants_setup onload && echo "smoke: onload"
    sets_file=''
    tr_wants_setup onload || echo "matrix: none"
)
sb_=$(. "$cloud/transport-settings.env"; echo "$TR_SOCKBUF")
sfx_=$(. "$cloud/transport-settings.env"; echo "$TR_AERON_MODE_SUFFIX")
oc_=$(. "$cloud/transport-settings.env"; echo "env $TR_ONLOAD_ENV onload --profile=$TR_ONLOAD_PROFILE")
same "transport.sh setup onload: the kernel arm's binary and flags under onload ($oc_), but --busy-read-us 0 and no TCP timers; the knobs Onload honours and the NICs' deferral at TR_ONLOAD_NAPI_DEFER; Aeron's arm not under onload itself, its media driver (driver mode onload) is; Onload wanted only by a set that runs it" \
"eio-tcp: --source 10.30.1.11 --connect 10.30.1.22:22000 --nodelay --quickack-per-read --rto-min-us 8000 --delack-max-us 8000 --busy-read-us 50 --rcvbuf $sb_ --sndbuf $sb_ --epoll-events 64
  knobs: sockbuf epoll_events quickack_per_read congestion busy_read_us rto_min_us napi_defer irq_idle napi 0; driver: none
  start: sh -c 'taskset -c 0 env PATH=P tcp_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
onload-eio-tcp: --source 10.30.1.11 --connect 10.30.1.22:22000 --nodelay --quickack-per-read --busy-read-us 0 --rcvbuf $sb_ --sndbuf $sb_ --epoll-events 64
  knobs: sockbuf epoll_events quickack_per_read napi_defer napi 0; driver: none
  start: sh -c 'taskset -c 0 env PATH=P $oc_ tcp_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
onload-mio-udp: --local 10.30.1.11:22000 --peer 10.30.1.22:22000 --busy-read-us 0 --rcvbuf $sb_ --sndbuf $sb_ --epoll-events 64
  knobs: sockbuf epoll_events napi_defer napi 0; driver: none
  start: sh -c 'taskset -c 0 env PATH=P $oc_ udp_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
onload-eio-udp: --local 10.30.1.11:22000 --peer 10.30.1.22:22000 --busy-read-us 0 --rcvbuf $sb_ --sndbuf $sb_ --epoll-events 64 --datagram-burst 64
  knobs: sockbuf epoll_events datagram_burst napi_defer napi 0; driver: none
  start: sh -c 'taskset -c 0 env PATH=P $oc_ udp_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
onload-aeron-rel: --endpoint 10.30.1.22:22000 --reply 10.30.1.11:22000 --interface 10.30.1.11 --aeron-dir /dev/shm/aeron-bench --driver external --driver-busy-threads 3 --term-length 1048576 --mtu 1408 --wire kernel
  knobs: term_length aeron_page aeron_so_rcvbuf nak_delay_ns napi_defer napi 0; driver: onload$sfx_
  start: sh -c 'taskset -c 0 env PATH=P aeron_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
aws-onload: onload
aws-onload: no fstack
smoke: onload
matrix: none" "$got"
# the hand-off arms (transport-aws-aeron-handoff.tsv): dpdk-aeron-rel and
# aeron-rel but for their driver's threading or place, each declared its
# twin (TR_DECLARED, a chain for -invoker); their driver modes pass
# mode_check (DRIVER=embedded: vm.sh starts no driver), aeron_lat's
# --driver and --driver-busy-threads follow them, an embedded driver's
# binary runs with driver.env under fstack_lat's privileges, and the
# events leave a driver's idle CPUs to the gap spinners
got=$(
    . "$cloud/transport.sh"
    tr_cloud=aws
    dir=$tmp/fstack hk=0 tr_bin_path=P iso=4,6,8,10,12,14 kind=two cpus=10,12,14
    for a in dpdk-aeron-rel dpdk-aeron-rel-emb dpdk-aeron-rel-shnet dpdk-aeron-rel-shared dpdk-aeron-rel-invoker aeron-rel-emb aeron-rel-invoker; do
        read -r bin setup _ rest <<<"$(tr_plan arm "$a")"
        read -ra args <<<"$rest"
        arm=$a
        m=$(tr_driver_mode)
        echo "$a: ${m/$(. "$cloud/transport-settings.env"; echo "$TR_AERON_MODE_SUFFIX")/<suffix>}"
        echo "  check $(mode_check "$m" remote x86_64 aws '' >/dev/null 2>&1; echo $?); knobs threading $(tr_knob aeron_threading) driver $(tr_knob aeron_driver); $(tr_bin_args "$bin" ping 10.30.1.11 10.30.1.22 "${args[@]}" | grep -o -- '--driver .* --driver-busy-threads [0-9]*'); events $(tr_events_cpus a)"
        echo "  start: $(tr_start r/pong.pid r/pong.out "$bin" --role pong | sed 's/ --inh-caps.*sys_rawio//')"
    done
    # INVOKER with the driver not the binary's, and a twin chain that goes round
    tr_sets_file=$cloud/transport-aws-aeron-handoff.tsv reps=3
    arm=dpdk-aeron-rel-shnet
    read -r bin setup _ rest <<<"$(tr_plan arm "$arm")"; read -ra args <<<"$rest"
    TR_DECLARED="dpdk-aeron-rel-shnet:dpdk-aeron-rel:aeron_threading=INVOKER" tr_driver_modes_check a 2>&1 >/dev/null | sed 's/^/refused: /' || true
    TR_DECLARED="dpdk-aeron-rel-emb:dpdk-aeron-rel-invoker:aeron_driver=embedded dpdk-aeron-rel-invoker:dpdk-aeron-rel-emb:aeron_threading=INVOKER" tr_declared_check 2>&1 | sed 's/^/refused: /' || true
    echo "handoff check: $(tr_check a 2>&1)"
)
same "transport.sh, the hand-off arms: each its twin's driver mode but for its threading (SHARED_NETWORK keeps the conductor's core, SHARED and INVOKER drop it) or DRIVER=embedded, each passing mode_check; aeron_lat's --driver (embedded, invoker) and busy threads (3, 2, 1, 0) from it; an embedded driver's binary started with ~/bench/driver.env under sudo setpriv; the events' spinners on the CPUs the driver leaves; INVOKER without the driver embedded and a twin chain going round refused; the sets file checks (66 points at 3 reps: h and k, set k on too)" \
"dpdk-aeron-rel: dpdk<suffix>
  check 0; knobs threading DEDICATED driver external; --driver external --driver-busy-threads 3; events 4,6,8,10,12,14 12,14
  start: sh -c 'taskset -c 0 env PATH=P aeron_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
dpdk-aeron-rel-emb: dpdk<suffix>@DRIVER=embedded
  check 0; knobs threading DEDICATED driver embedded; --driver embedded --driver-busy-threads 3; events 4,6,8,10,12,14 12,14
  start: sudo -n setpriv --reuid=\"\$(id -u)\" --regid=\"\$(id -g)\" --init-groups --reset-env sh -c '. ~/bench/driver.env; taskset -c 0 env PATH=P aeron_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
dpdk-aeron-rel-shnet: dpdk@CONDUCTOR=own@AERON_TERM_BUFFER_LENGTH=1048576@AERON_IPC_TERM_BUFFER_LENGTH=1048576@AERON_CONDUCTOR_IDLE_STRATEGY=spin@AERON_THREADING_MODE=SHARED_NETWORK
  check 0; knobs threading SHARED_NETWORK driver external; --driver external --driver-busy-threads 2; events 4,6,8,10,12,14 6,12,14
  start: sh -c 'taskset -c 0 env PATH=P aeron_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
dpdk-aeron-rel-shared: dpdk@AERON_TERM_BUFFER_LENGTH=1048576@AERON_IPC_TERM_BUFFER_LENGTH=1048576@AERON_THREADING_MODE=SHARED
  check 0; knobs threading SHARED driver external; --driver external --driver-busy-threads 1; events 4,6,8,10,12,14 6,8,12,14
  start: sh -c 'taskset -c 0 env PATH=P aeron_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
dpdk-aeron-rel-invoker: dpdk@AERON_TERM_BUFFER_LENGTH=1048576@AERON_IPC_TERM_BUFFER_LENGTH=1048576@AERON_THREADING_MODE=INVOKER@DRIVER=embedded
  check 0; knobs threading INVOKER driver embedded; --driver invoker --driver-busy-threads 0; events 4,6,8,10,12,14 4,6,8,12,14
  start: sudo -n setpriv --reuid=\"\$(id -u)\" --regid=\"\$(id -g)\" --init-groups --reset-env sh -c '. ~/bench/driver.env; taskset -c 0 env PATH=P aeron_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
aeron-rel-emb: kernel-busy<suffix>@NAPI_DEFER=0/0@DRIVER=embedded
  check 0; knobs threading DEDICATED driver embedded; --driver embedded --driver-busy-threads 3; events 4,6,8,10,12,14 12,14
  start: sudo -n setpriv --reuid=\"\$(id -u)\" --regid=\"\$(id -g)\" --init-groups --reset-env sh -c '. ~/bench/driver.env; taskset -c 0 env PATH=P aeron_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
aeron-rel-invoker: kernel-busy@AERON_TERM_BUFFER_LENGTH=1048576@AERON_IPC_TERM_BUFFER_LENGTH=1048576@NAPI_DEFER=0/0@AERON_THREADING_MODE=INVOKER@DRIVER=embedded
  check 0; knobs threading INVOKER driver embedded; --driver invoker --driver-busy-threads 0; events 4,6,8,10,12,14 4,6,8,12,14
  start: sudo -n setpriv --reuid=\"\$(id -u)\" --regid=\"\$(id -g)\" --init-groups --reset-env sh -c '. ~/bench/driver.env; taskset -c 0 env PATH=P aeron_lat --role pong & p=\$!; echo \$p >r/pong.pid; wait \$p; r=\$?; echo \$r >r/pong.pid.rc; exit \$r' >r/pong.out
refused: arm dpdk-aeron-rel-shnet: aeron_threading INVOKER with aeron_driver external: an INVOKER driver is the binary's own (aeron_driver embedded)
refused: TR_DECLARED dpdk-aeron-rel-emb: its twins go round ( dpdk-aeron-rel-emb dpdk-aeron-rel-invoker dpdk-aeron-rel-emb)
handoff check: 66" "$got"
got=$(
    . "$cloud/modes.sh"
    rc() { mode_check "$1" remote x86_64 aws '' >/dev/null 2>&1; echo -n " $?"; }
    echo -n "DRIVER:"; rc dpdk@DRIVER=embedded; rc kernel-busy@AERON_THREADING_MODE=INVOKER@DRIVER=embedded; rc dpdk@DRIVER=external; rc dpdk-raw@DRIVER=embedded; rc dpdk@DRIVER=embedded+idle; rc dpdk@DRIVER=embedded@DRIVER=embedded
    echo
)
same "modes.sh: DRIVER=embedded (no driver process; the bench binary's) under any threading mode; any other value, a raw floor's, +idle's and a second one refused" "DRIVER: 0 0 2 2 2 2" "$got"
got="$(plan binom 300000 30 0.01) | $(plan binom 300000 80 0.01) | $(plan binom 1000 0 0) | $(plan binom 1000 1 0) | $(plan binom 1000 2 0.01) | $(plan binom 6000000 60000 1)"
same "transport_plan.py binom: the dropped count against its 1-in-10,000 interval, exact at small means, none at 0 %" \
    "8 52 ok | 8 52 FAIL | 0 0 ok | 0 0 FAIL | 0 3 ok | 59051 60949 ok" "$got"

# transport.sh check: a declared twin (TR_DECLARED) is its twin's arm row
# but for a per-arm knob; aeron-ipc-huge's hugetlbfs (TR_AERON_HUGE_PAGES)
# holds its logs twice over (host.sh transport_huge_pages)
got=$(
    . "$cloud/transport.sh"
    reps=5
    c() { tr_check b 2>&1 | tr '\n' ' '; echo "rc ${PIPESTATUS[0]}"; }
    echo "ok: $(c)"
    echo "small: $(TR_AERON_HUGE_PAGES=16; c)"
    echo "twin: $(TR_DECLARED="aeron-ipc-huge:aeron-spy:aeron_page=2097152"; c)"
    echo "no twin: $(TR_DECLARED="aeron-ipc-huge:nosuch:aeron_page=2097152"; c)"
    echo "knob: $(TR_DECLARED="aeron-ipc-huge:aeron-ipc:nosuch=1"; c)"
    echo "page: $(TR_DECLARED="aeron-ipc-huge:aeron-ipc:aeron_page=65536"; c)"
    echo "form: $(TR_DECLARED="aeron-ipc-huge=aeron-ipc"; c)"
)
same "transport.sh check: a declared twin is its twin's arm but for one per-arm knob (aeron_page 4096 or 2097152); aeron-ipc-huge's hugetlbfs holds its logs twice over" "ok: 420 rc 0
small: TR_AERON_HUGE_PAGES 16: aeron-ipc-huge's hugetlbfs needs 28 pages of 2 MiB (term length 1048576, 1 streams: host.sh transport_huge_pages) rc 2
twin: TR_DECLARED aeron-ipc-huge: not aeron-spy's twin: [aeron_lat kernel one --mode ipc] against [aeron_lat kernel one --mode spy] rc 2
no twin: TR_DECLARED aeron-ipc-huge: no arm nosuch to be its twin rc 2
knob: TR_DECLARED aeron-ipc-huge: nosuch is not a per-arm knob of aeron_lat (transport-settings.env's list) rc 2
page: TR_DECLARED aeron-ipc-huge: aeron_page 65536: 4096 (TR_AERON_DIR, tmpfs) or 2097152 (TR_AERON_HUGE_DIR, 2 MiB hugetlbfs) rc 2
form: TR_DECLARED aeron-ipc-huge=aeron-ipc: <arm>:<twin>:<knob>=<value> rc 2" "$got"

# ---- host.sh and irqs.sh against fake trees ----
sys=$tmp/sys proc=$tmp/proc
for cpu in $(seq 0 15); do
    mkdir -p "$sys/devices/system/cpu/cpu$cpu/topology"
    echo "$((cpu / 2 * 2))-$((cpu / 2 * 2 + 1))" >"$sys/devices/system/cpu/cpu$cpu/topology/thread_siblings_list"
done
mkdir -p "$sys/devices/system/cpu/smt" "$proc/sys/net/ipv4" "$proc/sys/net/core" "$proc/sys/vm" "$proc/sys/kernel" "$tmp/fakebin"
echo on >"$sys/devices/system/cpu/smt/control"
echo 1 >"$sys/devices/system/cpu/smt/active"
echo 0-15 >"$sys/devices/system/cpu/online"
# fake sudo runs what it is given (the trees are the test's), logging it;
# writing off to smt/control takes the twins offline unless FAKE_SMT_STUCK
cat >"$tmp/fakebin/sudo" <<'SH'
#!/usr/bin/env bash
echo "sudo $*" >>"$FAKE_LOG"
if [[ $1 == tee && $2 == */smt/control && -z ${FAKE_SMT_STUCK:-} ]]; then
    v=$(cat); echo "$v" >"$2"
    if [[ $v == off ]]; then
        echo 0 >"${2%/control}/active"
        root=${2%/smt/control}
        for c in 1 3 5 7 9 11 13 15; do rm -rf "$root/cpu$c/topology"; done
        echo 0,2,4,6,8,10,12,14 >"$root/online"
    fi
    exit 0
fi
[[ $1 == tee && $2 == */smt/control ]] && { cat >/dev/null; exit 0; }
[[ $1 == tee && $2 == *refuse* ]] && { cat >/dev/null; exit 1; }
exec "$@"
SH
# fake ip: the routes ip -4 route show table all prints; route change logged
cat >"$tmp/fakebin/ip" <<'SH'
#!/usr/bin/env bash
echo "ip $*" >>"$FAKE_LOG"
if [[ "$*" == "-4 route show table all" ]]; then
    cat "$FAKE_ROUTES"
elif [[ $1 == route && $2 == change ]]; then
    shift 2
    # as the kernel: prefix and table pick the route, the first that matches,
    # whatever dev the request names (two NICs on one subnet: one is missed)
    awk -v new="$*" 'BEGIN {split(new, w, " "); done = 0} {tbl = ""; for (i = 1; i < NF; i++) if ($i == "table") tbl = $(i + 1)
        nt = ""; for (i in w) if (w[i] == "table") nt = w[i + 1]
        if (!done && $1 == w[1] && tbl == nt) {print new; done = 1} else print}' "$FAKE_ROUTES" >"$FAKE_ROUTES.new" && mv "$FAKE_ROUTES.new" "$FAKE_ROUTES"
elif [[ $1 == route && $2 == del ]]; then
    shift 2
    # prefix, table and (when given) dev pick the one route deleted
    awk -v sel="$*" 'BEGIN {split(sel, w, " "); done = 0} {dev = ""; tbl = ""; for (i = 1; i < NF; i++) {if ($i == "dev") dev = $(i + 1); if ($i == "table") tbl = $(i + 1)}
        nd = ""; nt = ""; for (i in w) {if (w[i] == "dev") nd = w[i + 1]; if (w[i] == "table") nt = w[i + 1]}
        if (!done && $1 == w[1] && tbl == nt && (nd == "" || dev == nd)) done = 1; else print}' "$FAKE_ROUTES" >"$FAKE_ROUTES.new" && mv "$FAKE_ROUTES.new" "$FAKE_ROUTES"
elif [[ $1 == route && $2 == append ]]; then
    shift 2
    echo "$*" >>"$FAKE_ROUTES"
fi
SH
# fake ethtool: rings settable up to 4096; coalescing unless FAKE_NO_COALESCE;
# FAKE_ENA: no adaptive TX (n/a), and a -C naming it is refused whole, as ENA's
cat >"$tmp/fakebin/ethtool" <<'SH'
#!/usr/bin/env bash
echo "ethtool $*" >>"$FAKE_LOG"
st=$FAKE_STATE
case $1 in
-G) [[ $4 -le 4096 && $6 -le 4096 ]] && echo "$4 $6" >"$st/ring-$2"; exit 0 ;;
-g) read -r r t <"$st/ring-$2" 2>/dev/null || { r=512 t=512; }
    printf 'Ring parameters for %s:\nPre-set maximums:\nRX:\t\t4096\nTX:\t\t4096\nCurrent hardware settings:\nRX:\t\t%s\nTX:\t\t%s\n' "$2" "$r" "$t" ;;
-c) [[ -n ${FAKE_NO_COALESCE:-} ]] && { echo "netlink error: Operation not supported" >&2; exit 76; }
    if [[ -n ${FAKE_ENA:-} ]]; then
        if [[ -e $st/coal-$2 ]]; then printf 'Adaptive RX: off  TX: n/a\nrx-usecs: 0\ntx-usecs: 0\n'; else printf 'Adaptive RX: on  TX: n/a\nrx-usecs: 20\ntx-usecs: 64\n'; fi
    elif [[ -e $st/coal-$2 ]]; then printf 'Adaptive RX: off  TX: off\nrx-usecs: 0\ntx-usecs: 0\n'; else printf 'Adaptive RX: on  TX: on\nrx-usecs: 8\ntx-usecs: 8\n'; fi ;;
-C) [[ -n ${FAKE_ENA:-} && " $* " == *" adaptive-tx "* ]] && { echo "netlink error: Operation not supported" >&2; exit 76; }
    [[ -z ${FAKE_COALESCE_STUCK:-} ]] && touch "$st/coal-$2"; exit 0 ;;
esac
SH
# fake tc: one root qdisc an interface, its counters growing per show -s
cat >"$tmp/fakebin/tc" <<'SH'
#!/usr/bin/env bash
echo "tc $*" >>"$FAKE_LOG"
st=$FAKE_STATE
case "$*" in
"qdisc replace dev "*" root netem "*) echo "${*##* loss }" | tr -d % >"$st/netem-$4" ;;
"qdisc del dev "*" root") rm -f "$st/netem-$4" ;;
"qdisc show dev "*) [[ -e $st/netem-$4 ]] && echo "qdisc netem 8001: root refcnt 2 limit 100000 loss $(cat "$st/netem-$4")%" || echo "qdisc mq 0: root" ;;
"-s qdisc show dev "*)
    [[ -e $st/netem-$5 ]] || { echo "qdisc mq 0: root"; exit 0; }
    echo "qdisc netem 8001: root refcnt 2 limit 100000 loss $(cat "$st/netem-$5")%"
    echo " Sent 1000 bytes 4000 pkt (dropped 7, overlimits 0 requeues 0)"
    ;;
esac
SH
chmod +x "$tmp/fakebin/"*
export FAKE_LOG=$tmp/fake.log FAKE_STATE=$tmp/state FAKE_ROUTES=$tmp/routes
mkdir -p "$FAKE_STATE"
hostrun() { # <command>...: host.sh and irqs.sh sourced against the fake trees
    env PATH="$tmp/fakebin:$PATH" LAB_SYS_ROOT="$sys" LAB_PROC_ROOT="$proc" bash -c '. "$1/irqs.sh"; . "$1/host.sh"; . "$1/transport-settings.env"; shift; "$@"' _ "$cloud" "$@"
}
got="map: $(hostrun transport_cpu_map)
layout: $(hostrun transport_layout "0 2 4,6,8,10,12,14")
clients: $(hostrun transport_clients "0 2 4,6,8,10,12,14" kernel) / $(hostrun transport_clients "0 2 4,6,8,10,12,14" sockets)
boot: $(hostrun transport_boot_args "0 2 4,6,8,10,12,14")"
echo "4,6,8,10,12,14" >"$sys/devices/system/cpu/isolated"
echo "4,6,8,10,12,14" >"$sys/devices/system/cpu/nohz_full"
got+="
boot check: $(hostrun transport_boot_check "0 2 4,6,8,10,12,14" 2>&1; echo " rc $?")"
echo "4-14" >"$sys/devices/system/cpu/isolated"
got+="
boot wrong: $(hostrun transport_boot_check "0 2 4,6,8,10,12,14" >/dev/null 2>&1; echo "rc $?")"
# a session that declares SMT on (a sets file's `smt on`): each isolated
# core whole in the map, SMT verified on, the boot check over the set
got+="
map smt on: $(hostrun transport_cpu_map on)
smt on: $(hostrun transport_smt_on 2>&1; echo " rc $?")"
echo "4-15" >"$sys/devices/system/cpu/isolated"
echo "4-15" >"$sys/devices/system/cpu/nohz_full"
got+="
boot check smt on: $(hostrun transport_boot_check "0 2 4,5,6,7,8,9,10,11,12,13,14,15" 2>&1; echo " rc $?")
map smt bad: $(hostrun transport_cpu_map maybe >/dev/null 2>&1; echo "rc $?")"
got+="
smt stuck: $(FAKE_SMT_STUCK=1 hostrun transport_smt_off >/dev/null 2>&1; echo "rc $?")
smt: $(hostrun transport_smt_off 2>&1; echo " rc $?")
map smt off: $(hostrun transport_cpu_map)
smt on after off: $(hostrun transport_smt_on >/dev/null 2>&1; echo "rc $?")
map smt on, no sibling: $(hostrun transport_cpu_map on 2>&1; echo " rc $?")"
mkdir -p "$tmp/sys4/devices/system/cpu/cpu0/topology"
echo 0 >"$tmp/sys4/devices/system/cpu/cpu0/topology/thread_siblings_list"
got+="
too few cores: $(env LAB_SYS_ROOT="$tmp/sys4" bash -c '. "$1/host.sh"; transport_cpu_map' _ "$cloud" >/dev/null 2>&1; echo "rc $?")"
same "host.sh: the CPU map (SMT on and off), the driver's layout, the clients' CPUs, the boot line and its check, SMT off verified" "map: 0 2 4,6,8,10,12,14
layout: 0 10 4 6 8
clients: 10,12,14 / 4,6,8,10,12,14
boot: isolcpus=nohz,domain,managed_irq,4,6,8,10,12,14 nohz_full=4,6,8,10,12,14 rcu_nocbs=4,6,8,10,12,14 irqaffinity=0
boot check: boot: isolated and nohz_full 4,6,8,10,12,14
 rc 0
boot wrong: rc 1
map smt on: 0 2 4,5,6,7,8,9,10,11,12,13,14,15
smt on: smt: on as the session declares (control on active 1 online 0-15)
 rc 0
boot check smt on: boot: isolated and nohz_full 4,5,6,7,8,9,10,11,12,13,14,15
 rc 0
map smt bad: rc 2
smt stuck: rc 1
smt: smt: control off active 0 online 0,2,4,6,8,10,12,14
 rc 0
map smt off: 0 2 4,6,8,10,12,14
smt on after off: rc 1
map smt on, no sibling: transport: smt on: no core has an SMT sibling (one thread a core): nothing to keep on
 rc 2
too few cores: rc 2" "$got"

# the quiet settings: those this kernel has written and read back, the rest
# skipped; one the kernel refuses fails the step
echo 1 >"$proc/sys/vm/stat_interval"
echo 1 >"$proc/sys/kernel/nmi_watchdog"
mkdir -p "$sys/kernel/mm/transparent_hugepage"
echo never >"$sys/kernel/mm/transparent_hugepage/enabled"
q=$(hostrun transport_quiet_apply 0,2 4,6,8,10,12,14 2>&1) && qrc=0 || qrc=$?
qlines=$(grep -c '^quiet /' <<<"$q" || true)
qskip=$(grep -c '^quiet skip' <<<"$q" || true)
mkdir -p "$proc/sys/kernel/refuse"
ref=$(env PATH="$tmp/fakebin:$PATH" LAB_SYS_ROOT="$sys" LAB_PROC_ROOT="$proc" bash -c '. "$1/host.sh"; transport_quiet_settings() { echo "/proc/sys/kernel/refuse|1|test"; }; transport_quiet_apply 0 4' _ "$cloud" 2>&1) && refrc=0 || refrc=$?
# a cpumask the kernel reads back zero-padded and comma-grouped (03 for 3,
# 00000000,000000ff for ff), or a cpu list read back as ranges (0-1,4 for
# 0,1,4), is the same set of CPUs: it holds
mkdir -p "$sys/bus/workqueue/devices/writeback"
echo ff >"$sys/bus/workqueue/devices/writeback/cpumask"
mask=$(env PATH="$tmp/fakebin:$PATH" LAB_SYS_ROOT="$sys" LAB_PROC_ROOT="$proc" bash -c '. "$1/host.sh"
transport_quiet_settings() { echo "/sys/bus/workqueue/devices/writeback/cpumask|3|test"; echo "/sys/kernel/padded/cpumask|ff|test"; echo "/proc/sys/kernel/watchdog_cpumask|0,1,4|test"; }
sudo() { if [[ $1 == cat && $2 == */writeback/cpumask ]]; then echo 03; elif [[ $1 == cat && $2 == */padded/cpumask ]]; then echo 00000000,000000ff; elif [[ $1 == cat && $2 == */watchdog_cpumask ]]; then echo 0-1,4; else "$@"; fi; }
mkdir -p "$LAB_PROC_ROOT/sys/kernel"; echo 0-7 >"$LAB_PROC_ROOT/sys/kernel/watchdog_cpumask"
mkdir -p "$LAB_SYS_ROOT/kernel/padded"; echo ff >"$LAB_SYS_ROOT/kernel/padded/cpumask"
transport_quiet_apply 0 4' _ "$cloud" 2>&1) && maskrc=0 || maskrc=$?
if [[ $maskrc != 0 || $mask == *REFUSED* ]]; then
    fail "host.sh quiet settings: a zero-padded cpumask read back is the same mask: rc $maskrc: $mask"
fi
if [[ $qrc == 0 && $qlines == 3 && $qskip -ge 6 && $(cat "$proc/sys/vm/stat_interval") == 120 && $refrc == 1 && $ref == *REFUSED* ]]; then
    pass "host.sh: the lab's quiet settings (k8s lab_quiet_settings) written and read back, the absent skipped, a refused one fails"
else
    fail "host.sh quiet settings: rc $qrc, $qlines set, $qskip skipped: $q; refused: $refrc $ref"
fi

# the sysctls: written and read back; the optional ones absent on an older
# kernel; one that reads back otherwise refused
for f in ipv4/tcp_congestion_control ipv4/tcp_sack ipv4/tcp_timestamps ipv4/tcp_slow_start_after_idle ipv4/tcp_rmem ipv4/tcp_wmem core/rmem_max core/wmem_max core/busy_read core/busy_poll; do
    echo 0 >"$proc/sys/net/$f"
done
s=$(hostrun transport_sysctls_apply 2>&1) && src=0 || src=$?
chk=$(hostrun transport_sysctls_apply check 2>&1) && chkrc=0 || chkrc=$?
echo bbr >"$proc/sys/net/ipv4/tcp_congestion_control"
wrong=$(hostrun transport_sysctls_apply check 2>&1) && wrongrc=0 || wrongrc=$?
touch "$proc/sys/net/ipv4/tcp_rto_min_us"
rm "$proc/sys/net/core/busy_poll"
missing=$(hostrun transport_sysctls_apply 2>&1) && missrc=0 || missrc=$?
if [[ $src == 0 && $(grep -c '^sysctl ' <<<"$s") == 12 && $s == *"tcp_rmem 4096 1048576 16777216"* && $s == *"tcp_rto_min_us absent"* &&
    $chkrc == 0 && $wrongrc == 1 && $wrong == *"WRONG net/ipv4/tcp_congestion_control: [bbr]"* &&
    $missrc == 1 && $missing == *"MISSING net/core/busy_poll"* && $(cat "$proc/sys/net/ipv4/tcp_rto_min_us") == 8000 ]]; then
    pass "host.sh: the TCP and busy-poll sysctls written and read back (the 6.15 ones optional), a wrong or missing required one refused"
else
    fail "host.sh sysctls: $src [$s] $chkrc $wrongrc [$wrong] $missrc [$missing]"
fi

# quickack: every route to the data subnet, in every table, with quickack 1;
# the main table holds one route per data NIC (both on the subnet), which a
# prefix-keyed `ip route change` cannot tell apart
printf '%s\n' "10.20.1.0/24 dev eth1 proto kernel scope link src 10.20.1.4" "10.20.1.0/24 dev eth2 proto kernel scope link src 10.20.1.5" \
    "10.20.1.0/24 dev eth1 table 100 scope link src 10.20.1.4" \
    "10.20.1.0/24 dev eth2 table 101 scope link src 10.20.1.5" "default via 10.20.0.1 dev eth0" >"$FAKE_ROUTES"
qa=$(hostrun transport_quickack_routes 10.20.1.0/24)
before=$(hostrun transport_quickack_check 10.20.1.0/24 >/dev/null 2>&1; echo $?)
hostrun transport_quickack_apply 10.20.1.0/24 >/dev/null 2>&1
after=$(hostrun transport_quickack_check 10.20.1.0/24 2>&1)
same "host.sh: quickack 1 on every data-subnet route of every table (one per NIC in main), checked" "10.20.1.0/24 dev eth1 proto kernel scope link src 10.20.1.4 quickack 1
10.20.1.0/24 dev eth2 proto kernel scope link src 10.20.1.5 quickack 1
10.20.1.0/24 dev eth1 table 100 scope link src 10.20.1.4 quickack 1
10.20.1.0/24 dev eth2 table 101 scope link src 10.20.1.5 quickack 1
before 1
quickack: 4 route(s) to 10.20.1.0/24 with quickack 1" "$qa
before $before
$after"

# rings and coalescing, read back; a driver with no coalescing to set
got="ring: $(hostrun transport_ring enP1 1024 1024 2>&1; echo " rc $?")
ring over: $(hostrun transport_ring enP1 8192 1024 >/dev/null 2>&1; echo "rc $?")
coalesce: $(hostrun transport_coalesce_off enP1 2>&1; echo " rc $?")
coalesce stuck: $(FAKE_COALESCE_STUCK=1 hostrun transport_coalesce_off enP2 >/dev/null 2>&1; echo "rc $?")
coalesce none: $(FAKE_NO_COALESCE=1 hostrun transport_coalesce_off enP3 2>&1; echo " rc $?")
coalesce ena: $(FAKE_ENA=1 hostrun transport_coalesce_off enP4 2>&1; echo " rc $?")"
same "host.sh: rings set and read back (above the maximum refused), coalescing off and read back (only what the driver has: ENA lacks adaptive TX), none where the driver has none" "ring: ring enP1 rx 1024 tx 1024
 rc 0
ring over: rc 1
coalesce: coalesce enP1 off (off off 0 0)
 rc 0
coalesce stuck: rc 1
coalesce none: coalesce enP3: none (the driver has no interrupt moderation)
 rc 0
coalesce ena: coalesce enP4 off (off n/a 0 0)
 rc 0" "$got"

# netem on every data NIC: set and checked, the counters read, cleared
got="set: $(hostrun netem_set 0.1 eth1 eth2; hostrun netem_check 0.1 eth1 eth2 2>&1; echo " rc $?")
wrong: $(hostrun netem_check 1 eth1 eth2 >/dev/null 2>&1; echo "rc $?")
none while set: $(hostrun netem_check none eth1 >/dev/null 2>&1; echo "rc $?")
stats: $(hostrun netem_stats eth1 eth2 eth3 | tr '\n' ';')
clear: $(hostrun netem_clear eth1 eth2; hostrun netem_check none eth1 eth2 >/dev/null 2>&1; echo "rc $?")"
same "host.sh: netem set and checked on each data NIC, its root counters read, cleared and checked gone" "set: netem: eth1 eth2 at 0.1
 rc 0
wrong: rc 1
none while set: rc 1
stats: eth1 4000 7;eth2 4000 7;
clear: rc 0" "$got"

# placement: busy threads on isolated CPUs, one a CPU
got="$(printf '%s\n' "101 4 99.5 sender" "102 6 99.1 receiver" "103 0 0.3 conductor-hk" "104 10 98.0 ping" | hostrun transport_placement 4,6,8,10,12,14)
$(printf '%s\n' "101 4 99.5 sender" "104 4 98.0 ping" "105 2 90 spin" | hostrun transport_placement 4,6,8,10,12,14; echo "rc $?")"
same "host.sh: the placement verdict (busy threads off the isolated CPUs, or sharing one, fail)" "ok 3 busy threads
FAIL ping(104) and sender(101) on CPU 4; spin(105) on CPU 2, not isolated;
rc 1" "$got"
# each thread's CPU time and last CPU from its stat (a comm with spaces and
# parentheses kept whole), not ps's lifetime average, the main thread's
# line the whole process's
mkdir -p "$proc/500/task/500" "$proc/500/task/501"
fields() { printf '%s ' S $(seq 4 13) "$1" "$2" $(seq 16 38) "$3" 0 0; }
echo "500 (aeron_lat) $(fields 3 2 0)" >"$proc/500/task/500/stat"
echo "501 (my (busy) one) $(fields 700 90 6)" >"$proc/500/task/501/stat"
same "host.sh: transport_task_times reads each thread's utime+stime and CPU from /proc/<pid>/task/<tid>/stat" "500 5 0 aeron_lat
501 790 6 my_(busy)_one" "$(hostrun transport_task_times)"
rm -rf "$proc/500"

# the 2 MiB hugetlbfs of aeron-ipc-huge's Aeron directory: refused when the
# kernel has too few free, unreserved 2 MiB pages (no mount tried); else
# mounted with every page reserved for it (min_size) and verified from
# /proc/mounts (hugetlbfs, pagesize 2M, its size); a mount already there and
# right kept, one of another type, page size or size refused
cat >"$tmp/fakebin/mount" <<'SH'
#!/usr/bin/env bash
# mount -t <type> -o <options> <source> <mount point>: into the fake /proc/mounts
echo "mount $*" >>"$FAKE_LOG"
o=$4
[[ -n ${FAKE_MOUNT_PAGESIZE:-} ]] && o=${o//pagesize=2M/pagesize=$FAKE_MOUNT_PAGESIZE}
echo "$5 $6 ${FAKE_MOUNT_FS:-$2} rw,relatime,$o 0 0" >>"$LAB_PROC_ROOT/mounts"
SH
cat >"$tmp/fakebin/mountpoint" <<'SH'
#!/usr/bin/env bash
# mountpoint -q <dir>: whether the fake /proc/mounts has it
awk -v d="$2" '$2 == d {found = 1} END {exit !found}' "$LAB_PROC_ROOT/mounts" 2>/dev/null
SH
chmod +x "$tmp/fakebin/mount" "$tmp/fakebin/mountpoint"
hp=$sys/kernel/mm/hugepages/hugepages-2048kB
mkdir -p "$hp"
echo 2048 >"$hp/nr_hugepages"
echo 20 >"$hp/free_hugepages"
echo 0 >"$hp/resv_hugepages"
: >"$proc/mounts"
: >"$FAKE_LOG"
mnt=$tmp/huge-aeron
few=$(hostrun transport_huge_mount "$mnt" 32 2>&1; echo "rc $?")
grep -q '^mount ' "$FAKE_LOG" && few+=" (mounted)"
echo 2048 >"$hp/free_hugepages"
echo 4 >"$hp/resv_hugepages"
ok=$(hostrun transport_huge_mount "$mnt" 32 2>&1; echo "rc $?")
again=$(hostrun transport_huge_mount "$mnt" 32 2>&1; echo "rc $?")
calls=$(grep -c '^mount ' "$FAKE_LOG")
args=$(grep -m1 '^mount ' "$FAKE_LOG")
other=$(hostrun transport_huge_check "$mnt" 16 2>&1; echo "rc $?")
tmpfs=$(FAKE_MOUNT_FS=tmpfs hostrun transport_huge_mount "$tmp/huge-tmpfs" 32 2>&1; echo "rc $?")
gig=$(FAKE_MOUNT_PAGESIZE=1024M hostrun transport_huge_mount "$tmp/huge-1g" 32 2>&1; echo "rc $?")
gone=$(hostrun transport_huge_check "$tmp/huge-none" 32 2>&1; echo "rc $?")
same "host.sh: aeron-ipc-huge's hugetlbfs: refused on too few free 2 MiB pages, mounted with its pages reserved and verified (hugetlbfs, 2M, its size), kept when right, refused when another type, page size or size" "hugetlbfs $mnt: 32 pages of 2 MiB wanted, the kernel has 20 free and unreserved (vm.nr_hugepages 2048): not mounted
rc 1
hugetlbfs $mnt: pagesize 2M, size 67108864, min_size 67108864 (32 pages of 2 MiB, reserved)
rc 0
hugetlbfs $mnt: pagesize 2M, size 67108864, min_size 67108864 (32 pages of 2 MiB, reserved)
rc 0
1 mount -t hugetlbfs -o mode=1777,pagesize=2M,size=67108864,min_size=67108864 none $mnt
hugetlbfs $mnt: rw,relatime,mode=1777,pagesize=2M,size=67108864,min_size=67108864, not hugetlbfs with pagesize=2M, size=33554432 and min_size=33554432
rc 1
hugetlbfs $tmp/huge-tmpfs: tmpfs rw,relatime,mode=1777,pagesize=2M,size=67108864,min_size=67108864, not hugetlbfs with pagesize=2M, size=67108864 and min_size=67108864
rc 1
hugetlbfs $tmp/huge-1g: rw,relatime,mode=1777,pagesize=1024M,size=67108864,min_size=67108864, not hugetlbfs with pagesize=2M, size=67108864 and min_size=67108864
rc 1
hugetlbfs $tmp/huge-none: not a mount point
rc 1" "$few
$ok
$again
$calls $args
$other
$tmpfs
$gig
$gone"
# the margin: the logs aeron-ipc-huge's points hold at once (a point's two
# runs' ping and echo publications, the last point's lingering), the CnC and
# loss report, twice over, in 2 MiB pages
same "host.sh: transport_huge_pages: the pages a 2 MiB hugetlbfs needs for an arm's term length and streams, twice over" "28 20 140" \
    "$(hostrun transport_huge_pages 1048576 1) $(hostrun transport_huge_pages 65536 1) $(hostrun transport_huge_pages 1048576 8)"

# irqs.sh: every data-NIC IRQ (msi_irqs) and VMBus channel moved to the IRQ
# CPU and checked; one that refuses fails it; IRQ deltas on the hot CPUs
for i in eth1 enP1; do mkdir -p "$sys/class/net/$i/device/msi_irqs"; done
for irq in 40 41; do touch "$sys/class/net/enP1/device/msi_irqs/$irq"; mkdir -p "$proc/irq/$irq"; echo 0-15 >"$proc/irq/$irq/smp_affinity_list"; done
mkdir -p "$sys/class/net/eth1/device/channels/7" "$sys/class/net/eth1/device/channels/8"
echo 5 >"$sys/class/net/eth1/device/channels/7/cpu"
echo 9 >"$sys/class/net/eth1/device/channels/8/cpu"
pin=$(hostrun irqs_pin_data 2 test eth1 enP1 2>&1; echo "rc $?")
check=$(hostrun irqs_data_check 2 eth1 enP1 | tail -1)
echo 0 >"$proc/irq/41/smp_affinity_list"
off=$(hostrun irqs_data_check 2 eth1 enP1; echo "rc $?")
none=$(hostrun irqs_data_check 2 eth9 >/dev/null; echo "rc $?")
printf '           CPU0       CPU2       CPU4       CPU6\n  40:  10  20  0  0  PCI-MSIX  mana_q0\nLOC:  100  100  10  12  Local timer interrupts\nRES:  1  1  0  2  Rescheduling interrupts\n' >"$tmp/irq-before"
printf '           CPU0       CPU2       CPU4       CPU6\n  40:  11  90  0  3  PCI-MSIX  mana_q0\nLOC:  200  200  40  42  Local timer interrupts\nRES:  1  1  0  5  Rescheduling interrupts\n' >"$tmp/irq-after"
delta=$(hostrun irqs_hot_delta "$tmp/irq-before" "$tmp/irq-after" 4,6,8)
same "irqs.sh: every data-NIC IRQ and VMBus channel on the IRQ CPU (one elsewhere, or none at all, fails); each hot CPU's interrupts over a point" "irqs test: ok eth1 channel 7;ok eth1 channel 8;ok enP1 irq 40;ok enP1 irq 41;4 of 4 data-NIC interrupts on CPU 2
rc 0
4 of 4 data-NIC interrupts on CPU 2
ok eth1 channel 7
ok eth1 channel 8
ok enP1 irq 40
FAIL enP1 irq 41 on 0
3 of 4 data-NIC interrupts on CPU 2
rc 1
rc 1
cpu4=30/0 cpu6=36/3" "$pin
$check
$off
$none
$delta"

# irqs.sh: a moved data-NIC IRQ still effective on its old CPU (x86 under
# Hyper-V moves it only at its next interrupt, which the old CPU takes: a real
# session's idle mlx5 vectors each fired once on their hot CPU mid-run) is
# made to fire now, by a frame from each CPU it is still effective on; one
# that never fires is reported, not fatal (fairness.py's irq rule counts
# what lands on a hot CPU)
for irq in 40 41; do echo 2 >"$proc/irq/$irq/smp_affinity_list"; done
echo 5 >"$proc/irq/40/effective_affinity_list"
echo 2 >"$proc/irq/41/effective_affinity_list"
# the kick: a frame from CPU <cpu> on <iface> fires every vector of it still
# effective there (the fake moves it: its effective CPUs become its affinity)
cat >"$tmp/fakebin/fakekick" <<'EOF'
#!/usr/bin/env bash
echo "kick cpu $1 $2" >>"$FAKE_KICKS"
for f in "$LAB_SYS_ROOT/class/net/$2/device/msi_irqs"/*; do
    [[ -e $f ]] || continue
    irq=${f##*/}
    [[ ${FAKE_KICK_DEAD:-} == "$irq" ]] && continue
    [[ $(cat "$LAB_PROC_ROOT/irq/$irq/effective_affinity_list") == "$1" ]] &&
        cp "$LAB_PROC_ROOT/irq/$irq/smp_affinity_list" "$LAB_PROC_ROOT/irq/$irq/effective_affinity_list"
done
exit 0
EOF
chmod +x "$tmp/fakebin/fakekick"
export FAKE_KICKS=$tmp/kicks
settled=$(hostrun irqs_data_settle 2 test fakekick eth1 enP1 2>&1; echo "rc $?")
kicks=$(cat "$FAKE_KICKS")
echo 5 >"$proc/irq/40/effective_affinity_list"
dead=$(FAKE_KICK_DEAD=40 hostrun irqs_data_settle 2 test fakekick eth1 enP1 2>&1; echo "rc $?")
same "irqs.sh: a data-NIC IRQ still effective on its old CPU fired by a frame from that CPU, then checked; one that never moves reported" "irqs test: 2 of 2 data-NIC IRQs effective on CPU 2 (40 moved by a frame from CPU 5)
rc 0
kick cpu 5 enP1
irqs test: 1 of 2 data-NIC IRQs effective on CPU 2; still on another until their next interrupt: 40@5 (after 3 frames from each)
rc 0" "$settled
$kicks
$dead"

# host.sh's Onload steps (vm.sh onload-up, stop) against a fake ENA: ethtool
# -l/-L its queues (4 at most), ip its MTU and maxmtu (9216; ENA's
# ENA_XDP_MAX_MTU, 3498, while an XDP program is attached), sfc_resource's
# afxdp register and unregister attaching and detaching it (FAKE_XDP_FAIL:
# Onload adds the NIC but its XDP set-up fails, maxmtu unchanged)
on=$tmp/onload
mkdir -p "$on/bin" "$on/sys/module/sfc_resource/afxdp" "$on/sys/module/sfc_resource/parameters" "$on/state"
for i in ens6 ens7; do mkdir -p "$on/sys/class/net/$i"; echo 9001 >"$on/sys/class/net/$i/mtu"; echo 4 >"$on/state/$i.comb"; echo 9216 >"$on/state/$i.max"; : >"$on/sys/class/net/$i/napi_defer_hard_irqs"; : >"$on/sys/class/net/$i/gro_flush_timeout"; done
: >"$on/sys/module/sfc_resource/afxdp/register"
: >"$on/sys/module/sfc_resource/afxdp/unregister"
cat >"$on/bin/sudo" <<'SH'
#!/usr/bin/env bash
echo "sudo $*" >>"$FAKE_LOG"
if [[ $1 == tee && $2 == */afxdp/register ]]; then
    i=$(cat); echo "$i" >>"$FAKE_ON/registered"
    [[ -n ${FAKE_XDP_FAIL:-} ]] || echo 3498 >"$FAKE_ON/state/$i.max"
    exit 0
fi
if [[ $1 == tee && $2 == */afxdp/unregister ]]; then
    i=$(cat); echo "$i" >>"$FAKE_ON/unregistered"; echo 9216 >"$FAKE_ON/state/$i.max"; exit 0
fi
exec "$@"
SH
cat >"$on/bin/ethtool" <<'SH'
#!/usr/bin/env bash
echo "ethtool $*" >>"$FAKE_LOG"
st=$FAKE_ON/state
case $1 in
-l) printf 'Channel parameters for %s:\nPre-set maximums:\nRX:\t\tn/a\nTX:\t\tn/a\nOther:\t\tn/a\nCombined:\t4\nCurrent hardware settings:\nRX:\t\tn/a\nTX:\t\tn/a\nOther:\t\tn/a\nCombined:\t%s\n' "$2" "$(cat "$st/$2.comb")" ;;
-L) [[ $4 -le 4 ]] || { echo "Cannot set device channel parameters: Invalid argument" >&2; exit 1; }
    # ENA's XDP: queues at most half the maximum while a program is attached
    [[ $(cat "$st/$2.max") != 3498 || $4 -le 2 ]] || { echo "Cannot set device channel parameters: Invalid argument" >&2; exit 1; }
    echo "$4" >"$st/$2.comb" ;;
esac
SH
cat >"$on/bin/ip" <<'SH'
#!/usr/bin/env bash
echo "ip $*" >>"$FAKE_LOG"
st=$FAKE_ON/state
if [[ "$1 $2 $3 $4 $5" == "-d -j link show dev" ]]; then
    printf '[{"ifname":"%s","mtu":%s,"max_mtu":%s}]\n' "$6" "$(cat "$LAB_SYS_ROOT/class/net/$6/mtu")" "$(cat "$st/$6.max")"
elif [[ "$1 $2 $3" == "link set dev" && $5 == mtu ]]; then
    (($6 <= $(cat "$st/$4.max"))) || { echo "Error: mtu greater than device maximum." >&2; exit 2; }
    echo "$6" >"$LAB_SYS_ROOT/class/net/$4/mtu"
fi
SH
chmod +x "$on/bin/"*
onrun() { env PATH="$on/bin:$PATH" LAB_SYS_ROOT="$on/sys" FAKE_ON="$on" bash -c '. "$1/host.sh"; . "$1/transport-settings.env"; shift; "$@"' _ "$cloud" "$@"; }
up=$(onrun onload_nic_up ens6 "$on/st" 2>&1; echo "rc $?")
st6="$(cat "$on/sys/class/net/ens6/mtu") $(cat "$on/state/ens6.comb") $(cat "$on/state/ens6.max") $(cat "$on/sys/class/net/ens6/napi_defer_hard_irqs")/$(cat "$on/sys/class/net/ens6/gro_flush_timeout") kept $(cat "$on/st/ens6.mtu") $(cat "$on/st/ens6.combined")"
down=$(onrun onload_nic_down ens6 "$on/st" 2>&1; echo "rc $?")
back="$(cat "$on/sys/class/net/ens6/mtu") $(cat "$on/state/ens6.comb") $(cat "$on/state/ens6.max") left [$(ls "$on/st" | grep -v ethtool.err | tr '\n' ' ')] $(tr '\n' ' ' <"$on/unregistered")"
again=$(onrun onload_nic_down ens7 "$on/st" 2>&1; echo "rc $?")
xdpfail=$(FAKE_XDP_FAIL=1 onrun onload_nic_up ens7 "$on/st" 2>&1; echo "rc $?")
undo=$(onrun onload_nic_down ens7 "$on/st" 2>&1; echo "rc $?")
st7="$(cat "$on/sys/class/net/ens7/mtu") $(cat "$on/state/ens7.comb") $(tail -1 "$on/unregistered")"
nomod=$(env PATH="$on/bin:$PATH" LAB_SYS_ROOT="$on/none" FAKE_ON="$on" bash -c '. "$1/host.sh"; . "$1/transport-settings.env"; onload_nic_up ens6 "$2"' _ "$cloud" "$on/st2" 2>&1; echo "rc $?")
# the mid-window check: an Onload stack of the arm's process (by its name
# in /proc/<pid>/comm), or none
mkdir -p "$on/proc/4242" "$on/proc/4343"
echo tcp_lat >"$on/proc/4242/comm"
echo sshd >"$on/proc/4343/comm"
dump='#stack-id stack-name      pids
0         -               4242 
1         -               4343,4344 '
verdict="$(printf '%s\n' "$dump" | LAB_PROC_ROOT="$on/proc" bash -c '. "$1/host.sh"; onload_stacks_verdict tcp_lat' _ "$cloud"; echo " rc $?")
$(printf '%s\n' "$dump" | LAB_PROC_ROOT="$on/proc" bash -c '. "$1/host.sh"; onload_stacks_verdict media_driver' _ "$cloud"; echo " rc $?")
$(printf '' | LAB_PROC_ROOT="$on/proc" bash -c '. "$1/host.sh"; onload_stacks_verdict tcp_lat' _ "$cloud"; echo " rc $?")"
same "host.sh Onload: a data NIC on one queue at Onload's MTU, interrupts not deferred, registered and its XDP program read back (maxmtu 3498), each first value kept; undone back to its 4 queues and 9001 (unregistered, maxmtu 9216); nothing for a NIC never touched; a failed XDP set-up fails closed and is unregistered all the same; no sfc_resource refused; the mid-window stack check by process name" \
"onload ens6: registered, AF_XDP on its native XDP (maxmtu 9216 -> 3498), 1 queue (was 4 of 4), MTU 3498 (was 9001), napi_defer 0
rc 0
3498 1 3498 0/0 kept 9001 4
rc 0
9001 4 9216 left [] ens6 
rc 0
onload: ens7's XDP program not attached after registering it (maxmtu 9216, then 9216): Onload's AF_XDP set-up failed on it (dmesg: hardware init failed)
rc 1
rc 0
9001 4 ens7
onload: $on/none/module/sfc_resource/afxdp/register missing (the sfc_resource module not loaded)
rc 1
ok 1 stack(s) of tcp_lat: 0:4242
 rc 0
FAIL no Onload stack of media_driver (stacks: 0:4242 1:4343,4344)
 rc 1
FAIL no Onload stack of tcp_lat (stacks: none)
 rc 1" "$up
$st6
$down
$back
$again
$xdpfail
$undo
$st7
$nomod
$verdict"

# ---- az.sh against a fake az of several groups ----
mkdir -p "$tmp/fakeaz" "$tmp/azstate/groups"
cat >"$tmp/fakeaz/az" <<'FAKEAZ'
#!/usr/bin/env bash
st=${FAKE_AZ_STATE:?}
echo "az $*" >>"${FAKE_AZ_LOG:?}"
arg() { local want=$1; shift; while (($#)); do [[ $1 == "$want" ]] && { echo "$2"; return; }; shift; done; }
g=$(arg -g "$@"); [[ -n $g ]] || g=$(arg -n "$@")
case "$*" in
"account show"*) ;;
"group exists"*) [[ -d $st/groups/$g ]] && echo true || echo false ;;
"group create"*)
    mkdir -p "$st/groups/$g"
    [[ $* =~ run=([^ ]+) ]] && echo "${BASH_REMATCH[1]}" >"$st/groups/$g/run"
    [[ $* =~ pair=([^ ]+) ]] && echo "${BASH_REMATCH[1]}" >"$st/groups/$g/pair"
    ;;
"group delete"*) rm -rf "${st:?}/groups/$g" ;;
"group wait"*) ;;
"group show"*) [[ -d $st/groups/$g ]] || exit 3; cat "$st/groups/$g/run" ;;
"vm list-skus"*MaxNetworkInterfaces*) [[ -n ${FAKE_AZ_NICS_FAIL:-} ]] && { echo "ERROR: throttled" >&2; exit 1; }; echo "${FAKE_AZ_NICS:-8}" ;;
"vm list-skus"*) echo standardDSv6Family ;;
"vm list-usage"*) case "$*" in *"=='cores'"*) printf '10\n200\n' ;; *) printf '0\n100\n' ;; esac ;;
"resource list --tag run="*)
    all="$*"; run=${all#*--tag run=}; run=${run%% *}
    want=; [[ $* =~ tags\.pair==\'([a-z]+)\' ]] && want=${BASH_REMATCH[1]}
    n=0
    for d in "$st"/groups/*/; do
        [[ -d $d && $(cat "$d/run" 2>/dev/null) == "$run" ]] || continue
        [[ -z $want || $(cat "$d/pair" 2>/dev/null) == "$want" ]] && n=$((n + 2))
    done
    echo $n ;;
"resource list"*) ;;
"vm show"*)
    vm=$(arg -n "$@")
    if [[ -e $st/evict-$vm ]]; then echo "ERROR: (ResourceNotFound) The Resource 'Microsoft.Compute/virtualMachines/$vm' under resource group '$g' was not found." >&2; exit 3; fi
    echo "$vm" ;;
"network public-ip show"*) [[ -n ${FAKE_AZ_IP:-} ]] || exit 1; echo "$FAKE_AZ_IP" ;;
"network nic show"*) echo 10.20.0.4 ;;
"disk show"*) echo "/subscriptions/0/resourceGroups/$g/providers/Microsoft.Compute/disks/$(arg -n "$@")" ;;
esac
exit 0
FAKEAZ
chmod +x "$tmp/fakeaz/az"
export FAKE_AZ_STATE=$tmp/azstate FAKE_AZ_LOG=$tmp/az-calls
got=$(
    export PATH="$tmp/fakeaz:$PATH"
    . "$cloud/azure/az.sh"
    echo "nics: $(azure_nic_check northcentralus 3 Standard_D16s_v6 Standard_D16s_v6 2>&1; echo " rc $?")"
    echo "nics few: $(FAKE_AZ_NICS=2 azure_nic_check northcentralus 3 Standard_D16s_v6 2>&1 >/dev/null; echo " rc $?")"
    echo "nics junk: $(FAKE_AZ_NICS=lots azure_nic_check northcentralus 3 Standard_D16s_v6 >/dev/null 2>&1; echo "rc $?")"
    echo "nics unread: $(FAKE_AZ_NICS_FAIL=1 azure_nic_check northcentralus 3 Standard_D16s_v6 >/dev/null 2>&1; echo "rc $?")"
    echo "spot: [$(azure_priority_args Spot)] regular: [$(azure_priority_args Regular)]"
    mkdir -p "$FAKE_AZ_STATE/groups/ga" "$FAKE_AZ_STATE/groups/gb"
    echo s1 >"$FAKE_AZ_STATE/groups/ga/run"; echo a >"$FAKE_AZ_STATE/groups/ga/pair"
    echo s1 >"$FAKE_AZ_STATE/groups/gb/run"; echo b >"$FAKE_AZ_STATE/groups/gb/pair"
    echo "left: all $(azure_left s1) a $(azure_left s1 a) b $(azure_left s1 b) other $(azure_left s2 a)"
    rm -rf "$FAKE_AZ_STATE/groups/"*
    # the watch: a VM Azure deleted TERMs the run and leaves its marker
    mkdir -p "$tmp/evict"
    bash -c 'trap "echo termed >'"$tmp"'/evict/victim; exit 0" TERM; while :; do sleep 0.1; done' & victim=$!
    touch "$FAKE_AZ_STATE/evict-b"
    azure_eviction_watch g1 0.2 "$victim" "$tmp/evict" a b 2>"$tmp/evict/log"
    wait "$victim" 2>/dev/null || true
    echo "evicted: $(ls "$tmp/evict" | tr '\n' ' ')$(grep -c 'spot eviction: VM b' "$tmp/evict/log")"
    rm -f "$FAKE_AZ_STATE/evict-b"
)
same "az.sh: MaxNetworkInterfaces read (too few, unread or not a number refused), Spot's arguments, leftovers counted by pair, the eviction watch" "nics: nics: MaxNetworkInterfaces Standard_D16s_v6 8, Standard_D16s_v6 8; the lab needs 3
 rc 0
nics few: nics: Standard_D16s_v6 takes 2 NICs and the lab needs 3 (management and two data NICs): pick a size that takes 3
 rc 2
nics junk: rc 2
nics unread: rc 2
spot: [--priority Spot --eviction-policy Delete --max-price -1] regular: []
left: all 4 a 2 b 2 other 0
evicted: evicted-b log victim 1" "$got"

# ---- azure/lab.sh LAB_SUITE=transport ----
# a home with an SSH key, the snapshots in $tmp, every cloud CLI a fake
# logging its calls and failing (refusals must come before any call)
mkdir -p "$tmp/home/.ssh" "$tmp/labtmp" "$tmp/nocloud" "$tmp/repo"
echo 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA lab-test' >"$tmp/home/.ssh/id_ed25519.pub"
printf '#!/bin/sh\necho "$0 $*" >>"%s/cloud-calls"\nexit 1\n' "$tmp" >"$tmp/nocloud/az"
chmod +x "$tmp/nocloud/az"
lab() { env PATH="$tmp/nocloud:$PATH" HOME="$tmp/home" TMPDIR="$tmp/labtmp" LAB_REPO="$tmp/repo" LAB_SUITE=transport "$@" bash "$cloud/azure/lab.sh"; }
# the neutral fixture of the hooks: an arms file and a source tree
fx=$tmp/fixture
mkdir -p "$fx/src"
cat >"$fx/arms.tsv" <<'EOF'
# a neutral arm and set, as an external suite would add its own
arm  fixture-echo  fixture_lat  sockets  two  --flavour plain
set  fx            a    -  ab  1000,2000  1  0,0.1  fixture-echo,aa
EOF
cat >"$fx/src/lab-build.sh" <<'EOF'
#!/usr/bin/env bash
# the fixture's build: its binary into the directory given
set -euo pipefail
cp "$(dirname "$0")/fixture_lat" "$1/fixture_lat"
chmod +x "$1/fixture_lat"
EOF
printf 'arm  fixture-dpdk  fixture_lat  dpdk  two\nset  fd  a  -  ab  1000  1  0.1  fixture-dpdk\n' >"$fx/dpdk-netem.tsv"
mkdir -p "$fx/nobuild"
why=
for refused in "LAB_PAIR=#LAB_PAIR is a or b" "LAB_PAIR=c#LAB_PAIR is a or b" "LAB_PAIR=a LAB_PRIORITY=Low#LAB_PRIORITY Low" \
    "LAB_PAIR=a LAB_ARMS_FILE=$fx/none.tsv#not a readable file" "LAB_PAIR=a LAB_ARMS_FILE=$fx/dpdk-netem.tsv#netem cannot reach" \
    "LAB_PAIR=a LAB_EXTRA_SRC=$fx/nobuild#a directory with a lab-build.sh" "LAB_PAIR=a LAB_MODES=kernel#latency suite's" \
    "LAB_PAIR=a LAB_RUN_STAMP=today#LAB_RUN_STAMP" "LAB_PAIR=a TRANSPORT_POINT_S=x#TRANSPORT_POINT_S is a whole number" \
    "LAB_PAIR=a LAB_MAX_HOURS=5#give LAB_MAX_HOURS="; do
    rm -f "$tmp/cloud-calls"
    # shellcheck disable=SC2086
    out=$(lab ${refused%#*} 2>&1) && why+=" ran with [${refused%#*}];"
    [[ $out == *"${refused#*#}"* ]] || why+=" [${refused%#*}]: $(tail -1 <<<"$out");"
    [[ ! -s $tmp/cloud-calls ]] || why+=" [${refused%#*}] called the cloud;"
done
out=$(LAB_SUITE= lab LAB_SUITE=latency LAB_PAIR=a 2>&1) && why+=" latency with LAB_PAIR ran;"
[[ $out == *"transport suite's"* ]] || why+=" latency with LAB_PAIR: $(tail -1 <<<"$out");"
[[ -z $why ]] && pass "azure/lab.sh LAB_SUITE=transport refuses a bad pair, priority, arms file (NETEM on its DPDK arm too), extra tree, latency settings, stamp, duration and deadline before any az call" ||
    fail "azure/lab.sh transport refusals:$why"
est=$(lab LAB_ESTIMATE=1 2>&1) && estrc=0 || estrc=$?
esta=$(lab LAB_ESTIMATE=1 LAB_PAIR=a LAB_ARMS_FILE="$fx/arms.tsv" 2>&1) && estarc=0 || estarc=$?
if [[ $estrc == 0 && $est == *"pair a: 510 points"*"pair b: 420 points"*"both pairs: about \$"* && $est == *"Standard_D16s_v6"* &&
    $estarc == 0 && $esta == *"pair a: 550 points (sets 2a, fan, 3a, fx)"* && $esta != *"pair b"* && ! -s $tmp/cloud-calls ]]; then
    pass "azure/lab.sh LAB_ESTIMATE=1: both pairs' points, hours and dollars on Standard_D16s_v6 by default; one pair with LAB_PAIR, an arms file's sets in it; no cloud call"
else
    fail "azure/lab.sh transport estimate ($estrc, $estarc): $est | $esta"
fi
echo "$est" >"$tmp/estimate.txt"

# the runs against the fake az, to their VMs: a public address never found
# ends them there (FAKE_AZ_IP unset), through their teardown
mkdir -p "$tmp/fakecli"
printf '#!/bin/sh\nexit 1\n' >"$tmp/fakecli/curl"
# sleep at most 0.2 s: the lab's polls and retries, run fast
printf '#!/bin/sh\nexec /bin/sleep "$(awk -v s="$1" '"'"'BEGIN {print (s > 0.2 ? 0.2 : s)}'"'"')"\n' >"$tmp/fakecli/sleep"
chmod +x "$tmp/fakecli/"*
azlab() { # <out file> <vars>...
    local o=$1
    shift
    env PATH="$tmp/fakecli:$tmp/fakeaz:$PATH" HOME="$tmp/home" TMPDIR="$tmp/labtmp" LAB_REPO="$tmp/repo" LAB_SUITE=transport \
        LAB_SSH_FROM=203.0.113.9 LAB_DETACH=0 "$@" bash "$cloud/azure/lab.sh" >"$o" 2>&1
}
: >"$FAKE_AZ_LOG"
rm -rf "$FAKE_AZ_STATE/groups/"* "$tmp/repo/target"
azlab "$tmp/nic.out" LAB_PAIR=a FAKE_AZ_NICS=2 && why=" exit 0" || why=
grep -q 'takes 2 NICs' "$tmp/nic.out" || why+=" not said;"
! grep -q 'group create' "$FAKE_AZ_LOG" || why+=" a group created;"
[[ -z $why ]] && pass "azure/lab.sh: a size taking fewer than three NICs refused before any group exists" || fail "azure/lab.sh NIC preflight:$why $(tail -2 "$tmp/nic.out")"

# two pairs side by side, one run stamp: their own groups and directories,
# tagged run and pair, each deleting only its own and counting its own
# leftovers. Pair b's group stands in for a pair still running when a's
# teardown counts (the fake counts 2 resources a group).
: >"$FAKE_AZ_LOG"
rm -rf "$FAKE_AZ_STATE/groups/"* "$tmp/repo/target"
azlab "$tmp/pa.out" LAB_PAIR=a LAB_RUN_STAMP=20261025-120000 &
p1=$!
azlab "$tmp/pb.out" LAB_PAIR=b LAB_RUN_STAMP=20261025-120000 LAB_PRIORITY=Spot &
p2=$!
wait $p1 || true
wait $p2 || true
why=
ga=ergon-lab-transport-a-20261025-120000 gb=ergon-lab-transport-b-20261025-120000
for p in a b; do
    g=ergon-lab-transport-$p-20261025-120000
    creates=$(grep -E " create .*-g $g |group create -n $g |^az tag update .*/$g/" "$FAKE_AZ_LOG" | grep -vE 'subnet create|nsg rule create' || true)
    [[ $(grep -c . <<<"$creates") -ge 12 ]] || why+=" pair $p: $(grep -c . <<<"$creates") creates;"
    ! grep -vq -- "--tags owner=ergon-lab run=20261025-120000 pair=$p" <<<"$creates" || why+=" pair $p: a create untagged;"
    [[ -d $tmp/repo/target/md-dpdk-bench/20261025-120000-transport-$p ]] || why+=" pair $p: no run directory;"
    grep -q "verified: resource group $g no longer exists" "$tmp/p$p.out" || why+=" pair $p: not verified gone;"
    grep -q "left tagged run=20261025-120000 pair=$p: 0" "$tmp/p$p.out" || why+=" pair $p: leftovers not its own;"
done
[[ $(grep -c "group delete -n $ga" "$FAKE_AZ_LOG") -ge 1 && $(grep -c "group delete -n $gb" "$FAKE_AZ_LOG") -ge 1 ]] || why+=" a group not deleted;"
grep "vm create -g $gb" "$FAKE_AZ_LOG" | grep -q -- '--priority Spot --eviction-policy Delete --max-price -1' || why+=" b not Spot;"
! grep "vm create -g $ga" "$FAKE_AZ_LOG" | grep -q -- '--priority' || why+=" a not Regular;"
grep -q 'vm create.*--size Standard_D16s_v6' "$FAKE_AZ_LOG" || why+=" not D16s_v6;"
# a's teardown with b's group standing: b's kept, a's count is 0
: >"$FAKE_AZ_LOG"
rm -rf "$tmp/repo/target"
mkdir -p "$FAKE_AZ_STATE/groups/$gb"
echo 20261025-120000 >"$FAKE_AZ_STATE/groups/$gb/run"
echo b >"$FAKE_AZ_STATE/groups/$gb/pair"
azlab "$tmp/pa2.out" LAB_PAIR=a LAB_RUN_STAMP=20261025-120000 || true
[[ -d $FAKE_AZ_STATE/groups/$gb ]] || why+=" a's run deleted b's group;"
! grep -q "group delete -n $gb" "$FAKE_AZ_LOG" || why+=" a's run asked to delete b's group;"
grep -q 'left tagged run=20261025-120000 pair=a: 0' "$tmp/pa2.out" || why+=" a counted b's resources: $(grep 'tagged run' "$tmp/pa2.out");"
rm -rf "$FAKE_AZ_STATE/groups/"*
[[ -z $why ]] && pass "azure/lab.sh: two pairs side by side take their own groups, directories and tags (owner, run, pair), Spot or Regular each, delete and verify only their own, and count only their own leftovers" ||
    fail "azure/lab.sh two pairs:$why"

# Spot eviction: once the VMs exist (an address found; ssh, scp and git
# faked: the run waits on its bootstrap), Azure deletes VM b: the watch ends
# the run through its teardown, exit 75, the group deleted and verified
git -C "$tmp/repo" init -q 2>/dev/null && touch "$tmp/repo/f" && git -C "$tmp/repo" add f &&
    git -C "$tmp/repo" -c user.email=t@t -c user.name=t commit -qm f
printf '#!/bin/sh\nexit 0\n' >"$tmp/fakecli/ssh"
printf '#!/bin/sh\nexit 0\n' >"$tmp/fakecli/scp"
chmod +x "$tmp/fakecli/ssh" "$tmp/fakecli/scp"
: >"$FAKE_AZ_LOG"
rm -rf "$tmp/repo/target"
( for _ in $(seq 100); do grep -q 'spot: both VMs looked for' "$tmp/evict.out" 2>/dev/null && break; /bin/sleep 0.1; done; touch "$FAKE_AZ_STATE/evict-b" ) &
azlab "$tmp/evict.out" LAB_PAIR=b LAB_PRIORITY=Spot LAB_EVICTION_POLL=0.2 FAKE_AZ_IP=203.0.113.20 && evrc=0 || evrc=$?
rm -f "$FAKE_AZ_STATE/evict-b"
why=
[[ $evrc == 75 ]] || why+=" exit $evrc;"
grep -q 'spot eviction: VM b' "$tmp/evict.out" || why+=" not said;"
grep -q 'verified: resource group ergon-lab-transport-b-.* no longer exists' "$tmp/evict.out" || why+=" not deleted and verified;"
[[ -z $(ls "$FAKE_AZ_STATE/groups") ]] || why+=" a group left;"
# its own group only: a real run's backstop on this machine is not this test's
evg=$(grep -o 'creating ergon-lab-transport-b-[0-9-]*' "$tmp/evict.out" | head -1 | cut -d' ' -f2)
[[ -n $evg ]] || why+=" its group not named;"
! pgrep -f "ergon-lab-reaper .*$evg" >/dev/null || why+=" its backstop left running;"
[[ -z $why ]] && pass "azure/lab.sh: a Spot VM Azure evicts ends the run through its teardown (exit 75), the group deleted and verified, the backstop stopped" ||
    fail "azure/lab.sh eviction:$why $(tail -3 "$tmp/evict.out")"
rm -f "$tmp/fakecli/ssh" "$tmp/fakecli/scp"

# ---- aws/lab.sh LAB_SUITE=transport ----
# Refusals and the estimate against an aws that logs and fails (nothing may
# reach it); then whole runs against fake_aws.py (stateful: it creates, lists
# and deletes what lab.sh asks), ssh and scp that succeed, a curl that fails
# and a sleep of 10 ms: a run creates and launches everything, then stops at
# packing the checkout (no git repository) and tears down by tag.
awsets=$cloud/transport-aws-dpdk.tsv
printf '#!/bin/sh\necho "$0 $*" >>"%s/cloud-calls"\nexit 1\n' "$tmp" >"$tmp/nocloud/aws"
chmod +x "$tmp/nocloud/aws"
awslab() { env PATH="$tmp/nocloud:$PATH" HOME="$tmp/home" TMPDIR="$tmp/labtmp" LAB_REPO="$tmp/repo" LAB_SUITE=transport "$@" bash "$cloud/aws/lab.sh"; }
why=
for refused in "LAB_PAIR=#LAB_PAIR is a or b" "LAB_PAIR=c#LAB_PAIR is a or b" "LAB_PAIR=a LAB_TUNE=1#runs on Azure" \
    "LAB_PAIR=a LAB_ALLOW_ARM=1 LAB_TYPE=c8g.2xlarge LAB_SETS_FILE=$awsets#an F-Stack arm runs on AWS x86_64 only" "LAB_PAIR=a LAB_PLACEMENT=cross-az LAB_NO_FLATOUT=1#cluster placement group" \
    "LAB_PAIR=a LAB_MODES=kernel#latency suite's" "LAB_PAIR=a LAB_RUN_STAMP=20261025-120000#claims a stamp of its own" \
    "LAB_PAIR=a LAB_ARMS_FILE=$fx/none.tsv#not a readable file" "LAB_PAIR=a LAB_ARMS_FILE=$fx/dpdk-netem.tsv#netem cannot reach" \
    "LAB_PAIR=a LAB_SETS_FILE=$fx/none.tsv#not a readable file" "LAB_PAIR=a LAB_EXTRA_SRC=$fx/nobuild#a directory with a lab-build.sh" \
    "LAB_PAIR=a TRANSPORT_POINT_S=x#TRANSPORT_POINT_S is a whole number" "LAB_PAIR=a LAB_SETS_FILE=$awsets LAB_MAX_HOURS=2#give LAB_MAX_HOURS=" \
    "LAB_PAIR=a LAB_SPOT_FALLBACK=1#LAB_SPOT_FALLBACK is 1 (with LAB_SPOT=1" "LAB_PAIR=a LAB_DETACH=2#LAB_DETACH is 0, 1 or unset" \
    "LAB_PAIR=a LAB_SEED=a.b#LAB_SEED is letters"; do
    rm -f "$tmp/cloud-calls"
    # shellcheck disable=SC2086
    out=$(awslab ${refused%#*} 2>&1) && why+=" ran with [${refused%#*}];"
    [[ $out == *"${refused#*#}"* ]] || why+=" [${refused%#*}]: $(tail -1 <<<"$out");"
    [[ ! -s $tmp/cloud-calls ]] || why+=" [${refused%#*}] called the cloud;"
done
out=$(LAB_SUITE= awslab LAB_SUITE=latency LAB_PAIR=a 2>&1) && why+=" latency with LAB_PAIR ran;"
[[ $out == *"transport suite's"* ]] || why+=" latency with LAB_PAIR: $(tail -1 <<<"$out");"
# the AWS DPDK session's F-Stack arm on Azure: refused there, by its setup
out=$(lab LAB_PAIR=a LAB_SETS_FILE="$awsets" 2>&1) && why+=" azure ran dpdk-tcp;"
[[ $out == *"an F-Stack arm runs on AWS x86_64 only"* ]] || why+=" azure dpdk-tcp: $(tail -1 <<<"$out");"
[[ ! -s $tmp/cloud-calls ]] || why+=" a refusal called the cloud;"
[[ -z $(ls -A "$tmp/labtmp") ]] || why+=" a snapshot left: $(ls "$tmp/labtmp");"
[[ -z $why ]] && pass "aws/lab.sh LAB_SUITE=transport refuses a bad pair, tuning, an F-Stack arm on Graviton, another placement, latency settings, a stamp, bad arms and sets files (NETEM on a DPDK arm too), extra tree, duration, deadline, spot fallback, detach and seed before any aws call; azure/lab.sh refuses the F-Stack arm" ||
    fail "aws/lab.sh transport refusals:$why"
rm -f "$tmp/cloud-calls"
est=$(awslab LAB_ESTIMATE=1 LAB_PAIR=a LAB_REPS=3 LAB_SETS_FILE="$awsets" 2>&1) && estrc=0 || estrc=$?
ests=$(awslab LAB_ESTIMATE=1 LAB_PAIR=a LAB_REPS=3 LAB_SETS_FILE="$awsets" LAB_SPOT=1 2>&1) && estsrc=0 || estsrc=$?
estm=$(awslab LAB_ESTIMATE=1 LAB_PAIR=b 2>&1) && estmrc=0 || estmrc=$?
if [[ $estrc == 0 && $est == *"pair a: 141 points (sets d1, d8, u, pin)"*"instance-hours, about \$"*" on demand"* &&
    $est == *"a c7a.2xlarge, b c7a.2xlarge, on-demand, 3 reps"* && $est == *"c7a.2xlarge \$0.41056/h"* && $est == *"detaches it"* &&
    $estsrc == 0 && $ests == *"at spot prices"*"c7a.2xlarge \$0.1907/h"*"describe-spot-price-history"* &&
    $estmrc == 0 && $estm == *"pair b: 420 points"* && ! -s $tmp/cloud-calls ]]; then
    pass "aws/lab.sh LAB_SUITE=transport LAB_ESTIMATE=1: the AWS DPDK session's points, hours, instance-hours and dollars on c7a.2xlarge (on demand, spot with LAB_SPOT=1), the matrix's pair b; no aws call"
else
    fail "aws/lab.sh transport estimate ($estrc, $estsrc, $estmrc): $est | $ests | $estm"
fi

# the SMT sessions on c7i.4xlarge (transport-aws-smt-off.tsv, then
# transport-aws-smt.tsv's `smt on`): their points and hours, no aws call;
# `smt on` refused with LAB_THREADS_PER_CORE=1, on Graviton and on C7a
rm -f "$tmp/cloud-calls"
why=
smtoff=$cloud/transport-aws-smt-off.tsv smton=$cloud/transport-aws-smt.tsv
out=$(awslab LAB_ESTIMATE=1 LAB_PAIR=a LAB_REPS=3 LAB_SPOT=1 LAB_SPOT_FALLBACK=1 LAB_TYPE=c7i.4xlarge LAB_SETS_FILE="$smtoff" 2>&1) || why+=" smt off estimate: $(tail -1 <<<"$out");"
[[ $out == *"pair a: 36 points (sets smtoff)"*"wall about "* && $out == *"a c7i.4xlarge, b c7i.4xlarge, spot, 3 reps"* ]] || why+=" smt off estimate: $out;"
out=$(awslab LAB_ESTIMATE=1 LAB_PAIR=a LAB_REPS=3 LAB_SPOT=1 LAB_SPOT_FALLBACK=1 LAB_TYPE=c7i.4xlarge LAB_SETS_FILE="$smton" 2>&1) || why+=" smt on estimate: $(tail -1 <<<"$out");"
[[ $out == *"pair a: 27 points (sets smton)"*"wall about "* ]] || why+=" smt on estimate: $out;"
for refused in "LAB_TYPE=c7i.4xlarge LAB_THREADS_PER_CORE=1#not with LAB_THREADS_PER_CORE=1" \
    "LAB_TYPE=c8g.2xlarge LAB_ALLOW_ARM=1#Graviton, which has no SMT" "LAB_TYPE=c7a.2xlarge#one thread a core"; do
    # shellcheck disable=SC2086
    out=$(awslab LAB_PAIR=a LAB_SETS_FILE="$smton" ${refused%#*} 2>&1) && why+=" smt on ran with [${refused%#*}];"
    [[ $out == *"${refused#*#}"* ]] || why+=" smt on [${refused%#*}]: $(tail -1 <<<"$out");"
done
[[ ! -s $tmp/cloud-calls ]] || why+=" an aws call;"
[[ -z $why ]] && pass "aws/lab.sh LAB_SUITE=transport: the SMT sessions' estimates on c7i.4xlarge (36 points SMT off, 27 SMT on); smt on refused with LAB_THREADS_PER_CORE=1, on Graviton and on C7a; no aws call" ||
    fail "aws/lab.sh SMT sessions:$why"

# the AWS Onload session (transport-aws-onload.tsv) on each type it runs on:
# Intel (c7i.4xlarge, SMT off), Graviton (c8g.2xlarge, LAB_ALLOW_ARM=1) and
# AMD (c7a.2xlarge), near 2 h at LAB_REPS=3 with OpenOnload's build; its
# smoke (transport-aws-onload-smoke.tsv) at LAB_REPS=1; no aws call
onsets=$cloud/transport-aws-onload.tsv
why=
for t in c7i.4xlarge c8g.2xlarge c7a.2xlarge; do
    allow=
    [[ $t == c8g.* ]] && allow=1
    e=$(awslab LAB_ESTIMATE=1 LAB_PAIR=a LAB_REPS=3 LAB_TYPE=$t ${allow:+LAB_ALLOW_ARM=1} LAB_SETS_FILE="$onsets" 2>&1) || why+=" $t: $(tail -1 <<<"$e");"
    [[ $e == *"pair a: 81 points (sets t, r)"*"wall about 1.94 h"* && $e == *"a $t, b $t"* ]] || why+=" $t: $(sed -n 2p <<<"$e");"
    printf '%s\n' "$e" >"$tmp/onload-estimate-$t.txt"
done
e=$(awslab LAB_ESTIMATE=1 LAB_PAIR=a LAB_REPS=1 LAB_SETS_FILE="$cloud/transport-aws-onload-smoke.tsv" 2>&1) || why+=" smoke: $(tail -1 <<<"$e");"
[[ $e == *"pair a: 20 points (sets ot, ou)"*"wall about 1.00 h"* ]] || why+=" smoke: $(sed -n 2p <<<"$e");"
[[ ! -s $tmp/cloud-calls ]] || why+=" an estimate called the cloud;"
[[ -z $why ]] && pass "aws/lab.sh LAB_SUITE=transport LAB_ESTIMATE=1: the Onload session on c7i.4xlarge, c8g.2xlarge (Graviton) and c7a.2xlarge, 81 points in about 1.94 h at 3 reps, and its smoke, 20 points in about 1 h; no aws call" ||
    fail "aws/lab.sh Onload estimates:$why"

awsbin=$tmp/awsbin
mkdir -p "$awsbin"
printf '#!/bin/sh\nexec python3 "%s" "$@"\n' "$here/fake_aws.py" >"$awsbin/aws"
printf '#!/bin/sh\nexit 0\n' >"$awsbin/ssh"
printf '#!/bin/sh\nexit 0\n' >"$awsbin/scp"
printf '#!/bin/sh\nexit 7\n' >"$awsbin/curl"
# a poll's pause, short (the address follower's 20 s ends its loop)
printf '#!/bin/sh\n[ "$1" = 20 ] && exit 1\nexec /bin/sleep 0.01\n' >"$awsbin/sleep"
chmod +x "$awsbin"/*
# awsfull <name> [VAR=value...]: aws/lab.sh LAB_SUITE=transport LAB_PAIR=a
# with the AWS DPDK sets against the fake, in $tmp/aws-<name>: raw (its
# calls), out, rc, state.json; killed after 300 s (a hang fails)
awsfull() {
    local d=$tmp/aws-$1 rc=0
    shift
    rm -rf "$d"
    mkdir -p "$d/home/.ssh" "$d/tmp" "$d/repo"
    echo 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA lab-test' >"$d/home/.ssh/id_ed25519.pub"
    : >"$d/raw"
    perl -e 'alarm shift; exec @ARGV' 300 \
        env PATH="$awsbin:$PATH" TMPDIR="$d/tmp" HOME="$d/home" LAB_REPO="$d/repo" LAB_SSH_FROM=192.0.2.10 \
        AWS_CALLS="$d/raw" FAKE_AWS_STATE="$d/state.json" LAB_SUITE=transport LAB_PAIR=a LAB_REPS=3 \
        LAB_SETS_FILE="$awsets" LAB_DETACH=0 "$@" bash "$cloud/aws/lab.sh" >"$d/out" 2>&1 || rc=$?
    echo "$rc" >"$d/rc"
}
# awsleft <state.json>: what the fake still holds (terminated, closed and
# cancelled are gone)
awsleft() {
    python3 -c 'import json, sys
s = json.load(open(sys.argv[1]))
print(" ".join(sorted(i for i, r in s["res"].items()
                      if not r.get("deleted") and r.get("state") not in ("terminated", "closed", "cancelled"))))' "$1"
}
# awsclean <name>: "" when the run's teardown deleted everything and said
# so, verified by tag; else why not
awsclean() {
    local d=$tmp/aws-$1 w=
    [[ -z $(awsleft "$d/state.json" 2>/dev/null) ]] || w+=" left $(awsleft "$d/state.json");"
    grep -q 'verified: nothing tagged run=.* remains in us-east-1' "$d/out" || w+=" not verified;"
    [[ -z $(ls -A "$d/tmp" | grep -v '^ergon-lab-aws.lock$') ]] || w+=" snapshot left: $(ls "$d/tmp");"
    echo "$w"
}

# a whole session against the fake: the ENI check before anything is made,
# c7a.2xlarge in a cluster group, every resource tagged owner, run and
# pair, the data ENIs at .4-.7, then (no checkout to pack) its teardown
awsfull run
d=$tmp/aws-run
why=$(awsclean run)
[[ $(cat "$d/rc") != 0 ]] || why+=" exit 0 with no checkout;"
grep -q 'cannot list the checkout' "$d/out" || why+=" not stopped at the checkout: $(tail -2 "$d/out");"
first_eni=$(grep -n 'MaximumNetworkInterfaces' "$d/raw" | head -1 | cut -d: -f1)
first_vpc=$(grep -n ' create-vpc ' "$d/raw" | head -1 | cut -d: -f1)
[[ -n $first_eni && -n $first_vpc && $first_eni -lt $first_vpc ]] || why+=" the ENI check not before the VPC ($first_eni, $first_vpc);"
creates=$(grep -- '--tag-specifications' "$d/raw" || true)
[[ $(grep -c . <<<"$creates") -ge 10 ]] || why+=" $(grep -c . <<<"$creates") tagged creates;"
stamp_=$(sed -n 's/.*{Key=run,Value=\([0-9-]*\)}.*/\1/p' <<<"$creates" | sort -u)
[[ $(grep -c . <<<"$stamp_") == 1 && -d $d/repo/target/md-dpdk-bench/aws-$stamp_ ]] || why+=" run tags [$stamp_] not the run's directory;"
while IFS= read -r line; do
    for spec in $(grep -o 'ResourceType=[^ ]*' <<<"$line"); do
        [[ $spec == *"{Key=owner,Value=ergon-lab},{Key=run,Value=$stamp_},{Key=pair,Value=a},"* ]] || why+=" untagged: $spec;"
    done
done <<<"$creates"
grep 'run-instances' "$d/raw" | grep -q -- '--instance-type c7a.2xlarge' || why+=" not c7a.2xlarge;"
! grep 'run-instances' "$d/raw" | grep -qv -- '--placement GroupName=ergon-md-dpdk-' || why+=" an instance outside the cluster group;"
[[ $(grep -c 'create-network-interface' "$d/raw") == 4 ]] || why+=" $(grep -c 'create-network-interface' "$d/raw") data ENIs;"
for ip in 10.30.1.4 10.30.1.5 10.30.1.6 10.30.1.7; do grep -q -- "--private-ip-address $ip " "$d/raw" || why+=" no ENI at $ip;"; done
grep -q 'ResourceType=volume' <<<"$creates" || why+=" volumes untagged;"
[[ $(cat "$d/repo/target/md-dpdk-bench/aws-$stamp_/pair" 2>/dev/null) == a && $(cat "$d/repo/target/md-dpdk-bench/aws-$stamp_/suite" 2>/dev/null) == transport ]] ||
    why+=" its directory does not say transport pair a;"
grep -q 'transport pair a seed' "$d/repo/target/md-dpdk-bench/aws-$stamp_/instances.txt" || why+=" instances.txt without the pair;"
[[ -z $why ]] && pass "aws/lab.sh LAB_SUITE=transport end to end: ENIs checked first, two c7a.2xlarge in one cluster group, every resource tagged owner, run and pair, four data ENIs, then deleted and verified by tag, the snapshot removed" ||
    fail "aws/lab.sh transport end to end:$why $(tail -3 "$d/out")"

# refusals that need AWS's answer: a type taking fewer than three ENIs
awsfull enis FAKE_MAX_ENIS=2
d=$tmp/aws-enis
why=
[[ $(cat "$d/rc") != 0 ]] || why+=" exit 0;"
grep -q 'takes 2 ENIs and the lab needs 3' "$d/out" || why+=" not said: $(tail -1 "$d/out");"
! grep -qE ' create-| run-instances | import-key-pair ' "$d/raw" || why+=" something created;"
[[ -z $(ls -A "$d/repo/target/md-dpdk-bench" 2>/dev/null) ]] || why+=" a run directory claimed;"
[[ -z $why ]] && pass "aws/lab.sh LAB_SUITE=transport: a type taking fewer than three ENIs refused before anything is created" ||
    fail "aws/lab.sh transport ENI check:$why"

# an operator's TERM mid-run (once the first data ENI is attached): the trap's
# teardown deletes and verifies everything, the run ends 130
awsfull term FAKE_TERM_AT=attach-network-interface
d=$tmp/aws-term
why=$(awsclean term)
[[ $(cat "$d/rc") == 130 ]] || why+=" exit $(cat "$d/rc");"
grep -q 'cannot list the checkout' "$d/out" && why+=" ran on past the TERM;"
[[ -z $why ]] && pass "aws/lab.sh LAB_SUITE=transport: a TERM mid-run ends it through its teardown (exit 130), everything deleted and verified" ||
    fail "aws/lab.sh transport TERM:$why $(tail -3 "$d/out")"

# spot: no zone with spot capacity: refused as before without the
# fallback; with LAB_SPOT_FALLBACK=1 both launch on demand, said so
awsfull nospot LAB_SPOT=1 FAKE_NO_SPOT_CAPACITY="us-east-1a us-east-1b"
d=$tmp/aws-nospot
why=$(awsclean nospot)
grep -q 'no zone had capacity for c7a.2xlarge and c7a.2xlarge' "$d/out" || why+=" not said: $(tail -1 "$d/out");"
! grep 'run-instances' "$d/raw" | grep -vq -- '--instance-market-options' || why+=" an on-demand launch;"
awsfull fallback LAB_SPOT=1 LAB_SPOT_FALLBACK=1 FAKE_NO_SPOT_CAPACITY="us-east-1a us-east-1b"
d=$tmp/aws-fallback
why+=$(awsclean fallback)
grep -q 'LAB_SPOT_FALLBACK=1, launching them on demand' "$d/out" || why+=" fallback not said;"
[[ $(grep 'run-instances' "$d/raw" | grep -vc -- '--instance-market-options') == 2 ]] || why+=" not two on-demand launches;"
grep -q '^market on-demand$' "$d"/repo/target/md-dpdk-bench/aws-*/instances.txt || why+=" instances.txt not on-demand;"
grep -q 'describe-spot-instance-requests' "$d/raw" || why+=" the spot requests not swept;"
[[ -z $why ]] && pass "aws/lab.sh LAB_SPOT=1: no spot capacity in any zone ends the run (deleted, verified); LAB_SPOT_FALLBACK=1 launches both on demand, said so and recorded, spot requests still swept" ||
    fail "aws/lab.sh transport spot fallback:$why"

# detached (LAB_DETACH=1): the parent claims the directory, says so and
# exits 0 with nothing created; the detached run in that directory creates,
# fails at the checkout and tears down, verified, its snapshot removed
awsfull detach LAB_DETACH=1
d=$tmp/aws-detach
why=
[[ $(cat "$d/rc") == 0 ]] || why+=" parent exit $(cat "$d/rc");"
grep -q 'detached: about' "$d/out" || why+=" not said: $(tail -1 "$d/out");"
ddir=$(ls -d "$d"/repo/target/md-dpdk-bench/aws-* 2>/dev/null | head -1)
dpid=$(cat "$ddir/lab.pid" 2>/dev/null)
for _ in $(seq 600); do kill -0 "$dpid" 2>/dev/null || break; /bin/sleep 0.1; done
kill -0 "$dpid" 2>/dev/null && { why+=" still running after 60 s;"; kill -TERM "$dpid"; }
cp "$ddir/lab.log" "$d/out" 2>/dev/null || why+=" no lab.log;"
why+=$(awsclean detach)
grep -q 'cannot list the checkout' "$d/out" || why+=" the detached run did not run on: $(tail -1 "$d/out");"
[[ $(grep -c "{Key=run,Value=${ddir##*/aws-}}" "$d/raw") -ge 10 ]] || why+=" not the claimed directory's stamp;"
[[ $(ls -d "$d"/repo/target/md-dpdk-bench/aws-* | grep -c .) == 1 ]] || why+=" a second directory claimed;"
[[ -z $why ]] && pass "aws/lab.sh LAB_DETACH=1: the parent claims the run's directory and exits; the detached run, in it and with its stamp, deletes and verifies everything, its snapshot removed" ||
    fail "aws/lab.sh transport detach:$why"

# ---- transport.sh run on fake hosts ----
# VM a and VM b are homes in the sandbox; the fake ssh runs a command in the
# home of the host it names; vm.sh is a fake logging every call; the bench
# binaries one stub (pong waits for TERM; ping and local write a row, a
# manifest and the histogram)
sb=$tmp/hosts
mkdir -p "$sb/bin"
cat >"$sb/bin/ssh" <<'SH'
#!/usr/bin/env bash
for a in "$@"; do case $a in *@10.30.0.4) h=a ;; *@10.30.0.6) h=b ;; esac; done
cmd=${*: -1}
HOME=$FAKE_HOSTS/$h FAKE_HOST=$h exec bash -c "cd; $cmd"
SH
cat >"$sb/bin/fakevm" <<'SH'
#!/usr/bin/env bash
echo "$FAKE_HOST vm.sh $*" >>"$FAKE_HOSTS/calls"
# the Aeron directory each per-arm call names (LAB_AERON_DIR)
case $1 in driver | stop | events-start | aeron-page) echo "$FAKE_HOST $1-dir ${LAB_AERON_DIR:-unset}" >>"$FAKE_HOSTS/calls" ;; esac
st=$HOME/state
mkdir -p "$st"
case $1 in
transport-map) echo "${FAKE_MAP:-0 2 4,6,8,10,12,14}" ;;
nics) if [[ $FAKE_HOST == a ]]; then printf 'sender eth1 enP1 m 10.20.1.4 24 x\nreceiver eth2 enP2 m 10.20.1.5 24 y\n'
      else printf 'sender eth1 enP1 m 10.20.1.6 24 x\nreceiver eth2 enP2 m 10.20.1.7 24 y\n'; fi ;;
hostcheck) if [[ ${FAKE_HOSTCHECK_WARN:-} == "$FAKE_HOST" ]]; then echo "T3 warn: cpu 6 has its tick (evidence: LOC 1000/s)"; exit 1; fi
    echo "T3 ok: every hot CPU tickless"; echo "T4 ok: the Aeron directory ${2:-/dev/shm/aeron-bench} is on ${2:+hugetlbfs}${2:-tmpfs}" ;;
# the page size of the Aeron directory: 2 MiB on the hugetlbfs, else 4 KiB
# (FAKE_AERON_PAGE_BAD: the hugetlbfs one reads back 4 KiB)
aeron-page) if [[ ${LAB_AERON_DIR:-} == /mnt/huge-aeron/* && -z ${FAKE_AERON_PAGE_BAD:-} ]]; then echo 2097152; else echo 4096; fi ;;
netem)
    case $2 in
    set) echo "$3" >"$st/netem" ;;
    clear) rm -f "$st/netem" ;;
    check) [[ $3 == none ]] && [[ ! -e $st/netem ]] && exit 0; [[ -e $st/netem && $(cat "$st/netem") == "$3" ]] ;;
    stats)
        [[ -e $st/netem ]] || exit 0
        n=$(cat "$st/n" 2>/dev/null || echo 0); n=$((n + 1)); echo $n >"$st/n"
        p=$(cat "$st/netem")
        # 1,000,000 packets offered a call, the loss's share of them dropped
        d=$(awk -v p="$p" -v n="$n" -v bad="${FAKE_NETEM_BAD:-}" 'BEGIN {print bad ? 0 : int(n * 10000 * p + 0.5)}')
        echo "eth1 $((n * 1000000 - d)) $d" ;;
    show) if [[ -e $st/netem ]]; then echo "eth1 netem"; else echo "eth1 mq"; fi ;;
    esac ;;
irqsnap) n=$(cat "$st/i" 2>/dev/null || echo 0); n=$((n + 1)); echo $n >"$st/i"
    printf '           CPU0       CPU2       CPU4       CPU6\nLOC:  %d  %d  %d  %d  Local timer interrupts\n' $n $n $n $n ;;
# the ping on FAKE_PING_CPU (4); a uring-sqpoll point's SQPOLL thread on
# the CPU FAKE_SQPOLL ("<arm part>:<cpu> ...") gives the point's arm (the
# point events-start last named on this host)
sample) if [[ -n ${FAKE_PLACEMENT_BAD:-} ]]; then echo "FAIL ping(9) on CPU 0, not isolated;"; else echo "ok 2 busy threads"; fi
    echo "  101 ${FAKE_PING_CPU:-4} 99.5 ping"; echo "    1   0  0.1 sshd"
    for e in ${FAKE_SQPOLL:-}; do
        [[ $(tail -1 "$st/events-started" 2>/dev/null) == *"-${e%%:*}"* ]] && echo "  700 ${e#*:} 100.0 iou-sqp-650"
    done ;;
# every transport sysctl at its declared value, the optional ones absent
# and the SMT state (vm.sh transport_smt_read): FAKE_SMT_ACTIVE, else 0
settings) (. "$FAKE_CLOUD/transport-settings.env"; . "$FAKE_CLOUD/host.sh"; transport_sysctls) |
    awk '{k = $1; gsub("/", ".", k); v = $2; for (i = 3; i < NF; i++) v = v " " $i; print k "=" ($NF == "optional" ? "absent" : v)}'
    echo "devices.system.cpu.smt.active=${FAKE_SMT_ACTIVE:-0}" ;;
nstat) n=$(cat "$st/s" 2>/dev/null || echo 0); n=$((n + 1)); echo $n >"$st/s"
    printf 'Tcp: RetransSegs InSegs\nTcp: %d %d\nUdp: InErrors\nUdp: 0\n' "$n" "$((n * 100))" ;;
# the data NICs' interrupt deferral (napi_defer_hard_irqs, gro_flush_timeout),
# both NICs alike: kernel-busy's driver sets 2/200000, kernel's 0/0;
# transport-net the settings' TR_NAPI_DEFER/TR_GRO_FLUSH_NS under the arm's
# TR_OVERRIDES (logged); napi reads them back as vm.sh napi prints them
driver) if [[ -n ${FAKE_DRIVER_FAIL:-} && $2 == "$FAKE_DRIVER_FAIL"* ]]; then
        # as vm.sh's ERR trap says a step that failed under set -e; the
        # driver's log the last one's
        echo "fake driver: EAL: Error - a stale log" >"$HOME/bench/driver.log"
        echo 'vm.sh: line 2096: "return $rc" failed (exit 1): stopping' >&2
        exit 1
    fi
    # the driver's environment (vm.sh's ~/bench/driver.env), which an
    # embedded driver's binary sources: here its mode alone
    echo "export FAKE_DRIVER_ENV='$2'" >"$HOME/bench/driver.env"
    # md-dpdk's stage trace and capture, as vm.sh points them at ~/bench:
    # the driver's files there (a header, a pcapng's first bytes), none else
    rm -f "$HOME/bench/stages.csv" "$HOME/bench/capture.pcapng"
    [[ $2 != *@RUSTERON_DPDK_STAGE_TRACE=* ]] || printf '# ergon md-dpdk stage trace v1\nstage,tsc,stream_id,seq,term_offset,value,ring\n' >"$HOME/bench/stages.csv"
    [[ $2 != *@RUSTERON_DPDK_CAPTURE_WINDOW_MS=* ]] || printf '\n\r\r\n' >"$HOME/bench/capture.pcapng"
    # the data NICs DPDK's (vm.sh nicstats then says so) until stop
    case $2 in dpdk*) touch "$st/dpdk" ;; *) rm -f "$st/dpdk" ;; esac
    case $2 in *@NAPI_DEFER=*) v=${2#*@NAPI_DEFER=}; v=${v%%@*}; echo "${v%/*} ${v#*/}" >"$st/napi" ;; kernel-busy*) echo "2 200000" >"$st/napi" ;; kernel | kernel@*) echo "0 0" >"$st/napi" ;;
    # Onload's driver: onload-up first, the NICs' deferral off
    onload@*) echo "0 0" >"$st/napi" ;; esac ;;
# setup onload: the NICs under Onload, their deferral off; the build each
# host has; an Onload stack of the arm's process mid-window (FAKE_ONLOAD_NOSTACK:
# none on that host, as an unaccelerated socket would leave)
onload-up) echo "0 0" >"$st/napi" ;;
onload-idle) ;;
onload-info) echo "onload a94512556626 patch 0123456789ab kernel 7.2.9+deb13-cloud-amd64 10.0.0 (fake)" ;;
onload-stacks) if [[ ${FAKE_ONLOAD_NOSTACK:-} == "$FAKE_HOST" ]]; then echo "FAIL no Onload stack of $2 (stacks: none)"; exit 1; fi
    echo "ok 1 stack(s) of $2: 0:4242" ;;
transport-net)
    if [[ ${2:-} != check ]]; then
        v=$(. "$FAKE_CLOUD/transport-settings.env"; . "$FAKE_CLOUD/host.sh"; transport_overrides; echo "$TR_NAPI_DEFER $TR_GRO_FLUSH_NS")
        echo "$v" >"$st/napi"
        echo "$FAKE_HOST transport-net napi $v" >>"$FAKE_HOSTS/calls"
        # the IRQ core's idle cap (TR_IRQ_IDLE), logged; FAKE_IDLE_STUCK: the
        # host keeps every state enabled whatever it is asked
        v=$(. "$FAKE_CLOUD/transport-settings.env"; . "$FAKE_CLOUD/host.sh"; transport_overrides; echo "$TR_IRQ_IDLE")
        [[ -z ${FAKE_IDLE_STUCK:-} ]] && echo "$v" >"$st/idle" || echo all >"$st/idle"
        echo "$FAKE_HOST transport-net irq_idle $v" >>"$FAKE_HOSTS/calls"
    fi ;;
napi) read -r n ns <"$st/napi" 2>/dev/null || { n=2 ns=200000; }; printf 'enP1 %s %s\nenP2 %s %s\n' "$n" "$ns" "$n" "$ns" ;;
stop) rm -f "$st/dpdk" ;;
events-stop-all) ;;
# the housekeeping and IRQ CPUs' idle states as host.sh transport_idle_read
# prints them (four states each, intel_idle's names), the IRQ CPU's past
# the cap transport-net set disabled; each call's usage one more
idle) read -r hk irq _ <<<"${FAKE_MAP:-0 2 4,6,8,10,12,14}"
    c=$(cat "$st/idle" 2>/dev/null || echo all); u=$(cat "$st/u" 2>/dev/null || echo 0); u=$((u + 1)); echo $u >"$st/u"
    echo "driver intel_idle governor menu max_cstate 9"
    for i in 0 1 2 3; do
        n=(POLL C1 C1E C6)
        echo "cpu$hk hk $i ${n[i]} 0 $((i * 100)) $((i * 300)) $u $((u * 10))"
        dis=0; [[ $c != all ]] && ((i > c)) && dis=1
        echo "cpu$irq irq $i ${n[i]} $dis $((i * 100)) $((i * 300)) $u $((u * 10))"
    done ;;
# each data NIC's counters as vm.sh nicstats prints them: its traffic, 1 MB
# more each way a call (a point's growth 2 MB a host), its moderation and
# driver counters (the queue's interrupts unmasked, 7 more a call); or dpdk
# alone while DPDK holds them
nicstats) n=$(cat "$st/nic" 2>/dev/null || echo 0); n=$((n + 1)); echo $n >"$st/nic"
    if [[ -e $st/dpdk ]]; then echo "sender eth1 dpdk"; echo "receiver eth2 dpdk"; else
        for r in "sender eth1" "receiver eth2"; do
            echo "$r tx_bytes=$((n * 1000000)) rx_bytes=$((n * 1000000)) tx_packets=$((n * 1000)) rx_packets=$((n * 1000)) bw_out_allowance_exceeded=0 pps_allowance_exceeded=0"
            echo "$r coalesce off n/a 0 0"
            echo "$r stat queue_0_tx_unmask_interrupt $((n * 7))"
            echo "$r stat pps_allowance_exceeded 0"
        done
    fi ;;
cc-available) echo "reno cubic" ;;
# a point's events: begun (logged), then read back as host.sh events_stop
# prints them: the trace's clock the wall clock's, and on the host that
# measured (its newest spikes file) an IRQ of 100 us on hot CPU 4, where
# the ping runs, at the max (the stub's: due at the run's start)
events-start) echo "$2 $3 $4 ${5:-}" >>"$st/events-started" ;;
events-stop)
    echo "$2" >>"$st/events-stopped"
    f=$(ls -t "$HOME"/bench/transport-*/run-*/out*/spikes-*.csv 2>/dev/null | head -1)
    s=$(sed -n 's/^# start_realtime_ns=//p' "$f" 2>/dev/null)
    printf '== clock\n1000000000 1000000000\n2000000000 2000000000\n== notes\nevents: trace on cpus 4,6,8,10,12,14 (fake)\nevents: osnoise unavailable: the kernel has no osnoise tracer (fake)\n== trace\n'
    if [[ -n $s ]]; then
        sec=$((s / 1000000000)) us=$((s % 1000000000 / 1000))
        printf '   ping-101 [004] d..2. %d.%06d: sched_switch: prev_comm=swapper/4 prev_pid=0 prev_prio=120 prev_state=R ==> next_comm=ping next_pid=101 next_prio=120\n' $((sec - 1)) 0
        printf '   ping-101 [004] d.h1. %d.%06d: irq_handler_entry: irq=24 name=fake_nic\n' "$sec" "$us"
        printf '   ping-101 [004] d.h1. %d.%06d: irq_handler_exit: irq=24 ret=handled\n' "$sec" $((us + 100))
        printf '   ping-101 [004] d..2. %d.%06d: sched_switch: prev_comm=ping prev_pid=101 prev_prio=120 prev_state=S ==> next_comm=swapper/4 next_pid=0 next_prio=120\n' $((sec + 1)) 0
    fi
    printf '== samples\nt=1000000000 tc=- tcp=RetransSegs=0 aeron=-\n' ;;
*) echo "fake vm.sh: $*" >&2; exit 2 ;;
esac
SH
cat >"$sb/bin/stub" <<'SH'
#!/usr/bin/env bash
# a bench binary as transport-cli.md has it
me=${0##*/}
# (with the driver's environment, when it was sourced: an embedded driver's binary)
echo "$FAKE_HOST $me $*${FAKE_DRIVER_ENV:+ [driver.env $FAKE_DRIVER_ENV]}" >>"$FAKE_HOSTS/runs"
role= out= label= rate= kind=oneway streams=1 warm=0 dur=0 mode= impl= stage= load= hot= dbusy= wire= busy=
while (($#)); do
    case $1 in --role) role=$2 ;; --out) out=$2 ;; --label) label=$2 ;; --rate) rate=$2 ;; --round-trip) kind=load ;;
    --streams) streams=$2 ;; --warmup-s) warm=$2 ;; --duration-s) dur=$2 ;; --mode) mode=$2 ;; --impl) impl=$2 ;;
    --stage-trace) stage=$2 ;; --busy-read-us) busy=$2 ;;
    --load) load=$2 ;; --hot-rate) hot=$2 ;; --driver-busy-threads) dbusy=$2 ;; --wire) wire=$2 ;;
    # the runner's own (the DPDK driver's capture window): never the binary's
    --capture-ms) echo "stub: --capture-ms is the runner's, not aeron_lat's" >&2; exit 2 ;; esac
    shift
done
# aeron_lat --stage-trace: its stages file (a header alone)
stages() { [[ -z $stage ]] || { mkdir -p "$out"; printf '# ergon aeron_lat stage trace v1\n# role=%s every=%s streams=%s stream_id=1001 echo_offset=10000 ticks_per_ns=2.0 measure_from_tsc=0\nstage,tsc,stream_id,seq,term_offset,value,ring\n' "$role" "$stage" "$streams" >"$out/stages-app.csv"; }; }
# the manifest's settings as the binaries write them: the transport, the
# schedule, the counters each host's loss accounting reads
case $me in
aeron_lat) transport=aeron-$mode extra=', "aeron.naks-sent": "0", "aeron.retransmits-sent": "0"'
    # labelled by the wire it was given (aeron_lat --wire, checked against its driver)
    [[ $wire != dpdk ]] || transport=${transport/%-udp/-dpdk} ;;
floor) transport=raw-dpdk extra= ;;
tcp_lat) transport=tcp extra=', "tcp.retrans": "0", "tcp.dsack_dups": "0", "tcp.retrans_not_dsack": "0", "tcp.lost_end": "0"' ;;
mpsc_lat) transport=mpsc extra= ;;
*) transport=udp extra= ;;
esac
[[ $me == udp_lat || $me == tcp_lat ]] && extra+=", \"busy_read_us\": \"${busy:-50}\""
settings="\"transport\": \"$transport\", \"streams\": \"$streams\", \"warmup_s\": \"$warm\", \"duration_s\": \"$dur\", \"scheduled\": \"0\", \"late\": \"0\", \"max_send_lag_ns\": \"0\", \"retrans\": \"0\"$extra"
[[ $role == oneway-send || $role == oneway-recv ]] && { echo "stub: --role $role is refused (one way is --role local)" >&2; exit 2; }
# --load mixed: both ends' counts as mixed.rs writes them, every message
# accounted for (each background stream's sent, read by each subscription,
# echoed and back), the drivers' bytes those its frames give (about 1.5 MB:
# inside the fake NICs' 2 MB a host), the DPDK transport's own counters
if [[ $load == mixed ]]; then
    n=$((streams - 1)) per=$((1500000 / ((streams - 1) * 320)))
    classes= pubs= fan= sent= zeros= recv= period=
    for ((i = 1; i <= n; i++)); do
        c=$(((i - 1) % 8)) f=$((1 + i % 4))
        classes+=$(echo SSSSBBIM | cut -c$((c + 1)))
        pubs+=,$(((c == 7) + 1)) fan+=,$f sent+=,$per zeros+=,0 period+=,1000000
        r=$per
        for ((k = 1; k < f; k++)); do r+=/$per; done
        recv+=,$r
    done
    fb=$((n * per * 320))
    settings+=", \"load\": \"mixed\", \"mixed.profile\": \"mixed-v1\", \"mixed.classes\": \"$classes\", \"mixed.publishers\": \"${pubs#,}\""
    settings+=", \"mixed.fanout\": \"${fan#,}\", \"mixed.period_ns\": \"${period#,}\", \"hot_rate\": \"$hot\", \"busy_threads\": \"$((2 + dbusy))\""
    settings+=", \"driver.bytes-sent\": \"$((fb + 96))\", \"driver.bytes-received\": \"$((fb + 96))\", \"mixed.frame_bytes_sent\": \"$fb\", \"mixed.frames_sent\": \"$((n * per))\""
    settings+=", \"nic.tx_bytes\": \"$((fb + 100000))\", \"nic.rx_bytes\": \"$((fb + 100000))\", \"nic.rx_missed\": \"0\", \"nic.allowance_exceeded\": \"0\""
    if [[ $role == pong ]]; then
        settings+=", \"mixed.recv\": \"${recv#,}\", \"mixed.echoed\": \"${sent#,}\", \"mixed.recv_gaps\": \"${zeros#,}\", \"mixed.recv_behind\": \"${zeros#,}\""
        settings+=", \"mixed.recv_refused\": \"${zeros#,}\", \"mixed.hot_echoes\": \"1\", \"mixed.frame_bytes_received\": \"$fb\""
        [[ -n ${FAKE_FANOUT_SHORT:-} && $label == *"$FAKE_FANOUT_SHORT"* ]] && settings=${settings/\"mixed.recv\": \"$per\/$per,/\"mixed.recv\": \"$per\/$((per - 1)),}
    else
        settings+=", \"mixed.scheduled\": \"${sent#,}\", \"mixed.sent\": \"${sent#,}\", \"mixed.echoes\": \"${sent#,}\", \"mixed.echo_gaps\": \"${zeros#,}\""
        settings+=", \"mixed.echo_behind\": \"${zeros#,}\", \"mixed.bp_measured\": \"${zeros#,}\", \"mixed.hot_sent\": \"1\", \"mixed.hot_frame_bytes\": \"96\""
        settings+=", \"mixed.echo_frame_bytes_received\": \"$fb\", \"all.p50_ns\": \"2\", \"all.p99_ns\": \"3\", \"all.p9999_ns\": \"5\", \"all.max_ns\": \"150000\""
    fi
fi
[[ $role == ping ]] && kind=load
# --mode spy-udp: the run's frames (100 pings, a round trip's 100 echoes) and
# its driver's bytes; its sink (--role pong) the frames it read (90 pings
# for a label FAKE_SINK_SHORT names)
if [[ $mode == spy-udp ]]; then
    if [[ $role == pong ]]; then
        pings=100
        [[ -n ${FAKE_SINK_SHORT:-} && $label == *"$FAKE_SINK_SHORT"* ]] && pings=90
        settings+=", \"sink.ping_frames\": \"$pings\", \"sink.echo_frames\": \"100\", \"sink.gaps\": \"0\", \"sink.behind\": \"0\", \"sink.refused\": \"0\""
    else
        settings+=', "sent": "100", "driver.bytes-sent": "19200"'
        [[ $kind == load ]] && settings+=', "echoes_sent": "100"'
    fi
fi
if [[ $role == pong ]]; then
    # one that exits before ready, as fstack_lat did on AWS as a plain user:
    # F-Stack's config echo on stdout, the EAL's error on stderr
    if [[ -n ${FAKE_PONG_EXIT:-} && $label == *"$FAKE_PONG_EXIT"* ]]; then
        echo "[dpdk]: lcore_mask=4"
        echo "EAL: Cannot use IOVA as 'PA' since physical addresses are not available" >&2
        echo "EAL: Error - exiting with code: 1" >&2
        echo "[freebsd.sysctl]: net.inet.tcp.delayed_ack=0"
        exit 1
    fi
    # ready once "bound", unless told to stay silent; a manifest when stopped
    [[ -n ${FAKE_PONG_SILENT:-} && $label == *"$FAKE_PONG_SILENT"* ]] || echo ready
    trap 'mkdir -p "$out"; stages; echo "{\"settings\": {$settings}}" >"$out/manifest.json"; exit 0' TERM
    while :; do sleep 0.05; done
fi
[[ -n ${FAKE_FAIL_LABEL:-} && $label == *"$FAKE_FAIL_LABEL"* ]] && { echo "stub: failing $label" >&2; exit 1; }
mkdir -p "$out"
stages
# the row as the binaries write it: kind first, the channel, no rep
# the max 150 us, due at second :50 of a minute (the per-minute cluster
# explains it: spikes.py), every other latency 1 to 5 ns
now=$(date +%s)
start=$(((now - now % 60 + 50) * 1000000000))
# p99.99 5 ns, or 2 for a label FAKE_BETTER names (the tuning test's better value)
p9999=5
[[ -n ${FAKE_BETTER:-} && $label == *"$FAKE_BETTER"* ]] && p9999=2
echo "$kind,$label,udp,64,$rate,1,1,2,3,4,$p9999,150000,1,1,1,1,0,$rate,0,0,0,0,1000,0,0,0,0,64,2,1,2,3,150000" >"$out/results.csv"
echo "{\"payload_sha256\": \"00\", \"payload_size\": 64, \"wire_bytes\": 64, \"core_build_hash\": \"stub\", \"settings\": {$settings}}" >"$out/manifest.json"
echo "1000,1" >"$out/histogram-$label.csv"
printf '# start_realtime_ns=%s\n# spikes_total=1\n# max=0,150000,0\nstream,due_ns,latency_ns\n0,0,150000\n' "$start" >"$out/spikes-$label.csv"
SH
# sh, slow to start a pong when FAKE_SH_DELAY says so: its PID file not yet
# written when the runner first looks
cat >"$sb/bin/sh" <<'SH'
#!/bin/bash
case "$*" in *pong.pid*) [[ -n ${FAKE_SH_DELAY:-} ]] && /bin/sleep "$FAKE_SH_DELAY" ;; esac
exec /bin/sh "$@"
SH
# the onload launcher: logged with the EF_ settings it was given, then the command
cat >"$sb/bin/onload" <<'SH'
#!/usr/bin/env bash
echo "$FAKE_HOST onload $* | $(env | grep '^EF_' | sort | tr '\n' ' ')" >>"$FAKE_HOSTS/onload"
while [[ ${1:-} == --* ]]; do shift; done
exec "$@"
SH
# sudo -n setpriv <options> <command>: logged, then the command as this
# user (an embedded driver's binary, as fstack_lat, runs under it)
cat >"$sb/bin/sudo" <<'SH'
#!/usr/bin/env bash
echo "$FAKE_HOST sudo $*" >>"$FAKE_HOSTS/sudo"
[[ ${1:-} == -n ]] && shift
if [[ ${1:-} == setpriv ]]; then
    shift
    while [[ ${1:-} == --* ]]; do shift; done
fi
exec "$@"
SH
cat >"$sb/bin/taskset" <<'SH'
#!/usr/bin/env bash
# taskset -c <cpu> <command>...: logged, then the command
echo "$FAKE_HOST taskset $2" >>"$FAKE_HOSTS/taskset"
shift 2
exec "$@"
SH
chmod +x "$sb/bin/"*
mkhost() { # <host>
    local h=$sb/$1
    mkdir -p "$h/ergon/md-dpdk/benches/cloud" "$h/ergon/target/release" "$h/bench" "$h/extra"
    ln -sf "$sb/bin/fakevm" "$h/ergon/md-dpdk/benches/cloud/vm.sh"
    for b in aeron_lat mpsc_lat udp_lat tcp_lat floor; do ln -sf "$sb/bin/stub" "$h/ergon/target/release/$b"; done
}
trun() { # <pair> <reps> [<arms file>] [vars...]: transport.sh run on host a
    local pair=$1 reps=$2 af=${3:-}
    shift 3 || shift $#
    rm -rf "$sb/a" "$sb/b" "$sb/calls" "$sb/runs" "$sb/taskset" "$sb/onload" "$sb/sudo"
    mkhost a
    mkhost b
    mkdir -p "$sb/a/bench/transport-$pair"
    cat >"$sb/a/bench/transport-$pair/env" <<ENV
A=10.30.0.4
B=10.30.0.6
user=azureuser
pair=$pair
session=20261025-120000
seed=11
reps=$reps
vm_a=g/a
vm_b=g/b
commit=abc
dirty=0
arms_file=$af
sets_file=${TRUN_SETS:-}
TRANSPORT_WARMUP_S=0
TRANSPORT_POINT_S=0
TRANSPORT_SWITCH_S=0
ENV
    env PATH="$sb/bin:$PATH" HOME="$sb/a" FAKE_HOSTS="$sb" FAKE_HOST=a FAKE_CLOUD="$cloud" TRANSPORT_READY_TIMEOUT_S=3 "$@" \
        bash "$cloud/transport.sh" run "$pair" >"$sb/run.out" 2>&1
}
# a bystander of the same name (another run's aeron_lat on this machine):
# the runner stops only the PIDs it started, never by name
mkdir -p "$tmp/bystander"
ln -s /bin/sleep "$tmp/bystander/aeron_lat"
# (it outlives the sessions run before it is checked, however slow the host)
"$tmp/bystander/aeron_lat" 3600 &
bystander=$!
trun b 1 "" && rrc=0 || rrc=$?
d=$sb/a/bench/transport-b
why=
[[ $rrc == 0 && $(cat "$d/rc") == 0 ]] || why+=" exit $rrc rc $(cat "$d/rc" 2>/dev/null);"
n=$(plan count b 1 "$matrix")
# a one-host arm's point is two runs (one way, then the round trip)
one=$(awk -F'\t' '$3 != "ab" {split($7, l, ","); split($8, st, ","); split($9, a, ","); n += length(l) * length(st) * length(a)} END {print n}' "$d/rounds.tsv")
[[ $(grep -c . "$d/results.csv") == $((n + one)) ]] || why+=" $(grep -c . "$d/results.csv") rows, not $n + $one;"
[[ $(grep -c ',oneway,' "$d/results.csv") == "$one" ]] || why+=" oneway rows;"
[[ $(find "$d/manifest" -name '*.env' | wc -l | tr -d ' ') == "$n" ]] || why+=" manifests;"
[[ $(find "$d/manifest" -name '*.json' | wc -l | tr -d ' ') == $((n + one)) ]] || why+=" binaries' manifests;"
! cut -d, -f1 "$d/results.csv" | grep -qv '^1$' || why+=" rep not in the rows' first column;"
! cut -d, -f2 "$d/results.csv" | grep -qvE '^(load|oneway)$' || why+=" the kind not in the second column;"
! cut -d, -f4 "$d/results.csv" | grep -qvE '^udp-r[0-9]+$' || why+=" the point column not <channel>-r<rate>;"
[[ $(awk -F, '{print NF}' "$d/results.csv" | sort -u) == 34 ]] || why+=" rows not 34 columns;"
# laid out for the analysis: a manifest and a spikes file a row, the declared settings
c=$(find "$d" -mindepth 2 -maxdepth 2 -name 'manifest-*.json' | wc -l | tr -d ' ')
c+=/$(find "$d" -mindepth 2 -maxdepth 2 -name 'spikes-*.csv' | wc -l | tr -d ' ')
[[ $c == "$((n + one))/$((n + one))" ]] || why+=" $c collected manifests/spikes files;"
grep -q '^transport_collect: .* 0 problems$' "$d/logs/collect.log" || why+=" collect: $(head -3 "$d/logs/collect.log");"
grep -q '^shared.net.core.busy_read=50$' "$d/transport-settings.env" && grep -q '^shared.net.ipv4.tcp_rto_min_us=absent$' "$d/transport-settings.env" || why+=" declared settings;"
m=$(find "$d/3b" -name 'manifest-3b-aa@STREAMS=1@NETEM=1-64-1-r10000.json' | head -1)
python3 -c 'import json, sys; m = json.load(open(sys.argv[1])); assert m["aa"] and m["aa_of"] == "aeron-rel" and "aa" in m["arm_order"] and m["netem_counters"]["a"]["eth1"]["dropped"] > 0 and m["loss_counters"]["b"]["nstat"]["TcpRetransSegs"] == 1 and m["threads"]["a"][0][0]["util"] > 0.9, m' "$m" 2>/dev/null || why+=" 3b A/A manifest: $m;"
m=$(find "$d/2b" -name 'manifest-2b-*' | head -1)
python3 -c 'import json, sys; m = json.load(open(sys.argv[1])); assert list(m["vms"]) == ["a"] and m["vms"]["a"] == "g/b" and m["hosts"] == {"a": "b"}, m' "$m" 2>/dev/null || why+=" 2b (host b's) manifest keyed a: $m;"
# a TCP point's traffic counters, split as tcp_lat reports them, on both hosts
m=$(find "$d/3b" -name 'manifest-3b-mio-tcp@STREAMS=1@NETEM=1-64-1-r10000.json' | head -1)
python3 -c 'import json, sys; m = json.load(open(sys.argv[1])); assert all(m["loss_counters"][h]["tcp_info"]["retrans_not_dsack"] == 0 and m["loss_counters"][h]["tcp_info"]["dsack_dups"] == 0 for h in "ab"), m["loss_counters"]' "$m" 2>/dev/null || why+=" 3b mio-tcp tcp_info split: $m;"
# the labels as the analysis parses them: every row its manifest, the one-host sets (1, 2b) one host each
python3 -c 'import sys; sys.path.insert(0, sys.argv[2]); import transport_points as tp; ps = tp.read_points(sys.argv[1]); assert ps and all(p.manifest for p in ps); assert {p.set for p in ps if tp.hosts_of(p.set) == ("a",)} == {"1", "2b"}; assert all(sorted(p.manifest["vms"]) == list(tp.hosts_of(p.set)) for p in ps); assert {p.arm for p in ps if p.manifest["aa"]} == {"aa"}' "$d" "$cloud" 2>/dev/null || why+=" transport_points.read_points: labels or hosts;"
[[ $(grep -c . "$d/order.csv") == "$(grep -c . "$d/rounds.tsv")" ]] || why+=" orders not recorded a round;"
[[ $(cut -f6 "$d/rounds.tsv" | sort -u | grep -c .) == "$(grep -c . "$d/rounds.tsv")" ]] || why+=" round seeds not each its own;"
# netem: set before each lossy round's first point and cleared after it, on both hosts, never under a DPDK arm
for lv in 0.01 0.1 1 0; do
    [[ $(grep -c "^a vm.sh netem set $lv$" "$sb/calls") == 1 && $(grep -c "^b vm.sh netem set $lv$" "$sb/calls") == 1 ]] || why+=" netem $lv not set once each host;"
done
awk '/vm.sh netem set/ {on = 1} /vm.sh netem clear/ {on = 0} on && /vm.sh driver dpdk/ {bad = 1} END {exit bad}' "$sb/calls" || why+=" a DPDK driver under netem;"
! grep -E 'NETEM' "$sb/runs" | grep -q 'dpdk' || why+=" a DPDK arm labelled with netem;"
m=$(grep -l '^mode=aeron-rel@STREAMS=1@NETEM=0.1$' "$d"/manifest/*.env | head -1)
grep -q '^netem_a=.* p=0.1 in \[.*\] ok$' "$m" && grep -q '^netem_b=.* ok$' "$m" || why+=" netem verdict: $(grep netem_ "$m" | tr '\n' ' ');"
grep -q '^placement_a=ok 2 busy threads$' "$m" && grep -q '^irq_hot_a=cpu4=1/0 cpu6=1/0' "$m" && grep -q '^seed=' "$m" && grep -q '^bin_sha256_a=' "$m" || why+=" manifest: $(tr '\n' ' ' <"$m");"
m=$(grep -l '^mode=mpsc@STREAMS=1$' "$d"/manifest/*.env | head -1)
grep -q '^placement_b=unused$' "$m" && grep -q '^netem_a=none$' "$m" || why+=" one-host manifest;"
# the flags of transport-cli.md
grep -q '^b tcp_lat --role pong .*--listen 10.20.1.7:22000 --nodelay --quickack-per-read' "$sb/runs" || why+=" tcp pong flags;"
grep -q '^a tcp_lat --role ping .*--source 10.20.1.4 --connect 10.20.1.7:22000 ' "$sb/runs" || why+=" tcp ping flags;"
grep -q '^a mpsc_lat --role local --cpus 4,6 --capacity 4096 --streams 1 --rate 1000000 --warmup-s 0 --duration-s 0 --size 64 --label 1:mpsc@STREAMS=1 ' "$sb/runs" || why+=" mpsc flags;"
grep -q '^a mpsc_lat --role local --cpus 4,6 .*--label 1:aa@STREAMS=1 ' "$sb/runs" || why+=" the A/A's label;"
grep -q '^b aeron_lat --role local --round-trip --cpus 10,12 --aeron-dir /dev/shm/aeron-bench --driver external --driver-busy-threads 3 --term-length 1048576 --mtu 1408 .*--mode spy$' "$sb/runs" || why+=" aeron flags;"
# the driver on the binaries' AERON_DIR (tmpfs), not vm.sh's hugetlbfs
# default; aeron-ipc-huge's alone (one round, on host a) on its 2 MiB
# hugetlbfs, its driver stopped and its events and page read there too
grep -q ' driver-dir /dev/shm/aeron-bench$' "$sb/calls" || why+=" driver not on TR_AERON_DIR;"
# DPDK's Aeron arms name no kernel interface (the NIC is DPDK's), the kernel's do
! grep -E 'aeron_lat .*--mode udp' "$sb/runs" | grep 'dpdk' | grep -q -- '--interface' || why+=" a DPDK aeron arm given --interface;"
# one-host DPDK: both channels on the receiver NIC (the only port DPDK reads)
grep -E 'aeron_lat --role local --round-trip .*--mode udp' "$sb/runs" | grep 'dpdk' | while read -r l; do e=${l#*--endpoint }; r=${l#*--reply }; [[ ${e%%:*} == "${r%%:*}" ]] || echo bad; done | grep -q bad && why+=" a one-host DPDK round trip with its echo off the receiver NIC;"
# two hosts: a DPDK arm's echo on host a's receiver NIC (10.20.1.5 in the fake), never its sender NIC
! grep -E 'aeron_lat --role (ping|pong) .*--mode udp' "$sb/runs" | grep dpdk | grep -qv -- '--reply 10.20.1.5:' || why+=" a two-host DPDK echo off host a's receiver NIC;"
[[ $(grep ' driver-dir ' "$sb/calls" | grep -cv ' driver-dir /dev/shm/aeron-bench$') == 1 && $(grep -c '^a driver-dir /mnt/huge-aeron/aeron-bench$' "$sb/calls") == 1 ]] ||
    why+=" drivers on another AERON_DIR: $(grep ' driver-dir ' "$sb/calls" | grep -v ' /dev/shm/aeron-bench$' | tr '\n' ' ');"
[[ $(grep -c '^a stop-dir /mnt/huge-aeron/aeron-bench$' "$sb/calls") == 1 && $(grep -c '^a events-start-dir /mnt/huge-aeron/aeron-bench$' "$sb/calls") == 3 &&
    $(grep -c '^a aeron-page-dir /mnt/huge-aeron/aeron-bench$' "$sb/calls") == 3 ]] || why+=" aeron-ipc-huge's stop, events or page not on its directory;"
# aeron-ipc-huge: aeron-ipc's flags but its Aeron directory
grep -q '^a aeron_lat --role local --round-trip --cpus 10,12 --aeron-dir /mnt/huge-aeron/aeron-bench --driver external --driver-busy-threads 3 --term-length 1048576 --mtu 1408 .*--label 1:aeron-ipc-huge@STREAMS=1 .*--mode ipc$' "$sb/runs" &&
    grep -q '^a aeron_lat --role local --round-trip --cpus 10,12 --aeron-dir /dev/shm/aeron-bench --driver external --driver-busy-threads 3 --term-length 1048576 --mtu 1408 .*--label 1:aeron-ipc@STREAMS=1 .*--mode ipc$' "$sb/runs" &&
    ! grep -v -- '--label 1:aeron-ipc-huge@' "$sb/runs" | grep -q huge-aeron || why+=" aeron-ipc-huge's flags;"
# its hostcheck on its own directory, on both hosts, its points pointing at it
grep -q '^a vm.sh hostcheck /mnt/huge-aeron/aeron-bench$' "$sb/calls" && grep -q '^b vm.sh hostcheck /mnt/huge-aeron/aeron-bench$' "$sb/calls" &&
    grep -q 'T4 ok: the Aeron directory /mnt/huge-aeron/aeron-bench is on hugetlbfs' "$d/hostcheck-a-hugepages.txt" || why+=" the hugetlbfs hostcheck;"
mi=$(grep -l '^mode=aeron-ipc@STREAMS=1$' "$d"/manifest/*.env | head -1)
mh=$(grep -l '^mode=aeron-ipc-huge@STREAMS=1$' "$d"/manifest/*.env | head -1)
grep -q '^knob.aeron_page=2097152$' "$mh" && grep -q '^aeron_page_a=2097152$' "$mh" && grep -q '^hostcheck_a=hostcheck-a-hugepages.txt$' "$mh" &&
    grep -q '^knob.aeron_page=4096$' "$mi" && grep -q '^aeron_page_a=4096$' "$mi" && grep -q '^hostcheck_a=hostcheck-a.txt$' "$mi" &&
    [[ $(grep -E '^(driver_mode|bin|args|knob.term_length|knob.aeron_threading)=' "$mh") == "$(grep -E '^(driver_mode|bin|args|knob.term_length|knob.aeron_threading)=' "$mi")" ]] ||
    why+=" aeron-ipc-huge's manifest against aeron-ipc's: $(grep -E '^(knob|aeron_page|hostcheck|driver_mode|args)' "$mh" | tr '\n' ' ');"
grep -q '^aeron-ipc-huge.knob.aeron_page=2097152$' "$d/transport-settings.env" && grep -q '^aeron-ipc.knob.aeron_page=4096$' "$d/transport-settings.env" &&
    grep -q '^declared.aeron-ipc-huge=aeron-ipc:knob.aeron_page$' "$d/transport-settings.env" &&
    [[ $(grep '^aeron-ipc-huge\.' "$d/transport-settings.env" | grep -v aeron_page | sed 's/^aeron-ipc-huge/x/') == "$(grep '^aeron-ipc\.' "$d/transport-settings.env" | grep -v aeron_page | sed 's/^aeron-ipc/x/')" ]] ||
    why+=" the declared difference in transport-settings.env: $(grep -E 'aeron-ipc|declared' "$d/transport-settings.env" | tr '\n' ' ');"
m=$(find "$d/1" -name 'manifest-1-aeron-ipc-huge@STREAMS=1-64-1-r10000.json' | head -1)
python3 -c 'import json, sys; m = json.load(open(sys.argv[1])); assert m["hostcheck"] == {"a": "hostcheck-a-hugepages.txt"} and m["settings"]["a"]["knob.aeron_page"] == "2097152", (m["hostcheck"], m["settings"])' "$m" 2>/dev/null &&
    [[ -s $d/1/hostcheck-a-hugepages.txt ]] || why+=" aeron-ipc-huge's collected manifest: $m;"
! grep -E '^. aeron_lat .*--mode (spy|ipc)' "$sb/runs" | grep -qE -- '--(endpoint|reply|interface) ' || why+=" an address given to Aeron IPC or spy;"
grep -q '^b aeron_lat --role local --round-trip --cpus 10,12 --endpoint 10.20.1.7:22000 --reply 10.20.1.6:22000 --interface 10.20.1.6 .*--mode udp --reliable false$' "$sb/runs" || why+=" aeron self-UDP flags;"
grep -q '^a aeron_lat --role ping --cpus 10 ' "$sb/runs" && grep -q '^b aeron_lat --role pong .*--cpus 10 ' "$sb/runs" || why+=" one CPU for ping and pong;"
for i in mio ergon; do
    grep -q "^a tcp_lat --role ping .*--busy-read-us 50 .*--impl $i\$" "$sb/runs" || why+=" --busy-read-us 50 not given to tcp_lat --impl $i;"
done
# the hosts' kernels lack the TCP timer sysctls (the fake's settings): no timer asked for
! grep -qE -- '--(rto-min|delack-max)-us' "$sb/runs" || why+=" a TCP timer asked of a kernel without it;"
grep -q '^a aeron_lat --role ping .*--mode udp --reliable true$' "$sb/runs" || why+=" aeron arm flags;"
grep -q '^b aeron_lat --role pong .*--endpoint 10.20.1.7:22000 --reply 10.20.1.4:22000 --interface 10.20.1.7 ' "$sb/runs" &&
    grep -q '^a aeron_lat --role ping .*--endpoint 10.20.1.7:22000 --reply 10.20.1.4:22000 --interface 10.20.1.4 ' "$sb/runs" || why+=" aeron addresses;"
# every binary started on the housekeeping CPU, by taskset, its PID recorded
[[ $(grep -c . "$sb/taskset") -ge $((n + one)) && -z $(grep -v ' taskset 0$' "$sb/taskset") ]] || why+=" not started on CPU 0: $(sort -u "$sb/taskset" | head -2);"
! grep -q 'oneway-' "$sb/runs" || why+=" a oneway role;"
# sets 1 (host a) and 2b (host b) at once, each on its own host only
! grep -q '^b mpsc_lat' "$sb/runs" && ! grep -q '^b aeron_lat .*--mode ipc' "$sb/runs" && ! grep -qE '^a aeron_lat --role local .*--mode (spy|udp)' "$sb/runs" ||
    why+=" a one-host set on the other host;"
grep -q '^a vm.sh hostcheck$' "$sb/calls" && grep -q '^b vm.sh hostcheck$' "$sb/calls" && [[ -s $d/hostcheck-a.txt ]] || why+=" hostcheck not run and saved;"
# end to end: the session as transport_collect.py leaves it, through
# fairness.py (no violation: the stub's every max at :50 of its minute) and
# analyse.py --transport (every set's tables, none refused, an A/A spread)
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1) || why+=" fairness: $(head -5 <<<"$fv" | tr '\n' ' ');"
tables=$(python3 "$cloud/analyse.py" --transport "$d" 2>&1) || why+=" analyse.py --transport failed: $(tail -2 <<<"$tables");"
for s in 1 2b 3b 3c; do grep -q "^### Set $s, " <<<"$tables" || why+=" no set $s table;"; done
! grep -q '^Refused' <<<"$tables" && grep -q 'A/A spread' <<<"$tables" && grep -q 'no qdisc' <<<"$tables" || why+=" tables refused or no A/A spread or no no-qdisc table;"
# each arm's per-arm knobs declared as it ran them (busy-read both socket
# impls'; no sweep line: no tuning session ran), every point's manifest
# recording them
grep -q '^mio-tcp.knob.busy_read_us=50$' "$d/transport-settings.env" && grep -q '^eio-tcp.knob.epoll_events=64$' "$d/transport-settings.env" &&
    grep -q '^mpsc.knob.capacity=4096$' "$d/transport-settings.env" && grep -q '^aeron-rel.knob.nak_delay_ns=1000$' "$d/transport-settings.env" &&
    grep -q '^eio-tcp.knob.busy_read_us=50$' "$d/transport-settings.env" && ! grep -q '^sweep\.' "$d/transport-settings.env" &&
    ! grep -q 'knob.congestion' "$d/transport-settings.env" || why+=" per-arm knobs: $(grep -v '^#' "$d/transport-settings.env" | grep -v '^shared' | tr '\n' ' ');"
m=$(grep -l '^mode=mio-tcp@STREAMS=1@NETEM=0.1$' "$d"/manifest/*.env | head -1)
grep -q '^knob.busy_read_us=50$' "$m" && grep -q '^knob.quickack_per_read=on$' "$m" && grep -q '^knob.congestion=cubic$' "$m" && grep -q '^base_arm=mio-tcp$' "$m" ||
    why+=" a point's knobs: $(grep '^knob' "$m" | tr '\n' ' ');"
m=$(grep -l '^mode=aeron-rel@STREAMS=1@NETEM=0.1$' "$d"/manifest/*.env | head -1)
grep -q '^driver_mode=kernel-busy@CONDUCTOR=own@' "$m" || why+=" an Aeron point's driver mode: $(grep '^driver_mode' "$m");"
# the data NICs' interrupt deferral, each arm at its own best: every socket
# point with none (0/0 on every NIC of both hosts: its own host settings,
# transport-net with TR_OVERRIDES), every Aeron kernel UDP point at its
# driver's @NAPI_DEFER=0/0 (TR_AERON_NAPI_DEFER), each read back after the point; the hosts' own
# (2/200000) put back after each socket arm; none read under DPDK or IPC
nw=
for m in $(grep -l '^setup=sockets$' "$d"/manifest/*.env); do
    grep -q '^knob.napi_defer=0$' "$m" && grep -q '^napi_a=enP1:0/0,enP2:0/0$' "$m" && grep -q '^napi_b=enP1:0/0,enP2:0/0$' "$m" &&
        grep -q '^napi_defer_a=0$' "$m" && grep -q '^napi_defer_b=0$' "$m" || nw+=" $(grep '^mode=' "$m"): $(grep -E '^(knob.napi|napi)' "$m" | tr '\n' ' ');"
done
[[ $(grep -l '^setup=sockets$' "$d"/manifest/*.env | wc -l | tr -d ' ') -gt 0 ]] || nw+=" no socket point;"
for m in $(grep -lE '^arm=(aeron-rel|aeron-self)$' "$d"/manifest/*.env); do
    grep -q '^knob.napi_defer=0$' "$m" && grep -qE '^napi_[ab]=enP1:0/0,enP2:0/0$' "$m" && grep -qE '^napi_defer_[ab]=0$' "$m" &&
        ! grep -qE '^napi_[ab]=.*:2/' "$m" && grep -q '@NAPI_DEFER=0/0' "$m" ||
        nw+=" $(grep '^mode=' "$m"): $(grep -E '^(knob.napi|napi)' "$m" | tr '\n' ' ');"
done
! grep -lE '^arm=(mpsc|aeron-ipc|aeron-spy|dpdk-aeron-self)$' "$d"/manifest/*.env | xargs grep -lE '^(knob\.napi_defer|napi_)' >/dev/null 2>&1 || nw+=" a NIC deferral recorded for a non-UDP or DPDK arm;"
for h in a b; do
    s=$(grep "^$h transport-net napi " "$sb/calls" | cut -d' ' -f4,5 | tr '\n' ';')
    [[ $s =~ ^(0\ 0\;2\ 200000\;)+$ ]] || nw+=" host $h's transport-net NIC deferral not 0/0 for each socket arm, put back after: $s;"
done
grep -q '^mio-tcp.knob.napi_defer=0$' "$d/transport-settings.env" && grep -q '^aeron-rel.knob.napi_defer=0$' "$d/transport-settings.env" &&
    grep -qE '^best.knob.napi_defer=(.*,)?aeron-rel:0,(.*,)?mio-tcp:0(,|$)' "$d/transport-settings.env" || nw+=" collected: $(grep napi "$d/transport-settings.env" | tr '\n' ' ');"
[[ -z $nw ]] || why+=" NIC deferral:$nw"
echo "$tables" >"$tmp/tables.md"
# every point's events on each host it used: begun before its binaries
# (the fake's log orders them), read back after, gzipped, laid out per host
# and tag; each max then labelled by them (the fake's IRQ at it), never
# unexplained
evwhy=
for h in a b; do
    [[ -s $sb/$h/state/events-started && $(grep -c . "$sb/$h/state/events-started") == $(grep -c . "$sb/$h/state/events-stopped") ]] ||
        evwhy+=" host $h: events started $(grep -c . "$sb/$h/state/events-started" 2>/dev/null) stopped $(grep -c . "$sb/$h/state/events-stopped" 2>/dev/null);"
done
[[ $(find "$d/events" -name '*.log.gz' | wc -l | tr -d ' ') -ge $n ]] || evwhy+=" $(find "$d/events" -name '*.log.gz' | wc -l) events logs for $n points;"
c=$(find "$d" -mindepth 2 -maxdepth 2 -name 'a-events-*.csv' | wc -l | tr -d ' ')
[[ $c == $((n + one)) ]] || evwhy+=" $c host-a events files, not $((n + one));"
[[ $(find "$d/3b" -name 'b-events-*.csv' | wc -l | tr -d ' ') -gt 0 && -z $(find "$d/1" "$d/2b" -name 'b-events-*.csv') ]] || evwhy+=" host b's events files on the wrong sets;"
# spinners on the hot CPUs the point leaves idle: a socket ping's (4) and an
# Aeron arm's driver (4, 6, 8) and local ends (10, 12) left out
grep -q '^3b-r1-mio-tcp@STREAMS=1@NETEM=1-10000 4,6,8,10,12,14 6,8,10,12,14 *$' "$sb/a/state/events-started" || evwhy+=" mio-tcp's spinners: $(grep -m1 3b-r1-mio-tcp "$sb/a/state/events-started");"
grep -q '^1-r1-aeron-ipc@STREAMS=1-10000 4,6,8,10,12,14 14 aeron$' "$sb/a/state/events-started" || evwhy+=" aeron-ipc's spinners or counters: $(grep -m1 aeron-ipc "$sb/a/state/events-started");"
labels=$(python3 "$cloud/spikes.py" --transport "$d" 2>&1)
[[ $(grep -c 'IRQ fake_nic on hot CPU 4 (a, 100 µs)' <<<"$labels") == $((n + one)) ]] && grep -q '^unexplained: none$' <<<"$labels" ||
    evwhy+=" labels: $(grep -c 'IRQ fake_nic' <<<"$labels") IRQ labels of $((n + one)); $(grep -m2 '| unexplained' <<<"$labels");"
python3 -c 'import json, sys; m = json.load(open(sys.argv[1])); e = m["events"]; assert set(e) == {"a", "b"} and any("osnoise unavailable" in x for x in e["a"]["notes"]), e' \
    "$(find "$d/3b" -name 'manifest-3b-mio-tcp@STREAMS=1@NETEM=1-64-1-r10000.json' | head -1)" 2>/dev/null || evwhy+=" the manifest's events notes;"
[[ -z $evwhy ]] && pass "transport.sh run: each point's events begun before its binaries and read back after on every host it used (spinners on the idle hot CPUs, Aeron's counters), laid out per host and tag, every max labelled from them (spikes.py --transport)" ||
    fail "transport.sh run events:$evwhy"
[[ -z $why ]] && pass "transport.sh run: every point of pair b once (rows with their rep, manifests, the binaries' manifests), each round's seed and order recorded, netem around each lossy round on both hosts and never under DPDK, its counters checked, transport-cli.md's flags, sets 1 and 2b each on its own host, the hostcheck reports saved; fairness.py clean on the collected session and analyse.py --transport's tables" ||
    fail "transport.sh run:$why $(tail -3 "$sb/run.out")"

# a bad placement and netem counters off their loss are in the manifests
# (and logged); a failed point recorded and the session's exit 1
trun b 1 "" FAKE_PLACEMENT_BAD=1 FAKE_NETEM_BAD=1 FAKE_AERON_PAGE_BAD=1 FAKE_FAIL_LABEL='3c:mio-tcp@STREAMS=1@NETEM=0' FAKE_PONG_SILENT='3c:eio-tcp@STREAMS=1@NETEM=0' FAKE_PONG_EXIT='3c:aeron-rel@STREAMS=1@NETEM=0' FAKE_DRIVER_FAIL=dpdk FAKE_SH_DELAY=0.3 && rrc=0 || rrc=$?
why=
[[ $rrc != 0 ]] || why+=" exit 0;"
m=$(grep -l '^mode=aeron-rel@STREAMS=1@NETEM=1$' "$d"/manifest/*.env | head -1)
grep -q '^placement_a=FAIL' "$m" && grep -q '^netem_a=.* FAIL$' "$m" || why+=" verdicts: $(grep -E 'placement_|netem_' "$m" | tr '\n' ' ');"
grep -q 'WARNING: point .* netem counters off its loss' "$sb/run.out" || why+=" not logged;"
# the hugetlbfs read back at 4 KiB: aeron-ipc-huge's points break the settings rule
fv=$(python3 "$cloud/fairness.py" "$d" 2>/dev/null || true)
grep -q '^settings: 1: 1-aeron-ipc-huge@STREAMS=1-64-1-r10000: a knob.aeron_page=4096, the env gives 2097152$' <<<"$fv" ||
    why+=" a 4 KiB page on aeron-ipc-huge not flagged: $(grep aeron_page <<<"$fv" | head -2 || true);"
[[ $(grep -c '^3c,1,0,mio-tcp,0,1$' "$d/failures.csv") == 1 ]] || why+=" failures: $(cat "$d/failures.csv");"
# a pong that never says ready: its point failed, its ping never started
[[ $(grep -c '^3c,1,0,eio-tcp,0,1$' "$d/failures.csv") == 1 ]] && ! grep -q '^a tcp_lat --role ping .*--label 3c:eio-tcp@STREAMS=1@NETEM=0 ' "$sb/runs" &&
    grep -q 'the pong did not say ready within 3 s' "$sb/run.out" || why+=" a silent pong: $(grep -c eio-tcp "$d/failures.csv") failures;"
# a pong that exits before ready: its exit status and its error lines (the
# EAL's, on stderr) in the log line, not its last two (F-Stack's config echo)
[[ $(grep -c '^3c,1,0,aeron-rel,0,1$' "$d/failures.csv") == 1 ]] &&
    grep -q "point 3c-r1-aeron-rel@STREAMS=1@NETEM=0-50000: the pong exited before it said ready (exit 1): EAL: Cannot use IOVA as 'PA' since physical addresses are not available EAL: Error - exiting with code: 1 *$" "$sb/run.out" ||
    why+=" a pong that exited: $(grep 'exited before' "$sb/run.out" | head -1);"
# a failed driver set-up: the step's error line in run.log, the driver's log kept
grep -q 'host b: dpdk set-up failed: vm.sh: line 2096: "return $rc" failed (exit 1): stopping (.*/logs/setup-b.log, driver log .*/logs/driver-b-1.log)' "$sb/run.out" &&
    grep -q 'fake driver: EAL: Error' "$d/logs/driver-b-1.log" || why+=" a failed driver set-up: $(grep 'set-up failed' "$sb/run.out" | head -1);"
# every pong slow to write its PID file: waited for, not taken for an exit
[[ $(grep -vc ',dpdk-aeron-self,' "$d/failures.csv") == 3 ]] && ! grep 'exited before it said ready' "$sb/run.out" | grep -vq 'point 3c-r1-aeron-rel@' || why+=" a slow pong taken for gone: $(tr '\n' ' ' <"$d/failures.csv");"
[[ -z $why ]] && pass "transport.sh run: a failed placement or netem verdict in the manifest and logged, a failed point, a pong that never said ready and one that exited (its exit status and error lines logged) in failures.csv (a pong slow to start waited for), the session's exit 1" ||
    fail "transport.sh run failures:$why"

# ---- sink arms (aeron_lat --mode spy-udp) ----
# the plan: a sink arm runs in a set on host a, netem -, and holds host b
# too (never beside a set on b); a one-host arm may share its set
refusals=
for case in "set z b - b 1 1 - aeron-spy-udp|runs in a set on host a" "set z a - a 1 1 0.1 aeron-spy-udp|runs in a set on host a" \
    "set z a - ab 1 1 - aeron-spy-udp|host ab takes" "set z a p1 a 1 1 - aeron-spy-udp
set y a p1 b 1 1 - aeron-spy|one-host sets on different hosts"; do
    printf '%s\n' "${case%|*}" >"$bad"
    if out=$(plan modes a "$matrix" "$bad" 2>&1); then refusals+=" accepted[${case%|*}]"; fi
    [[ $out == *"${case#*|}"* ]] || refusals+=" [${case%|*}: $out]"
done
printf '%s\n' "set z a - a 1 1 - aeron-spy-udp,aeron-spy,aa" >"$bad"
plan modes a "$matrix:arms" "$bad" >/dev/null 2>&1 || refusals+=" a sink arm beside a one-host arm refused;"
[[ -z $refusals ]] && pass "transport_plan.py: a sink arm only in a set on host a with no netem, beside one-host arms, never at once with a set on b" ||
    fail "transport_plan.py sink refusals:$refusals"
# a session of the spy set: aeron-spy (one host), aeron-spy-udp and
# dpdk-aeron-spy-udp (sink arms: --role local on a, a sink on b for each
# run), and the A/A of aeron-spy-udp
spysets=$tmp/spy-sets.tsv
echo "set spy a - a 10000 1 - aeron-spy-udp,aeron-spy,dpdk-aeron-spy-udp,aa" >"$spysets"
TRUN_SETS="$spysets" trun a 1 "" && rrc=0 || rrc=$?
d=$sb/a/bench/transport-a
why=
[[ $rrc == 0 && $(cat "$d/rc") == 0 ]] || why+=" exit $rrc rc $(cat "$d/rc" 2>/dev/null): $(tail -3 "$sb/run.out" | tr '\n' ' ');"
[[ $(grep -c . "$d/results.csv") == 8 ]] || why+=" $(grep -c . "$d/results.csv") rows, not 8 (4 arms, one way and round trip);"
# the measuring runs on a to the sink's address on b (the receiver NIC, the
# echoes TR_PORT_SPAN/2 above), a's own sender NIC its kernel interface
grep -q '^a aeron_lat --role local --round-trip --cpus 10,12 --endpoint 10.20.1.7:22000 --reply 10.20.1.7:22512 --interface 10.20.1.4 --aeron-dir .*--label spy:aeron-spy-udp@STREAMS=1 .*--mode spy-udp$' "$sb/runs" ||
    why+=" spy-udp local flags: $(grep -m1 'role local .*spy-udp' "$sb/runs");"
grep -q '^b aeron_lat --role pong --streams 1 .*--label spy:aeron-spy-udp@STREAMS=1 .*--cpus 10 --out .*out-sink0 --endpoint 10.20.1.7:22000 --reply 10.20.1.7:22512 --interface 10.20.1.7 --aeron-dir .*--mode spy-udp$' "$sb/runs" ||
    why+=" sink flags: $(grep -m1 'role pong' "$sb/runs");"
! grep -E 'aeron_lat .*spy:dpdk-aeron-spy-udp' "$sb/runs" | grep -q -- '--interface' || why+=" a DPDK sink arm given --interface;"
! grep -q '^b aeron_lat --role local' "$sb/runs" && ! grep -q '^a aeron_lat --role pong' "$sb/runs" || why+=" a sink arm's ends on the wrong hosts;"
# two sinks a sink arm's point (one a run), each on its run's stream ids;
# none for aeron-spy
[[ $(grep -c '^b aeron_lat --role pong ' "$sb/runs") == 6 ]] || why+=" $(grep -c '^b aeron_lat --role pong ' "$sb/runs") sinks, not 6;"
for sidv in $(grep '^b aeron_lat --role pong ' "$sb/runs" | sed 's/.*--stream-id \([0-9]*\).*/\1/'); do
    grep -q "^a aeron_lat --role local .*--stream-id $sidv " "$sb/runs" || why+=" sink stream id $sidv not a run's;"
done
! grep -q '^b aeron_lat .*spy:aeron-spy@' "$sb/runs" || why+=" a sink for aeron-spy;"
# media drivers on b for the sink arms alone (3 rounds: kernel, dpdk, the A/A)
[[ $(grep -c '^b vm.sh driver ' "$sb/calls") == 3 && $(grep -c '^a vm.sh driver ' "$sb/calls") == 4 ]] ||
    why+=" drivers: a $(grep -c '^a vm.sh driver ' "$sb/calls"), b $(grep -c '^b vm.sh driver ' "$sb/calls");"
# events: spinners on the hot CPUs each host leaves idle: on a the local
# ends' (10, 12) and the driver's in use, on b the sink's (10) and the driver's
grep -q '^spy-r1-aeron-spy-udp@STREAMS=1-10000 4,6,8,10,12,14 14 aeron$' "$sb/a/state/events-started" &&
    grep -q '^spy-r1-aeron-spy-udp@STREAMS=1-10000 4,6,8,10,12,14 12,14 aeron$' "$sb/b/state/events-started" &&
    ! grep -q 'aeron-spy@' "$sb/b/state/events-started" || why+=" events: a $(grep -m1 spy-udp "$sb/a/state/events-started"), b $(grep -m1 spy-udp "$sb/b/state/events-started");"
m=$(grep -l '^mode=aeron-spy-udp@STREAMS=1$' "$d"/manifest/*.env | head -1)
grep -q '^arm_hosts=sink$' "$m" && grep -q '^placement_b=ok' "$m" || why+=" sink arm runner manifest: $(grep -E '^(arm_hosts|placement_)' "$m" | tr '\n' ' ');"
[[ -s $d/points/spy-r1-aeron-spy-udp@STREAMS=1-10000/sink0/manifest.json && -s $d/points/spy-r1-aeron-spy-udp@STREAMS=1-10000/sink1/manifest.json ]] || why+=" sinks' manifests not kept;"
python3 -c '
import json, sys
sys.path.insert(0, sys.argv[2])
import transport_points as tp
ps = tp.read_points(sys.argv[1])
assert len(ps) == 8, len(ps)
for p in ps:
    m = p.manifest
    if p.arm == "aeron-spy":
        assert m["hosts"] == {"a": "a"} and m["sink"] is None and tp.hosts_of_point(p) == ("a",), m["hosts"]
    else:
        assert m["hosts"] == {"a": "a", "b": "b"} and tp.hosts_of_point(p) == ("a", "b"), (p.tag, m["hosts"])
        assert m["sink"]["settings"]["sink.ping_frames"] == "100", m["sink"]
        assert m["loss_counters"]["b"]["aeron"]["naks-sent"] == 0, m["loss_counters"]
' "$d" "$cloud" 2>/dev/null || why+=" collected sink manifests or hosts;"
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1) || why+=" fairness: $(head -5 <<<"$fv" | tr '\n' ' ');"
tables=$(python3 "$cloud/analyse.py" --transport "$d" 2>&1) || why+=" analyse.py --transport failed: $(tail -2 <<<"$tables");"
grep -q '^### Set spy, ' <<<"$tables" && ! grep -q '^Refused' <<<"$tables" || why+=" no set spy table, or one refused;"
# a sink that read fewer pings than its run sent: the sink rule
TRUN_SETS="$spysets" trun a 1 "" FAKE_SINK_SHORT=spy:aa
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1 || true)
[[ $(grep -c '^sink: spy: .*the sink received 90 of 100 pings$' <<<"$fv") == 2 ]] && ! grep '^sink:' <<<"$fv" | grep -qv 'spy-aa@' ||
    why+=" a short sink not flagged (or another): $(grep '^sink' <<<"$fv" | head -3 | tr '\n' ' ');"
[[ -z $why ]] && pass "transport.sh run: a sink arm runs on a with a sink on b for each run (its run's stream ids, the sink's address, a driver on b), aeron-spy beside it on a alone; the sinks' manifests collected, fairness.py clean, and its sink rule flags a sink short of the pings sent" ||
    fail "transport.sh run sink arms:$why"
d=$sb/a/bench/transport-b

# a hostcheck warning stops the session before its first point
trun b 1 "" FAKE_HOSTCHECK_WARN=b && rrc=0 || rrc=$?
if [[ $rrc != 0 && $(cat "$d/rc" 2>/dev/null) == 1 && ! -s $sb/runs ]] && grep -q 'host b: the hostcheck report is not clean.*T3 warn' "$sb/run.out"; then
    pass "transport.sh run: a hostcheck warning on either host stops the session before its first point"
else
    fail "transport.sh run with a hostcheck warning: exit $rrc, $(grep -c . "$sb/runs" 2>/dev/null) runs, $(tail -1 "$sb/run.out")"
fi
# the SMT sessions (transport-aws-smt-off.tsv, transport-aws-smt.tsv) at 1
# rep: SMT off, uring-sqpoll's SQPOLL thread on the next isolated core
# ({next_core}); SMT on (`smt on`, each isolated core whole in the map),
# its SQPOLL thread on the loop CPU's sibling ({loop_sibling}). Each: the
# arguments (no SO_BUSY_POLL for uring-sqpoll), the SQPOLL CPU and the
# loop's sibling never a gap spinner's, the declared SMT read back at every
# point, fairness.py clean (T10 and the SQPOLL thread exempt only where they
# belong) and every table there
why=
TRUN_SETS=$cloud/transport-aws-smt-off.tsv trun a 1 "" FAKE_SQPOLL="eio-tcp-sqpoll-core:6" && rrc=0 || rrc=$?
d=$sb/a/bench/transport-a
[[ $rrc == 0 && $(cat "$d/rc") == 0 ]] || why+=" smt off: exit $rrc: $(tail -3 "$sb/run.out" | tr '\n' ' ');"
grep -q 'SMT off' "$sb/run.out" || why+=" smt off: not logged;"
grep -q '^devices/system/cpu/smt/active 0 optional$' "$d/declared-sysctls.txt" || why+=" smt off: not declared 0;"
grep -q '^a tcp_lat --role ping --cpus 4 .*--busy-read-us 0 .*--impl ergon --backend uring-sqpoll --sqpoll-cpu 6' "$sb/runs" || why+=" smt off: the sqpoll-core ping's arguments: $(grep sqpoll "$sb/runs" | head -1);"
grep -q '^b tcp_lat --role pong .*--cpus 4 .*--busy-read-us 0 .*--sqpoll-cpu 6' "$sb/runs" || why+=" smt off: the sqpoll-core pong's arguments;"
grep -q '^a tcp_lat --role ping .*--busy-read-us 50 .*--impl mio' "$sb/runs" || why+=" smt off: mio-tcp's busy poll;"
# host a's events: the spinners on the isolated CPUs less the ping's (4) and
# the SQPOLL thread's (6)
grep -q -- '-eio-tcp-sqpoll-core.* 8,10,12,14 $' "$sb/a/state/events-started" || why+=" smt off: events $(grep sqpoll "$sb/a/state/events-started" | head -1);"
grep -q '^knob.busy_read_us=' "$d"/manifest/smtoff-r1-eio-tcp-sqpoll-core*.env && why+=" smt off: a busy_read_us knob for uring-sqpoll;"
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1) || why+=" smt off fairness: $(head -5 <<<"$fv" | tr '\n' ' ');"
tables=$(python3 "$cloud/analyse.py" --transport "$d" 2>&1) || why+=" smt off analyse.py: $(tail -2 <<<"$tables");"
grep -q '^### Set smtoff, ' <<<"$tables" && ! grep -q '^Refused' <<<"$tables" || why+=" smt off: no table, or one refused;"
# the SQPOLL thread never seen on its CPU: the sqpoll rule
TRUN_SETS=$cloud/transport-aws-smt-off.tsv trun a 1 "" FAKE_SQPOLL="eio-tcp-sqpoll-core:8"
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1 || true)
grep -q '^sqpoll: smtoff: .*no SQPOLL thread (iou-sqp-\*) busy on --sqpoll-cpu 6' <<<"$fv" || why+=" smt off: an SQPOLL thread off its CPU not flagged: $(head -2 <<<"$fv" | tr '\n' ' ');"
smtmap="0 1 2,10,3,11,4,12,5,13,6,14,7,15"
TRUN_SETS=$cloud/transport-aws-smt.tsv trun a 1 "" FAKE_MAP="$smtmap" FAKE_SMT_ACTIVE=1 FAKE_PING_CPU=2 FAKE_SQPOLL="eio-tcp-sqpoll-sib:10" && rrc=0 || rrc=$?
[[ $rrc == 0 && $(cat "$d/rc") == 0 ]] || why+=" smt on: exit $rrc: $(tail -3 "$sb/run.out" | tr '\n' ' ');"
grep -q 'SMT on' "$sb/run.out" || why+=" smt on: not logged;"
grep -q '^devices/system/cpu/smt/active 1 optional$' "$d/declared-sysctls.txt" || why+=" smt on: not declared 1;"
grep -q '^a tcp_lat --role ping --cpus 2 .*--busy-read-us 0 .*--backend uring-sqpoll --sqpoll-cpu 10' "$sb/runs" || why+=" smt on: the sqpoll-sib ping's arguments: $(grep sqpoll "$sb/runs" | head -1);"
grep -q '^b tcp_lat --role pong .*--cpus 2 .*--sqpoll-cpu 10' "$sb/runs" || why+=" smt on: the sqpoll-sib pong's arguments;"
# the loop's sibling (10) traced and never a spinner's, under either arm
for a in eio-tcp-sqpoll-sib mio-tcp-smt; do
    grep -q -- "-$a.* 3,11,4,12,5,13,6,14,7,15 \$" "$sb/a/state/events-started" || why+=" smt on: $a's events $(grep -- "-$a" "$sb/a/state/events-started" | head -1);"
done
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1) || why+=" smt on fairness: $(head -5 <<<"$fv" | tr '\n' ' ');"
tables=$(python3 "$cloud/analyse.py" --transport "$d" 2>&1) || why+=" smt on analyse.py: $(tail -2 <<<"$tables");"
grep -q '^### Set smton, ' <<<"$tables" && ! grep -q '^Refused' <<<"$tables" || why+=" smt on: no table, or one refused;"
# the same session's SMT read back off at its points: the settings rule
TRUN_SETS=$cloud/transport-aws-smt.tsv trun a 1 "" FAKE_MAP="$smtmap" FAKE_PING_CPU=2 FAKE_SQPOLL="eio-tcp-sqpoll-sib:10"
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1 || true)
grep -q '^settings: smton: .*devices.system.cpu.smt.active=0, the env gives 1' <<<"$fv" || why+=" smt on: SMT read back off not flagged: $(head -2 <<<"$fv" | tr '\n' ' ');"
# an SMT-on sets file takes two-host socket arms alone, and {loop_sibling} needs one
printf 'smt on\nset x a - ab 10000 1 - aeron-rel,mio-tcp,aa\n' >"$tmp/smt-aeron.tsv"
printf 'set x a - ab 10000 1 - eio-tcp-sqpoll-sib,mio-tcp,aa\n' >"$tmp/smt-sib-off.tsv"
printf 'smt on\nsmt off\nset x a - ab 10000 1 - mio-tcp,aa\n' >"$tmp/smt-both.tsv"
c=$( (. "$cloud/transport.sh"; tr_sets_file=$tmp/smt-aeron.tsv tr_check a 2>&1) ) && why+=" an Aeron arm under smt on taken;"
grep -q 'smt on: arm aeron-rel (aeron_lat, setup kernel, hosts two): an SMT-on session runs two-host socket arms only' <<<"$c" || why+=" smt on Aeron: $c;"
c=$( (. "$cloud/transport.sh"; tr_sets_file=$tmp/smt-sib-off.tsv tr_check a 2>&1) ) && why+=" {loop_sibling} without smt on taken;"
grep -q 'arm eio-tcp-sqpoll-sib: {loop_sibling} (the loop CPU.s SMT sibling) needs a sets file with smt on' <<<"$c" || why+=" sib without smt on: $c;"
c=$(plan smt "$matrix:arms" "$tmp/smt-both.tsv" 2>&1) && why+=" disagreeing smt rows taken;"
grep -q 'smt rows disagree' <<<"$c" || why+=" smt rows: $c;"
[[ -z $why ]] && pass "transport.sh run: the SMT sessions: SMT off with uring-sqpoll's SQPOLL thread on the next core, SMT on (each isolated core whole) with it on the loop's sibling; no SO_BUSY_POLL for uring-sqpoll, the SQPOLL CPU and the loop's sibling never a spinner's, the declared SMT read back, fairness.py clean and its smt, sqpoll and settings rules flagging a misplaced SQPOLL thread and SMT read back off; check refuses an Aeron arm under smt on and {loop_sibling} without it" ||
    fail "transport.sh run SMT sessions:$why"

# the c7i diagnosis (transport-aws-c7i-diag.tsv) at 1 rep: mio-tcp and its
# declared twins, no SO_BUSY_POLL (-br0) and the IRQ core's idle capped at
# the first halt state (-c1) or at polling (-c0), beside uring-sqpoll and the
# A/A. Each twin's one knob (its arguments, or transport-net's TR_IRQ_IDLE
# put back to all at the next arm), read back per point (irq_idle_<host>),
# every point's idle states before and after and its NICs' moderation after,
# fairness.py clean, its settings rule flagging a cap the host did not hold,
# and the session's points and hours on c7i.4xlarge on demand
why=
c7i=$cloud/transport-aws-c7i-diag.tsv
TRUN_SETS=$c7i trun a 1 "" FAKE_SQPOLL="eio-tcp-sqpoll-core:6" && rrc=0 || rrc=$?
d=$sb/a/bench/transport-a
[[ $rrc == 0 && $(cat "$d/rc") == 0 ]] || why+=" exit $rrc: $(tail -3 "$sb/run.out" | tr '\n' ' ');"
grep -q '^a tcp_lat --role ping .*--busy-read-us 0 .*--impl mio' "$sb/runs" || why+=" mio-tcp-br0's ping without SO_BUSY_POLL;"
for a in mio-tcp mio-tcp-br0 mio-tcp-c1 mio-tcp-c0; do
    m=$(grep -l "^arm=$a\$" "$d"/manifest/*.env 2>/dev/null | head -1 || true)
    case $a in mio-tcp-c1) want=1 br=50 ;; mio-tcp-c0) want=0 br=50 ;; mio-tcp-br0) want=all br=0 ;; *) want=all br=50 ;; esac
    [[ -n $m ]] && grep -q "^knob.irq_idle=$want\$" "$m" && grep -q "^irq_idle_a=$want\$" "$m" && grep -q "^irq_idle_b=$want\$" "$m" &&
        grep -q "^knob.busy_read_us=$br\$" "$m" || why+=" $a: $(grep -hE '^(knob.irq_idle|irq_idle|knob.busy_read)' "$m" 2>/dev/null | tr '\n' ' ');"
done
m=$(grep -l '^arm=eio-tcp-sqpoll-core$' "$d"/manifest/*.env 2>/dev/null | head -1 || true)
[[ -n $m ]] && grep -q '^knob.irq_idle=all$' "$m" && grep -q '^irq_idle_a=all$' "$m" || why+=" sqpoll's idle: $(grep irq_idle "$m" | tr '\n' ' ');"
# each arm's cap on both hosts, all put back after a capped arm
for h in a b; do
    s=$(grep "^$h transport-net irq_idle " "$sb/calls" | cut -d' ' -f4 | tr '\n' ' ')
    [[ $s == *"0 all "* && $s == *"1 all "* && $(tr ' ' '\n' <<<"$s" | grep -c '^[01]$') == 2 ]] || why+=" host $h's transport-net idle caps: $s;"
done
n=0
for f in "$d"/manifest/*.env; do
    pid=$(basename "$f" .env)
    for h in a b; do
        [[ -s $d/idle/$pid-$h-before && -s $d/idle/$pid-$h-after && -s $d/nicstats/$pid-$h-before && -s $d/nicstats/$pid-$h-after ]] || n=$((n + 1))
    done
done
[[ $n == 0 ]] || why+=" $n point-hosts without idle or NIC counters read back;"
m=$(grep -l '^arm=mio-tcp-c1$' "$d"/manifest/*.env 2>/dev/null | head -1 || true)
[[ -n $m ]] && grep -q '^cpu2 irq 3 C6 1 ' "$d/idle/$(basename "$m" .env)-a-after" || why+=" the IRQ CPU's states not read;"
# nicstats' one format: traffic, moderation and driver counters per NIC
f=$d/nicstats/$(basename "$m" .env)-a-after
[[ -n $m ]] && grep -q '^sender eth1 tx_bytes=[0-9]* ' "$f" && grep -q '^receiver eth2 coalesce off n/a 0 0$' "$f" &&
    grep -q '^sender eth1 stat queue_0_tx_unmask_interrupt [0-9]*$' "$f" || why+=" nicstats without traffic, moderation or driver counters;"
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1) || why+=" fairness: $(head -5 <<<"$fv" | tr '\n' ' ');"
tables=$(python3 "$cloud/analyse.py" --transport "$d" 2>&1) || why+=" analyse.py: $(tail -2 <<<"$tables");"
grep -q '^### Set c7i, ' <<<"$tables" && ! grep -q '^Refused' <<<"$tables" || why+=" no table, or one refused;"
# a host that does not hold the cap: the settings rule
TRUN_SETS=$c7i trun a 1 "" FAKE_SQPOLL="eio-tcp-sqpoll-core:6" FAKE_IDLE_STUCK=1
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1 || true)
grep -q '^settings: c7i: .*knob.irq_idle=all, the env gives 1' <<<"$fv" || why+=" a cap not held not flagged: $(head -2 <<<"$fv" | tr '\n' ' ');"
rm -f "$tmp/cloud-calls"
out=$(awslab LAB_ESTIMATE=1 LAB_PAIR=a LAB_REPS=3 LAB_TYPE=c7i.4xlarge LAB_SETS_FILE="$c7i" 2>&1) || why+=" estimate: $(tail -1 <<<"$out");"
[[ $out == *"pair a: 18 points (sets c7i)"*"wall about "* && $out == *"a c7i.4xlarge, b c7i.4xlarge, on-demand, 3 reps"* ]] || why+=" estimate: $out;"
[[ ! -s $tmp/cloud-calls ]] || why+=" an aws call;"
[[ -z $why ]] && pass "transport.sh run: the c7i diagnosis: mio-tcp's twins without SO_BUSY_POLL and with the IRQ core's idle capped at C1 or polling, each cap set by transport-net, read back per point and put back after, every point's idle states and NIC moderation and counters recorded, fairness.py clean and its settings rule flagging a cap not held; 18 points on c7i.4xlarge on demand, no aws call" ||
    fail "transport.sh run c7i diagnosis:$why"

# the tuning session (LAB_TUNE=1: pair tune-a), cut down to two knobs of
# pair a's one-host arms: mpsc's capacity (1024 the stub's better p99.99)
# and aeron-ipc's threading (no difference): the sweep, then the
# confirmation of capacity alone at TR_TUNE_CONFIRM_REPS reps, then the
# frozen choice: 1024 for mpsc, aeron-ipc's DEDICATED kept
trun tune-a 1 "" tune_arms=mpsc,aeron-ipc tune_knobs=capacity,aeron_threading FAKE_BETTER=capacity-1024 && trc=0 || trc=$?
t=$sb/a/bench/transport-tune-a
why=
[[ $trc == 0 && $(cat "$t/rc") == 0 ]] || why+=" exit $trc: $(grep -E 'ERROR|tune:' "$sb/run.out" | tail -3 | tr '\n' ' ');"
[[ $(grep -c '^set ' "$t/tune-sweep.tsv" 2>/dev/null) == 2 && $(grep -c . "$t/variants.txt" 2>/dev/null) == 5 ]] || why+=" sweep plan: $(cat "$t/tune-sweep.tsv" 2>/dev/null | tr '\n' ' ');"
grep -q '^set 1\.[0-9]* tune - a 1000000 1 - mpsc--capacity-4096,mpsc--capacity-64,mpsc--capacity-1024,aa$' "$t/tune-sweep.tsv" || why+=" mpsc's sweep set;"
grep -q '^set 1\.c1 tune - a 1000000 1 - mpsc--capacity-4096,mpsc--capacity-1024,aa$' "$t/tune-confirm.tsv" && [[ $(grep -c '^set ' "$t/tune-confirm.tsv") == 1 ]] ||
    why+=" confirmation: $(grep '^set' "$t/tune-confirm.tsv" | tr '\n' ' ');"
# the sweep's rounds at 1 rep, the confirmation's at 3
[[ $(awk -F'\t' '$2 ~ /\.c/ {print $4}' "$t/rounds.tsv" | sort -u | tr '\n' ' ') == "1 2 3 " && $(awk -F'\t' '$2 !~ /\.c/ {print $4}' "$t/rounds.tsv" | sort -u) == 1 ]] ||
    why+=" rounds: $(cut -f2,4 "$t/rounds.tsv" | tr '\n' ' ');"
grep -q 'mpsc_lat --role local .*--capacity 1024 .*--label 1\.c1:mpsc--capacity-1024@STREAMS=1 ' "$sb/runs" &&
    grep -q 'aeron_lat --role local .*--driver-busy-threads 1 .*--label 1\.[0-9]*:aeron-ipc--aeron_threading-SHARED@' "$sb/runs" &&
    grep -q 'vm.sh driver kernel-busy@AERON_TERM_BUFFER_LENGTH=1048576@AERON_IPC_TERM_BUFFER_LENGTH=1048576@AERON_THREADING_MODE=SHARED$' "$sb/calls" ||
    why+=" the variants' flags and driver mode: $(grep -m2 -E 'SHARED|capacity-1024' "$sb/runs" | tr '\n' ' ');"
grep -q '^TR_ARM_KNOBS="aeron-ipc:aeron_threading=DEDICATED mpsc:capacity=1024"$' "$t/tuned.env" &&
    grep -q '^# changed mpsc capacity: 4096 -> 1024: p99.99 better at ' "$t/tuned.env" && grep -q '^# kept aeron-ipc aeron_threading DEDICATED: ' "$t/tuned.env" &&
    grep -q '^TR_TUNED_SWEPT="aeron_threading:aeron-ipc capacity:mpsc"$' "$t/tuned.env" && grep -q '^TR_TUNED_RUN="20261025-120000"$' "$t/tuned.env" ||
    why+=" frozen: $(tr '\n' ' ' <"$t/tuned.env" 2>/dev/null);"
grep -q '| mpsc | capacity | 4096 |' "$t/tune-report.md" 2>/dev/null || why+=" no tune-report.md row;"
# the tuning session's own fairness: each round's variants differ in their
# one knob, every one of them swept for it; every max labelled
fv=$(python3 "$cloud/fairness.py" "$t" 2>&1)
! grep -qE '^(shared-knob|settings|max|aa|order):' <<<"$fv" || why+=" fairness: $(grep -E '^(shared-knob|settings|max|aa|order):' <<<"$fv" | head -3 | tr '\n' ' ');"
grep -q '^sweep.knob.capacity=mpsc--capacity-1024,mpsc--capacity-4096,mpsc--capacity-64$' "$t/transport-settings.env" || why+=" sweep line: $(grep '^sweep' "$t/transport-settings.env" | tr '\n' ' ');"
# the frozen section into a settings file, in place of the empty one; the
# comparison session's flags then the frozen value's, its collector's sweep line
cp "$cloud/transport-settings.env" "$tmp/settings.env"
python3 "$cloud/transport_tune.py" apply "$tmp/settings.env" "$t/tuned.env" >/dev/null || why+=" apply failed;"
[[ $(grep -c '^# ==== tuned:' "$tmp/settings.env") == 1 ]] && grep -q '^TR_ARM_KNOBS="aeron-ipc:aeron_threading=DEDICATED mpsc:capacity=1024"$' "$tmp/settings.env" &&
    ! grep -q '^TR_ARM_KNOBS=$' "$tmp/settings.env" || why+=" applied: $(grep '^TR_' "$tmp/settings.env" | tail -4 | tr '\n' ' ');"
got=$(
    . "$cloud/transport.sh"
    . "$tmp/settings.env"
    dir=$tmp arm=mpsc bin=mpsc_lat setup=none args=()
    echo "$(tr_bin_args mpsc_lat local x y)"
    arm=aeron-ipc bin=aeron_lat setup=kernel args=(--mode ipc)
    echo "$(tr_driver_mode)"
)
[[ $got == "--capacity 1024
kernel-busy$(. "$cloud/transport-settings.env"; echo "$TR_AERON_MODE_SUFFIX")" ]] || why+=" the tuned flags: $got;"
[[ -z $why ]] && pass "transport.sh run tune-a (LAB_TUNE=1): the sweep of each arm and knob (its current value first and its A/A), the confirmation of the value that beat it at 3 reps, the frozen choice with its evidence (tuned.env, tune-report.md), fairness's shared-knob rule clean on the variants, the section applied and its values the comparison's flags" ||
    fail "transport.sh run tune:$why"

# the socket arms' data-NIC interrupt deferral swept (TR_SWEEP_NAPI_DEFER),
# cut down to mio-udp: its default 0 first, then 2; each variant's points on
# its own value (0/0, or 2/200000: transport-net's TR_OVERRIDES), read back,
# the variant at 2 accepted as swept (shared-knob, settings and napi clean)
trun tune-a 1 "" tune_arms=mio-udp tune_knobs=napi_defer && trc=0 || trc=$?
t=$sb/a/bench/transport-tune-a
why=
[[ $trc == 0 ]] || why+=" exit $trc: $(grep -E 'ERROR|tune:' "$sb/run.out" | tail -3 | tr '\n' ' ');"
grep -q '^mio-udp--napi_defer-0 mio-udp napi_defer 0 1 ' "$t/variants.txt" 2>/dev/null && grep -q '^mio-udp--napi_defer-2 mio-udp napi_defer 2 0 ' "$t/variants.txt" ||
    why+=" variants: $(tr '\n' ' ' <"$t/variants.txt" 2>/dev/null);"
m0=$(grep -l '^arm=mio-udp--napi_defer-0$' "$t"/manifest/*.env 2>/dev/null | head -1)
m2=$(grep -l '^arm=mio-udp--napi_defer-2$' "$t"/manifest/*.env 2>/dev/null | head -1)
[[ -n $m0 && -n $m2 ]] && grep -q '^napi_a=enP1:0/0,enP2:0/0$' "$m0" && grep -q '^napi_b=enP1:0/0,enP2:0/0$' "$m0" &&
    grep -q '^napi_a=enP1:2/200000,enP2:2/200000$' "$m2" && grep -q '^napi_defer_b=2$' "$m2" && grep -q '^knob.napi_defer=2$' "$m2" ||
    why+=" the variants' NICs: $(grep -hE '^(arm|napi)' "$m0" "$m2" 2>/dev/null | tr '\n' ' ');"
fv=$(python3 "$cloud/fairness.py" "$t" 2>&1)
! grep -qE '^(shared-knob|settings|napi):' <<<"$fv" || why+=" fairness: $(grep -E '^(shared-knob|settings|napi):' <<<"$fv" | head -3 | tr '\n' ' ');"
[[ -z $why ]] && pass "transport.sh run tune-a: the socket arms' data-NIC interrupt deferral swept (0, the default, and 2), each variant's NICs at its own value and read back, fairness clean on the variants" ||
    fail "transport.sh run tune, NIC deferral:$why"

if kill -0 "$bystander" 2>/dev/null; then
    pass "transport.sh run: its exit stops only the bench binaries it started (by their recorded PIDs), never another process of the same name"
else
    fail "transport.sh run stopped a bystander aeron_lat it never started"
fi
kill "$bystander" 2>/dev/null || true

# the hooks with the neutral fixture: the arms file's set in pair a's
# session, its binary built by the tree's lab-build.sh into ~/extra/bin
cp "$sb/bin/stub" "$fx/src/fixture_lat"
got=$(
    . "$cloud/transport.sh"
    reps=5
    echo "check: $(tr_check a "$fx/arms.tsv" 2>&1)"
    tr_arms_file=$fx/arms.tsv
    echo "arm: $(tr_plan arm fixture-echo)"
)
# pair a at one rep with the arms file; each host's ~/extra as lib.sh leaves
# it, built by the tree's own lab-build.sh (as vm.sh transport-build runs it)
rm -rf "$sb/a" "$sb/b" "$tmp/home-a" "$tmp/home-b"
mkhost a
mkhost b
for h in a b; do
    cp -R "$fx/src/." "$sb/$h/extra/"
    mkdir -p "$sb/$h/extra/bin"
    (cd "$sb/$h/extra" && bash lab-build.sh "$sb/$h/extra/bin")
done
cp "$fx/arms.tsv" "$sb/a/bench/arms.tsv"
# trun makes the homes anew: these built ones instead
cp -R "$sb/a" "$tmp/home-a"
cp -R "$sb/b" "$tmp/home-b"
mkhost() { cp -R "$tmp/home-$1" "$sb/$1"; }
# shellcheck disable=SC2016
trun a 1 '$HOME/bench/arms.tsv' && frc=0 || frc=$?
res=$sb/a/bench/transport-a/results.csv
rows="rc $frc rows $(grep -c ',fx:' "$res" || true) runs $(grep -c '^a fixture_lat --role ping .*--local 10.20.1.4:22000 --peer 10.20.1.7:22000 --busy-read-us 50 --rcvbuf 2097152 --sndbuf 2097152 --nodelay --quickack-per-read --streams .*--flavour plain$' "$sb/runs" || true) aa $(grep -c ',fx:aa@' "$res" || true)"
same "LAB_ARMS_FILE and LAB_EXTRA_SRC with a neutral fixture: its arms and sets checked with the matrix, its binary built by lab-build.sh and found on the VMs' PATH, its rows recorded, its A/A arm run" "check: 550
arm: fixture_lat sockets two --flavour plain
rc 0 rows 8 runs 8 aa 4" "$got
$rows"

# an arm on each host's other data NIC (--nics alt, the runner's: not passed
# on; tcp_lat's ping binding its source there), and an arms-file binary
# given the other NICs' addresses ({own_alt}, {peer_alt}) beside its own
cat >"$tmp/home-a/bench/arms-alt.tsv" <<'ARMS'
arm  fixture-two  fixture_lat  sockets  two  --tcp-local {own_alt} --tcp-peer {peer_alt} --flavour plain
arm  tcp-other    tcp_lat      sockets  two  --impl mio --nics alt
set  fxn          a    -  ab  1000  1  -  fixture-two,tcp-other
ARMS
# shellcheck disable=SC2016
trun a 1 '$HOME/bench/arms-alt.tsv' && frc=0 || frc=$?
why=
((frc == 0)) || why+=" exit $frc: $(tail -3 "$sb/run.out" | tr '\n' ' ');"
grep -q '^a fixture_lat --role ping .*--local 10.20.1.4:22000 --peer 10.20.1.7:22000 .*--tcp-local 10.20.1.5 --tcp-peer 10.20.1.6 --flavour plain$' "$sb/runs" || why+=" ping's other NICs;"
grep -q '^b fixture_lat --role pong .*--local 10.20.1.7:22000 --peer 10.20.1.4:22000 .*--tcp-local 10.20.1.6 --tcp-peer 10.20.1.5 --flavour plain$' "$sb/runs" || why+=" pong's other NICs;"
grep -q '^a tcp_lat --role ping .*--source 10.20.1.5 --connect 10.20.1.6:22000 .*--impl mio$' "$sb/runs" || why+=" tcp ping on the other NICs;"
grep -q '^b tcp_lat --role pong .*--listen 10.20.1.6:22000 .*--impl mio$' "$sb/runs" || why+=" tcp pong on the other NICs;"
! grep -qE -- '--nics|\{(own|peer)_alt\}' "$sb/runs" || why+=" the runner's own arguments passed on;"
[[ -z $why ]] && pass "transport.sh run: --nics alt puts an arm on each host's other data NIC (tcp_lat's ping bound to its source there), {own_alt}/{peer_alt} give an arms-file binary those addresses; neither passed on" ||
    fail "transport.sh run, the other data NICs:$why"

# the Onload smoke (transport-aws-onload-smoke.tsv: every Onload arm beside
# kernel arms) on the fake hosts: each socket arm's binary under the onload
# launcher with the profile and TR_ONLOAD_ENV, --busy-read-us 0, no TCP
# timers; aeron_lat never under it, its media driver started by `driver
# onload@...`; onload-up before each other Onload arm and `stop` after every
# one; each Onload point's manifest with its command, its build and its stacks
# on both hosts, its NICs' deferral read back 0/0; fairness.py clean; then a
# host without the arm's stack mid-window: in the manifest, logged, and
# fairness.py's onload rule
cp "$cloud/transport-aws-onload-smoke.tsv" "$tmp/onload-smoke.tsv"
TRUN_SETS=$tmp/onload-smoke.tsv trun a 1 "" && orc=0 || orc=$?
d=$sb/a/bench/transport-a
why=
[[ $orc == 0 && $(cat "$d/rc") == 0 ]] || why+=" exit $orc: $(tail -3 "$sb/run.out" | tr '\n' ' ');"
n=$(TRUN_SETS= plan count a 1 "$matrix:arms" "$tmp/onload-smoke.tsv")
[[ $(grep -c . "$d/results.csv") == "$n" ]] || why+=" $(grep -c . "$d/results.csv") rows, not $n;"
for b in tcp_lat udp_lat; do
    grep -q "^a onload --profile=latency $b --role ping .* | EF_AF_XDP_ZEROCOPY=0 EF_NO_FAIL=0 EF_RFC_RTO_MIN=8 $" "$sb/onload" &&
        grep -q "^b onload --profile=latency $b --role pong .* | EF_AF_XDP_ZEROCOPY=0 EF_NO_FAIL=0 EF_RFC_RTO_MIN=8 $" "$sb/onload" || why+=" $b not under onload with its settings;"
done
! grep -q ' aeron_lat ' "$sb/onload" || why+=" aeron_lat under onload;"
! grep -E '^. onload .* (mio-tcp|aeron-rel)@' "$sb/onload" | grep -q -- '--label ot:mio-tcp@\|--label ou:aeron-rel@' || why+=" a kernel arm under onload;"
# 4 Onload socket arms and the 2 A/A arms repeating them, 2 loads, ping and pong
[[ $(grep -c . "$sb/onload") == $((6 * 2 * 2)) ]] || why+=" $(grep -c . "$sb/onload") runs under onload, not 24;"
! grep -E '^a tcp_lat --role ping .*--label ot:onload-' "$sb/runs" | grep -qv -- '--busy-read-us 0 ' || why+=" an Onload TCP ping with kernel busy poll;"
! grep -E '^. udp_lat .*--label ou:onload-' "$sb/runs" | grep -qv -- '--busy-read-us 0 ' || why+=" an Onload UDP run with kernel busy poll;"
grep -q '^a tcp_lat --role ping .*--busy-read-us 50 .*--label ot:mio-tcp@' "$sb/runs" || why+=" the kernel TCP arm without its busy poll;"
for h in a b; do
    grep -q "^$h vm.sh driver onload@CONDUCTOR=own@" "$sb/calls" || why+=" host $h: no Onload media driver;"
    [[ $(grep -c "^$h vm.sh onload-up$" "$sb/calls") == 6 ]] || why+=" host $h: $(grep -c "^$h vm.sh onload-up$" "$sb/calls") onload-ups, not 6;"
    grep -q "^$h vm.sh onload-info$" "$sb/calls" || why+=" host $h: its Onload build not read;"
    # before every point of an Onload socket arm (4 arms and 2 A/A, 2 loads each), never an Aeron one's
    [[ $(grep -c "^$h vm.sh onload-idle$" "$sb/calls") == 12 ]] || why+=" host $h: $(grep -c "^$h vm.sh onload-idle$" "$sb/calls") onload-idle checks, not 12;"
    # each Onload arm's set-up stopped after it (onload-up, then the next call a stop)
    awk -v h="$h" '$1 == h && $3 == "onload-up" {up = 1; next} $1 == h && up && $3 == "stop" {up = 0} END {exit up}' "$sb/calls" || why+=" host $h: an onload-up never stopped;"
done
for m in $(grep -l '^setup=onload$' "$d"/manifest/*.env); do
    grep -q '^onload_cmd=env EF_AF_XDP_ZEROCOPY=0 EF_NO_FAIL=0 EF_RFC_RTO_MIN=8 onload --profile=latency$' "$m" &&
        grep -q '^onload_a=ok 1 stack(s) of ' "$m" && grep -q '^onload_b=ok 1 stack(s) of ' "$m" &&
        grep -q '^onload_build_a=onload a94512556626 ' "$m" && grep -q '^knob.napi_defer=0$' "$m" && grep -q '^napi_a=enP1:0/0,enP2:0/0$' "$m" &&
        ! grep -q '^knob.busy_read_us' "$m" || why+=" $(grep '^mode=' "$m"): $(grep -E '^(onload|knob|napi_)' "$m" | tr '\n' ' ');"
done
m=$(grep -l '^arm=onload-aeron-rel$' "$d"/manifest/*.env | head -1)
grep -q '^onload_under=media_driver$' "$m" && grep -q '^onload_a=ok 1 stack(s) of media_driver' "$m" && grep -q '^driver_mode=onload@' "$m" || why+=" the Onload Aeron point: $(grep -E '^(onload|driver)' "$m" | tr '\n' ' ');"
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1) || why+=" fairness: $(head -5 <<<"$fv" | tr '\n' ' ');"
TRUN_SETS=$tmp/onload-smoke.tsv trun a 1 "" FAKE_ONLOAD_NOSTACK=b || true
m=$(grep -l '^mode=onload-eio-tcp@STREAMS=1$' "$d"/manifest/*.env | head -1)
grep -q '^onload_b=FAIL no Onload stack of tcp_lat' "$m" && grep -q "WARNING: point .*: host b's Onload stacks: FAIL" "$sb/run.out" || why+=" no stack not recorded: $(grep '^onload_b' "$m");"
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1 || true)
grep -q "^onload: ot: .*onload-eio-tcp@STREAMS=1.*: b's Onload stack mid-window: FAIL no Onload stack of tcp_lat" <<<"$fv" || why+=" fairness's onload rule: $(grep -m2 '^onload' <<<"$fv");"
! grep -E '^[a-z-]+:' <<<"$fv" | grep -qv '^onload:' || why+=" other violations: $(grep -E '^[a-z-]+:' <<<"$fv" | grep -v '^onload:' | head -2 | tr '\n' ' ');"
[[ -z $why ]] && pass "transport.sh run, setup onload: each socket arm's binary under onload (profile, TR_ONLOAD_ENV), no kernel busy poll or TCP timers; Aeron's media driver under onload instead; onload-up and stop around each arm, no stack left before each socket point (onload-idle); each point's command, build, stacks and NIC deferral in its manifest; fairness clean, and its onload rule flags a host without the arm's stack" ||
    fail "transport.sh run, setup onload:$why"

# the hand-off session (transport-aws-aeron-handoff.tsv) on the fake hosts:
# each arm's driver mode set up on both hosts (DRIVER=embedded: vm.sh starts
# none), aeron_lat's --driver and busy threads by arm, an embedded driver's
# binary under sudo setpriv with the driver's environment sourced (no other
# arm's), the knobs and driver mode in each manifest, fairness.py clean
# (the declared rule takes each arm as its twin's but for its knob, the
# invoker's through its chain); then an emb point whose driver mode differs
# beyond DRIVER: the declared rule flags it
# (its kernel set k too, which the file leaves commented out)
sed 's/^#set k   /set k    /' "$cloud/transport-aws-aeron-handoff.tsv" >"$tmp/handoff.tsv"
grep -q '^set k ' "$tmp/handoff.tsv" || fail "transport-aws-aeron-handoff.tsv: no set k to uncomment"
TRUN_SETS=$tmp/handoff.tsv trun a 1 "" && hrc=0 || hrc=$?
d=$sb/a/bench/transport-a
why=
[[ $hrc == 0 && $(cat "$d/rc") == 0 ]] || why+=" exit $hrc: $(tail -3 "$sb/run.out" | tr '\n' ' ');"
n=$(TRUN_SETS= plan count a 1 "$matrix:arms" "$tmp/handoff.tsv")
[[ $(grep -c . "$d/results.csv") == "$n" ]] || why+=" $(grep -c . "$d/results.csv") rows, not $n;"
for h in a b; do
    for m in 'dpdk@CONDUCTOR=own@.*_IDLE_STRATEGY=noop' 'dpdk@CONDUCTOR=own@.*_IDLE_STRATEGY=noop@DRIVER=embedded' 'dpdk@CONDUCTOR=own@.*@AERON_THREADING_MODE=SHARED_NETWORK' \
        'dpdk@AERON_TERM_BUFFER_LENGTH=[0-9]*@AERON_IPC_TERM_BUFFER_LENGTH=[0-9]*@AERON_THREADING_MODE=SHARED' 'dpdk@AERON_TERM_BUFFER_LENGTH=[0-9]*@AERON_IPC_TERM_BUFFER_LENGTH=[0-9]*@AERON_THREADING_MODE=INVOKER@DRIVER=embedded' \
        'kernel-busy@CONDUCTOR=own@.*@NAPI_DEFER=0/0@DRIVER=embedded' 'kernel-busy@AERON_TERM_BUFFER_LENGTH=[0-9]*@AERON_IPC_TERM_BUFFER_LENGTH=[0-9]*@NAPI_DEFER=0/0@AERON_THREADING_MODE=INVOKER@DRIVER=embedded'; do
        grep -q "^$h vm.sh driver $m$" "$sb/calls" || why+=" host $h: no driver $m;"
    done
done
for e in 'dpdk-aeron-rel@|external --driver-busy-threads 3' 'dpdk-aeron-rel-emb@|embedded --driver-busy-threads 3' 'dpdk-aeron-rel-shnet@|external --driver-busy-threads 2' \
    'dpdk-aeron-rel-shared@|external --driver-busy-threads 1' 'dpdk-aeron-rel-invoker@|invoker --driver-busy-threads 0' 'aeron-rel-emb@|embedded --driver-busy-threads 3' \
    'aeron-rel-invoker@|invoker --driver-busy-threads 0'; do
    for r in 'a aeron_lat --role ping' 'b aeron_lat --role pong'; do
        grep "^$r " "$sb/runs" | grep -- ":${e%|*}" | grep -q -- "--driver ${e#*|} " || why+=" ${r%% *} ${e%|*}: not --driver ${e#*|};"
    done
done
# an embedded driver's runs (emb and invoker, h's 3 loads and k's 1, ping and
# pong) with the driver's environment and under sudo setpriv; no other run
emb=$(grep -cE -- '--driver (embedded|invoker) ' "$sb/runs")
[[ $emb == 16 ]] || why+=" $emb embedded runs, not 16;"
[[ $(grep -cE -- '--driver (embedded|invoker) .*\[driver.env (dpdk|kernel-busy)@.*@DRIVER=embedded\]$' "$sb/runs") == 16 ]] || why+=" an embedded run without its driver.env;"
! grep -- '--driver external ' "$sb/runs" | grep -q 'driver.env' || why+=" an external driver's run sourced driver.env;"
[[ $(grep -c '^. sudo -n setpriv .*--ambient-caps=+sys_admin,+dac_override,+ipc_lock,+sys_rawio sh -c ' "$sb/sudo" 2>/dev/null) == 16 ]] ||
    why+=" $(grep -c . "$sb/sudo" 2>/dev/null) runs under sudo setpriv, not 16;"
for m in "$d"/manifest/*.env; do
    a=$(sed -n 's/^arm=//p' "$m")
    case $a in
    *-invoker) want='aeron_threading=INVOKER aeron_driver=embedded' ;;
    *-emb) want='aeron_threading=DEDICATED aeron_driver=embedded' ;;
    *-shnet) want='aeron_threading=SHARED_NETWORK aeron_driver=external' ;;
    *-shared) want='aeron_threading=SHARED aeron_driver=external' ;;
    *) want='aeron_threading=DEDICATED aeron_driver=external' ;;
    esac
    [[ "$(grep -E '^knob\.aeron_(threading|driver)=' "$m" | sed 's/^knob\.//' | paste -sd' ' -)" == "$want" ]] || why+=" $a: knobs $(grep -E '^knob.aeron_(threading|driver)' "$m" | tr '\n' ' ');"
    grep -q '^driver_mode=' "$m" || why+=" $a: no driver mode;"
done
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1) || why+=" fairness: $(head -5 <<<"$fv" | tr '\n' ' ');"
grep -q '^declared\.dpdk-aeron-rel-invoker=dpdk-aeron-rel-emb:knob\.aeron_threading$' "$d/transport-settings.env" ||
    why+=" the invoker's declaration not collected;"
m=$(grep -l '^arm=dpdk-aeron-rel-emb$' "$d"/manifest/*.env | head -1)
sed -i.bak 's/^\(driver_mode=.*\)@DRIVER=embedded$/\1@RUSTERON_DPDK_BURST_SIZE=32@DRIVER=embedded/' "$m" && rm -f "$m.bak"
python3 "$cloud/transport_collect.py" "$d" >/dev/null 2>&1 || true
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1 || true)
grep -q "^declared: h: .*dpdk-aeron-rel-emb and its twin dpdk-aeron-rel differ in media driver mode" <<<"$fv" || why+=" a driver mode beyond DRIVER not flagged: $(grep -m2 '^declared' <<<"$fv");"
[[ -z $why ]] && pass "transport.sh run, the hand-off session: each arm's driver mode on both hosts (DRIVER=embedded: no process), aeron_lat's --driver and busy threads by arm, an embedded driver's binary with driver.env under sudo setpriv (no other), knobs and driver mode in every manifest; fairness.py clean (each declared twin, the invoker's through -emb), and a driver mode differing beyond DRIVER flagged" ||
    fail "transport.sh run, the hand-off session:$why"

# The stage-trace session (transport-aws-dpdk-trace.tsv): the traced arms'
# drivers asked for md-dpdk's stage trace (and the DPDK one its capture
# window), the untraced arms' not; aeron_lat --stage-trace on the traced
# arms' runs alone, --capture-ms never (the runner's); each traced point's
# driver files fetched from both hosts, the capture on an arm's first point
# alone; fairness.py clean, and transport_stages.py run over the session
TRUN_SETS=$cloud/transport-aws-dpdk-trace.tsv trun a 1 "" && src=0 || src=$?
d=$sb/a/bench/transport-a
why=
[[ $src == 0 && $(cat "$d/rc") == 0 ]] || why+=" exit $src: $(tail -3 "$sb/run.out" | tr '\n' ' ');"
n=$(TRUN_SETS= plan count a 1 "$matrix:arms" "$cloud/transport-aws-dpdk-trace.tsv")
[[ $n == 12 && $(grep -c . "$d/results.csv") == "$n" ]] || why+=" $(grep -c . "$d/results.csv") rows, plan $n (not 12);"
for h in a b; do
    grep -q "^$h vm.sh driver dpdk@CONDUCTOR=own@.*_IDLE_STRATEGY=noop@RUSTERON_DPDK_STAGE_TRACE=64@RUSTERON_DPDK_CAPTURE_WINDOW_MS=10000:1000$" "$sb/calls" || why+=" host $h: no traced DPDK driver;"
    grep -q "^$h vm.sh driver kernel-busy@CONDUCTOR=own@.*@NAPI_DEFER=0/0@RUSTERON_DPDK_STAGE_TRACE=64$" "$sb/calls" || why+=" host $h: no traced kernel driver;"
    grep -q "^$h vm.sh driver dpdk@CONDUCTOR=own@.*_IDLE_STRATEGY=noop$" "$sb/calls" || why+=" host $h: no untraced DPDK driver;"
done
[[ $(grep -c -- '--stage-trace 64' "$sb/runs") == 12 ]] || why+=" $(grep -c -- '--stage-trace 64' "$sb/runs") runs with --stage-trace, not 12 (6 traced points, ping and pong);"
! grep -- '--stage-trace' "$sb/runs" | grep -qv -- '-trace@\|-trace ' || why+=" an untraced arm's run with --stage-trace;"
! grep -q -- '--capture-ms' "$sb/runs" || why+=" --capture-ms passed to a binary;"
for p in "$d"/points/*; do
    pid=${p##*/}
    case $pid in
    *-trace*) for h in a b; do [[ -s $p/stages-driver-$h.csv.gz ]] || why+=" $pid: no stages-driver-$h;"; done ;;
    *) ! ls "$p"/stages-driver-* >/dev/null 2>&1 || why+=" $pid: an untraced point's driver stages;" ;;
    esac
    case $pid in
    t1-*dpdk-aeron-rel-trace*-10000 | t32-*dpdk-aeron-rel-trace*-50000)
        for h in a b; do [[ -s $p/capture-$h.pcapng.gz ]] || why+=" $pid: no capture-$h;"; done ;;
    *) ! ls "$p"/capture-* >/dev/null 2>&1 || why+=" $pid: a capture past the arm's first point;" ;;
    esac
done
[[ $(ls "$d"/points/*/0/stages-app.csv "$d"/points/*/pong/stages-app.csv 2>/dev/null | wc -l | tr -d ' ') == 12 ]] || why+=" the binaries' stages files not kept;"
grep -q '^args=--mode udp --reliable true --stage-trace 64$' "$d"/manifest/t1-r1-dpdk-aeron-rel-trace*-10000.env || why+=" the traced arm's manifest args;"
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1) || why+=" fairness: $(head -5 <<<"$fv" | tr '\n' ' ');"
sv=$(python3 "$cloud/transport_stages.py" run "$d" 2>&1) || why+=" transport_stages.py run: $(tail -3 <<<"$sv" | tr '\n' ' ');"
[[ -s $d/stages.md && $(grep -c '^### ' "$d/stages.md") == 6 ]] || why+=" stages.md: $(grep -c '^### ' "$d/stages.md" 2>/dev/null) points, not 6;"
[[ -z $why ]] && pass "transport.sh run, the stage-trace session: the traced arms' drivers traced (the DPDK one's capture window too), aeron_lat --stage-trace on their runs alone, --capture-ms the runner's, each traced point's driver files from both hosts and the capture on an arm's first point alone; fairness.py clean, transport_stages.py run over it" ||
    fail "transport.sh run, the stage-trace session:$why"

# the mixed-load sessions (transport-aws-aeron-mixed.tsv) on the fake hosts,
# each pair (a the kernel's, b DPDK's): every point a row; each mixed arm's
# driver mode with the duty-cycle thresholds (each threading's own keys
# apart), --load mixed --hot-rate and two CPUs a host (hot, background); the
# NIC counters read around every point and a mixed one's collected (a
# DPDK host's the binary's own); the raw DPDK floor on dpdk-raw's set-up with
# the driver's environment under sudo setpriv; fairness.py clean, its
# mixed-load rules then flagging a fan-out subscription short; analyse.py's
# mixed table and checklist
for pair in a b; do
    TRUN_SETS=$cloud/transport-aws-aeron-mixed.tsv trun "$pair" 1 "" && mrc=0 || mrc=$?
    d=$sb/a/bench/transport-$pair
    why=
    [[ $mrc == 0 && $(cat "$d/rc") == 0 ]] || why+=" exit $mrc: $(tail -3 "$sb/run.out" | tr '\n' ' ');"
    n=$(plan count "$pair" 1 "$matrix:arms" "$cloud/transport-aws-aeron-mixed.tsv")
    [[ $n == 27 && $(grep -c . "$d/results.csv") == "$n" ]] || why+=" $(grep -c . "$d/results.csv") rows, $n planned (27);"
    stack=kernel-busy
    [[ $pair == b ]] && stack=dpdk
    for h in a b; do
        [[ $(grep -c "^$h vm.sh driver $stack@.*@AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD=10000@AERON_DRIVER_SENDER_CYCLE_THRESHOLD=10000@AERON_DRIVER_RECEIVER_CYCLE_THRESHOLD=10000" "$sb/calls") -ge 5 ]] ||
            why+=" host $h: no driver mode with the thresholds;"
        grep -q "^$h vm.sh driver dpdk-raw$" "$sb/calls" || why+=" host $h: no dpdk-raw set-up;"
    done
    for r in 'a aeron_lat --role ping' 'b aeron_lat --role pong'; do
        [[ $(grep "^$r " "$sb/runs" | grep -c -- '--cpus 10,12 .*--load mixed --hot-rate 10000') == 24 ]] ||
            why+=" ${r%% *}: not 24 mixed runs on CPUs 10,12: $(grep "^$r " "$sb/runs" | head -1);"
    done
    # the floor set's dpdk-raw (and on b its A/A)
    floors=1
    [[ $pair == b ]] && floors=2
    [[ $(grep -c "^b floor --role pong .*--endpoint .* --reply .*\[driver.env dpdk-raw\]$" "$sb/runs") == "$floors" ]] ||
        why+=" not $floors floor pongs with the driver's environment: $(grep floor "$sb/runs" | head -1);"
    grep -q 'setpriv .*floor --role ping' "$sb/sudo" || why+=" the floor not under sudo setpriv;"
    mixed_pts=$(grep -l -- '--load mixed' "$d"/manifest/*.env | wc -l | tr -d ' ')
    all_pts=$(ls "$d"/manifest/*.env | wc -l | tr -d ' ')
    [[ $mixed_pts == 24 && $(ls "$d"/nicstats | wc -l | tr -d ' ') == $((all_pts * 4)) ]] ||
        why+=" $(ls "$d"/nicstats | wc -l) NIC snapshots for $all_pts points, $mixed_pts mixed (4 each);"
    src=$(python3 -c 'import glob, json, sys
for f in sorted(glob.glob(sys.argv[1] + "/*/manifest-*.json")):
    m = json.load(open(f))
    if (m.get("binary") or {}).get("settings", {}).get("load") == "mixed":
        print(" ".join(sorted({c["source"] for c in (m.get("nic_counters") or {}).values()})) or "none")' "$d" | sort -u | tr '\n' ' ')
    want=kernel
    [[ $pair == b ]] && want=dpdk
    [[ $src == "$want " ]] || why+=" NIC counters' sources $src, not $want;"
    w=kernel
    [[ $pair == b ]] && w=dpdk
    [[ $(grep -- "aeron_lat --role .*--load mixed" "$sb/runs" | grep -c -- "--wire $w ") == 48 ]] || why+=" not every mixed run given --wire $w;"
    # a mixed point traces the housekeeping and IRQ CPUs too (0 and 2 here)
    for h in a b; do
        [[ $(awk '$1 ~ /aeron-mix/ && ("," $2 ",") ~ /^,0,2,/' "$sb/$h/state/events-started" | wc -l | tr -d ' ') == 24 ]] ||
            why+=" host $h: mixed points' traced CPUs not with 0 and 2: $(grep -m1 aeron-mix "$sb/$h/state/events-started");"
    done
    fv=$(python3 "$cloud/fairness.py" "$d" 2>&1) || why+=" fairness: $(head -5 <<<"$fv" | tr '\n' ' ');"
    an=$(python3 "$cloud/analyse.py" --transport "$d" 2>&1) || why+=" analyse.py failed: $(tail -2 <<<"$an");"
    grep -q '^## Mixed load: hot stream beside a background mix' <<<"$an" || why+=" no mixed table;"
    [[ $(grep -c '^- \*\*' <<<"$an") == 6 ]] || why+=" $(grep -c '^- \*\*' <<<"$an") arms in the checklist, not 6 (five and the A/A);"
    grep -q 'floor-relative overhead: hot p50 .* − dpdk-raw floor\|floor-relative overhead: hot p50 .* − mio-udp floor' <<<"$an" || why+=" no floor-relative line;"
    [[ -z $why ]] && pass "transport.sh run, the mixed-load session, pair $pair: every point a row; the mixed arms' driver modes with the duty-cycle thresholds, --load mixed on CPUs 10,12 each end, --wire by set-up, the housekeeping and IRQ CPUs traced; NIC counters around every point, a mixed one's collected ($want); the raw DPDK floor on dpdk-raw with driver.env under sudo setpriv; fairness.py clean; analyse.py's mixed table, checklist and floor lines" ||
        fail "transport.sh run, the mixed-load session, pair $pair:$why"
done
TRUN_SETS=$cloud/transport-aws-aeron-mixed.tsv trun a 1 "" FAKE_FANOUT_SHORT=ka:aeron-mix-shared@STREAMS=32 || true
d=$sb/a/bench/transport-a
fv=$(python3 "$cloud/fairness.py" "$d" 2>&1 || true)
grep -q "^mixed-counts: ka: .*aeron-mix-shared@STREAMS=32.*: background stream 1 subscription 1 read" <<<"$fv" &&
    pass "fairness.py mixed-counts: a fan-out subscription that read one message short is flagged" ||
    fail "fairness.py mixed-counts: a short fan-out subscription not flagged: $(head -3 <<<"$fv")"

# transport_prepare's steps on the operator machine (lib.sh's helpers faked):
# the tree and the arms file to both VMs, the build, the map's boot line, a
# reboot, transport-tune on both, VM a's key
steps=$(
    . "$cloud/transport.sh"
    log() { :; }
    die() { echo "die $*"; exit 1; }
    out=$tmp/prep extra_src=$fx/src arms_file=$fx/arms.tsv A=1.1.1.1 B=2.2.2.2 user=u ssh_opts=()
    mkdir -p "$out"
    scp() { echo "scp ${*: -2}"; }
    on() { echo "on $1 $2"; }
    detached() { echo "detached $1"; }
    both() { echo "both $1"; }
    reboot_both() { echo reboot; }
    remote_access() { echo key; }
    transport_prepare
    tar -tf "$out/extra.tar" | grep -c 'lab-build.sh'
)
same "transport.sh transport_prepare: LAB_EXTRA_SRC and LAB_ARMS_FILE to both VMs, then the build, the map's boot line, a reboot, transport-tune, VM a's key" "scp $tmp/prep/extra.tar u@1.1.1.1:extra.tar
on 1.1.1.1 rm -rf extra && mkdir -p extra && tar -xf extra.tar -C extra && rm extra.tar
scp $fx/arms.tsv u@1.1.1.1:bench/arms.tsv
scp $tmp/prep/extra.tar u@2.2.2.2:extra.tar
on 2.2.2.2 rm -rf extra && mkdir -p extra && tar -xf extra.tar -C extra && rm extra.tar
scp $fx/arms.tsv u@2.2.2.2:bench/arms.tsv
detached transport-build
both transport-isolate
reboot
both transport-tune
key
1" "$steps"

for script in transport.sh azure/lab.sh azure/az.sh aws/lab.sh host.sh irqs.sh vm.sh modes.sh; do
    bash -n "$cloud/$script" && pass "bash -n $script" || fail "bash -n $script"
done
if command -v shellcheck >/dev/null; then
    shellcheck -S warning "$cloud/transport.sh" "$here/test-transport-runner.sh" && pass "shellcheck transport.sh test-transport-runner.sh" || fail "shellcheck"
fi

if ((failed)); then
    echo "test-transport-runner: FAILED" >&2
    exit 1
fi
echo "test-transport-runner: all passed"
