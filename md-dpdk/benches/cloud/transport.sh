#!/usr/bin/env bash
# The transport latency suite (azure/lab.sh LAB_SUITE=transport): the matrix
# (transport-matrix.tsv, plus LAB_ARMS_FILE's rows) as rounds of points on
# the two VMs of one pair session (LAB_PAIR a or b). Each rep's round at a
# netem level runs the set's arms in an order drawn from its own seed (the
# session's seed and the round: transport_plan.py), the A/A arm (the set's
# first arm again, @AA=1) among them. Each point is one run of the arm's
# bench binary (transport-cli.md) at one load: for two-host arms its pong on
# host b (the ping starts once it says ready), then its ping on host a;
# one-host arms --role local. The first load
# of an arm and stream count warms up 30 s, each later one 5 s (the switch).
# Around every point, on each host it uses: the IRQ counts (deltas on the
# isolated CPUs), a thread placement sample mid-window, and under netem the
# qdisc's counters, checked against the binomial interval of its loss. Each
# point gets a manifest (manifest/<point>.env, the binary's manifest.json as
# manifest/<point>.json); its --out directories go to points/<point>/ and
# its rows (the rep in their first column, the point column
# <channel>-r<rate>) to results.csv. At the end transport_collect.py lays the
# directory out as transport-format.md has it, for fairness.py and analyse.py.
#
#   transport.sh check <pair> [<arms file>]   the matrix and arms file parse, every mode a
#                         point runs passes modes.sh mode_check, and so do the Aeron arms'
#                         driver modes; prints the pair's point count (before any cloud call)
#   transport.sh run <pair>   on VM a (transport_remote starts it): reads
#                         ~/bench/transport-<pair>/env, writes results.csv, rounds.tsv,
#                         order.csv, manifest/, points/, logs/, failures.csv there, then rc
#   transport.sh smoke <dir>  the local end-to-end smoke (LAB_LOCAL_SMOKE=1): no az, no ssh;
#                         hosts a and b are homes under <dir>/hosts on this one Linux box
#                         (loopback addresses, local-vm.sh in place of vm.sh, one media
#                         driver for both), transport-smoke.tsv's sets at 1 rep and short
#                         durations; then transport_collect.py, fairness.py and
#                         analyse.py --transport (test/transport-smoke-docker.sh runs it)
#
# Sourced (azure/lab.sh, the tests): functions only, among them
# transport_prepare and transport_remote, the operator machine's side.
#
# Durations (env, else the plan's): TRANSPORT_WARMUP_S (30), TRANSPORT_POINT_S
# (30), TRANSPORT_SWITCH_S (5), TRANSPORT_READY_TIMEOUT_S (60: the longest the
# runner waits for a pong's one `ready` line before the point fails);
# TRANSPORT_MAX_FAILURES (3): that many failed runs (an arm at a stream count)
# in a row stop the session.
set -uo pipefail

tr_here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=transport-settings.env
. "$tr_here/transport-settings.env"
# shellcheck source=modes.sh
. "$tr_here/modes.sh"

# tr_files [<arms file>]: the plan's files: the matrix, then the arms file;
# under LAB_LOCAL_SMOKE the matrix's arm rows alone, then transport-smoke.tsv's
# sets (TRANSPORT_SMOKE_SETS), then the arms file
tr_files() {
    # the tuning session's own plan files (tr_tune)
    if [[ -n ${tr_plan_files:-} ]]; then printf '%s\n' $tr_plan_files; return; fi
    if [[ -n ${LAB_LOCAL_SMOKE:-} ]]; then
        local arms=${LOCAL_SMOKE_ROOT:?}/matrix-arms.tsv
        grep '^arm ' "$tr_here/transport-matrix.tsv" >"$arms"
        printf '%s\n' "$arms" "${TRANSPORT_SMOKE_SETS:-$tr_here/transport-smoke.tsv}" ${1:+"$1"}
        return
    fi
    # LAB_SETS_FILE: its set rows in place of the matrix's (the cloud smoke's)
    if [[ -n ${tr_sets_file:-} ]]; then
        printf '%s\n' "$tr_here/transport-matrix.tsv:arms" "$tr_sets_file" ${1:+"$1"}
        return
    fi
    printf '%s\n' "$tr_here/transport-matrix.tsv" ${1:+"$1"}
}

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
    [[ $pair == tune-a || $pair == tune-b ]] && { tr_check_tune "${pair#tune-}"; return; }
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
    tr_declared_check || return 2
    tr_huge_check "$pair" || return 2
    echo "$n"
}

# tr_declared <arm>: "<twin> <knob> <value>", the arm's TR_DECLARED entry
# (transport-settings.env), or nothing (1) when it has none
tr_declared() {
    local e k
    for e in ${TR_DECLARED:-}; do
        [[ $e == "$1:"* ]] || continue
        e=${e#*:} k=${e#*:}
        echo "${e%%:*} ${k%%=*} ${k#*=}"
        return 0
    done
    return 1
}

# tr_declared_check: every TR_DECLARED entry an arm of the plan whose arm
# row is its twin's, its knob a per-arm knob of that binary, aeron_page 4096
# or 2097152; 2 saying why not
tr_declared_check() {
    local e a twin k v mine theirs arm bin setup args
    for e in ${TR_DECLARED:-}; do
        [[ $e =~ ^[A-Za-z0-9_-]+:[A-Za-z0-9_-]+:[a-z_]+=[^[:space:]]+$ ]] || { echo "TR_DECLARED $e: <arm>:<twin>:<knob>=<value>" >&2; return 2; }
        a=${e%%:*}
        read -r twin k v <<<"$(tr_declared "$a")"
        mine=$(tr_plan arm "$a" 2>/dev/null) || { echo "TR_DECLARED $a: no arm $a" >&2; return 2; }
        theirs=$(tr_plan arm "$twin" 2>/dev/null) || { echo "TR_DECLARED $a: no arm $twin to be its twin" >&2; return 2; }
        [[ $mine == "$theirs" ]] || { echo "TR_DECLARED $a: not $twin's twin: [$mine] against [$theirs]" >&2; return 2; }
        read -r bin setup _ args <<<"$mine"
        read -ra args <<<"${args:-}"
        arm=$a
        [[ " $(tr_arm_knobs | tr '\n' ' ') " == *" $k "* ]] ||
            { echo "TR_DECLARED $a: $k is not a per-arm knob of $bin (transport-settings.env's list)" >&2; return 2; }
        [[ $k != aeron_page || $v == 4096 || $v == 2097152 ]] ||
            { echo "TR_DECLARED $a: aeron_page $v: 4096 (TR_AERON_DIR, tmpfs) or 2097152 (TR_AERON_HUGE_DIR, 2 MiB hugetlbfs)" >&2; return 2; }
    done
}

# tr_huge_check <pair>: TR_AERON_HUGE_PAGES holds what each of the pair's
# arms on the hugetlbfs (aeron_page 2097152) needs at its term length and
# most streams (host.sh transport_huge_pages); 2 saying which needs more
tr_huge_check() {
    local a s need arm bin setup args
    for a in $(tr_huge_arms "$(tr_plan rounds "$1" 1 1)"); do
        read -r bin setup _ args <<<"$(tr_plan arm "$a")"
        read -ra args <<<"${args:-}"
        arm=$a
        s=$(tr_plan rounds "$1" 1 1 | awk -F'\t' -v a="$a" '("," $9 ",") ~ ("," a "[,@]") {n = split($8, st, ","); for (i = 1; i <= n; i++) if (st[i] > m) m = st[i]} END {print m + 0}')
        need=$(. "$tr_here/host.sh"; transport_huge_pages "$(tr_knob term_length)" "$s")
        ((TR_AERON_HUGE_PAGES >= need)) || {
            echo "TR_AERON_HUGE_PAGES $TR_AERON_HUGE_PAGES: $a's hugetlbfs needs $need pages of 2 MiB (term length $(tr_knob term_length), $s streams: host.sh transport_huge_pages)" >&2
            return 2
        }
    done
}

# tr_huge_arms <rounds>: the Aeron arms of these rounds (transport_plan.py
# rounds' lines) on the hugetlbfs (aeron_page 2097152), one a line
tr_huge_arms() {
    local a arm
    for a in $(cut -f9 <<<"$1" | tr ',' '\n' | sed 's/@.*//' | sort -u); do
        [[ $(tr_plan arm "$a" 2>/dev/null) == aeron_lat\ * ]] || continue
        arm=$a
        [[ $(tr_knob aeron_page) != 2097152 ]] || echo "$a"
    done
}

# tr_aeron_dir: the current arm's Aeron directory (aeron_lat --aeron-dir,
# the driver's AERON_DIR): TR_AERON_HUGE_DIR at aeron_page 2097152, else
# TR_AERON_DIR
tr_aeron_dir() {
    if [[ $(tr_knob aeron_page) == 2097152 ]]; then echo "$TR_AERON_HUGE_DIR"; else echo "$TR_AERON_DIR"; fi
}

# tr_arm_vm: vm.sh for a call about the current arm (its driver, its
# events, its directory's page size): on the arm's own Aeron directory
tr_arm_vm() { echo "LAB_AERON_DIR=$(tr_aeron_dir) ${vm#LAB_AERON_DIR=* }"; }

# tr_hostcheck_file <host>: the session hostcheck report that covers the
# current arm on that host: on the hugetlbfs, the one run on its directory
tr_hostcheck_file() {
    if [[ $bin == aeron_lat && $(tr_aeron_dir) == "$TR_AERON_HUGE_DIR" ]]; then echo "hostcheck-$1-hugepages.txt"; else echo "hostcheck-$1.txt"; fi
}

# tr_check_tune <a|b>: the tuning session of that pair (LAB_TUNE=1) planned
# as the VM will (transport_tune.py sweep, LAB_TUNE_ARMS and LAB_TUNE_KNOBS;
# a knob a kernel lacks is dropped only there), every variant's driver mode
# through modes.sh; 0 and the sweep's point count, or 2 saying why
tr_check_tune() {
    local p=$1 files tmpd v m why rc=0
    [[ -z $tr_arms_file || -r $tr_arms_file ]] || { echo "LAB_ARMS_FILE $tr_arms_file: not a readable file" >&2; return 2; }
    mapfile -t files < <(tr_files "$tr_arms_file")
    tmpd=$(mktemp -d "${TMPDIR:-/tmp}/ergon-tune.XXXXXX") || return 2
    if ! why=$(python3 "$tr_here/transport_tune.py" sweep --settings "$tr_here/transport-settings.env" --out "$tmpd" --pair "$p" \
        --arms "${LAB_TUNE_ARMS:-}" --knobs "${LAB_TUNE_KNOBS:-}" "${files[@]}" 2>&1); then
        echo "LAB_TUNE=1 pair $p: $why" >&2
        rm -rf -- "$tmpd"
        return 2
    fi
    (
        dir=$tmpd tr_plan_files="$tmpd/tune-arms.tsv $tmpd/tune-sweep.tsv"
        TRANSPORT_ARMS=$(tr_plan arms) || exit 2
        while read -r v _ _ _ _; do
            read -r _ bin setup _ args <<<"$(tr_plan arm "$v")"
            read -ra args <<<"${args:-}"
            arm=$v
            [[ $setup == kernel || $setup == dpdk ]] || continue
            m=$(tr_driver_mode)
            why=$(mode_check "$m" remote x86_64 azure '' 2>&1) || { echo "LAB_TUNE=1: variant $v's driver mode $m: $why" >&2; exit 2; }
        done <"$tmpd/variants.txt"
        modes=$(tr_plan modes tune) || exit 2
        for m in $modes; do
            why=$(mode_check "$m" remote x86_64 azure '' 2>&1) || { echo "$why" >&2; exit 2; }
        done
        tr_plan count tune 1
    ) || rc=2
    rm -rf -- "$tmpd"
    return $rc
}

# ---- the operator machine's side (azure/lab.sh, after lib.sh prepare_vms) ----

# transport_prepare: LAB_EXTRA_SRC's tree, LAB_ARMS_FILE and LAB_SETS_FILE on both VMs,
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
        if [[ -n ${sets_file:-} ]]; then
            scp -q "${ssh_opts[@]}" "$sets_file" "$user@$host:bench/sets.tsv" || die "copy of LAB_SETS_FILE to $host failed"
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
sets_file=${sets_file:+\$HOME/bench/sets.tsv}
tune_arms=${LAB_TUNE_ARMS:-}
tune_knobs=${LAB_TUNE_KNOBS:-}
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

# tr_bin_args <binary> <role> <own ip> <peer ip> [<arm arguments>...]: the
# binary's addresses and host settings, the same for every arm that runs it
# (transport-cli.md, each binary's --help; transport-settings.env). Base port
# TR_PORT_BASE, stream i on port + i. aeron_lat --mode udp: --endpoint is the
# pong's address (ping: the peer; pong: its own), --reply the ping's, and
# --interface this end's own; --mode ipc and spy take no address (the binary
# refuses one). udp_lat: --local own, --peer the other end (local: the
# receiver binds --peer). tcp_lat: the pong (and local's receiver) listens,
# the ping connects. --busy-read-us is mio's alone (ergon-io's epoll-spin
# refuses it: its SO_BUSY_POLL is 0).
tr_bin_args() {
    local b=$1 role=$2 own=$3 peer=$4 port=$TR_PORT_BASE out=
    shift 4
    local args=" $* " busy= sb qa
    [[ $args == *" --impl mio "* ]] && busy="--busy-read-us $(tr_knob busy_read_us) "
    sb=$(tr_knob sockbuf)
    qa="--quickack-per-read"
    [[ $(tr_knob quickack_per_read) == off ]] && qa+=" false"
    case $b in
    mpsc_lat) out="--capacity $(tr_knob capacity)" ;;
    aeron_lat)
        out="--aeron-dir $(tr_aeron_dir) --driver external --driver-busy-threads $(tr_driver_busy) --term-length $(tr_knob term_length) --mtu $TR_MTU"
        if [[ $args == *" --mode udp "* ]]; then
            local ep=$peer rp=$own
            [[ $role == pong ]] && ep=$own rp=$peer
            out="--endpoint $ep:$port --reply $rp:$port --interface $own $out"
        fi
        ;;
    udp_lat)
        out="--local $own:$port --peer $peer:$port ${busy}--rcvbuf $sb --sndbuf $sb --epoll-events $(tr_knob epoll_events)"
        [[ $args == *" --impl ergon "* ]] && out+=" --datagram-burst $(tr_knob datagram_burst)"
        ;;
    tcp_lat)
        local a="--connect $peer:$port"
        [[ $role == pong ]] && a="--listen $own:$port"
        [[ $role == local ]] && a="--listen $peer:$port"
        out="$a --nodelay $qa $(tr_timer_args)${busy}--rcvbuf $sb --sndbuf $sb --epoll-events $(tr_knob epoll_events)"
        ;;
    *)
        # an arms file's binary: its addresses, and on the kernel's path
        # (setup sockets) every socket and TCP setting the built-in socket
        # arms get, so it cannot run with other ones (transport-cli.md)
        out="--local $own:$port --peer $peer:$port"
        [[ ${setup:-} == sockets ]] && out+=" --busy-read-us $(tr_knob busy_read_us) --rcvbuf $sb --sndbuf $sb --nodelay $qa $(tr_timer_args)"
        out=${out% }
        ;;
    esac
    echo "$out"
}

# tr_knob <knob>: the current arm's value of a per-arm knob
# (transport-settings.env's list): a tuning variant's own value
# (variants.txt: the tuning session), else a declared twin's own knob
# (TR_DECLARED), else the tuned section's (TR_ARM_KNOBS "<arm>:<knob>=<value>
# ...": a declared twin's twin's), else the knob's default
tr_knob() {
    local k=$1 v= kv base twin dk dv
    if [[ -s ${dir:-}/variants.txt ]]; then
        v=$(awk -v a="${arm:-}" -v k="$k" '$1 == a && $3 == k {print $4; exit}' "$dir/variants.txt")
        [[ -z $v ]] || { echo "$v"; return; }
    fi
    base=$(tr_base_arm)
    if read -r twin dk dv < <(tr_declared "$base"); then
        [[ $dk == "$k" ]] && { echo "$dv"; return; }
        base=$twin
    fi
    for kv in ${TR_ARM_KNOBS:-}; do
        [[ $kv == "$base:$k="* ]] && { echo "${kv#*=}"; return; }
    done
    case $k in
    busy_read_us) echo "$TR_BUSY_READ" ;;
    epoll_events) echo "$TR_EPOLL_EVENTS" ;;
    datagram_burst) echo "$TR_DATAGRAM_BURST" ;;
    sockbuf) echo "$TR_SOCKBUF" ;;
    quickack_per_read) echo "$TR_QUICKACK_PER_READ" ;;
    rto_min_us) echo "$TR_TCP_RTO_MIN_US" ;;
    congestion) echo "$TR_TCP_CONGESTION" ;;
    term_length) echo "$TR_TERM_LENGTH" ;;
    aeron_path) echo "$TR_AERON_PATH" ;;
    aeron_so_rcvbuf) echo "$TR_AERON_SO_RCVBUF" ;;
    nak_delay_ns) echo "$TR_AERON_NAK_DELAY_NS" ;;
    rx_free_thresh) echo "$TR_DPDK_RX_FREE_THRESH" ;;
    aeron_threading) echo "$TR_AERON_IPC_THREADING" ;;
    capacity) echo "$TR_MPSC_CAPACITY" ;;
    aeron_page) echo "$TR_AERON_PAGE" ;;
    *) echo "tr_knob: no knob $k" >&2; return 2 ;;
    esac
}

# tr_base_arm: the current arm's own name: a tuning variant's base arm
# (variants.txt), else the arm itself
tr_base_arm() {
    local b=
    [[ -s ${dir:-}/variants.txt ]] && b=$(awk -v a="${arm:-}" '$1 == a {print $2; exit}' "$dir/variants.txt")
    echo "${b:-${arm:-}}"
}

# tr_arm_knobs: the per-arm knobs the current arm has, one a line (its
# binary, its --impl, --mode and --reliable, its setup), each recorded in
# its points' manifests (knob.<name>=<value>) for transport_collect.py
tr_arm_knobs() {
    local a=" ${args[*]:-} "
    case $bin in
    udp_lat)
        echo sockbuf epoll_events
        if [[ $a == *" --impl mio "* ]]; then echo busy_read_us; else echo datagram_burst; fi
        ;;
    tcp_lat)
        echo sockbuf epoll_events quickack_per_read congestion
        [[ $a == *" --impl mio "* ]] && echo busy_read_us
        [[ $(tr_timer_args) == *--rto-min-us* ]] && echo rto_min_us
        ;;
    aeron_lat)
        echo term_length aeron_page
        if [[ $setup == dpdk ]]; then
            echo rx_free_thresh
        elif [[ $a == *" --mode ipc "* ]]; then
            echo aeron_threading
        elif [[ $a == *" --mode udp "* ]]; then
            echo aeron_path aeron_so_rcvbuf
        fi
        [[ $a == *" --mode udp "* && $a == *" --reliable true "* ]] && echo nak_delay_ns
        ;;
    mpsc_lat) echo capacity ;;
    esac
    return 0
}

# tr_driver_mode: the current Aeron arm's media driver mode (vm.sh driver):
# its path (aeron_path: kernel-busy or kernel; dpdk) and
# TR_AERON_MODE_SUFFIX, with its driver knobs where they differ from the
# driver's own (the socket buffer, the NAK delay, DPDK's rx_free_thresh) and
# aeron-ipc's SHARED threading (one agent: no conductor of its own)
tr_driver_mode() {
    local m s=$TR_AERON_MODE_SUFFIX v k knobs
    knobs=" $(tr_arm_knobs | tr '\n' ' ') "
    if [[ $setup == dpdk ]]; then m=dpdk; else m=kernel-busy; fi
    [[ $knobs == *" aeron_path "* ]] && m=$(tr_knob aeron_path)
    for k in aeron_so_rcvbuf nak_delay_ns rx_free_thresh; do
        [[ $knobs == *" $k "* ]] || continue
        v=$(tr_knob "$k")
        case $k in
        aeron_so_rcvbuf) [[ $v == 2097152 ]] || s+="@AERON_SOCKET_SO_RCVBUF=$v" ;;
        nak_delay_ns) [[ $v == 1000 ]] || s+="@AERON_NAK_UNICAST_DELAY=$v" ;;
        rx_free_thresh) [[ $v == pmd ]] || s+="@RUSTERON_DPDK_RX_FREE_THRESH=$v" ;;
        esac
    done
    if [[ $knobs == *" aeron_threading "* && $(tr_knob aeron_threading) == SHARED ]]; then
        s=${s//@CONDUCTOR=own/}
        s=${s//@AERON_CONDUCTOR_IDLE_STRATEGY=spin/}
        s=${s//@AERON_SENDER_IDLE_STRATEGY=noop/}
        s=${s//@AERON_RECEIVER_IDLE_STRATEGY=noop/}
        s+="@AERON_THREADING_MODE=SHARED"
    fi
    echo "$m$s"
}

# tr_driver_busy: the external driver's busy threads (aeron_lat
# --driver-busy-threads): TR_AERON_DRIVER_BUSY_THREADS, one under SHARED
tr_driver_busy() {
    if [[ $(tr_driver_mode) == *@AERON_THREADING_MODE=SHARED* ]]; then echo 1; else echo "$TR_AERON_DRIVER_BUSY_THREADS"; fi
}

# tr_host_overrides: the TR_OVERRIDES the current arm's host set-up runs
# with (vm.sh transport-net, driver): its congestion control where not the
# hosts' default, and under the kernel path (no busy poll) the busy-poll
# sysctls kernel's driver sets (0) for transport-net check
tr_host_overrides() {
    local o= knobs
    knobs=" $(tr_arm_knobs | tr '\n' ' ') "
    [[ $knobs == *" congestion "* && $(tr_knob congestion) != "$TR_TCP_CONGESTION" ]] && o+="TR_TCP_CONGESTION=$(tr_knob congestion) "
    [[ $knobs == *" aeron_path "* && $(tr_knob aeron_path) == kernel ]] && o+="TR_BUSY_READ=0 TR_BUSY_POLL=0 "
    echo "${o% }"
}

# tr_timer_args: "--rto-min-us N --delack-max-us N " for the timers both
# hosts' kernels have (settings-<a|b>.txt, read as the session starts: the
# sysctl present), each only where both have it; the binaries refuse one a
# kernel lacks. An arm without one runs on the kernel's default, recorded in
# its manifest (rto_min_us_asked kernel)
tr_timer_args() {
    local k v out=
    for k in rto_min delack_max; do
        grep -q "^net.ipv4.tcp_${k}_us=absent$" "$dir/settings-a.txt" "$dir/settings-b.txt" 2>/dev/null && continue
        if [[ $k == rto_min ]]; then v=$(tr_knob rto_min_us); else v=$TR_TCP_DELACK_MAX_US; fi
        out+="--${k//_/-}-us $v "
    done
    printf '%s' "$out"
}

# tr_role_cpus <role>: the --cpus of a run, from the arm's CPUs ($cpus,
# transport_clients): one per busy thread, in each binary's thread order:
# local two (sender and receiver, or ping and echo), ping and pong one.
# Under LAB_LOCAL_SMOKE both ends share one box, so the pong takes the last.
tr_role_cpus() {
    local c
    IFS=, read -ra c <<<"$cpus"
    case $1 in
    local) echo "${c[0]},${c[1]}" ;;
    pong) if [[ -n ${LAB_LOCAL_SMOKE:-} ]]; then echo "${c[${#c[@]} - 1]}"; else echo "${c[0]}"; fi ;;
    *) echo "${c[0]}" ;;
    esac
}

# tr_label <set> <arm> <aa> <streams> <netem>: a row's label
# (transport-format.md): <set>:<arm, or aa for the A/A>@STREAMS=<n>[@NETEM=<pct>]
tr_label() {
    local l="$1:$2"
    (($3)) && l="$1:aa"
    l+="@STREAMS=$4"
    [[ $5 == - ]] || l+="@NETEM=$5"
    echo "$l"
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
    if [[ $netem == none ]]; then
        # the root qdiscs after the point: none netem
        if ! [[ -s $dir/netem/$pid-$h-after ]]; then echo "unread FAIL"
        elif grep -q ' netem$' "$dir/netem/$pid-$h-after"; then echo "netem present FAIL"
        else echo "no qdisc: $(tr '\n' ' ' <"$dir/netem/$pid-$h-after")ok"; fi
        return
    fi
    read -r sent dropped < <(awk 'FNR == NR {s[$1] = $2; d[$1] = $3; next} ($1 in s) {ts += $2 - s[$1]; td += $3 - d[$1]; n++}
        END {if (n) print ts + td, td; else print "x x"}' "$dir/netem/$pid-$h-before" "$dir/netem/$pid-$h-after" 2>/dev/null)
    if [[ ${sent:-x} == x ]]; then echo "unread FAIL"; return; fi
    verdict=$(python3 "$tr_here/transport_plan.py" binom "$sent" "$dropped" "$netem") || verdict="? ? FAIL"
    read -r lo hi ok <<<"$verdict"
    echo "$sent/$dropped p=$netem in [$lo,$hi] $ok"
}

# tr_snap <before|after> <pid>: each host the point uses: its IRQ counts, its
# kernel counters (/proc/net/snmp and netstat: nstat's) and, under netem, its
# qdisc counters; after the point also its settings read back (vm.sh
# settings) and an Aeron arm's directory's page size (vm.sh aeron-page)
tr_snap() {
    local h
    for h in "${run_hosts[@]}"; do
        on "$(tr_host "$h")" "$vm irqsnap" >"$dir/irq/$2-$h-$1" 2>/dev/null
        on "$(tr_host "$h")" "$vm nstat" >"$dir/nstat/$2-$h-$1" 2>/dev/null
        case $netem in
        -) ;;
        none) on "$(tr_host "$h")" "$vm netem show" >"$dir/netem/$2-$h-$1" 2>/dev/null ;;
        *) on "$(tr_host "$h")" "$vm netem stats" >"$dir/netem/$2-$h-$1" 2>/dev/null ;;
        esac
        [[ $1 == before ]] || on "$(tr_host "$h")" "$vm settings" >"$dir/settings/$2-$h" 2>/dev/null
        [[ $1 == before || $bin != aeron_lat ]] || on "$(tr_host "$h")" "$(tr_arm_vm) aeron-page" >"$dir/aeronpage/$2-$h" 2>/dev/null
    done
}

# tr_manifest <pid> <load> <warm-up s> <start ns> <end ns>: the point's
# manifest (manifest/<point>.env; the binary's own is manifest/<point>.json)
tr_manifest() {
    local pid=$1 h hot place nline k v
    {
        echo "point=$pid"
        echo "set=$sid"
        echo "rep=$rep"
        echo "round=$sid-r$rep-netem$netem"
        echo "arm=$arm"
        echo "mode=$mode"
        echo "label=$label"
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
        echo "args=${args[*]:-}"
        for h in "${run_hosts[@]}"; do echo "bin_sha256_$h=$(awk -v b="$bin" '$2 ~ "/" b "$" {print $1}' "$dir/bins-$h.txt")"; done
        echo "commit=$commit"
        echo "dirty=$dirty"
        echo "settings_sha256=$settings_sha"
        # the arm's per-arm knobs as it ran them (transport_collect.py
        # declares them; fairness.py's settings and shared-knob rules)
        echo "base_arm=$(tr_base_arm)"
        for k in $(tr_arm_knobs); do echo "knob.$k=$(tr_knob "$k")"; done
        [[ $setup == kernel || $setup == dpdk ]] && echo "driver_mode=$(tr_driver_mode)"
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
            if [[ $bin == aeron_lat ]]; then
                v=$(head -1 "$dir/aeronpage/$pid-$h" 2>/dev/null)
                [[ $v =~ ^[0-9]+$ ]] || v=unread
                echo "aeron_page_$h=$v"
            fi
            [[ $nline != *FAIL ]] || log "WARNING: point $pid: host $h's netem counters off its loss: $nline"
            [[ $place == ok* ]] || log "WARNING: point $pid: host $h's thread placement: $place"
        done
        echo "hostcheck_a=$(tr_hostcheck_file a)"
        echo "hostcheck_b=$(tr_hostcheck_file b)"
    } >"$dir/manifest/$pid.env"
}

# tr_arm_up <setup> <host>...: the arm's host set-up on each: a media driver
# (kernel: kernel-busy's path, then transport-net check; dpdk), the kernel
# path (sockets: transport-net), or nothing; tr_arm_down: the driver stopped
tr_arm_up() {
    local setup=$1 h ov
    shift
    ov=$(tr_host_overrides)
    [[ -z $ov ]] || ov="TR_OVERRIDES='$ov' "
    for h in "$@"; do
        case $setup in
        kernel) on "$(tr_host "$h")" "$(tr_arm_vm) driver $(tr_driver_mode) && ${ov}$vm transport-net check" ;;
        dpdk) on "$(tr_host "$h")" "$(tr_arm_vm) driver $(tr_driver_mode)" ;;
        sockets) on "$(tr_host "$h")" "${ov}$vm transport-net" ;;
        none) true ;;
        esac >>"$dir/logs/setup-$h.log" 2>&1 || { log "host $h: $setup set-up failed ($dir/logs/setup-$h.log)"; return 1; }
    done
}
# (a socket arm whose host settings differed: the hosts' own put back)
tr_arm_down() {
    local setup=$1 h
    shift
    if [[ $setup == sockets && -n $(tr_host_overrides) ]]; then
        for h in "$@"; do on "$(tr_host "$h")" "$vm transport-net" >>"$dir/logs/setup-$h.log" 2>&1 || log "host $h: settings not put back"; done
    fi
    [[ $setup == kernel || $setup == dpdk ]] || return 0
    for h in "$@"; do on "$(tr_host "$h")" "$(tr_arm_vm) stop" >>"$dir/logs/setup-$h.log" 2>&1 || log "host $h: vm.sh stop failed"; done
}

# tr_events_cpus <host>: "<traced cpus> <spinner cpus|->" for this point on
# <host>, each a list: traced, the hot CPUs the point's busy threads run on
# there (the media driver's three under kernel and dpdk: host.sh
# transport_layout; the binary's: tr_role_cpus) and the spinners'; the
# spinners', the map's isolated CPUs the point leaves idle (- with none).
# Under LAB_LOCAL_SMOKE one box is both hosts: neither host spins on a CPU
# either end uses, a two-host point's idle ones are dealt between a and b in
# turn, and each host takes one (the box isolates nothing: more would starve
# what else must run there).
tr_events_cpus() {
    local h=$1 used=" " mine_used=" " c all out=() i=0 mine r
    IFS=, read -ra all <<<"$iso"
    if [[ $setup == kernel || $setup == dpdk ]]; then used+="${all[0]} ${all[1]} ${all[2]} "; mine_used=$used; fi
    if [[ $kind == two ]]; then
        for r in ping pong; do
            c=$(tr_role_cpus $r)
            [[ $r == ping && $h == a || $r == pong && $h == b ]] && mine_used+="$c "
            [[ -z ${LAB_LOCAL_SMOKE:-} && ! ($r == ping && $h == a || $r == pong && $h == b) ]] || used+="$c "
        done
    else
        used+="$(tr_role_cpus local | tr , ' ') "
        mine_used=$used
    fi
    mine=0
    [[ $h == b ]] && mine=1
    for c in "${all[@]}"; do
        [[ $used == *" $c "* ]] && continue
        if [[ -n ${LAB_LOCAL_SMOKE:-} && $kind == two ]]; then
            ((i++ % 2 == mine)) || continue
        fi
        out+=("$c")
    done
    [[ -z ${LAB_LOCAL_SMOKE:-} ]] || out=("${out[@]:0:1}")
    local traced
    traced=$(printf '%s\n' $mine_used "${out[@]}" | sort -nu | paste -sd, -)
    if ((${#out[@]})); then echo "$traced $(IFS=,; echo "${out[*]}")"; else echo "$traced -"; fi
}

# tr_events <start|stop> <pid>: the point's events on each host it uses
# (vm.sh events-start before any of its binaries starts; events-stop after
# its window, gzipped into events/<pid>-<host>.log.gz for
# transport_collect.py). One that does not start is logged, never fatal: its
# max then has no events to be labelled by, which spikes.py says.
tr_events() {
    local what=$1 pid=$2 h ex=
    [[ $setup == kernel || $setup == dpdk ]] && ex=aeron
    for h in "${run_hosts[@]}"; do
        if [[ $what == start ]]; then
            on "$(tr_host "$h")" "$(tr_arm_vm) events-start $pid $(tr_events_cpus "$h") $ex" >>"$dir/logs/events-$h.log" 2>&1 ||
                log "WARNING: point $pid: host $h's events did not start ($dir/logs/events-$h.log): its max goes unlabelled"
        else
            on "$(tr_host "$h")" "$vm events-stop $pid" 2>>"$dir/logs/events-$h.log" | gzip >"$dir/events/$pid-$h.log.gz"
        fi
    done
}

# tr_point <k>: one point at loads[k] (the round's variables: sid host rep
# netem rseed order position arm aa bin setup kind args n loads label), with
# the snapshots around it, a placement sample half way through its window,
# its rows (rep in their first column, the point column <channel>-r<rate>)
# into results.csv and its --out directories into points/<point>/ (the
# pong's as points/<point>/pong); 1 when it failed. A two-host arm's point is
# one run: its pong on b, which must print one `ready` line within
# TRANSPORT_READY_TIMEOUT_S, then its ping on a (round trips). A one-host
# arm's is two, one after the other: --role local (one way, the same TSC) and
# --role local --round-trip. The first point of an arm and stream count warms
# up TRANSPORT_WARMUP_S, each later one TRANSPORT_SWITCH_S (the new load,
# unrecorded).
tr_point() {
    local k=$1 load=${loads[$1]} warm pid rdir=bench/transport-$pair/run-$sid rc=0 meas log common pong= start end v runs=()
    warm=$TRANSPORT_SWITCH_S
    ((k == 0)) && warm=$TRANSPORT_WARMUP_S
    pid="$tag-$load"
    log=$dir/logs/$pid
    common="--streams $n --rate $load --warmup-s $warm --duration-s $TRANSPORT_POINT_S --size $TR_PAYLOAD"
    common+=" --label $label --spike-ns $TR_SPIKE_NS --stall-ns $TR_STALL_NS"
    # Aeron: each run on stream ids of its own (ping and pong the same), so
    # no publication of an earlier run, lingering in the driver, is the image
    # a subscription takes (tr_stream_base)
    local sid0= sid1=
    if [[ $bin == aeron_lat ]]; then
        sid0=" --stream-id $(tr_stream_base)" sid1=" --stream-id $(($(tr_stream_base) + 128))"
        tr_stream_seq=$((tr_stream_seq + 1))
    fi
    for h in "${run_hosts[@]}"; do on "$(tr_host "$h")" "rm -rf $rdir && mkdir -p $rdir" || return 1; done
    # the point's events on each host, begun before any binary of it starts
    # (so before its warm-up), read back after its window (tr_events)
    tr_events start "$pid"
    if [[ $kind == two ]]; then
        meas=a
        # the pong first, on b (a simple background command: the SSH session
        # ends as it returns); the ping starts once it says ready
        on "$B" "nohup $(tr_start "$rdir/pong.pid" "$rdir/pong.out" "$bin --role pong $common$sid0 --cpus $(tr_role_cpus pong) --out $rdir/out-pong $(tr_bin_args "$bin" pong "$(tr_nic b receiver)" "$(tr_nic a sender)" "${args[@]}") ${args[*]}") 2>&1 </dev/null &" ||
            { log "point $pid: pong did not start on b"; return 1; }
        pong=1
        local why
        tr_pong_ready "$rdir"
        case $? in
        0) why= ;;
        2) why="the pong exited before it said ready" ;;
        3) why="the pong said ready twice" ;;
        *) why="the pong did not say ready within ${TRANSPORT_READY_TIMEOUT_S:-60} s" ;;
        esac
        if [[ -n $why ]]; then
            on "$B" "kill -TERM \$(cat $rdir/pong.pid) 2>/dev/null; sleep 1; kill -KILL \$(cat $rdir/pong.pid) 2>/dev/null; cat $rdir/pong.out" >"$log.pong" 2>&1
            log "point $pid: $why: $(tail -2 "$log.pong" | tr '\n' ' ')"
            tr_events stop "$pid"
            return 1
        fi
        runs=("--role ping --cpus $(tr_role_cpus ping) $(tr_bin_args "$bin" ping "$(tr_nic a sender)" "$(tr_nic b receiver)" "${args[@]}")$sid0")
    else
        meas=$host
        v=$(tr_bin_args "$bin" local "$(tr_nic "$host" sender)" "$(tr_nic "$host" receiver)" "${args[@]}")
        runs=("--role local --cpus $(tr_role_cpus local) $v$sid0" "--role local --round-trip --cpus $(tr_role_cpus local) $v$sid1")
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
    tr_events stop "$pid"
    if [[ -n $pong ]]; then
        on "$B" "kill -TERM \$(cat $rdir/pong.pid) 2>/dev/null; for _ in \$(seq 50); do kill -0 \$(cat $rdir/pong.pid) 2>/dev/null || break; sleep 0.1; done; kill -KILL \$(cat $rdir/pong.pid) 2>/dev/null; true" >/dev/null 2>&1
        on "$B" "cat $rdir/pong.out" >"$log.pong" 2>&1
        mkdir -p "$dir/points/$pid/pong"
        on "$B" "tar -C $rdir/out-pong -cf - . 2>/dev/null" | tar -xf - -C "$dir/points/$pid/pong" 2>/dev/null
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
    for v in "${!runs[@]}"; do tr_row "$rep" <"$dir/points/$pid/$v/results.csv" >>"$dir/results.csv"; done
}

# tr_stream_base: the next run's aeron_lat --stream-id: 1001 + 256 k for the
# k-th Aeron point of a set (mod 4096), room for 128 streams and their
# echoes (+10000) each, and never one an earlier point used while the
# driver can still hold it. A one-host point's two runs differ too: its
# round trip takes base + 128.
tr_stream_seq=0
tr_stream_base() {
    echo $((1001 + (tr_stream_seq % 4096) * 256))
}

# tr_row <rep>: a binary's row (stdin) as results.csv takes it
# (transport-format.md): the rep before it, its channel column (the third)
# as <channel>-r<rate> (the rate is the fifth)
tr_row() { awk -F, -v OFS=, -v rep="$1" 'NF {$3 = $3 "-r" $5; print rep, $0}'; }

# tr_pong_ready <run dir>: 0 once the pong's output (stdout and stderr) holds
# exactly one line `ready`; 1 when it exits first, says it twice, or
# TRANSPORT_READY_TIMEOUT_S (60) passes. A pid file not yet written (the
# pong's shell starting) is waited for, not taken for an exit
tr_pong_ready() {
    local tenths=$((${TRANSPORT_READY_TIMEOUT_S:-60} * 10))
    on "$B" "for _ in \$(seq $tenths); do n=\$(grep -cx ready $1/pong.out 2>/dev/null); [ \"\$n\" = 1 ] && exit 0; [ \"\${n:-0}\" -gt 1 ] && exit 3; p=\$(cat $1/pong.pid 2>/dev/null); [ -z \"\$p\" ] || kill -0 \$p 2>/dev/null || exit 2; sleep 0.1; done; exit 1" >/dev/null 2>&1
}

# tr_invoke: an arm at one stream count: each load's point in order; 1 when
# one failed (the rest still run)
tr_invoke() {
    local k rc=0 tag cpus label
    mode="$arm@STREAMS=$n"
    [[ $netem == - ]] || mode+="@NETEM=$netem"
    ((aa)) && mode+="@AA=1"
    tag="$sid-r$rep-$(file_mode "$mode")"
    label=$(tr_label "$sid" "$arm" "$aa" "$n" "$netem")
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
        if [[ $level == - || $level == none ]]; then
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
    done 9<"${rounds_file:-$dir/rounds.tsv}"
}

# tr_load_arms: TRANSPORT_ARMS and arms.txt from the plan's files (tr_files)
tr_load_arms() {
    local a
    TRANSPORT_ARMS=$(tr_plan arms) || return 1
    : >"$dir/arms.txt"
    for a in $(sed 's/=[^ ]*//g' <<<"$TRANSPORT_ARMS"); do echo "$a $(tr_plan arm "$a")" >>"$dir/arms.txt"; done
}

# tr_groups <rounds file>: its groups in order, each group's sets at once
# (their logs in logs/set-<id>.log and here: the operator's progress lines,
# their status in set-<id>.rc); 1 at the first group with a set that failed
# (keep-going: every group runs, 1 at the end; the tuning session's)
tr_groups() {
    local rounds_file=$1 keep=${2:-} gi pids s sets rc=0
    for gi in $(cut -f1 "$rounds_file" | uniq); do
        pids=()
        sets=$(awk -F'\t' -v g="$gi" '$1 == g && !seen[$2]++ {print $2}' "$rounds_file")
        for s in $sets; do
            { tr_set "$s" </dev/null; echo $? >"$dir/logs/set-$s.rc"; } 2>&1 | tee -a "$dir/logs/set-$s.log" >&2 &
            pids+=($!)
        done
        for s in "${pids[@]}"; do wait "$s"; done
        for s in $sets; do [[ $(cat "$dir/logs/set-$s.rc" 2>/dev/null) == 0 ]] || rc=1; done
        ((rc == 0)) || [[ -n $keep ]] || { log "a set of group $gi failed (logs/set-*.log)"; return 1; }
    done
    return $rc
}

# tr_tune: the tuning session (LAB_TUNE=1, pair tune: transport_tune.py).
# The sweep: every arm and knob of transport-settings.env's TR_SWEEP_* (less
# what LAB_TUNE_ARMS and LAB_TUNE_KNOBS leave out, and a knob or value a
# host's kernel lacks), one round each of its values and an A/A of its
# current value, one rep; then the confirmation: each knob whose sweep found
# a value outside the A/A band of the current one, its top two and the
# current value, TR_TUNE_CONFIRM_REPS reps; then the frozen choice
# (tuned.env, tune-report.md: transport_tune.py freeze), which the
# operator's lab.sh writes into transport-settings.env. rounds.tsv holds both
# phases' rounds.
tr_tune() {
    local rc=0 absent= f m v
    # what the hosts' kernels lack: the TCP timers (settings-*.txt), bbr
    grep -q '^net.ipv4.tcp_rto_min_us=absent$' "$dir/settings-a.txt" "$dir/settings-b.txt" 2>/dev/null && absent+="rto_min_us "
    for f in a b; do
        on "$(tr_host "$f")" "$vm cc-available" 2>/dev/null | grep -qw bbr || absent+="congestion=bbr "
    done
    [[ -z $absent ]] || log "tune: the hosts lack: $absent(not swept)"
    local files
    mapfile -t files < <(tr_files "${tr_arms_file:-}")
    tr_plan_files="$dir/tune-arms.tsv $dir/tune-sweep.tsv"
    python3 "$tr_here/transport_tune.py" sweep --settings "$tr_here/transport-settings.env" --out "$dir" \
        --pair "${tune_pair-${pair#tune-}}" --arms "${tune_arms:-}" --knobs "${tune_knobs:-}" --absent "$absent" "${files[@]}" >"$dir/logs/tune-sweep.log" 2>&1 ||
        { log "tune: no sweep plan: $(tail -2 "$dir/logs/tune-sweep.log")"; return 1; }
    tr_load_arms || { log "tune: the sweep plan's arms do not parse"; return 1; }
    # every variant's driver mode as modes.sh checks it (vm.sh would refuse it at its start)
    while read -r arm _ _ _ _; do
        read -r _ bin setup _ args <<<"$(grep -m1 "^$arm " "$dir/arms.txt")"
        read -ra args <<<"${args:-}"
        [[ $setup == kernel || $setup == dpdk ]] || continue
        m=$(tr_driver_mode)
        v=$(mode_check "$m" remote x86_64 azure '' 2>&1) || { log "tune: variant $arm's driver mode $m: $v"; return 1; }
    done <"$dir/variants.txt"
    arm=
    tr_plan rounds tune "$seed" 1 >"$dir/rounds-sweep.tsv" || { log "tune: the sweep's rounds do not parse"; return 1; }
    log "tune: sweep of $(grep -c . "$dir/variants.txt") variants: $(tr_plan count tune 1) points in $(grep -c . "$dir/rounds-sweep.tsv") rounds"
    cp "$dir/rounds-sweep.tsv" "$dir/rounds.tsv"
    tr_groups "$dir/rounds-sweep.tsv" keep-going || rc=1
    python3 "$tr_here/transport_collect.py" "$dir" >"$dir/logs/collect-sweep.log" 2>&1 || rc=1
    python3 "$tr_here/transport_tune.py" confirm "$dir" >"$dir/tune-confirm.tsv" 2>"$dir/logs/tune-confirm.log" ||
        { log "tune: the sweep's results do not decide ($dir/logs/tune-confirm.log): $(tail -2 "$dir/logs/tune-confirm.log")"; return 1; }
    if grep -q '^set ' "$dir/tune-confirm.tsv"; then
        tr_plan_files="$dir/tune-arms.tsv $dir/tune-confirm.tsv"
        tr_plan rounds tune "$seed" "${TR_TUNE_CONFIRM_REPS:-3}" >"$dir/rounds-confirm.tsv" || { log "tune: the confirmation's rounds do not parse"; return 1; }
        log "tune: confirmation of $(grep -c '^set ' "$dir/tune-confirm.tsv") knobs: $(tr_plan count tune "${TR_TUNE_CONFIRM_REPS:-3}") points"
        cat "$dir/rounds-confirm.tsv" >>"$dir/rounds.tsv"
        tr_groups "$dir/rounds-confirm.tsv" keep-going || rc=1
        python3 "$tr_here/transport_collect.py" "$dir" >"$dir/logs/collect-confirm.log" 2>&1 || rc=1
    else
        log "tune: no knob's sweep found a value outside the A/A band of its current one: nothing to confirm"
    fi
    python3 "$tr_here/transport_tune.py" freeze "$dir" "$session" >"$dir/tuned.env" 2>"$dir/logs/tune-freeze.log" ||
        { log "tune: no frozen choice ($dir/logs/tune-freeze.log): $(tail -2 "$dir/logs/tune-freeze.log")"; rc=1; }
    log "tune: frozen ($dir/tuned.env, tune-report.md): $(grep -c '^# changed' "$dir/tuned.env" 2>/dev/null || echo 0) knobs changed"
    return $rc
}

# tr_run <pair>: the session on VM a
tr_run() {
    pair=$1
    dir=$HOME/bench/transport-$pair
    # shellcheck source=/dev/null
    . "$dir/env"
    tr_arms_file=${arms_file:-}
    tr_sets_file=${sets_file:-}
    TRANSPORT_WARMUP_S=${TRANSPORT_WARMUP_S:-30} TRANSPORT_POINT_S=${TRANSPORT_POINT_S:-30} TRANSPORT_SWITCH_S=${TRANSPORT_SWITCH_S:-5}
    known=$dir/known_hosts
    ssh_key=${TRANSPORT_SSH_KEY:-$HOME/.ssh/lab}
    out=$dir
    cd "$HOME" || exit 1
    # shellcheck source=lib.sh
    . "$tr_here/lib.sh"
    # the media driver on TR_AERON_DIR, the directory the binaries are given
    # (vm.sh's own default is the latency lab's hugetlbfs mount)
    vm="LAB_AERON_DIR=$TR_AERON_DIR $vm"
    # shellcheck source=irqs.sh
    . "$tr_here/irqs.sh"
    # shellcheck source=host.sh
    . "$tr_here/host.sh"
    mkdir -p "$dir/manifest" "$dir/points" "$dir/logs" "$dir/irq" "$dir/netem" "$dir/samples" "$dir/nstat" "$dir/settings" "$dir/events" "$dir/aeronpage"
    # LAB_LOCAL_SMOKE: both hosts homes on this box (tr_smoke_hooks)
    [[ -z ${LAB_LOCAL_SMOKE:-} ]] || tr_smoke_hooks
    : >"$dir/failures.csv"
    local h rc=0
    tr_load_arms || die "the plan's arms do not parse"
    if [[ $pair == tune-? ]]; then
        log "session $session: the tuning session (LAB_TUNE=1), seed $seed"
    else
        tr_plan rounds "$pair" "$seed" "$reps" >"$dir/rounds.tsv" || die "the plan's rounds do not parse"
        log "session $session pair $pair: seed $seed, $(tr_plan count "$pair" "$reps") points in $(grep -c . "$dir/rounds.tsv") rounds"
    fi
    # the declared settings as sourced (TR_*); transport_collect.py writes
    # transport-settings.env, fairness.py's form of them
    cp "$tr_here/transport-settings.env" "$dir/transport-settings.tr.env"
    transport_sysctls >"$dir/declared-sysctls.txt"
    settings_sha=$(sha256sum "$dir/transport-settings.tr.env" 2>/dev/null | awk '{print $1}')
    # the map, the NICs, the binaries and the ergon-tune report of each host
    # (and one on the hugetlbfs Aeron directory, where an arm of the plan
    # runs there: its points' report); a report with any warning stops the
    # session before its first point
    local huge=
    [[ ! -s $dir/rounds.tsv ]] || huge=$(tr_huge_arms "$(cat "$dir/rounds.tsv")")
    arm=
    for h in a b; do
        on "$(tr_host "$h")" "$vm transport-map" >"$dir/map-$h.txt" 2>"$dir/logs/map-$h.err" || die "host $h: no transport map"
        on "$(tr_host "$h")" "$vm nics" >"$dir/nics-$h.txt" 2>/dev/null || die "host $h: data NICs not read"
        on "$(tr_host "$h")" "$vm settings" >"$dir/settings-$h.txt" 2>/dev/null || die "host $h: settings not read"
        on "$(tr_host "$h")" "sha256sum ergon/target/release/* extra/bin/* 2>/dev/null" >"$dir/bins-$h.txt" 2>/dev/null
        on "$(tr_host "$h")" "$vm netem clear" >>"$dir/logs/netem-$h.log" 2>&1 || die "host $h: a netem qdisc not cleared before the session"
        if ! on "$(tr_host "$h")" "$vm hostcheck" >"$dir/hostcheck-$h.txt" 2>&1; then
            # the smoke's box isolates nothing: its report stands, fairness.py flags it
            [[ -n ${LAB_LOCAL_SMOKE:-} ]] && { log "smoke: host $h's hostcheck report is not clean (no isolated CPUs here): recorded, not stopping"; continue; }
            die "host $h: the hostcheck report is not clean ($dir/hostcheck-$h.txt): $(grep -E '^[^ ]+ (warn|unknown): ' "$dir/hostcheck-$h.txt" | head -3 | tr '\n' ';') stopping before the first point"
        fi
        [[ -n $huge ]] || continue
        on "$(tr_host "$h")" "$vm hostcheck $TR_AERON_HUGE_DIR" >"$dir/hostcheck-$h-hugepages.txt" 2>&1 ||
            die "host $h: the hostcheck report on $TR_AERON_HUGE_DIR ($(echo $huge)) is not clean ($dir/hostcheck-$h-hugepages.txt): $(grep -E '^[^ ]+ (warn|unknown): ' "$dir/hostcheck-$h-hugepages.txt" | head -3 | tr '\n' ';') stopping before the first point"
    done
    cmp -s "$dir/map-a.txt" "$dir/map-b.txt" || die "the hosts' CPU maps differ: $(cat "$dir/map-a.txt") and $(cat "$dir/map-b.txt")"
    map=$(cat "$dir/map-a.txt")
    read -r hk _ iso <<<"$map"
    if [[ $pair == tune-? ]]; then
        tr_tune || rc=1
    else
        tr_groups "$dir/rounds.tsv" || rc=1
    fi
    for h in a b; do on "$(tr_host "$h")" "$vm netem clear" >>"$dir/logs/netem-$h.log" 2>&1 || rc=1; done
    [[ ! -s $dir/failures.csv ]] || { log "failed runs: $(grep -c . "$dir/failures.csv") (failures.csv)"; rc=1; }
    # the directory as transport-format.md has it, for fairness.py and analyse.py
    python3 "$tr_here/transport_collect.py" "$dir" >"$dir/logs/collect.log" 2>&1 ||
        { log "transport_collect.py failed ($dir/logs/collect.log): $(tail -2 "$dir/logs/collect.log" | tr '\n' ' ')"; rc=1; }
    log "session $session pair $pair: $(grep -c . "$dir/results.csv" 2>/dev/null || echo 0) rows, exit $rc"
    return $rc
}

# ---- LAB_LOCAL_SMOKE: the local end-to-end smoke (transport.sh smoke) ----

# tr_smoke_hooks: under LAB_LOCAL_SMOKE, in place of lib.sh's: `on` runs a
# command in the host's home on this box (A and B are host-a and host-b, under
# LOCAL_SMOKE_ROOT/hosts), and vm.sh is local-vm.sh (loopback addresses, one
# media driver for both hosts)
tr_smoke_hooks() {
    on() {
        local h=$1
        shift
        HOME=$LOCAL_SMOKE_ROOT/hosts/${h#host-} bash -c "cd; $*"
    }
    vm=ergon/md-dpdk/benches/cloud/local-vm.sh
    TR_AERON_DIR=${LOCAL_SMOKE_AERON_DIR:-/dev/shm/aeron-transport-smoke}
}

# tr_smoke <dir>: the smoke: hosts a and b as homes under <dir>/hosts (each
# ~/ergon this checkout, its target the build's), the settings applied
# (local-vm.sh transport-tune), transport.sh run a over transport-smoke.tsv
# at 1 rep with short durations, then fairness.py and analyse.py --transport
# into <dir>/fairness.txt and <dir>/tables.md; the run's rc
tr_smoke() {
    local root=$1 repo h e target rc
    mkdir -p "$root" && root=$(cd "$root" && pwd) || return 2
    repo=$(cd "$tr_here/../../.." && pwd)
    target=${CARGO_TARGET_DIR:-$repo/target}
    for b in aeron_lat mpsc_lat udp_lat tcp_lat hostcheck media_driver; do
        [[ -x $target/release/$b ]] || { echo "smoke: no $target/release/$b (cargo build --release first: test/transport-smoke-docker.sh)" >&2; return 2; }
    done
    rm -rf "$root/hosts"
    for h in a b; do
        mkdir -p "$root/hosts/$h/ergon" "$root/hosts/$h/bench"
        for e in "$repo"/*; do [[ ${e##*/} == target ]] || ln -s "$e" "$root/hosts/$h/ergon/${e##*/}"; done
        ln -s "$target" "$root/hosts/$h/ergon/target"
    done
    export LAB_LOCAL_SMOKE=1 LOCAL_SMOKE_ROOT=$root
    for h in a b; do
        HOME=$root/hosts/$h bash "$tr_here/local-vm.sh" transport-tune >"$root/transport-tune-$h.log" 2>&1 ||
            { echo "smoke: host $h's settings did not hold ($root/transport-tune-$h.log): $(tail -3 "$root/transport-tune-$h.log")" >&2; return 1; }
    done
    # LAB_TUNE=1: the tuning session instead, every arm on this one box
    # (tune_pair empty), cut down to TR_SMOKE_TUNE_ARMS and _KNOBS unless
    # LAB_TUNE_ARMS and LAB_TUNE_KNOBS say otherwise
    local p=a
    [[ ${LAB_TUNE:-} == 1 ]] && p=tune-a
    mkdir -p "$root/hosts/a/bench/transport-$p"
    cat >"$root/hosts/a/bench/transport-$p/env" <<ENV
A=host-a
B=host-b
user=$(id -un)
pair=$p
tune_pair=
tune_arms=${LAB_TUNE_ARMS-$TR_SMOKE_TUNE_ARMS}
tune_knobs=${LAB_TUNE_KNOBS-$TR_SMOKE_TUNE_KNOBS}
session=smoke-$(date +%Y%m%d-%H%M%S)
seed=${LAB_SEED:-$RANDOM}
reps=1
vm_a=smoke/a
vm_b=smoke/b
commit=$(git -C "$repo" rev-parse HEAD 2>/dev/null)
dirty=$(git -C "$repo" status --porcelain 2>/dev/null | grep -c .)
arms_file=
TRANSPORT_WARMUP_S=${TRANSPORT_WARMUP_S:-2}
TRANSPORT_POINT_S=${TRANSPORT_POINT_S:-3}
TRANSPORT_SWITCH_S=${TRANSPORT_SWITCH_S:-1}
ENV
    HOME=$root/hosts/a bash "$tr_here/transport.sh" run "$p" 2>&1 | tee "$root/run.log" >&2
    rc=$(cat "$root/hosts/a/bench/transport-$p/rc" 2>/dev/null || echo 1)
    ln -sfn "hosts/a/bench/transport-$p" "$root/run"
    python3 "$tr_here/fairness.py" "$root/run" >"$root/fairness.txt" 2>&1
    python3 "$tr_here/analyse.py" --transport --show-invalid "$root/run" >"$root/tables.md" 2>&1 || rc=1
    tr_smoke_summary "$root" || rc=1
    return "$rc"
}

# tr_smoke_summary <dir>: the smoke's verdict, into <dir>/summary.txt and
# stdout: every point a row (a one-host point two: one way and the round
# trip), no failed run, the collector's problems none, fairness.py's
# violations counted by rule, the tables rendered; 1 when any but the
# fairness count is off (the box isolates nothing: its fairness violations
# are listed for a reader to explain)
tr_smoke_summary() {
    local root=$1 run=$1/run want got fails tables bad=0
    want=$(awk -F'\t' '{split($7, l, ","); split($8, st, ","); split($9, a, ","); n += length(l) * length(st) * length(a) * ($3 == "ab" ? 1 : 2)} END {print n + 0}' "$run/rounds.tsv")
    got=$(grep -c . "$run/results.csv" 2>/dev/null)
    fails=$(grep -c . "$run/failures.csv" 2>/dev/null)
    tables=$(grep -c '^### Set ' "$root/tables.md" 2>/dev/null)
    got=${got:-0} fails=${fails:-0} tables=${tables:-0}
    {
        echo "smoke: $got rows of $want expected, $fails failed runs, $tables tables"
        echo "collect: $(head -1 "$run/logs/collect.log" 2>/dev/null)"
        echo "fairness: $(tail -1 "$root/fairness.txt"), by rule:"
        sed -n 's/^\([a-z-]*\): .*/\1/p' "$root/fairness.txt" | sort | uniq -c | sed 's/^/  /'
        echo "maxima: $(python3 "$tr_here/spikes.py" --transport "$run" 2>/dev/null | sed -n 's/^By cause: //p')"
        if [[ -e $run/tune-sweep.tsv ]]; then
            echo "tune: $(grep -c '^set ' "$run/tune-sweep.tsv") swept, $(grep -c '^set ' "$run/tune-confirm.tsv" 2>/dev/null || echo 0) confirmed, $(grep -c '^# changed' "$run/tuned.env" 2>/dev/null || echo 0) changed (tuned.env, tune-report.md)"
            grep -E '^(TR_|# changed|# kept)' "$run/tuned.env" 2>/dev/null | sed 's/^/  /'
        fi
    } | tee "$root/summary.txt"
    [[ $got == "$want" && $fails == 0 && $tables -gt 0 ]] || bad=1
    [[ ! -e $run/tune-sweep.tsv || -s $run/tuned.env ]] || bad=1
    grep -q ' 0 problems$' "$run/logs/collect.log" 2>/dev/null || bad=1
    return $bad
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
    case ${1:-} in
    check) tr_check "${2:?pair}" "${3:-}" ;;
    smoke) tr_smoke "${2:?a directory for the smoke}" ;;
    run)
        log() { echo "[$(date +%T)] $*" >&2; }
        die() { log "ERROR: $*"; exit 1; }
        tr_pair=${2:?pair}
        dir=$HOME/bench/transport-$tr_pair
        # on any exit: no bench binary or driver left on either VM, then rc
        # on any exit: the bench binaries this runner started (their PIDs, as
        # tr_start recorded them: never by name, which would stop another's),
        # netem and the media driver, on both VMs; then rc
        trap 'rc=$?; for h in "${A:-}" "${B:-}"; do [[ -n $h ]] && on "$h" "for f in bench/transport-$tr_pair/run-*/*.pid; do [ -s \$f ] && kill -TERM \$(cat \$f) 2>/dev/null; done; ${vm:-ergon/md-dpdk/benches/cloud/vm.sh} events-stop-all; ${vm:-ergon/md-dpdk/benches/cloud/vm.sh} netem clear; ${vm:-ergon/md-dpdk/benches/cloud/vm.sh} stop" >/dev/null 2>&1 </dev/null; done; kill $(jobs -p) 2>/dev/null; echo $rc >"$dir/rc"' EXIT
        tr_run "$2"
        ;;
    *) awk 'NR > 1 && !/^#/ {exit} NR > 1 {print}' "$0" >&2; exit 2 ;;
    esac
fi
