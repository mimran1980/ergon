# Shared by md-dpdk/benches/cloud/azure/lab.sh and md-dpdk/benches/cloud/aws/lab.sh, sourced
# after the cloud's VMs exist. The caller sets: A and B (the VMs' public
# addresses), user (their admin user), out (the results directory), known (its
# known_hosts), data_subnet, reps, modes and states (arrays), rtt_messages,
# rtt_warmup, tput_seconds, a_private and b_private (the VMs' management
# addresses, which VM a drives the benchmark over); optionally placement (AWS:
# precision-time turns on the PTP hardware clock and NIC RX timestamps;
# cross-az and cross-region put the hosts in two zones or regions, each data
# subnet its own, data_subnet_a and data_subnet_b),
# ssh_key, base (LAB_BASE: a git revision of the checkout built next to it,
# into ~/bin-base on each VM, for modes with @BIN=bin-base), tput_flows
# (LAB_TPUT_FLOWS: flows in the multi-flow sustained maximum, which runs only
# from 2), local_modes and allow_arm (AWS LAB_ALLOW_ARM=1: vm.sh bootstraps
# an aarch64 host, Graviton, only with it), no_flatout (LAB_NO_FLATOUT=1: no
# phase that sends as fast as it can: no 32 B throughput run, no sustained
# maximum, no several flows; the load sweep runs its ladder alone), max_hours
# (LAB_MAX_HOURS: the instances' deadline, how long a state is waited for),
# warmup_seconds and warmup_rate (LAB_WARMUP_SECONDS, LAB_WARMUP_RATE: a
# warm-up load before each mode's first ladder, recorded nowhere),
# rtt_spike_ns and rtt_stall_ns (LAB_RTT_SPIKE_NS, 30000, and
# LAB_RTT_STALL_NS, 10000: the cross-host round trip's clients stamp every
# round trip and every stall of their own loop of at least that many ns, so
# each idle max, 72-79 us in G3, gets a cause; the clients' own defaults,
# 100 and 20 us, would leave it unstamped).
# The functions run the benchmark on both VMs in lockstep; teardown stays with
# the caller's trap.

# a mode's settings (JUMBO, its largest message), read as lab.sh checked them
# shellcheck source=modes.sh
. "$(dirname "${BASH_SOURCE[0]}")/modes.sh"
ssh_opts=(-o BatchMode=yes -o StrictHostKeyChecking=accept-new -o "UserKnownHostsFile=$known"
    -o ConnectTimeout=15 -o ServerAliveInterval=30 -o ServerAliveCountMax=10)
[[ -n ${ssh_key:-} ]] && ssh_opts+=(-i "$ssh_key")
vm=ergon/md-dpdk/benches/cloud/vm.sh

# on <host> <command>: over SSH, the remote command's own exit status
# returned as is. The remote shell says first that it ran (on_ran, on
# stderr, which on_said takes out of what it passes on), so a command is
# never run twice: a 255 before it said so is a connection failure (this
# machine's link dropping or failing over), retried for about 3 minutes; a
# 255 after (the remote shell killed by a signal, or its session lost
# mid-way) is a lab fault, logged (lab_fault) and returned, never run again:
# a client started twice would hold the ports of the one still running. The
# remote shell takes SIGTERM (a pkill -f naming a client's words matches its
# command line too; the lab's own, end_client, names the client's path) by
# waiting for its command, whose status it then returns, so ending a client
# is never a 255. Once the run is over
# (lab_abandon, aws/lab.sh's file: an instance AWS reclaimed), nothing more
# is tried: 255 at once, in every subshell too, so no step waits out its
# retries before the teardown.
on_ran='lab on(): the remote command started'
on() {
    local host=$1 rc=255 try said cmd
    shift
    # the words joined by spaces, as ssh joins them
    printf -v cmd '%s ' "$@"
    cmd=${cmd% }
    for try in $(seq 18); do
        [[ -n ${lab_abandon:-} && -e $lab_abandon ]] && return 255
        # stdout as it comes (fd 3); stderr through on_said
        {
            ssh "${ssh_opts[@]}" "$user@$host" "trap : TERM; echo '$on_ran' >&2; $cmd" 2>&1 1>&3 3>&- | on_said >&2 3>&-
            rc=${PIPESTATUS[0]} said=${PIPESTATUS[1]}
        } 3>&1
        ((rc != 255)) && return $rc
        if ((said == 0)); then
            lab_fault "$host: '$cmd' ran, and its SSH session ended 255 (the remote shell killed by a signal, or the session lost): not run again"
            return 255
        fi
        sleep 10
    done
    return $rc
}

# on_said: on()'s filter of the remote side's stderr: every line passed on as
# it comes but the first on_ran, whose coming is its status (0: it came)
on_said() {
    local line seen=1
    while IFS= read -r line || [[ -n $line ]]; do
        if ((seen)) && [[ $line == "$on_ran" ]]; then
            seen=0
        else
            printf '%s\n' "$line"
        fi
    done
    return $seen
}

# lab_fault <what>: a fault of the lab itself, not of what it measures (a
# client past its deadline, an SSH session lost or hung): logged, and kept
# in $out/lab-faults.log, which a log sent elsewhere (on() 2>/dev/null) does
# not lose
lab_fault() {
    log "LAB FAULT: $*"
    [[ -n ${out:-} && -d $out ]] && echo "$(date +%T) $*" >>"$out/lab-faults.log"
    return 0
}

# A benchmark client's lab-side deadline, in seconds from its start, which
# vm.sh client enforces on its VM (LAB_CLIENT_DEADLINE: SIGTERM then, SIGKILL
# its grace, 10 s, later, and it exits 124), so a client that hangs (an echo
# stuck back-pressured, a lost peer) never holds a run up.
# client_deadline <seconds>: a client whose own run is <seconds> (a ping's
# or a publisher's schedule): twice that and a minute (a client waits up to
# a minute for its peer), so only a client that hangs meets it
client_deadline() { echo $((2 * ($1 + 1) + 60)); }
# answer_deadline <seconds>: a client that answers one of client_deadline
# <seconds> (a pong, an echo, a throughput subscriber), started before it:
# that deadline and half a minute
answer_deadline() { echo $(($(client_deadline "$1") + 30)); }
# client_bound <deadline>: how long a wait on that client's SSH job goes on
# (wait_bounded): its deadline, its grace and a minute for SSH
client_bound() { echo $(($1 + 70)); }

# wait_bounded <seconds> <what> <pid>...: waits, as wait does (the last
# one's status), for local jobs running clients (on ... &), each ended on
# its VM by its deadline. One that ran past it (124) is a lab fault; one
# still running <seconds> from now (to the second above: never sooner; its
# SSH session hung) is too, and it and its ssh are ended here.
wait_bounded() {
    local bound=$1 what=$2 pid rc=0 until
    shift 2
    until=$((SECONDS + bound + 1))
    for pid in "$@"; do
        while kill -0 "$pid" 2>/dev/null && ((SECONDS < until)); do sleep 0.1; done
        if kill -0 "$pid" 2>/dev/null; then
            lab_fault "$what: still running $bound s on (its SSH session hung): ended here"
            end_tree "$pid"
        fi
        wait "$pid" 2>/dev/null
        rc=$?
        ((rc == 124)) && lab_fault "$what: overran its deadline: vm.sh client ended it (124)"
    done
    return $rc
}

# client_rc <what> <status>: a client run in the foreground (on ...) that
# ran past its deadline (124) is a lab fault
client_rc() {
    (($2 == 124)) && lab_fault "$1: overran its deadline: vm.sh client ended it (124)"
    return 0
}

# end_tree <pid>: SIGTERM to a local job and every process under it (its
# ssh), by PID, the job first so it starts nothing more
end_tree() {
    local pids=("$1") i=0 child
    while ((i < ${#pids[@]})); do
        for child in $(pgrep -P "${pids[i]}" 2>/dev/null); do pids+=("$child"); done
        i=$((i + 1))
    done
    kill "${pids[@]}" 2>/dev/null
    return 0
}

# end_client <host> <prog> <mode>: SIGTERM, on <host>, to its client <prog>
# <mode> (a pong, an echo, an idle subscription), named by the program's
# path ("/rtt xpong": vm.sh client starts it by path), which names only the
# client, its timeout(1) and the sudo above them: never an ssh whose remote
# command is vm.sh client's (on VM a, which runs the lab and its localhost
# clients both, the lab's own: SIGTERM would end it 255, a lab fault, the
# client's output cut). Its first letter bracketed, so that pkill's own
# command line does not match.
end_client() {
    on "$1" "sudo pkill -f '/[${2:0:1}]${2:1} $3'" 2>/dev/null || true
}

# both <what> <vm.sh args>: on both VMs in parallel, logging to $out/<node>-<what>.log
# With rtt_align (LAB_RTT_ALIGN), wait until 2 s before that second of the
# minute: the pong starts, and the ping 2 s later starts on it. A host event
# that recurs at a fixed second each minute (Azure: a whole-VM pause near :57)
# then falls outside every mode's run alike instead of in whichever it hits.
align_rtt() {
    [[ -n ${rtt_align:-} ]] || return 0
    sleep "$(python3 -c 'import sys, time; print(round((float(sys.argv[1]) - 2 - time.time()) % 60, 3))' "$rtt_align")"
}

both() {
    local what=$1; shift
    on "$A" "$vm $*" >"$out/a-$what.log" 2>&1 & local p1=$!
    on "$B" "$vm $*" >"$out/b-$what.log" 2>&1 & local p2=$!
    local rc=0
    wait $p1 || { rc=1; log "a: vm.sh $* failed ($out/a-$what.log)"; tail -20 "$out/a-$what.log" >&2; }
    wait $p2 || { rc=1; log "b: vm.sh $* failed ($out/b-$what.log)"; tail -20 "$out/b-$what.log" >&2; }
    return $rc
}

# snap <what> [nodes]: vm.sh counters on VM a and VM b (or the nodes given) at
# once, each to $out/<node>-counters-<what>.log under a "# realtime=" line:
# taken just before and after a flat-out run, what the transport and the NICs
# counted over it (analyse.py's throughput phase frames)
snap() {
    local what=$1 node host pids=()
    for node in ${2:-a b}; do
        [[ $node == a ]] && host=$A || host=$B
        { echo "# realtime=$(date +%s.%N)"; on "$host" "$vm counters" 2>&1; } >"$out/$node-counters-$what.log" & pids+=($!)
    done
    wait "${pids[@]}" 2>/dev/null || true
}

# allowance_grew <before> <after>: the NIC allowance counters (ENA's
# *_allowance_exceeded: bw_in, bw_out, pps, conntrack, linklocal; packets
# AWS shaped or dropped) that grew between two snap snapshots, as "<role>
# <counter> +<growth>" joined by ", ": a kernel mode's from ethtool
# ("<role> <counter>: <value> ..."), a DPDK mode's from the PMD's xstats
# among the transport's counters ("<value> 92 role=<role> <counter>").
# Nothing where none grew, a snapshot is missing or lacks the counter, or
# the two read different embedded drivers (INVOKER: "cnc <id>")
allowance_grew() {
    python3 - "$1" "$2" <<'PY'
import re, sys
def read(path):
    out, cnc = {}, None
    try:
        lines = open(path, errors="replace").read().splitlines()
    except OSError:
        return None, None
    for line in lines:
        words = line.split()
        if line.startswith("#") or not words:
            continue
        if len(words) == 2 and words[0] == "cnc":
            cnc = words[1]
        elif words[0] in ("sender", "receiver"):
            for k, v in re.findall(r"(\w+allowance_exceeded): (\d+)", line):
                out[words[0], k] = out.get((words[0], k), 0) + int(v)
        else:
            for v, role, k in re.findall(r"(\d+) 92 role=(\w+) (\w+allowance_exceeded)\b", line):
                out[role, k] = out.get((role, k), 0) + int(v)
    return out, cnc
(b, b_cnc), (a, a_cnc) = read(sys.argv[1]), read(sys.argv[2])
if b is not None and a is not None and b_cnc == a_cnc:
    print(", ".join(f"{r} {k} +{a[r, k] - b[r, k]}" for r, k in sorted(a) if (r, k) in b and a[r, k] > b[r, k]))
PY
}

# point_counters <before|after> <tag> <nodes> [<what>]: snap around one load
# point (load_sweep, mix_run) on its hosts, to
# $out/<node>-counters-load-<before|after>-<tag>.log; after, each host whose
# NIC allowance counters grew over it (allowance_grew) is named in a warning
# that says what (<what>)
point_counters() {
    local when=$1 tag=$2 nodes=$3 what=${4:-$2} node grew
    snap "load-$when-$tag" "$nodes"
    [[ $when == after ]] || return 0
    for node in $nodes; do
        grew=$(allowance_grew "$out/$node-counters-load-before-$tag.log" "$out/$node-counters-load-after-$tag.log")
        [[ -z $grew ]] || log "WARNING: $what: host $node's NIC allowance exceeded over the point (packets AWS shaped or dropped): $grew"
    done
}

# file_mode <mode>: a mode as file names spell it, a / (BIN's directory) as _
file_mode() { printf '%s' "${1//\//_}"; }

# detached <what> <vm.sh args>: like both, but each VM runs the step under
# nohup and we poll for its exit code, so a dropped SSH session (a NIC
# reconfiguring during apt, say) cannot kill a long step
detached() {
    local what=$1; shift
    local host node rc=0 status
    for host in "$A" "$B"; do
        on "$host" "rm -f bench/$what.rc; nohup bash -c '$vm $*; echo \$? >bench/$what.rc' >bench/$what.log 2>&1 </dev/null &" ||
            { log "cannot start vm.sh $* on $host"; return 1; }
    done
    for host in "$A" "$B"; do
        node=a; [[ $host == "$B" ]] && node=b
        status=
        for _ in $(seq 720); do
            status=$(on "$host" "cat bench/$what.rc 2>/dev/null" 2>/dev/null) && [[ -n $status ]] && break
            sleep 10
        done
        on "$host" "cat bench/$what.log" >"$out/$node-$what.log" 2>&1 || true
        if [[ $status != 0 ]]; then
            rc=1
            log "$node: vm.sh $* failed (exit ${status:-unknown}; $out/$node-$what.log)"
            tail -20 "$out/$node-$what.log" >&2
        fi
    done
    return $rc
}

wait_ssh() {
    local host
    for host in "$A" "$B"; do
        for _ in $(seq 120); do on "$host" true 2>/dev/null && break; sleep 5; done
        on "$host" true || die "no SSH to $host"
    done
}

# host_subnets <host>: "<its data subnet> <the other host's, when off it>"
# (cross-AZ, cross-region: lab.sh's data_subnet_a and data_subnet_b; one
# subnet for both otherwise, data_subnet)
host_subnets() {
    local own=${data_subnet_a:-$data_subnet} other=${data_subnet_b:-$data_subnet}
    [[ $1 == "$A" ]] || { local t=$own; own=$other; other=$t; }
    [[ $other != "$own" ]] || other=
    echo "$own${other:+ $other}"
}

# record_base_rtt: the network's own round trip between the hosts' data NICs
# (vm.sh base-rtt, a's sender to b's receiver) in both hosts' environment
# records (env-<host>.txt)
record_base_rtt() {
    local rcv net host
    rcv=$(on "$B" "$vm nics" 2>/dev/null | awk '$1 == "receiver" {print $5}')
    net=$(on "$A" "$vm base-rtt $rcv" 2>/dev/null) || net=
    for host in "$A" "$B"; do
        echo "base-rtt a's sender -> b's receiver $rcv (ping, ns: min avg max mdev, replies/sent) ${net:-not measured}" >>"$out/env-$host.txt"
    done
}

# record_market: the instances' market (aws/lab.sh: spot with LAB_SPOT=1,
# else on-demand) in both hosts' environment records (env-<host>.txt);
# nothing where the cloud gives none (Azure)
record_market() {
    local host line
    [[ -n ${market:-} ]] || return 0
    line="market $market"
    [[ $market == spot ]] && line+=" (AWS spot instances: AWS may reclaim them mid-run)"
    for host in "$A" "$B"; do echo "$line" >>"$out/env-$host.txt"; done
}

# record_knoise <state>: what each host's diag-start said of KNOISE (vm.sh
# knoise_start: osnoise on the agent and app CPUs, or "osnoise unavailable:
# <why>", a limit of the run's diagnostics, and any event its kernel lacks)
# in its environment record (env-<host>.txt), with the state
record_knoise() {
    local state=$1 node host lines
    for node in a b; do
        [[ $node == a ]] && host=$A || host=$B
        if [[ ! -e $out/$node-diag-start-$state.log ]]; then
            lines="knoise not recorded (no diag-start log)"
        else
            lines=$(grep -E '^(osnoise|knoise) ' "$out/$node-diag-start-$state.log") ||
                lines="knoise not recorded (diag-start said nothing of it)"
        fi
        awk -v s="$state" '{print s ": " $0}' <<<"$lines" >>"$out/env-$host.txt"
    done
}

# reboot both VMs and wait for them to come back
reboot_both() {
    local host
    for host in "$A" "$B"; do on "$host" 'sudo systemctl reboot' 2>/dev/null || true; done
    sleep 30
    wait_ssh
}

# copy the checkout (tracked and new files; not the upstream submodules), and
# with base (LAB_BASE) that revision's tracked files (git archive: the VMs get
# no git history), then bootstrap, reboot into the backports kernel and build
prepare_vms() {
    wait_ssh
    log "copying the checkout"
    # a file git tracks but the working tree has deleted would make tar fail and
    # cost the whole run (the VMs are up by then, and were billed for nothing):
    # it is left out of the tarball and named here instead
    local list=$out/pack.list missing=() f
    (cd "$repo" && git ls-files -co --exclude-standard | grep -v -e '^aeron$' -e '^simple-binary-encoding$') >"$list" ||
        die "cannot list the checkout"
    while IFS= read -r f; do
        [[ -f $repo/$f ]] || missing+=("$f")
    done <"$list"
    ((${#missing[@]})) && log "WARNING: left out of the checkout, absent from the working tree: ${missing[*]}"
    (cd "$repo" && while IFS= read -r f; do if [[ -f $f ]]; then printf '%s\n' "$f"; fi; done <"$list" |
        COPYFILE_DISABLE=1 tar --no-xattrs -cf - -T -) >"$out/ergon.tar" || die "cannot pack the checkout"
    local tars=(ergon.tar)
    if [[ -n ${base:-} ]]; then
        git -C "$repo" rev-parse --verify "$base^{commit}" >"$out/base-commit" || die "LAB_BASE $base: not a commit"
        git -C "$repo" archive --format=tar "$base" >"$out/ergon-base.tar" || die "cannot pack LAB_BASE $base"
        tars+=(ergon-base.tar)
    fi
    local host tar own peer want_ynl=
    # a mode busy-polling its NAPIs (NAPI_BUSY) needs the kernel's ynl: vm.sh
    # build fetches it (build-ynl) only where bench/want_ynl says 1
    if modes_want_ynl "${modes[@]}" ${local_modes:-}; then want_ynl=1; fi
    for host in "$A" "$B"; do
        read -r own peer <<<"$(host_subnets "$host")"
        for tar in "${tars[@]}"; do
            local try copied=
            for try in 1 2 3 4 5; do
                scp -q "${ssh_opts[@]}" "$out/$tar" "$user@$host:$tar" && { copied=1; break; }
                sleep 15
            done
            [[ -n $copied ]] || die "copy of $tar to $host failed"
        done
        on "$host" "rm -rf ergon bench && mkdir -p ergon bench && echo $own >bench/data_subnet${peer:+ && echo $peer >bench/peer_subnet} && echo ${placement:-} >bench/placement && echo ${allow_arm:-} >bench/allow_arm && echo $want_ynl >bench/want_ynl && tar -xf ergon.tar -C ergon && rm ergon.tar" ||
            die "unpack on $host failed"
        if [[ -n ${base:-} ]]; then
            on "$host" "rm -rf ergon-base bin-base && mkdir -p ergon-base && tar -xf ergon-base.tar -C ergon-base && rm ergon-base.tar" ||
                die "unpack of LAB_BASE on $host failed"
        fi
    done
    git -C "$repo" rev-parse HEAD >"$out/commit"
    git -C "$repo" status --porcelain >"$out/dirty"
    log "bootstrap"
    detached bootstrap bootstrap || die "bootstrap failed"
    reboot_both
    log "build"
    detached build build || die "build failed"
    dpdk_ready
    modes_ready
    if [[ -n ${base:-} ]]; then
        log "build LAB_BASE $base ($(cat "$out/base-commit")) into ~/bin-base"
        detached build-base build-base || die "LAB_BASE build failed"
    fi
    # precision-time: boot with the PHC turned on (ergon-lab-phc.service)
    # and chrony on it, as the build left them
    [[ ${placement:-} == precision-time ]] && reboot_both
    both tune-boot tune || die "tune failed"
    # each host's environment, with the baseline every mode runs on top of
    # (once tune has set it) and the base build, if any
    for host in "$A" "$B"; do
        on "$host" "echo \"kernel \$(uname -r)\"; echo \"dpdk \$(pkg-config --modversion libdpdk)\";
            echo \"cpu \$(lscpu | sed -n 's/^Model name: *//p')\"; echo \"cpus \$(nproc)\";
            echo \"rustc \$(~/.cargo/bin/rustc -V)\"; $vm layout; $vm nics; $vm ptp; $vm baseline; $vm host" >"$out/env-$host.txt" 2>&1
        if [[ -n ${base:-} ]]; then
            echo "base $base $(cat "$out/base-commit") in ~/bin-base (@BIN=bin-base)" >>"$out/env-$host.txt"
        fi
    done
    record_market
    record_base_rtt
    both clockcheck-boot clockcheck 120 || log "WARNING: clockcheck failed"
    both criterion-boot criterion || log "WARNING: criterion failed"
}

# the benchmark: every state in $states, each state's reps run on VM a by
# remote.sh (bench_state) over the private network, so this machine's
# connection only has to start a state and collect it; the isolated states'
# reboots are driven from here: isolated, and isolated-poll, the same with
# idle CPUs polling rather than halting (vm.sh isolate poll; Graviton's only
# way to keep CPUs out of idle, nohlt). Results to $out/results.csv.
run_bench() {
    results=$out/results.csv
    if ((reps == 0)); then
        log "LAB_REPS=0: VMs built, no benchmark run (a=$A b=$B, known_hosts $known)"
        return 0
    fi
    remote_access
    local state
    for state in "${states[@]}"; do
        if [[ $state == isolated || $state == isolated-poll ]]; then
            local variant=
            [[ $state == isolated-poll ]] && variant=poll
            log "state $state: kernel command line, reboot"
            both "isolate${variant:+-$variant}" isolate $variant || die "isolate failed"
            reboot_both
            both "tune-$state" tune || die "tune failed"
            both "clockcheck-$state" clockcheck 120 || log "WARNING: clockcheck failed"
            both "criterion-$state" criterion || log "WARNING: criterion failed"
        fi
        # a host that stalls the VM shows up here, before hours are spent on it
        both "preflight-$state" preflight 120 || log "WARNING: preflight failed"
        grep -h "on_several_cpus" "$out/a-preflight-$state.log" "$out/b-preflight-$state.log" >&2 || true
        # both clocks settled before anything is measured (vm.sh timesync-wait:
        # on AWS chronyc waitsync, which bounds the correction still to apply,
        # not the error against UTC); every boot (each isolated state's) starts
        # chrony anew. Spikes are matched across the hosts' clocks.
        both "timesync-$state" timesync-wait ||
            die "state $state: chrony did not synchronise both hosts' clocks (vm.sh timesync-wait; $out/{a,b}-timesync-$state.log)"
        both "diag-start-$state" diag-start || log "WARNING: diagnostics did not start"
        record_knoise "$state"
        remote_state "$state" || die "$state benchmark failed"
        both "diag-$state" diag-fetch || log "WARNING: diagnostics not fetched"
        both "diag-stop-$state" diag-stop || true
    done
    python3 "$repo/md-dpdk/benches/cloud/analyse.py" "$results" >"$out/summary.md"
    # every spike and every rep's max, attributed: host stall, guest preemption or unexplained
    for state in "${states[@]}"; do
        python3 "$repo/md-dpdk/benches/cloud/spikes.py" "$out/$state" >>"$out/summary.md" 2>&1 || true
    done
    cat "$out/summary.md"
}

# VM a's own key, authorised on both VMs, for remote.sh
remote_access() {
    local pub host
    on "$A" '[[ -f ~/.ssh/lab ]] || ssh-keygen -q -t ed25519 -N "" -f ~/.ssh/lab' || die "VM a key"
    pub=$(on "$A" 'cat ~/.ssh/lab.pub') || die "VM a key"
    for host in "$A" "$B"; do
        on "$host" "grep -qF '$pub' ~/.ssh/authorized_keys || echo '$pub' >>~/.ssh/authorized_keys" || die "authorise VM a on $host"
    done
}

# dpdk_ready: after the build, whether each host's media driver has the DPDK
# transport. vm.sh build on aarch64 (Graviton) builds the driver without it
# should md-dpdk's dpdk feature not build there, and says why in
# ~/bench/dpdk_unavailable. A DPDK mode, cross-host or localhost, would fail
# at its first driver start, hours in: the run stops here instead, before any
# state. With kernel modes only it is noted and the run goes on.
dpdk_ready() {
    local host why dpdk=0 m
    for m in "${modes[@]}" ${local_modes:-}; do
        [[ ${m%%@*} == dpdk* ]] && dpdk=1
    done
    for host in "$A" "$B"; do
        why=$(on "$host" "cat bench/dpdk_unavailable 2>/dev/null")
        [[ -n $why ]] || continue
        if ((dpdk)); then
            die "$host: its media driver has no DPDK transport ($why): drop the DPDK modes, or run them where the dpdk feature builds"
        fi
        log "$host: its media driver has no DPDK transport ($why); kernel modes only, as asked"
    done
}

# modes_ready: after the build, whether each host can run every mode's
# settings (vm.sh ready: a fifth core for @IRQ=spare and @NAPI_BUSY=spare,
# Linux 6.17 and the kernel's ynl for @NAPI_BUSY). A mode that could not
# would fail at its first driver start, hours in: the run stops here
# instead, before any state, saying why.
modes_ready() {
    local host why
    for host in "$A" "$B"; do
        why=$(on "$host" "$vm ready ${modes[*]} ${local_modes:-}" 2>&1) || die "$host: ${why:-vm.sh ready failed}"
    done
}

# remote_state <state>: run remote.sh for the state on VM a, detached; poll
# until it finishes, then fetch its directory into $out and its results
remote_state() {
    local state=$1 dir=bench/remote-$1 status= last= line
    on "$A" "rm -rf $dir && mkdir -p $dir && cat >$dir/env" <<ENV || return 1
A=$a_private
B=$b_private
user=$user
reps=$reps
modes='${modes[*]}'
local_modes='${local_modes:-}'
load_sizes='${load_sizes:-}'
load_fractions='${load_fractions:-10 50 80 95 100}'
load_ladder='${load_ladder:-}'
load_seconds=${load_seconds:-5}
mix='${mix:-}'
mix_seconds=${mix_seconds:-10}
rtt_messages=$rtt_messages
rtt_warmup=$rtt_warmup
rtt_align='${rtt_align:-}'
tput_seconds=$tput_seconds
tput_flows='${tput_flows:-}'
no_flatout='${no_flatout:-}'
placement='${placement:-}'
warmup_seconds=${warmup_seconds:-0}
warmup_rate=${warmup_rate:-0}
rtt_spike_ns='${rtt_spike_ns:-30000}'
rtt_stall_ns='${rtt_stall_ns:-10000}'
ENV
    on "$A" "nohup bash ergon/md-dpdk/benches/cloud/remote.sh $1 >$dir/run.log 2>&1 </dev/null &" || return 1
    log "state $state: running on VM a ($dir)"
    mkdir -p "$out/$state"
    # as long as the instances live (max_hours, lab.sh's LAB_MAX_HOURS; 8 h
    # without it): load sweeps are long, and a state given up on early would
    # fail a run its deadline allows
    for i in $(seq $((${max_hours:-8} * 360))); do
        sleep 10
        status=$(on "$A" "cat $dir/rc 2>/dev/null" 2>/dev/null) && [[ -n $status ]] && break
        # every 5 minutes, the results so far: an interrupted run keeps them
        ((i % 30 == 0)) && on "$A" "cat $dir/results.csv 2>/dev/null" >"$out/$state/results.partial.csv" 2>/dev/null
        line=$(on "$A" "grep '^\[' $dir/run.log | tail -1" 2>/dev/null) || continue
        [[ -n $line && $line != "$last" ]] && { echo "  a: $line" >&2; last=$line; }
    done
    mkdir -p "$out/$state"
    on "$A" "tar -C $dir -cf - ." | tar -xf - -C "$out/$state" || log "WARNING: could not fetch $dir"
    cat "$out/$state/results.csv" >>"$results" 2>/dev/null
    [[ $status == 0 ]] || { log "state $state on VM a: exit ${status:-unknown} ($out/$state/run.log)"; tail -20 "$out/$state/run.log" >&2; return 1; }
}

# csv_field <text>: <text> as one CSV field (quoted, its quotes doubled,
# where it holds a comma or a quote)
csv_field() {
    local v=$1
    if [[ $v == *[,\"]* ]]; then
        v=${v//\"/\"\"}
        printf '"%s"' "$v"
    else
        printf '%s' "$v"
    fi
}

# hostcheck_rows <rep> <label> <node> [<log>]: the media driver's start-up
# host checks on host <node> (a or b) as results rows, one a line it
# printed, <rep>,hostcheck,<label>,<node>,<warning|info>,<what it said>
# (CSV-quoted where it holds a comma: analyse.py reads results.csv as CSV),
# or <rep>,hostcheck,<label>,<node>,none, when it printed none. From the
# driver's own log (vm.sh hostchecks), or, given <log>, a client's output:
# under INVOKER each client embeds the driver, which prints its lines there
# (invoker_named has checked the driver started). A host whose checks could
# not be read (vm.sh hostchecks failed: the session lost) is not one that
# said nothing: <rep>,hostcheck,<label>,<node>,unknown,not read: ..., and a
# warning
hostcheck_rows() {
    local rep=$1 label=$2 node=$3 log=${4:-} host lines line any=0 rc=0
    if [[ -n $log ]]; then
        lines=$(cat "$log" 2>/dev/null)
    else
        [[ $node == a ]] && host=$A || host=$B
        lines=$(on "$host" "$vm hostchecks" 2>/dev/null) || rc=$?
        if ((rc)); then
            log "WARNING: $label rep $rep: $node's media driver host checks not read (vm.sh hostchecks failed: $rc)"
            printf '%s,hostcheck,%s,%s,unknown,%s\n' "$rep" "$label" "$node" "$(csv_field "not read: vm.sh hostchecks failed ($rc)")" >>"$results"
            return 0
        fi
    fi
    while IFS= read -r line; do
        [[ $line =~ ^ergon-md-dpdk\ (warning|info):\ (.*)$ ]] || continue
        printf '%s,hostcheck,%s,%s,%s,%s\n' "$rep" "$label" "$node" "${BASH_REMATCH[1]}" "$(csv_field "${BASH_REMATCH[2]}")" >>"$results"
        any=1
    done <<<"$lines"
    ((any)) || printf '%s,hostcheck,%s,%s,none,\n' "$rep" "$label" "$node" >>"$results"
}

# loss_row <rep> <label> <host-name> <before> <after>: one results row of the
# loss counters' growth over a run (vm.sh loss before and after); a state, not
# a counter (routes-missing: data NICs without their route), as found after it.
# Under INVOKER a host's line is its embedded drivers' ledger (host.sh
# invoker_ledger_total: invoker-ledger=<drivers>, no counter): a counter the
# line before lacks (no driver read yet) grows from 0. A failed Aeron read
# (aeron-read-failed=1) before or after the run is kept, as loss: the
# counters it stands for are missing from that side, so their growth is unknown.
# Aeron's telemetry on the line (info- keys, host.sh aeron_telemetry: the
# agents' longest cycles and cycles over threshold, back-pressure, status
# messages, flow control) is never loss: it goes to an aeron row of its own,
# <rep>,aeron,<label>,<host>,<key>=<value>;..., the longest cycles and the
# thresholds as found after the run (since the driver started), the rest as
# their growth over it (from 0 under the ledger; left out where the line
# before lacks one otherwise), and the loss row is as without it.
loss_row() {
    python3 - "$@" <<'PY' >>"$results"
import sys
rep, label, host, before, after = sys.argv[1:6]
parse = lambda s: dict(kv.split("=", 1) for kv in s.split(";") if "=" in kv)
INFO, AS_FOUND = "info-", ("-max-cycle-ns", "-threshold-ns")
b_all, a_all = parse(before), parse(after)
b = {k: v for k, v in b_all.items() if not k.startswith(INFO)}
a = {k: v for k, v in a_all.items() if not k.startswith(INFO)}
if not a or not b:  # a failed read: never an absolute count passed off as growth
    print(f"{rep},loss,{label},{host},invalid=1")
else:
    STATES, LEDGER, FAILED = ("routes-missing",), "invoker-ledger", "aeron-read-failed"
    ledger = LEDGER in b
    grew = [f"{k}={int(a[k]) if k in STATES else int(a[k]) - int(b.get(k, 0))}" for k in a
            if k != LEDGER and (k in b or ledger or k == FAILED)]
    if FAILED in b and FAILED not in a:
        grew.append(f"{FAILED}=1")
    print(f"{rep},loss,{label},{host}," + ";".join(grew))
    info = [f"{k}={int(v) if k.endswith(AS_FOUND) else int(v) - int(b_all.get(k, 0))}" for k, v in a_all.items()
            if k.startswith(INFO) and (k.endswith(AS_FOUND) or k in b_all or ledger)]
    if info:
        print(f"{rep},aeron,{label},{host}," + ";".join(info))
PY
}

# invoker_named <mode> <client log>...: under INVOKER, every client's log
# has the harness's line for a driver it embedded and ran INVOKER
# ("harness driver: embedded threading=INVOKER ...", on stderr: lab.rs
# Lab::connect, the threading mode read back from the driver's context); a
# client that ran against an external driver ("harness driver: external
# ..."), or none, stops the run
invoker_named() {
    local mode=$1 f
    shift
    [[ $(mode_get "$mode" AERON_THREADING_MODE) == INVOKER ]] || return 0
    for f in "$@"; do
        grep -q '^harness driver: embedded threading=INVOKER ' "$f" 2>/dev/null ||
            die "mode $mode: the client did not name INVOKER as the threading mode of a driver it embedded ($f: $(grep -m1 '^harness driver:' "$f" 2>/dev/null || echo 'no harness driver line'))"
    done
}

# label_mode <label>: the mode a results label names (<state>:<mode>, a
# localhost one's mode after local-)
label_mode() {
    local m=${1#*:}
    printf '%s' "${m#local-}"
}

# fits <label> <size>: whether a message of <size> fits the label's mode's MTU
# (modes.sh mode_payload); if not, said so, and the size is skipped there (a
# client refuses a message that would fragment). lab.sh named them all first.
fits() {
    local m payload
    m=$(label_mode "$1")
    payload=$(mode_payload "$m" "${placement:-}")
    ((${2:-0} <= payload)) && return 0
    log "$1 $2 B: over its largest message ($payload B at MTU $(mode_mtu "$m" "${placement:-}")): skipped"
    return 1
}

# pmtu_check <tag>: with jumbo frames (a mode's JUMBO=1), on both hosts at
# once, the data NICs at the jumbo MTU and the path to each of the other
# host's data addresses checked at it (vm.sh pmtu), logged to
# <node>-pmtu-<tag>.log; 1 if either fails
pmtu_check() {
    local tag=$1 rc=0
    on "$A" "$vm pmtu $b_ips" >"$out/a-pmtu-$tag.log" 2>&1 & local p1=$!
    on "$B" "$vm pmtu $a_ips" >"$out/b-pmtu-$tag.log" 2>&1 & local p2=$!
    wait $p1 || { rc=1; tail -3 "$out/a-pmtu-$tag.log" >&2; }
    wait $p2 || { rc=1; tail -3 "$out/b-pmtu-$tag.log" >&2; }
    return $rc
}

# mtu_rows <rep> <label> <node>...: each host's effective MTU over the run
# (vm.sh mtu: the link's and Aeron's) as results rows
# <rep>,mtu,<label>,<node>,<link MTU>,<Aeron MTU>
mtu_rows() {
    local rep=$1 label=$2 node host line
    shift 2
    for node in "$@"; do
        [[ $node == a ]] && host=$A || host=$B
        line=$(on "$host" "$vm mtu" 2>/dev/null)
        if [[ $line =~ ^[0-9]+\ [0-9]+$ ]]; then
            echo "$rep,mtu,$label,$node,${line/ /,}" >>"$results"
        else
            log "WARNING: $label rep $rep: $node's MTU not read (${line:-nothing})"
        fi
    done
}

# load_sweep <state> <rep> <mode> <label> <far host> <far ip> <near ip> <near cpu> <far cpu>:
# with the drivers up in <mode> (as file names spell it: file_mode), for each
# size in $load_sizes: the one-way
# sustained maximum (tput, VM a publishing to the far host), then the round
# trip open loop (VM a pings, the far host pongs) at each fraction of it in
# $load_fractions and one burst run, each with its loss counters on both
# hosts. For localhost the far host is VM a and both addresses 127.0.0.1.
# With no_flatout (LAB_NO_FLATOUT=1) no sustained maximum: the ladder's rates
# alone (lab.sh refuses a size without one), its burst at their median.
load_sweep() {
    local state=$1 rep=$2 mode=$3 label=$4 far=$5 far_ip=$6 near_ip=$7 near_cpu=$8 far_cpu=$9
    local size max= frac rate before_a before_b pong sub tag port=22200 nodes=a warmed=
    [[ $far == "$B" ]] && nodes="a b"
    for size in ${load_sizes:-}; do
        fits "$label" "$size" || continue
        tag=$state-$mode-$size-$rep
        # $load_ladder ("<size>:<rate>,<rate> ..."): absolute rates for a size,
        # where a fraction of a flat-out maximum means nothing (a maximum the
        # network cannot carry without loss, so it collapses); points r<rate>
        local ladder= points
        for entry in ${load_ladder:-}; do [[ ${entry%%:*} == "$size" ]] && ladder=${entry#*:}; done
        if [[ -n ${no_flatout:-} ]]; then
            [[ -n $ladder ]] || { log "WARNING: $label $size B: LAB_NO_FLATOUT=1 and no ladder for it: skipped"; continue; }
        else
            # the sustained maximum's own loss (a flat-out run is where a ring
            # overflows), and its counters just before and after (frames per message)
            before_a=$(on "$A" "$vm loss" 2>/dev/null)
            [[ $far == "$B" ]] && before_b=$(on "$B" "$vm loss" 2>/dev/null)
            on "$far" "LAB_CLIENT_DEADLINE=$(answer_deadline "$tput_seconds") $vm client $label tput xsub $far_ip:20125 $tput_seconds $far_cpu" \
                2>"$out/maxsub-$tag.log" >"$out/max-$tag.csv" & sub=$!
            sleep 2
            snap "max-before-$tag" "$nodes"
            on "$A" "LAB_CLIENT_DEADLINE=$(client_deadline $((tput_seconds + 6))) $vm client $label tput xpub $far_ip:20125 $((tput_seconds + 6)) $near_cpu $size" \
                >"$out/maxpub-$tag.log" 2>&1
            client_rc "$label $size B: the sustained maximum's publisher" $?
            wait_bounded "$(client_bound "$(answer_deadline "$tput_seconds")")" "$label $size B: the sustained maximum's subscriber" $sub || true
            snap "max-after-$tag" "$nodes"
            loss_row "$rep" "$label:$size:max" a "$before_a" "$(on "$A" "$vm loss" 2>/dev/null)"
            [[ $far == "$B" ]] && loss_row "$rep" "$label:$size:max" b "$before_b" "$(on "$B" "$vm loss" 2>/dev/null)"
            max=$(awk -F, '{printf "%d", $4}' "$out/max-$tag.csv")
            awk -F, -v rep="$rep" -v label="$label" -v size="$size" '{print rep ",tputmax," label "," size "," $4 "," $5 "," $6}' \
                "$out/max-$tag.csv" >>"$results"
            [[ -n $max && $max -gt 0 ]] || { log "WARNING: $label $size B: no sustained maximum"; continue; }
        fi
        if [[ -n $ladder ]]; then
            points="$(tr , ' ' <<<"$ladder" | sed 's/[0-9][0-9]*/r&/g') burst"
        else
            points="${load_fractions:-} burst"
        fi
        # LAB_WARMUP_SECONDS: once a mode and rep, before its first ladder, at
        # this size, on endpoints of its own (warm_up)
        if [[ -z $warmed ]] && ((${warmup_seconds:-0} > 0)); then
            warm_up "$label" "$far" "$far_ip:$port" "$near_ip:$((port + 1))" "$size" "$near_cpu" "$far_cpu" "$tag"
            port=$((port + 2))
            warmed=1
        fi
        for frac in $points; do
            if [[ $frac == r* ]]; then
                rate=${frac#r}
            elif [[ $frac == burst ]]; then
                rate=$((max / 2)) # half the maximum, in bursts of 1000
                [[ -n $ladder ]] && rate=$(tr , '\n' <<<"$ladder" | sort -n | awk '{a[NR]=$1} END {print a[int((NR+1)/2)]}')
            else
                rate=$((max * frac / 100))
            fi
            ((rate > 0)) || continue
            # fresh endpoints per point: no lingering session of the last one
            # (its clients were killed, so the driver holds them until liveness
            # times out) can be the image the new ones attach to
            local ping_ep=$far_ip:$port pong_ep=$near_ip:$((port + 1))
            port=$((port + 2))
            before_a=$(on "$A" "$vm loss" 2>/dev/null)
            [[ $far == "$B" ]] && before_b=$(on "$B" "$vm loss" 2>/dev/null)
            point_counters before "$tag-$frac" "$nodes"
            on "$far" "LAB_CLIENT_DEADLINE=$(answer_deadline "$load_seconds") $vm client $label rtt xpong $ping_ep $pong_ep $far_cpu" \
                >"$out/lpong-$tag-$frac.log" 2>&1 & pong=$!
            sleep 2
            on "$A" "LAB_CLIENT_DEADLINE=$(client_deadline "$load_seconds") $vm client $label load xping $ping_ep $pong_ep $rate $load_seconds $size $near_cpu $([[ $frac == burst ]] && echo 1000)" \
                2>"$out/load-$tag-$frac.log" | sed "s/^/$rep,/;s/,udp,/,udp-$frac,/" | tee -a "$results" >&2
            client_rc "$label $size B $frac: the load client" "${PIPESTATUS[0]}"
            on "$A" 'cat bench/load-spikes.csv' >"$out/spikes-$tag-$frac.csv" 2>/dev/null || true
            # the point's whole histogram, as the load client wrote it (vm.sh
            # client removes the last one before it starts): analyse.py merges
            # each point's over its reps
            on "$A" 'cat bench/rtt-histogram.csv' >"$out/histogram-$tag-$frac.csv" 2>/dev/null || true
            # the pong ends by itself once the ping's image goes; then make sure
            for _ in $(seq 20); do kill -0 $pong 2>/dev/null || break; sleep 0.5; done
            end_client "$far" rtt xpong
            wait_bounded "$(client_bound "$(answer_deadline "$load_seconds")")" "$label $size B $frac: the pong" $pong || true
            point_counters after "$tag-$frac" "$nodes" "$label $size B $frac"
            loss_row "$rep" "$label:$size:$frac" a "$before_a" "$(on "$A" "$vm loss" 2>/dev/null)"
            [[ $far == "$B" ]] && loss_row "$rep" "$label:$size:$frac" b "$before_b" "$(on "$B" "$vm loss" 2>/dev/null)"
        done
    done
}

# warm_up <label> <far host> <ping endpoint> <pong endpoint> <size> <near cpu>
# <far cpu> <tag>: LAB_WARMUP_SECONDS of load at LAB_WARMUP_RATE before a
# mode's first ladder (load_sweep), run as a load point is (the far host
# pongs, VM a pings, each on its app CPU) but recorded nowhere: labelled
# <label>:warmup, its row and log in warmup-<tag>.log alone, no loss row, no
# spikes, no histogram. Aeron's own harness warms 30 s at 25,000 msgs/s; the
# load client's own warm-up is each point's first second, at its rate
warm_up() {
    local label=$1 far=$2 ping_ep=$3 pong_ep=$4 size=$5 near_cpu=$6 far_cpu=$7 tag=$8 pong
    log "$label $size B: warm-up, $warmup_seconds s at $warmup_rate/s (not recorded)"
    on "$far" "LAB_CLIENT_DEADLINE=$(answer_deadline "$warmup_seconds") $vm client $label:warmup rtt xpong $ping_ep $pong_ep $far_cpu" >"$out/warmup-pong-$tag.log" 2>&1 & pong=$!
    sleep 2
    on "$A" "LAB_CLIENT_DEADLINE=$(client_deadline "$warmup_seconds") $vm client $label:warmup load xping $ping_ep $pong_ep $warmup_rate $warmup_seconds $size $near_cpu" \
        >"$out/warmup-$tag.log" 2>&1 || { client_rc "$label $size B: the warm-up's load client" $?; log "WARNING: $label $size B: the warm-up failed ($out/warmup-$tag.log)"; }
    for _ in $(seq 20); do kill -0 $pong 2>/dev/null || break; sleep 0.5; done
    end_client "$far" rtt xpong
    wait_bounded "$(client_bound "$(answer_deadline "$warmup_seconds")")" "$label $size B: the warm-up's pong" $pong || true
}

# flow_count <VM a's app CPUs> <VM b's app CPUs>: how many flows the
# multi-flow sustained maximum (multi_flow_max) runs: $tput_flows
# (LAB_TPUT_FLOWS), off unless asked for (unset, 0 or 1: 0, none), and at most
# one per app CPU on each VM (2 where both have a fifth core)
flow_count() {
    local near=($1) far=($2) most n=${tput_flows:-0}
    ((n >= 2)) || { echo 0; return; }
    most=$((${#near[@]} < ${#far[@]} ? ${#near[@]} : ${#far[@]}))
    if ((n > most)); then
        log "WARNING: LAB_TPUT_FLOWS=$n, but the layouts give $most app CPUs on each VM (a: $1, b: $2): $((most >= 2 ? most : 0)) flows"
        n=$most
    fi
    echo $((n >= 2 ? n : 0))
}

# multi_flow_max <state> <rep> <mode> <label> <far ip> <near cpus> <far cpus>:
# with the drivers up in <mode> (as file names spell it: file_mode), for 32 B
# and each size in $load_sizes, the sustained maximum over several flows at
# once: per CPU in <near cpus> and <far cpus>, a publication on VM a and its
# subscription on VM b, pinned there, on a channel of their own, so on a UDP
# port of their own: a flow of their own (one 5-tuple) to a cloud network.
# The subscribers start first, then every publisher at once, so their measured
# seconds should overlap; nothing makes them (on() retries a dropped SSH
# connection 10 s on, so a flow can start late and have the path to itself for
# a while, raising the sum), so each subscriber reports its measured window on
# the wall clock (tput's last two fields): all on VM b, one clock. With
# the counters on both hosts just before and after (frames per message), and
# the loss counters on both. Result rows: each flow's mean msgs/s over its own
# measured window (received / window) and their sum, which is the concurrent
# aggregate -- a median of one-second samples is a typical second, and a sum of
# those is not what the flows carried at once (a flow without a result, or
# without a window, leaves the sum empty), then the flows' overlap, the span
# every flow's window shares over the shortest window, truncated to the
# thousandth so a run never reads as better overlapped than it was (empty when
# a flow reported no window; analyse.py counts a run from 0.9):
# <rep>,tputflows,<label>,<size>,<flows>,<sum>,<flow 1>,...,<flow n>,<overlap>
multi_flow_max() {
    local state=$1 rep=$2 mode=$3 label=$4 far_ip=$5 near=($6) far=($7)
    local n=${#near[@]} size tag i before_a before_b subs pubs sum flow each port=22700
    local start end latest earliest shortest windows span overlap
    for size in $(printf '%s\n' 32 ${load_sizes:-} | awk '!seen[$0]++'); do
        fits "$label" "$size" || continue
        tag=$state-$mode-$size-$rep
        before_a=$(on "$A" "$vm loss" 2>/dev/null)
        before_b=$(on "$B" "$vm loss" 2>/dev/null)
        # fresh ports per size: no lingering session of the last size's
        subs=() pubs=()
        for i in $(seq "$n"); do
            on "$B" "LAB_CLIENT_DEADLINE=$(answer_deadline "$tput_seconds") $vm client $label tput xsub $far_ip:$((port + i - 1)) $tput_seconds ${far[i - 1]}" \
                2>"$out/flowsub-$tag-$i.log" >"$out/flows-$tag-$i.csv" & subs+=($!)
        done
        sleep 2
        snap "flows-before-$tag"
        for i in $(seq "$n"); do
            on "$A" "LAB_CLIENT_DEADLINE=$(client_deadline $((tput_seconds + 6))) $vm client $label tput xpub $far_ip:$((port + i - 1)) $((tput_seconds + 6)) ${near[i - 1]} $size" \
                >"$out/flowpub-$tag-$i.log" 2>&1 & pubs+=($!)
        done
        port=$((port + n))
        wait_bounded "$(client_bound "$(client_deadline $((tput_seconds + 6)))")" "$label $size B, $n flows: a publisher" "${pubs[@]}"
        wait_bounded "$(client_bound "$(answer_deadline "$tput_seconds")")" "$label $size B, $n flows: a subscriber" "${subs[@]}"
        snap "flows-after-$tag"
        loss_row "$rep" "$label:$size:flows$n" a "$before_a" "$(on "$A" "$vm loss" 2>/dev/null)"
        loss_row "$rep" "$label:$size:flows$n" b "$before_b" "$(on "$B" "$vm loss" 2>/dev/null)"
        sum=0 each= latest= earliest= shortest= windows=1
        for i in $(seq "$n"); do
            # $4 is tput's median of its one-second samples; the rate this row
            # reports is the mean over the flow's own measured window
            # (received / window), which is what a concurrent sum can add.
            median= received= start= end= flow=
            read -r median received start end < <(awk -F, '$1 == "tput" {printf "%s %s %s %s\n", $4, $7, $9, $10; exit}' "$out/flows-$tag-$i.csv")
            if [[ -z $median ]]; then
                log "WARNING: $label $size B, $n flows at once: no result from flow $i ($out/flowsub-$tag-$i.log)"
                sum= windows=
            elif [[ ! $start =~ ^[0-9]+$ || ! $end =~ ^[0-9]+$ ]] || ((end <= start)) ||
                ! [[ $received =~ ^[0-9]+$ ]]; then
                # Without a window there is no mean, and a sum of medians is
                # not the aggregate: the sum is left empty rather than wrong.
                log "WARNING: $label $size B, $n flows at once: no measured window from flow $i (a tput client older than the window): sum and overlap unknown"
                sum= windows=
            else
                # python3: exact integer arithmetic. bash's 64-bit multiply
                # overflows on a long high-rate run, and awk's `r * 1e9` is a
                # double -- exact only to 2^53, so it truncates a rate like
                # 22000000 to 21999999
                flow=$(python3 -c 'import sys; r, s, e = map(int, sys.argv[1:]); print(r * 10**9 // (e - s))' \
                    "$received" "$start" "$end")
                if [[ -z $latest ]] || ((start > latest)); then latest=$start; fi
                if [[ -z $earliest ]] || ((end < earliest)); then earliest=$end; fi
                if [[ -z $shortest ]] || ((end - start < shortest)); then shortest=$((end - start)); fi
            fi
            [[ -n $sum && -n $flow ]] && sum=$((sum + flow))
            each+=,${flow:-$median}
        done
        overlap=
        if [[ -n $windows ]]; then
            # in thousandths, truncated
            span=$((earliest > latest ? earliest - latest : 0))
            span=$((span * 1000 / shortest))
            overlap=$((span / 1000)).$(printf '%03d' $((span % 1000)))
            ((span >= 900)) ||
                log "WARNING: $label $size B, $n flows at once: their measured windows overlapped $overlap of the shortest: analyse.py leaves the run out"
        fi
        echo "$rep,tputflows,$label,$size,$n,$sum$each,$overlap" >>"$results"
    done
}

# mix_run <state> <rep> <mode> <label> <far host> <far ip> <near ip> <near cpu> <far cpu>:
# for each profile in $mix ("<profile>=<name:stream:msgs/s:size[:burst]> ...;..."),
# every stream of the profile at once, open loop on its own schedule, on one
# pair of channels (VM a pings, the far host echoes each stream on stream + 1000),
# with loss counters on both hosts and the spikes (LOAD_SPIKE_NS) kept per run
# (<mode> as file names spell it: file_mode).
# Result rows: <rep>,mix,<label>,udp-<profile>/<stream name>,...
mix_run() {
    local state=$1 rep=$2 mode=$3 label=$4 far=$5 far_ip=$6 near_ip=$7 near_cpu=$8 far_cpu=$9
    local profile pname specs streams ping_ep pong_ep before_a before_b echo tag port=22800 nodes=a
    [[ $far == "$B" ]] && nodes="a b"
    local IFS_SAVE=$IFS
    IFS=';'
    local profiles=(${mix:-})
    IFS=$IFS_SAVE
    for profile in "${profiles[@]}"; do
        [[ $profile == *=* ]] || { log "WARNING: mix profile '$profile' is not <name>=<specs>"; continue; }
        pname=${profile%%=*} specs=${profile#*=}
        fits "$label" "$(tr ' ' '\n' <<<"$specs" | awk -F: 'NF >= 4 && $4 > m {m = $4} END {print m + 0}')" || continue
        streams=$(tr ' ' '\n' <<<"$specs" | awk -F: 'NF >= 4 {print $2}' | tr '\n' ' ')
        tag=$state-$mode-mix-$pname-$rep
        ping_ep=$far_ip:$port pong_ep=$near_ip:$((port + 1))
        port=$((port + 2))
        before_a=$(on "$A" "$vm loss" 2>/dev/null)
        [[ $far == "$B" ]] && before_b=$(on "$B" "$vm loss" 2>/dev/null)
        point_counters before "$tag" "$nodes"
        on "$far" "LAB_CLIENT_DEADLINE=$(answer_deadline "$mix_seconds") $vm client $label load xecho $ping_ep $pong_ep $far_cpu $streams" \
            >"$out/mecho-$tag.log" 2>&1 & echo=$!
        sleep 2
        on "$A" "LAB_CLIENT_DEADLINE=$(client_deadline "$mix_seconds") $vm client $label load xmix $ping_ep $pong_ep $mix_seconds $near_cpu $specs" \
            2>"$out/mix-$tag.log" | sed "s/^/$rep,/;s/,udp-/,udp-$pname\//" | tee -a "$results" >&2
        client_rc "$label mix $pname: the load client" "${PIPESTATUS[0]}"
        on "$A" 'cat bench/load-spikes.csv' >"$out/spikes-$tag.csv" 2>/dev/null || true
        for _ in $(seq 20); do kill -0 $echo 2>/dev/null || break; sleep 0.5; done
        end_client "$far" load xecho
        wait_bounded "$(client_bound "$(answer_deadline "$mix_seconds")")" "$label mix $pname: the echo" $echo || true
        point_counters after "$tag" "$nodes" "$label mix $pname"
        loss_row "$rep" "$label:mix:$pname" a "$before_a" "$(on "$A" "$vm loss" 2>/dev/null)"
        [[ $far == "$B" ]] && loss_row "$rep" "$label:mix:$pname" b "$before_b" "$(on "$B" "$vm loss" 2>/dev/null)"
    done
}

# bench_state <state>, on VM a (remote.sh): $reps reps of every mode in a fresh
# random order each rep, against VM a itself and VM b
bench_state() {
    local state=$1 rep mode fm label pong sub
    results=$out/results.csv
    local a_nics b_nics net us host
    a_nics=$(on "$A" "$vm nics" 2>>"$out/nics.log")
    b_nics=$(on "$B" "$vm nics" 2>>"$out/nics.log")
    a_rcv=$(awk '$1 == "receiver" {print $5}' <<<"$a_nics")
    b_rcv=$(awk '$1 == "receiver" {print $5}' <<<"$b_nics")
    [[ -n $a_rcv && -n $b_rcv ]] || { cat "$out/nics.log" >&2; die "cannot read the data NICs"; }
    # every data address of each host (the path MTU check's peers)
    a_ips=$(awk '{printf "%s%s", sep, $5; sep = " "}' <<<"$a_nics")
    b_ips=$(awk '{printf "%s%s", sep, $5; sep = " "}' <<<"$b_nics")
    # The network's own round trip (vm.sh base-rtt: ping from a's sender to
    # b's receiver): a row for analyse.py, which shows each mode's round trip
    # above it, and its max in microseconds on both hosts (vm.sh client
    # scales the lost-round-trip timeout of an unreliable channel by it)
    net=$(on "$A" "$vm base-rtt $b_rcv" 2>>"$out/nics.log")
    if [[ $net =~ ^[0-9]+\ [0-9]+\ [0-9]+\ [0-9]+\ [0-9]+/[0-9]+$ ]]; then
        echo "0,netrtt,$state,${net// /,}" >>"$results"
        read -r _ _ us _ <<<"$net"
        us=$(((us + 999) / 1000))
        for host in "$A" "$B"; do on "$host" "echo $us >bench/base_rtt_us" || true; done
        log "network round trip, a's sender to b's receiver (ping, ns): min avg max mdev replies $net"
    else
        log "WARNING: the network's own round trip not measured (${net:-nothing})"
    fi
    app_a=$(on "$A" "$vm layout" | awk '{print $2}')
    app2_a=$(on "$A" "$vm layout" | awk '{print $5}')
    app_b=$(on "$B" "$vm layout" | awk '{print $2}')
    app2_b=$(on "$B" "$vm layout" | awk '{print $5}')
    log "receiver ports: a=$a_rcv b=$b_rcv; app CPUs: a=$app_a b=$app_b"
    # the multi-flow sustained maximum's flows (LAB_TPUT_FLOWS; none unless
    # asked for), each end of each on an app CPU of its own
    local cpus_a=($app_a $app2_a) cpus_b=($app_b $app2_b) flows
    flows=$(flow_count "${cpus_a[*]}" "${cpus_b[*]}")
    for rep in $(seq "$reps"); do
        for mode in $(printf '%s\n' "${modes[@]}" | python3 -c 'import random,sys; l=sys.stdin.read().split(); random.shuffle(l); print(" ".join(l))'); do
            label=$state:$mode
            fm=$(file_mode "$mode")
            log "$state rep $rep: $mode"
            # jumbo frames: the path checked at the jumbo MTU first, both ways
            if [[ $(mode_get "${mode%+idle}" JUMBO) == 1 ]]; then
                pmtu_check "$state-$fm-$rep" ||
                    die "mode $mode: the path MTU check failed: the path between the hosts drops jumbo frames ($out/{a,b}-pmtu-$state-$fm-$rep.log)"
            fi
            # chrony's view of both clocks before the mode, kept with the run
            # (spikes.py: the hosts' offsets when it matches across them); a
            # check too (vm.sh timesync check: under precision-time the PHC
            # selected within its bound), which stops the run before the drivers
            both "timesync-before-$state-$fm-$rep" timesync check ||
                die "mode $mode: a host's clock failed its check before the drivers ($out/{a,b}-timesync-before-$state-$fm-$rep.log)"
            # <mode>+idle: the same, with an idle localhost subscription held on
            # each driver throughout (what an unused host-local channel costs)
            both "driver-$state-$fm-$rep" driver "${mode%+idle}" || die "$mode drivers did not start"
            mtu_rows "$rep" "$label" a b
            # what each host's driver said of the host as it started (under
            # INVOKER the clients' embedded drivers say it: after the round trip)
            if [[ $(mode_get "${mode%+idle}" AERON_THREADING_MODE) != INVOKER ]]; then
                hostcheck_rows "$rep" "$label" a
                hostcheck_rows "$rep" "$label" b
            fi
            idle_pids=()
            if [[ $mode == *+idle ]]; then
                # held until killed below; one log per VM, both must be up first
                for n in a b; do
                    [[ $n == a ]] && host=$A || host=$B
                    on "$host" "LAB_CLIENT_DEADLINE=3660 $vm client $label rtt idle 127.0.0.1:22100 3600" \
                        >"$out/idle-$state-$fm-$rep-$n.log" 2>&1 & idle_pids+=($!)
                done
                for _ in $(seq 40); do
                    [[ $(cat "$out/idle-$state-$fm-$rep-"{a,b}.log 2>/dev/null | grep -c 'idle subscription') -ge 2 ]] && break
                    sleep 0.5
                done
                [[ $(cat "$out/idle-$state-$fm-$rep-"{a,b}.log 2>/dev/null | grep -c 'idle subscription') -ge 2 ]] ||
                    die "$mode rep $rep: the idle subscriptions did not start"
            fi
            both "counters-before-$state-$fm-$rep" counters || true
            # DPDK on AWS: whether the ENA's low-latency queue BAR is write-combining
            if [[ ${mode%%@*} == dpdk* ]]; then both "wc-$state-$fm-$rep" wc-check || true; fi
            both "jitter-before-rtt-$state-$fm-$rep" jitter || true
            loss_a=$(on "$A" "$vm loss" 2>/dev/null); loss_b=$(on "$B" "$vm loss" 2>/dev/null)
            # round trip: B pongs, A pings (ping's endpoint on B, pong's on A),
            # the ping started at second $rtt_align of the minute when set
            align_rtt
            # every round trip of rtt_spike_ns or more stamped, every loop stall
            # of rtt_stall_ns or more (LAB_RTT_SPIKE_NS, LAB_RTT_STALL_NS)
            on "$B" "LOOP_STALL_NS=${rtt_stall_ns:-10000} LAB_CLIENT_DEADLINE=$(answer_deadline 25) $vm client $label rtt xpong $b_rcv:20123 $a_rcv:20124 $app_b" \
                >"$out/pong-$state-$fm-$rep.log" 2>&1 & pong=$!
            sleep 2
            # (RTT_SECONDS=20: 20 s measured after up to 5 s of warm-up)
            on "$A" "RTT_SECONDS=20 LOAD_SPIKE_NS=${rtt_spike_ns:-30000} LOOP_STALL_NS=${rtt_stall_ns:-10000} LAB_CLIENT_DEADLINE=$(client_deadline 25) $vm client $label rtt xping $b_rcv:20123 $a_rcv:20124 $rtt_messages $rtt_warmup $app_a" \
                2>"$out/ping-$state-$fm-$rep.log" | sed "s/^/$rep,/" | tee -a "$results" >&2
            client_rc "$label rep $rep: the round trip's ping" "${PIPESTATUS[0]}"
            end_client "$B" rtt xpong
            wait_bounded "$(client_bound "$(answer_deadline 25)")" "$label rep $rep: the round trip's pong" $pong || true
            invoker_named "$mode" "$out/ping-$state-$fm-$rep.log" "$out/pong-$state-$fm-$rep.log"
            if [[ $(mode_get "${mode%+idle}" AERON_THREADING_MODE) == INVOKER ]]; then
                hostcheck_rows "$rep" "$label" a "$out/ping-$state-$fm-$rep.log"
                hostcheck_rows "$rep" "$label" b "$out/pong-$state-$fm-$rep.log"
            fi
            on "$A" 'cat bench/rtt-histogram.csv' >"$out/histogram-$state-$fm-$rep.csv" 2>/dev/null || true
            on "$A" 'cat bench/load-spikes.csv' >"$out/spikes-$state-$fm-rtt-$rep.csv" 2>/dev/null || true
            on "$A" 'cat bench/stalls-ping.csv' >"$out/stalls-ping-$state-$fm-$rep.csv" 2>/dev/null || true
            on "$B" 'cat bench/stalls-pong.csv' >"$out/stalls-pong-$state-$fm-$rep.csv" 2>/dev/null || true
            # each host's DPDK event log, where the mode turned it on
            # (@RUSTERON_DPDK_EVENT_LOG=<home>/bench/events.csv: the agents' loop
            # gaps, ARP and housekeeping's calls, each timed; vm.sh driver
            # removes the last one): its events so far, the round trip's among
            # them, for spikes.py; none kept where a host has none
            for n in a b; do
                [[ $n == a ]] && host=$A || host=$B
                on "$host" 'cat bench/events.csv' >"$out/$n-rings-$state-$fm-$rep.log" 2>/dev/null || true
                [[ -s $out/$n-rings-$state-$fm-$rep.log ]] || rm -f "$out/$n-rings-$state-$fm-$rep.log"
            done
            both "jitter-after-rtt-$state-$fm-$rep" jitter || true
            # NIC arrival -> the receive burst on each receiver port (DPDK with NIC RX timestamps only)
            for host in "$A" "$B"; do
                # (awk, not sed: a label may hold a /, BIN's directory)
                on "$host" "$vm rx-hw-delay" 2>/dev/null | awk -v p="$rep,rxhw,$label," '{print p $0}' >>"$results" || true
            done
            # the round trip's own loss (the row after the throughput run covers both)
            mid_a=$(on "$A" "$vm loss" 2>/dev/null); mid_b=$(on "$B" "$vm loss" 2>/dev/null)
            loss_row "$rep" "$label:rtt" a "$loss_a" "$mid_a"
            loss_row "$rep" "$label:rtt" b "$loss_b" "$mid_b"
            # throughput: A publishes, B subscribes, with the counters on both
            # hosts just before and after (its wire frames per message); flat
            # out, so not with no_flatout (LAB_NO_FLATOUT=1)
            if [[ -z ${no_flatout:-} ]]; then
                # one job, a subshell, so wait_bounded ends all of it and has
                # on()'s status (a pipeline's $! is its last process: sed)
                (on "$B" "LAB_CLIENT_DEADLINE=$(answer_deadline "$tput_seconds") $vm client $label tput xsub $b_rcv:20125 $tput_seconds $app_b" \
                    2>"$out/sub-$state-$fm-$rep.log" | sed "s/^/$rep,/" >>"$results"; exit "${PIPESTATUS[0]}") & sub=$!
                sleep 2
                snap "tput-before-$state-$fm-$rep"
                on "$A" "LAB_CLIENT_DEADLINE=$(client_deadline $((tput_seconds + 6))) $vm client $label tput xpub $b_rcv:20125 $((tput_seconds + 6)) $app_a" \
                    >"$out/pub-$state-$fm-$rep.log" 2>&1
                client_rc "$label rep $rep: the throughput publisher" $?
                wait_bounded "$(client_bound "$(answer_deadline "$tput_seconds")")" "$label rep $rep: the throughput subscriber" $sub ||
                    log "WARNING: $mode rep $rep throughput subscriber failed"
                snap "tput-after-$state-$fm-$rep"
                tail -1 "$results" >&2
                invoker_named "$mode" "$out/sub-$state-$fm-$rep.log" "$out/pub-$state-$fm-$rep.log"
            fi
            both "counters-after-$state-$fm-$rep" counters || true
            loss_row "$rep" "$label" a "$loss_a" "$(on "$A" "$vm loss" 2>/dev/null)"
            loss_row "$rep" "$label" b "$loss_b" "$(on "$B" "$vm loss" 2>/dev/null)"
            # latency under load (LAB_LOAD_SIZES), the same drivers
            [[ -n ${load_sizes:-} && $mode != *+idle ]] &&
                load_sweep "$state" "$rep" "$fm" "$label" "$B" "$b_rcv" "$a_rcv" "$app_a" "$app_b"
            [[ -n ${mix:-} && $mode != *+idle ]] &&
                mix_run "$state" "$rep" "$fm" "$label" "$B" "$b_rcv" "$a_rcv" "$app_a" "$app_b"
            # several flows at once (LAB_TPUT_FLOWS), after everything above so
            # that runs as it did without it; the same drivers. Not under
            # INVOKER: each flow's clients would embed a driver of their own
            if ((flows)) && [[ $(mode_get "$mode" AERON_THREADING_MODE) == INVOKER ]]; then
                log "$label: several flows at once not run under INVOKER (each flow's clients would embed a driver of their own on one host)"
            fi
            # nor where the mode takes the fifth core, the second flow's app
            # CPU (@IRQ=spare, @NAPI_BUSY=spare, @CONDUCTOR=own)
            if ((flows)) && mode_spare "$mode"; then
                log "$label: several flows at once not run: the mode takes the fifth core ($(spare_takers "$mode")), the second flow's app CPU"
            fi
            ((flows)) && [[ $mode != *+idle && -z ${no_flatout:-} && $(mode_get "$mode" AERON_THREADING_MODE) != INVOKER ]] &&
                ! mode_spare "$mode" &&
                multi_flow_max "$state" "$rep" "$fm" "$label" "$b_rcv" "${cpus_a[*]:0:flows}" "${cpus_b[*]:0:flows}"
            if ((${#idle_pids[@]})); then
                kill "${idle_pids[@]}" 2>/dev/null
                for host in "$A" "$B"; do end_client "$host" rtt idle; done
                wait_bounded "$(client_bound 3660)" "$label: an idle subscription" "${idle_pids[@]}" || true
            fi
            both "errors-$state-$fm-$rep" errors || true
            if [[ ${mode%%@*} == dpdk* ]]; then both "pmd-$state-$fm-$rep" pmd-log || true; fi
            both "stop-$state-$fm-$rep" stop || die "drivers did not stop"
            both "timesync-after-$state-$fm-$rep" timesync || true
        done
        # localhost: both ends on VM a over 127.0.0.1, through one driver
        [[ -n $app2_a ]] || { [[ -n ${local_modes:-} ]] && log "WARNING: no fifth core on VM a, localhost runs skipped"; continue; }
        for mode in $(printf '%s\n' ${local_modes:-} | python3 -c 'import random,sys; l=sys.stdin.read().split(); random.shuffle(l); print(" ".join(l))'); do
            label=$state:local-$mode
            fm=$(file_mode "$mode")
            log "$state rep $rep: localhost $mode"
            on "$A" "$vm timesync check" >"$out/a-timesync-before-$state-local-$fm-$rep.log" 2>&1 ||
                die "mode local-$mode: the clock failed its check before the driver ($out/a-timesync-before-$state-local-$fm-$rep.log)"
            on "$A" "$vm driver $mode" >"$out/driver-$state-local-$fm-$rep.log" 2>&1 || die "$mode driver did not start"
            mtu_rows "$rep" "$label" a
            hostcheck_rows "$rep" "$label" a
            if [[ ${mode%%@*} == dpdk* ]]; then on "$A" "$vm wc-check" >"$out/a-wc-$state-local-$fm-$rep.log" 2>&1 || true; fi
            on "$A" "$vm jitter" >"$out/a-jitter-before-rtt-$state-local-$fm-$rep.log" 2>&1 || true
            align_rtt
            on "$A" "LAB_CLIENT_DEADLINE=$(answer_deadline 25) $vm client $label rtt xpong 127.0.0.1:20123 127.0.0.1:20124 $app2_a" \
                >"$out/pong-$state-local-$fm-$rep.log" 2>&1 & pong=$!
            sleep 2
            # a localhost round trip is ~6 us: log its spikes from 25 us, not 100
            on "$A" "RTT_SECONDS=20 LOAD_SPIKE_NS=25000 LAB_CLIENT_DEADLINE=$(client_deadline 25) $vm client $label rtt xping 127.0.0.1:20123 127.0.0.1:20124 $rtt_messages $rtt_warmup $app_a" \
                2>"$out/ping-$state-local-$fm-$rep.log" | sed "s/^/$rep,/" | tee -a "$results" >&2
            client_rc "$label rep $rep: the round trip's ping" "${PIPESTATUS[0]}"
            end_client "$A" rtt xpong
            wait_bounded "$(client_bound "$(answer_deadline 25)")" "$label rep $rep: the round trip's pong" $pong || true
            on "$A" 'cat bench/rtt-histogram.csv' >"$out/histogram-$state-local-$fm-$rep.csv" 2>/dev/null || true
            on "$A" 'cat bench/load-spikes.csv' >"$out/spikes-$state-local-$fm-rtt-$rep.csv" 2>/dev/null || true
            on "$A" 'cat bench/stalls-ping.csv' >"$out/stalls-ping-$state-local-$fm-$rep.csv" 2>/dev/null || true
            on "$A" 'cat bench/stalls-pong.csv' >"$out/stalls-pong-$state-local-$fm-$rep.csv" 2>/dev/null || true
            on "$A" "$vm jitter" >"$out/a-jitter-after-rtt-$state-local-$fm-$rep.log" 2>&1 || true
            if [[ -z ${no_flatout:-} ]]; then
                # one job, as the cross-host subscriber's
                (on "$A" "LAB_CLIENT_DEADLINE=$(answer_deadline "$tput_seconds") $vm client $label tput xsub 127.0.0.1:20125 $tput_seconds $app2_a" \
                    2>"$out/sub-$state-local-$fm-$rep.log" | sed "s/^/$rep,/" >>"$results"; exit "${PIPESTATUS[0]}") & sub=$!
                sleep 2
                on "$A" "LAB_CLIENT_DEADLINE=$(client_deadline $((tput_seconds + 6))) $vm client $label tput xpub 127.0.0.1:20125 $((tput_seconds + 6)) $app_a" \
                    >"$out/pub-$state-local-$fm-$rep.log" 2>&1
                client_rc "$label rep $rep: the throughput publisher" $?
                wait_bounded "$(client_bound "$(answer_deadline "$tput_seconds")")" "$label rep $rep: the throughput subscriber" $sub ||
                    log "WARNING: localhost $mode rep $rep throughput subscriber failed"
            fi
            [[ -n ${load_sizes:-} ]] &&
                load_sweep "$state" "$rep" "local-$fm" "$label" "$A" 127.0.0.1 127.0.0.1 "$app_a" "$app2_a"
            [[ -n ${mix:-} ]] &&
                mix_run "$state" "$rep" "local-$fm" "$label" "$A" 127.0.0.1 127.0.0.1 "$app_a" "$app2_a"
            if [[ ${mode%%@*} == dpdk* ]]; then on "$A" "$vm pmd-log" >"$out/a-pmd-$state-local-$fm-$rep.log" 2>&1 || true; fi
            on "$A" "$vm stop" >/dev/null 2>&1 || die "driver did not stop"
            on "$A" "$vm timesync" >"$out/a-timesync-after-$state-local-$fm-$rep.log" 2>&1 || true
        done
    done
}
