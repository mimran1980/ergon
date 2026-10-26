#!/usr/bin/env bash
# The transport latency suite's runner and cloud side, with no cloud and no
# VM: fakes for az, ssh, scp, sleep, curl, vm.sh, the bench binaries, and
# /sys and /proc trees. Checks that:
#   * modes.sh takes an arm's STREAMS, NETEM and AA, refuses anything else,
#     and refuses NETEM on every DPDK arm (by name or by its dpdk set-up);
#   * transport_plan.py counts the plan's points (510 for pair a, 420 for
#     b), draws each round's arm order from its seed (reproducible, the A/A
#     arm once a round, another order on another rep), and refuses a broken
#     matrix: aa first, an unknown or repeated arm, one- and two-host arms in
#     one set, two sets at once on one host; its binomial interval;
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
#   * transport.sh run against fake hosts: every point of pair b once (rows,
#     manifests, rounds and orders recorded), netem set and cleared around
#     each lossy round and never under a DPDK arm, the binaries' flags as
#     their --help has them (transport-cli.md: addresses only for Aeron's
#     UDP, --busy-read-us only for mio, one CPU a busy thread), each row as
#     the binary writes it with the rep before it and its point column
#     <channel>-r<rate>, the session laid out for the analysis
#     (transport_collect.py), the ping started only once its pong says ready
#     (a pong that never does fails its point), a placement or netem verdict
#     that fails in the manifest, a failed point recorded, a hostcheck
#     warning stopping the session before its first point; LAB_ARMS_FILE and
#     LAB_EXTRA_SRC's hooks with a neutral fixture (transport_prepare's steps
#     too).
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
)
same "modes.sh: an arm takes STREAMS, NETEM and AA alone; NETEM refused on every DPDK arm" "ok: 0 0 0 0 0
refused: 2 2 2 2 2 2 2 2 2 2 2
by setup: 2 0
1" "$got"

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
# fake ethtool: rings settable up to 4096; coalescing unless FAKE_NO_COALESCE
cat >"$tmp/fakebin/ethtool" <<'SH'
#!/usr/bin/env bash
echo "ethtool $*" >>"$FAKE_LOG"
st=$FAKE_STATE
case $1 in
-G) [[ $4 -le 4096 && $6 -le 4096 ]] && echo "$4 $6" >"$st/ring-$2"; exit 0 ;;
-g) read -r r t <"$st/ring-$2" 2>/dev/null || { r=512 t=512; }
    printf 'Ring parameters for %s:\nPre-set maximums:\nRX:\t\t4096\nTX:\t\t4096\nCurrent hardware settings:\nRX:\t\t%s\nTX:\t\t%s\n' "$2" "$r" "$t" ;;
-c) [[ -n ${FAKE_NO_COALESCE:-} ]] && { echo "netlink error: Operation not supported" >&2; exit 76; }
    if [[ -e $st/coal-$2 ]]; then printf 'Adaptive RX: off  TX: off\nrx-usecs: 0\ntx-usecs: 0\n'; else printf 'Adaptive RX: on  TX: on\nrx-usecs: 8\ntx-usecs: 8\n'; fi ;;
-C) [[ -z ${FAKE_COALESCE_STUCK:-} ]] && touch "$st/coal-$2"; exit 0 ;;
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
got+="
smt stuck: $(FAKE_SMT_STUCK=1 hostrun transport_smt_off >/dev/null 2>&1; echo "rc $?")
smt: $(hostrun transport_smt_off 2>&1; echo " rc $?")
map smt off: $(hostrun transport_cpu_map)"
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
smt stuck: rc 1
smt: smt: control off active 0 online 0,2,4,6,8,10,12,14
 rc 0
map smt off: 0 2 4,6,8,10,12,14
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
coalesce none: $(FAKE_NO_COALESCE=1 hostrun transport_coalesce_off enP3 2>&1; echo " rc $?")"
same "host.sh: rings set and read back (above the maximum refused), coalescing off and read back, none where the driver has none" "ring: ring enP1 rx 1024 tx 1024
 rc 0
ring over: rc 1
coalesce: coalesce enP1 off (off off 0 0)
 rc 0
coalesce stuck: rc 1
coalesce none: coalesce enP3: none (the driver has no interrupt moderation)
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
    "LAB_PAIR=a LAB_ARMS_FILE=$fx/none.tsv#not a readable file" "LAB_PAIR=a LAB_ARMS_FILE=$fx/dpdk-netem.tsv#NETEM on a DPDK arm" \
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
transport-map) echo "0 2 4,6,8,10,12,14" ;;
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
sample) if [[ -n ${FAKE_PLACEMENT_BAD:-} ]]; then echo "FAIL ping(9) on CPU 0, not isolated;"; else echo "ok 2 busy threads"; fi
    echo "  101   4 99.5 ping"; echo "    1   0  0.1 sshd" ;;
# every transport sysctl at its declared value, the optional ones absent
settings) (. "$FAKE_CLOUD/transport-settings.env"; . "$FAKE_CLOUD/host.sh"; transport_sysctls) |
    awk '{k = $1; gsub("/", ".", k); v = $2; for (i = 3; i < NF; i++) v = v " " $i; print k "=" ($NF == "optional" ? "absent" : v)}' ;;
nstat) n=$(cat "$st/s" 2>/dev/null || echo 0); n=$((n + 1)); echo $n >"$st/s"
    printf 'Tcp: RetransSegs InSegs\nTcp: %d %d\nUdp: InErrors\nUdp: 0\n' "$n" "$((n * 100))" ;;
# the data NICs' interrupt deferral (napi_defer_hard_irqs, gro_flush_timeout),
# both NICs alike: kernel-busy's driver sets 2/200000, kernel's 0/0;
# transport-net the settings' TR_NAPI_DEFER/TR_GRO_FLUSH_NS under the arm's
# TR_OVERRIDES (logged); napi reads them back as vm.sh napi prints them
driver) case $2 in *@NAPI_DEFER=*) v=${2#*@NAPI_DEFER=}; v=${v%%@*}; echo "${v%/*} ${v#*/}" >"$st/napi" ;; kernel-busy*) echo "2 200000" >"$st/napi" ;; kernel | kernel@*) echo "0 0" >"$st/napi" ;; esac ;;
transport-net)
    if [[ ${2:-} != check ]]; then
        v=$(. "$FAKE_CLOUD/transport-settings.env"; . "$FAKE_CLOUD/host.sh"; transport_overrides; echo "$TR_NAPI_DEFER $TR_GRO_FLUSH_NS")
        echo "$v" >"$st/napi"
        echo "$FAKE_HOST transport-net napi $v" >>"$FAKE_HOSTS/calls"
    fi ;;
napi) read -r n ns <"$st/napi" 2>/dev/null || { n=2 ns=200000; }; printf 'enP1 %s %s\nenP2 %s %s\n' "$n" "$ns" "$n" "$ns" ;;
stop | events-stop-all) ;;
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
echo "$FAKE_HOST $me $*" >>"$FAKE_HOSTS/runs"
role= out= label= rate= kind=oneway streams=1 warm=0 dur=0 mode= impl=
while (($#)); do
    case $1 in --role) role=$2 ;; --out) out=$2 ;; --label) label=$2 ;; --rate) rate=$2 ;; --round-trip) kind=load ;;
    --streams) streams=$2 ;; --warmup-s) warm=$2 ;; --duration-s) dur=$2 ;; --mode) mode=$2 ;; --impl) impl=$2 ;; esac
    shift
done
# the manifest's settings as the binaries write them: the transport, the
# schedule, the counters each host's loss accounting reads
case $me in
aeron_lat) transport=aeron-$mode extra=', "aeron.naks-sent": "0", "aeron.retransmits-sent": "0"' ;;
tcp_lat) transport=tcp extra=', "tcp.retrans": "0", "tcp.dsack_dups": "0", "tcp.retrans_not_dsack": "0", "tcp.lost_end": "0"' ;;
mpsc_lat) transport=mpsc extra= ;;
*) transport=udp extra= ;;
esac
[[ $impl == mio ]] && extra+=', "busy_read_us": "50"'
settings="\"transport\": \"$transport\", \"streams\": \"$streams\", \"warmup_s\": \"$warm\", \"duration_s\": \"$dur\", \"scheduled\": \"0\", \"late\": \"0\", \"max_send_lag_ns\": \"0\", \"retrans\": \"0\"$extra"
[[ $role == oneway-send || $role == oneway-recv ]] && { echo "stub: --role $role is refused (one way is --role local)" >&2; exit 2; }
[[ $role == ping ]] && kind=load
if [[ $role == pong ]]; then
    # ready once "bound", unless told to stay silent; a manifest when stopped
    [[ -n ${FAKE_PONG_SILENT:-} && $label == *"$FAKE_PONG_SILENT"* ]] || echo ready
    trap 'mkdir -p "$out"; echo "{\"settings\": {$settings}}" >"$out/manifest.json"; exit 0' TERM
    while :; do sleep 0.05; done
fi
[[ -n ${FAKE_FAIL_LABEL:-} && $label == *"$FAKE_FAIL_LABEL"* ]] && { echo "stub: failing $label" >&2; exit 1; }
mkdir -p "$out"
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
    for b in aeron_lat mpsc_lat udp_lat tcp_lat; do ln -sf "$sb/bin/stub" "$h/ergon/target/release/$b"; done
}
trun() { # <pair> <reps> [<arms file>] [vars...]: transport.sh run on host a
    local pair=$1 reps=$2 af=${3:-}
    shift 3 || shift $#
    rm -rf "$sb/a" "$sb/b" "$sb/calls" "$sb/runs" "$sb/taskset"
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
"$tmp/bystander/aeron_lat" 600 &
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
grep -q '^a tcp_lat --role ping .*--busy-read-us 50 .*--impl mio$' "$sb/runs" && ! grep -E '^. tcp_lat .*--impl ergon$' "$sb/runs" | grep -q -- '--busy-read-us' || why+=" --busy-read-us for mio alone;"
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
# each arm's per-arm knobs declared as it ran them (mio's busy-read its
# own; no sweep line: no tuning session ran), every point's manifest
# recording them
grep -q '^mio-tcp.knob.busy_read_us=50$' "$d/transport-settings.env" && grep -q '^eio-tcp.knob.epoll_events=64$' "$d/transport-settings.env" &&
    grep -q '^mpsc.knob.capacity=4096$' "$d/transport-settings.env" && grep -q '^aeron-rel.knob.nak_delay_ns=1000$' "$d/transport-settings.env" &&
    ! grep -q '^eio-tcp.knob.busy_read_us' "$d/transport-settings.env" && ! grep -q '^sweep\.' "$d/transport-settings.env" &&
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
trun b 1 "" FAKE_PLACEMENT_BAD=1 FAKE_NETEM_BAD=1 FAKE_AERON_PAGE_BAD=1 FAKE_FAIL_LABEL='3c:mio-tcp@STREAMS=1@NETEM=0' FAKE_PONG_SILENT='3c:eio-tcp@STREAMS=1@NETEM=0' FAKE_SH_DELAY=0.3 && rrc=0 || rrc=$?
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
# every pong slow to write its PID file: waited for, not taken for an exit
[[ $(grep -c . "$d/failures.csv") == 2 ]] && ! grep -q 'exited before it said ready' "$sb/run.out" || why+=" a slow pong taken for gone: $(tr '\n' ' ' <"$d/failures.csv");"
[[ -z $why ]] && pass "transport.sh run: a failed placement or netem verdict in the manifest and logged, a failed point and a pong that never said ready in failures.csv (a pong slow to start waited for), the session's exit 1" ||
    fail "transport.sh run failures:$why"

# a hostcheck warning stops the session before its first point
trun b 1 "" FAKE_HOSTCHECK_WARN=b && rrc=0 || rrc=$?
if [[ $rrc != 0 && $(cat "$d/rc" 2>/dev/null) == 1 && ! -s $sb/runs ]] && grep -q 'host b: the hostcheck report is not clean.*T3 warn' "$sb/run.out"; then
    pass "transport.sh run: a hostcheck warning on either host stops the session before its first point"
else
    fail "transport.sh run with a hostcheck warning: exit $rrc, $(grep -c . "$sb/runs" 2>/dev/null) runs, $(tail -1 "$sb/run.out")"
fi
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

for script in transport.sh azure/lab.sh azure/az.sh host.sh irqs.sh vm.sh modes.sh; do
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
