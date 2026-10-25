#!/usr/bin/env bash
# The transport latency suite (azure/lab.sh LAB_SUITE=transport): the matrix
# (transport-matrix.tsv, plus LAB_ARMS_FILE's rows) as rounds of points on
# the two VMs of one pair session (LAB_PAIR a or b). Each rep's round at a
# netem level runs the set's arms in an order drawn from its own seed (the
# session's seed and the round: transport_plan.py), the A/A arm (the set's
# first arm again, @AA=1) among them. Each point is one run of the arm's
# bench binary (transport-cli.md) at one load: for two-host arms its pong on
# host b, then its ping on host a; one-host arms --role local. The first load
# of an arm and stream count warms up 30 s, each later one 5 s (the switch).
# Around every point, on each host it uses: the IRQ counts (deltas on the
# isolated CPUs), a thread placement sample mid-window, and under netem the
# qdisc's counters, checked against the binomial interval of its loss. Each
# point gets a manifest (manifest/<point>.env, the binary's manifest.json as
# manifest/<point>.json); its --out directory goes to points/<point>/ and its
# rows (the rep in their first column) to results.csv.
#
#   transport.sh check <pair> [<arms file>]   the matrix and arms file parse, every mode a
#                         point runs passes modes.sh mode_check, and so do the Aeron arms'
#                         driver modes; prints the pair's point count (before any cloud call)
#   transport.sh run <pair>   on VM a (transport_remote starts it): reads
#                         ~/bench/transport-<pair>/env, writes results.csv, rounds.tsv,
#                         order.csv, manifest/, points/, logs/, failures.csv there, then rc
#
# Sourced (azure/lab.sh, the tests): functions only, among them
# transport_prepare and transport_remote, the operator machine's side.
#
# Durations (env, else the plan's): TRANSPORT_WARMUP_S (30), TRANSPORT_POINT_S
# (30), TRANSPORT_SWITCH_S (5), TRANSPORT_PONG_WAIT_S (2: the pong's time to bind
# before the ping starts); TRANSPORT_MAX_FAILURES (3): that many failed runs
# (an arm at a stream count) in a row stop the session.
set -uo pipefail

tr_here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=transport-settings.env
. "$tr_here/transport-settings.env"
# shellcheck source=modes.sh
. "$tr_here/modes.sh"

# tr_files [<arms file>]: the plan's files: the matrix, then the arms file
tr_files() { printf '%s\n' "$tr_here/transport-matrix.tsv" ${1:+"$1"}; }

# tr_plan <command> <args>...: transport_plan.py over the matrix and the
# session's arms file (tr_arms_file)
tr_plan() {
    local cmd=$1 files
    shift
    mapfile -t files < <(tr_files "${tr_arms_file:-}")
    python3 "$tr_here/transport_plan.py" "$cmd" "$@" "${files[@]}"
}

# tr_check <pair> [<arms file>]: 0 and the pair's point count, or 2 saying why
tr_check() {
    local pair=$1 m why n
    tr_arms_file=${2:-}
    [[ $pair == a || $pair == b ]] || { echo "LAB_PAIR $pair: a or b (the sessions of transport-matrix.tsv)" >&2; return 2; }
    [[ -z $tr_arms_file || -r $tr_arms_file ]] || { echo "LAB_ARMS_FILE $tr_arms_file: not a readable file" >&2; return 2; }
    TRANSPORT_ARMS=$(tr_plan arms) || return 2
    local modes
    modes=$(tr_plan modes "$pair") || return 2
    for m in $modes; do
        why=$(mode_check "$m" remote x86_64 azure '' 2>&1) || { echo "$why" >&2; return 2; }
    done
    for m in "kernel-busy$TR_AERON_MODE_SUFFIX" "dpdk$TR_AERON_MODE_SUFFIX"; do
        why=$(mode_check "$m" remote x86_64 azure '' 2>&1) || { echo "TR_AERON_MODE_SUFFIX: $why" >&2; return 2; }
    done
    # the bench ports against the wildcard ranges and Linux's default
    # ephemeral range (each host's own is checked at transport-tune)
    why=$(. "$tr_here/host.sh"; transport_ports_check "$TR_PORT_BASE" "$TR_PORT_SPAN" 32768 60999 2>&1) ||
        { echo "TR_PORT_BASE: $why" >&2; return 2; }
    n=$(tr_plan count "$pair" "${reps:-5}") || return 2
    ((n > 0)) || { echo "LAB_PAIR $pair: no point to run in $(tr_files "$tr_arms_file" | tr '\n' ' ')" >&2; return 2; }
    echo "$n"
}

# ---- the operator machine's side (azure/lab.sh, after lib.sh prepare_vms) ----

# transport_prepare: LAB_EXTRA_SRC's tree and LAB_ARMS_FILE on both VMs,
# the bench binaries built (vm.sh transport-build), the CPU map's boot line,
# a reboot into it, transport-tune on both (fail closed), VM a's key on both
transport_prepare() {
    local host
    if [[ -n ${extra_src:-} ]]; then
        log "LAB_EXTRA_SRC $extra_src: to ~/extra on both VMs"
        (cd "$extra_src" && COPYFILE_DISABLE=1 tar --no-xattrs --exclude .git --exclude target -cf - .) >"$out/extra.tar" ||
            die "cannot pack LAB_EXTRA_SRC $extra_src"
    fi
    for host in "$A" "$B"; do
        if [[ -n ${extra_src:-} ]]; then
            scp -q "${ssh_opts[@]}" "$out/extra.tar" "$user@$host:extra.tar" || die "copy of LAB_EXTRA_SRC to $host failed"
            on "$host" "rm -rf extra && mkdir -p extra && tar -xf extra.tar -C extra && rm extra.tar" || die "unpack of LAB_EXTRA_SRC on $host failed"
        fi
        if [[ -n ${arms_file:-} ]]; then
            scp -q "${ssh_opts[@]}" "$arms_file" "$user@$host:bench/arms.tsv" || die "copy of LAB_ARMS_FILE to $host failed"
        fi
    done
    log "transport: bench binaries"
    detached transport-build transport-build || die "transport-build failed"
    log "transport: CPU map and its boot line"
    both transport-isolate transport-isolate || die "transport-isolate failed"
    reboot_both
    both transport-tune transport-tune || die "transport-tune failed: a setting did not hold ($out/{a,b}-transport-tune.log)"
    remote_access
}

# transport_remote <pair>: the session on VM a (transport.sh run), detached
# there; polled until it ends (as long as the VMs live), its results so far
# fetched every 5 minutes, then its whole directory into $out
transport_remote() {
    local pair=$1 dir=bench/transport-$1 status= last= line i
    on "$A" "rm -rf $dir && mkdir -p $dir && cat >$dir/env" <<ENV || return 1
A=$a_private
B=$b_private
user=$user
pair=$pair
session=$stamp
seed=$seed
reps=$reps
vm_a=$vm_a
vm_b=$vm_b
commit=$(git -C "$repo" rev-parse HEAD 2>/dev/null)
dirty=$(git -C "$repo" status --porcelain 2>/dev/null | grep -c .)
arms_file=${arms_file:+\$HOME/bench/arms.tsv}
TRANSPORT_WARMUP_S=${TRANSPORT_WARMUP_S:-30}
TRANSPORT_POINT_S=${TRANSPORT_POINT_S:-30}
TRANSPORT_SWITCH_S=${TRANSPORT_SWITCH_S:-5}
ENV
    on "$A" "nohup bash ergon/md-dpdk/benches/cloud/transport.sh run $pair >$dir/run.log 2>&1 </dev/null &" || return 1
    log "transport pair $pair: running on VM a ($dir, seed $seed)"
    mkdir -p "$out/transport"
    for i in $(seq $((${max_hours:-10} * 360))); do
        sleep 10
        status=$(on "$A" "cat $dir/rc 2>/dev/null" 2>/dev/null) && [[ -n $status ]] && break
        ((i % 30 == 0)) && on "$A" "cat $dir/results.csv 2>/dev/null" >"$out/transport/results.partial.csv" 2>/dev/null
        line=$(on "$A" "grep '^\[' $dir/run.log | tail -1" 2>/dev/null) || continue
        [[ -n $line && $line != "$last" ]] && { echo "  a: $line" >&2; last=$line; }
    done
    on "$A" "tar -C $dir -cf - ." | tar -xf - -C "$out/transport" || log "WARNING: could not fetch $dir"
    [[ $status == 0 ]] || { log "transport pair $pair on VM a: exit ${status:-unknown} ($out/transport/run.log)"; tail -20 "$out/transport/run.log" >&2; return 1; }
}

# ---- VM a's side (transport.sh run) ----

# tr_host <a|b>: that host's private address
tr_host() { if [[ $1 == a ]]; then echo "$A"; else echo "$B"; fi; }

# tr_nic <a|b> <sender|receiver>: that host's data NIC address (vm.sh nics,
# read once a session into nics-<host>.txt)
tr_nic() { awk -v r="$2" '$1 == r {print $5}' "$dir/nics-$1.txt"; }

# tr_bin_path: the PATH a bench binary is found on, on either host
tr_bin_path='"$HOME/ergon/target/release:$HOME/extra/bin:$PATH"'

# tr_now: CLOCK_REALTIME in ns (a point's start and end in its manifest)
tr_now() {
    local t
    t=$(date +%s%N)
    [[ $t =~ ^[0-9]+$ ]] || t=$(python3 -c 'import time; print(time.time_ns())')
    echo "$t"
}

# tr_bin_args <binary> <role> <own ip> <peer ip>: the binary's addresses
# (transport-cli.md: base port TR_PORT_BASE, stream i on port + i; ping's
# endpoint the peer, its reply its own address; pong's the other way round)
# and its host settings, the same for every arm that runs it
# (transport-settings.env)
tr_bin_args() {
    local b=$1 role=$2 own=$3 peer=$4 port=$TR_PORT_BASE
    case $b in
    mpsc_lat) echo "--capacity $TR_MPSC_CAPACITY" ;;
    aeron_lat)
        local ep=$peer rp=$own
        [[ $role == pong ]] && ep=$own rp=$peer
        echo "--endpoint $ep:$port --reply $rp:$port --interface $own --aeron-dir $TR_AERON_DIR --driver external" \
            "--driver-busy-threads $TR_AERON_DRIVER_BUSY_THREADS --term-length $TR_TERM_LENGTH --mtu $TR_MTU"
        ;;
    udp_lat) echo "--local $own:$port --peer $peer:$port --busy-read-us $TR_BUSY_READ --rcvbuf $TR_SOCKBUF --sndbuf $TR_SOCKBUF" ;;
    tcp_lat)
        local a
        if [[ $role == pong ]]; then a="--listen $own:$port"; else a="--connect $peer:$port"; fi
        echo "$a --nodelay --quickack-per-read --rto-min-us $TR_TCP_RTO_MIN_US --delack-max-us $TR_TCP_DELACK_MAX_US --busy-read-us $TR_BUSY_READ --rcvbuf $TR_SOCKBUF --sndbuf $TR_SOCKBUF"
        ;;
    *) echo "--local $own:$port --peer $peer:$port" ;;
    esac
}

# tr_start <pid file> <out file> <binary> <args>...: the remote command that
# runs a bench binary started on the housekeeping CPU (its Aeron client
# conductor keeps that affinity; --cpus pins the busy threads), its PID
# recorded first, so the runner's EXIT trap stops it by PID, never by name
tr_start() {
    local pidf=$1 outf=$2
    shift 2
    echo "sh -c 'echo \$\$ >$pidf; exec taskset -c $hk env PATH=$tr_bin_path $*' >$outf"
}

# tr_netem_line <pid> <host>: the point's netem verdict on that host:
# "<offered>/<dropped> p=<pct> in [<lo>,<hi>] ok|FAIL" (transport_plan.py
# binom over the qdisc counters' growth, every data NIC summed), or none
tr_netem_line() {
    local pid=$1 h=$2 sent dropped verdict lo hi ok
    [[ $netem != - ]] || { echo none; return; }
    read -r sent dropped < <(awk 'FNR == NR {s[$1] = $2; d[$1] = $3; next} ($1 in s) {ts += $2 - s[$1]; td += $3 - d[$1]; n++}
        END {if (n) print ts + td, td; else print "x x"}' "$dir/netem/$pid-$h-before" "$dir/netem/$pid-$h-after" 2>/dev/null)
    if [[ ${sent:-x} == x ]]; then echo "unread FAIL"; return; fi
    verdict=$(python3 "$tr_here/transport_plan.py" binom "$sent" "$dropped" "$netem") || verdict="? ? FAIL"
    read -r lo hi ok <<<"$verdict"
    echo "$sent/$dropped p=$netem in [$lo,$hi] $ok"
}

# tr_snap <before|after> <pid>: each host the point uses: its IRQ counts and,
# under netem, its qdisc counters
tr_snap() {
    local h
    for h in "${run_hosts[@]}"; do
        on "$(tr_host "$h")" "$vm irqsnap" >"$dir/irq/$2-$h-$1" 2>/dev/null
        [[ $netem == - ]] || on "$(tr_host "$h")" "$vm netem stats" >"$dir/netem/$2-$h-$1" 2>/dev/null
    done
}

# tr_manifest <pid> <load> <warm-up s> <start ns> <end ns>: the point's
# manifest (manifest/<point>.env; the binary's own is manifest/<point>.json)
tr_manifest() {
    local pid=$1 h hot place nline
    {
        echo "point=$pid"
        echo "set=$sid"
        echo "rep=$rep"
        echo "round=$sid-r$rep-netem$netem"
        echo "arm=$arm"
        echo "mode=$mode"
        echo "streams=$n"
        echo "load=$2"
        echo "warmup_s=$3"
        echo "netem=$netem"
        echo "aa=$aa"
        echo "seed=$rseed"
        echo "order=$order"
        echo "position=$position"
        echo "session=$session"
        echo "pair=$pair"
        echo "vm_a=$vm_a"
        echo "vm_b=$vm_b"
        echo "bin=$bin"
        for h in "${run_hosts[@]}"; do echo "bin_sha256_$h=$(awk -v b="$bin" '$2 ~ "/" b "$" {print $1}' "$dir/bins-$h.txt")"; done
        echo "commit=$commit"
        echo "dirty=$dirty"
        echo "settings_sha256=$settings_sha"
        echo "start_ns=$4"
        echo "end_ns=$5"
        for h in a b; do
            if [[ " ${run_hosts[*]} " != *" $h "* ]]; then
                echo "irq_hot_$h=unused"
                echo "placement_$h=unused"
                echo "netem_$h=unused"
                continue
            fi
            hot=$(irqs_hot_delta "$dir/irq/$pid-$h-before" "$dir/irq/$pid-$h-after" "$iso" 2>/dev/null)
            echo "irq_hot_$h=${hot:-unread}"
            place=$(head -1 "$dir/samples/$pid-$h" 2>/dev/null)
            [[ $place == ok* || $place == FAIL* ]] || place="unread"
            echo "placement_$h=$place"
            nline=$(tr_netem_line "$pid" "$h")
            echo "netem_$h=$nline"
            [[ $nline != *FAIL ]] || log "WARNING: point $pid: host $h's netem counters off its loss: $nline"
            [[ $place == ok* ]] || log "WARNING: point $pid: host $h's thread placement: $place"
        done
        echo "hostcheck_a=hostcheck-a.txt"
        echo "hostcheck_b=hostcheck-b.txt"
    } >"$dir/manifest/$pid.env"
}

# tr_arm_up <setup> <host>...: the arm's host set-up on each: a media driver
# (kernel: kernel-busy's path, then transport-net check; dpdk), the kernel
# path (sockets: transport-net), or nothing; tr_arm_down: the driver stopped
tr_arm_up() {
    local setup=$1 h
    shift
    for h in "$@"; do
        case $setup in
        kernel) on "$(tr_host "$h")" "$vm driver kernel-busy$TR_AERON_MODE_SUFFIX && $vm transport-net check" ;;
        dpdk) on "$(tr_host "$h")" "$vm driver dpdk$TR_AERON_MODE_SUFFIX" ;;
        sockets) on "$(tr_host "$h")" "$vm transport-net" ;;
        none) true ;;
        esac >>"$dir/logs/setup-$h.log" 2>&1 || { log "host $h: $setup set-up failed ($dir/logs/setup-$h.log)"; return 1; }
    done
}
tr_arm_down() {
    local setup=$1 h
    shift
    [[ $setup == kernel || $setup == dpdk ]] || return 0
    for h in "$@"; do on "$(tr_host "$h")" "$vm stop" >>"$dir/logs/setup-$h.log" 2>&1 || log "host $h: vm.sh stop failed"; done
}

# tr_point <k>: one point at loads[k] (the round's variables: sid host rep
# netem rseed order position arm aa bin setup kind args n loads), with the
# snapshots around it, a placement sample half way through its window, its
# rows (rep in their first column) into results.csv and its --out
# directories into points/<point>/; 1 when it failed. A two-host arm's point
# is one run: its pong on b, its ping on a (round trips). A one-host arm's is
# two, one after the other: --role local (one way, the same TSC) and --role
# local --round-trip. The first point of an arm and stream count warms up
# TRANSPORT_WARMUP_S, each later one TRANSPORT_SWITCH_S (the new load,
# unrecorded).
tr_point() {
    local k=$1 load=${loads[$1]} warm pid rdir=bench/transport-$pair/run rc=0 meas log common pong= start end v runs=()
    warm=$TRANSPORT_SWITCH_S
    ((k == 0)) && warm=$TRANSPORT_WARMUP_S
    pid="$tag-$load"
    log=$dir/logs/$pid
    common="--streams $n --rate $load --warmup-s $warm --duration-s $TRANSPORT_POINT_S --size $TR_PAYLOAD --cpus $cpus"
    common+=" --label set$sid:$mode --spike-ns $TR_SPIKE_NS --stall-ns $TR_STALL_NS"
    for h in "${run_hosts[@]}"; do on "$(tr_host "$h")" "rm -rf $rdir && mkdir -p $rdir" || return 1; done
    if [[ $kind == two ]]; then
        meas=a
        # the pong first, on b; it has TRANSPORT_PONG_WAIT_S to bind (a simple
        # background command: the SSH session ends as it returns)
        on "$B" "nohup $(tr_start "$rdir/pong.pid" "$rdir/pong.out" "$bin --role pong $common --out $rdir/out-pong $(tr_bin_args "$bin" pong "$(tr_nic b receiver)" "$(tr_nic a sender)") ${args[*]}") 2>&1 </dev/null &" ||
            { log "point $pid: pong did not start on b"; return 1; }
        pong=1
        sleep "${TRANSPORT_PONG_WAIT_S:-2}"
        if ! on "$B" "kill -0 \$(cat $rdir/pong.pid)" 2>/dev/null; then
            on "$B" "cat $rdir/pong.out" >"$log.pong" 2>&1
            log "point $pid: pong exited before the ping started: $(tail -2 "$log.pong" | tr '\n' ' ')"
            return 1
        fi
        runs=("--role ping $(tr_bin_args "$bin" ping "$(tr_nic a sender)" "$(tr_nic b receiver)")")
    else
        meas=$host
        v=$(tr_bin_args "$bin" local "$(tr_nic "$host" sender)" "$(tr_nic "$host" receiver)")
        runs=("--role local $v" "--role local --round-trip $v")
    fi
    tr_snap before "$pid"
    (
        sleep "$(awk -v w="$warm" -v s="$TRANSPORT_POINT_S" 'BEGIN {print w + s / 2}')"
        for h in "${run_hosts[@]}"; do on "$(tr_host "$h")" "$vm sample" >"$dir/samples/$pid-$h" 2>/dev/null; done
    ) &
    local sampler=$!
    start=$(tr_now)
    : >"$log.out"
    : >"$log.err"
    for v in "${!runs[@]}"; do
        on "$(tr_host "$meas")" "$(tr_start "$rdir/run.pid" "$rdir/run$v.out" "$bin ${runs[$v]} $common --out $rdir/out$v ${args[*]}") 2>&1" >>"$log.out" 2>>"$log.err" || rc=$?
        on "$(tr_host "$meas")" "cat $rdir/run$v.out" >>"$log.out" 2>/dev/null
        ((rc == 0)) || break
    done
    end=$(tr_now)
    wait "$sampler" 2>/dev/null
    tr_snap after "$pid"
    if [[ -n $pong ]]; then
        on "$B" "kill -TERM \$(cat $rdir/pong.pid) 2>/dev/null; for _ in \$(seq 50); do kill -0 \$(cat $rdir/pong.pid) 2>/dev/null || break; sleep 0.1; done; kill -KILL \$(cat $rdir/pong.pid) 2>/dev/null; true" >/dev/null 2>&1
        on "$B" "cat $rdir/pong.out" >"$log.pong" 2>&1
    fi
    tr_manifest "$pid" "$load" "$warm" "$start" "$end"
    local got=0
    for v in "${!runs[@]}"; do
        mkdir -p "$dir/points/$pid/$v"
        on "$(tr_host "$meas")" "tar -C $rdir/out$v -cf - . 2>/dev/null" | tar -xf - -C "$dir/points/$pid/$v" 2>/dev/null
        [[ -f $dir/points/$pid/$v/manifest.json ]] && cp "$dir/points/$pid/$v/manifest.json" "$dir/manifest/$pid${v/#0/}.json"
        [[ -s $dir/points/$pid/$v/results.csv ]] && got=$((got + 1))
    done
    if ((rc != 0 || got != ${#runs[@]})); then
        log "point $pid: $bin exit $rc, $got of ${#runs[@]} runs with a row: $(tail -2 "$log.err" 2>/dev/null | tr '\n' ' ')"
        return 1
    fi
    for v in "${!runs[@]}"; do
        awk -F, -v OFS=, -v rep="$rep" '{$1 = rep; print}' "$dir/points/$pid/$v/results.csv" >>"$dir/results.csv"
    done
}

# tr_invoke: an arm at one stream count: each load's point in order; 1 when
# one failed (the rest still run)
tr_invoke() {
    local k rc=0 tag cpus roleargs
    mode="$arm@STREAMS=$n"
    [[ $netem == - ]] || mode+="@NETEM=$netem"
    ((aa)) && mode+="@AA=1"
    tag="$sid-r$rep-$(file_mode "$mode")"
    if [[ $kind == two ]]; then run_hosts=(a b); else run_hosts=("$host"); fi
    cpus=$(transport_clients "$map" "$setup")
    for k in "${!loads[@]}"; do tr_point "$k" || rc=1; done
    return $rc
}


# tr_netem_round <level> <host>...: the round's netem on each host (set and
# checked; - checks there is none); tr_netem_off: cleared and checked
tr_netem_round() {
    local level=$1 h
    shift
    for h in "$@"; do
        if [[ $level == - ]]; then
            on "$(tr_host "$h")" "$vm netem check none" >>"$dir/logs/netem-$h.log" 2>&1 || { log "host $h: a netem qdisc where the round has none"; return 1; }
        else
            on "$(tr_host "$h")" "$vm netem set $level" >>"$dir/logs/netem-$h.log" 2>&1 || { log "host $h: netem $level% not set"; return 1; }
        fi
    done
}
tr_netem_off() {
    local h rc=0
    for h in "$@"; do on "$(tr_host "$h")" "$vm netem clear" >>"$dir/logs/netem-$h.log" 2>&1 || { log "host $h: netem not cleared"; rc=1; }; done
    return $rc
}

# tr_set <set id>: every round of the set, in rounds.tsv's order
tr_set() {
    local want=$1 gi sid host rep netem rseed loadlist streamlist order position armspec arm aa
    local bin setup kind args loads streams rhosts failures=0 spec
    while IFS=$'\t' read -r -u 9 gi sid host rep netem rseed loadlist streamlist order; do
        [[ $sid == "$want" ]] || continue
        IFS=, read -ra loads <<<"$loadlist"
        IFS=, read -ra streams <<<"$streamlist"
        if [[ $host == ab ]]; then rhosts=(a b); else rhosts=("$host"); fi
        echo "$sid,$rep,$netem,$rseed,${order//,/ }" >>"$dir/order.csv"
        log "set $sid rep $rep netem $netem: seed $rseed, order $order"
        tr_netem_round "$netem" "${rhosts[@]}" || return 1
        position=0
        for armspec in ${order//,/ }; do
            position=$((position + 1))
            arm=${armspec%%@*} aa=0
            [[ $armspec == *@AA=1 ]] && aa=1
            spec=$(grep -m1 "^$arm " "$dir/arms.txt") || { log "no arm $arm"; return 1; }
            read -r _ bin setup kind args <<<"$spec"
            read -ra args <<<"${args:-}"
            if ! tr_arm_up "$setup" "${rhosts[@]}"; then
                echo "$sid,$rep,$netem,$arm,$aa,setup" >>"$dir/failures.csv"
                failures=$((failures + 1))
            else
                for n in "${streams[@]}"; do
                    if tr_invoke; then failures=0; else
                        echo "$sid,$rep,$netem,$arm,$aa,$n" >>"$dir/failures.csv"
                        failures=$((failures + 1))
                    fi
                    ((failures < ${TRANSPORT_MAX_FAILURES:-3})) || break
                done
            fi
            tr_arm_down "$setup" "${rhosts[@]}"
            ((failures < ${TRANSPORT_MAX_FAILURES:-3})) || { log "set $sid: $failures failed runs in a row: stopping the session"; [[ $netem == - ]] || tr_netem_off "${rhosts[@]}"; return 1; }
        done
        [[ $netem == - ]] || tr_netem_off "${rhosts[@]}" || return 1
    done 9<"$dir/rounds.tsv"
}

# tr_run <pair>: the session on VM a
tr_run() {
    pair=$1
    dir=$HOME/bench/transport-$pair
    # shellcheck source=/dev/null
    . "$dir/env"
    tr_arms_file=${arms_file:-}
    TRANSPORT_WARMUP_S=${TRANSPORT_WARMUP_S:-30} TRANSPORT_POINT_S=${TRANSPORT_POINT_S:-30} TRANSPORT_SWITCH_S=${TRANSPORT_SWITCH_S:-5}
    known=$dir/known_hosts
    ssh_key=${TRANSPORT_SSH_KEY:-$HOME/.ssh/lab}
    out=$dir
    cd "$HOME" || exit 1
    # shellcheck source=lib.sh
    . "$tr_here/lib.sh"
    # shellcheck source=irqs.sh
    . "$tr_here/irqs.sh"
    # shellcheck source=host.sh
    . "$tr_here/host.sh"
    mkdir -p "$dir/manifest" "$dir/points" "$dir/logs" "$dir/irq" "$dir/netem" "$dir/samples"
    : >"$dir/failures.csv"
    TRANSPORT_ARMS=$(tr_plan arms) || die "the plan's arms do not parse"
    : >"$dir/arms.txt"
    local a h gi rc=0 pids s sets
    for a in $(sed 's/=[^ ]*//g' <<<"$TRANSPORT_ARMS"); do echo "$a $(tr_plan arm "$a")" >>"$dir/arms.txt"; done
    tr_plan rounds "$pair" "$seed" "$reps" >"$dir/rounds.tsv" || die "the plan's rounds do not parse"
    log "session $session pair $pair: seed $seed, $(tr_plan count "$pair" "$reps") points in $(grep -c . "$dir/rounds.tsv") rounds"
    cp "$tr_here/transport-settings.env" "$dir/transport-settings.env"
    settings_sha=$(sha256sum "$dir/transport-settings.env" 2>/dev/null | awk '{print $1}')
    # the map, the NICs, the binaries and the ergon-tune report of each host;
    # a report with any warning stops the session before its first point
    for h in a b; do
        on "$(tr_host "$h")" "$vm transport-map" >"$dir/map-$h.txt" 2>"$dir/logs/map-$h.err" || die "host $h: no transport map"
        on "$(tr_host "$h")" "$vm nics" >"$dir/nics-$h.txt" 2>/dev/null || die "host $h: data NICs not read"
        on "$(tr_host "$h")" "sha256sum ergon/target/release/* extra/bin/* 2>/dev/null" >"$dir/bins-$h.txt" 2>/dev/null
        on "$(tr_host "$h")" "$vm netem clear" >>"$dir/logs/netem-$h.log" 2>&1 || die "host $h: a netem qdisc not cleared before the session"
        if ! on "$(tr_host "$h")" "$vm hostcheck" >"$dir/hostcheck-$h.txt" 2>&1; then
            die "host $h: the hostcheck report is not clean ($dir/hostcheck-$h.txt): $(grep -E '^[^ ]+ (warn|unknown): ' "$dir/hostcheck-$h.txt" | head -3 | tr '\n' ';') stopping before the first point"
        fi
    done
    cmp -s "$dir/map-a.txt" "$dir/map-b.txt" || die "the hosts' CPU maps differ: $(cat "$dir/map-a.txt") and $(cat "$dir/map-b.txt")"
    map=$(cat "$dir/map-a.txt")
    read -r hk _ iso <<<"$map"
    for gi in $(cut -f1 "$dir/rounds.tsv" | uniq); do
        pids=()
        # each set of the group at once, its log in logs/set-<id>.log and
        # here (the operator's progress lines), its status in set-<id>.rc
        sets=$(awk -F'\t' -v g="$gi" '$1 == g && !seen[$2]++ {print $2}' "$dir/rounds.tsv")
        for s in $sets; do
            { tr_set "$s" </dev/null; echo $? >"$dir/logs/set-$s.rc"; } 2>&1 | tee -a "$dir/logs/set-$s.log" >&2 &
            pids+=($!)
        done
        for s in "${pids[@]}"; do wait "$s"; done
        for s in $sets; do [[ $(cat "$dir/logs/set-$s.rc" 2>/dev/null) == 0 ]] || rc=1; done
        ((rc == 0)) || { log "a set of group $gi failed (logs/set-*.log)"; break; }
    done
    for h in a b; do on "$(tr_host "$h")" "$vm netem clear" >>"$dir/logs/netem-$h.log" 2>&1 || rc=1; done
    [[ ! -s $dir/failures.csv ]] || { log "failed runs: $(grep -c . "$dir/failures.csv") (failures.csv)"; rc=1; }
    log "session $session pair $pair: $(grep -c . "$dir/results.csv" 2>/dev/null || echo 0) rows, exit $rc"
    return $rc
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
    case ${1:-} in
    check) tr_check "${2:?pair}" "${3:-}" ;;
    run)
        log() { echo "[$(date +%T)] $*" >&2; }
        die() { log "ERROR: $*"; exit 1; }
        tr_pair=${2:?pair}
        dir=$HOME/bench/transport-$tr_pair
        # on any exit: no bench binary or driver left on either VM, then rc
        # on any exit: the bench binaries this runner started (their PIDs, as
        # tr_start recorded them: never by name, which would stop another's),
        # netem and the media driver, on both VMs; then rc
        trap 'rc=$?; for h in "${A:-}" "${B:-}"; do [[ -n $h ]] && on "$h" "for f in bench/transport-$tr_pair/run/*.pid; do [ -s \$f ] && kill -TERM \$(cat \$f) 2>/dev/null; done; ergon/md-dpdk/benches/cloud/vm.sh netem clear; ergon/md-dpdk/benches/cloud/vm.sh stop" >/dev/null 2>&1 </dev/null; done; kill $(jobs -p) 2>/dev/null; echo $rc >"$dir/rc"' EXIT
        tr_run "$2"
        ;;
    *) awk 'NR > 1 && !/^#/ {exit} NR > 1 {print}' "$0" >&2; exit 2 ;;
    esac
fi
