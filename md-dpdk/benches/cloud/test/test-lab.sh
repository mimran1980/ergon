#!/usr/bin/env bash
# Fixture test for the lab's analysis and a syntax check of its scripts, with
# no cloud and no VM. fixture/ is a run directory as lab.sh leaves one: a
# results.csv of three reps of DPDK against a base build (dpdk@BIN=bin-base)
# and one rep of kernel busy polling and of a localhost sustained maximum,
# with two-flow sustained maxima (one failed, one rep without its one-flow
# run; one overlapping 0.899 of the shortest window, one exactly 0.900, one a
# row from before the overlap was recorded), and a state directory (isolated/) with
# the jitter snapshots around two reps' round trips and the counters snapshots
# around six flat-out runs (one the two-flow run left out at 0.899) and a
# localhost one. Checks that:
#   * Fisher's exact test gives the textbook two-sided p values;
#   * analyse.py --ab prints expected-ab.md: medians and ranges per metric, a
#     verdict only where the ranges do not overlap, NAK runs pooled with
#     Fisher's p (a host's failed loss read, invalid=1, leaves its run out),
#     and the agents' poll-cycle gaps per rep (n/a where a snapshot is
#     missing or the driver changed between them);
#   * a mode given with its state compares the same as one given bare, and an
#     unknown mode fails, naming the modes there are;
#   * analyse.py prints expected-points.md: per load point, the reps that
#     lost anything of the reps run, and the NAKs sent;
#   * analyse.py prints expected-max.md as its sustained maximum table: the
#     flat-out ceiling with its loss, and the highest ladder rate below the
#     first lossy point (a NAK, a short send, an unechoed message, a failed
#     loss read; the transport's send-side counters alone are not loss), and
#     apart from it the highest rate it keeps up with (98% of offered);
#   * analyse.py prints expected-flows.md as its multi-flow sustained
#     maximum table: the aggregate, each flow, the ratio to the one-flow run
#     of the same rep (the 32 B flat-out run at 32 B), counting only runs
#     whose flows overlapped at least 0.9 (0.900 counts, 0.899 does not, nor
#     an unknown overlap: both reported as left out, with the A/B's rows),
#     a failed flow's run counted apart, and the loss counters with refused
#     sends apart; without multi-flow rows it leaves that table out and
#     changes nothing else;
#   * analyse.py prints expected-frames.md as its throughput phase frames
#     table (the localhost run left out: its frames cross no counted NIC; the
#     two-flow run left out for its overlap named, with no msgs/s, not
#     another rep's), and without the counters snapshots leaves only that
#     table out;
#   * analyse.py and spikes.py read gzipped spike files (spikes-*.csv.gz)
#     exactly as plain ones;
#   * analyse.py treats isolated-nic-irqs (vm.sh loss: the data NICs'
#     interrupts on CPUs other than the housekeeping one) as a check on the
#     run, never loss: a load point where only it grew stays loss-free, the
#     multi-flow table leaves it out of its loss counters, and a note under
#     the loss tables lists each run where it grew (a DPDK one too: vm.sh
#     counts the IRQs the kernel keeps on the NICs DPDK holds), says so where
#     none did, names a run a host did not read it in (unknown there, never
#     counted as none), and is absent from results without it; under a
#     mode's @IRQ= it says the queue IRQs count off the CPU the mode chose;
#   * irqs.sh against a fake /proc: irqs_check lists every IRQ with a
#     handler whose effective CPUs are not the housekeeping one alone, by
#     name (smp_affinity_list where a kernel has no effective list, or an IRQ
#     not started has it empty); irqs_pin stops irqbalance, moves every IRQ
#     that lets it and new IRQs' default, and lists what stayed (a managed
#     IRQ, whose affinity refuses the write); irqs_nic_off counts the data
#     NICs' interrupts (IRQs named after their interfaces or naming their PCI
#     addresses, not a near miss of either; under DPDK igb_uio's, by address)
#     in every CPU's column but housekeeping's, each IRQ once however many
#     NICs it names, 0 with no NIC given, and nothing, never 0, where a NIC
#     given matches no IRQ; a mode's @IRQ=: the data NICs' queue IRQs (named
#     after their interfaces, not a near miss, not the management IRQ)
#     pinned to its CPU and checked there (fail closed), and counted off it;
#   * lib.sh's allowance_grew (ENA's allowance counters that grew between
#     two counters snapshots: ethtool's or the PMD's, none from a missing
#     snapshot or another embedded driver), and in a dry run the counters
#     read on each host before and after every load point and mixed run;
#   * vm.sh's kernel modes name the data NICs to the media driver
#     (RUSTERON_MEDIA_DRIVER_DATA_NICS: each interface, Azure's VF too), so
#     its NIC checks run; the DPDK mode does not;
#   * lib.sh, dry (every VM command faked), with LAB_TPUT_FLOWS=2 runs the
#     multi-flow sustained maximum in cross-host modes only, at 32 B and each
#     load size, after the mode's other runs and before its drivers stop: one
#     channel port and one app CPU per flow at each end, the flows at once
#     (no publisher ends before the last starts, no subscriber before then
#     either), its row (a failed flow's left empty) with the flows' overlap
#     from their subscribers' windows (truncated to the thousandth, empty
#     when a flow has no window, each shortfall logged), loss rows and
#     counters snapshots; that the tput line's readers read the fields they
#     did with its window appended; that unset, 0 or 1 runs none and the
#     app CPUs cap it; that it reaches VM a's environment; and that with it
#     unset nothing multi-flow runs and every other row is the same;
#   * both lab.sh refuse a mode with a comma (it would split results.csv)
#     before any cloud call, and snapshot themselves in $TMPDIR (not the
#     user's temporary directory, where macOS's mktemp -d puts them);
#   * aws/lab.sh, against a fake aws, refuses a Graviton (Arm) type without
#     LAB_ALLOW_ARM=1, an Arm beside an x86 instance with it, an unknown
#     Aeron threading mode, two in one mode and an unknown state, before any
#     cloud call; with it, has AWS confirm both types are arm64 (refusing one
#     that is not) and takes Debian 13's arm64 AMI; an x86 pair makes the
#     calls it did before; and LAB_DELETE deletes and verifies whatever else
#     is set, none of those checks in its way;
#   * host.sh against a fake /sys and fake driver logs: the architectures
#     vm.sh runs on, Graviton's builds for the instance's own core, the core
#     layout (every Graviton CPU a core, x86's first
#     SMT thread of each), the kernel command line by architecture (arm64:
#     iommu.passthrough=1 from bootstrap; isolated-poll's idle polling,
#     nohlt or idle=poll), each Aeron threading mode's CPUs and idle
#     strategies (an unknown one refused, as Aeron would fall back to
#     DEDICATED), ENA's low-latency queues on every port and their
#     write-combining mapping (fail closed), the IOMMU in passthrough, and the
#     transparent hugepage sizes put back to the top-level setting;
#   * vm.sh itself, in a sandbox of fake sudo, uname and nproc: isolate on a
#     host already isolated still covers every CPU but the first, x86's line
#     as before Graviton, Graviton's and isolate poll's by architecture; and
#     bootstrap refuses aarch64 without LAB_ALLOW_ARM=1 before any sudo;
#   * lib.sh, dry: the isolated-poll state; DPDK modes stop a run on a host
#     whose media driver has no DPDK transport, kernel modes do not;
#   * analyse.py names threading modes and isolated-poll readably, and sorts
#     that state after isolated and before any it does not know; spikes.py
#     attributes preemptions of the shared agents;
#   * the same-host hairpin (@HAIRPIN=1) and sparse round trips
#     (@RTT_GAP_US): modes.sh takes and refuses them (and rtt_plan's seconds,
#     round trips and warm-up; four cores on host a), lib.sh runs a hairpin
#     as one floor client on host a and hands a paced mode's clients its
#     pacing and plan, vm.sh client passes RTT_GAP_US, vm.sh routes a kernel
#     hairpin out of its NICs (fail closed, put back) and ready refuses a
#     host without the receiver core, analyse.py tables both, spikes.py
#     judges a hairpin on host a alone and a paced run's length by its gap,
#     estimate.py times and sizes both, and aws/lab.sh refuses what cannot
#     run before any cloud call;
#   * aws/lab.sh end to end against a stateful fake aws (fake_aws.py): a
#     cluster run, a precision-time run and LAB_DELETE make exactly the calls
#     of expected-aws-calls.txt (the lab's before cross-AZ and cross-region,
#     its waits since polled), delete everything and verify it; cross-AZ (a
#     subnet pair per zone, the next pair on no capacity, LAB_AZS) and
#     cross-region (peered VPCs, routes, security groups by CIDR, AMI and key
#     per region, all tagged, deleted and verified in both, LAB_DELETE of a
#     kept run too, and a run that fails or is interrupted); no placement
#     holds the CLI lock across an aws ... wait, and instances that never run
#     time out after 60 polls, said so, and are deleted; LAB_THREADS_PER_CORE=1
#     launches with every core at one thread and changes nothing else;
#     LAB_SPOT=1 launches one-time spot instances, their requests tagged and
#     swept (an open one cancelled; LAB_DELETE too, from the run's
#     directory), the market recorded, nothing else changed but the spot
#     watch's looks; a spot instance reclaimed mid-run, as it starts or at
#     launch ends the run through the teardown at once, saying so, exit 75,
#     with LAB_KEEP=1 too (nothing kept of a broken pair); a TERM during the
#     teardown does not cut it short; a run started in another's second
#     takes a stamp of its own;
#   * modes.sh's settings and checks, which both lab.sh apply before any cloud
#     call (INVOKER, jumbo frames, SMT=off, Aeron URI parameters by side,
#     message sizes up to 8192 B skipped per mode, an MTU the path or the
#     DPDK transport cannot carry, the overrides the DPDK transport refuses
#     at install, LAB_NO_FLATOUT and the placements it guards); LAB_ESTIMATE's
#     arithmetic (estimate.py: the four-core types priced) and no cloud call,
#     with LAB_SPOT=1 the on-demand figure as an upper bound; a plan longer
#     than LAB_MAX_HOURS said so, a real run of it refused before any cloud
#     call; lib.sh's wait for a state as long as LAB_MAX_HOURS; lib.sh with
#     LAB_NO_FLATOUT runs nothing flat out; @IRQ=, @NAPI_DEFER= and
#     @NAPI_BUSY= (kernel modes; IRQ on AWS; the spare core not with both
#     ends on one host; no comma);
#   * host.sh's network helpers (ping summaries, gateways, policy routes to an
#     off-subnet peer, jumbo environment, the path MTU check, the effective
#     MTU, the lost-round-trip timeout), INVOKER's and the shared modes'
#     checks (the shared agent's CPU, the embedded drivers' ledger and the
#     client gate, Graviton's LLQ check on a driver started with the embedded
#     drivers' environment, SMT); lib.sh's path check before a JUMBO mode, its size
#     skips, MTU and network round-trip rows, per-host subnets, ledger-aware
#     loss rows and INVOKER's client check; vm.sh client's parameters and
#     INVOKER environment; vm.sh build's client features (the driver's,
#     dpdk included where it has it: INVOKER embeds it); analyse.py's round
#     trip above the network's and INVOKER's frames;
#   * a four-core host (c7i.2xlarge, SMT on and off; c8g.xlarge): its layout,
#     every threading mode's CPUs, the CPU each @IRQ= names, no spare core,
#     and the gap spinner never on a core of the layout; host.sh's NAPI
#     settings and threaded busy polling against a fake ynl (fail closed);
#     vm.sh's @IRQ=, @NAPI_DEFER= and @NAPI_BUSY= set for a mode and undone
#     after it against a fake /proc and /sys, and SMT=off's gap spinner
#     paused (marked for spikes.py, which then rules no host stall out) and
#     started again; vm.sh ready (a fifth core, Linux 6.17 and ynl) and build-ynl (ynl from
#     linux-source at exactly the running kernel's release, fail closed);
#     lib.sh: no SSH once a run is abandoned, the market in the hosts'
#     records, no second flow on the spare core, and prepare_vms asking for
#     ynl and checking each host's readiness before any state;
#   * the clock benchmarks' removal: aws/lab.sh refuses LAB_CLOCK_ONLY and
#     LAB_DISCIPLINE_SECONDS before any cloud call, never a full run in their
#     place;
#   * the pre-flight before any cloud call (modes.sh, both lab.sh): an
#     upper-case @KEY that is neither a setting the lab knows nor an AERON_*
#     or RUSTERON_* variable, a pair or transport vm.sh would refuse, the
#     RUSTERON_DPDK_* values md-dpdk's config.rs validates and the NICs' ring
#     limits (ENA 8192 receive, 512 transmit; MANA 256), NIC RX timestamps
#     outside precision-time, @CONDUCTOR=own's rules, the fifth core by each
#     type's physical cores (a type the table lacks refused, said so), the
#     PTP hardware clock's families and precision-time's DPDK timestamp pair,
#     a warm-up without its rate;
#   * @CONDUCTOR=own in vm.sh (the conductor alone on the fifth core,
#     spinning, housekeeping named apart, undone by `stop`, refused where it
#     cannot be) and the fail-closed check of its thread at the driver's start;
#   * Aeron's stall telemetry: the duty-cycle thresholds by Aeron 1.53.3's C
#     names, the counters read by their exact labels (no near miss, no other
#     type), vm.sh loss carrying them, lib.sh's aeron rows beside unchanged
#     loss rows (INVOKER's ledger taking the longest cycle), analyse.py's
#     table of them, never loss;
#   * the conductor's duty-cycle threshold strictly above its idle strategy's
#     longest park (vm.sh duration_ns, conductor_max_park_ns,
#     conductor_cycle_threshold and conductor_cycle_threshold_apply: derived
#     after the mode's overrides and before the driver or INVOKER's clients
#     start, an explicit threshold that does not clear the park refused by
#     name, common_env never hard-coding it);
#   * each load point's histogram fetched (a stale one never), merged over its
#     reps by analyse.py with HdrHistogram's percentiles; the warm-up once a
#     mode and rep before its first ladder, recorded nowhere, and in
#     estimate.py's time and bytes; the new prices;
#   * time on AWS: chrony's configuration (Amazon Time Sync, the PHC first
#     under precision-time, idempotent), bootstrap's chrony and build-ptp's
#     PHC by devlink on the primary ENI (no out-of-tree driver, fail closed),
#     the PHC's selection, lib.sh's wait before each state and log around each
#     mode (its check stopping a mode), vm.sh timesync and timesync-wait, and
#     spikes.py's note of the hosts' clock offsets;
#   * both lab.sh remove their snapshot as they exit, after any teardown, and
#     a kept run's delete command names the checkout's lab.sh;
#   * lib.sh's on() against a fake ssh: a remote command that ran is never
#     run again (one killed by a signal is a lab fault, logged), a connection
#     failure still is, 18 times, and SIGTERM to the remote shell returns its
#     command's status; vm.sh client's deadline (LAB_CLIENT_DEADLINE) ends a
#     pong that ignores SIGTERM the grace after it or after the lab's
#     SIGTERM, vm.sh exiting itself (124, 137), none left running; end to
#     end, lib.sh's bounded wait on a client ends a hung SSH session, and
#     the lab's end_client never SIGTERMs its own ssh to the client; in a
#     dry run every answering client's hung session (a subscriber piped
#     into the results too) is ended whole at its wait's bound;
#   * the media driver's start-up host checks: vm.sh hostchecks (its warning
#     and info lines from driver.log, nothing else), lib.sh's hostcheck rows
#     per host and mode (an external driver's log, INVOKER's clients'
#     output, CSV-quoted, none said so, a host not read unknown and warned
#     of, never none) and analyse.py's table of them;
#   * the media-receive-timestamp mode: modes.sh takes media-rcv-ts-offset on
#     a subscription as reserved or a payload offset from 16 to 24, never
#     over the clients' stamp and sequence, and vm.sh hands it to the
#     clients as SUB_CHANNEL_PARAMS;
#   * the final round's plan (G3): accepted before any cloud call,
#     and LAB_ESTIMATE=1's hours, bytes and dollars at the lab's spot prices
#     beside the on-demand ceiling (estimate.py's spot table);
#   * ATTR-LAB: the idle cross-host round trip's clients stamp round trips of
#     30 us and loop stalls of 10 us or more (LAB_RTT_SPIKE_NS,
#     LAB_RTT_STALL_NS; refused unless whole nanoseconds), vm.sh client hands
#     both through sudo, each host's DPDK event log saved with the round trip;
#     spikes.py gives each rep's 10 largest round trips every overlapping
#     event (client stalls, the event log's agent gaps, ARP and housekeeping
#     calls, spinner gaps, KNOISE's), each label proven by mutation, their
#     ping index and phase in 1 s and 30 s, and each source against chance
#     (1,000 circular shifts, a fixed seed); analyse.py reads the ping column;
#   * KNOISE: vm.sh knoise_start against a fake tracefs (osnoise with
#     NO_OSNOISE_WORKLOAD on exactly the agent and app CPUs, its noise events,
#     IPI senders, compaction and migration; "osnoise unavailable" said, and
#     the tracer kept off where NO_OSNOISE_WORKLOAD cannot be set; a missing
#     event named), knoise_stop, diag-start refusing while a rep runs
#     and handing knoise_start the layout's CPUs, lib.sh record_knoise;
#   * vm.sh, irqs.sh, host.sh, modes.sh, lib.sh, remote.sh and both lab.sh
#     parse (bash -n), and analyse.py, spikes.py, estimate.py and
#     fake_aws.py compile.
#
# Run from anywhere:  bash md-dpdk/benches/cloud/test/test-lab.sh
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cloud=$(cd "$here/.." && pwd)
tmp=$(mktemp -d)
# the test's own directory, removed on any exit (an interrupt too): by its
# path from mktemp, checked
trap '[[ -n ${tmp:-} && -d $tmp && ${tmp##*/} == tmp.* ]] && rm -rf -- "$tmp"' EXIT
failed=0
fail() { echo "FAIL: $*" >&2; failed=1; }
pass() { echo "ok: $*"; }

# A table's padding and its separator rows' width are presentation: compare
# what is left.
normalise() {
    python3 -c '
import re, sys
for line in sys.stdin.read().rstrip("\n").split("\n"):
    line = re.sub(r"\s+", " ", line).strip()
    print(re.sub(r"-{3,}", "---", line))
'
}
# section <heading>: from that "## " heading to the next one
section() {
    awk -v h="$1" '$0 == h {on = 1; print; next} on && /^## / {exit} on {print}'
}
# drop <heading>: all but that "## " section
drop() {
    awk -v h="$1" '$0 == h {skip = 1; next} skip && /^## / {skip = 0} !skip'
}
same() { # <what> <expected file> <actual file>
    if diff -u <(normalise <"$2") <(normalise <"$3") >"$tmp/diff"; then
        pass "$1"
    else
        fail "$1"
        cat "$tmp/diff" >&2
    fi
}

python3 - "$cloud" <<'PY' && pass "Fisher's exact test" || fail "Fisher's exact test"
import sys
sys.path.insert(0, sys.argv[1])
import analyse
cases = [((3, 1, 1, 3), 34 / 70),  # the tea-tasting table
         ((3, 0, 0, 3), 0.1), ((5, 0, 0, 5), 2 / 252), ((1, 2, 2, 1), 1.0), ((0, 5, 0, 5), 1.0),
         ((1, 14, 5, 10), 100100 / 593775), ((1, 14, 5, 9), 38038 / 475020)]
for table, want in cases:
    got = analyse.fisher_exact(*table)
    assert abs(got - want) < 1e-12, (table, got, want)
PY

# the loss-free rule on its own: a short send is loss, the send-side
# counters behind it are not on their own (a full ring's queued vector)
python3 - "$cloud" "$tmp/lf.csv" <<'PY' && pass "loss-free counts short sends" || fail "loss-free counts short sends"
import sys
sys.path.insert(0, sys.argv[1])
import analyse
load = "{rep},load,isolated:{m},udp-r{rate},288,{rate},1,28000,30000,33000,38000,42000,60000,28500,1250000,1500000,1500000,0,{rate},9000\n"
zero = "naks-sent=0;naks-received=0;retransmits-sent=0;short-sends=0"
loss = "{rep},loss,isolated:{m}:288:r{rate},{h},{kv}\n"
rows = []
for rep in (1, 2):
    for m, at250, at500 in (("x", zero + ";dpdk-tx-eagain=9", zero.replace("short-sends=0", "short-sends=3")),
                            ("y", zero.replace("short-sends=0", "short-sends=1"), zero)):
        for rate, kv in ((250000, at250), (500000, at500)):
            rows.append(load.format(rep=rep, m=m, rate=rate))
            for h in "ab":
                rows.append(loss.format(rep=rep, m=m, rate=rate, h=h, kv=kv if (h, rep) == ("a", 2) else zero))
open(sys.argv[2], "w").writelines(rows)
# z keeps up at 250 k/s, falls behind at 500 k/s (achieved 400 k) with nothing
# lost, and keeps up again at 750 k/s: saturation is not loss
for rep in (1, 2):
    for rate, achieved in ((250000, 250000), (500000, 400000), (750000, 750000)):
        rows.append(load.format(rep=rep, m="z", rate=rate).replace(f",0,{rate},9000", f",0,{achieved},9000"))
        rows += [loss.format(rep=rep, m="z", rate=rate, h=h, kv=zero) for h in "ab"]
open(sys.argv[2], "w").writelines(rows)
r = analyse.read_results(sys.argv[2])
got = (analyse.loss_free(r, "isolated:x", 288), analyse.loss_free(r, "isolated:y", 288))
assert got == ("250,000/s", "none (250,000/s lost)"), got
got = (analyse.loss_free(r, "isolated:z", 288), analyse.keeps_up(r, "isolated:z", 288), analyse.keeps_up(r, "isolated:x", 288))
assert got == ("750,000/s", "250,000/s", "500,000/s"), got
PY

# the work behind the percentiles: scheduled (what the run owed), late (the
# sends that found the following slot already due) and censored (the round
# trips at the histogram's bound), counted with sent and received. A row from
# before they were counted still parses, and is reported without them rather
# than as a zero that would read as measured
python3 - "$cloud" "$tmp/work.csv" <<'PY' && pass "load rows carry scheduled, late and censored" || fail "load rows carry scheduled, late and censored"
import subprocess, sys
cloud, path = sys.argv[1], sys.argv[2]
# one point with the three counts, one point from before they were counted
new = ("1,load,isolated:dpdk,udp-r250000,288,250000,1,28000,30000,33000,38000,42000,60000,28500,"
       "1250000,1500000,1500000,0,250000,9000,1500000,12,3\n")
old = ("2,load,isolated:dpdk,udp-r500000,288,500000,1,28000,30000,33000,38000,42000,60000,28500,"
       "1250000,1500000,1500000,0,500000,9000\n")
open(path, "w").writelines([new, old])
sys.path.insert(0, cloud)
import analyse
r = analyse.read_results(path)
both = r.load[("isolated:dpdk", 288, "r250000")]
was = r.load[("isolated:dpdk", 288, "r500000")]
assert len(both) == 1 and both[0][12:] == [1500000, 12, 3], both
assert len(was) == 1 and was[0][12:] == [None, None, None], was
out = subprocess.run([sys.executable, f"{cloud}/analyse.py", path], capture_output=True, text=True)
assert out.returncode == 0, out.stderr
rows = [ln for ln in out.stdout.splitlines() if "250,000/s" in ln or "500,000/s" in ln]
assert any("1,500,000/1,500,000/1,500,000" in ln and "late 12, censored 3" in ln for ln in rows), rows
assert any("1,500,000/1,500,000" in ln and "late 12" not in ln for ln in rows), rows
PY

# isolated-nic-irqs, the data NICs' interrupts on CPUs other than the
# housekeeping one: a check on the run, never loss, in every mode. A kernel
# mode and a DPDK one, one rep each of the round trip, throughput, two load
# points and two flows at once, every loss row with it (or, for results from
# before it, without; or a host's row without it, a run where vm.sh could not
# read it); only it grows, in the runs given
python3 - "$cloud" "$tmp/nic-irqs" <<'PY' && pass "isolated-nic-irqs: a check, not loss" || fail "isolated-nic-irqs: a check, not loss"
import contextlib, io, os, subprocess, sys
sys.path.insert(0, sys.argv[1])
import analyse

def write(name, irqs):
    rows = []
    for m in ("kernel", "dpdk"):
        rows += [f"1,rtt,isolated:{m},udp,30000,31000,32000,33000,34000,90000,30500,100000,0\n",
                 f"1,tput,isolated:{m},udp,9000000,8900000,9100000,45000000,0\n",
                 f"1,tputflows,isolated:{m},32,2,18000000,9000000,9000000,0.995\n"]
        rows += [f"1,load,isolated:{m},udp-r{rate},288,{rate},1,28000,30000,33000,38000,42000,60000,28500,"
                 f"1250000,1500000,1500000,0,{rate},9000\n" for rate in (250000, 500000)]
        for run in ("rtt", "", "288:r250000", "288:r500000", "32:flows2"):
            label = f"isolated:{m}" + (f":{run}" if run else "")
            for h in "ab":
                kv = "naks-sent=0;short-sends=0"
                if irqs is not None and irqs.get((label, h), 0) is not None:
                    kv += f";isolated-nic-irqs={irqs.get((label, h), 0)}"
                rows.append(f"1,loss,{label},{h},{kv}\n")
    path = os.path.join(sys.argv[2], name, "results.csv")
    os.makedirs(os.path.dirname(path))
    open(path, "w").writelines(rows)
    return path

def note(path):
    report = subprocess.run([sys.executable, os.path.join(sys.argv[1], "analyse.py"), path],
                            capture_output=True, text=True, check=True).stdout
    return [l for l in report.splitlines() if l.startswith("Data NIC interrupts on CPUs other than")]

grew = write("grew", {("isolated:kernel", "a"): 5, ("isolated:kernel:288:r500000", "b"): 7,
                      ("isolated:kernel:32:flows2", "a"): 3, ("isolated:dpdk", "a"): 4,
                      ("isolated:dpdk:288:r500000", "a"): 2})
r = analyse.read_results(grew)
got = (analyse.loss_free(r, "isolated:kernel", 288), analyse.loss_free(r, "isolated:dpdk", 288))
assert got == ("500,000/s", "500,000/s"), f"loss-free: {got}"
out = io.StringIO()
with contextlib.redirect_stdout(out):
    analyse.flows(r)
row = [l.split("|") for l in out.getvalue().splitlines() if l.startswith("| isolated, kernel UDP ")]
assert row and row[0][-3].strip() == "none", f"the multi-flow table's loss counters: {row}"
lead = ("Data NIC interrupts on CPUs other than the housekeeping one (`isolated-nic-irqs`: on each host, the "
        "growth over the run of the data NICs' IRQ counts, under DPDK those of the IRQs the kernel keeps on their PCI "
        "addresses, in those CPUs' columns of /proc/interrupts; a check on the run, not loss. vm.sh pins every IRQ to "
        "the housekeeping CPU before each driver starts, and the kernel places a NIC's IRQs anew whenever it brings "
        "the NIC up or resets it): ")
want = [lead + "5 of 10 runs grew: `isolated:kernel` rep 1 (a 5); `isolated:kernel:288:r500000` rep 1 (b 7); "
        "`isolated:kernel:32:flows2` rep 1 (a 3); `isolated:dpdk` rep 1 (a 4); `isolated:dpdk:288:r500000` rep 1 (a 2)."]
assert note(grew) == want, f"the note: {note(grew)}"
none = write("none", {})
assert note(none) == [lead + "none of 10 runs grew."], f"the note where none grew: {note(none)}"
unread = write("unread", {("isolated:kernel:288:r250000", "b"): None, ("isolated:kernel", "a"): 5,
                          ("isolated:dpdk:rtt", "a"): None})
want = [lead + "1 of 10 runs grew: `isolated:kernel` rep 1 (a 5). Not read on a host, so unknown there: "
        "`isolated:kernel:288:r250000` rep 1 (b); `isolated:dpdk:rtt` rep 1 (a)."]
assert note(unread) == want, f"the note where a host did not read it: {note(unread)}"
before = write("before", None)
assert note(before) == [], f"the note without the counter: {note(before)}"
# a mode with @IRQ= (its data NICs' queue IRQs on the CPU it chose: vm.sh
# counts theirs off that CPU): the note says so; without one it does not
irq = os.path.join(sys.argv[2], "irq", "results.csv")
os.makedirs(os.path.dirname(irq))
open(irq, "w").writelines(f"1,loss,isolated:kernel@IRQ=receiver,{h},naks-sent=0;isolated-nic-irqs=0\n" for h in "ab")
want = [lead + "none of 1 runs grew. Under a mode's @IRQ= its data NICs' queue IRQs count on CPUs other than the one it "
        "chose (vm.sh pins them there; its driver log names it), and their other IRQs on CPUs other than the housekeeping one."]
assert note(irq) == want, f"the note with @IRQ=: {note(irq)}"
assert all("@IRQ=" not in n for n in note(grew) + note(none)), "the note without @IRQ="
PY

python3 "$cloud/analyse.py" --ab dpdk dpdk@BIN=bin-base "$here/fixture/results.csv" >"$tmp/ab.md" ||
    fail "analyse.py --ab exited $?"
same "analyse.py --ab" "$here/expected-ab.md" "$tmp/ab.md"

python3 "$cloud/analyse.py" --ab isolated:dpdk isolated:dpdk@BIN=bin-base "$here/fixture/results.csv" >"$tmp/ab-labels.md" ||
    fail "analyse.py --ab with states exited $?"
same "analyse.py --ab with states" "$here/expected-ab.md" "$tmp/ab-labels.md"

if python3 "$cloud/analyse.py" --ab dpdk dpdk@nothing "$here/fixture/results.csv" >"$tmp/ab-unknown.md" 2>"$tmp/ab-unknown.err"; then
    fail "analyse.py --ab with an unknown mode succeeded"
elif grep -q 'isolated:dpdk@BIN=bin-base' "$tmp/ab-unknown.err"; then
    pass "analyse.py --ab with an unknown mode fails, naming the modes"
else
    fail "analyse.py --ab with an unknown mode: $(cat "$tmp/ab-unknown.err")"
fi

python3 "$cloud/analyse.py" "$here/fixture/results.csv" >"$tmp/full.md" || fail "analyse.py exited $?"
section "## Throughput phase frames" <"$tmp/full.md" >"$tmp/frames.md"
same "analyse.py throughput phase frames" "$here/expected-frames.md" "$tmp/frames.md"
section "## Loss by load point (reps that lost anything, of the reps run; NAKs sent)" <"$tmp/full.md" >"$tmp/points.md"
same "analyse.py loss by load point" "$here/expected-points.md" "$tmp/points.md"
# a short send an ARP miss explains (refused while a next hop's first ARP
# resolution is in flight, retried by Aeron) is start-up, not loss: every
# INVOKER client's embedded driver resolves anew, so round A's c9g pair
# "lost" at every DPDK INVOKER point on 2 short sends = 2 ARP misses. One
# beyond the ARP misses, or one with none, is still loss
python3 - "$cloud" <<'PY' && pass "analyse.py: short sends an ARP miss explains are not loss; any beyond them are" || fail "analyse.py: short sends and ARP misses"
import sys, types
sys.path.insert(0, sys.argv[1])
import analyse
def lost(a, b):
    r = types.SimpleNamespace(load_echo={("isolated:dpdk", 288, "r100000", 1): (10, 10)},
                              loss_rep={("isolated:dpdk:288:r100000", 1, "a"): a, ("isolated:dpdk:288:r100000", 1, "b"): b})
    return analyse.rep_lost(r, "isolated:dpdk", 288, "r100000", 1)
assert not lost({"short-sends": 2, "dpdk-arp-miss": 2}, {"short-sends": 0}), "ARP-explained short sends counted as loss"
assert not lost({"short-sends": 1, "dpdk-arp-miss": 2}, {}), "fewer short sends than ARP misses counted as loss"
assert lost({"short-sends": 3, "dpdk-arp-miss": 2}, {}), "a short send beyond the ARP misses not counted"
assert lost({"short-sends": 2}, {}), "a short send with no ARP miss not counted"
assert lost({"naks-sent": 1, "short-sends": 2, "dpdk-arp-miss": 2}, {}), "a NAK not counted"
PY
section "## Sustained one-way maximum by message size (M msgs/s)" <"$tmp/full.md" >"$tmp/max.md"
same "analyse.py sustained maximum, flat-out and loss-free" "$here/expected-max.md" "$tmp/max.md"
flows="## Sustained maximum over several flows at once (M msgs/s)"
section "$flows" <"$tmp/full.md" >"$tmp/flows.md"
same "analyse.py sustained maximum over several flows" "$here/expected-flows.md" "$tmp/flows.md"

# without multi-flow rows (LAB_TPUT_FLOWS unset, as by default, or no fifth core): no such
# table, and the rest as before (the frames table aside: it needs the state
# directory's snapshots)
mkdir -p "$tmp/one-flow"
grep -v ',tputflows,' "$here/fixture/results.csv" >"$tmp/one-flow/results.csv"
python3 "$cloud/analyse.py" "$tmp/one-flow/results.csv" >"$tmp/one-flow.md" || fail "analyse.py without multi-flow rows exited $?"
if grep -q 'several flows' "$tmp/one-flow.md"; then
    fail "analyse.py printed the multi-flow table without multi-flow rows"
elif diff -u <(drop "$flows" <"$tmp/full.md" | drop "## Throughput phase frames") "$tmp/one-flow.md" >"$tmp/diff"; then
    pass "analyse.py without multi-flow rows"
else
    fail "analyse.py without multi-flow rows changed more than the multi-flow table"
    cat "$tmp/diff" >&2
fi

# without the snapshots: the same report less that one table
cp -R "$here/fixture" "$tmp/bare"
rm "$tmp/bare"/isolated/*-counters-*
python3 "$cloud/analyse.py" "$tmp/bare/results.csv" >"$tmp/bare.md" || fail "analyse.py without snapshots exited $?"
drop "## Throughput phase frames" <"$tmp/full.md" >"$tmp/full-less-frames.md"
if grep -q 'Throughput phase frames' "$tmp/bare.md"; then
    fail "analyse.py printed the frames table without snapshots"
elif diff -u "$tmp/full-less-frames.md" "$tmp/bare.md" >"$tmp/diff"; then
    pass "analyse.py without snapshots"
else
    fail "analyse.py without snapshots changed more than the frames table"
    cat "$tmp/diff" >&2
fi

# spike files gzipped (the large ones are, to save disk): both reports the same.
# Two of them, an idle round trip with 30 spikes and a load point's, written
# into the state directory and, for analyse.py's per-run summary, next to
# results.csv
cp -R "$here/fixture" "$tmp/gz"
python3 - "$tmp/gz" <<'PY'
import os, sys
for d in (sys.argv[1], os.path.join(sys.argv[1], "isolated")):
    for name, n in (("spikes-isolated-dpdk-rtt-1.csv", 30), ("spikes-isolated-dpdk-288-1-r250000.csv", 3)):
        with open(os.path.join(d, name), "w") as f:
            f.write("# start_realtime_ns=1792032877578595733\nstream,due_ns,latency_ns\n")
            f.writelines(f"0,{i * 33_000_007},{150_000 + 20_000 * i}\n" for i in range(n))
            f.write(f"# spikes_total={n}\n")
for host in "ab":  # spikes.py needs each host's diagnostics; empty ones explain nothing
    with open(os.path.join(sys.argv[1], f"{host}-diag-isolated.log"), "w") as f:
        f.write("== gapspin\n== trace\n")
PY
python3 "$cloud/analyse.py" "$tmp/gz/results.csv" >"$tmp/plain.md" || fail "analyse.py with spike files exited $?"
python3 "$cloud/spikes.py" "$tmp/gz/isolated" >"$tmp/plain-spikes.md" || fail "spikes.py exited $?"
gzip "$tmp/gz"/spikes-*.csv "$tmp/gz"/isolated/spikes-*.csv
python3 "$cloud/analyse.py" "$tmp/gz/results.csv" >"$tmp/gzipped.md" || fail "analyse.py with gzipped spike files exited $?"
python3 "$cloud/spikes.py" "$tmp/gz/isolated" >"$tmp/gzipped-spikes.md" || fail "spikes.py with gzipped spike files exited $?"
if ! grep -Eq 'isolated-dpdk-rtt +\| 1 +\| 30 ' "$tmp/plain.md" || ! grep -q 'isolated-dpdk-rtt | 27 ' "$tmp/plain-spikes.md"; then
    fail "the spike files were not read"
elif diff -u "$tmp/plain.md" "$tmp/gzipped.md" >"$tmp/diff" && diff -u "$tmp/plain-spikes.md" "$tmp/gzipped-spikes.md" >>"$tmp/diff"; then
    pass "gzipped spike files read as plain ones"
else
    fail "gzipped spike files read differently"
    cat "$tmp/diff" >&2
fi

# lib.sh dry, as remote.sh runs it on VM a: every VM command faked (on), VM
# a's layout with a fifth core (app CPUs 2 and 4), VM b's too (6 and 7); one
# rep of a cross-host mode, the same with an idle subscription, and a
# localhost mode, with three load sizes and one mix profile, run with
# LAB_TPUT_FLOWS=2 and again with it unset. Every fake subscriber prints its
# measured window on the wall clock, as tput does. The fake multi-flow
# clients take a while, as real ones do, and log when they start and end. Per
# size, by port: at 32 B (22700-22701) the flows overlap 0.996 of the
# shortest window; at 288 B (22702-22703) the second starts 0.8972 s late and
# its window is the shortest, so they overlap 0.85475, 0.854 truncated; at
# 1344 B the subscriber on port 22705 prints nothing (a failed flow); at 64 B
# the one on 22707 prints no window (a client from before windows).
dry=$tmp/dry
mkdir -p "$dry"
cat >"$tmp/dry.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { command sleep 0.05; }
S=1792032877000000000
# a client's label (vm.sh client <label> ...), which it prints as tput does
label() { local l=${1#*client }; echo "${l%% *}"; }
on() {
    local host=$1 cmd
    shift
    cmd=$*
    printf '%s %s\n' "$host" "$cmd" >>"$out/calls"
    case $cmd in
    *"vm.sh nics") echo "receiver ens6 ens6v 00:00:00:00:00:00 $([[ $host == "$A" ]] && echo 10.30.1.5 || echo 10.30.1.7) 24 0000:00:07.0" ;;
    *"vm.sh layout") [[ $host == "$A" ]] && echo "0 2 1 3 4" || echo "0 6 1 3 7" ;;
    *"vm.sh loss") echo "naks-sent=0;short-sends=0" ;;
    *"vm.sh counters") echo " 1 82 rusteron-dpdk role=sender bdf=0000:00:06.0 port=0 q=0 dir=tx pkts;" ;;
    *" rtt idle "*) echo "idle subscription" ;;
    *" tput x"[ps]"ub "*:227[0-9][0-9]" "*)
        # a subscriber lasts until its publisher has sent a while, a publisher its seconds
        local role=${cmd#* tput x} ep=${cmd#* tput x?ub } port window got
        role=${role%% *} ep=${ep%% *} port=${ep##*:}
        echo "$role-start $port" >>"$out/flow-order"
        command sleep "$([[ $role == sub ]] && echo 0.8 || echo 0.3)"
        echo "$role-end $port" >>"$out/flow-order"
        # the window and the messages it carried. The row's msgs/s is tput's
        # median of its one-second samples; multi_flow_max reports the mean
        # over the window (got / (end - start)), which is what a concurrent
        # sum can add. At 22701 and 22703 the mean is deliberately below the
        # median, so a sum of medians and a sum of means differ
        case $port in
        22700) window=,$S,$((S + 5000000000)) got=113500000 ;;            # mean 22700000
        22701) window=,$((S + 20000000)),$((S + 5030000000)) got=110220000 ;; # mean 22000000
        22702) window=,$S,$((S + 5000000000)) got=100000000 ;;            # mean 20000000
        22703) window=,$((S + 897200000)),$((S + 5697200000)) got=101280000 ;; # mean 21100000
        22704) window=,$S,$((S + 5000000000)) got=113520000 ;;            # mean 22704000
        22706) window=,$S,$((S + 5000000000)) got=113530000 ;;            # mean 22706000
        22707) window= got=106000000 ;;                                   # no window: no mean
        esac
        [[ $role == sub && $port != 22705 ]] && echo "tput,$(label "$cmd"),udp,${port}000,1,2,$got,0$window"
        ;;
    *" tput xsub "*)
        local ep=${cmd#* tput xsub }
        ep=${ep%% *}
        echo "tput,$(label "$cmd"),udp,${ep##*:}000,1,2,3,0,$S,$((S + 5000000000))"
        ;;
    *"/env") cat >"$out/env-written" ;;
    *"/rc 2>/dev/null") echo 0 ;;
    esac
    return 0
}
reps=1 local_modes=kernel load_sizes='288 1344 64' load_fractions=50 load_seconds=1 load_ladder= mix='p=s:3001:1000:32' mix_seconds=1
tput_seconds=5 rtt_messages=10 rtt_warmup=1 rtt_align=
if [[ $3 == unset ]]; then unset tput_flows; else tput_flows=$3; fi
modes=(dpdk dpdk+idle)
bench_state isolated
[[ $3 == unset ]] && exit 0
# how many flows: none unless LAB_TPUT_FLOWS is 2 or more, capped at the app
# CPUs both VMs have
for tput_flows in "" 0 1 2 3; do
    echo "${tput_flows:-unset} $(flow_count "2 4" "6 7"), one app CPU $(flow_count "2" "6 7")"
done >"$out/counts"
# VM a's environment
tput_flows=2 a_private=10.30.0.10 b_private=10.30.0.11
remote_state isolated
SH
bash "$tmp/dry.sh" "$cloud" "$dry" 2 </dev/null 2>"$dry/stderr" || fail "lib.sh dry run exited $?"
S=1792032877000000000
expect_rows='1,tputflows,isolated:dpdk,32,2,44700000,22700000,22000000,0.996
1,tputflows,isolated:dpdk,288,2,41100000,20000000,21100000,0.854
1,tputflows,isolated:dpdk,1344,2,,22704000,,
1,tputflows,isolated:dpdk,64,2,,22706000,22707000,'
expect_loss=$(for size in 32 288 1344 64; do for node in a b; do
    echo "1,loss,isolated:dpdk:$size:flows2,$node,naks-sent=0;short-sends=0"
done; done)
# each flow's subscriber on VM b and publisher on VM a: its own port and app CPU
expect_calls=$(
    i=0
    for size in 32 288 1344 64; do
        for cpu in 2 4; do
            echo "192.0.2.1 LAB_CLIENT_DEADLINE=84 ergon/md-dpdk/benches/cloud/vm.sh client isolated:dpdk tput xpub 10.30.1.7:$((22700 + i)) 11 $cpu $size"
            i=$((i + 1))
        done
    done
    i=0
    for size in 32 288 1344 64; do
        for cpu in 6 7; do
            echo "192.0.2.2 LAB_CLIENT_DEADLINE=102 ergon/md-dpdk/benches/cloud/vm.sh client isolated:dpdk tput xsub 10.30.1.7:$((22700 + i)) 5 $cpu"
            i=$((i + 1))
        done
    done | sort
)
snaps=0
for node in a b; do
    for when in before after; do
        for size in 32 288 1344 64; do
            grep -q '^# realtime=' "$dry/$node-counters-flows-$when-isolated-dpdk-$size-1.log" 2>/dev/null && snaps=$((snaps + 1))
        done
    done
done
# Each size's flows at once: every publisher started before any ends, and
# every subscriber still up when the last starts (one after another, each
# flow would have the path to itself, and their sum would pass for a per-flow
# ceiling). And after the mode's every other run, its drivers still up.
order=$(python3 - "$dry/flow-order" "$dry/calls" 2>&1 <<'PY'
import re, sys
events = [line.split() for line in open(sys.argv[1])]
for first in (22700, 22702, 22704, 22706):  # 32, 288, 1344 and 64 B
    at = lambda what: [i for i, (e, p) in enumerate(events) if e == what and first <= int(p) < first + 2]
    if len(at("pub-start")) != 2 or len(at("pub-end")) != 2 or len(at("sub-end")) != 2:
        print(f"ports {first}-{first + 1}: not two flows: {events}")
    elif max(at("pub-start")) > min(at("pub-end")):
        print(f"ports {first}-{first + 1}: publishers one after another: {events}")
    elif max(at("pub-start")) > min(at("sub-end")):
        print(f"ports {first}-{first + 1}: a subscriber ended before the last publisher started: {events}")
calls = open(sys.argv[2]).read().splitlines()
flows = [i for i, c in enumerate(calls) if re.search(r" tput x(sub|pub) \S+:227\d\d ", c)]
others = [i for i, c in enumerate(calls) if "client isolated:dpdk " in c and i not in flows]
stops = [i for i, c in enumerate(calls) if c.endswith("vm.sh stop")]
if not flows or not others or max(others) > flows[0] or any(min(others) < i < flows[-1] for i in stops):
    print("not after the mode's other runs with its drivers up")
PY
) || order="the check failed: $order"
# The tput line's two new fields at its end: every reader still reads the
# fields it did (the throughput row's msgs/s, received and lost; the load
# sweep's maximum, and the rate it derives from it), and analyse.py counts the
# 32 B run, leaves out the 288 B and 64 B ones, and counts the 1344 B one failed
readers=$(python3 - "$cloud" "$dry/results.csv" "$dry/calls" 2>&1 <<'PY'
import sys
sys.path.insert(0, sys.argv[1])
import analyse
r = analyse.read_results(sys.argv[2])
calls = open(sys.argv[3]).read()
checks = [
    (r.tput_rep.get(("isolated:dpdk", 1)), 20125000.0),
    (r.tput_lost.get("isolated:dpdk"), [(3, 0)]),
    (r.tputmax_rep.get(("isolated:dpdk", 288, 1)), 20125000.0),
    ("load xping 10.30.1.7:22200 10.30.1.5:22201 10062500 1 288 2" in calls, True),
    ([(rep, total, overlap) for rep, total, _, overlap in r.tputflows.get(("isolated:dpdk", 32, 2), [])],
     [(1, 44700000.0, 0.996)]),
    # each flow's mean over its own window, not its median (22700000 and
    # 22701000): a sum of medians is not what the flows carried at once
    (r.tputflows.get(("isolated:dpdk", 32, 2), [])[0][2], [22700000.0, 22000000.0]),
    (dict(r.tputflows_apart), {("isolated:dpdk", 288, 2): [(1, 0.854)], ("isolated:dpdk", 64, 2): [(1, None)]}),
    (dict(r.tputflows_failed), {("isolated:dpdk", 1344, 2): 1}),
]
for got, want in checks:
    if got != want:
        print(f"got {got!r}, want {want!r}")
PY
) || readers="the check failed: $readers"
if grep -E 'unbound variable|command not found|syntax error' "$dry/stderr"; then
    fail "lib.sh dry run: shell errors"
elif [[ $(grep ',tputflows,' "$dry/results.csv" 2>/dev/null) != "$expect_rows" ]]; then
    fail "lib.sh dry run: multi-flow rows: $(grep ',tputflows,' "$dry/results.csv" 2>/dev/null; tail -5 "$dry/stderr" "$dry/log" 2>/dev/null)"
elif [[ $(grep ':flows' "$dry/results.csv") != "$expect_loss" ]]; then
    fail "lib.sh dry run: multi-flow loss rows: $(grep ':flows' "$dry/results.csv")"
elif [[ $(grep -E 'tput x(sub|pub) [^ ]*:227' "$dry/calls" | sort) != "$expect_calls" ]]; then
    fail "lib.sh dry run: multi-flow clients: $(grep -E 'tput x(sub|pub) [^ ]*:227' "$dry/calls" | sort)"
elif ((snaps != 16)); then
    fail "lib.sh dry run: $snaps of 16 multi-flow counters snapshots"
elif [[ -n $order ]]; then
    fail "lib.sh dry run: multi-flow order: $order"
elif [[ $(grep -c "^1,tput,isolated:dpdk,udp,20125000,1,2,3,0,$S,$((S + 5000000000))\$" "$dry/results.csv") != 1 ]] ||
    [[ $(grep '^1,tputmax,isolated:dpdk,' "$dry/results.csv") != $'1,tputmax,isolated:dpdk,288,20125000,1,2\n1,tputmax,isolated:dpdk,1344,20125000,1,2\n1,tputmax,isolated:dpdk,64,20125000,1,2' ]] ||
    [[ -n $readers ]]; then
    fail "lib.sh dry run: the tput line's readers: $readers $(grep -E ',tput(max)?,isolated:dpdk,' "$dry/results.csv")"
elif ! grep -q 'WARNING: isolated:dpdk 1344 B, 2 flows at once: no result from flow 2' "$dry/log"; then
    fail "lib.sh dry run: the failed flow was not reported: $(cat "$dry/log")"
elif ! grep -q 'WARNING: isolated:dpdk 64 B, 2 flows at once: no measured window from flow 2' "$dry/log"; then
    fail "lib.sh dry run: the flow without a window was not reported: $(cat "$dry/log")"
elif ! grep -q 'WARNING: isolated:dpdk 288 B, 2 flows at once: their measured windows overlapped 0.854 of the shortest' "$dry/log" ||
    grep -q 'isolated:dpdk 32 B, 2 flows at once: their measured windows' "$dry/log"; then
    fail "lib.sh dry run: the overlap was not reported as it should be: $(cat "$dry/log")"
elif [[ $(cat "$dry/counts") != $'unset 0, one app CPU 0\n0 0, one app CPU 0\n1 0, one app CPU 0\n2 2, one app CPU 0\n3 2, one app CPU 0' ]]; then
    fail "lib.sh dry run: flow counts: $(cat "$dry/counts")"
elif ! grep -q 'LAB_TPUT_FLOWS=3, but' "$dry/log" || ! grep -q 'LAB_TPUT_FLOWS=2, but' "$dry/log"; then
    fail "lib.sh dry run: LAB_TPUT_FLOWS over the app CPUs was not reported: $(cat "$dry/log")"
elif ! grep -qx "tput_flows='2'" "$dry/env-written" 2>/dev/null; then
    fail "lib.sh dry run: LAB_TPUT_FLOWS did not reach VM a's environment"
else
    pass "lib.sh dry: multi-flow sustained maximum with LAB_TPUT_FLOWS=2"
fi

# Every load point and mixed run in that dry run has vm.sh counters (snap:
# the NICs' allowance counters, ENA's *_allowance_exceeded, with the
# transport's and the NICs' packets) on each of its hosts just before it
# starts and after it ends: cross-host on both, localhost on VM a; and none
# grew there, so no allowance warning
points=0 missing=
for f in "$dry"/load-*.log "$dry"/mix-*.log; do
    [[ -e $f ]] || continue
    key=${f##*/}
    key=${key%.log}
    key=${key#load-}
    key=${key#mix-}
    nodes="a b"
    [[ $key == *-local-* ]] && nodes=a
    for node in $nodes; do
        for when in before after; do
            grep -q '^# realtime=' "$dry/$node-counters-load-$when-$key.log" 2>/dev/null || missing+=" $node-counters-load-$when-$key"
        done
    done
    points=$((points + 1))
done
if ((points < 14)) || [[ -n $missing ]] || grep -q 'allowance' "$dry/log"; then
    fail "lib.sh dry run: the NICs' counters around every load point: $points points, missing:$missing $(grep 'allowance' "$dry/log")"
else
    pass "lib.sh dry: the NICs' allowance counters read on each host before and after every load point and mixed run ($points)"
fi

# Every client the dry run started carries its lab-side deadline (vm.sh
# client's LAB_CLIENT_DEADLINE), each kind its own: a ping or publisher twice
# its schedule and a minute (the round trip's 25 s, a load point's and the
# mix's 1 s, the publisher's 11 s), a client that answers one (pong, echo,
# subscriber) half a minute more, the idle subscription its hour and a
# minute; and lib.sh never waits on a client's job but through wait_bounded
deadlines=$(python3 - "$dry/calls" <<'PY'
import re, sys
seen = set()
for line in open(sys.argv[1]):
    m = re.search(r"/vm\.sh client \S+ (\S+) (\S+)", line)
    if m:
        d = re.search(r"\bLAB_CLIENT_DEADLINE=(\d+) ", line)
        seen.add(f"{m[1]} {m[2]} {d[1] if d else 'none'}")
print(";".join(sorted(seen)))
PY
) || true
bare=$(grep -nE '\bwait +"?\$\{?(pong|sub|subs|pub|pubs|echo|idle_pids)\b' "$cloud/lib.sh" || true)
if [[ $deadlines != "load xecho 94;load xmix 64;load xping 64;rtt idle 3660;rtt xping 112;rtt xpong 142;rtt xpong 94;tput xpub 84;tput xsub 102" ]]; then
    fail "lib.sh dry: the clients' deadlines: $deadlines"
elif [[ -n $bare ]]; then
    fail "lib.sh: a wait on a client's job not bounded (wait_bounded): $bare"
else
    pass "lib.sh dry: every client started with its deadline, an answering client's past its ping's; every wait on a client's job bounded"
fi

# lib.sh dry against SSH sessions that hang: every answering client bench_state
# starts in the background and then waits on (a pong, an echo, a throughput
# subscriber; a cross-host and a localhost mode, with a load size and a mix),
# its on() never returning, each wait's bound 1 s (client_bound). Every one,
# a throughput subscriber piped through sed into the results too, is ended
# at its bound by wait_bounded, its whole job (bash's $! of a pipeline is its
# last process, sed, whose wait waits for the whole pipeline), a lab fault
# naming it; none runs on to its end (40 s), none is left running. And
# every client run in the foreground (a ping, a publisher, a load client)
# ends 124, as vm.sh client does once its deadline ended it: a lab fault
# each (client_rc).
hung=$tmp/dry-hung
mkdir -p "$hung"
cat >"$tmp/dry-hung.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { command sleep 0.05; }
client_bound() { echo 1; }
on() {
    local host=$1 cmd
    shift
    cmd=$*
    case $cmd in
    *"vm.sh nics") echo "receiver ens6 ens6v 00:00:00:00:00:00 $([[ $host == "$A" ]] && echo 10.30.1.5 || echo 10.30.1.7) 24 0000:00:07.0" ;;
    *"vm.sh layout") [[ $host == "$A" ]] && echo "0 2 1 3 4" || echo "0 6 1 3 7" ;;
    *"vm.sh loss") echo "naks-sent=0;short-sends=0" ;;
    *" rtt xpong "* | *" tput xsub "* | *" load xecho "*)
        # its SSH session hangs
        echo "$BASHPID ${cmd#*client }" >>"$out/hung"
        command sleep 40
        echo "${cmd#*client }" >>"$out/ran-out"
        ;;
    *"/vm.sh client "*)
        # past its deadline
        echo "${cmd#*client }" >>"$out/overran"
        return 124
        ;;
    esac
    return 0
}
reps=1 local_modes=kernel load_sizes=288 load_fractions=50 load_seconds=1 load_ladder= mix='p=s:3001:1000:32' mix_seconds=1
tput_seconds=5 rtt_messages=10 rtt_warmup=1 rtt_align=
unset tput_flows
modes=(kernel)
bench_state isolated
SH
bash "$tmp/dry-hung.sh" "$cloud" "$hung" </dev/null 2>"$hung/stderr" || fail "lib.sh dry, hung sessions: exited $?"
hung_left=0
while read -r pid _; do
    if kill -0 "$pid" 2>/dev/null; then
        hung_left=$((hung_left + 1))
        kill -KILL "$pid" 2>/dev/null || true
    fi
done <"$hung/hung"
hung_kinds=$(awk '{print $3, $4, ($5 ~ /^127\./ ? "local" : "cross")}' "$hung/hung" | sort -u | tr '\n' ';')
hung_n=$(wc -l <"$hung/hung" | tr -d ' ')
hung_faults=$(grep -c 'still running 1 s on (its SSH session hung): ended here' "$hung/lab-faults.log" 2>/dev/null || true)
overran_kinds=$(awk '{print $2, $3}' "$hung/overran" | sort -u | tr '\n' ';')
overran_n=$(wc -l <"$hung/overran" | tr -d ' ')
overran_faults=$(grep -c 'overran its deadline: vm.sh client ended it (124)' "$hung/lab-faults.log" 2>/dev/null || true)
if [[ $hung_kinds != "load xecho cross;load xecho local;rtt xpong cross;rtt xpong local;tput xsub cross;tput xsub local;" ]]; then
    fail "lib.sh dry, hung sessions: the clients that hung: $hung_kinds"
elif [[ -s $hung/ran-out ]] || ((hung_left)); then
    fail "lib.sh dry, hung sessions: a wait not ended at its bound: ran on to its end: $(tr '\n' ';' <"$hung/ran-out" 2>/dev/null); still running: $hung_left"
elif [[ $hung_faults != "$hung_n" ]]; then
    fail "lib.sh dry, hung sessions: $hung_faults lab faults for $hung_n hung sessions"
elif [[ $overran_kinds != "load xmix;rtt xping;tput xpub;" || $overran_faults != "$overran_n" ]]; then
    fail "lib.sh dry, foreground clients past their deadline ($overran_kinds): $overran_faults lab faults for $overran_n"
else
    pass "lib.sh dry: every hung SSH session of an answering client ($hung_n: pongs, echoes, subscribers, one piped into the results) ended at its wait's bound, whole, a lab fault each; every foreground client past its deadline ($overran_n) a lab fault"
fi

# LAB_TPUT_FLOWS unset: nothing multi-flow runs (no client on its ports, no
# snapshot, no row, nothing logged about it), and every other row as with it
off=$tmp/dry-off
mkdir -p "$off"
bash "$tmp/dry.sh" "$cloud" "$off" unset </dev/null 2>"$off/stderr" || fail "lib.sh dry run without LAB_TPUT_FLOWS exited $?"
if grep -E 'unbound variable|command not found|syntax error' "$off/stderr"; then
    fail "lib.sh dry run without LAB_TPUT_FLOWS: shell errors"
elif grep -E ':227[0-9][0-9] ' "$off/calls" || compgen -G "$off/*-counters-flows-*" >/dev/null ||
    grep -E ',tputflows,|:flows' "$off/results.csv" || grep -iE 'flows' "$off/log"; then
    fail "lib.sh dry run without LAB_TPUT_FLOWS ran the multi-flow sustained maximum"
elif ! grep -q ',tput,isolated:dpdk,' "$off/results.csv"; then
    fail "lib.sh dry run without LAB_TPUT_FLOWS: no results: $(tail -5 "$off/stderr" "$off/log" 2>/dev/null)"
elif ! diff -u <(grep -vE ',tputflows,|:flows' "$dry/results.csv" | sort) <(sort "$off/results.csv") >"$tmp/diff"; then
    fail "lib.sh dry run without LAB_TPUT_FLOWS: other rows changed"
    cat "$tmp/diff" >&2
else
    pass "lib.sh dry: no multi-flow sustained maximum with LAB_TPUT_FLOWS unset, the other rows unchanged"
fi

# lib.sh dry with LAB_NO_FLATOUT=1: nothing flat out (no throughput client at
# all: no 32 B throughput run, no sustained maximum, no several flows, nor
# their snapshots), the load sweep from its ladder alone with its burst at the
# ladder's median rate, and every other run and loss row as before; and the
# setting reaches VM a's environment
nf=$tmp/dry-noflat
mkdir -p "$nf"
sed -e 's/^reps=1 local_modes=kernel load_sizes=.288 1344 64. load_fractions=50 /reps=1 local_modes=kernel load_sizes="288 1344" load_fractions=50 no_flatout=1 load_ladder="288:1000,2000 1344:500" /' \
    -e 's/ load_ladder= mix=/ mix=/' "$tmp/dry.sh" >"$tmp/dry-noflat.sh"
bash "$tmp/dry-noflat.sh" "$cloud" "$nf" unset </dev/null 2>"$nf/stderr" || fail "lib.sh dry run with LAB_NO_FLATOUT=1 exited $?"
# each load client: "<label> <size> <rate>[ burst]" (its args after "load
# xping": two endpoints, rate, seconds, size, CPU, then a burst's 1000)
points=$(awk '{for (i = 1; i < NF; i++) if ($i == "load" && $(i + 1) == "xping") {
        print $(i - 1), $(i + 6), $(i + 4) (NF > i + 7 ? " burst" : "")}}' "$nf/calls" | sort | tr '\n' ';')
want_points="isolated:dpdk 1344 500;isolated:dpdk 1344 500 burst;isolated:dpdk 288 1000;isolated:dpdk 288 1000 burst;isolated:dpdk 288 2000;isolated:local-kernel 1344 500;isolated:local-kernel 1344 500 burst;isolated:local-kernel 288 1000;isolated:local-kernel 288 1000 burst;isolated:local-kernel 288 2000;"
if ! grep -q 'load_sizes=' "$tmp/dry-noflat.sh" || ! grep -q 'no_flatout=1' "$tmp/dry-noflat.sh"; then
    fail "lib.sh dry with LAB_NO_FLATOUT=1: the dry script was not set up"
elif grep -E 'unbound variable|command not found|syntax error' "$nf/stderr"; then
    fail "lib.sh dry with LAB_NO_FLATOUT=1: shell errors"
elif grep -q ' tput x' "$nf/calls" || grep -qE ',(tput|tputmax|tputflows),' "$nf/results.csv" ||
    compgen -G "$nf/*-counters-tput-*" >/dev/null || compgen -G "$nf/*-counters-max-*" >/dev/null; then
    fail "lib.sh dry with LAB_NO_FLATOUT=1: a flat-out run: $(grep ' tput x' "$nf/calls" | head -2)"
elif [[ $points != "$want_points" ]]; then
    fail "lib.sh dry with LAB_NO_FLATOUT=1: load points $points"
elif ! grep -q 'client isolated:dpdk rtt xping' "$nf/calls" || ! grep -q ',loss,isolated:dpdk:288:r2000,b,' "$nf/results.csv" ||
    ! grep -q ',loss,isolated:dpdk,a,' "$nf/results.csv"; then
    fail "lib.sh dry with LAB_NO_FLATOUT=1: rows missing: $(cut -d, -f2,3 "$nf/results.csv" | sort -u | tr '\n' ' ')"
else
    pass "lib.sh dry with LAB_NO_FLATOUT=1: no flat-out run, the ladder alone (burst at its median), the rest as before"
fi
# VM a's environment carries it (and the placement)
(
    set +e
    out=$tmp/dry-env known=$tmp/dry-env/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
    mkdir -p "$out"
    log() { :; }
    . "$cloud/lib.sh"
    on() { [[ $2 == *"/env" ]] && cat >"$out/env-written"; [[ $2 == *"/rc 2>/dev/null" ]] && echo 0; return 0; }
    sleep() { :; }
    reps=1 modes=(dpdk) rtt_messages=1 rtt_warmup=1 tput_seconds=5 no_flatout=1 placement=cross-az a_private=10.30.0.10 b_private=10.30.2.10
    results=$out/results.csv
    remote_state isolated >/dev/null 2>&1
) || true
if grep -qx "no_flatout='1'" "$tmp/dry-env/env-written" 2>/dev/null && grep -qx "placement='cross-az'" "$tmp/dry-env/env-written"; then
    pass "lib.sh: LAB_NO_FLATOUT and the placement reach VM a's environment"
else
    fail "lib.sh: VM a's environment: $(cat "$tmp/dry-env/env-written" 2>/dev/null | tr '\n' ' ')"
fi

# ATTR-LAB: the idle cross-host round trip stamps every round trip of 30 us or
# more (LOAD_SPIKE_NS=30000, the client's own default 100 us would leave every
# idle max of 72-79 us unstamped) and every stall of either client's loop of
# 10 us or more (LOOP_STALL_NS=10000) in the dry run above, with
# LAB_RTT_SPIKE_NS and LAB_RTT_STALL_NS unset; the localhost round trip keeps
# its 25 us; given, both knobs reach VM a's environment (remote.sh) and the
# clients' command lines. Each host's DPDK event log (a mode's
# @RUSTERON_DPDK_EVENT_LOG=<home>/bench/events.csv) is saved with the round
# trip as <node>-rings-<run>.log for spikes.py; a host without one leaves none
# (the round trip's ports, 20123-20124: a load point's echo is an xpong too)
xping=$(grep -E ' rtt xping 10\.[0-9.]+:20123 ' "$dry/calls" || true)
xpong=$(grep -E ' rtt xpong 10\.[0-9.]+:20123 ' "$dry/calls" || true)
local_ping=$(grep ' rtt xping 127\.' "$dry/calls" || true)
(
    set +e
    out=$tmp/dry-knobs known=$tmp/dry-knobs/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
    mkdir -p "$out"
    log() { :; }
    . "$cloud/lib.sh"
    on() { [[ $2 == *"/env" ]] && cat >"$out/env-written"; [[ $2 == *"/rc 2>/dev/null" ]] && echo 0; return 0; }
    sleep() { :; }
    reps=1 modes=(dpdk) rtt_messages=1 rtt_warmup=1 tput_seconds=5 a_private=10.30.0.10 b_private=10.30.0.11
    rtt_spike_ns=40000 rtt_stall_ns=12000
    results=$out/results.csv
    remote_state isolated >/dev/null 2>&1
) || true
# (VM b's DPDK event log holds a row, VM a has none: b's saved with the
# round trip, a's not at all)
python3 - "$tmp/dry.sh" "$tmp/dry-knobs.sh" <<'PY' || fail "lib.sh dry with the spike and stall knobs: the dry script was not set up"
import sys
s = open(sys.argv[1]).read()
for old, new in (("tput_seconds=5 rtt_messages=10 rtt_warmup=1 rtt_align=\n",
                  "tput_seconds=5 rtt_messages=10 rtt_warmup=1 rtt_align= rtt_spike_ns=40000 rtt_stall_ns=12000\n"),
                 ('    *"/env") cat >"$out/env-written" ;;\n',
                  '    *"cat bench/events.csv") [[ $host == "$B" ]] && echo "gap,1,1792032878000000000,30000,1,4242" ;;\n'
                  '    *"/env") cat >"$out/env-written" ;;\n')):
    assert s.count(old) == 1, old
    s = s.replace(old, new)
open(sys.argv[2], "w").write(s)
PY
mkdir -p "$tmp/dry-knobs-run"
bash "$tmp/dry-knobs.sh" "$cloud" "$tmp/dry-knobs-run" unset </dev/null 2>"$tmp/dry-knobs-run/stderr" || fail "lib.sh dry run with the spike and stall knobs exited $?"
knob_ping=$(grep -E ' rtt xping 10\.[0-9.]+:20123 ' "$tmp/dry-knobs-run/calls" || true)
knob_pong=$(grep -E ' rtt xpong 10\.[0-9.]+:20123 ' "$tmp/dry-knobs-run/calls" || true)
if [[ -z $xping || $(grep -vc ' LOAD_SPIKE_NS=30000 LOOP_STALL_NS=10000 ' <<<"$xping") != 0 ]]; then
    fail "lib.sh dry: the cross-host round trip's ping without LOAD_SPIKE_NS=30000 LOOP_STALL_NS=10000: $xping"
elif [[ -z $xpong || $(grep -vc ' LOOP_STALL_NS=10000 ' <<<"$xpong") != 0 ]]; then
    fail "lib.sh dry: the cross-host round trip's pong without LOOP_STALL_NS=10000: $xpong"
elif [[ -z $local_ping || $(grep -vc ' LOAD_SPIKE_NS=25000 ' <<<"$local_ping") != 0 ]]; then
    fail "lib.sh dry: the localhost round trip's ping lost its LOAD_SPIKE_NS=25000: $local_ping"
elif ! grep -qx "rtt_spike_ns='40000'" "$tmp/dry-knobs/env-written" 2>/dev/null || ! grep -qx "rtt_stall_ns='12000'" "$tmp/dry-knobs/env-written"; then
    fail "lib.sh: the spike and stall knobs do not reach VM a's environment: $(tr '\n' ' ' <"$tmp/dry-knobs/env-written" 2>/dev/null)"
elif ! grep -q 'cat bench/events.csv' "$tmp/dry-knobs.sh" || [[ $(cat "$tmp/dry-knobs-run/b-rings-isolated-dpdk-1.log" 2>/dev/null) != "gap,1,1792032878000000000,30000,1,4242" ]] ||
    compgen -G "$tmp/dry-knobs-run/a-rings-*" >/dev/null; then
    fail "lib.sh dry: the DPDK event log not saved with the round trip (b's) or an empty one kept (a's): $(ls "$tmp/dry-knobs-run" | grep rings)"
elif [[ -z $knob_ping || $(grep -vc ' LOAD_SPIKE_NS=40000 LOOP_STALL_NS=12000 ' <<<"$knob_ping") != 0 || -z $knob_pong ||
    $(grep -vc ' LOOP_STALL_NS=12000 ' <<<"$knob_pong") != 0 ]]; then
    fail "lib.sh dry with the spike and stall knobs: $knob_ping / $knob_pong"
else
    pass "lib.sh: the idle cross-host round trip stamps round trips of 30 us and loop stalls of 10 us or more (LAB_RTT_SPIKE_NS, LAB_RTT_STALL_NS), localhost's 25 us kept"
fi

# LAB_ESTIMATE=1 (aws/lab.sh's, below): estimate.py's arithmetic, against a
# plan worked by hand: two c7i.4xlarge across zones, one isolated state, two modes, two reps,
# 288 B at a ladder of 1000 and 2000/s (the burst at the median, 1000/s),
# 5 s a point, nothing flat out. Wall time: setup 600 s, the isolated state's
# 450 s, per mode and rep 12 s + the round trip's 85 s + 3 points of 5 + 7 s,
# teardown 180 s: 600 + 450 + 4 x 133 + 180 = 1762 s; instances 1762/3600 h x $0.714 x 2. Bytes each way per mode and rep: the round trip's 350,000
# messages of 32 + 60 B, and (5 + 1) s of each point at its rate of 288 + 60 B
# (an echo back for each): 32,200,000 + 6 x (1000 + 2000 + 1000) x 348 =
# 40,552,000; x 4 = 162,208,000 B each way. Cross-AZ: $0.01/GB out of one zone
# and $0.01/GB into the other, so $0.02 a GB crossing; cross-region $0.02/GB
# out of the sending region; one zone, nothing.
python3 - "$cloud" <<'PY' && pass "estimate.py: wall time, instance-hours, bytes each way and dollars by placement" || fail "estimate.py: arithmetic"
import sys
sys.path.insert(0, sys.argv[1])
import estimate
plan = dict(type_a="c7i.4xlarge", type_b="c7i.4xlarge", placement="cross-az", reps=2, modes=["kernel", "dpdk"],
            local_modes=[], states=["isolated"], load_sizes=[288], load_fractions=[10, 50], load_ladder={288: [1000, 2000]},
            load_seconds=5, mix=[], mix_seconds=10, tput_seconds=5, tput_flows=0, rtt_messages=300000, rtt_warmup=50000,
            no_flatout=True, max_hours=8)
e = estimate.estimate(**plan)
close = lambda a, b: abs(a - b) < 1e-9 * max(1, abs(b))
assert e["wall_s"] == 1762, e["wall_s"]
assert close(e["instance_hours"], 2 * 1762 / 3600), e["instance_hours"]
assert close(e["instance_usd"], 1762 / 3600 * 0.714 * 2), e["instance_usd"]
assert close(e["deadline_usd"], 8 * 0.714 * 2), e["deadline_usd"]
assert e["bytes_ab"] == 162208000 and e["bytes_ba"] == 162208000, (e["bytes_ab"], e["bytes_ba"])
assert close(e["transfer_usd"], 2 * 0.162208 * 0.02), e["transfer_usd"]
assert close(estimate.estimate(**dict(plan, placement="cross-region"))["transfer_usd"], 2 * 0.162208 * 0.02)
assert estimate.estimate(**dict(plan, placement="cluster"))["transfer_usd"] == 0
# a type with no price: the instances' dollars unknown, said so
u = estimate.estimate(**dict(plan, type_b="z9.huge"))
assert u["instance_usd"] is None and "z9.huge" in u["unknown"], u
# flat out: at most one flow's 8 Gbit/s for the publisher's tput_seconds + 6
f = estimate.estimate(**dict(plan, placement="cluster", no_flatout=False, load_ladder={288: [1000, 2000]}))
assert f["bytes_ab"] - e["bytes_ab"] == 4 * 2 * (5 + 6) * 1e9, f["bytes_ab"] - e["bytes_ab"]
# the four-core types (c7i.2xlarge: 4 cores x 2; c8g.xlarge, c9g.xlarge: 4
# cores) and the bare-metal ones, at us-east-1's on-demand $/h
for t, usd in (("c7i.2xlarge", 0.357), ("c8g.xlarge", 0.15952), ("c9g.xlarge", 0.17388),
               ("c8g.metal-24xl", 3.82848), ("c7i.metal-24xl", 4.284)):
    assert estimate.PRICES.get(t) == usd, (t, estimate.PRICES.get(t), usd)
g = estimate.estimate(**dict(plan, type_a="c8g.xlarge", type_b="c9g.xlarge"))
assert close(g["instance_usd"], 1762 / 3600 * (0.15952 + 0.17388)) and not g["unknown"], g
# a plan longer than LAB_MAX_HOURS: each instance powers off at its deadline,
# cutting the run short. 400 reps of the plan: 600 + 450 + 800 x 133 + 180
# = 107,630 s, 29.9 h; the hours to give it: a fifth more, rounded up, 36
assert not e["over_deadline"], e
o = estimate.estimate(**dict(plan, reps=400))
assert o["wall_s"] == 107630 and o["over_deadline"] and o["hours_needed"] == 36, o
assert not estimate.estimate(**dict(plan, reps=400, max_hours=30))["over_deadline"]
# spot: the lab's us-east-1 spot prices (each below its on-demand price,
# a one-time spot instance's default maximum, so the ceiling), the
# instances at them beside the on-demand figure; a type without one has none
for t, usd in (("c9g.2xlarge", 0.153), ("c8g.xlarge", 0.077), ("c8g.2xlarge", 0.147), ("c7i.2xlarge", 0.200)):
    assert estimate.SPOT_PRICES.get(t) == usd and usd < estimate.PRICES[t], (t, estimate.SPOT_PRICES.get(t), usd)
sp = estimate.estimate(**dict(plan, type_a="c9g.2xlarge", type_b="c9g.2xlarge", placement="cluster"))
assert close(sp["spot_usd"], 1762 / 3600 * 0.153 * 2) and close(sp["instance_usd"], 1762 / 3600 * 0.34776 * 2), sp
assert close(sp["spot_deadline_usd"], 8 * 0.153 * 2), sp
assert e["spot_usd"] is None and e["spot_deadline_usd"] is None, e
# c8g.2xlarge, the D-b pair: priced both ways (on demand twice c8g.xlarge's)
c8 = estimate.estimate(**dict(plan, type_a="c8g.2xlarge", type_b="c8g.2xlarge", placement="cluster"))
assert close(c8["instance_usd"], 1762 / 3600 * 0.31904 * 2) and close(c8["spot_usd"], 1762 / 3600 * 0.147 * 2), c8
# a -nodiag state costs what its state does
nd = estimate.estimate(**dict(plan, states=["isolated-nodiag"]))
assert nd["wall_s"] == estimate.estimate(**dict(plan, states=["isolated"]))["wall_s"], nd
PY

# lib.sh dry, jumbo frames and distance: a kernel mode and the same with
# JUMBO=1, sizes 288 and 8192 B from a ladder. Before a JUMBO mode's drivers
# start, both hosts check the path at the jumbo MTU (vm.sh pmtu, to every data
# address of the other host; no such check otherwise); a size over a mode's
# largest message is skipped there, said so (8192 B in kernel, 1376 B at its
# 1408); each run records its effective MTU on both hosts (mtu rows); and
# each state starts with the network's own round trip (vm.sh base-rtt: a
# netrtt row, its max in microseconds on both hosts for the lost-round-trip
# timeout). With the path check failing the run stops before that mode's
# drivers start.
cat >"$tmp/dry-jumbo.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
on() {
    local host=$1 cmd
    shift
    cmd=$*
    printf '%s %s\n' "$host" "$cmd" >>"$out/calls"
    case $cmd in
    *"vm.sh nics")
        if [[ $host == "$A" ]]; then
            printf 'sender ens6 ens6 0a 10.30.1.4 24 0000:00:06.0\nreceiver ens7 ens7 0b 10.30.1.5 24 0000:00:07.0\n'
        else
            printf 'sender ens6 ens6 0c 10.30.3.6 24 0000:00:06.0\nreceiver ens7 ens7 0d 10.30.3.7 24 0000:00:07.0\n'
        fi
        ;;
    *"vm.sh layout") echo "0 2 1 3 4" ;;
    *"vm.sh loss") echo "naks-sent=0;short-sends=0" ;;
    *"vm.sh pmtu "*) [[ -z ${FAIL_PMTU:-} ]] || { echo "pmtu 10.30.1.4 -> 10.30.3.7: 9001 bytes do not pass"; return 1; } ;;
    *"vm.sh mtu") echo "9001 8960" ;;
    *"vm.sh base-rtt "*) echo "45000 61000 212000 12000 198/200" ;;
    *" tput xsub "*) echo "tput,x,udp,1000,1,2,3,0" ;;
    esac
    return 0
}
reps=1 local_modes= load_sizes='288 8192' load_fractions=50 load_seconds=1 load_ladder='288:1000 8192:500' mix= mix_seconds=1
tput_seconds=5 rtt_messages=10 rtt_warmup=1 rtt_align= placement=cross-az
modes=(kernel kernel@JUMBO=1)
bench_state isolated
SH
dj=$tmp/dry-jumbo
mkdir -p "$dj"
bash "$tmp/dry-jumbo.sh" "$cloud" "$dj" </dev/null 2>"$dj/stderr" || fail "lib.sh dry run with jumbo frames exited $?"
# per host, its path checks and driver starts in order
order=$(for h in 1 2; do grep -E "^192.0.2.$h .*vm.sh (pmtu|driver) " "$dj/calls" | awk -v h=$h '{print h, $3, $4}'; done | tr '\n' ';')
sizes=$(awk '{for (i = 1; i < NF; i++) if ($i == "load" && $(i + 1) == "xping") print $(i - 1), $(i + 6)}' "$dj/calls" | sort -u | tr '\n' ';')
if grep -E 'unbound variable|command not found|syntax error' "$dj/stderr"; then
    fail "lib.sh dry with jumbo frames: shell errors"
elif [[ $order != "1 driver kernel;1 pmtu 10.30.3.6;1 driver kernel@JUMBO=1;2 driver kernel;2 pmtu 10.30.1.4;2 driver kernel@JUMBO=1;" &&
    $order != "1 pmtu 10.30.3.6;1 driver kernel@JUMBO=1;1 driver kernel;2 pmtu 10.30.1.4;2 driver kernel@JUMBO=1;2 driver kernel;" ]]; then
    fail "lib.sh dry with jumbo frames: the path check and driver starts: $order"
elif ! grep -q 'vm.sh pmtu 10.30.3.6 10.30.3.7$' "$dj/calls" || ! grep -q 'vm.sh pmtu 10.30.1.4 10.30.1.5$' "$dj/calls"; then
    fail "lib.sh dry with jumbo frames: the path check's peers: $(grep 'vm.sh pmtu' "$dj/calls")"
elif [[ $sizes != "isolated:kernel 288;isolated:kernel@JUMBO=1 288;isolated:kernel@JUMBO=1 8192;" ]]; then
    fail "lib.sh dry with jumbo frames: sizes run: $sizes"
elif ! grep -q 'isolated:kernel 8192 B: over its largest message (1376 B at MTU 1408): skipped' "$dj/log"; then
    fail "lib.sh dry with jumbo frames: the skip was not said: $(cat "$dj/log")"
elif [[ $(grep -c ',mtu,' "$dj/results.csv") != 4 ]] || ! grep -qx '1,mtu,isolated:kernel@JUMBO=1,b,9001,8960' "$dj/results.csv"; then
    fail "lib.sh dry with jumbo frames: mtu rows: $(grep ',mtu,' "$dj/results.csv")"
elif ! grep -qx '0,netrtt,isolated,45000,61000,212000,12000,198/200' "$dj/results.csv" ||
    ! grep -q 'vm.sh base-rtt 10.30.3.7$' "$dj/calls" ||
    [[ $(grep -c 'echo 212 >bench/base_rtt_us' "$dj/calls") != 2 ]]; then
    fail "lib.sh dry with jumbo frames: the network's round trip: $(grep -E 'netrtt' "$dj/results.csv"; grep -E 'base' "$dj/calls")"
else
    pass "lib.sh dry: the path checked at the jumbo MTU before a JUMBO mode, sizes over a mode's MTU skipped (said so), each run's MTU and the network's round trip recorded"
fi
djf=$tmp/dry-jumbo-fail
mkdir -p "$djf"
FAIL_PMTU=1 bash "$tmp/dry-jumbo.sh" "$cloud" "$djf" </dev/null 2>"$djf/stderr" && fail "lib.sh dry: a failed path check did not stop the run"
if grep -q 'ERROR: .*kernel@JUMBO=1.*path MTU' "$djf/log" && ! grep -q 'vm.sh driver kernel@JUMBO=1' "$djf/calls"; then
    pass "lib.sh dry: a path that drops jumbo frames stops the run before the mode's drivers start"
else
    fail "lib.sh dry: a failed path check: $(tail -2 "$djf/log")"
fi

# analyse.py, where a state has the network's own round trip (netrtt): each
# mode's round trip above it, p50 to max less the network's minimum, beside
# the network's min/avg/max; without netrtt rows, no such table
python3 - "$cloud" "$tmp/netrtt" <<'PY' && pass "analyse.py: the round trip above the network's own" || fail "analyse.py: the round trip above the network's own"
import os, subprocess, sys
d = sys.argv[2]
os.makedirs(d)
rows = ["0,netrtt,isolated,1000000,1100000,1900000,50000,200/200\n"]
for rep, base in ((1, 1500000), (2, 1700000)):
    rows.append(f"{rep},rtt,isolated:kernel,udp,{base},{base + 100000},{base + 200000},{base + 300000},{base + 400000},{base + 900000},{base},1000,0\n")
open(os.path.join(d, "results.csv"), "w").writelines(rows)
out = subprocess.run([sys.executable, os.path.join(sys.argv[1], "analyse.py"), os.path.join(d, "results.csv")],
                     capture_output=True, text=True, check=True).stdout
lines = out.split("## UDP round trip above the network's own (µs)", 1)
assert len(lines) == 2, out
table = [l for l in lines[1].splitlines() if l.startswith("| isolated")]
want = "| isolated, kernel UDP | 1000.00/1100.00/1900.00 | 2 | 600.00 (500.00–700.00) | 800.00 (700.00–900.00) | 1000.00 (900.00–1100.00) | 1500.00 (1400.00–1600.00) |"
got = " ".join(table[0].split()) if table else ""
assert got == " ".join(want.split()), (got, want)
open(os.path.join(d, "results.csv"), "w").writelines(rows[1:])
out = subprocess.run([sys.executable, os.path.join(sys.argv[1], "analyse.py"), os.path.join(d, "results.csv")],
                     capture_output=True, text=True, check=True).stdout
assert "above the network" not in out, out
PY

# lib.sh dry, the media driver's start-up host checks per host and mode: an
# external driver's (vm.sh hostchecks on each host once its driver started:
# a's two lines, b's none) and under INVOKER each client's embedded driver's,
# from the round trip's ping (a) and pong (b) output, not the drivers' logs;
# each line a results row, <rep>,hostcheck,<label>,<node>,<severity>,<what
# it said> (quoted as CSV where it holds a comma), and a host that said
# nothing one row of severity none, every rep; a host whose checks could not
# be read (vm.sh hostchecks failed: b's second, an SSH session lost) one row
# of severity unknown saying so, never none, and a warning in the log
cat >"$tmp/dry-checks.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
on() {
    local host=$1 cmd
    shift
    cmd=$*
    printf '%s %s\n' "$host" "$cmd" >>"$out/calls"
    case $cmd in
    *"vm.sh nics")
        if [[ $host == "$A" ]]; then
            printf 'sender ens6 ens6 0a 10.30.1.4 24 0000:00:06.0\nreceiver ens7 ens7 0b 10.30.1.5 24 0000:00:07.0\n'
        else
            printf 'sender ens6 ens6 0c 10.30.1.6 24 0000:00:06.0\nreceiver ens7 ens7 0d 10.30.1.7 24 0000:00:07.0\n'
        fi
        ;;
    *"vm.sh layout") echo "0 2 1 3 4" ;;
    *"vm.sh loss") echo "naks-sent=0" ;;
    *"vm.sh hostchecks")
        if [[ $host == "$A" ]]; then
            echo 'ergon-md-dpdk warning: T3 transparent huge pages always, against madvise (fix: echo madvise), max 1,200 us'
            echo 'ergon-md-dpdk info: M4 kernel busy polling off (Graviton)'
        else
            # b's second read fails as on() fails a lost session
            n=$(($(cat "$out/b-hostchecks" 2>/dev/null || echo 0) + 1))
            echo "$n" >"$out/b-hostchecks"
            ((n != 2)) || return 255
        fi
        ;;
    *" rtt xping "* | *" rtt xpong "*)
        if [[ $cmd == *INVOKER* ]]; then
            echo "harness driver: embedded threading=INVOKER transport=dpdk-ena cpu=2 dir=/dev/shm/aeron" >&2
            echo "ergon-md-dpdk warning: T1 CPU 2 not isolated from the scheduler, max 40 us" >&2
        fi
        [[ $cmd == *" rtt xping "* ]] && echo "rtt,x,udp,1,2,3,4,5,6,7,100,0"
        ;;
    esac
    return 0
}
reps=2 local_modes= load_sizes= mix= tput_seconds=5 rtt_messages=10 rtt_warmup=1 rtt_align= placement=cluster no_flatout=1
modes=(kernel dpdk@AERON_THREADING_MODE=INVOKER)
bench_state isolated
SH
dk=$tmp/dry-checks
mkdir -p "$dk"
bash "$tmp/dry-checks.sh" "$cloud" "$dk" </dev/null 2>"$dk/stderr" || fail "lib.sh dry run with host checks exited $?"
got=$(python3 -c '
import csv, sys
rows = sorted(tuple(r) for r in csv.reader(open(sys.argv[1])) if len(r) > 1 and r[1] == "hostcheck")
print("\n".join("|".join(r) for r in rows))' "$dk/results.csv" 2>&1)
want="1|hostcheck|isolated:dpdk@AERON_THREADING_MODE=INVOKER|a|warning|T1 CPU 2 not isolated from the scheduler, max 40 us
1|hostcheck|isolated:dpdk@AERON_THREADING_MODE=INVOKER|b|warning|T1 CPU 2 not isolated from the scheduler, max 40 us
1|hostcheck|isolated:kernel|a|info|M4 kernel busy polling off (Graviton)
1|hostcheck|isolated:kernel|a|warning|T3 transparent huge pages always, against madvise (fix: echo madvise), max 1,200 us
1|hostcheck|isolated:kernel|b|none|
2|hostcheck|isolated:dpdk@AERON_THREADING_MODE=INVOKER|a|warning|T1 CPU 2 not isolated from the scheduler, max 40 us
2|hostcheck|isolated:dpdk@AERON_THREADING_MODE=INVOKER|b|warning|T1 CPU 2 not isolated from the scheduler, max 40 us
2|hostcheck|isolated:kernel|a|info|M4 kernel busy polling off (Graviton)
2|hostcheck|isolated:kernel|a|warning|T3 transparent huge pages always, against madvise (fix: echo madvise), max 1,200 us
2|hostcheck|isolated:kernel|b|unknown|not read: vm.sh hostchecks failed (255)"
if grep -E 'unbound variable|command not found|syntax error' "$dk/stderr"; then
    fail "lib.sh dry with host checks: shell errors"
elif [[ $got != "$want" ]]; then
    fail "lib.sh dry: the host-check rows"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
elif ! grep -q '^1,hostcheck,isolated:kernel,a,warning,"T3 transparent huge pages always, against madvise (fix: echo madvise), max 1,200 us"$' "$dk/results.csv"; then
    fail "lib.sh dry: a host check holding a comma is not quoted: $(grep T3 "$dk/results.csv")"
elif ! grep -q "^WARNING: isolated:kernel rep 2: b's media driver host checks not read (vm.sh hostchecks failed: 255)" "$dk/log"; then
    fail "lib.sh dry: an unread host check not warned of: $(cat "$dk/log" 2>/dev/null)"
else
    pass "lib.sh dry: each host's media driver start-up checks per mode in results.csv (an external driver's log, INVOKER's clients' output), none said so, an unread host's unknown, never none"
fi

# analyse.py prints those rows per mode: each host's lines, their severity,
# in how many of the mode's reps (a host that said nothing: none), warnings
# before information; and no such table without them
python3 - "$cloud" "$tmp/hostchecks" <<'PY' && pass "analyse.py: the media driver's host checks per mode and host" || fail "analyse.py: the media driver's host checks"
import os, subprocess, sys
sys.path.insert(0, sys.argv[1])
import analyse
d = sys.argv[2]
os.makedirs(d)
rtt = ["1,rtt,isolated:kernel,udp,1000,1100,1200,1300,1400,1500,1100,1000,0\n",
       "2,rtt,isolated:kernel,udp,1000,1100,1200,1300,1400,1500,1100,1000,0\n",
       "1,rtt,isolated:dpdk,udp,900,1000,1100,1200,1300,1400,1000,1000,0\n"]
checks = ['1,hostcheck,isolated:kernel,a,warning,"T3 transparent huge pages always, max 1,200 us"\n',
          "1,hostcheck,isolated:kernel,a,info,M4 kernel busy polling off\n",
          "1,hostcheck,isolated:kernel,b,none,\n",
          '2,hostcheck,isolated:kernel,a,warning,"T3 transparent huge pages always, max 1,200 us"\n',
          "2,hostcheck,isolated:kernel,b,unknown,not read: vm.sh hostchecks failed (255)\n",
          "1,hostcheck,isolated:dpdk,a,none,\n",
          "1,hostcheck,isolated:dpdk,b,warning,T1 CPU 2 not isolated | a pipe\n"]
open(os.path.join(d, "results.csv"), "w").writelines(rtt + checks)
run = lambda: subprocess.run([sys.executable, os.path.join(sys.argv[1], "analyse.py"), os.path.join(d, "results.csv")],
                             capture_output=True, text=True, check=True).stdout
out = run()
head = "## Media driver host checks at start-up (per mode and host)"
assert head in out, out
table = [" ".join(l.split()) for l in out.split(head, 1)[1].split("\n## ", 1)[0].splitlines()
         if l.startswith("| ") and not set(l) <= set("|- ")]
k, p = analyse.name("isolated:kernel"), analyse.name("isolated:dpdk")
want = ["| state, transport | host | severity | finding | reps |",
        f"| {k} | a | warning | T3 transparent huge pages always, max 1,200 us | 2 of 2 |",
        f"| {k} | a | info | M4 kernel busy polling off | 1 of 2 |",
        f"| {k} | b | none | | 1 of 2 |",
        f"| {k} | b | unknown | not read: vm.sh hostchecks failed (255) | 1 of 2 |",
        f"| {p} | a | none | | 1 of 1 |",
        f"| {p} | b | warning | T1 CPU 2 not isolated \\| a pipe | 1 of 1 |"]
assert table == [" ".join(w.split()) for w in want], "\n".join(table)
open(os.path.join(d, "results.csv"), "w").writelines(rtt)
assert head not in run()
PY

# lib.sh's per-host records: each host's data subnet and, cross-AZ or
# cross-region, the other host's (vm.sh routes to it through its gateway),
# one subnet for both otherwise (no peer); and the network's own round trip
# in both hosts' environment records
subnets=$(
    set +e
    exec 2>&1
    out=$tmp/subnets known=$tmp/subnets/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
    mkdir -p "$out"
    log() { :; }
    # shellcheck source=../lib.sh
    . "$cloud/lib.sh"
    data_subnet=10.30.1.0/24
    echo "one subnet: [$(host_subnets "$A")] [$(host_subnets "$B")]"
    data_subnet_a=10.30.1.0/24 data_subnet_b=10.30.3.0/24
    echo "two: [$(host_subnets "$A")] [$(host_subnets "$B")]"
    on() {
        case $2 in
        *"vm.sh nics") [[ $1 == "$B" ]] && echo "receiver ens7 ens7 0d 10.30.3.7 24 0000:00:07.0" ;;
        *"vm.sh base-rtt 10.30.3.7") [[ $1 == "$A" ]] && echo "45000 61000 212000 12000 198/200" ;;
        esac
        return 0
    }
    record_base_rtt
    cat "$out/env-$A.txt" "$out/env-$B.txt"
) || true
want="one subnet: [10.30.1.0/24] [10.30.1.0/24]
two: [10.30.1.0/24 10.30.3.0/24] [10.30.3.0/24 10.30.1.0/24]
base-rtt a's sender -> b's receiver 10.30.3.7 (ping, ns: min avg max mdev, replies/sent) 45000 61000 212000 12000 198/200
base-rtt a's sender -> b's receiver 10.30.3.7 (ping, ns: min avg max mdev, replies/sent) 45000 61000 212000 12000 198/200"
if [[ $subnets == "$want" ]]; then
    pass "lib.sh: each host's data subnet and its peer's when off it; the network's round trip in both hosts' records"
else
    fail "lib.sh: per-host subnets and records"
    diff -u <(echo "$want") <(echo "$subnets") >&2 || true
fi
# lib.sh, a run that is over (aws/lab.sh's abandon file: a spot instance
# reclaimed): on() tries nothing more and fails at once as a dropped
# connection does (255), in every subshell too, so no step waits out its SSH
# retries (3 minutes a call); before that, as ever. And the market (aws/lab.sh
# LAB_SPOT: spot or on-demand) in both hosts' environment records; none
# where the cloud gives none (Azure)
ended=$(
    set +e
    exec 2>&1
    out=$tmp/ended known=$tmp/ended/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
    mkdir -p "$out"
    log() { :; }
    # shellcheck source=../lib.sh
    . "$cloud/lib.sh"
    ssh() { echo ssh >>"$out/ssh"; return 255; }
    sleep() { :; }
    lab_abandon=$out/abandon
    on "$A" true
    echo "before: $? $(grep -c ssh "$out/ssh")"
    : >"$out/abandon"
    : >"$out/ssh"
    on "$A" true
    echo "after: $? $(grep -c ssh "$out/ssh")"
    echo "subshell: $(on "$B" true; echo $?) $(grep -c ssh "$out/ssh")"
    market=spot record_market
    market=on-demand record_market
    record_market
    cat "$out/env-$A.txt" "$out/env-$B.txt"
) || true
want="before: 255 18
after: 255 0
subshell: 255 0
market spot (AWS spot instances: AWS may reclaim them mid-run)
market on-demand
market spot (AWS spot instances: AWS may reclaim them mid-run)
market on-demand"
if [[ $ended == "$want" ]]; then
    pass "lib.sh: no SSH tried once the run is abandoned (255 at once); the market in both hosts' records"
else
    fail "lib.sh: an abandoned run's SSH, the market record"
    diff -u <(echo "$want") <(echo "$ended") >&2 || true
fi

# irqs.sh against a fake /proc: four CPUs, housekeeping 0. The data NICs'
# ENA queues one a CPU (as bringing an interface up leaves them) and one
# management IRQ, a queue named as an older kernel names it (no address in its
# chip's name), the management NIC's queue, a near miss of the data NIC's
# address and name both, a managed NVMe queue (its affinity refuses a write:
# a link into nothing stands in), one IRQ with no effective_affinity_list (an
# older kernel) and one with it empty (not started), and an IRQ with no
# handler (absent from /proc/interrupts). A movable IRQ's effective CPUs
# follow its affinity (a link to it).
proc=$tmp/proc
mkdir -p "$proc/irq"
cat >"$proc/interrupts" <<'EOF'
            CPU0       CPU1       CPU2       CPU3
   0:         40          0          0          0  IR-IO-APIC    2-edge      timer
  24:         10          0          2          0  PCI-MSIX-0000:28:00.0    0-edge      ena-mgmnt@pci:0000:28:00.0
  25:        100          3          0          0  PCI-MSIX-0000:28:00.0    1-edge      enp40s0-Tx-Rx-0
  26:          0          0         50          0  PCI-MSIX-0000:28:00.0    2-edge      enp40s0-Tx-Rx-1
  27:          0          7          0          0  PCI-MSIX-0000:29:00.0    1-edge      enp41s0-Tx-Rx-0
  28:          0          4          0          0  PCI-MSI 1572864-edge      enp41s0-Tx-Rx-1
  29:          0          0          0          9  PCI-MSIX-0000:27:00.0    1-edge      enp39s0-Tx-Rx-0
  30:          0          0          5          0  PCI-MSIX-0000:28:00.1    1-edge      enp40s0np1-Tx-Rx-0
  40:          0          0          0         11  PCI-MSIX-0000:00:04.0    3-edge      nvme0q3
 NMI:          1          2          3          4   Non-maskable interrupts
 LOC:        100        200        300        400   Local timer interrupts
EOF
# <irq> <smp_affinity_list> <effective_affinity_list: link, none, empty or the CPUs>
while read -r irq smp eff; do
    mkdir -p "$proc/irq/$irq"
    echo "$smp" >"$proc/irq/$irq/smp_affinity_list"
    case $eff in
    link) ln -s smp_affinity_list "$proc/irq/$irq/effective_affinity_list" ;;
    none) ;;
    empty) : >"$proc/irq/$irq/effective_affinity_list" ;;
    *) echo "$eff" >"$proc/irq/$irq/effective_affinity_list" ;;
    esac
done <<'EOF'
0 0-3 link
5 0-3 empty
24 0 link
25 1 link
26 2 link
27 1 link
28 1 none
29 3 empty
30 2 link
40 3 3
EOF
rm "$proc/irq/40/smp_affinity_list"
ln -s "$tmp/nothing/here" "$proc/irq/40/smp_affinity_list"
echo f >"$proc/irq/default_smp_affinity"
# A DPDK host's: the data NICs on igb_uio, whose one IRQ each (the ENA
# PMD's admin queue) names only the address, beside the management NIC's
# queue; and a MANA device, whose ports (one a data NIC) share its address.
proc_dpdk=$tmp/proc-dpdk
mkdir -p "$proc_dpdk"
cat >"$proc_dpdk/interrupts" <<'EOF'
            CPU0       CPU1       CPU2       CPU3
  29:          0          0          0          9  PCI-MSIX-0000:27:00.0    1-edge      enp39s0-Tx-Rx-0
  60:         12          4          0          0  PCI-MSIX-0000:28:00.0    0-edge      igb_uio
  61:          5          0          0          0  PCI-MSIX-0000:29:00.0    0-edge      igb_uio
  70:          0          0          8          0  PCI-MSIX-7870:00:00.0    1-edge      mana_q0@pci:7870:00:00.0
EOF
# one argument a NIC: <interface>,<VF>,<PCI address> (DPDK: the address alone)
data_nics="enp40s0,enp40s0,0000:28:00.0 enp41s0,enp41s0,0000:29:00.0"
irqs=$(
    set +e
    exec 2>&1
    LAB_PROC_ROOT=$proc
    # shellcheck source=../irqs.sh
    . "$cloud/irqs.sh" || exit 1
    sudo() { "$@"; }
    systemctl() { echo "systemctl $*" >>"$tmp/irqs-calls"; }
    echo "check: $(irqs_check 0)"
    # shellcheck disable=SC2086
    echo "nic: $(irqs_nic_off 0 $data_nics) $(irqs_nic_off 2 $data_nics) $(irqs_nic_off 0 '' ',,')" \
        "[$(irqs_nic_off 0 $data_nics enp42s0,enp42s0,0000:2a:00.0)]"
    LAB_PROC_ROOT=$proc_dpdk
    echo "dpdk: $(irqs_nic_off 0 0000:28:00.0 0000:29:00.0) [$(irqs_nic_off 0 0000:28:00.0 0000:2a:00.0)]" \
        "$(irqs_nic_off 0 eth1,enP30832s1,7870:00:00.0 eth2,enP30832s1d1,7870:00:00.0)"
    LAB_PROC_ROOT=$proc
    irqs_pin 0 "after the test" 2>&1 >/dev/null | sed 's/^/pin: /'
    for irq in 0 5 24 25 26 27 28 29 30; do printf '%s' "$(cat "$proc/irq/$irq/smp_affinity_list")"; done
    echo " default $(cat "$proc/irq/default_smp_affinity")"
) || true
# Before pinning, every IRQ off CPU 0 listed (28's CPUs read from its
# affinity, it having no effective list, and 29's, its list empty); the data
# NICs' interrupts off CPU 0 (IRQs 24 to 28: 2 + 3 + 50 + 7 + 4), off CPU 2 as
# if it were housekeeping (10 + 103 + 0 + 7 + 4), none with no NIC given, and
# unknown with a third NIC that has no IRQ; on the DPDK host, igb_uio's off
# CPU 0 (4), unknown with a NIC whose igb_uio IRQ is not there, and the MANA
# queue once for both ports (8); after pinning, the managed queue alone
# elsewhere, every other affinity CPU 0 (5's too, with no handler) and new
# IRQs' default mask 1
want="check: 1 on CPU 0 alone, 8 elsewhere (0:timer@0-3 25:enp40s0-Tx-Rx-0@1 26:enp40s0-Tx-Rx-1@2 27:enp41s0-Tx-Rx-0@1 28:enp41s0-Tx-Rx-1@1 29:enp39s0-Tx-Rx-0@3 30:enp40s0np1-Tx-Rx-0@2 40:nvme0q3@3), default for new ones f
nic: 66 124 0 []
dpdk: 4 [] 8
pin: irqs after the test: 8 on CPU 0 alone, 1 elsewhere (40:nvme0q3@3), default for new ones 1
000000000 default 1"
if [[ $irqs != "$want" ]]; then
    fail "irqs.sh against a fake /proc"
    diff -u <(echo "$want") <(echo "$irqs") >&2 || true
elif [[ $(cat "$tmp/irqs-calls" 2>/dev/null) != "systemctl stop irqbalance" ]]; then
    fail "irqs.sh: irqbalance was not stopped: $(cat "$tmp/irqs-calls" 2>/dev/null)"
else
    pass "irqs.sh against a fake /proc: pinned, checked, data NIC interrupts off housekeeping counted"
fi
# irqs.sh, a mode's @IRQ= (vm.sh driver): the data NICs' queue IRQs, those
# named after their interfaces (ENA's <interface>-Tx-Rx-<n>: 25 to 28; not
# 30, a near miss, nor 24, the management IRQ, which stays on housekeeping),
# pinned to the CPU the mode chose and checked there by their affinity,
# failing (1) while one is elsewhere (28: its write refused, a link into
# nothing stands in) and when an interface has none (a driver that names them
# otherwise); an effective CPU still elsewhere (27's kept on 1) is reported
# but passes, as a VM moves an IRQ only at its next interrupt (AWS round B's
# first run stopped on exactly that). And
# the check on the run, isolated-nic-irqs, with that CPU (-q): the queue
# IRQs' interrupts counted off it, the NICs' other IRQs' off housekeeping's
# (-q 2: 2 + 103 + 0 + 7 + 4); -q naming housekeeping counts as without it;
# under DPDK no queue IRQ is the kernel's, so as without it
rm "$proc/irq/27/effective_affinity_list"
echo 1 >"$proc/irq/27/effective_affinity_list"
rm "$proc/irq/28/smp_affinity_list"
ln -s "$tmp/nothing/here" "$proc/irq/28/smp_affinity_list"
queues=$(
    set +e
    exec 2>&1
    LAB_PROC_ROOT=$proc
    # shellcheck source=../irqs.sh
    . "$cloud/irqs.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    sudo() { "$@"; }
    echo "queues: $(irqs_nic_queues enp40s0 enp41s0 | tr '\n' ';')"
    irqs_pin_queues 3 "after the test" enp40s0 enp41s0
    echo "pin: $?"
    for irq in 24 25 26 27 28 30; do cat "$proc/irq/$irq/smp_affinity_list" 2>/dev/null; done | paste -sd' ' -
    echo "check:$(rc irqs_queues_check 3 enp40s0)$(rc irqs_queues_check 3 enp40s0 enp41s0)$(rc irqs_queues_check 2 enp40s0)"
    rm "$proc/irq/28/smp_affinity_list"
    echo 3 >"$proc/irq/28/smp_affinity_list"
    echo "lag: $(irqs_queues_check 3 enp40s0 enp41s0) rc $?"
    irqs_pin_queues 3 "for nothing" enp42s0
    echo "none: $?"
    # shellcheck disable=SC2086
    echo "nic: $(irqs_nic_off -q 2 0 $data_nics) $(irqs_nic_off -q 0 0 $data_nics) $(irqs_nic_off 0 $data_nics) [$(irqs_nic_off -q 2 0 $data_nics enp42s0,enp42s0,0000:2a:00.0)]"
    LAB_PROC_ROOT=$proc_dpdk
    echo "dpdk: $(irqs_nic_off -q 2 0 0000:28:00.0 0000:29:00.0)"
) || true
want="queues: 25 enp40s0-Tx-Rx-0;26 enp40s0-Tx-Rx-1;27 enp41s0-Tx-Rx-0;28 enp41s0-Tx-Rx-1;
irqs after the test: 3 of 4 data NIC queue IRQs on CPU 3 alone, elsewhere: 28:enp41s0-Tx-Rx-1@?; effective elsewhere until their next interrupt (a VM moves an IRQ then): 27:enp41s0-Tx-Rx-0@1
pin: 1
0 3 3 3 0
check: 0 1 1
lag: 4 of 4 data NIC queue IRQs on CPU 3 alone; effective elsewhere until their next interrupt (a VM moves an IRQ then): 27:enp41s0-Tx-Rx-0@1 rc 0
irqs for nothing: no queue IRQ named after enp42s0 (ENA's <interface>-Tx-Rx-<n>)
none: 1
nic: 116 66 66 []
dpdk: 4"
if [[ $queues != "$want" ]]; then
    fail "irqs.sh: a mode's data NIC queue IRQs on its CPU, and the check with it"
    diff -u <(echo "$want") <(echo "$queues") >&2 || true
else
    pass "irqs.sh: @IRQ's queue IRQs pinned and checked (fail closed), isolated-nic-irqs counting them off the chosen CPU"
fi

# a comma in a mode would split every results.csv row for it: both lab.sh
# refuse it before any cloud call (fake aws and az first on PATH log calls;
# a home of its own with an SSH key, which azure/lab.sh looks for first, and
# the scripts' snapshots in $tmp)
mkdir -p "$tmp/fakebin" "$tmp/commahome/.ssh" "$tmp/commatmp"
echo 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA lab-test' >"$tmp/commahome/.ssh/id_ed25519.pub"
for cli in aws az; do printf '#!/bin/sh\necho "$0 $*" >>"%s/cloud-calls"\nexit 1\n' "$tmp" >"$tmp/fakebin/$cli"; chmod +x "$tmp/fakebin/$cli"; done
# shellcheck disable=SC2016
printf '#!/bin/sh\nd=$(/usr/bin/mktemp "$@") || exit 1\necho "$d" >>"%s/mktemp.log"\necho "$d"\n' "$tmp" >"$tmp/fakebin/mktemp"
chmod +x "$tmp/fakebin/mktemp"
for lab in aws/lab.sh azure/lab.sh; do
    if PATH="$tmp/fakebin:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/commarepo" \
        LAB_MODES="kernel-busy@COALESCE=20,64" bash "$cloud/$lab" >"$tmp/comma.out" 2>&1; then
        fail "$lab accepted a comma in a mode"
    elif grep -q "comma" "$tmp/comma.out" && [[ ! -s $tmp/cloud-calls ]]; then
        pass "$lab refuses a comma in a mode before any cloud call"
    else
        fail "$lab and a comma in a mode: $(tail -2 "$tmp/comma.out"; cat "$tmp/cloud-calls" 2>/dev/null)"
    fi
    rm -f "$tmp/cloud-calls"
done

# both lab.sh run from a snapshot of these scripts made in $TMPDIR, here the
# test's own (macOS's mktemp -d, given no template, ignores TMPDIR: every
# run's megabyte copy stayed in the user's temporary directory, thousands of
# them from these tests until the disk filled), and remove it as they exit:
# the trap's last step, after any teardown, by its own path, checked (a
# directory under $TMPDIR named as mktemp made it, holding the script). A
# mktemp first on PATH logs each directory it makes for them.
snaps=$(grep -c "^$tmp/commatmp/*ergon-lab\.[A-Za-z0-9]\{6\}\$" "$tmp/mktemp.log" 2>/dev/null || true)
left=$(find "$tmp/commatmp" -mindepth 1 -maxdepth 1 -name 'ergon-lab.*' | wc -l | tr -d ' ') || true
if [[ $snaps -lt 2 ]]; then
    fail "lab.sh snapshots in \$TMPDIR: $snaps made there ($(head -3 "$tmp/mktemp.log" 2>/dev/null))"
elif [[ $left != 0 ]]; then
    fail "lab.sh snapshots left behind in \$TMPDIR: $left"
else
    pass "both lab.sh snapshot themselves in \$TMPDIR and remove the snapshot as they exit, a run refused before any cloud call too"
fi

# azure/lab.sh checks every mode as aws/lab.sh does (modes.sh), before any
# az call: AWS-only settings (jumbo frames) and INVOKER on one host refused
for refused in "LAB_MODES=kernel@JUMBO=1|AWS only" "LAB_MODES=kernel@IRQ=receiver|AWS only" "LAB_LOCAL_MODES=kernel@AERON_THREADING_MODE=INVOKER|out of scope" \
    "LAB_MODES=kernel@SUB_PARAMS=rcv-wnd=1m+bogus=1|bogus is not an Aeron URI parameter" "LAB_NO_FLATOUT=1 LAB_TPUT_FLOWS=2|flat out" \
    "LAB_RTT_SPIKE_NS=-1|LAB_RTT_SPIKE_NS is a whole number of nanoseconds" "LAB_RTT_STALL_NS=10k|LAB_RTT_STALL_NS is a whole number of nanoseconds"; do
    # shellcheck disable=SC2086
    if env PATH="$tmp/fakebin:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/commarepo" \
        ${refused%|*} bash "$cloud/azure/lab.sh" >"$tmp/az.out" 2>&1; then
        fail "azure/lab.sh ran with ${refused%|*}"
    elif grep -qF -- "${refused##*|}" "$tmp/az.out" && [[ ! -s $tmp/cloud-calls ]]; then
        pass "azure/lab.sh refuses ${refused%|*} before any cloud call (${refused##*|})"
    else
        fail "azure/lab.sh with ${refused%|*}: $(tail -2 "$tmp/az.out"; cat "$tmp/cloud-calls" 2>/dev/null)"
    fi
    rm -f "$tmp/cloud-calls"
done

# aws/lab.sh and Graviton (Arm) types. A fake aws first on PATH logs every
# call, answers the read-only ones (an instance type is arm64 when its family
# has a g after the generation, as c8g's, unless FAKE_ARCH says otherwise) and
# refuses everything else, so a run stops at its first create (the VPC) and
# tears down what it created: nothing.
mkdir -p "$tmp/fakeaws" "$tmp/labtmp" "$tmp/labhome/.ssh" "$tmp/labrepo"
echo 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA lab-test' >"$tmp/labhome/.ssh/id_ed25519.pub"
cat >"$tmp/fakeaws/aws" <<'SH'
#!/bin/sh
echo "aws $*" >>"$AWS_CALLS"
case "$*" in
"sts get-caller-identity"*) echo '{"Account": "000000000000"}' ;;
*" describe-instance-type-offerings "*) printf 'us-east-1a\n' ;;
*" describe-instance-types "*)
    case "$*" in *g.*) echo "${FAKE_ARCH:-arm64}" ;; *) printf 'i386\tx86_64\n' ;; esac ;;
*" describe-images "*"arm64"*) printf 'ami-0arm64\t/dev/xvda\n' ;;
*" describe-images "*) echo ami-0amd64 ;;
*" describe-"*) ;;
*) exit 1 ;;
esac
SH
chmod +x "$tmp/fakeaws/aws"
# lab_aws <calls file> [VAR=value...]: aws/lab.sh against the fake, its
# output in <calls file>.out
lab_aws() {
    local calls=$1
    shift
    : >"$calls"
    env PATH="$tmp/fakeaws:$PATH" TMPDIR="$tmp/labtmp" HOME="$tmp/labhome" LAB_REPO="$tmp/labrepo" \
        LAB_SSH_FROM=192.0.2.10 AWS_CALLS="$calls" "$@" bash "$cloud/aws/lab.sh" >"$calls.out" 2>&1
}
# the calls' verbs up to the first create, as one line
verbs() { awk '{print ($2 == "ec2" ? $5 : $3)} $5 ~ /^create-/ {exit}' "$1" | tr '\n' ' '; }
# an Arm type is refused without LAB_ALLOW_ARM=1, an Arm and an x86 instance
# even with it, a threading mode Aeron does not know (it would run DEDICATED),
# INVOKER where each client's embedded driver would meet another on one host
# (localhost, +idle), what the DPDK transport refuses at install (a pacing
# wait under a one-agent mode, a network agent that does not spin, an MTU
# over its maximum), an MTU over the path's (inter-Region peering's 8500,
# the placement's), a mode setting with a value it does not take, Aeron URI
# parameters the lab does not know or on the wrong side (Aeron ignores an
# unknown one silently),
# SMT=off where there is no SMT, LAB_THREADS_PER_CORE where AWS takes no CPU
# options, a placement the lab does not know, cross-AZ and cross-region
# without LAB_NO_FLATOUT=1 (their traffic is billed), what LAB_NO_FLATOUT=1
# cannot run (several flows flat out; fractions of a flat-out maximum), a
# message size over 8192 B or fitting no mode's MTU, and a state lib.sh does
# not know (it would run as pinned), each before any cloud call
for refused in "LAB_TYPE=c8g.4xlarge|LAB_ALLOW_ARM=1" "LAB_TYPE_B=c7gn.4xlarge|LAB_ALLOW_ARM=1" "LAB_TYPE=a1.large|LAB_ALLOW_ARM=1" \
    "LAB_ALLOW_ARM=1 LAB_TYPE_A=c8g.4xlarge LAB_TYPE_B=c7i.4xlarge|one architecture" \
    "LAB_MODES=kernel@AERON_THREADING_MODE=bogus|DEDICATED, SHARED_NETWORK, SHARED or INVOKER" \
    "LAB_LOCAL_MODES=kernel@AERON_THREADING_MODE=shared|DEDICATED, SHARED_NETWORK, SHARED or INVOKER" \
    "LAB_MODES=kernel@AERON_THREADING_MODE=SHARED@AERON_THREADING_MODE=bogus|one AERON_THREADING_MODE at most" \
    "LAB_MODES=kernel-busy@AERON_THREADING_MODE=SHARED@RX_RING=4096@AERON_THREADING_MODE=SHARED_NETWORK|one AERON_THREADING_MODE at most" \
    "LAB_LOCAL_MODES=kernel@AERON_THREADING_MODE=INVOKER|localhost) is out of scope" \
    "LAB_MODES=dpdk@AERON_THREADING_MODE=INVOKER+idle|INVOKER with +idle" \
    "LAB_MODES=dpdk@AERON_THREADING_MODE=SHARED@RUSTERON_DPDK_TX_PACE_SPIN_US=20|refuses a pacing wait" \
    "LAB_MODES=dpdk@AERON_THREADING_MODE=SHARED_NETWORK@AERON_SHAREDNETWORK_IDLE_STRATEGY=backoff|does not busy-spin" \
    "LAB_MODES=dpdk@AERON_MTU_LENGTH=4096|the transport refuses it" \
    "LAB_PLACEMENT=cross-region LAB_REGION2=us-west-2 LAB_NO_FLATOUT=1 LAB_MODES=kernel@JUMBO=1@PUB_PARAMS=mtu=8960|is over 8448" \
    "LAB_MODES=kernel@JUMBO=yes|JUMBO=1 or nothing" \
    "LAB_MODES=kernel@SUB_PARAMS=rcv-window=1m|rcv-window is not an Aeron URI parameter the lab knows" \
    "LAB_MODES=kernel@PUB_PARAMS=rcv-wnd=1m|rcv-wnd is a subscription's" \
    "LAB_MODES=kernel@SUB_PARAMS=term-length=64m|term-length is a publication's" \
    "LAB_MODES=kernel@SUB_PARAMS=rcv-wnd=1m|so-rcvbuf=1m|<key>=<value>+<key>=<value>" \
    "LAB_MODES=kernel@SUB_PARAMS=rcv-wnd=1m,so-rcvbuf=1m|comma" \
    "LAB_ALLOW_ARM=1 LAB_TYPE=c8g.4xlarge LAB_MODES=kernel@SMT=off|no SMT" \
    "LAB_MODES=kernel@SMT=on|SMT=off or nothing" \
    "LAB_THREADS_PER_CORE=1 LAB_MODES=kernel@SMT=off|no SMT to turn off" \
    "LAB_THREADS_PER_CORE=2|LAB_THREADS_PER_CORE is 1" \
    "LAB_THREADS_PER_CORE=1 LAB_ALLOW_ARM=1 LAB_TYPE=c8g.4xlarge|Graviton" \
    "LAB_THREADS_PER_CORE=1 LAB_TYPE=c7i.metal-24xl|bare metal" \
    "LAB_THREADS_PER_CORE=1 LAB_TYPE_B=c7a.4xlarge|C7a" \
    "LAB_PLACEMENT=spread|cluster, precision-time, cross-az or cross-region" \
    "LAB_PLACEMENT=cross-az|LAB_NO_FLATOUT=1" \
    "LAB_PLACEMENT=cross-region LAB_REGION2=us-west-2|LAB_NO_FLATOUT=1" \
    "LAB_PLACEMENT=cross-region LAB_NO_FLATOUT=1|LAB_REGION2" \
    "LAB_PLACEMENT=cross-region LAB_NO_FLATOUT=1 LAB_REGION2=us-east-1|another region" \
    "LAB_PLACEMENT=cross-az LAB_NO_FLATOUT=1 LAB_AZS=us-east-1a|two zones" \
    "LAB_NO_FLATOUT=1 LAB_TPUT_FLOWS=2|flat out" \
    "LAB_NO_FLATOUT=1 LAB_LOAD_SIZES=288|LAB_LOAD_LADDER" \
    "LAB_LOAD_SIZES=9000|8192" \
    "LAB_LOAD_SIZES=4096|fits no mode" \
    "LAB_MIX=p=o:3001:1000:9000|8192" \
    "LAB_SPOT=yes|LAB_SPOT is 1" \
    "LAB_MAX_HOURS=8h|LAB_MAX_HOURS is a whole number of hours" "LAB_MAX_HOURS=0|LAB_MAX_HOURS is a whole number of hours" \
    "LAB_MODES=dpdk@IRQ=receiver|kernel modes only" "LAB_LOCAL_MODES=kernel@IRQ=spare|fifth core" \
    "LAB_MODES=kernel@IRQ=all|hk, receiver or spare" "LAB_MODES=kernel@NAPI_DEFER=2|<n>/<ns>" \
    "LAB_MODES=dpdk@NAPI_BUSY=spare|kernel modes only" "LAB_LOCAL_MODES=kernel@NAPI_BUSY=spare|fifth core" \
    "LAB_STATES=isolate|pinned, isolated or isolated-poll" \
    "LAB_STATES=isolated-nodiags|optionally ending -nodiag" \
    "LAB_RTT_SPIKE_NS=30us|LAB_RTT_SPIKE_NS is a whole number of nanoseconds" "LAB_RTT_STALL_NS=0|LAB_RTT_STALL_NS is a whole number of nanoseconds" \
    "LAB_MODES=dpdk@HAIRPIN=1|the raw floor" "LAB_ALLOW_ARM=1 LAB_TYPE=c9g.large LAB_MODES=dpdk-raw@HAIRPIN=1|four cores" \
    "LAB_ALLOW_ARM=1 LAB_TYPE_A=c9g.xlarge LAB_TYPE_B=c9g.large LAB_MODES=dpdk-raw@HAIRPIN=1|on host b too" \
    "LAB_MODES=dpdk@RTT_GAP_US=5ms|1 to 1000000" "LAB_LOCAL_MODES=kernel@RTT_GAP_US=1000|cross-host"; do
    # shellcheck disable=SC2086
    if lab_aws "$tmp/arm-refused" ${refused%|*}; then
        fail "aws/lab.sh ran with ${refused%|*}"
    elif grep -qF -- "${refused##*|}" "$tmp/arm-refused.out" && [[ ! -s $tmp/arm-refused ]]; then
        pass "aws/lab.sh refuses ${refused%|*} before any cloud call (${refused##*|})"
    else
        fail "aws/lab.sh with ${refused%|*}: $(tail -3 "$tmp/arm-refused.out"; cat "$tmp/arm-refused")"
    fi
done
# with LAB_ALLOW_ARM=1: AWS confirms both types are arm64, then Debian 13's
# arm64 AMI (with its root device) is chosen
lab_aws "$tmp/arm-ok" LAB_ALLOW_ARM=1 LAB_TYPE=c8g.4xlarge || true
images=$(grep ' describe-images ' "$tmp/arm-ok" || true)
if [[ $(verbs "$tmp/arm-ok") != "get-caller-identity describe-instance-types describe-instance-type-offerings describe-instance-type-offerings describe-images create-vpc " ]]; then
    fail "aws/lab.sh, Graviton with LAB_ALLOW_ARM=1: calls $(verbs "$tmp/arm-ok"); $(tail -3 "$tmp/arm-ok.out")"
elif [[ $images != *"Name=name,Values=debian-13-arm64-* Name=architecture,Values=arm64 "* || $images != *RootDeviceName* ]] ||
    ! grep -q 'AMI ami-0arm64' "$tmp/arm-ok.out" || grep -q run-instances "$tmp/arm-ok"; then
    fail "aws/lab.sh, Graviton with LAB_ALLOW_ARM=1: not Debian 13's arm64 AMI: $images; $(grep AMI "$tmp/arm-ok.out")"
else
    pass "aws/lab.sh with LAB_ALLOW_ARM=1: both types confirmed arm64, Debian 13's arm64 AMI"
fi
# a type the name calls Arm that AWS does not report as arm64: refused
if lab_aws "$tmp/arm-not" LAB_ALLOW_ARM=1 LAB_TYPE=c8g.4xlarge FAKE_ARCH=x86_64; then
    fail "aws/lab.sh ran a type AWS does not report as arm64"
elif grep -q 'not arm64' "$tmp/arm-not.out" && [[ $(verbs "$tmp/arm-not") == "get-caller-identity describe-instance-types " ]]; then
    pass "aws/lab.sh refuses an Arm-named type AWS does not report as arm64, before creating anything"
else
    fail "aws/lab.sh and a type AWS does not report as arm64: $(tail -3 "$tmp/arm-not.out"; verbs "$tmp/arm-not")"
fi
# x86 makes the calls it made before Arm was allowed, the amd64 AMI's query unchanged
lab_aws "$tmp/x86-ok" LAB_TYPE=c7i.4xlarge || true
if [[ $(verbs "$tmp/x86-ok") != "get-caller-identity describe-instance-type-offerings describe-instance-type-offerings describe-images create-vpc " ]] ||
    ! grep -qF " describe-images --owners 136693071363 --filters Name=name,Values=debian-13-amd64-* Name=architecture,Values=x86_64 --query sort_by(Images,&CreationDate)[-1].ImageId --output text" "$tmp/x86-ok" ||
    ! grep -q 'AMI ami-0amd64' "$tmp/x86-ok.out"; then
    fail "aws/lab.sh, x86: calls changed: $(verbs "$tmp/x86-ok"); $(grep ' describe-images ' "$tmp/x86-ok"); $(tail -3 "$tmp/x86-ok.out")"
else
    pass "aws/lab.sh, x86: the same calls and AMI query as before"
fi
# What is now allowed reaches the cloud (its first calls; the fake refuses the
# first create): INVOKER (on Graviton under DPDK too: vm.sh checks its LLQ on
# a driver started for the purpose) and the shared threading modes under
# DPDK, jumbo frames with sizes up to 8192 B (a size over a mode's largest
# message is skipped in that mode, and said so before any cloud call), Aeron
# URI parameters beside a deeper receive ring (kernel RX_RING, DPDK's
# RUSTERON_DPDK_RX_DESCRIPTORS; a kernel socket's buffer up to net.core's 16
# MiB), SMT=off and LAB_THREADS_PER_CORE=1 on x86,
# and cross-AZ and cross-region with LAB_NO_FLATOUT=1 and a ladder for every size
# The final round's plan (scratchpad final-round-plan.md), as its
# command line gives it: G3, a c9g.2xlarge spot pair, the isolated state, 5
# reps, nothing flat out, the 288 B and 1344 B ladders (a burst each) after a
# 30 s warm-up at 25,000/s, DPDK with no RX descriptor override, kernel UDP
# with coalescing off and the queue IRQs on the spare core, and DPDK with
# Aeron's media receive timestamps in each frame's reserved value
G3_PLAN="LAB_SPOT=1|LAB_ALLOW_ARM=1|LAB_TYPE=c9g.2xlarge|LAB_STATES=isolated|LAB_REPS=5|LAB_NO_FLATOUT=1|LAB_LOAD_SIZES=288 1344|LAB_LOAD_LADDER=288:100000,1000000 1344:500000|LAB_LOAD_SECONDS=10|LAB_WARMUP_SECONDS=30|LAB_WARMUP_RATE=25000|LAB_MODES=dpdk kernel@COALESCE=off@IRQ=spare dpdk@SUB_PARAMS=media-rcv-ts-offset=reserved|LAB_MAX_HOURS=2"
accepted=0
for case in "LAB_MODES=dpdk@AERON_THREADING_MODE=SHARED_NETWORK dpdk@AERON_THREADING_MODE=SHARED kernel@AERON_THREADING_MODE=INVOKER dpdk@AERON_THREADING_MODE=INVOKER" \
    "LAB_MODES=kernel kernel@JUMBO=1|LAB_LOAD_SIZES=288 8192|LAB_LOAD_LADDER=288:1000 8192:1000|LAB_MIX=p=o:3001:100:32 q:3002:100:8192" \
    "LAB_MODES=dpdk@SUB_PARAMS=rcv-wnd=64m+so-rcvbuf=64m@RUSTERON_DPDK_RX_DESCRIPTORS=8192 kernel@SUB_PARAMS=rcv-wnd=16m+so-rcvbuf=16m+cc=cubic@PUB_PARAMS=term-length=128m@RX_RING=8192" \
    "LAB_ALLOW_ARM=1|LAB_TYPE=c8g.4xlarge|LAB_MODES=dpdk@AERON_THREADING_MODE=INVOKER kernel@AERON_THREADING_MODE=INVOKER" \
    "LAB_MODES=kernel@SMT=off dpdk|LAB_TYPE=c7i.4xlarge" "LAB_THREADS_PER_CORE=1|LAB_TYPE=c7i.4xlarge" \
    "LAB_PLACEMENT=cross-az|LAB_NO_FLATOUT=1|LAB_LOAD_SIZES=288|LAB_LOAD_LADDER=288:1000,2000" \
    "LAB_PLACEMENT=cross-region|LAB_REGION2=us-west-2|LAB_NO_FLATOUT=1|LAB_MODES=kernel dpdk@JUMBO=1" \
    "LAB_TYPE=c7i.4xlarge|LAB_MODES=kernel@IRQ=hk kernel@IRQ=receiver kernel-busy@IRQ=spare@NAPI_BUSY=spare kernel@NAPI_DEFER=2/200000|LAB_LOCAL_MODES=kernel@IRQ=receiver" \
    "$G3_PLAN"; do
    IFS='|' read -ra vars <<<"$case"
    lab_aws "$tmp/accepted" "${vars[@]}" || true
    if ! grep -q 'sts get-caller-identity' "$tmp/accepted" || grep -q 'ERROR: mode\|refuse' "$tmp/accepted.out"; then
        fail "aws/lab.sh refused ${vars[*]}: $(tail -3 "$tmp/accepted.out")"
    elif [[ $case == *8192* && $case == *JUMBO* ]] &&
        ! grep -q '8192 B is over mode kernel.s largest message (1376 B at MTU 1408): skipped there' "$tmp/accepted.out"; then
        fail "aws/lab.sh with 8192 B messages did not say which modes skip them: $(grep -i 8192 "$tmp/accepted.out")"
    else
        accepted=$((accepted + 1))
    fi
done
((accepted == 10)) && pass "aws/lab.sh takes INVOKER (Graviton's DPDK too), DPDK's shared modes, jumbo frames and 8192 B, URI parameters with deep rings, SMT off, cross-AZ and cross-region (oversized messages skipped per mode, said so), the data NICs' IRQ and NAPI settings, and the final round's G3 plan (its media-receive-timestamp mode)"

# LAB_ESTIMATE=1 (estimate.py's arithmetic is checked above): the plan's cost
# printed, and no cloud call, not even signing in
lab_aws "$tmp/estimate" LAB_ESTIMATE=1 LAB_PLACEMENT=cross-az LAB_NO_FLATOUT=1 LAB_TYPE=c7i.4xlarge || true
if [[ -s $tmp/estimate ]]; then
    fail "aws/lab.sh LAB_ESTIMATE=1 made cloud calls: $(head -3 "$tmp/estimate")"
elif ! grep -q 'instance-hours' "$tmp/estimate.out" || ! grep -q 'cross-AZ' "$tmp/estimate.out" || ! grep -q '\$' "$tmp/estimate.out"; then
    fail "aws/lab.sh LAB_ESTIMATE=1: $(cat "$tmp/estimate.out")"
else
    pass "aws/lab.sh LAB_ESTIMATE=1: the plan's instance-hours, GB and dollars, no cloud call"
fi
# with LAB_SPOT=1: the same on-demand figure, as an upper bound, saying spot
# costs less (no price looked up: still no cloud call)
lab_aws "$tmp/estimate-spot" LAB_ESTIMATE=1 LAB_SPOT=1 LAB_PLACEMENT=cross-az LAB_NO_FLATOUT=1 LAB_TYPE=c7i.4xlarge || true
od=$(grep '^instances about' "$tmp/estimate.out" | sed 's/^instances about \(\$[0-9.]*\).*/\1/')
if [[ -s $tmp/estimate-spot ]]; then
    fail "aws/lab.sh LAB_ESTIMATE=1 LAB_SPOT=1 made cloud calls: $(head -3 "$tmp/estimate-spot")"
elif [[ -z $od ]] || ! grep -q "^instances at most $od (on-demand.*an upper bound: LAB_SPOT=1 runs spot instances.*lower" "$tmp/estimate-spot.out" ||
    ! grep -q '^total at most \$.*spot.*lower' "$tmp/estimate-spot.out"; then
    fail "aws/lab.sh LAB_ESTIMATE=1 LAB_SPOT=1: $(cat "$tmp/estimate-spot.out")"
else
    pass "aws/lab.sh LAB_ESTIMATE=1 LAB_SPOT=1: the on-demand figure as an upper bound, spot said to be lower, no cloud call"
fi
# The final round's plan (G3_PLAN, above) with LAB_ESTIMATE=1: no cloud
# call. G3: setup 600 s, the isolated state 450 s, teardown 180 s, and 5 reps of 3 modes of 219 s (12 s, the
# round trip's 85 s, the warm-up's 30 + 7 s, then 288 B at 100,000 and
# 1,000,000/s and the burst at 100,000, 1344 B at 500,000/s and its burst,
# 10 + 7 s a point): 4,515 s, 1.25 h; at the spot price ($0.153/h) $0.38, at
# on-demand ($0.34776/h) $0.87; each way per mode and rep the round trip's
# 32,200,000 B, the warm-up's 31 x 25,000 x 348 B, 11 x 1,200,000 x 348 B and
# 11 x 1,000,000 x 1404 B: 20,339,500,000 B, x 15 = 305.092 GB, free within
# one zone.
plan_estimate() { # <calls file> <plan>: lab.sh LAB_ESTIMATE=1 with the plan's variables
    local vars
    IFS='|' read -ra vars <<<"$2"
    lab_aws "$1" LAB_ESTIMATE=1 "${vars[@]}" || true
}
plan_estimate "$tmp/estimate-g3" "$G3_PLAN"
if [[ -s $tmp/estimate-g3 ]]; then
    fail "aws/lab.sh LAB_ESTIMATE=1 of the final round's plan made cloud calls: $(head -2 "$tmp/estimate-g3")"
elif ! grep -q '^wall time about 1.25 h: 2.51 instance-hours' "$tmp/estimate-g3.out" ||
    ! grep -q '^instances about \$0.38 at spot prices (us-east-1 .*\$0.153/h and \$0.153/h.*), at most \$0.87 at on-demand prices' "$tmp/estimate-g3.out" ||
    ! grep -q '^between the hosts about 305.092 GB a to b and 305.092 GB b to a' "$tmp/estimate-g3.out" ||
    ! grep -q '^total about \$0.38 at spot prices, at most \$0.87 at on-demand prices' "$tmp/estimate-g3.out" ||
    grep -q 'WARNING' "$tmp/estimate-g3.out"; then
    fail "aws/lab.sh LAB_ESTIMATE=1 of G3: $(cat "$tmp/estimate-g3.out")"
else
    pass "aws/lab.sh LAB_ESTIMATE=1 of the final round's plan: hours, spot and on-demand dollars, bytes, no cloud call"
fi
# a plan longer than LAB_MAX_HOURS (each instance powers off at its deadline,
# cutting the run short): 100 reps of the default plan, about 18.9 h.
# LAB_ESTIMATE=1 says so, with the hours to give it (a fifth more, 23); a
# real run of it is refused before any cloud call, and goes ahead once
# LAB_MAX_HOURS covers it
lab_aws "$tmp/estimate-long" LAB_ESTIMATE=1 LAB_REPS=100 || true
lab_aws "$tmp/long" LAB_REPS=100 || true
lab_aws "$tmp/long-hours" LAB_REPS=100 LAB_MAX_HOURS=23 || true
if [[ -s $tmp/estimate-long ]] || ! grep -q '^WARNING: about 18.88 h, longer than LAB_MAX_HOURS=8: .*LAB_MAX_HOURS=23' "$tmp/estimate-long.out"; then
    fail "aws/lab.sh LAB_ESTIMATE=1, a plan over LAB_MAX_HOURS: $(cat "$tmp/estimate-long.out")"
elif [[ -s $tmp/long ]] || ! grep -q 'ERROR: .*longer than LAB_MAX_HOURS=8: .*LAB_MAX_HOURS=23' "$tmp/long.out"; then
    fail "aws/lab.sh, a plan over LAB_MAX_HOURS not refused before any cloud call: $(head -2 "$tmp/long"; tail -2 "$tmp/long.out")"
elif ! grep -q 'sts get-caller-identity' "$tmp/long-hours"; then
    fail "aws/lab.sh, a plan within LAB_MAX_HOURS=23 refused: $(tail -2 "$tmp/long-hours.out")"
else
    pass "aws/lab.sh: a plan longer than LAB_MAX_HOURS said so by LAB_ESTIMATE=1 and refused before any cloud call, with the hours to give it"
fi

# LAB_DELETE only deletes: none of the checks above stands in its way, so a
# run left behind is always deleted and verified by tag, whatever else is set
for kept in "LAB_TYPE=c8g.4xlarge" "LAB_ALLOW_ARM=1 LAB_TYPE_A=c8g.4xlarge LAB_TYPE_B=c7i.4xlarge" \
    "LAB_MODES=kernel@AERON_THREADING_MODE=INVOKER LAB_STATES=isolate"; do
    # shellcheck disable=SC2086
    lab_aws "$tmp/delete" LAB_DELETE=20990101-000000 $kept || true
    if grep -q 'verified: nothing tagged run=20990101-000000 remains' "$tmp/delete.out" &&
        grep -q ' describe-instances ' "$tmp/delete" && ! grep -q ' describe-instance-types ' "$tmp/delete"; then
        pass "aws/lab.sh LAB_DELETE with $kept: deleted and verified, nothing refused"
    else
        fail "aws/lab.sh LAB_DELETE with $kept: $(tail -3 "$tmp/delete.out")"
    fi
done

# aws/lab.sh end to end against a stateful fake aws (fake_aws.py: it creates,
# lists and deletes what lab.sh asks, refusing a delete AWS would refuse for a
# dependency), fake ssh and scp that succeed, a curl that fails (so the
# address follower calls nothing) and a sleep of 10 ms (polls). A run creates everything, launches, attaches
# the data ENIs, then stops at packing the checkout (LAB_REPO is no git
# repository) and tears down. lab_full <name> [VAR=value...]: in
# $tmp/full-<name>/: calls (the run's stamp as <stamp>, its directory as
# <dir>; the zone listings, made at once, sorted), out, rc (lab.sh's exit
# status) and state.json, the fake's resources (FULL_AGAIN=1: the last run's
# directory and resources kept, as for LAB_DELETE after LAB_KEEP=1;
# FULL_BIN=<dir>: other fakes first on PATH). A run still going after
# FULL_TIMEOUT (300) seconds is killed: a hang fails, never blocks the test.
fullbin=$tmp/fullbin
mkdir -p "$fullbin"
printf '#!/bin/sh\nexec python3 "%s" "$@"\n' "$here/fake_aws.py" >"$fullbin/aws"
printf '#!/bin/sh\nexit 0\n' >"$fullbin/ssh"
printf '#!/bin/sh\nexit 0\n' >"$fullbin/scp"
printf '#!/bin/sh\nexit 7\n' >"$fullbin/curl"
# a poll's pause, short (the address follower's 20 s ends its loop)
printf '#!/bin/sh\n[ "$1" = 20 ] && exit 1\nexec /bin/sleep 0.01\n' >"$fullbin/sleep"
chmod +x "$fullbin"/*
# hosts that never answer SSH (an instance gone): ssh fails as a dropped link
# does, after 2 s, so a TERM in the meantime lands in a step whose stderr is
# redirected (wait_ssh's on ... 2>/dev/null), whose trap runs the teardown
downbin=$tmp/downbin
mkdir -p "$downbin"
cp "$fullbin"/* "$downbin/"
printf '#!/bin/sh\necho "ssh $*" >>"$AWS_CALLS.ssh"\n/bin/sleep 2\nexit 255\n' >"$downbin/ssh"
chmod +x "$downbin/ssh"
lab_full() {
    local d=$tmp/full-$1
    shift
    [[ -n ${FULL_AGAIN:-} ]] || rm -rf "$d"
    mkdir -p "$d/home/.ssh" "$d/tmp" "$d/repo"
    echo 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA lab-test' >"$d/home/.ssh/id_ed25519.pub"
    : >"$d/raw"
    local rc=0
    perl -e 'alarm shift; exec @ARGV' "${FULL_TIMEOUT:-300}" \
        env PATH="${FULL_BIN:-$fullbin}:$PATH" TMPDIR="$d/tmp" HOME="$d/home" LAB_REPO="$d/repo" LAB_SSH_FROM=192.0.2.10 \
        AWS_CALLS="$d/raw" FAKE_AWS_STATE="$d/state.json" "$@" bash "$cloud/aws/lab.sh" >"$d/out" 2>&1 || rc=$?
    echo "$rc" >"$d/rc"
    # lab.sh lists both types' zones at once (comm of two process
    # substitutions), so those two calls come in either order: sorted
    sed -E "s/[0-9]{8}-[0-9]{6}/<stamp>/g; s#$d#<dir>#g" "$d/raw" | python3 -c '
import sys
lines, out = sys.stdin.read().splitlines(), []
for line in lines:
    if " describe-instance-type-offerings " in line and out and " describe-instance-type-offerings " in out[-1][-1]:
        out[-1].append(line)
    else:
        out.append([line])
print("\n".join(l for block in out for l in sorted(block)))' >"$d/calls"
}
# what the fake still holds (an instance terminated is gone, as is a spot
# request closed or cancelled): its ids
full_left() {
    python3 -c 'import json, sys
s = json.load(open(sys.argv[1]))
print(" ".join(sorted(i for i, r in s["res"].items()
                      if not r.get("deleted") and r.get("state") not in ("terminated", "closed", "cancelled"))))' "$1"
}
# expected-aws-calls.txt: each scenario's calls under "## <scenario>", as
# lab.sh made them before cross-AZ, cross-region, LAB_THREADS_PER_CORE,
# LAB_SPOT and the cost guard: a cluster run whose b found no capacity in the
# first zone (a launched there, given back, both relaunched in the next), a
# precision-time run, and LAB_DELETE of a run with nothing left. None of the
# new options set, each must make exactly these calls. Regenerated once, on
# purpose, for one change alone: the cluster and precision-time placements
# ran `aws ec2 wait instance-terminated` (giving a back, and in the teardown)
# and `aws ec2 wait instance-running`, each holding the shared AWS CLI lock
# for the whole wait (minutes, every other lab session and billing check
# blocked behind it, and an `aws login` refresh with it); they now poll
# describe-instances for the instances' state, the lock taken for each short
# call, as the cross-AZ and cross-region placements already did. Every other
# call is unchanged.
for scenario in "cluster|LAB_TYPE_A=c7i.4xlarge LAB_TYPE_B=c7i.8xlarge FAKE_NO_CAPACITY=us-east-1a/c7i.8xlarge" \
    "precision-time|LAB_PLACEMENT=precision-time LAB_MODES=kernel" "delete|LAB_DELETE=20990101-000000"; do
    name=${scenario%%|*}
    # shellcheck disable=SC2086
    lab_full "$name" ${scenario#*|}
    awk -v h="## $name" '$0 == h {on = 1; next} on && /^## / {exit} on' "$here/expected-aws-calls.txt" >"$tmp/full-$name/want"
    if [[ ! -s $tmp/full-$name/want ]]; then
        fail "aws/lab.sh $name end to end: no expected calls for it"
    elif grep -q ' wait ' "$tmp/full-$name/calls"; then
        fail "aws/lab.sh $name end to end: an aws ... wait, the CLI lock held across it: $(grep ' wait ' "$tmp/full-$name/calls" | head -2)"
    elif ! diff -u "$tmp/full-$name/want" "$tmp/full-$name/calls" >"$tmp/diff"; then
        fail "aws/lab.sh $name end to end: the calls changed"
        cat "$tmp/diff" >&2
    elif [[ -n $(full_left "$tmp/full-$name/state.json" 2>/dev/null) ]] ||
        ! grep -q 'verified: nothing tagged run=.* remains in us-east-1' "$tmp/full-$name/out"; then
        fail "aws/lab.sh $name end to end: left $(full_left "$tmp/full-$name/state.json"); $(tail -3 "$tmp/full-$name/out")"
    else
        pass "aws/lab.sh $name end to end: the same calls as before, nothing left, verified"
    fi
done
# instances that never run (FAKE_PENDING=1: they stay pending), in each
# placement that waits for them: the wait polls to its deadline (60 short
# describe-instances calls, the CLI lock taken for each, never an aws ...
# wait), ends with a clear timeout error, and the teardown deletes and
# verifies everything
for case in "cluster|" "precision-time|LAB_PLACEMENT=precision-time LAB_MODES=kernel" "cross-az|LAB_PLACEMENT=cross-az LAB_NO_FLATOUT=1"; do
    name=pending-${case%%|*}
    # shellcheck disable=SC2086
    lab_full "$name" FAKE_PENDING=1 ${case#*|}
    polls=$(grep -c ' describe-instances --instance-ids .*State.Name' "$tmp/full-$name/calls" || true)
    if [[ $(cat "$tmp/full-$name/rc") != 1 ]] || ! grep -q 'timed out after 600 s waiting for .* to be running' "$tmp/full-$name/out"; then
        fail "aws/lab.sh ${case%%|*}, instances that never run: no timeout error (exit $(cat "$tmp/full-$name/rc"); $(tail -3 "$tmp/full-$name/out"))"
    elif grep -q ' wait ' "$tmp/full-$name/calls" || ((polls < 60 || polls > 62)); then
        fail "aws/lab.sh ${case%%|*}, instances that never run: $polls state polls; $(grep ' wait ' "$tmp/full-$name/calls" | head -1)"
    elif [[ -n $(full_left "$tmp/full-$name/state.json") ]] || ! grep -q 'verified: nothing tagged run=.* remains in us-east-1' "$tmp/full-$name/out"; then
        fail "aws/lab.sh ${case%%|*}, instances that never run: left $(full_left "$tmp/full-$name/state.json"); $(tail -3 "$tmp/full-$name/out")"
    else
        pass "aws/lab.sh ${case%%|*}: instances that never run time out after 60 polls (no aws ... wait), said so, deleted and verified"
    fi
done

# full_check <name> <python>: a check of run <name> against the fake's
# resources (res: its resources by id; calls: the call lines; out: lab.sh's
# output; by_ip: each data ENI's address to the zone of the instance it is
# attached to); its output, one problem a line, nothing when it holds
full_check() {
    python3 - "$tmp/full-$1" "$2" <<'PY'
import json, os, sys
d = sys.argv[1]
s = json.load(open(os.path.join(d, "state.json")))
calls = open(os.path.join(d, "calls")).read().splitlines()
out = open(os.path.join(d, "out")).read()
res = s["res"]
by_ip = {r["ip"]: res[res[r["instance"]]["subnet"]]["az"] for r in res.values() if r["kind"] == "eni" and r.get("instance")}
problems = []
def want(cond, what):
    if not cond:
        problems.append(what)
exec(sys.argv[2])
print("\n".join(problems))
PY
}
# cross-AZ: one VPC, a mgmt and a data subnet in each host's zone (b's
# 10.30.2.0/24 and 10.30.3.0/24), no placement group, each instance and its
# data ENIs in its own zone (b's at 10.30.3.6 and .7). The first pair of
# zones with capacity for both: b has none in us-east-1b, so a is given back
# (polled to terminated: the CLI lock is never held across a wait) and the
# next pair tried. Deleted and verified.
lab_full crossaz LAB_PLACEMENT=cross-az LAB_NO_FLATOUT=1 LAB_TYPE=c7i.4xlarge \
    FAKE_ZONES="us-east-1=us-east-1a us-east-1b us-east-1c" FAKE_NO_CAPACITY=us-east-1b/c7i.4xlarge
problems=$(full_check crossaz '
want(by_ip.get("10.30.1.4") == by_ip.get("10.30.1.5") == "us-east-1a", f"a not in us-east-1a: {by_ip}")
want(by_ip.get("10.30.3.6") == by_ip.get("10.30.3.7") == "us-east-1c", f"b not in us-east-1c: {by_ip}")
want(not any(" create-placement-group " in c or "--placement" in c for c in calls), "a placement group")
want(not any(" wait " in c for c in calls), "an aws ... wait (the lock held across it)")
subnets = sorted((r["cidr"], r["az"]) for r in res.values() if r["kind"] == "subnet")
want(("10.30.2.0/24", "us-east-1b") in subnets and ("10.30.3.0/24", "us-east-1c") in subnets, f"b subnets: {subnets}")
want(sum(" run-instances " in c for c in calls) == 4, "not a, b refused, a and b again")
want("verified: nothing tagged run=" in out and " remains in us-east-1" in out, "not verified")
left = [i for i, r in res.items() if not r.get("deleted") and r.get("state") != "terminated"]
want(not left, f"left: {left}")
')
if [[ -n $problems ]]; then
    fail "aws/lab.sh cross-az: $problems; $(tail -3 "$tmp/full-crossaz/out")"
elif ! grep -qx 'zone a us-east-1a b us-east-1c' "$tmp"/full-crossaz/repo/target/md-dpdk-bench/aws-*/instances.txt; then
    fail "aws/lab.sh cross-az: instances.txt: $(cat "$tmp"/full-crossaz/repo/target/md-dpdk-bench/aws-*/instances.txt)"
else
    pass "aws/lab.sh cross-az: a subnet pair per zone, no placement group, the next zone pair on no capacity, deleted and verified"
fi
# LAB_AZS: those zones, a's then b's, and no other
lab_full crossaz-azs LAB_PLACEMENT=cross-az LAB_NO_FLATOUT=1 LAB_AZS="us-east-1c us-east-1a" \
    FAKE_ZONES="us-east-1=us-east-1a us-east-1b us-east-1c"
problems=$(full_check crossaz-azs '
want(by_ip.get("10.30.1.4") == "us-east-1c" and by_ip.get("10.30.3.6") == "us-east-1a", f"zones: {by_ip}")
want(sum(" run-instances " in c for c in calls) == 2, "more than one try")
')
if [[ -z $problems ]]; then
    pass "aws/lab.sh cross-az with LAB_AZS: a and b in the zones given"
else
    fail "aws/lab.sh cross-az with LAB_AZS: $problems; $(tail -3 "$tmp/full-crossaz-azs/out")"
fi
# a zone of LAB_AZS that does not offer the type: refused before creating anything
lab_full crossaz-bad LAB_PLACEMENT=cross-az LAB_NO_FLATOUT=1 LAB_AZS="us-east-1a us-east-1d"
if [[ $(cat "$tmp/full-crossaz-bad/rc") != 0 ]] && grep -q 'us-east-1d' "$tmp/full-crossaz-bad/out" &&
    ! grep -q ' create-' "$tmp/full-crossaz-bad/calls"; then
    pass "aws/lab.sh cross-az refuses a zone of LAB_AZS without the type, before creating anything"
else
    fail "aws/lab.sh cross-az with a zone without the type: $(tail -3 "$tmp/full-crossaz-bad/out")"
fi
# cross-region: a VPC in each region (10.30.0.0/16, 10.31.0.0/16), peered
# (requested from a's region, accepted in b's, tagged in both), a route to the
# other VPC each side, security groups by the peer's CIDR (a security group
# cannot reference one in another region), each region's AMI, the key pair in
# both, every resource tagged; teardown deletes the peering and everything
# else in both regions and verifies both, with no aws ... wait
lab_full xregion LAB_PLACEMENT=cross-region LAB_REGION2=us-west-2 LAB_NO_FLATOUT=1 LAB_TYPE=c7i.4xlarge
problems=$(full_check xregion '
vpcs = {r["region"]: r["cidr"] for r in res.values() if r["kind"] == "vpc"}
want(vpcs == {"us-east-1": "10.30.0.0/16", "us-west-2": "10.31.0.0/16"}, f"VPCs: {vpcs}")
pcx = [r for r in res.values() if r["kind"] == "pcx"]
want(len(pcx) == 1 and pcx[0]["regions"] == ["us-east-1", "us-west-2"], f"peering: {pcx}")
for p in pcx:
    for region in p["regions"]:
        t = p.get("tags_by_region", {}).get(region, {})
        want(t.get("owner") == "ergon-lab" and t.get("run"), f"peering untagged in {region}: {t}")
want(any(" accept-vpc-peering-connection " in c and "--region us-west-2 " in c for c in calls), "not accepted in b region")
rts = {r["region"]: r["routes"] for r in res.values() if r["kind"] == "rt"}
want(any(x[0] == "10.31.0.0/16" and x[1].startswith("pcx-") for x in rts.get("us-east-1", [])), f"no route to b VPC: {rts}")
want(any(x[0] == "10.30.0.0/16" and x[1].startswith("pcx-") for x in rts.get("us-west-2", [])), f"no route to a VPC: {rts}")
data = {r["region"]: r.get("rt") for r in res.values() if r["kind"] == "subnet" and r["cidr"].endswith(".1.0/24")}
want(all(data.values()) and len(data) == 2, f"data subnets without the route table: {data}")
sgs = {r["region"]: r["rules"] for r in res.values() if r["kind"] == "sg"}
want(["-1", None, "10.31.0.0/16"] in sgs.get("us-east-1", []) and ["-1", None, "10.30.0.0/16"] in sgs.get("us-west-2", []), f"security groups: {sgs}")
keys = sorted(r["region"] for r in res.values() if r["kind"] == "key")
want(keys == ["us-east-1", "us-west-2"], f"key pairs: {keys}")
want(any(" describe-images " in c and "--region us-west-2 " in c for c in calls), "no AMI looked up in us-west-2")
want(by_ip.get("10.30.1.4", "").startswith("us-east-1") and by_ip.get("10.31.1.6", "").startswith("us-west-2"), f"hosts: {by_ip}")
untagged = [i for i, r in res.items() if r["kind"] != "pcx" and (r.get("tags", {}).get("owner") != "ergon-lab" or not r.get("tags", {}).get("run"))]
want(not untagged, f"untagged: {untagged}")
want(not any(" wait " in c for c in calls), "an aws ... wait (the lock held across it)")
want(any(" delete-vpc-peering-connection " in c for c in calls), "the peering not deleted")
want("remains in us-east-1" in out and "remains in us-west-2" in out, "not verified in both regions")
left = [i for i, r in res.items() if not r.get("deleted") and r.get("state") != "terminated"]
want(not left, f"left: {left}")
')
if [[ -n $problems ]]; then
    fail "aws/lab.sh cross-region: $problems; $(tail -3 "$tmp/full-xregion/out")"
elif [[ $(cat "$tmp"/full-xregion/repo/target/md-dpdk-bench/aws-*/regions 2>/dev/null) != "us-east-1 us-west-2" ]]; then
    fail "aws/lab.sh cross-region: no regions file for LAB_DELETE"
else
    pass "aws/lab.sh cross-region: VPCs peered across the regions, routes and security groups by the peer CIDR, each region's AMI and key, all tagged; deleted and verified in both"
fi
# LAB_KEEP=1 then LAB_DELETE=<stamp>: both regions swept (the run's regions
# file), the peering included, and verified in both
lab_full xkeep LAB_PLACEMENT=cross-region LAB_REGION2=us-west-2 LAB_NO_FLATOUT=1 LAB_KEEP=1
kept=$(full_left "$tmp/full-xkeep/state.json")
keep_said=$(grep 'LAB_KEEP: run .* kept' "$tmp/full-xkeep/out" || true)
stamp_kept=$(sed -n 's/.*LAB_DELETE=\([0-9-]*\) .*/\1/p' "$tmp/full-xkeep/out" | head -1)
FULL_AGAIN=1 lab_full xkeep LAB_DELETE="$stamp_kept"
if [[ -z $kept || -z $stamp_kept ]]; then
    fail "aws/lab.sh cross-region LAB_KEEP: nothing kept ($(tail -3 "$tmp/full-xkeep/out"))"
elif [[ -n $(full_left "$tmp/full-xkeep/state.json") ]] || ! grep -q "remains in us-west-2" "$tmp/full-xkeep/out" ||
    ! grep -q ' delete-vpc-peering-connection ' "$tmp/full-xkeep/calls"; then
    fail "aws/lab.sh cross-region LAB_DELETE: left $(full_left "$tmp/full-xkeep/state.json"); $(tail -3 "$tmp/full-xkeep/out")"
else
    pass "aws/lab.sh LAB_DELETE of a kept cross-region run: both regions and the peering deleted, verified in both"
fi
# cross-region on failure and on interrupt: b finding no capacity in any zone
# of its region (the run dies after a's instance, the peering and both VPCs
# exist), and a TERM once the peering is accepted (lab.sh's own trap): each
# deletes everything in both regions, the peering included, and verifies both
# (exit 1 from die, 130 from the TERM trap)
for case in "xfail|failed|FAKE_NO_CAPACITY=us-west-2a us-west-2b|ERROR: no zone of us-west-2 had capacity|1" \
    "xterm|interrupted|FAKE_TERM_AT=accept-vpc-peering-connection|deleting run|130"; do
    IFS='|' read -r name what setting why code <<<"$case"
    lab_full "$name" LAB_PLACEMENT=cross-region LAB_REGION2=us-west-2 LAB_NO_FLATOUT=1 "$setting"
    if [[ $(cat "$tmp/full-$name/rc") != "$code" ]] || ! grep -q "$why" "$tmp/full-$name/out"; then
        fail "aws/lab.sh cross-region $what: not exit $code with \"$why\" (exit $(cat "$tmp/full-$name/rc"); $(tail -3 "$tmp/full-$name/out"))"
    elif [[ -n $(full_left "$tmp/full-$name/state.json") ]] || ! grep -q ' delete-vpc-peering-connection ' "$tmp/full-$name/calls" ||
        ! grep -q "remains in us-east-1" "$tmp/full-$name/out" || ! grep -q "remains in us-west-2" "$tmp/full-$name/out"; then
        fail "aws/lab.sh cross-region $what: left $(full_left "$tmp/full-$name/state.json"); $(tail -3 "$tmp/full-$name/out")"
    else
        pass "aws/lab.sh cross-region $what: both regions and the peering deleted, verified in both"
    fi
done
# LAB_THREADS_PER_CORE=1: each instance launched with one thread a core and
# its type's every core (AWS's DefaultCores), and otherwise the calls of the
# cluster scenario above, the core counts' lookups aside
lab_full tpc LAB_THREADS_PER_CORE=1 LAB_TYPE_A=c7i.4xlarge LAB_TYPE_B=c7i.8xlarge FAKE_NO_CAPACITY=us-east-1a/c7i.8xlarge
awk -v h="## cluster" '$0 == h {on = 1; next} on && /^## / {exit} on' "$here/expected-aws-calls.txt" >"$tmp/full-tpc/want"
if ! grep -q -- '--instance-type c7i.4xlarge .*--cpu-options CoreCount=8,ThreadsPerCore=1 ' "$tmp/full-tpc/calls" ||
    ! grep -q -- '--instance-type c7i.8xlarge .*--cpu-options CoreCount=16,ThreadsPerCore=1 ' "$tmp/full-tpc/calls"; then
    fail "aws/lab.sh LAB_THREADS_PER_CORE=1: $(grep run-instances "$tmp/full-tpc/calls" | head -2)"
elif ! diff -u "$tmp/full-tpc/want" <(grep -v ' describe-instance-types ' "$tmp/full-tpc/calls" |
    sed 's/ --cpu-options CoreCount=[0-9]*,ThreadsPerCore=1//') >"$tmp/diff"; then
    fail "aws/lab.sh LAB_THREADS_PER_CORE=1: other calls changed"
    cat "$tmp/diff" >&2
else
    pass "aws/lab.sh LAB_THREADS_PER_CORE=1: launched with every core at one thread, nothing else changed"
fi

# LAB_SPOT=1: every instance a one-time spot instance that AWS terminates on
# interruption, still terminated on instance-initiated shutdown and with the
# deadline's user data, its spot request tagged with the run; a spot request
# a refused launch left open (FAKE_SPOT_LEAVE_OPEN=1) cancelled by the
# teardown, which lists the run's spot requests in its verification; the
# market in instances.txt and in the run's directory (for LAB_DELETE).
# Otherwise the cluster scenario's calls, the spot watch's aside (a
# describe-instances of the instances' state and its reason, every 30 s
# while they run).
spot_opts='--instance-market-options MarketType=spot,SpotOptions={SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}'
spot_tags='ResourceType=spot-instances-request,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=<stamp>},{Key=Name,Value=ergon-md-dpdk-spot}]'
# the calls less the spot watch's and the spot requests' own, the spot options taken out of each launch
spot_less() {
    grep -v -e 'StateReason' -e ' describe-spot-instance-requests ' -e ' cancel-spot-instance-requests ' "$1" |
        sed -e "s/ $spot_opts//" -e "s/ $(sed 's/[][]/\\&/g' <<<"$spot_tags")//"
}
lab_full spot LAB_SPOT=1 LAB_TYPE_A=c7i.4xlarge LAB_TYPE_B=c7i.8xlarge FAKE_NO_CAPACITY=us-east-1a/c7i.8xlarge FAKE_SPOT_LEAVE_OPEN=1
awk -v h="## cluster" '$0 == h {on = 1; next} on && /^## / {exit} on' "$here/expected-aws-calls.txt" >"$tmp/full-spot/want"
launches=$(grep -c ' run-instances ' "$tmp/full-spot/calls" || true)
spot_launches=$(grep ' run-instances ' "$tmp/full-spot/calls" | grep -F -- " $spot_opts " | grep -F -- " $spot_tags" |
    grep -c -- '--instance-initiated-shutdown-behavior terminate --user-data file://' || true)
problems=$(full_check spot '
sirs = {i: r for i, r in res.items() if r["kind"] == "sir"}
want(len(sirs) == 4, f"spot requests: {sirs}")
want(all(r["tags"].get("run") and r["tags"].get("owner") == "ergon-lab" for r in sirs.values()), f"untagged spot requests: {sirs}")
# the first a, given back in us-east-1a: closed with it; the b refused there:
# left open, cancelled; the pair: cancelled before the instances are terminated
want(sorted(r["state"] for r in sirs.values()) == ["cancelled", "cancelled", "cancelled", "closed"], f"spot requests: {sirs}")
want(any(" cancel-spot-instance-requests " in c for c in calls), "the open spot request not cancelled")
last = [c for c in calls if " describe-" in c][-13:]
want(any(" describe-spot-instance-requests " in c and "Values=open,active" in c for c in last), "spot requests not in the verification")
want(not any(" wait " in c for c in calls), "an aws ... wait")
')
if ((launches != 4 || spot_launches != 4)); then
    fail "aws/lab.sh LAB_SPOT=1: $spot_launches of $launches launches spot: $(grep ' run-instances ' "$tmp/full-spot/calls" | head -1)"
elif [[ -n $problems ]]; then
    fail "aws/lab.sh LAB_SPOT=1: $problems; $(tail -3 "$tmp/full-spot/out")"
elif ! diff -u "$tmp/full-spot/want" <(spot_less "$tmp/full-spot/calls") >"$tmp/diff"; then
    fail "aws/lab.sh LAB_SPOT=1: other calls changed"
    cat "$tmp/diff" >&2
elif [[ -n $(full_left "$tmp/full-spot/state.json") ]] || ! grep -q 'verified: nothing tagged run=.* remains in us-east-1' "$tmp/full-spot/out"; then
    fail "aws/lab.sh LAB_SPOT=1: left $(full_left "$tmp/full-spot/state.json"); $(tail -3 "$tmp/full-spot/out")"
elif ! grep -qx 'market spot' "$tmp"/full-spot/repo/target/md-dpdk-bench/aws-*/instances.txt ||
    [[ $(cat "$tmp"/full-spot/repo/target/md-dpdk-bench/aws-*/market) != spot ]] ||
    ! grep -qx 'market on-demand' "$tmp"/full-cluster/repo/target/md-dpdk-bench/aws-*/instances.txt; then
    fail "aws/lab.sh LAB_SPOT=1: the market not recorded: $(cat "$tmp"/full-spot/repo/target/md-dpdk-bench/aws-*/instances.txt)"
else
    pass "aws/lab.sh LAB_SPOT=1: one-time spot instances, their requests tagged, an open one cancelled, all verified; the rest as on demand"
fi
# A spot instance AWS reclaims mid-run (FAKE_RECLAIM=ssh: b, at the spot
# watch's first look once the run is trying SSH to hosts that do not answer,
# the TERM landing in that SSH step, its stderr redirected: the teardown's
# lines must still be seen)
# or as it starts (FAKE_RECLAIM=1: b, at the first look): the run ends
# through the normal teardown at once, never waiting out its SSH retries,
# saying the instance was reclaimed, exit 75 (try again); everything deleted
# and verified. Reclaimed at launch, before it ran (FAKE_RECLAIM=0): the
# wait for it to run ends at the first poll, the same way.
for case in "spot-reclaim|FAKE_RECLAIM=ssh|(b)|mid-run" "spot-start|FAKE_RECLAIM=1|(b)|as it starts" \
    "spot-launch|FAKE_RECLAIM=0|(a)|at launch"; do
    IFS='|' read -r name setting who when <<<"$case"
    FULL_BIN=$downbin FULL_TIMEOUT=120 lab_full "$name" LAB_SPOT=1 "$setting"
    polls=$(grep -c ' describe-instances --instance-ids .*State.Name --output' "$tmp/full-$name/calls" || true)
    if [[ $(cat "$tmp/full-$name/rc") != 75 ]] ||
        ! grep -q "spot instance i-[0-9]* $who was reclaimed by AWS (Server.SpotInstanceTermination)" "$tmp/full-$name/out"; then
        fail "aws/lab.sh LAB_SPOT=1, $setting: not exit 75 with the reclaim said (exit $(cat "$tmp/full-$name/rc"); $(grep -i 'reclaim\|ERROR' "$tmp/full-$name/out" | head -3; tail -2 "$tmp/full-$name/out"))"
    elif [[ $name == spot-launch ]] && ((polls > 3)); then
        fail "aws/lab.sh LAB_SPOT=1, reclaimed at launch: waited $polls polls for it to run"
    elif [[ $name == spot-reclaim ]] && [[ ! -s $tmp/full-$name/raw.ssh ]]; then
        fail "aws/lab.sh LAB_SPOT=1, reclaimed mid-run: never reached SSH"
    elif [[ $name != spot-launch ]] && grep -q 'ERROR' "$tmp/full-$name/out"; then
        # the watch's TERM ends the run, never a step that failed on (in the
        # benchmark's polls only the TERM would, hours on)
        fail "aws/lab.sh LAB_SPOT=1, $setting: ended by an error, not the spot watch: $(grep ERROR "$tmp/full-$name/out")"
    elif ! grep -q 'did not finish: a spot instance was reclaimed; exit 75' "$tmp/full-$name/out"; then
        fail "aws/lab.sh LAB_SPOT=1, $setting: no closing word on the exit status: $(tail -4 "$tmp/full-$name/out")"
    elif [[ -n $(full_left "$tmp/full-$name/state.json") ]] || ! grep -q 'verified: nothing tagged run=.* remains in us-east-1' "$tmp/full-$name/out" ||
        grep -q ' wait ' "$tmp/full-$name/calls"; then
        fail "aws/lab.sh LAB_SPOT=1, $setting: left $(full_left "$tmp/full-$name/state.json"); $(tail -3 "$tmp/full-$name/out")"
    else
        pass "aws/lab.sh LAB_SPOT=1, a spot instance reclaimed $when: said so, the normal teardown at once, exit 75, verified"
    fi
done
# cross-region with LAB_SPOT=1: the spot watch looks at each host in its own
# region, an instance reclaimed there ends the run as above, and each
# region's spot requests are swept and listed in its verification
FULL_BIN=$downbin FULL_TIMEOUT=120 lab_full spot-xregion LAB_SPOT=1 LAB_PLACEMENT=cross-region LAB_REGION2=us-west-2 \
    LAB_NO_FLATOUT=1 FAKE_RECLAIM=ssh
problems=$(full_check spot-xregion '
looks = [c for c in calls if "StateReason" in c]
want(any("--region us-east-1 " in c and "--region us-west-2 " not in c for c in looks) and any("--region us-west-2 " in c for c in looks),
     f"the watch not in both regions: {looks[:2]}")
for region in ("us-east-1", "us-west-2"):
    want(sum(" describe-spot-instance-requests " in c and f"--region {region} " in c for c in calls) >= 2,
         f"spot requests not swept and verified in {region}")
    want(f"remains in {region}" in out, f"not verified in {region}")
want(not any(" wait " in c for c in calls), "an aws ... wait")
')
if [[ $(cat "$tmp/full-spot-xregion/rc") != 75 ]] || ! grep -qE 'spot instance i-[0-9]+ \((a|b)\) was reclaimed by AWS' "$tmp/full-spot-xregion/out"; then
    fail "aws/lab.sh LAB_SPOT=1 cross-region, reclaimed: exit $(cat "$tmp/full-spot-xregion/rc"); $(tail -3 "$tmp/full-spot-xregion/out")"
elif [[ -n $problems || -n $(full_left "$tmp/full-spot-xregion/state.json") ]]; then
    fail "aws/lab.sh LAB_SPOT=1 cross-region: $problems; left $(full_left "$tmp/full-spot-xregion/state.json")"
else
    pass "aws/lab.sh LAB_SPOT=1 cross-region: each host watched in its region, the reclaim ends the run, spot requests swept and verified in both"
fi
# LAB_KEEP=1, then LAB_DELETE of that run without LAB_SPOT: its directory
# says spot, so its spot requests are swept too (an open one cancelled) and
# listed in the verification
lab_full spot-keep LAB_SPOT=1 LAB_KEEP=1 FAKE_NO_CAPACITY=us-east-1a FAKE_SPOT_LEAVE_OPEN=1
stamp_kept=$(sed -n 's/.*LAB_DELETE=\([0-9-]*\) .*/\1/p' "$tmp/full-spot-keep/out" | head -1)
FULL_AGAIN=1 lab_full spot-keep LAB_DELETE="$stamp_kept"
if [[ -z $stamp_kept ]] || ! grep -q ' cancel-spot-instance-requests ' "$tmp/full-spot-keep/calls" ||
    ! grep -q ' describe-spot-instance-requests .*Values=open,active' "$tmp/full-spot-keep/calls"; then
    fail "aws/lab.sh LAB_DELETE of a kept spot run: spot requests not swept: $(tail -3 "$tmp/full-spot-keep/out")"
elif [[ -n $(full_left "$tmp/full-spot-keep/state.json") ]] || ! grep -q 'verified: nothing tagged run=.* remains in us-east-1' "$tmp/full-spot-keep/out"; then
    fail "aws/lab.sh LAB_DELETE of a kept spot run: left $(full_left "$tmp/full-spot-keep/state.json"); $(tail -3 "$tmp/full-spot-keep/out")"
else
    pass "aws/lab.sh LAB_DELETE of a kept spot run: its spot requests swept (from its directory), verified"
fi
# LAB_KEEP=1 and a spot instance reclaimed (as it starts): the pair is broken,
# so nothing is kept, the other instance least of all (it would bill to
# LAB_MAX_HOURS for nothing while exit 75 has a retry start another pair):
# deleted and verified, saying why LAB_KEEP was not honoured, exit 75
FULL_BIN=$downbin FULL_TIMEOUT=120 lab_full spot-keep-reclaim LAB_SPOT=1 LAB_KEEP=1 FAKE_RECLAIM=1
if [[ $(cat "$tmp/full-spot-keep-reclaim/rc") != 75 ]] || [[ -n $(full_left "$tmp/full-spot-keep-reclaim/state.json") ]] ||
    ! grep -q 'verified: nothing tagged run=.* remains in us-east-1' "$tmp/full-spot-keep-reclaim/out" ||
    ! grep -q 'LAB_KEEP: not kept: a spot instance was reclaimed' "$tmp/full-spot-keep-reclaim/out"; then
    fail "aws/lab.sh LAB_SPOT=1 LAB_KEEP=1, reclaimed: exit $(cat "$tmp/full-spot-keep-reclaim/rc"), left $(full_left "$tmp/full-spot-keep-reclaim/state.json"); $(tail -3 "$tmp/full-spot-keep-reclaim/out")"
else
    pass "aws/lab.sh LAB_SPOT=1 LAB_KEEP=1, a spot instance reclaimed: nothing kept (the pair is broken), said so, deleted and verified, exit 75"
fi
# A TERM (or Ctrl-C) once the teardown has begun (an operator's second one, or
# the spot watch's, racing a run that ended on its own: FAKE_TERM_AT=
# terminate-instances, the teardown's first delete) must not cut it short:
# everything deleted and verified, the run's own exit status kept (1, from
# die), as azure/lab.sh's teardown does
lab_full term-teardown FAKE_TERM_AT=terminate-instances
if [[ $(cat "$tmp/full-term-teardown/rc") != 1 ]] || [[ -n $(full_left "$tmp/full-term-teardown/state.json") ]] ||
    ! grep -q 'verified: nothing tagged run=.* remains in us-east-1' "$tmp/full-term-teardown/out"; then
    fail "aws/lab.sh, a TERM during the teardown: exit $(cat "$tmp/full-term-teardown/rc"), left $(full_left "$tmp/full-term-teardown/state.json"); $(tail -3 "$tmp/full-term-teardown/out")"
else
    pass "aws/lab.sh, a TERM during the teardown: ignored, everything deleted and verified, the run's exit status kept"
fi
# Runs started in one second (three pairs at once) would share a stamp, and
# with it their tags, names (key pair, placement group) and run directory,
# each one's teardown deleting the others' resources. A run claims its
# directory: one already there (another run's, this second: a fake date gives
# 20990101-000000 first, then 20990101-000001) is left alone, and the run
# takes the next second's stamp for everything
datebin=$tmp/datebin
mkdir -p "$datebin"
cp "$fullbin"/* "$datebin/"
# shellcheck disable=SC2016
printf '#!/bin/sh\nif [ "$1" = +%%Y%%m%%d-%%H%%M%%S ]; then\n    n=$(cat "$AWS_CALLS.date" 2>/dev/null || echo 0)\n    echo $((n + 1)) >"$AWS_CALLS.date"\n    printf "20990101-%%06d\\n" "$n"\nelse\n    exec /bin/date "$@"\nfi\n' >"$datebin/date"
chmod +x "$datebin/date"
rm -rf "$tmp/full-stamp"
mkdir -p "$tmp/full-stamp/repo/target/md-dpdk-bench/aws-20990101-000000"
echo "another run's" >"$tmp/full-stamp/repo/target/md-dpdk-bench/aws-20990101-000000/note"
FULL_AGAIN=1 FULL_BIN=$datebin lab_full stamp
if [[ $(ls "$tmp/full-stamp/repo/target/md-dpdk-bench/aws-20990101-000000") != note ]] ||
    [[ ! -s $tmp/full-stamp/repo/target/md-dpdk-bench/aws-20990101-000001/instances.txt ]] ||
    grep -q 'Value=20990101-000000' "$tmp/full-stamp/raw" || ! grep -q 'Value=20990101-000001' "$tmp/full-stamp/raw" ||
    [[ -n $(full_left "$tmp/full-stamp/state.json") ]] || ! grep -q 'verified: nothing tagged run=20990101-000001 remains' "$tmp/full-stamp/out"; then
    fail "aws/lab.sh, a run directory already taken this second: $(ls "$tmp/full-stamp/repo/target/md-dpdk-bench"); $(grep -o 'Value=2099[0-9-]*' "$tmp/full-stamp/raw" | sort | uniq -c | tr '\n' ' '); $(tail -2 "$tmp/full-stamp/out")"
else
    pass "aws/lab.sh: a run started in the same second as another takes a stamp (tags, names, directory) of its own"
fi

# host.sh, what vm.sh sets by architecture and threading mode, against fakes:
# a Graviton's /sys (16 CPUs, each a core), an AWS x86's (8 cores x 2, twins N
# and N+8) and an Azure x86's (4 cores x 2, twins adjacent); a DPDK driver's
# log and memory map; IOMMU groups; transparent hugepage sizes.
sys=$tmp/sys
fake_cpus() { # <root> <cpus> <siblings of cpu N, from N and the cpu count>
    local cpu
    for cpu in $(seq 0 $(($2 - 1))); do
        mkdir -p "$1/devices/system/cpu/cpu$cpu/topology"
        eval "echo \"$3\"" >"$1/devices/system/cpu/cpu$cpu/topology/thread_siblings_list"
    done
}
# each CPU's siblings, from its number when written
# shellcheck disable=SC2016
fake_cpus "$sys/graviton" 16 '$cpu'
# shellcheck disable=SC2016
fake_cpus "$sys/aws-x86" 16 '$((cpu % 8)),$((cpu % 8 + 8))'
# shellcheck disable=SC2016
fake_cpus "$sys/azure-x86" 8 '$((cpu / 2 * 2))-$((cpu / 2 * 2 + 1))'
mkdir -p "$sys/graviton/devices/system/cpu/cpufreq" # not a CPU
pci=$sys/graviton/bus/pci/devices
mkdir -p "$pci/0000:00:06.0" "$pci/0000:00:07.0" "$pci/0000:00:08.0" "$sys/graviton/kernel/iommu_groups/5" "$sys/graviton/kernel/iommu_groups/6"
echo DMA-FQ >"$sys/graviton/kernel/iommu_groups/5/type"
echo identity >"$sys/graviton/kernel/iommu_groups/6/type"
ln -s ../../../../kernel/iommu_groups/5 "$pci/0000:00:07.0/iommu_group"
ln -s ../../../../kernel/iommu_groups/6 "$pci/0000:00:08.0/iommu_group"
thp=$sys/graviton/kernel/mm/transparent_hugepage
for size in 16 64 2048; do mkdir -p "$thp/hugepages-${size}kB"; done
echo 'always inherit madvise [never]' >"$thp/hugepages-16kB/enabled"
echo '[always] inherit madvise never' >"$thp/hugepages-64kB/enabled"
echo 'always inherit madvise [never]' >"$thp/hugepages-2048kB/enabled"
ena_log() { # <placement policy>...: a DPDK driver's log, one ENA port a policy
    echo "transport backend: dpdk-ena"
    for p in "$@"; do
        printf 'ENA_DRIVER: eth_ena_dev_init(): LLQ entry size 256B\nENA_DRIVER: eth_ena_dev_init(): Placement policy: %s\n' "$p"
        echo 'ENA_DRIVER: ena_calc_io_queue_size(): tx queue size 512'
    done
}
ena_log 'Low latency' 'Low latency' >"$tmp/llq-ok.log"
ena_log 'Low latency' 'Regular' >"$tmp/llq-regular.log"
echo "transport backend: dpdk-ena" >"$tmp/llq-none.log"
# one port's line only: the other port's queues are not known to be in LLQ
ena_log 'Low latency' >"$tmp/llq-one.log"
for bdf in 0000:00:06.0 0000:00:07.0; do
    printf '1100800000-1100802000 rw-s 00000000 00:05 1 /sys/devices/pci0000:00/%s/resource0\n' "$bdf"
    printf '1100804000-1100904000 rw-s 00000000 00:05 2 /sys/devices/pci0000:00/%s/resource2_wc\n' "$bdf"
done >"$tmp/maps-wc"
sed 's/07.0\/resource2_wc/07.0\/resource2/' "$tmp/maps-wc" >"$tmp/maps-uc"
host=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    echo "arch:$(rc host_arch_ok x86_64 '')$(rc host_arch_ok aarch64 '')$(rc host_arch_ok aarch64 1)$(rc host_arch_ok armv7l 1)"
    echo "debian: $(debian_arch x86_64) $(debian_arch aarch64)$(rc debian_arch riscv64)"
    echo "build env: [$(build_env aarch64)] [$(build_env x86_64)]"
    for root in graviton aws-x86 azure-x86; do
        # host.sh reads it
        # shellcheck disable=SC2034
        LAB_SYS_ROOT=$sys/$root
        echo "layout $root: $(cpu_layout '') / noisy 2 9: $(cpu_layout '2 9')"
    done
    echo "boot: [$(boot_args aarch64 aws)] [$(boot_args x86_64 aws)] [$(boot_args aarch64 azure)]"
    echo "isolate x86 aws: $(isolation_args x86_64 aws 15)"
    echo "isolate x86 azure: $(isolation_args x86_64 azure 7)"
    echo "isolate arm: $(isolation_args aarch64 aws 15)"
    echo "isolate arm poll: $(isolation_args aarch64 aws 15 poll)"
    echo "isolate x86 poll: $(isolation_args x86_64 aws 15 poll)"
    echo "isolate bad:$(rc isolation_args aarch64 aws 15 nohz)"
    for mode in DEDICATED SHARED_NETWORK SHARED INVOKER; do
        echo "threads $mode:"
        threading_env "$mode" 0 2 3
    done
    echo "threads refused:$(rc threading_env shared 0 2 3)$(rc threading_env SHARED 0 '' '')$(rc threading_env SHARED_NETWORK 0 '' 3)$(rc threading_env INVOKER '' '' '')"
    echo "threading of: [$(threading_of 'RX_RING=4096@AERON_THREADING_MODE=SHARED')] [$(threading_of 'RX_RING=4096')]$(rc threading_of 'AERON_THREADING_MODE=SHARED@AERON_THREADING_MODE=DEDICATED')"
    # shellcheck disable=SC2034
    LAB_SYS_ROOT=$sys/graviton
    echo "llq:$(rc ena_llq_check "$tmp/llq-ok.log" "$tmp/maps-wc" 0000:00:06.0 0000:00:07.0)$(rc ena_llq_check "$tmp/llq-regular.log" "$tmp/maps-wc" 0000:00:06.0 0000:00:07.0)$(rc ena_llq_check "$tmp/llq-none.log" "$tmp/maps-wc" 0000:00:06.0 0000:00:07.0)$(rc ena_llq_check "$tmp/llq-ok.log" "$tmp/maps-uc" 0000:00:06.0 0000:00:07.0)$(rc ena_llq_check "$tmp/llq-one.log" "$tmp/maps-wc" 0000:00:06.0 0000:00:07.0)"
    ena_llq_check "$tmp/llq-ok.log" "$tmp/maps-uc" 0000:00:06.0 0000:00:07.0 | sed 's/^/llq uc: /'
    echo "iommu:$(rc iommu_passthrough_ok 0000:00:06.0 0000:00:08.0)$(rc iommu_passthrough_ok 0000:00:06.0 0000:00:07.0)"
    mthp_changes 2048 | sed "s|$thp/||"
) || true
want="arch: 0 2 0 2
debian: amd64 arm64 1
build env: [export CFLAGS='-mcpu=native' RUSTFLAGS='-Ctarget-cpu=native'] []
layout graviton: 0 1 2 3 4 / noisy 2 9: 0 1 3 4 5
layout aws-x86: 0 1 2 3 4 / noisy 2 9: 0 3 4 5 6
layout azure-x86: 0 2 4 6 / noisy 2 9: 0 4 6 2
boot: [iommu.passthrough=1] [] []
isolate x86 aws: isolcpus=nohz,domain,managed_irq,1-15 nohz_full=1-15 rcu_nocbs=1-15 irqaffinity=0 iommu=pt
isolate x86 azure: isolcpus=nohz,domain,managed_irq,1-7 nohz_full=1-7 rcu_nocbs=1-7 irqaffinity=0
isolate arm: isolcpus=nohz,domain,managed_irq,1-15 nohz_full=1-15 rcu_nocbs=1-15 irqaffinity=0
isolate arm poll: isolcpus=nohz,domain,managed_irq,1-15 nohz_full=1-15 rcu_nocbs=1-15 irqaffinity=0 nohlt
isolate x86 poll: isolcpus=nohz,domain,managed_irq,1-15 nohz_full=1-15 rcu_nocbs=1-15 irqaffinity=0 iommu=pt idle=poll
isolate bad: 2
threads DEDICATED:
export AERON_THREADING_MODE='DEDICATED'
export AERON_CONDUCTOR_CPU_AFFINITY='0'
export AERON_SENDER_CPU_AFFINITY='2'
export AERON_RECEIVER_CPU_AFFINITY='3'
export AERON_SENDER_IDLE_STRATEGY='noop'
export AERON_RECEIVER_IDLE_STRATEGY='noop'
threads SHARED_NETWORK:
export AERON_THREADING_MODE='SHARED_NETWORK'
export AERON_CONDUCTOR_CPU_AFFINITY='0'
export AERON_SENDER_CPU_AFFINITY='2'
export AERON_SHAREDNETWORK_IDLE_STRATEGY='noop'
threads SHARED:
export AERON_THREADING_MODE='SHARED'
export AERON_CONDUCTOR_CPU_AFFINITY='2'
export AERON_SHARED_IDLE_STRATEGY='noop'
threads INVOKER:
export AERON_THREADING_MODE='INVOKER'
export RUSTERON_DPDK_HOUSEKEEPING_CPU='0'
threads refused: 2 2 2 2
threading of: [SHARED] [] 2
llq: 0 1 1 1 1
llq uc: llq 2 of 2 ports Low latency; write-combining: 0000:00:06.0 resource2_wc, 0000:00:07.0 NOT (resource2)
iommu: 0 1
hugepages-2048kB/enabled inherit
hugepages-64kB/enabled never"
if [[ $host != "$want" ]]; then
    fail "host.sh against fakes"
    diff -u <(echo "$want") <(echo "$host") >&2 || true
else
    pass "host.sh against fakes: architecture, layout, kernel command line, threading modes, LLQ, IOMMU, THP sizes"
fi

# host.sh, INVOKER's and the shared modes' checks against fakes:
#  * the shared agent's thread on its CPU (vm.sh, at each driver's start):
#    SHARED's ("[conductor, sender, receiver]", the kernel's 15 characters of
#    it, else the media driver's main thread) and SHARED_NETWORK's ("[sender,
#    receiver]") on the sender CPU host.sh gave it, refused anywhere else or
#    missing; DEDICATED unchecked here;
#  * an embedded driver's CnC file named by its pid and start time (Aeron's
#    CnC metadata), the ledger vm.sh loss keeps of each embedded driver's loss
#    counters (the last read of each, summed, so the counters grow across
#    clients as one driver's would, with how many drivers it holds), and a
#    client refused while the last client's driver was never read (its loss
#    would be lost);
#  * SMT=off where smt/control allows it only (x86: on or off; not
#    notsupported, forceoff, notimplemented, nor arm64), and the core layout
#    unchanged with the siblings offline (their topology gone)
proc_threads=$tmp/proc-threads
mkdir -p "$proc_threads"
fake_thread() { # <pid> <tid> <comm> <allowed CPUs>
    mkdir -p "$proc_threads/$1/task/$2"
    echo "$3" >"$proc_threads/$1/task/$2/comm"
    printf 'Name:\t%s\nCpus_allowed_list:\t%s\n' "$3" "$4" >"$proc_threads/$1/task/$2/status"
}
fake_thread 100 100 media_driver 0
fake_thread 100 101 '[conductor, sen' 2
fake_thread 200 200 media_driver 2
fake_thread 300 300 media_driver 0
fake_thread 400 400 media_driver 0
fake_thread 400 401 '[sender, receiv' 2
fake_thread 500 500 media_driver 0
fake_thread 500 501 '[sender, receiv' 0
fake_thread 600 600 media_driver 0
fake_thread 600 601 conductor 0
python3 - "$tmp/cnc.dat" <<'PY'
import struct, sys
# Aeron's CnC metadata (pack 4): version, five buffer lengths, client liveness,
# start timestamp (ms), pid, page size
open(sys.argv[1], "wb").write(struct.pack("<6iqqqi", 0x200, 1, 2, 3, 4, 5, 10_000_000_000, 1792032877123, 4242, 4096) + bytes(64))
PY
shared=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    echo "agent:$(rc shared_agent_check SHARED "$proc_threads/100" 2)$(rc shared_agent_check SHARED "$proc_threads/200" 2)$(rc shared_agent_check SHARED "$proc_threads/300" 2)$(rc shared_agent_check SHARED_NETWORK "$proc_threads/400" 2)$(rc shared_agent_check SHARED_NETWORK "$proc_threads/500" 2)$(rc shared_agent_check SHARED_NETWORK "$proc_threads/600" 2)$(rc shared_agent_check DEDICATED "$proc_threads/600" 2)"
    shared_agent_check SHARED_NETWORK "$proc_threads/500" 2
    shared_agent_check SHARED_NETWORK "$proc_threads/600" 2
    echo "cnc: $(cnc_identity "$tmp/cnc.dat")$(rc cnc_identity "$tmp/no-cnc.dat")"
    ledger=$tmp/ledger
    rm -rf "$ledger"
    echo "ledger empty: $(invoker_ledger_total "$ledger")"
    invoker_ledger_record "$ledger" 4242-1 "naks-sent=1;short-sends=0;dpdk-nobufs=0"
    echo "ledger one: $(invoker_ledger_total "$ledger")"
    invoker_ledger_record "$ledger" 4242-1 "naks-sent=2;short-sends=0;dpdk-nobufs=0"
    invoker_ledger_record "$ledger" 4300-2 "naks-sent=5;short-sends=1;dpdk-nobufs=0"
    echo "ledger two: $(invoker_ledger_total "$ledger")"
    invoker_ledger_record "$ledger" 4400-3 "aeron-read-failed=1"
    echo "ledger failed read: $(invoker_ledger_total "$ledger")"
    echo "ready:$(rc invoker_client_ready "$tmp/no-cnc.dat" "$ledger")$(rc invoker_client_ready "$tmp/cnc.dat" "$ledger")"
    invoker_client_ready "$tmp/cnc.dat" "$ledger"
    invoker_ledger_record "$ledger" 4242-1792032877123 "naks-sent=0"
    echo "ready once read:$(rc invoker_client_ready "$tmp/cnc.dat" "$ledger")"
    echo "smt:$(rc smt_check x86_64 on)$(rc smt_check x86_64 off)$(rc smt_check x86_64 notsupported)$(rc smt_check x86_64 forceoff)$(rc smt_check x86_64 notimplemented)$(rc smt_check aarch64 on)$(rc smt_check x86_64 '')"
) || true
want="agent: 0 0 2 0 2 2 0
SHARED_NETWORK: its shared agent ([sender, receiv/501) may run on 0, not on the sender CPU 2 alone
SHARED_NETWORK: no thread of its shared agent ([sender, receiv) in the media driver
cnc: 4242-1792032877123 1
ledger empty: invoker-ledger=0
ledger one: invoker-ledger=1;naks-sent=1;short-sends=0;dpdk-nobufs=0
ledger two: invoker-ledger=2;naks-sent=7;short-sends=1;dpdk-nobufs=0
ledger failed read: invoker-ledger=3;naks-sent=7;short-sends=1;dpdk-nobufs=0;aeron-read-failed=1
ready: 0 1
the last client's embedded driver (CnC 4242-1792032877123) was never read by vm.sh loss: its loss counters would be lost
ready once read: 0
smt: 0 0 2 2 2 2 2"
if [[ $shared != "$want" ]]; then
    fail "host.sh: shared agents' CPUs, embedded drivers' ledger, SMT"
    diff -u <(echo "$want") <(echo "$shared") >&2 || true
else
    pass "host.sh: the shared agent on its CPU (fail closed), embedded drivers' CnC ledger and client gate, SMT=off where allowed"
fi
# host.sh invoker_llq_probe, INVOKER under DPDK on Graviton (vm.sh driver):
# ENA's LLQ and write-combining check needs a driver process's log and
# memory map, which no embedded driver leaves, so the bundled driver starts
# once with the embedded drivers' environment (INVOKER on the app CPU, its
# idle strategy noop, as lab.rs embeds it; started on housekeeping's), is
# checked, and stops whatever the check found: 0 only when every port is in
# LLQ and mapped write-combining, 1 otherwise or when it never starts. A fake
# sudo stands in for the driver: the PMD's lines, then "media driver started"
# (unless FAKE_FAILS), alive until pkill; and for the read of its map.
llqprobe=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    sleep() { /bin/sleep 0.02; }
    pgrep() { echo 4242; }
    sudo() {
        case $1 in
        bash)
            [[ $* == *media_driver* ]] || return 1
            printf '%s\n' "$3" >"$tmp/probe-cmd"
            echo "$BASHPID" >"$tmp/probe-pid"
            cat "$FAKE_PMD"
            if [[ -n ${FAKE_FAILS:-} ]]; then echo "DPDK native error"; return 1; fi
            echo "media driver started"
            exec /bin/sleep 30
            ;;
        cat) [[ $2 == /proc/4242/maps ]] && cat "$FAKE_MAPS" ;;
        pkill) [[ $* == *media_driver* ]] && kill "$(cat "$tmp/probe-pid")" 2>/dev/null ;;
        esac
        return 0
    }
    probe() { # <pmd log> <maps> [FAKE_FAILS]: rc, then whether the stand-in is gone
        FAKE_PMD=$1 FAKE_MAPS=$2 FAKE_FAILS=${3:-}
        rm -f "$tmp/probe-pid" "$tmp/probe-cmd"
        rc invoker_llq_probe "$tmp/probe.env" 0 3 /home/admin/ergon/target/release/media_driver \
            "$tmp/probe.log" "$tmp/probe.maps" 0000:00:06.0 0000:00:07.0
        kill -0 "$(cat "$tmp/probe-pid" 2>/dev/null || echo 999999)" 2>/dev/null && echo -n " (still running)"
    }
    echo "probe:$(probe "$tmp/llq-ok.log" "$tmp/maps-wc")$(probe "$tmp/llq-regular.log" "$tmp/maps-wc")$(probe "$tmp/llq-ok.log" "$tmp/maps-uc")$(probe "$tmp/llq-ok.log" "$tmp/maps-wc" fails)"
    probe "$tmp/llq-ok.log" "$tmp/maps-wc" >/dev/null
    echo "command: $(cat "$tmp/probe-cmd")"
    grep -c 'Placement policy: Low latency' "$tmp/probe.log"
) || true
want="probe: 0 1 1 1
command: . '$tmp/probe.env' && export AERON_CONDUCTOR_CPU_AFFINITY='3' AERON_SHARED_IDLE_STRATEGY='noop' && exec taskset -c 0 '/home/admin/ergon/target/release/media_driver'
2"
if [[ $llqprobe != "$want" ]]; then
    fail "host.sh: INVOKER's LLQ probe on Graviton"
    diff -u <(echo "$want") <(echo "$llqprobe") >&2 || true
else
    pass "host.sh: INVOKER under DPDK on Graviton: a driver started with the embedded drivers' environment, LLQ and write-combining checked (fail closed), then stopped"
fi
# the layout with SMT off: AWS x86's siblings (8-15) offline, their topology
# gone; Azure's twins (adjacent) likewise: the same cores, so the same layout,
# read without a word on stderr (every vm.sh call reads it)
for root in aws-x86 azure-x86; do
    cp -R "$sys/$root" "$sys/$root-smtoff"
done
for cpu in $(seq 8 15); do rm -rf "$sys/aws-x86-smtoff/devices/system/cpu/cpu$cpu/topology"; done
for cpu in 1 3 5 7; do rm -rf "$sys/azure-x86-smtoff/devices/system/cpu/cpu$cpu/topology"; done
layouts=$(
    # shellcheck source=../host.sh
    . "$cloud/host.sh"
    for root in aws-x86 aws-x86-smtoff azure-x86 azure-x86-smtoff; do
        # shellcheck disable=SC2034
        LAB_SYS_ROOT=$sys/$root
        echo "$root: $(cpu_layout '' 2>&1)"
    done
)
if [[ $layouts == $'aws-x86: 0 1 2 3 4\naws-x86-smtoff: 0 1 2 3 4\nazure-x86: 0 2 4 6\nazure-x86-smtoff: 0 2 4 6' ]]; then
    pass "host.sh: the core layout unchanged with the SMT siblings offline"
else
    fail "host.sh: layouts with SMT off: $layouts"
fi
# A four-core instance, the smallest a cross-host run needs: c7i.2xlarge (4
# cores x 2, AWS's twins N and N+4; with SMT off the twins offline, each
# core's siblings itself) and c8g.xlarge or c9g.xlarge (4 cores, no SMT).
# Their layout (housekeeping, app, sender, receiver: no fifth core, so no
# localhost run, no second flow and no spare core), each Aeron threading
# mode's CPUs on it and the CPU @IRQ=receiver names (DEDICATED's receiver;
# the shared modes' one network agent, on the sender's core; INVOKER's each
# client's own thread, on its app CPU); the diagnostics' gap spinner's CPU
# (outside the layout: the first core it leaves, else housekeeping's twin,
# else none, never a core of the layout, as housekeeping's own was on a
# four-core Graviton); and which CPUs SMT=off takes offline (vm.sh stops the
# spinner for the mode when its CPU is one). Bigger hosts as before.
# shellcheck disable=SC2016
fake_cpus "$sys/c7i-2xl" 8 '$((cpu % 4)),$((cpu % 4 + 4))'
# shellcheck disable=SC2016
fake_cpus "$sys/c8g-xl" 4 '$cpu'
cp -R "$sys/c7i-2xl" "$sys/c7i-2xl-smtoff"
for cpu in 4 5 6 7; do rm -rf "$sys/c7i-2xl-smtoff/devices/system/cpu/cpu$cpu/topology"; done
for cpu in 0 1 2 3; do echo "$cpu" >"$sys/c7i-2xl-smtoff/devices/system/cpu/cpu$cpu/topology/thread_siblings_list"; done
four=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    for root in c7i-2xl c7i-2xl-smtoff c8g-xl aws-x86 graviton azure-x86; do
        # shellcheck disable=SC2034
        LAB_SYS_ROOT=$sys/$root
        read -r hk app sender receiver app2 <<<"$(cpu_layout '')"
        echo "$root: layout $hk $app $sender $receiver fifth ${app2:-none} spinner $(diag_spare_cpu $hk $app $sender $receiver $app2 || echo none)"
        [[ $root == c7i-2xl* || $root == c8g-xl ]] || continue
        for mode in DEDICATED SHARED_NETWORK SHARED INVOKER; do
            echo "  $mode: $(threading_env "$mode" "$hk" "$sender" "$receiver" |
                sed -nE "s/^export (AERON_[A-Z]+_CPU_AFFINITY|RUSTERON_DPDK_HOUSEKEEPING_CPU)='(.*)'$/\1=\2/p" | tr '\n' ' ')irq hk $(setting_cpu hk "$mode" "$hk" "$app" "$sender" "$receiver" "$app2") receiver $(setting_cpu receiver "$mode" "$hk" "$app" "$sender" "$receiver" "$app2") spare$(rc setting_cpu spare "$mode" "$hk" "$app" "$sender" "$receiver" "$app2")"
        done
    done
    setting_cpu spare DEDICATED 0 1 2 3 ''
    echo "spare on five cores: $(setting_cpu spare SHARED 0 1 2 3 4) refused:$(rc setting_cpu all DEDICATED 0 1 2 3 4)$(rc setting_cpu receiver BOGUS 0 1 2 3 4)"
    LAB_SYS_ROOT=$sys/c7i-2xl
    echo "smt off takes:$(for c in 0 1 2 3 4 5 6 7; do smt_takes_offline "$c" && echo -n " $c"; done)"
    LAB_SYS_ROOT=$sys/c8g-xl
    echo "smt off takes on Graviton:$(for c in 0 1 2 3; do smt_takes_offline "$c" && echo -n " $c"; done)"
) || true
want="c7i-2xl: layout 0 1 2 3 fifth none spinner 4
  DEDICATED: AERON_CONDUCTOR_CPU_AFFINITY=0 AERON_SENDER_CPU_AFFINITY=2 AERON_RECEIVER_CPU_AFFINITY=3 irq hk 0 receiver 3 spare 2
  SHARED_NETWORK: AERON_CONDUCTOR_CPU_AFFINITY=0 AERON_SENDER_CPU_AFFINITY=2 irq hk 0 receiver 2 spare 2
  SHARED: AERON_CONDUCTOR_CPU_AFFINITY=2 irq hk 0 receiver 2 spare 2
  INVOKER: RUSTERON_DPDK_HOUSEKEEPING_CPU=0 irq hk 0 receiver 1 spare 2
c7i-2xl-smtoff: layout 0 1 2 3 fifth none spinner none
  DEDICATED: AERON_CONDUCTOR_CPU_AFFINITY=0 AERON_SENDER_CPU_AFFINITY=2 AERON_RECEIVER_CPU_AFFINITY=3 irq hk 0 receiver 3 spare 2
  SHARED_NETWORK: AERON_CONDUCTOR_CPU_AFFINITY=0 AERON_SENDER_CPU_AFFINITY=2 irq hk 0 receiver 2 spare 2
  SHARED: AERON_CONDUCTOR_CPU_AFFINITY=2 irq hk 0 receiver 2 spare 2
  INVOKER: RUSTERON_DPDK_HOUSEKEEPING_CPU=0 irq hk 0 receiver 1 spare 2
c8g-xl: layout 0 1 2 3 fifth none spinner none
  DEDICATED: AERON_CONDUCTOR_CPU_AFFINITY=0 AERON_SENDER_CPU_AFFINITY=2 AERON_RECEIVER_CPU_AFFINITY=3 irq hk 0 receiver 3 spare 2
  SHARED_NETWORK: AERON_CONDUCTOR_CPU_AFFINITY=0 AERON_SENDER_CPU_AFFINITY=2 irq hk 0 receiver 2 spare 2
  SHARED: AERON_CONDUCTOR_CPU_AFFINITY=2 irq hk 0 receiver 2 spare 2
  INVOKER: RUSTERON_DPDK_HOUSEKEEPING_CPU=0 irq hk 0 receiver 1 spare 2
aws-x86: layout 0 1 2 3 fifth 4 spinner 5
graviton: layout 0 1 2 3 fifth 4 spinner 5
azure-x86: layout 0 2 4 6 fifth none spinner 1
spare: no fifth core on this host (its layout 0 1 2 3: four cores), so no spare one: a host with five cores or more (c7i.4xlarge, c8g.2xlarge and up)
spare on five cores: 4 refused: 2 2
smt off takes: 4 5 6 7
smt off takes on Graviton:"
if [[ $four != "$want" ]]; then
    fail "host.sh on four cores: layouts, threading modes, IRQ CPUs, the gap spinner's CPU, SMT off"
    diff -u <(echo "$want") <(echo "$four") >&2 || true
else
    pass "host.sh on four cores (c7i.2xlarge, SMT on and off; c8g.xlarge): the layout, every threading mode's CPUs, @IRQ's CPU, no spare core, the gap spinner never on a layout core"
fi

# host.sh, the NAPI settings (vm.sh driver, kernel modes): @NAPI_DEFER's
# values (napi_defer_hard_irqs, at most S32_MAX, and gro_flush_timeout in ns,
# a slash between); @NAPI_BUSY's kernel (Linux 6.17 and up: threaded busy
# polling, set per NAPI over netlink) and the kernel's own ynl, here a fake
# answering as ynl does (Python literals; JSON with --output-json): every
# NAPI of the data NICs listed by ifindex (napi-get dump), each set to
# threaded busy-poll (napi-set), its kthread's pid read back (napi-get) and
# pinned to the CPU given, one line a NAPI; a NAPI the kernel refuses, a
# kthread with no pid or one taskset cannot move fails it (1), listing what
# it set so far; and each set back to threaded disabled
napi=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    mkdir -p "$tmp/napi-state"
    sudo() { "$@"; }
    taskset() { echo "taskset $*" >>"$tmp/napi-calls"; [[ -z ${FAKE_TASKSET_FAIL:-} ]]; }
    ynl_fake() {
        echo "ynl $*" >>"$tmp/napi-calls"
        python3 - "$tmp/napi-state" "$@" <<'PY'
import json, os, pprint, sys
state, args = sys.argv[1], sys.argv[2:]
json_out = "--output-json" in args
args = [a for a in args if a != "--output-json"]
op, verb, req = args[0], args[1], json.loads(args[args.index("--json") + 1])
napis = {8193: 2, 8194: 2, 8195: 3, 8196: 3}  # id: ifindex
def show(i):
    path = os.path.join(state, str(i))
    threaded = open(path).read().strip() if os.path.exists(path) else "disabled"
    d = {"defer-hard-irqs": 0, "gro-flush-timeout": 0, "id": i, "ifindex": napis[i], "irq": 25 + i - 8193,
         "irq-suspend-timeout": 0, "threaded": threaded}
    if threaded != "disabled" and str(i) != os.environ.get("FAKE_NO_PID"):
        d["pid"] = 900 + i - 8193
    return d
out = None
if (op, verb) == ("--dump", "napi-get"):
    out = [show(i) for i, ix in napis.items() if ix == req["ifindex"]]
elif (op, verb) == ("--do", "napi-get"):
    out = show(req["id"])
elif (op, verb) == ("--do", "napi-set"):
    if str(req["id"]) == os.environ.get("FAKE_REFUSE"):
        sys.exit("Netlink error: Invalid argument")
    open(os.path.join(state, str(req["id"])), "w").write(req["threaded"] + "\n")
if out is not None:
    print(json.dumps(out) if json_out else pprint.pformat(out, compact=True))
PY
    }
    ynl() { ynl_fake "$@"; }
    echo "kernel:$(rc kernel_at_least 7.2.6+deb13-cloud-amd64 6 17)$(rc kernel_at_least 6.17+bpo-cloud-arm64 6 17)$(rc kernel_at_least 6.16.3+deb13-cloud-amd64 6 17)$(rc kernel_at_least 6.12.43+deb13-cloud-arm64 6 17)$(rc kernel_at_least 10.1.0 6 17)$(rc kernel_at_least x 6 17)"
    echo "defer: [$(napi_defer_args 2/200000)] [$(napi_defer_args 0/0)]$(rc napi_defer_args 2)$(rc napi_defer_args 2,200000)$(rc napi_defer_args 2147483648/1)$(rc napi_defer_args /5)"
    echo "napis: $(napi_list 2 3 | tr '\n' ';')$(rc napi_list 9)"
    : >"$tmp/napi-calls"
    napi_busy_on 4 2 3
    echo "on: $? threaded $(cat "$tmp/napi-state"/* | paste -sd' ' -)"
    grep -c 'napi-set' "$tmp/napi-calls"
    grep '^taskset' "$tmp/napi-calls" | tr '\n' ';'
    echo
    napi_busy_off 8193 8194 8195 8196
    echo "off: $? threaded $(cat "$tmp/napi-state"/* | paste -sd' ' -)"
    FAKE_REFUSE=8195 napi_busy_on 4 2 3
    echo "refused: $?"
    FAKE_NO_PID=8194 napi_busy_on 4 2
    echo "no pid: $?"
    FAKE_TASKSET_FAIL=1 napi_busy_on 4 3
    echo "taskset: $?"
    napi_busy_off 8193 8194 8195 8196 >/dev/null
    # ynl's JSON replies (--output-json) read alike
    ynl() { ynl_fake --output-json "$@"; }
    echo "json: $(napi_list 3 | tr '\n' ';')"
) || true
want="kernel: 0 0 1 1 0 1
defer: [2 200000] [0 0] 2 2 2 2
napis: 2 8193;2 8194;3 8195;3 8196; 1
napi 8193 ifindex 2 pid 900 cpu 4
napi 8194 ifindex 2 pid 901 cpu 4
napi 8195 ifindex 3 pid 902 cpu 4
napi 8196 ifindex 3 pid 903 cpu 4
on: 0 threaded busy-poll busy-poll busy-poll busy-poll
4
taskset -pc 4 900;taskset -pc 4 901;taskset -pc 4 902;taskset -pc 4 903;
off: 0 threaded disabled disabled disabled disabled
napi 8193 ifindex 2 pid 900 cpu 4
napi 8194 ifindex 2 pid 901 cpu 4
NAPI_BUSY: napi-set of NAPI 8195 (ifindex 3) to threaded busy-poll failed: Netlink error: Invalid argument
refused: 1
napi 8193 ifindex 2 pid 900 cpu 4
napi 8194 ifindex 2 pid ? cpu ?
NAPI_BUSY: NAPI 8194 (ifindex 2) is threaded busy-poll but has no kthread pid (napi-get: threaded busy-poll)
no pid: 1
napi 8195 ifindex 3 pid ? cpu ?
NAPI_BUSY: taskset -pc 4 902 (NAPI 8195's kthread) failed
taskset: 1
json: 3 8195;3 8196;"
if [[ $napi != "$want" ]]; then
    fail "host.sh: the NAPI settings, threaded busy polling over a fake ynl"
    diff -u <(echo "$want") <(echo "$napi") >&2 || true
else
    pass "host.sh: NAPI_DEFER's values, Linux 6.17 for NAPI_BUSY, each data NIC NAPI set to threaded busy-poll and its kthread pinned (fail closed, listing what was set), then disabled"
fi

# vm.sh's own @IRQ=, @NAPI_DEFER= and @NAPI_BUSY= (host_setting, as `driver`
# applies them) and their undoing (restore_host, as `stop`, which every
# driver start runs first, does it), taken from vm.sh with its helpers and
# run against a fake /proc and /sys/class/net (sudo running what it is
# given): a mode's settings must not outlive it, or the next mode runs with
# them. IRQ=receiver (DEDICATED: CPU 3) puts the data NICs' queue IRQs there,
# every other IRQ on housekeeping's CPU 0; NAPI_DEFER=10/50000 over the
# transport's own 0 and 0; NAPI_BUSY=spare records the fifth core (4), and
# with its NAPIs set busy (napi_busy, the data NICs' threaded flags) the
# undoing sets each NAPI back to disabled and both flags to 0. After it every
# IRQ is on CPU 0, the NAPI values are 0 and 0, and nothing of the three is left.
rt=$tmp/vm-restore
mkdir -p "$rt/home/bench" "$rt/proc/irq"
for dev in ens6 ens7; do
    mkdir -p "$rt/sys/class/net/$dev"
    echo 0 >"$rt/sys/class/net/$dev/napi_defer_hard_irqs"
    echo 0 >"$rt/sys/class/net/$dev/gro_flush_timeout"
    echo 0 >"$rt/sys/class/net/$dev/threaded"
done
cat >"$rt/proc/interrupts" <<'EOF'
            CPU0       CPU1       CPU2       CPU3       CPU4
   0:         40          0          0          0          0  IR-IO-APIC    2-edge      timer
  24:         10          0          0          0          0  PCI-MSIX-0000:28:00.0    0-edge      ena-mgmnt@pci:0000:28:00.0
  25:        100          0          0          0          0  PCI-MSIX-0000:28:00.0    1-edge      ens6-Tx-Rx-0
  26:          0          0          0          0          0  PCI-MSIX-0000:28:00.0    2-edge      ens6-Tx-Rx-1
  27:          0          0          0          0          0  PCI-MSIX-0000:29:00.0    1-edge      ens7-Tx-Rx-0
  28:          0          0          0          0          0  PCI-MSIX-0000:29:00.0    2-edge      ens7-Tx-Rx-1
  29:          0          0          0          0          0  PCI-MSIX-0000:27:00.0    1-edge      ens5-Tx-Rx-0
EOF
for irq in 0 24 25 26 27 28 29; do
    mkdir -p "$rt/proc/irq/$irq"
    echo 0 >"$rt/proc/irq/$irq/smp_affinity_list"
    ln -s smp_affinity_list "$rt/proc/irq/$irq/effective_affinity_list"
done
printf '#!/bin/sh\nexit 0\n' >"$rt/home/bench/ynl"
chmod +x "$rt/home/bench/ynl"
# vm_fn <name>: a function of vm.sh's, as written there
vm_fn() { awk -v f="$1() {" '$0 == f {on = 1} on {print} on && $0 == "}" {exit}' "$cloud/vm.sh"; }
restored=$(
    set +e
    exec 2>&1
    LAB_PROC_ROOT=$rt/proc LAB_SYS_ROOT=$rt/sys
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    # shellcheck source=../irqs.sh
    . "$cloud/irqs.sh" || exit 1
    eval "$(for f in save_host pin_irqs ynl_ready host_setting restore_host; do vm_fn "$f"; done)"
    run=$rt/home/bench CLOUD=aws HK_CPU=0 APP_CPU=1 SENDER_CPU=2 RECEIVER_CPU=3 APP2_CPU=4 threading=DEDICATED
    # sudo runs what it is given in the fakes only: anything else is listed, never run
    sudo() { [[ $1 == systemctl ]] && return 0; [[ $* == *"$tmp/"* ]] || { echo "sudo $*" >>"$tmp/sudo-outside"; return 1; }; "$@"; }
    log() { :; }
    uname() { echo 7.2.6+deb13-cloud-amd64; }
    ynl() { echo "ynl $*" >>"$rt/ynl-calls"; }
    nics() { printf 'sender ens6 ens6 m6 10.30.1.4 24 0000:28:00.0\nreceiver ens7 ens7 m7 10.30.1.5 24 0000:29:00.0\n'; }
    data_ifaces() { printf 'ens6\nens7\n'; }
    affinities() { for irq in 0 24 25 26 27 28 29; do printf '%s:%s ' "$irq" "$(cat "$rt/proc/irq/$irq/smp_affinity_list")"; done; }
    napi_values() { for dev in ens6 ens7; do printf '%s:%s/%s/%s ' "$dev" $(cat "$rt/sys/class/net/$dev/"{napi_defer_hard_irqs,gro_flush_timeout,threaded}); done; }
    for setting in IRQ=receiver NAPI_DEFER=10/50000 NAPI_BUSY=spare; do
        (host_setting "${setting%%=*}" "${setting#*=}" kernel) >/dev/null 2>&1 || echo "$setting refused"
    done
    echo "set: $(affinities)| $(napi_values)| irq_cpu $(cat "$run/irq_cpu") napi_busy_cpu $(cat "$run/napi_busy_cpu")"
    # the mode's busy NAPIs, as `driver` leaves them (napi_busy_on)
    printf 'napi 8193 ifindex 2 pid 900 cpu 4\nnapi 8195 ifindex 3 pid 902 cpu 4\n' >"$run/napi_busy"
    for dev in ens6 ens7; do echo 1 >"$rt/sys/class/net/$dev/threaded"; done
    restore_host >/dev/null 2>&1
    echo "undone: $(affinities)| $(napi_values)| left: [$(cd "$run" && ls irq_cpu napi_busy napi_busy_cpu restore/* 2>/dev/null | tr '\n' ' ')]"
    sed 's/^ynl //' "$rt/ynl-calls" 2>/dev/null
    echo "sudo outside the fakes: [$(cat "$tmp/sudo-outside" 2>/dev/null)]"
)
want='set: 0:0 24:0 25:3 26:3 27:3 28:3 29:0 | ens6:10/50000/0 ens7:10/50000/0 | irq_cpu 3 napi_busy_cpu 4
undone: 0:0 24:0 25:0 26:0 27:0 28:0 29:0 | ens6:0/0/0 ens7:0/0/0 | left: []
--do napi-set --json {"id": 8193, "threaded": "disabled"}
--do napi-set --json {"id": 8195, "threaded": "disabled"}
sudo outside the fakes: []'
if [[ $restored != "$want" ]]; then
    fail "vm.sh: a mode's IRQ, NAPI_DEFER and NAPI_BUSY settings, then undone"
    diff -u <(echo "$want") <(echo "$restored") >&2 || true
else
    pass "vm.sh: @IRQ=, @NAPI_DEFER= and @NAPI_BUSY= set for a mode and undone after it (queue IRQs back on housekeeping, the NAPIs' values and threading as before, nothing left)"
fi
# SMT=off on a four-core x86 (c7i.2xlarge's fake /sys), as above: the
# diagnostics' gap spinner on housekeeping's twin, CPU 4, which goes offline,
# is stopped for the mode, the pause marked in its log for spikes.py; undone,
# SMT is back on, the resumption marked and the spinner started again there
# (on CPU 4, however slowly its record is read)
st=$tmp/vm-smt
mkdir -p "$st/home/bench" "$st/sys"
cp -R "$sys/c7i-2xl/devices" "$st/sys/"
mkdir -p "$st/sys/devices/system/cpu/smt"
echo on >"$st/sys/devices/system/cpu/smt/control"
echo 4 >"$st/home/bench/diag-spare"
echo "1792032877.000100 60" >"$st/home/bench/diag-gapspin.log"
printf '#!/bin/sh\necho "gapspin $*" >>"%s/calls"\n' "$st" >"$st/home/bench/gapspin"
chmod +x "$st/home/bench/gapspin"
smt=$(
    set +e
    exec 2>&1
    LAB_PROC_ROOT=$rt/proc LAB_SYS_ROOT=$st/sys
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    # shellcheck source=../irqs.sh
    . "$cloud/irqs.sh" || exit 1
    eval "$(for f in layout save_host pin_irqs host_setting restore_host gapspin_us; do vm_fn "$f"; done)"
    run=$st/home/bench CLOUD=aws ARCH=x86_64 HK_CPU=0 smt_calls=$st/calls
    sudo() { [[ $1 == systemctl ]] && return 0; [[ $* == *"$tmp/"* ]] || { echo "sudo $*" >>"$tmp/sudo-outside"; return 1; }; "$@"; }
    log() { :; }
    data_ifaces() { :; }
    pgrep() { [[ $* == "-x gapspin" ]]; }
    # (not $st: host_setting has a local of that name)
    pkill() { echo "pkill $*" >>"$smt_calls"; }
    # the paused spinner's CPU read slowly: the spinner must be started with
    # it, never race the file's removal (a spinner given no CPU spins CPU 0)
    cat() { [[ ${1:-} != */diag-paused ]] || command sleep 0.3; command cat "$@"; }
    (host_setting SMT off kernel) >/dev/null 2>&1 || echo "SMT=off refused"
    echo "set: smt $(cat "$st/sys/devices/system/cpu/smt/control") paused on $(cat "$run/diag-paused" 2>/dev/null)"
    restore_host >/dev/null 2>&1
    wait
    echo "undone: smt $(cat "$st/sys/devices/system/cpu/smt/control") paused file left: $(ls "$run/diag-paused" 2>/dev/null | wc -l | tr -d ' ')"
    cat "$st/calls"
    sed -E 's/ [0-9]+\.[0-9]+$/ <t>/' "$run/diag-gapspin.log"
    echo "sudo outside the fakes: [$(cat "$tmp/sudo-outside" 2>/dev/null)]"
)
want='set: smt off paused on 4
undone: smt on paused file left: 0
pkill -x gapspin
gapspin 4 50
1792032877.000100 60
# paused <t>
# resumed <t>
sudo outside the fakes: []'
if [[ $smt != "$want" ]]; then
    fail "vm.sh: SMT=off's gap spinner pause and resumption"
    diff -u <(echo "$want") <(echo "$smt") >&2 || true
else
    pass "vm.sh: SMT=off stops the gap spinner whose CPU goes offline, marks the pause for spikes.py, and starts it again after the mode"
fi
# SMT=off keeps the cores of the SMT-on layout: the pre-flight saw CPU 5,
# core 1's twin, stall, so with SMT on core 1 ranks last (0 2 3 1); once the
# twins are offline cpu_layout sees no twin and would rank it first (0 1 2 3),
# which stopped AWS round A's c7i.2xlarge pair ("the core layout changed").
# The mode runs on the SMT-on layout, kept while SMT is off and dropped with
# it; a CPU of that layout itself offline is still refused. The fake sudo
# turns the fake /sys's SMT off by pointing it at a copy without the twins.
sm=$tmp/vm-smt-noisy
mkdir -p "$sm/home/bench"
for v in on off off-bad; do
    mkdir -p "$sm/$v"
    cp -R "$sys/c7i-2xl$([[ $v == on ]] || echo -smtoff)/devices" "$sm/$v/"
    mkdir -p "$sm/$v/devices/system/cpu/smt"
done
rm -rf "$sm/off-bad/devices/system/cpu/cpu1/topology"
echo on >"$sm/on/devices/system/cpu/smt/control"
echo 5 >"$sm/home/bench/noisy_cpus"
smt_noisy() {
    local offtree=$1
    ln -sfn "$sm/on" "$sm/sys"
    (
        set +e
        exec 2>&1
        LAB_PROC_ROOT=$rt/proc LAB_SYS_ROOT=$sm/sys
        # shellcheck source=../host.sh
        . "$cloud/host.sh" || exit 1
        # shellcheck source=../irqs.sh
        . "$cloud/irqs.sh" || exit 1
        eval "$(for f in layout save_host pin_irqs host_setting restore_host; do vm_fn "$f"; done)"
        run=$sm/home/bench CLOUD=aws ARCH=x86_64 HK_CPU=0
        sudo() {
            if [[ $1 == tee && ${2:-} == */smt/control ]]; then
                local v
                v=$(command cat)
                if [[ $v == off ]]; then ln -sfn "$sm/$offtree" "$sm/sys"; else ln -sfn "$sm/on" "$sm/sys"; fi
                echo "$v" >"$sm/sys/devices/system/cpu/smt/control"
                return 0
            fi
            [[ $1 == systemctl ]] && return 0
            [[ $* == *"$tmp/"* ]] || { echo "sudo $*" >>"$tmp/sudo-outside"; return 1; }
            "$@"
        }
        log() { :; }
        data_ifaces() { :; }
        pgrep() { return 1; }
        echo "before: $(layout)"
        (host_setting SMT off kernel) >"$sm/set.log" 2>&1 || echo "SMT=off refused: $(grep -m1 'SMT=off' "$sm/set.log")"
        echo "during: $(layout) smt $(cat "$sm/sys/devices/system/cpu/smt/control")"
        restore_host >/dev/null 2>&1
        echo "after: $(layout) smt $(cat "$sm/sys/devices/system/cpu/smt/control")"
    )
}
smtn="$(smt_noisy off)
--
$(smt_noisy off-bad)
sudo outside the fakes: [$(cat "$tmp/sudo-outside" 2>/dev/null || true)]"
want='before: 0 2 3 1
during: 0 2 3 1 smt off
after: 0 2 3 1 smt on
--
before: 0 2 3 1
SMT=off refused: SMT=off: CPU 1 of the layout (0 2 3 1) went offline
during: 0 2 3 1 smt on
after: 0 2 3 1 smt on
sudo outside the fakes: []'
if [[ $smtn != "$want" ]]; then
    fail "vm.sh: SMT=off keeps the SMT-on layout"
    diff -u <(echo "$want") <(echo "$smtn") >&2 || true
else
    pass "vm.sh: SMT=off runs on the SMT-on layout (a noisy twin offline reorders nothing), refuses a layout CPU offline, and drops the kept layout with SMT"
fi

# a CPU's interrupts, by its column's name: with SMT off /proc/interrupts
# lists the online CPUs only (Azure's 0 2 4 6), so a position is not a CPU
proc_sparse=$tmp/proc-sparse
mkdir -p "$proc_sparse"
cat >"$proc_sparse/interrupts" <<'EOF'
            CPU0       CPU2       CPU4       CPU6
  24:         10         20         30         40  PCI-MSIX-0000:28:00.0    0-edge      ena-mgmnt
 LOC:        100        200        300        400   Local timer interrupts
 RES:          1          2          3          4   Rescheduling interrupts
EOF
sparse=$(
    LAB_PROC_ROOT=$proc_sparse
    # shellcheck source=../irqs.sh
    . "$cloud/irqs.sh"
    for cpu in 0 2 4 6; do echo "cpu $cpu $(irqs_on_cpu "$cpu")"; done
)
if [[ $sparse == $'cpu 0 irqs=111 timer=100 res=1\ncpu 2 irqs=222 timer=200 res=2\ncpu 4 irqs=333 timer=300 res=3\ncpu 6 irqs=444 timer=400 res=4' ]]; then
    pass "irqs.sh: a CPU's interrupts read from its own column, by name, with CPUs offline"
else
    fail "irqs.sh: per-CPU interrupts with CPUs offline: $sparse"
fi

# modes.sh, a mode's settings as lab.sh checks them before any cloud call and
# vm.sh applies them: a setting's value; the Aeron URI parameters a mode gives
# its clients' publications (PUB_PARAMS) or subscriptions (SUB_PARAMS), with
# + between parameters (no | to start a pipe in an ssh command line, no comma
# to split a results.csv row), each on its own side only and from the list
# the lab knows (Aeron ignores a parameter it does not know); the MTU a mode's
# publications use and their largest message (an override, a channel's mtu,
# jumbo frames: 8500 across inter-Region VPC peering, else 9001); and each
# mode's checks by where it runs, the architecture, the cloud and the
# instances' threads per core
modes=$(
    set +e
    exec 2>&1
    # shellcheck source=../modes.sh
    . "$cloud/modes.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    echo "params: $(channel_params sub 'rcv-wnd=16m+so-rcvbuf=16m+cc=cubic+nak-delay=2ms') / $(channel_params pub 'term-length=64m+mtu=8960+linger=0+so-sndbuf=4m')"
    echo "params refused:$(rc channel_params pub 'rcv-wnd=16m')$(rc channel_params sub 'term-length=64m')$(rc channel_params sub 'rcv-window=1m')$(rc channel_params sub 'rcv-wnd=1m|so-rcvbuf=1m')$(rc channel_params sub 'rcv-wnd')$(rc channel_params sub '')$(rc channel_params sub 'rcv-wnd=1m++cc=cubic')$(rc channel_params sub 'rcv-wnd=1m+rcv-wnd=2m')$(rc channel_params sub 'cc=cu/bic')$(rc channel_params side 'cc=cubic')"
    # media-rcv-ts-offset (a subscription's): where Aeron writes each
    # message's 8-byte receive time. reserved (the frame header's reserved
    # value, which no client reads), or a payload offset past what the
    # clients read (bytes 0 to 15: the send stamp and sequence) within rtt's
    # 32-byte message (16 to 24); nothing lower (it would overwrite them), no
    # higher (rtt's messages would go unstamped), no other spelling (-8 is
    # Aeron's own code for reserved), never on a publication; the lab's DPDK
    # stamping mode passes the pre-flight
    echo "rcv stamps: $(channel_params sub 'media-rcv-ts-offset=reserved') $(channel_params sub 'rcv-wnd=2m+media-rcv-ts-offset=16') $(channel_params sub 'media-rcv-ts-offset=24')$(rc channel_params sub 'media-rcv-ts-offset=0')$(rc channel_params sub 'media-rcv-ts-offset=8')$(rc channel_params sub 'media-rcv-ts-offset=15')$(rc channel_params sub 'media-rcv-ts-offset=25')$(rc channel_params sub 'media-rcv-ts-offset=-8')$(rc channel_params sub 'media-rcv-ts-offset=0x10')$(rc channel_params pub 'media-rcv-ts-offset=reserved')$(rc mode_check 'dpdk@SUB_PARAMS=media-rcv-ts-offset=reserved' remote arm64 aws '' cluster)"
    channel_params sub 'media-rcv-ts-offset=8'
    echo "get: [$(mode_get 'dpdk@JUMBO=1@SMT=off' JUMBO)] [$(mode_get 'dpdk@JUMBO=1@SMT=off' SMT)] [$(mode_get dpdk JUMBO)]$(rc mode_get dpdk JUMBO)"
    echo "mtu: $(mode_mtu kernel cluster) $(mode_mtu kernel@JUMBO=1 cluster) $(mode_mtu dpdk@JUMBO=1 cross-region) $(mode_mtu kernel@JUMBO=1@AERON_MTU_LENGTH=4096 cluster) $(mode_mtu 'kernel@PUB_PARAMS=mtu=1024+linger=0' cluster) $(mode_mtu 'dpdk@JUMBO=1+idle' cross-az)"
    echo "payload: $(mode_payload kernel cluster) $(mode_payload kernel@JUMBO=1 cross-region)"
    echo "jumbo: $(jumbo_mtus cluster) / $(jumbo_mtus cross-az) / $(jumbo_mtus cross-region)"
    # what the DPDK transport refuses at install, refused here first: under the
    # one-agent modes a pacing wait (RUSTERON_DPDK_TX_PACE_SPIN_US), and in
    # every mode a network agent's idle strategy that does not spin (INVOKER's
    # is the client's own: noop)
    echo "transport:$(rc mode_check dpdk@AERON_THREADING_MODE=SHARED@RUSTERON_DPDK_TX_PACE_SPIN_US=20 remote x86_64 aws '')$(rc mode_check dpdk-spin@AERON_THREADING_MODE=SHARED_NETWORK@RUSTERON_DPDK_TX_PACE_SPIN_US=1 remote x86_64 aws '')$(rc mode_check dpdk@AERON_THREADING_MODE=INVOKER@RUSTERON_DPDK_TX_PACE_SPIN_US=5 remote x86_64 aws '')$(rc mode_check dpdk@AERON_THREADING_MODE=SHARED@RUSTERON_DPDK_TX_PACE_SPIN_US=0 remote x86_64 aws '')$(rc mode_check dpdk@RUSTERON_DPDK_TX_PACE_SPIN_US=20 remote x86_64 aws '')$(rc mode_check kernel@AERON_THREADING_MODE=SHARED@RUSTERON_DPDK_TX_PACE_SPIN_US=20 remote x86_64 aws '')$(rc mode_check dpdk@AERON_THREADING_MODE=SHARED_NETWORK@AERON_SHAREDNETWORK_IDLE_STRATEGY=backoff remote x86_64 aws '')$(rc mode_check dpdk@AERON_THREADING_MODE=SHARED@AERON_SHARED_IDLE_STRATEGY=yield remote x86_64 aws '')$(rc mode_check dpdk@AERON_RECEIVER_IDLE_STRATEGY=sleep-ns remote x86_64 aws '')$(rc mode_check dpdk@AERON_THREADING_MODE=SHARED@AERON_SHARED_IDLE_STRATEGY=spin remote x86_64 aws '')$(rc mode_check dpdk@AERON_SENDER_IDLE_STRATEGY=aeron_idle_strategy_noop remote x86_64 aws '')$(rc mode_check kernel@AERON_SENDER_IDLE_STRATEGY=backoff remote x86_64 aws '')"
    # a mode's MTU (mode_mtu) refused where its frames would not get through:
    # not 32-aligned (Aeron's frame alignment); over the DPDK transport's
    # RUSTERON_DPDK_MAX_AERON_MTU (1408, jumbo frames' or the mode's own);
    # over the path: 1500-byte data NICs (kernel, without JUMBO=1; Azure),
    # inter-Region VPC peering's 8500 bytes, a VPC's 9001
    echo "mtu checks:$(rc mode_check dpdk@AERON_MTU_LENGTH=1472 remote x86_64 aws '' cluster)$(rc mode_check dpdk@AERON_MTU_LENGTH=1472@RUSTERON_DPDK_MAX_AERON_MTU=1472 remote x86_64 aws '' cluster)$(rc mode_check kernel@AERON_MTU_LENGTH=1472 remote x86_64 aws '' cluster)$(rc mode_check kernel@AERON_MTU_LENGTH=1504 remote x86_64 aws '' cluster)$(rc mode_check kernel@PUB_PARAMS=mtu=4096 remote x86_64 aws '' cluster)$(rc mode_check kernel@JUMBO=1@PUB_PARAMS=mtu=8960 remote x86_64 aws '' cross-region)$(rc mode_check dpdk@JUMBO=1@PUB_PARAMS=mtu=8448 remote x86_64 aws '' cross-region)$(rc mode_check dpdk@JUMBO=1 remote x86_64 aws '' cross-region)$(rc mode_check kernel@AERON_MTU_LENGTH=1000 remote x86_64 aws '' cluster)$(rc mode_check dpdk@RUSTERON_DPDK_MAX_AERON_MTU=4096@AERON_MTU_LENGTH=4096 remote x86_64 aws '' cluster)$(rc mode_check dpdk@RUSTERON_DPDK_MAX_AERON_MTU=4096@AERON_MTU_LENGTH=4096 remote x86_64 azure '' '')$(rc mode_check dpdk@JUMBO=1@RUSTERON_DPDK_MAX_AERON_MTU=8960@PUB_PARAMS=mtu=8960 remote x86_64 aws '' cross-region)$(rc mode_check kernel@AERON_MTU_LENGTH=big remote x86_64 aws '' cluster)$(rc mode_check dpdk@RUSTERON_DPDK_MAX_AERON_MTU=lots remote x86_64 aws '' cluster)"
    mode_check kernel@JUMBO=1@PUB_PARAMS=mtu=8960 remote x86_64 aws '' cross-region
    # Aeron's sizes (k, m, g: powers of 1024), and what Aeron refuses of a
    # subscription, or the kernel would cap unsaid: a receiver window over its
    # socket's receive buffer (so-rcvbuf; else @AERON_SOCKET_SO_RCVBUF; else
    # vm.sh's 2 MiB), and in a kernel mode a socket buffer over vm.sh's
    # net.core maximum, 16 MiB (DPDK has no socket: a number only)
    echo "sizes: $(aeron_size 4096) $(aeron_size 64k) $(aeron_size 8M) $(aeron_size 1g)$(rc aeron_size 8mb)$(rc aeron_size '')"
    echo "windows:$(rc mode_check 'kernel@SUB_PARAMS=rcv-wnd=8m' remote x86_64 aws '' cross-region)$(rc mode_check 'kernel@SUB_PARAMS=rcv-wnd=8m+so-rcvbuf=8m' remote x86_64 aws '' cross-region)$(rc mode_check 'dpdk@SUB_PARAMS=rcv-wnd=8m+so-rcvbuf=4m' remote x86_64 aws '' cross-region)$(rc mode_check 'kernel@SUB_PARAMS=rcv-wnd=8m@AERON_SOCKET_SO_RCVBUF=8388608' remote x86_64 aws '' cross-az)$(rc mode_check 'kernel@SUB_PARAMS=rcv-wnd=2m' remote x86_64 aws '' cluster)$(rc mode_check 'kernel@SUB_PARAMS=rcv-wnd=32m+so-rcvbuf=32m' remote x86_64 aws '' cluster)$(rc mode_check 'kernel@PUB_PARAMS=so-sndbuf=32m' remote x86_64 aws '' cluster)$(rc mode_check 'kernel@PUB_PARAMS=term-length=32m+so-sndbuf=8m@SUB_PARAMS=rcv-wnd=8m+so-rcvbuf=8m' remote x86_64 aws '' cross-region)$(rc mode_check 'dpdk@SUB_PARAMS=rcv-wnd=32m+so-rcvbuf=32m' remote x86_64 aws '' cluster)"
    # the data NICs' IRQs and NAPI by mode: @IRQ=hk|receiver|spare (AWS's
    # ENA queues: kernel modes on AWS only; spare, the fifth core, not where a
    # localhost run takes it), @NAPI_DEFER=<n>/<ns> (a slash: no comma),
    # @NAPI_BUSY=spare; each in kernel modes only, once at most
    echo "irq:$(rc mode_check kernel@IRQ=hk remote x86_64 aws '')$(rc mode_check kernel@IRQ=receiver remote x86_64 aws '')$(rc mode_check kernel-busy@IRQ=spare@AERON_THREADING_MODE=SHARED remote arm64 aws '')$(rc mode_check dpdk@IRQ=receiver remote x86_64 aws '')$(rc mode_check dpdk-spin@IRQ=hk remote x86_64 aws '')$(rc mode_check kernel@IRQ=cpu3 remote x86_64 aws '')$(rc mode_check kernel@IRQ=spare local x86_64 aws '')$(rc mode_check kernel@IRQ=receiver local x86_64 aws '')$(rc mode_check kernel@IRQ=receiver remote x86_64 azure '')$(rc mode_check kernel@IRQ=receiver@IRQ=hk remote x86_64 aws '')"
    echo "napi defer:$(rc mode_check kernel@NAPI_DEFER=2/200000 remote x86_64 aws '')$(rc mode_check kernel-busy@NAPI_DEFER=0/0 remote x86_64 azure '')$(rc mode_check kernel@NAPI_DEFER=2 remote x86_64 aws '')$(rc mode_check kernel@NAPI_DEFER=2,200000 remote x86_64 aws '')$(rc mode_check kernel@NAPI_DEFER=x/y remote x86_64 aws '')$(rc mode_check kernel@NAPI_DEFER=2147483648/0 remote x86_64 aws '')$(rc mode_check dpdk@NAPI_DEFER=2/200000 remote x86_64 aws '')$(rc mode_check kernel@NAPI_DEFER=1/1@NAPI_DEFER=2/2 remote x86_64 aws '')"
    echo "napi busy:$(rc mode_check kernel@NAPI_BUSY=spare remote x86_64 aws '')$(rc mode_check kernel-busy@NAPI_BUSY=spare@IRQ=spare remote arm64 aws '')$(rc mode_check dpdk@NAPI_BUSY=spare remote x86_64 aws '')$(rc mode_check kernel@NAPI_BUSY=receiver remote x86_64 aws '')$(rc mode_check kernel@NAPI_BUSY=spare local x86_64 aws '')$(rc mode_check kernel@NAPI_BUSY=spare@NAPI_BUSY=spare remote x86_64 aws '')"
    mode_check dpdk@IRQ=receiver remote x86_64 aws ''
    mode_check kernel@IRQ=spare local x86_64 aws ''
    mode_check kernel@NAPI_DEFER=2,200000 remote x86_64 aws ''
    # the spare core's users (lib.sh: no second flow there), and the modes
    # that need the kernel's ynl built (vm.sh build-ynl)
    echo "spare:$(rc mode_spare kernel@IRQ=spare)$(rc mode_spare kernel@NAPI_BUSY=spare+idle)$(rc mode_spare kernel@IRQ=receiver)$(rc mode_spare dpdk) ynl:$(rc modes_want_ynl kernel dpdk kernel@NAPI_BUSY=spare)$(rc modes_want_ynl kernel kernel@IRQ=spare)$(rc modes_want_ynl)"
    echo "checks:$(rc mode_check kernel remote x86_64 aws '')$(rc mode_check kernel@AERON_THREADING_MODE=INVOKER remote x86_64 aws '')$(rc mode_check kernel@AERON_THREADING_MODE=INVOKER local x86_64 aws '')$(rc mode_check dpdk@AERON_THREADING_MODE=INVOKER+idle remote x86_64 aws '')$(rc mode_check dpdk@AERON_THREADING_MODE=INVOKER remote arm64 aws '')$(rc mode_check kernel@AERON_THREADING_MODE=INVOKER remote arm64 aws '')$(rc mode_check dpdk@AERON_THREADING_MODE=SHARED remote x86_64 aws '')$(rc mode_check kernel@JUMBO=1 remote x86_64 azure '')$(rc mode_check kernel@JUMBO=1 remote x86_64 aws '')$(rc mode_check kernel@SMT=off remote arm64 aws '')$(rc mode_check kernel@SMT=off remote x86_64 aws 1)$(rc mode_check kernel@SMT=off remote x86_64 azure '')"
) || true
want="params: rcv-wnd=16m|so-rcvbuf=16m|cc=cubic|nak-delay=2ms / term-length=64m|mtu=8960|linger=0|so-sndbuf=4m
params refused: 2 2 2 2 2 2 2 2 2 2
rcv stamps: media-rcv-ts-offset=reserved rcv-wnd=2m|media-rcv-ts-offset=16 media-rcv-ts-offset=24 2 2 2 2 2 2 2 0
SUB_PARAMS: media-rcv-ts-offset=8: reserved (the frame header's reserved value, which no client reads) or a payload offset from 16 to 24: the clients read bytes 0 to 15 of every message (the send stamp and the sequence), which an 8-byte receive time at a lower offset would overwrite, and at a higher one rtt's 32-byte messages would go unstamped (Aeron writes none that does not fit)
get: [1] [off] [] 1
mtu: 1408 8960 8448 4096 1024 8960
payload: 1376 8416
jumbo: 9001 8960 / 9001 8960 / 8500 8448
transport: 2 2 2 0 0 0 2 2 2 0 0 0
mtu checks: 2 0 0 2 2 2 0 0 2 0 2 2 2 2
mode kernel@JUMBO=1@PUB_PARAMS=mtu=8960: MTU 8960 is over 8448, the jumbo MTU's 8500 bytes less IP and UDP: its larger frames would be dropped
sizes: 4096 65536 8388608 1073741824 2 2
windows: 2 0 2 0 0 2 2 0 0
irq: 0 0 0 2 2 2 2 0 2 2
napi defer: 0 0 2 2 2 2 2 2
napi busy: 0 0 2 2 2 2
mode dpdk@IRQ=receiver: IRQ is for kernel modes only (DPDK takes the data NICs' queues from the kernel: no interrupts)
mode kernel@IRQ=spare: IRQ=spare with both ends on one host (localhost): the run takes the fifth core for its second end
mode kernel@NAPI_DEFER=2,200000: a comma would split its results.csv rows (COALESCE takes <rx>/<tx>)
spare: 0 0 1 1 ynl: 0 1 1
checks: 0 0 2 2 0 0 0 2 0 2 2 0"
if [[ $modes != "$want" ]]; then
    fail "modes.sh: settings, URI parameters, MTUs and checks"
    diff -u <(echo "$want") <(echo "$modes") >&2 || true
else
    pass "modes.sh: settings, URI parameters by side, MTUs (jumbo 8500 across regions), each mode's checks"
fi

# host.sh's network helpers, against fakes: ping's summary as nanoseconds
# (min avg max mdev, replies of sent; nothing back is a failure); a subnet's
# gateway (AWS's VPC router, its first address); each data NIC's policy
# routes (its own subnet, and a peer's off-subnet, cross-AZ or cross-region,
# through the gateway); jumbo frames' driver environment by placement and
# transport; the path MTU check, ping with Don't Fragment at the link MTU from
# every data address to every peer's, retried while the peer is still setting
# its MTU and failing when the path drops it (inter-Region peering's 8500);
# the effective MTU a run records; and the lost round trips' timeout, which a
# long base round trip stretches
hostnet=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    sleep() { :; }
    echo "stats: $(printf 'PING 10.30.3.7 (10.30.3.7) from 10.30.1.4 : 56(84) bytes of data.\n\n--- 10.30.3.7 ping statistics ---\n200 packets transmitted, 198 received, 1%% packet loss, time 1990ms\nrtt min/avg/max/mdev = 0.045/0.061/0.212/0.012 ms\n' | ping_stats)"
    echo "stats none:$(printf '3 packets transmitted, 0 received, 100%% packet loss, time 2010ms\n' | rc ping_stats)"
    echo "gateway: $(subnet_gateway 10.30.3.6/24) $(subnet_gateway 10.31.1.7/24) $(subnet_gateway 10.20.1.4/24)"
    echo "routes:"
    policy_routes 10.30.1.4 24 ens6 100 10.30.3.0/24
    policy_routes 10.30.1.5 24 ens7 101 ''
    policy_routes 10.30.1.5 24 ens7 101 10.30.1.0/24
    echo "jumbo kernel cross-az:"
    jumbo_env cross-az kernel
    echo "jumbo dpdk cross-region:"
    jumbo_env cross-region dpdk
    # a path that carries <= FAKE_PATH_MTU bytes; ping -s is the ICMP payload
    ping() {
        local size=0 a
        echo "ping $*" >>"$tmp/pings"
        for a in "$@"; do [[ ${prev:-} == -s ]] && size=$a; prev=$a; done
        prev=
        (((size + 28) <= FAKE_PATH_MTU))
    }
    : >"$tmp/pings"
    FAKE_PATH_MTU=9001 path_mtu_check 9001 10.30.1.4 10.30.1.5 -- 10.30.3.6 10.30.3.7 | sed 's/^/pmtu ok: /'
    echo "pmtu pings: $(grep -c . "$tmp/pings") $(head -1 "$tmp/pings")"
    FAKE_PATH_MTU=8500 path_mtu_check 9001 10.30.1.4 -- 10.31.1.7 2>&1 | sed 's/^/pmtu 8500: /'
    echo "pmtu 8500 rc:$(FAKE_PATH_MTU=8500 rc path_mtu_check 9001 10.30.1.4 -- 10.31.1.7) at 8500:$(FAKE_PATH_MTU=8500 rc path_mtu_check 8500 10.30.1.4 -- 10.31.1.7)"
    printf "export AERON_MTU_LENGTH='1408'\nexport AERON_MTU_LENGTH='8960'\nexport RUSTERON_DPDK_MAX_AERON_MTU='8960'\n" >"$tmp/env-jumbo"
    printf "export AERON_MTU_LENGTH='1408'\n" >"$tmp/env-plain"
    # (a mode's PUB_PARAMS mtu is what its publications send at: it wins)
    echo "effective: [$(effective_mtu "$tmp/env-jumbo" '' 0 9001 9001)] [$(effective_mtu "$tmp/env-jumbo" '' 1)] [$(effective_mtu "$tmp/env-plain" '' 1)] [$(effective_mtu "$tmp/env-plain" '' 0 1500 9001)] [$(effective_mtu "$tmp/env-jumbo" 'term-length=32m|mtu=4096' 1)]"
    echo "lost timeout: $(rtt_lost_timeout_us 'rcv-wnd=1m' 70000) $(rtt_lost_timeout_us 'reliable=false|group=false|tether=false' '') $(rtt_lost_timeout_us 'reliable=false|group=false|tether=false' 70000) $(rtt_lost_timeout_us 'cc=cubic|reliable=false' 100)"
) || true
want="stats: 45000 61000 212000 12000 198/200
stats none: 1
gateway: 10.30.3.1 10.31.1.1 10.20.1.1
routes:
10.30.1.0/24 dev ens6 src 10.30.1.4 table 100
10.30.3.0/24 via 10.30.1.1 dev ens6 src 10.30.1.4 table 100
10.30.1.0/24 dev ens7 src 10.30.1.5 table 101
10.30.1.0/24 dev ens7 src 10.30.1.5 table 101
jumbo kernel cross-az:
export AERON_MTU_LENGTH='8960'
jumbo dpdk cross-region:
export AERON_MTU_LENGTH='8448'
export RUSTERON_DPDK_MAX_AERON_MTU='8448'
pmtu ok: pmtu 10.30.1.4 -> 10.30.3.6: 9001 bytes pass
pmtu ok: pmtu 10.30.1.4 -> 10.30.3.7: 9001 bytes pass
pmtu ok: pmtu 10.30.1.5 -> 10.30.3.6: 9001 bytes pass
pmtu ok: pmtu 10.30.1.5 -> 10.30.3.7: 9001 bytes pass
pmtu pings: 4 ping -M do -s 8973 -c 1 -W 1 -I 10.30.1.4 10.30.3.6
pmtu 8500: pmtu 10.30.1.4 -> 10.31.1.7: 9001 bytes do not pass (ping -M do -s 8973, 20 tries): the path's MTU is lower
pmtu 8500 rc: 1 at 8500: 0
effective: [9001 8960] [8988 8960] [1436 1408] [1500 1408] [8988 4096]
lost timeout: 0 50000 280000 50000"
if [[ $hostnet != "$want" ]]; then
    fail "host.sh network helpers"
    diff -u <(echo "$want") <(echo "$hostnet") >&2 || true
else
    pass "host.sh: ping summaries, gateways, policy routes to an off-subnet peer, jumbo environment, the path MTU check, the effective MTU, lost round trips' timeout"
fi

# vm.sh itself in a sandbox: fake sudo, uname, nproc and curl first on its
# PATH (sudo logs what it was given, tee's input too, and runs nothing; curl
# fetches nothing), its home in $tmp, the fake /sys's CPUs. isolate on a host
# already isolated (a login's own CPUs are housekeeping's alone: nproc 1 of
# 16) still isolates every CPU but the first, x86's line exactly the one
# before Graviton (AWS's iommu=pt, not Azure's), Graviton's without it,
# isolate poll adds idle=poll or nohlt, a bad argument writes nothing; and
# bootstrap on aarch64 without lab.sh's LAB_ALLOW_ARM=1 stops before any sudo.
sb=$tmp/vm-sandbox
mkdir -p "$sb/bin" "$sb/home/ergon" "$sb/home/bench"
cat >"$sb/bin/sudo" <<'SH'
#!/usr/bin/env bash
echo "sudo $*" >>"$FAKE_LOG"
if [[ $1 == tee ]]; then sed 's/^/  /' >>"$FAKE_LOG"; fi
exit 0
SH
# the fakes' own arguments and FAKE_* expand when they run
# shellcheck disable=SC2016
{
    printf '#!/bin/sh\n[ "$1" = -m ] && echo "$FAKE_ARCH" || echo Linux\n' >"$sb/bin/uname"
    printf '#!/bin/sh\n[ "$1" = --all ] && echo 16 || echo 1\n' >"$sb/bin/nproc"
    printf '#!/bin/sh\necho "curl $*" >>"$FAKE_LOG"\nexit 1\n' >"$sb/bin/curl"
}
chmod +x "$sb/bin/"*
# vm.sh puts the sbin directories first on its PATH: nothing there may stand
# in for a fake, or the sandbox would run the real one
# shellcheck disable=SC2030
resolved=$(PATH=/usr/local/sbin:/usr/sbin:/sbin:$sb/bin:$PATH; for c in sudo uname nproc curl; do command -v "$c"; done | tr '\n' ' ')
if [[ $resolved != "$sb/bin/sudo $sb/bin/uname $sb/bin/nproc $sb/bin/curl " ]]; then
    fail "vm.sh sandbox: a system command shadows a fake ($resolved): not run"
else
    vm_sb() { # <arch> <cloud> <sys root> <vm.sh args>...: "<rc> <sudo calls> [<the grub line written>]"
        echo "$2" >"$sb/home/bench/cloud"
        : >"$sb/log"
        local rc=0
        # shellcheck disable=SC2031
        env -i PATH="$sb/bin:$PATH" HOME="$sb/home" FAKE_LOG="$sb/log" FAKE_ARCH="$1" LAB_SYS_ROOT="$sys/$3" \
            bash "$cloud/vm.sh" "${@:4}" >"$sb/out" 2>&1 || rc=$?
        # the grub file's own $GRUB_CMDLINE_LINUX_DEFAULT, literally
        # shellcheck disable=SC2016
        echo "$rc $(grep -c '^sudo' "$sb/log") [$(sed -n 's/^  GRUB_CMDLINE_LINUX_DEFAULT="\$GRUB_CMDLINE_LINUX_DEFAULT \(.*\)"$/\1/p' "$sb/log")]"
    }
    got="x86 aws: $(vm_sb x86_64 aws aws-x86 isolate)
x86 azure: $(vm_sb x86_64 azure azure-x86 isolate)
x86 aws poll: $(vm_sb x86_64 aws aws-x86 isolate poll)
arm aws: $(vm_sb aarch64 aws graviton isolate)
arm aws poll: $(vm_sb aarch64 aws graviton isolate poll)
arm bad: $(vm_sb aarch64 aws graviton isolate nohz)
arm bootstrap without LAB_ALLOW_ARM: $(vm_sb aarch64 aws graviton bootstrap) $(grep -c 'LAB_ALLOW_ARM=1' "$sb/out" || true)"
    want="x86 aws: 0 3 [isolcpus=nohz,domain,managed_irq,1-15 nohz_full=1-15 rcu_nocbs=1-15 irqaffinity=0 iommu=pt]
x86 azure: 0 3 [isolcpus=nohz,domain,managed_irq,1-15 nohz_full=1-15 rcu_nocbs=1-15 irqaffinity=0]
x86 aws poll: 0 3 [isolcpus=nohz,domain,managed_irq,1-15 nohz_full=1-15 rcu_nocbs=1-15 irqaffinity=0 iommu=pt idle=poll]
arm aws: 0 3 [isolcpus=nohz,domain,managed_irq,1-15 nohz_full=1-15 rcu_nocbs=1-15 irqaffinity=0]
arm aws poll: 0 3 [isolcpus=nohz,domain,managed_irq,1-15 nohz_full=1-15 rcu_nocbs=1-15 irqaffinity=0 nohlt]
arm bad: 2 0 []
arm bootstrap without LAB_ALLOW_ARM: 2 0 [] 1"
    if [[ $got != "$want" ]]; then
        fail "vm.sh in a sandbox: isolate and bootstrap's architecture gate"
        diff -u <(echo "$want") <(echo "$got") >&2 || true
    else
        pass "vm.sh in a sandbox: isolate on an isolated host covers every CPU, x86's line as before, Graviton's; bootstrap refuses aarch64 without LAB_ALLOW_ARM=1"
    fi
fi

# lib.sh loss_row: an INVOKER host's loss line is its embedded drivers'
# ledger (invoker-ledger=<drivers>, then their summed counters): a counter the
# line before the run lacks (no driver read yet) grows from 0, and the marker
# is no counter; a failed Aeron read on either side of an external driver's
# run is kept (aeron-read-failed=1, loss), never dropped with the counters it
# hid; otherwise every row as before
rows=$(
    set +e
    exec 2>&1
    out=$tmp/lossrow known=$tmp/lossrow/known_hosts user=admin A=192.0.2.1 B=192.0.2.2 results=$tmp/lossrow/results.csv
    mkdir -p "$out"
    log() { :; }
    . "$cloud/lib.sh"
    loss_row 1 l1 a "invoker-ledger=0;kernel-udp-InErrors=4;routes-missing=0" \
        "invoker-ledger=1;naks-sent=3;short-sends=0;kernel-udp-InErrors=6;routes-missing=0"
    loss_row 1 l2 a "invoker-ledger=1;naks-sent=3;short-sends=0;kernel-udp-InErrors=6" "invoker-ledger=2;naks-sent=5;short-sends=0;kernel-udp-InErrors=6"
    loss_row 1 l3 b "naks-sent=1;kernel-udp-InErrors=0" "aeron-read-failed=1;kernel-udp-InErrors=0"
    loss_row 1 l4 b "aeron-read-failed=1;kernel-udp-InErrors=0" "naks-sent=9;kernel-udp-InErrors=0"
    loss_row 1 l5 b "naks-sent=1;kernel-udp-InErrors=0;routes-missing=0" "naks-sent=4;kernel-udp-InErrors=1;routes-missing=1"
    loss_row 1 l6 b "" "naks-sent=4"
    cat "$results"
) || true
want="1,loss,l1,a,naks-sent=3;short-sends=0;kernel-udp-InErrors=2;routes-missing=0
1,loss,l2,a,naks-sent=2;short-sends=0;kernel-udp-InErrors=0
1,loss,l3,b,aeron-read-failed=1;kernel-udp-InErrors=0
1,loss,l4,b,kernel-udp-InErrors=0;aeron-read-failed=1
1,loss,l5,b,naks-sent=3;kernel-udp-InErrors=1;routes-missing=1
1,loss,l6,b,invalid=1"
if [[ $rows == "$want" ]]; then
    pass "lib.sh loss_row: an INVOKER host's ledger grows from 0, a failed Aeron read is kept, the rest as before"
else
    fail "lib.sh loss_row"
    diff -u <(echo "$want") <(echo "$rows") >&2 || true
fi

# lib.sh allowance_grew: the NIC allowance counters (ENA's
# *_allowance_exceeded: bw_in, bw_out, pps, conntrack, linklocal) that grew
# between two vm.sh counters snapshots, as snap saves them around a load
# point: a kernel mode's from ethtool, a DPDK mode's from the PMD's xstats
# among the transport's counters; nothing where none grew, a snapshot is
# missing, or the two are of different embedded drivers (INVOKER's CnC).
# point_counters takes both snapshots on a point's hosts and warns, naming
# the point, of a host whose counters grew
al=$tmp/allowance
mkdir -p "$al"
printf '# realtime=1\nsender bw_in_allowance_exceeded: 0 bw_out_allowance_exceeded: 3 pps_allowance_exceeded: 0 conntrack_allowance_exceeded: 0 linklocal_allowance_exceeded: 0 tx_packets: 100 rx_packets: 50\nreceiver bw_in_allowance_exceeded: 1 bw_out_allowance_exceeded: 0 pps_allowance_exceeded: 0 conntrack_allowance_exceeded: 0 linklocal_allowance_exceeded: 0 tx_packets: 10 rx_packets: 90\n' >"$al/k-before"
printf '# realtime=2\nsender bw_in_allowance_exceeded: 0 bw_out_allowance_exceeded: 5 pps_allowance_exceeded: 2 conntrack_allowance_exceeded: 0 linklocal_allowance_exceeded: 0 tx_packets: 900 rx_packets: 50\nreceiver bw_in_allowance_exceeded: 1 bw_out_allowance_exceeded: 0 pps_allowance_exceeded: 0 conntrack_allowance_exceeded: 0 linklocal_allowance_exceeded: 0 tx_packets: 10 rx_packets: 990\n' >"$al/k-after"
printf '# realtime=1\n12 92 role=sender bw_out_allowance_exceeded;0 92 role=receiver linklocal_allowance_exceeded;1 82 rusteron-dpdk role=sender bdf=0000:00:06.0 port=0 q=0 dir=tx pkts;\n' >"$al/d-before"
printf '# realtime=2\n12 92 role=sender bw_out_allowance_exceeded;4 92 role=receiver linklocal_allowance_exceeded;900 82 rusteron-dpdk role=sender bdf=0000:00:06.0 port=0 q=0 dir=tx pkts;\n' >"$al/d-after"
printf '# realtime=1\ncnc 4242-1\n0 92 role=sender pps_allowance_exceeded;\n' >"$al/i-before"
printf '# realtime=2\ncnc 4300-2\n7 92 role=sender pps_allowance_exceeded;\n' >"$al/i-after"
grew=$(
    set +e
    exec 2>&1
    out=$al known=$al/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
    log() { :; }
    . "$cloud/lib.sh"
    echo "kernel: [$(allowance_grew "$al/k-before" "$al/k-after")]"
    echo "dpdk: [$(allowance_grew "$al/d-before" "$al/d-after")]"
    echo "none: [$(allowance_grew "$al/k-after" "$al/k-after")]"
    echo "missing: [$(allowance_grew "$al/k-before" "$al/nothing")]"
    echo "another driver: [$(allowance_grew "$al/i-before" "$al/i-after")]"
    # point_counters around one point on both hosts (snap faked: each host's
    # snapshot from the fixtures), host a's allowance counters growing
    cp "$al/k-before" "$al/a-before"
    cp "$al/k-after" "$al/a-after"
    cp "$al/k-after" "$al/b-before"
    cp "$al/k-after" "$al/b-after"
    snap() { local node; for node in $2; do cp "$al/$node-$phase" "$out/$node-counters-$1.log"; done; }
    log() { echo "log: $*"; }
    phase=before point_counters before t1 "a b"
    phase=after point_counters after t1 "a b" "isolated:kernel 288 B r1000"
    # a second point, host b's (DPDK) counters growing and host a's not
    cp "$al/k-after" "$al/a-before"
    cp "$al/d-before" "$al/b-before"
    cp "$al/d-after" "$al/b-after"
    phase=before point_counters before t2 "a b"
    phase=after point_counters after t2 "a b" "isolated:dpdk 288 B r1000"
    echo "kept: $(cd "$al" && ls ./*-counters-load-* | tr '\n' ' ')"
)
want="kernel: [sender bw_out_allowance_exceeded +2, sender pps_allowance_exceeded +2]
dpdk: [receiver linklocal_allowance_exceeded +4]
none: []
missing: []
another driver: []
log: WARNING: isolated:kernel 288 B r1000: host a's NIC allowance exceeded over the point (packets AWS shaped or dropped): sender bw_out_allowance_exceeded +2, sender pps_allowance_exceeded +2
log: WARNING: isolated:dpdk 288 B r1000: host b's NIC allowance exceeded over the point (packets AWS shaped or dropped): receiver linklocal_allowance_exceeded +4
kept: ./a-counters-load-after-t1.log ./a-counters-load-after-t2.log ./a-counters-load-before-t1.log ./a-counters-load-before-t2.log ./b-counters-load-after-t1.log ./b-counters-load-after-t2.log ./b-counters-load-before-t1.log ./b-counters-load-before-t2.log "
if [[ $grew == "$want" ]]; then
    pass "lib.sh allowance_grew: the NIC allowance counters that grew over a load point, from ethtool or the PMD's xstats; none from a missing snapshot or another embedded driver; point_counters keeps both hosts' snapshots and warns of each host whose counters grew (a, then b)"
else
    fail "lib.sh allowance_grew"
    diff -u <(echo "$want") <(echo "$grew") >&2 || true
fi

# lib.sh dry with an INVOKER mode (each client embeds the driver): every
# client's log must have the harness's line for an embedded driver run
# INVOKER ("harness driver: embedded threading=INVOKER ..."; one that ran
# against an external driver stops the run, whatever else its line says);
# several flows at once are not run there (each flow's clients would embed a
# driver of their own on one host), and said so
cat >"$tmp/dry-invoker.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
on() {
    local host=$1 cmd
    shift
    cmd=$*
    printf '%s %s\n' "$host" "$cmd" >>"$out/calls"
    case $cmd in
    *"vm.sh nics") echo "receiver ens7 ens7 0d $([[ $host == "$A" ]] && echo 10.30.1.5 || echo 10.30.1.7) 24 0000:00:07.0" ;;
    *"vm.sh layout") echo "0 2 1 3 4" ;;
    *"vm.sh loss") echo "invoker-ledger=0;naks-sent=0" ;;
    *" rtt x"* | *" tput x"*)
        # the harness's line (lab.rs Lab::connect); EXTERNAL: a client that ran
        # against an external driver, the word invoker elsewhere on its line
        if [[ -n ${EXTERNAL:-} ]]; then
            echo "harness driver: external dir=/mnt/huge/aeron-bench (HARNESS_DRIVER=invoker unread)" >&2
        else
            echo "harness driver: embedded threading=INVOKER transport=dpdk-ena cpu=2 dir=/mnt/huge/aeron-bench" >&2
        fi
        local l=${cmd#*client }
        [[ $cmd == *" tput xsub "* ]] && echo "tput,${l%% *},udp,1000,1,2,3,0,1,2"
        ;;
    esac
    return 0
}
reps=1 local_modes= load_sizes= load_fractions=50 load_seconds=1 load_ladder= mix= mix_seconds=1
tput_seconds=5 rtt_messages=10 rtt_warmup=1 rtt_align= tput_flows=2
modes=(dpdk@AERON_THREADING_MODE=INVOKER)
bench_state isolated
SH
di=$tmp/dry-invoker
mkdir -p "$di"
bash "$tmp/dry-invoker.sh" "$cloud" "$di" </dev/null 2>"$di/stderr" || fail "lib.sh dry run with INVOKER exited $?"
dis=$tmp/dry-invoker-silent
mkdir -p "$dis"
EXTERNAL=1 bash "$tmp/dry-invoker.sh" "$cloud" "$dis" </dev/null 2>"$dis/stderr" && fail "lib.sh dry: clients that did not run INVOKER did not stop the run"
if grep -E 'unbound variable|command not found|syntax error' "$di/stderr" "$dis/stderr"; then
    fail "lib.sh dry with INVOKER: shell errors"
elif grep -qE ':227[0-9][0-9] ' "$di/calls" || ! grep -q 'several flows at once.*INVOKER' "$di/log"; then
    fail "lib.sh dry with INVOKER: several flows: $(grep -iE 'flows' "$di/log")"
elif grep -q ERROR "$di/log" || ! grep -q ',tput,isolated:dpdk@AERON_THREADING_MODE=INVOKER,' "$di/results.csv"; then
    fail "lib.sh dry with INVOKER: $(tail -3 "$di/log")"
elif ! grep -q 'ERROR: .*INVOKER.*did not name' "$dis/log"; then
    fail "lib.sh dry with INVOKER, clients on an external driver: $(tail -3 "$dis/log")"
else
    pass "lib.sh dry with INVOKER: each client's log names INVOKER (the run stops otherwise), no several flows at once (said so)"
fi

# lib.sh dry with the modes that take the layout's fifth core for the data
# NICs (@IRQ=spare, @NAPI_BUSY=spare) and LAB_TPUT_FLOWS=2: their second flow
# would run its clients on that core, so neither runs several flows at once
# (said so); a mode that does not (@IRQ=receiver) runs them as ever
cat >"$tmp/dry-spare.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
on() {
    local host=$1 cmd
    shift
    cmd=$*
    printf '%s %s\n' "$host" "$cmd" >>"$out/calls"
    case $cmd in
    *"vm.sh nics") echo "receiver ens7 ens7 0d $([[ $host == "$A" ]] && echo 10.30.1.5 || echo 10.30.1.7) 24 0000:00:07.0" ;;
    *"vm.sh layout") echo "0 2 1 3 4" ;;
    *"vm.sh loss") echo "naks-sent=0" ;;
    *" tput xsub "*) local l=${cmd#*client }; echo "tput,${l%% *},udp,1000,1,2,3,0,1,2" ;;
    esac
    return 0
}
reps=1 local_modes= load_sizes= load_fractions=50 load_seconds=1 load_ladder= mix= mix_seconds=1
tput_seconds=5 rtt_messages=10 rtt_warmup=1 rtt_align= tput_flows=2
modes=(kernel@IRQ=spare kernel-busy@NAPI_BUSY=spare kernel@IRQ=receiver)
bench_state isolated
SH
ds=$tmp/dry-spare
mkdir -p "$ds"
bash "$tmp/dry-spare.sh" "$cloud" "$ds" </dev/null 2>"$ds/stderr" || fail "lib.sh dry run with the spare core's modes exited $?"
flowed=$(grep -E ':227[0-9][0-9] ' "$ds/calls" | sed 's/.*client \([^ ]*\) .*/\1/' | sort -u | tr '\n' ' ')
if grep -E 'unbound variable|command not found|syntax error' "$ds/stderr"; then
    fail "lib.sh dry with the spare core's modes: shell errors"
elif [[ $flowed != "isolated:kernel@IRQ=receiver " ]]; then
    fail "lib.sh dry with the spare core's modes: several flows at once in $flowed"
elif [[ $(grep -c 'several flows at once not run.*fifth core' "$ds/log") != 2 ]]; then
    fail "lib.sh dry with the spare core's modes: not said: $(grep -i flows "$ds/log")"
else
    pass "lib.sh dry: no second flow on the fifth core under @IRQ=spare or @NAPI_BUSY=spare (said so), as ever otherwise"
fi

# lib.sh prepare_vms, dry (every VM command faked; the checkout a git
# repository of one file): the hosts told whether to fetch the kernel's ynl
# (bench/want_ynl: a mode with NAPI_BUSY, so vm.sh build-ynl), each host
# asked after the build whether it can run every mode's settings (vm.sh
# ready: a fifth core, Linux 6.17 and that ynl), a host that cannot stopping
# the run there, before tune and any state, saying why; and the market
# (aws/lab.sh: spot) in both hosts' records
cat >"$tmp/dry-prepare.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2 repo=$out/repo data_subnet=10.30.1.0/24
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
scp() { return 0; }
on() {
    local host=$1
    shift
    printf '%s %s\n' "$host" "$*" >>"$out/calls"
    case $* in
    "cat bench/"*".rc 2>/dev/null") echo 0 ;;
    *"vm.sh ready "*)
        if [[ -n ${FAIL_READY:-} && $host == "$B" ]]; then echo "mode kernel@IRQ=spare: spare: no fifth core on this host"; return 2; fi
        ;;
    esac
    return 0
}
modes=(kernel@IRQ=spare dpdk) local_modes=kernel market=spot
if [[ -n ${YNL:-} ]]; then modes+=(kernel@NAPI_BUSY=spare); fi
if [[ -n ${DPDKB:-} ]]; then modes+=(dpdk@DPDK=patched-refill dpdk@DPDK=stock@RUSTERON_DPDK_RX_FREE_THRESH=32); fi
prepare_vms
echo "rc $?" >>"$out/calls"
SH
for case in ynl plain fail dpdk; do
    dp=$tmp/dry-prepare-$case
    mkdir -p "$dp/repo"
    git -C "$dp/repo" init -q && echo x >"$dp/repo/f"
    env YNL="$([[ $case == ynl ]] && echo 1)" FAIL_READY="$([[ $case == fail ]] && echo 1)" DPDKB="$([[ $case == dpdk ]] && echo 1)" \
        bash "$tmp/dry-prepare.sh" "$cloud" "$dp" </dev/null 2>"$dp/stderr" || true
done
order() { grep -nE 'vm.sh (ready|tune)$|vm.sh ready |cat bench/build.rc' "$1" | sed -E 's/^([0-9]+):[^ ]* .*vm.sh (ready|tune).*/\1 \2/; s/^([0-9]+):.*build.rc.*/\1 build/' | sort -n | awk '{print $2}' | uniq | tr '\n' ' '; }
dp=$tmp/dry-prepare
if grep -E 'unbound variable|command not found|syntax error' "$dp"-*/stderr; then
    fail "lib.sh prepare_vms dry: shell errors"
elif ! grep -q 'echo 1 >bench/want_ynl' "$dp-ynl/calls" || grep -q 'echo 1 >bench/want_ynl' "$dp-plain/calls"; then
    fail "lib.sh prepare_vms dry: want_ynl: $(grep -o 'echo[^&]*want_ynl' "$dp"-{ynl,plain}/calls | sort -u)"
elif [[ $(grep -c 'echo patched-refill stock >bench/want_dpdk' "$dp-dpdk/calls") != 2 || $(grep -c 'echo  >bench/want_dpdk' "$dp-plain/calls") != 2 ]]; then
    fail "lib.sh prepare_vms dry: want_dpdk: $(grep -o 'echo[^&]*want_dpdk' "$dp"-{dpdk,plain}/calls | sort -u)"
elif [[ $(grep -c 'vm.sh ready kernel@IRQ=spare dpdk kernel@NAPI_BUSY=spare kernel$' "$dp-ynl/calls") != 2 ]] ||
    [[ $(order "$dp-ynl/calls") != "build ready tune " ]] || ! grep -q '^rc 0' "$dp-ynl/calls"; then
    fail "lib.sh prepare_vms dry: the hosts' readiness: $(grep 'vm.sh ready' "$dp-ynl/calls"); order $(order "$dp-ynl/calls"); $(tail -2 "$dp-ynl/log")"
elif grep -q 'vm.sh tune' "$dp-fail/calls" || grep -qs '^rc 0' "$dp-fail/calls" ||
    ! grep -q 'ERROR: 192.0.2.2: .*no fifth core' "$dp-fail/log"; then
    fail "lib.sh prepare_vms dry: a host that cannot run a mode did not stop the run: $(tail -2 "$dp-fail/log")"
elif ! grep -qx 'market spot (AWS spot instances: AWS may reclaim them mid-run)' "$dp-plain/env-192.0.2.1.txt" ||
    ! grep -qx 'market spot (AWS spot instances: AWS may reclaim them mid-run)' "$dp-plain/env-192.0.2.2.txt"; then
    fail "lib.sh prepare_vms dry: the market not recorded: $(cat "$dp-plain"/env-*.txt)"
else
    pass "lib.sh prepare_vms dry: ynl fetched for NAPI_BUSY only, the @DPDK= builds asked for (bench/want_dpdk), each host's readiness for every mode checked after the build (stopping the run before tune), the market recorded"
fi

# lib.sh remote_state, dry: a state that never finishes (VM a writes no rc)
# is waited for as long as the instances live (max_hours, lab.sh's
# LAB_MAX_HOURS: a poll every 10 s, 360 an hour), not a fixed 8 hours, so a
# state of over 8 h under a raised LAB_MAX_HOURS is not given up on; without
# max_hours, 8 h
cat >"$tmp/dry-wait.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2 a_private=10.0.0.1 b_private=10.0.0.2
reps=1 modes=(kernel) rtt_messages=1 rtt_warmup=1 tput_seconds=5 results=$out/results.csv
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
on() { [[ $2 == "cat bench/remote-pinned/rc 2>/dev/null" ]] && echo poll >>"$out/polls"; return 0; }
[[ -n ${MAX:-} ]] && max_hours=$MAX
remote_state pinned
echo "rc $?" >>"$out/log"
SH
waits=
for max in 1 12 ''; do
    dw=$tmp/dry-wait-${max:-none}
    mkdir -p "$dw"
    MAX=$max bash "$tmp/dry-wait.sh" "$cloud" "$dw" </dev/null 2>"$dw/stderr" || true
    waits+="${max:-none}:$(wc -l <"$dw/polls" | tr -d ' '):$(tail -1 "$dw/log") "
done
if [[ $waits != "1:360:rc 1 12:4320:rc 1 none:2880:rc 1 " ]]; then
    fail "lib.sh remote_state dry: the wait for a state that never ends: $waits"
else
    pass "lib.sh remote_state dry: a state waited for as long as LAB_MAX_HOURS (8 h without it), then failed"
fi

# vm.sh client in a sandbox (fake sudo logs the command it would run): the
# mode's channel parameters reach the client (PUB_CHANNEL_PARAMS beside
# SUB_CHANNEL_PARAMS), an unreliable channel's lost-round-trip timeout
# stretched by a long network round trip; under INVOKER (vm.sh driver leaves
# ~/bench/harness_driver) the client runs with the driver's whole environment
# (driver.env: AERON_*, RUSTERON_DPDK_*, the transport) and HARNESS_DRIVER=invoker,
# and without it, as before, with neither
sbc=$tmp/vm-client
mkdir -p "$sbc/bin" "$sbc/home/ergon" "$sbc/home/bench"
cp "$sb/bin/"* "$sbc/bin/"
printf 'term-length=64m|mtu=8960\n' >"$sbc/home/bench/pub_channel_params"
printf 'rcv-wnd=16m|reliable=false|group=false|tether=false\n' >"$sbc/home/bench/sub_channel_params"
echo 70000 >"$sbc/home/bench/base_rtt_us"
echo aws >"$sbc/home/bench/cloud"
client_sb() {
    : >"$sbc/log"
    env -i PATH="$sbc/bin:$PATH" HOME="$sbc/home" FAKE_LOG="$sbc/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" \
        bash "$cloud/vm.sh" client isolated:kernel rtt xping 10.30.1.7:20123 10.30.1.5:20124 10 1 2 >"$sbc/out" 2>&1
    grep '^sudo ' "$sbc/log" | tail -1
}
external=$(client_sb)
printf "export AERON_THREADING_MODE='INVOKER'\nexport RUSTERON_MEDIA_DRIVER_TRANSPORT='dpdk-ena'\n" >"$sbc/home/bench/driver.env"
echo invoker >"$sbc/home/bench/harness_driver"
invoker=$(client_sb)
if [[ $external != *" SUB_CHANNEL_PARAMS=rcv-wnd=16m|reliable=false|group=false|tether=false PUB_CHANNEL_PARAMS=term-length=64m|mtu=8960 RTT_LOST_TIMEOUT_US=280000 "* ]] ||
    [[ $external == *HARNESS_DRIVER* || $external == *driver.env* ]]; then
    fail "vm.sh client: $external $(tail -2 "$sbc/out")"
elif [[ $invoker != *"$sbc/home/bench/driver.env"* || $invoker != *" HARNESS_DRIVER=invoker "* || $invoker != *" PUB_CHANNEL_PARAMS=term-length=64m|mtu=8960 "* ]]; then
    fail "vm.sh client under INVOKER: $invoker $(tail -2 "$sbc/out")"
else
    pass "vm.sh client: the mode's publication and subscription parameters, the lost-round-trip timeout by the network's round trip; under INVOKER the driver's environment and HARNESS_DRIVER=invoker"
fi
# vm.sh client hands LOAD_SPIKE_NS and LOOP_STALL_NS to the client through
# sudo, which drops the caller's environment (lib.sh sets them on the round
# trip's command line): 100 us and 20 us, the clients' own, without them
stall_client() { # <environment>...: the client's command line vm.sh client hands sudo
    : >"$sbc/log"
    env -i PATH="$sbc/bin:$PATH" HOME="$sbc/home" FAKE_LOG="$sbc/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" "$@" \
        bash "$cloud/vm.sh" client isolated:kernel rtt xping 10.30.1.7:20123 10.30.1.5:20124 10 1 2 >"$sbc/out" 2>&1
    grep '^sudo ' "$sbc/log" | tail -1
}
stall_given=$(stall_client LOAD_SPIKE_NS=30000 LOOP_STALL_NS=10000)
stall_default=$(stall_client)
if [[ $stall_given != *" LOAD_SPIKE_NS=30000 "* || $stall_given != *" LOOP_STALL_NS=10000 "* ]]; then
    fail "vm.sh client: LOAD_SPIKE_NS and LOOP_STALL_NS given do not reach the client: $stall_given"
elif [[ $stall_default != *" LOAD_SPIKE_NS=100000 "* || $stall_default != *" LOOP_STALL_NS=20000 "* ]]; then
    fail "vm.sh client without them: $stall_default"
else
    pass "vm.sh client: LOAD_SPIKE_NS and LOOP_STALL_NS reach the client through sudo (100 us and 20 us without them)"
fi
# KNOISE: vm.sh knoise_start against a fake tracefs ($LAB_SYS_ROOT/kernel/tracing,
# the instance knoise as mkdir leaves it): the osnoise tracer in that
# instance with NO_OSNOISE_WORKLOAD and exactly the agent and app CPUs given,
# its irq, softirq, thread and NMI noise events, the IPI senders, compaction
# and page migration on, the instance's cpumask printed as a list where the
# kernel has the option; without osnoise in available_tracers, "osnoise
# unavailable" said (a reported limit, never silent), its files untouched,
# the other events on all the same, as where the instance refuses the tracer
# (its options put back); an event the kernel lacks named, the rest
# on. knoise_stop puts the tracer and the workload option back. diag-start
# refuses while a rep runs (a media driver or a client), before it writes
# anything, and otherwise hands knoise_start the layout's agent and app CPUs
knoise_tree() { # <root> <tracers>: a fake tracefs
    local t=$1/kernel/tracing e
    mkdir -p "$t/osnoise" "$t/instances/knoise/options"
    echo "$2" >"$t/available_tracers"
    echo "OSNOISE_WORKLOAD" >"$t/osnoise/options"
    echo "0-7" >"$t/osnoise/cpus"
    echo nop >"$t/instances/knoise/current_tracer"
    echo 1 >"$t/instances/knoise/tracing_on"
    echo 1408 >"$t/instances/knoise/buffer_size_kb"
    echo 0 >"$t/instances/knoise/options/bitmask-list"
    : >"$t/instances/knoise/trace"
    for e in osnoise/irq_noise osnoise/softirq_noise osnoise/thread_noise osnoise/nmi_noise ipi/ipi_send_cpumask ipi/ipi_send_cpu \
        compaction/mm_compaction_begin migrate/mm_migrate_pages; do
        mkdir -p "$t/instances/knoise/events/$e"
        echo 0 >"$t/instances/knoise/events/$e/enable"
    done
}
knoise_state() { # <root>: what the fake tracefs holds now, one line
    local t=$1/kernel/tracing i=$1/kernel/tracing/instances/knoise e
    printf 'options=%s cpus=%s tracer=%s on=%s buffer=%s list=%s events=' "$(cat "$t/osnoise/options" 2>/dev/null || echo -)" "$(cat "$t/osnoise/cpus")" \
        "$(cat "$i/current_tracer" 2>/dev/null || echo -)" "$(cat "$i/tracing_on")" "$(cat "$i/buffer_size_kb")" "$(cat "$i/options/bitmask-list" 2>/dev/null)"
    for e in "$i"/events/*/*/enable; do
        [[ $(cat "$e") == 1 ]] && printf '%s,' "$(basename "$(dirname "$e")")"
    done
    echo
}
kn=$tmp/knoise
knoise_tree "$kn/full" "hwlat osnoise timerlat function nop"
knoise_tree "$kn/none" "function nop"
knoise_tree "$kn/old" "hwlat osnoise nop"
# a kernel whose instances take no osnoise tracer: its write refused (a
# directory in its place, which refuses root too)
knoise_tree "$kn/refused" "hwlat osnoise nop"
rm "$kn/refused/kernel/tracing/instances/knoise/current_tracer"
mkdir "$kn/refused/kernel/tracing/instances/knoise/current_tracer"
# a kernel with the osnoise tracer but no osnoise/options (before Linux 6.3,
# such as 6.1): NO_OSNOISE_WORKLOAD cannot be set, so the tracer would start
# its workload threads on the agent CPUs; it must stay off (a directory in the
# options file's place refuses the write, as tracefs refuses a new file)
knoise_tree "$kn/noopt" "hwlat osnoise nop"
rm "$kn/noopt/kernel/tracing/osnoise/options"
mkdir "$kn/noopt/kernel/tracing/osnoise/options"
rm -r "$kn/old/kernel/tracing/instances/knoise/events/compaction" "$kn/old/kernel/tracing/instances/knoise/events/ipi/ipi_send_cpumask" \
    "$kn/old/kernel/tracing/instances/knoise/options/bitmask-list"
knoised=$(
    set +e
    exec 2>&1
    eval "$(vm_fn knoise_start; vm_fn knoise_stop)"
    for tree in full none old refused noopt; do
        echo "$tree: $(LAB_SYS_ROOT=$kn/$tree knoise_start 1,2,3,4 | tr '\n' '|')"
        echo "$tree now: $(knoise_state "$kn/$tree")"
    done
    LAB_SYS_ROOT=$kn/full knoise_stop
    echo "full stopped: $(knoise_state "$kn/full")"
)
want_knoise='full: osnoise on cpus 1,2,3,4 (NO_OSNOISE_WORKLOAD: noise timed on them, no workload)|
full now: options=NO_OSNOISE_WORKLOAD cpus=1,2,3,4 tracer=osnoise on=1 buffer=8192 list=1 events=mm_compaction_begin,ipi_send_cpu,ipi_send_cpumask,mm_migrate_pages,irq_noise,nmi_noise,softirq_noise,thread_noise,
none: osnoise unavailable: the kernel has no osnoise tracer (available_tracers: function nop)|
none now: options=OSNOISE_WORKLOAD cpus=0-7 tracer=nop on=1 buffer=8192 list=1 events=mm_compaction_begin,ipi_send_cpu,ipi_send_cpumask,mm_migrate_pages,
old: osnoise on cpus 1,2,3,4 (NO_OSNOISE_WORKLOAD: noise timed on them, no workload)|knoise events missing: ipi/ipi_send_cpumask compaction/mm_compaction_begin|
old now: options=NO_OSNOISE_WORKLOAD cpus=1,2,3,4 tracer=osnoise on=1 buffer=8192 list= events=ipi_send_cpu,mm_migrate_pages,irq_noise,nmi_noise,softirq_noise,thread_noise,
refused: osnoise unavailable: the knoise instance did not take the osnoise tracer (it has unreadable)|
refused now: options=OSNOISE_WORKLOAD cpus=0-7 tracer=- on=1 buffer=8192 list=1 events=mm_compaction_begin,ipi_send_cpu,ipi_send_cpumask,mm_migrate_pages,
noopt: osnoise unavailable: this osnoise tracer has no NO_OSNOISE_WORKLOAD option (Linux 6.3 and later), so it would run its workload on the agent CPUs|
noopt now: options=- cpus=0-7 tracer=nop on=1 buffer=8192 list=1 events=mm_compaction_begin,ipi_send_cpu,ipi_send_cpumask,mm_migrate_pages,
full stopped: options=OSNOISE_WORKLOAD cpus=1,2,3,4 tracer=nop on=0 buffer=8192 list=1 events=mm_compaction_begin,ipi_send_cpu,ipi_send_cpumask,mm_migrate_pages,irq_noise,nmi_noise,softirq_noise,thread_noise,'
# diag-start in a sandbox: a fake pgrep says a rep runs (a media driver, or
# none), gcc, ip and pkill do nothing, sudo logs
ks=$tmp/vm-knoise
mkdir -p "$ks/bin" "$ks/home/ergon" "$ks/home/bench"
cp "$sb/bin/"* "$ks/bin/"
echo aws >"$ks/home/bench/cloud"
# shellcheck disable=SC2016
{
    printf '#!/bin/sh\n[ -n "$FAKE_RUNNING" ] && { echo "$FAKE_RUNNING"; exit 0; }\nexit 1\n' >"$ks/bin/pgrep"
    printf '#!/bin/sh\necho "gcc $*" >>"$FAKE_LOG"\nwhile [ "$1" != -o ]; do shift; done\nprintf "#!/bin/sh\\n" >"$2"; chmod +x "$2"\n' >"$ks/bin/gcc"
    printf '#!/bin/sh\nexit 0\n' >"$ks/bin/ip"
    printf '#!/bin/sh\nexit 1\n' >"$ks/bin/pkill"
}
chmod +x "$ks/bin/"*
diag_sb() { # <running>: "<rc> <sudo and gcc calls> <the CPUs knoise_start was given>"
    : >"$ks/log"
    local rc=0
    env -i PATH="$ks/bin:$PATH" HOME="$ks/home" FAKE_LOG="$ks/log" FAKE_ARCH=x86_64 FAKE_RUNNING="$1" LAB_SYS_ROOT="$sys/aws-x86" \
        bash "$cloud/vm.sh" diag-start >"$ks/out" 2>&1 || rc=$?
    echo "$rc $(grep -cE '^(sudo|gcc) ' "$ks/log") [$(sed -n 's/.*knoise_start \([0-9,]*\)$/\1/p' "$ks/log")] [$(cat "$ks/home/bench/knoise-cpus" 2>/dev/null)]"
}
running=$(diag_sb 4242)
running_said=$(cat "$ks/out")
idle=$(diag_sb "")
layout_cpus=$(env -i PATH="$ks/bin:$PATH" HOME="$ks/home" FAKE_LOG=/dev/null FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" bash "$cloud/vm.sh" layout | awk '{$1 = ""; print substr($0, 2)}' | tr ' ' ',')
if [[ $knoised != "$want_knoise" ]]; then
    fail "vm.sh knoise_start and knoise_stop against a fake tracefs"
    diff -u <(echo "$want_knoise") <(echo "$knoised") >&2 || true
elif [[ $running != "2 0 [] []" || $running_said != *"a rep is running"* ]]; then
    fail "vm.sh diag-start while a rep runs: $running: $running_said"
elif [[ -z $layout_cpus || $idle != "0 "*" [$layout_cpus] [$layout_cpus]" ]]; then
    fail "vm.sh diag-start: knoise_start not given the agent and app CPUs ($layout_cpus): $idle $(tail -3 "$ks/out")"
else
    pass "vm.sh: KNOISE (osnoise with NO_OSNOISE_WORKLOAD on exactly the agent and app CPUs, its noise events, IPI senders, compaction and migration; 'osnoise unavailable' said; a missing event named), put back by knoise_stop; diag-start refuses while a rep runs"
fi
# lib.sh record_knoise: each host's KNOISE lines from its diag-start log in
# its environment record (env-<host>.txt) with the state, "osnoise
# unavailable" included; a host whose log has none says so
(
    set +e
    out=$tmp/knoise-env known=$tmp/knoise-env/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
    mkdir -p "$out"
    log() { :; }
    . "$cloud/lib.sh"
    printf 'diag spinner cpu 5, traced cpus 3e\nosnoise unavailable: the kernel has no osnoise tracer (available_tracers: function nop)\n' >"$out/a-diag-start-isolated.log"
    printf 'osnoise on cpus 1,2,3,4 (NO_OSNOISE_WORKLOAD: noise timed on them, no workload)\nknoise events missing: compaction/mm_compaction_begin\n' >"$out/b-diag-start-isolated.log"
    record_knoise isolated
    record_knoise pinned
) || true
if [[ $(cat "$tmp/knoise-env/env-192.0.2.1.txt" 2>/dev/null) != "isolated: osnoise unavailable: the kernel has no osnoise tracer (available_tracers: function nop)
pinned: knoise not recorded (no diag-start log)" ||
    $(cat "$tmp/knoise-env/env-192.0.2.2.txt" 2>/dev/null) != "isolated: osnoise on cpus 1,2,3,4 (NO_OSNOISE_WORKLOAD: noise timed on them, no workload)
isolated: knoise events missing: compaction/mm_compaction_begin
pinned: knoise not recorded (no diag-start log)" ]]; then
    fail "lib.sh record_knoise: $(cat "$tmp/knoise-env/env-"*.txt 2>/dev/null | tr '\n' '|')"
else
    pass "lib.sh record_knoise: each host's KNOISE state per state in its environment record, 'osnoise unavailable' included"
fi
# The media-receive-timestamp mode's way to the clients: its
# @SUB_PARAMS=media-rcv-ts-offset=reserved applied as `driver` applies a
# mode's settings (vm.sh host_setting, modes.sh channel_params) is every
# subscription channel's parameter (~/bench/sub_channel_params), which
# `client` hands the clients as SUB_CHANNEL_PARAMS; an offset over what the
# clients read is refused there too, writing nothing
rs=$tmp/vm-rcvstamp
mkdir -p "$rs/home/bench"
stamped=$(
    set +e
    exec 2>&1
    # shellcheck source=../modes.sh
    . "$cloud/modes.sh" || exit 1
    eval "$(for f in append_params save_host host_setting; do vm_fn "$f"; done)"
    run=$rs/home/bench
    log() { :; }
    (host_setting SUB_PARAMS media-rcv-ts-offset=8 dpdk) >/dev/null 2>&1 && echo "offset 8 taken"
    [[ -e $run/sub_channel_params ]] && echo "offset 8 written"
    (host_setting SUB_PARAMS media-rcv-ts-offset=reserved dpdk) >/dev/null || echo "reserved refused"
    echo "params: $(cat "$run/sub_channel_params")"
)
rm -f "$sbc/home/bench/driver.env" "$sbc/home/bench/harness_driver"
cp "$rs/home/bench/sub_channel_params" "$sbc/home/bench/sub_channel_params" 2>/dev/null || rm -f "$sbc/home/bench/sub_channel_params"
stamped_client=$(client_sb)
if [[ $stamped != "params: media-rcv-ts-offset=reserved" ]]; then
    fail "vm.sh host_setting SUB_PARAMS=media-rcv-ts-offset: $stamped"
elif [[ $stamped_client != *" SUB_CHANNEL_PARAMS=media-rcv-ts-offset=reserved PUB_CHANNEL_PARAMS="* ]]; then
    fail "vm.sh client in the media-receive-timestamp mode: $stamped_client"
else
    pass "vm.sh: @SUB_PARAMS=media-rcv-ts-offset=reserved reaches every subscription the clients create (an offset over their stamp and sequence refused)"
fi
# vm.sh hostchecks: the media driver's start-up host checks as it printed
# them in this run's driver.log (ConfiguredDriver::from_env, md-dpdk's
# checks.rs: `ergon-md-dpdk warning:` and `ergon-md-dpdk info:` lines), and
# nothing else of the log; none at all from a driver that said none, or no log
printf '%s\n' 'transport backend: dpdk-ena' \
    'ergon-md-dpdk warning: T3 transparent huge pages always, against madvise (fix: echo madvise), max 1,200 us' \
    'an ergon-md-dpdk info: line quoted mid-line' 'ergon-md-dpdk info: M4 kernel busy polling off (Graviton)' \
    'media driver started' >"$sbc/home/bench/driver.log"
checks_sb() { env -i PATH="$sbc/bin:$PATH" HOME="$sbc/home" FAKE_LOG=/dev/null FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" bash "$cloud/vm.sh" hostchecks 2>&1; echo "rc $?"; }
checks=$(checks_sb)
printf 'media driver started\n' >"$sbc/home/bench/driver.log"
quiet=$(checks_sb)
rm -f "$sbc/home/bench/driver.log"
nolog=$(checks_sb)
if [[ $checks != "ergon-md-dpdk warning: T3 transparent huge pages always, against madvise (fix: echo madvise), max 1,200 us
ergon-md-dpdk info: M4 kernel busy polling off (Graviton)
rc 0" || $quiet != "rc 0" || $nolog != "rc 0" ]]; then
    fail "vm.sh hostchecks: $checks / $quiet / $nolog"
else
    pass "vm.sh hostchecks: the driver's start-up warning and info lines from its log, nothing else"
fi

# lib.sh's on() against a fake ssh that runs the remote command here as
# OpenSSH runs it there: through bash -c, its exit status ssh's, 255 when the
# remote shell died of a signal; the client's own known_hosts warning first
# on stderr. The remote side says first that it ran, so on() never runs a
# command twice: a remote shell killed by a signal (as SIGKILL ended the G2
# pong's, and on() then started an identical pong on the same ports) is not
# retried but logged as a lab fault (the log and lab-faults.log); a genuine
# connection failure (refused, nothing run) is still retried, 18 times, and
# a link that fails twice runs its command on the third try, once. What the
# remote side says to show it ran never reaches the caller; everything else
# does, stdout and stderr apart. And SIGTERM to the remote shell (the lab's
# pkill -f naming a client matches its command line too) leaves it waiting
# for its command, whose status it returns, never 255.
onx=$tmp/on-ssh
mkdir -p "$onx"
# shellcheck disable=SC2016
cat >"$onx/fake-ssh.sh" <<'SH'
# ssh <options> <user@host> <command>: the command run here, as OpenSSH runs
# it there (in FAKE_REMOTE_HOME when set), with its exit statuses; FAKE_SSH:
# refused (nothing run), flaky (the first two tries refused), hang (the
# first try hangs 20 s, the rest are refused); the lab's pkill -f '<pattern>'
# SIGTERMs, by PID, what its pattern matches of this test's pong's processes
# (their command lines name its endpoint, FAKE_PONG_EP), this lab-side ssh
# among them, as on VM a, which runs the lab and its localhost clients both.
# The ssh, as OpenSSH's, carries the remote command in its arguments and,
# sent SIGTERM, says "Killed by signal 15." and exits 255.
ssh() {
    local cmd=${*: -1} host=${*: -2:1} pid
    echo "$cmd" >>"$out/ssh-calls"
    case ${FAKE_SSH:-run} in
    refused) echo "ssh: connect to host $host port 22: Connection refused" >&2; return 255 ;;
    flaky) (($(wc -l <"$out/ssh-calls") > 2)) || { echo "ssh: connect to host $host port 22: Operation timed out" >&2; return 255; } ;;
    hang)
        if [[ -e $out/hung ]]; then echo "ssh: connect to host $host port 22: Connection refused" >&2; return 255; fi
        : >"$out/hung"
        command sleep 20 &
        echo $! >"$out/hang-pid"
        wait $!
        return 255
        ;;
    esac
    if [[ $cmd =~ pkill\ -f\ \'([^\']+)\' ]]; then
        for pid in $(pgrep -f "${BASH_REMATCH[1]}"); do
            if [[ $(ps -o command= -p "$pid" 2>/dev/null) == *"${FAKE_PONG_EP:?}"* ]]; then kill -TERM "$pid" 2>/dev/null; fi
        done
        return 0
    fi
    echo "Warning: Permanently added '$host' (ED25519) to the list of known hosts." >&2
    python3 -c '
import os, signal, subprocess, sys
def killed(sig, _frame):
    print(f"Killed by signal {sig}.", file=sys.stderr, flush=True)
    os._exit(255)
signal.signal(signal.SIGTERM, killed)
r = subprocess.run(["bash", "-c", sys.argv[1]], cwd=os.environ.get("FAKE_REMOTE_HOME"))
sys.exit(255 if r.returncode < 0 else r.returncode)' "$cmd"
}
SH
said=$(
    set +e
    out=$onx known=$onx/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
    log() { echo "$*" >>"$out/log"; }
    # shellcheck source=../lib.sh
    . "$cloud/lib.sh"
    # shellcheck source=/dev/null
    . "$onx/fake-ssh.sh"
    sleep() { :; }
    lines() { wc -l <"$1" | tr -d ' '; }
    : >"$out/ssh-calls"
    on "$A" 'echo out; echo err >&2; exit 3' >"$onx/a.out" 2>"$onx/a.err"
    echo "ran: $? tries $(lines "$out/ssh-calls") [$(cat "$onx/a.out")] [$(tr '\n' '|' <"$onx/a.err")]"
    : >"$out/ssh-calls" && : >"$out/log"
    on "$A" "echo \$\$ >>$onx/b-ran; echo before >&2; kill -KILL \$\$" 2>"$onx/b.err"
    echo "killed: $? tries $(lines "$out/ssh-calls") runs $(lines "$onx/b-ran") [$(tr '\n' '|' <"$onx/b.err" | sed 's/Warning: [^|]*|//g')] faults $(grep -c 'LAB FAULT' "$out/log") $(grep -c "192.0.2.1: .* ran, and its SSH session ended 255" "$out/lab-faults.log")"
    : >"$out/ssh-calls" && : >"$out/log"
    FAKE_SSH=refused on "$A" true 2>"$onx/c.err"
    echo "refused: $? tries $(lines "$out/ssh-calls") faults $(grep -c 'LAB FAULT' "$out/log") said $(grep -c 'Connection refused' "$onx/c.err")"
    : >"$out/ssh-calls"
    FAKE_SSH=flaky on "$A" "echo x >>$onx/d-ran; exit 5" 2>/dev/null
    echo "flaky: $? tries $(lines "$out/ssh-calls") runs $(lines "$onx/d-ran")"
    : >"$out/ssh-calls" && : >"$out/log"
    on "$A" "echo \$\$ >$onx/e-ran; command sleep 1; echo survived-$$; exit 4" >"$onx/e.out" 2>/dev/null &
    job=$!
    for _ in $(seq 200); do [[ -s $onx/e-ran ]] && break; command sleep 0.05; done
    kill -TERM "$(cat "$onx/e-ran")"
    wait $job
    echo "termed: $? tries $(lines "$out/ssh-calls") [$(cat "$onx/e.out")] faults $(grep -c 'LAB FAULT' "$out/log")"
) || true
want="ran: 3 tries 1 [out] [Warning: Permanently added 'admin@192.0.2.1' (ED25519) to the list of known hosts.|err|]
killed: 255 tries 1 runs 1 [before|] faults 1 1
refused: 255 tries 18 faults 0 said 18
flaky: 5 tries 3 runs 1
termed: 4 tries 1 [survived-$$] faults 0"
if [[ $said == "$want" ]]; then
    pass "lib.sh on(): a remote command killed by a signal is not retried (a lab fault, logged); a connection failure still is, 18 times; a flaky link's command runs once; SIGTERM leaves the remote shell its command's status; its marker never reaches the caller"
else
    fail "lib.sh on(): retries"
    diff -u <(echo "$want") <(echo "$said") >&2 || true
fi

# vm.sh client's deadline (LAB_CLIENT_DEADLINE, LAB_CLIENT_GRACE), in a
# sandbox like vm.sh client's above but whose sudo and taskset run what they
# are given, with timeout (GNU's where the host has it; else a stand-in for
# what vm.sh relies on: SIGTERM to its command at the deadline or when it
# gets one itself, SIGKILL the grace later, itself dying of it as GNU's
# does, 124 when the deadline ended it) and a fake rtt xpong that never ends
# by itself: one that ignores SIGTERM (stubborn, as G2's pong, spinning in
# its echo, did) or one that stops on it (polite).
#   * The deadline ends a stubborn pong the grace after it, a polite one at
#     it, and vm.sh exits 124 (an exit of its own, never a signal), saying
#     which.
#   * SIGTERM to every process naming the pong (the lab's pkill -f '[r]tt
#     xpong'; here by PID) leaves vm.sh waiting for it: a polite pong ends at
#     once (0), a stubborn one the grace later (137), and vm.sh exits with
#     that status, never killed.
#   * No fake rtt is left running (by its PID).
sbd=$tmp/vm-deadline
mkdir -p "$sbd/bin" "$sbd/home/ergon/target/release" "$sbd/home/ergon/md-dpdk/benches" "$sbd/home/bench"
cp "$sb/bin/uname" "$sb/bin/nproc" "$sb/bin/curl" "$sbd/bin/"
ln -s "$cloud" "$sbd/home/ergon/md-dpdk/benches/cloud"
echo aws >"$sbd/home/bench/cloud"
cat >"$sbd/bin/sudo" <<'SH'
#!/usr/bin/env bash
echo "sudo $*" >>"$FAKE_LOG"
[[ $1 == env ]] && exec "$@"
exit 0
SH
# shellcheck disable=SC2016
printf '#!/bin/sh\nshift 2\nexec "$@"\n' >"$sbd/bin/taskset"
if timeout --version 2>/dev/null | head -1 | grep -q 'GNU coreutils'; then
    timeout_is="GNU timeout"
else
    timeout_is="a stand-in for GNU timeout (none on this host: GNU's own behaviour unknown here)"
    cat >"$sbd/bin/timeout" <<'PY'
#!/usr/bin/env python3
"""GNU timeout(1) as vm.sh client uses it, where the host has none (macOS):
timeout --kill-after=GRACE DEADLINE COMMAND... SIGTERM to COMMAND at
DEADLINE, or when timeout gets one itself; SIGKILL GRACE later, timeout then
dying of SIGKILL too (GNU's sends it to its whole process group); 124 once
the deadline ended COMMAND, else COMMAND's status."""
import os, signal, subprocess, sys, time
args = sys.argv[1:]
grace = float(args.pop(0).split("=", 1)[1]) if args[0].startswith("--kill-after=") else None
end = time.monotonic() + float(args.pop(0))
child = subprocess.Popen(args)
state = {"timed_out": False, "kill_at": None}
def term(timed_out):
    state["timed_out"] = state["timed_out"] or timed_out
    if grace is not None and state["kill_at"] is None:
        state["kill_at"] = time.monotonic() + grace
    child.send_signal(signal.SIGTERM)
signal.signal(signal.SIGTERM, lambda *_: term(False))
while child.poll() is None:
    now = time.monotonic()
    if now >= end and not state["timed_out"]:
        term(True)
    if state["kill_at"] is not None and now >= state["kill_at"]:
        child.kill()
        child.wait()
        os.kill(os.getpid(), signal.SIGKILL)
    time.sleep(0.01)
sys.exit(124 if state["timed_out"] else (128 - child.returncode if child.returncode < 0 else child.returncode))
PY
fi
cat >"$sbd/home/ergon/target/release/rtt" <<'SH'
#!/usr/bin/env bash
# a pong that never ends by itself: FAKE_RTT=stubborn ignores SIGTERM (as G2's
# did), polite stops on it; its PID in $FAKE_LOG.rtt
echo $$ >>"$FAKE_LOG.rtt"
if [[ ${FAKE_RTT:-stubborn} == polite ]]; then
    trap 'echo "rtt: pong ended: SIGTERM" >&2; exit 0' TERM
else
    trap 'echo "rtt: SIGTERM ignored" >&2' TERM
fi
while :; do sleep 0.05; done
SH
chmod +x "$sbd/bin/"* "$sbd/home/ergon/target/release/rtt"
deadline_sb() { # <FAKE_RTT> <deadline> <grace> <SIGTERM this long after the pong started, or ->: "<how vm.sh ended> <in its bound or not> left <rtts running>"
    python3 - "$sbd" "$cloud" "$sys/aws-x86" "$@" <<'PY'
import os, signal, subprocess, sys, time
sbd, cloud, root, rtt, deadline, grace, term = sys.argv[1:8]
ep = f"198.51.100.7:{20000 + os.getpid() % 40000}"
open(f"{sbd}/log", "w").close()
if os.path.exists(f"{sbd}/log.rtt"):
    os.remove(f"{sbd}/log.rtt")
env = {"PATH": f"{sbd}/bin:{os.environ['PATH']}", "HOME": f"{sbd}/home", "FAKE_LOG": f"{sbd}/log", "FAKE_ARCH": "x86_64",
       "LAB_SYS_ROOT": root, "FAKE_RTT": rtt, "LAB_CLIENT_DEADLINE": deadline, "LAB_CLIENT_GRACE": grace}
start = time.monotonic()
p = subprocess.Popen(["bash", f"{cloud}/vm.sh", "client", "isolated:kernel", "rtt", "xpong", ep, "198.51.100.8:20124", "2"],
                     env=env, stdout=subprocess.DEVNULL, stderr=open(f"{sbd}/out", "w"))
since, bound = start, float(deadline) + (float(grace) if rtt == "stubborn" else 0)
# the pong up first: a slow host takes seconds to start vm.sh (it reads the
# host's layout), which the deadline, counted from timeout's start just
# before the pong's, leaves out, so it is timed from the pong's start
up = time.monotonic() + 30
while not (os.path.exists(f"{sbd}/log.rtt") and open(f"{sbd}/log.rtt").read().strip()) and time.monotonic() < up:
    time.sleep(0.01)
since = time.monotonic()
if term != "-":
    # then the lab's SIGTERM
    time.sleep(float(term))
    since, bound = time.monotonic(), (float(grace) if rtt == "stubborn" else 0)
    for pid in subprocess.run(["pgrep", "-f", f"rtt xpong {ep}"], capture_output=True, text=True).stdout.split():
        try:
            os.kill(int(pid), signal.SIGTERM)
        except ProcessLookupError:
            pass
try:
    rc = p.wait(timeout=30)
except subprocess.TimeoutExpired:
    p.kill()
    rc = p.wait()
took = time.monotonic() - since
# slack for a busy machine: 0.5 s early (the pong starts just after
# timeout, its deadline's clock), 4 s late
verdict = "in-bound" if bound - 0.5 <= took <= bound + 4 else f"out-of-bound({took:.2f}s, bound {bound}s)"
print(f"vm.sh client, {rtt} pong, deadline {deadline} s, grace {grace} s, SIGTERM {term}: ended {took:.3f} s on (bound {bound} s)", file=sys.stderr)
pids = [int(l) for l in open(f"{sbd}/log.rtt")] if os.path.exists(f"{sbd}/log.rtt") else []
deadline_at = time.monotonic() + 1
def running(pid):  # a zombie (its parent gone, not yet reaped) has ended
    stat = subprocess.run(["ps", "-o", "stat=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()
    return stat != "" and not stat.startswith("Z")
while any(map(running, pids)) and time.monotonic() < deadline_at:
    time.sleep(0.05)
left = sum(map(running, pids))
for pid in pids:
    if running(pid):
        os.kill(pid, signal.SIGKILL)
print(f"{'exit' if rc >= 0 else 'signal'} {abs(rc)} {verdict} left {left} of {len(pids)}")
PY
    grep -o 'client: rtt xpong.*' "$sbd/out" | tail -1 || true
}
got="stubborn at its deadline: $(deadline_sb stubborn 2 1 -)
polite at its deadline: $(deadline_sb polite 2 1 -)
stubborn, the lab's SIGTERM: $(deadline_sb stubborn 60 1 0.5)
polite, the lab's SIGTERM: $(deadline_sb polite 60 1 0.5)"
want="stubborn at its deadline: exit 124 in-bound left 0 of 1
client: rtt xpong overran its deadline of 2 s: it ignored SIGTERM, and SIGKILL ended it 1 s later
polite at its deadline: exit 124 in-bound left 0 of 1
client: rtt xpong overran its deadline of 2 s: SIGTERM ended it
stubborn, the lab's SIGTERM: exit 137 in-bound left 0 of 1
client: rtt xpong: SIGTERM: it ignored it, and SIGKILL ended it 1 s later (137)
polite, the lab's SIGTERM: exit 0 in-bound left 0 of 1
client: rtt xpong: SIGTERM: it ended (0)"
if [[ $got == "$want" ]]; then
    pass "vm.sh client: a pong that ignores SIGTERM is ended the grace after its deadline or the lab's SIGTERM, vm.sh exiting 124 or 137 itself, never killed, no client left ($timeout_is)"
else
    fail "vm.sh client's deadline ($timeout_is)"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
fi

# lib.sh end to end, on() over the fake ssh into the sandbox above (each
# command run there as on its VM): a pong that ignores SIGTERM, started as
# bench_state starts one (on ... &), then ended as the lab ends one
# (end_client: SIGTERM to what names its path, this lab-side ssh not among
# them): its job ends the grace (1 s) later, 137, vm.sh saying so in its
# log, its command run once (no retry), no lab fault. Left alone, its deadline (2 s)
# ends it, 124, and the bounded wait logs that it overran. And an SSH
# session that never returns is ended by the bounded wait at its bound (1 s),
# a lab fault, its ssh gone (by PID).
e2e=$(
    set +e
    out=$tmp/on-pong known=$tmp/on-pong/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
    mkdir -p "$out"
    : >"$out/log"
    log() { echo "$*" >>"$out/log"; }
    # shellcheck source=../lib.sh
    . "$cloud/lib.sh"
    # shellcheck source=/dev/null
    . "$onx/fake-ssh.sh"
    export HOME=$sbd/home PATH=$sbd/bin:$PATH FAKE_LOG=$sbd/log FAKE_ARCH=x86_64 LAB_SYS_ROOT=$sys/aws-x86 FAKE_REMOTE_HOME=$sbd/home FAKE_RTT=stubborn
    export FAKE_PONG_EP=198.51.100.9:$((20000 + $$ % 40000))
    # on()'s retries and wait_bounded's polls quick
    sleep() { command sleep 0.05; }
    pong_up() { : >"$FAKE_LOG.rtt"; for _ in $(seq 200); do [[ -s $FAKE_LOG.rtt ]] && return; command sleep 0.05; done; }
    took() { python3 -c 'import sys; t = float(sys.argv[2]) - float(sys.argv[1]); print(f"lib.sh end to end: {t:.3f} s (bound {sys.argv[3]} s)", file=sys.stderr); print("in-bound" if t <= float(sys.argv[3]) else f"late({t:.2f}s)")' "$1" "$EPOCHREALTIME" "$2"; }
    # processes still running, by PID (a zombie, not yet reaped, has ended)
    running() { local pid n=0 stat; for pid in $(cat "$@"); do stat=$(ps -o stat= -p "$pid" 2>/dev/null); [[ -n $stat && $stat != Z* ]] && n=$((n + 1)); done; echo $n; }
    : >"$out/ssh-calls"
    rm -f "$FAKE_LOG.rtt"
    on "$B" "LAB_CLIENT_DEADLINE=60 LAB_CLIENT_GRACE=1 $vm client isolated:kernel rtt xpong $FAKE_PONG_EP 198.51.100.8:20124 2" >"$out/pong.log" 2>&1 &
    pong=$!
    pong_up
    t0=$EPOCHREALTIME
    end_client "$B" rtt xpong
    wait_bounded 30 "the test's pong" $pong
    echo "the lab's SIGTERM: $? $(took "$t0" 5) runs $(grep -c ' rtt xpong ' "$out/ssh-calls") faults $(grep -c 'LAB FAULT' "$out/log") left $(running "$FAKE_LOG.rtt") said $(grep -c 'client: rtt xpong: SIGKILL ended it (137): it ignored a SIGTERM, and timeout(1) sends SIGKILL 1 s after one' "$out/pong.log")"
    : >"$out/ssh-calls"
    t0=$EPOCHREALTIME
    on "$B" "LAB_CLIENT_DEADLINE=2 LAB_CLIENT_GRACE=1 $vm client isolated:kernel rtt xpong $FAKE_PONG_EP 198.51.100.8:20124 2" >"$out/pong.log" 2>&1 &
    pong=$!
    wait_bounded 30 "the test's pong" $pong
    echo "its deadline: $? $(took "$t0" 8) runs $(grep -c ' rtt xpong ' "$out/ssh-calls") faults $(grep -c "LAB FAULT: the test's pong: overran its deadline" "$out/log") left $(running "$FAKE_LOG.rtt")"
    : >"$out/log"
    rm -f "$out/hang-pid" "$out/hung"
    FAKE_SSH=hang on "$B" "LAB_CLIENT_DEADLINE=60 $vm client isolated:kernel rtt xpong $FAKE_PONG_EP 198.51.100.8:20124 2" >/dev/null 2>&1 &
    job=$!
    for _ in $(seq 200); do [[ -s $out/hang-pid ]] && break; command sleep 0.05; done
    t0=$EPOCHREALTIME
    wait_bounded 1 "the test's hung session" $job
    echo "a hung session: $? $(took "$t0" 5) faults $(grep -c "LAB FAULT: the test's hung session: still running 1 s on" "$out/log") left $(running "$out/hang-pid")"
) || true
want="the lab's SIGTERM: 137 in-bound runs 1 faults 0 left 0 said 1
its deadline: 124 in-bound runs 1 faults 1 left 0
a hung session: 143 in-bound faults 1 left 0"
if [[ $e2e == "$want" ]]; then
    pass "lib.sh end to end: a pong that ignores SIGTERM ended within the grace of the lab's SIGTERM (not retried, no fault) or of its deadline (a lab fault); a hung SSH session ended at the wait's bound ($timeout_is)"
else
    fail "lib.sh end to end: a pong that ignores SIGTERM ($timeout_is)"
    diff -u <(echo "$want") <(echo "$e2e") >&2 || true
    tail -5 "$tmp/on-pong/pong.log" "$sbd/out" >&2 2>/dev/null || true
fi

# vm.sh build and build-base in a sandbox (fake cargo logs each build, and
# fails a dpdk one with FAKE_NO_DPDK=1, as where the dpdk feature does not
# build): the benchmark clients get the media driver's features, DPDK's
# included wherever the driver has it, since under INVOKER each embeds the
# driver with the transport the mode selects (without the feature, dpdk-ena
# is refused: FeatureDisabled); without it, as the driver, kernel only. A
# base revision's clients alike, when its harness has the feature.
sbb=$tmp/vm-build
mkdir -p "$sbb/bin" "$sbb/home/ergon/md-dpdk/harness" "$sbb/home/ergon-base/md-dpdk/harness" "$sbb/home/bench" "$sbb/home/dpdk-kmods"
cp "$sb/bin/"* "$sbb/bin/"
# shellcheck disable=SC2016
{
    printf '#!/bin/sh\necho "cargo $*" >>"$FAKE_LOG"\ncase "$* $FAKE_NO_DPDK" in *static,dpdk*1) echo "error: md-dpdk requires Linux" >&2; exit 101 ;; esac\nexit 0\n' >"$sbb/bin/cargo"
    printf '#!/bin/sh\nexit 0\n' >"$sbb/bin/make"
    printf '#!/bin/sh\nexit 0\n' >"$sbb/bin/rustup"
}
chmod +x "$sbb/bin/"*
echo aws >"$sbb/home/bench/cloud"
# the checkout's own harness manifest; the base's with the feature too
cp "$cloud/../../harness/Cargo.toml" "$sbb/home/ergon/md-dpdk/harness/Cargo.toml"
printf '[features]\ndpdk = ["ergon-md-dpdk/dpdk"]\n' >"$sbb/home/ergon-base/md-dpdk/harness/Cargo.toml"
mkdir -p "$sbb/home/ergon-base/md-dpdk/harness/src/bin"
touch "$sbb/home/ergon-base/md-dpdk/harness/src/bin/"{rtt,load,tput}.rs
build_sb() { # <arch> <FAKE_NO_DPDK> <vm.sh command>: its cargo builds, one a line, "<package> <features> <bins>"
    : >"$sbb/log"
    env -i PATH="$sbb/bin:$PATH" HOME="$sbb/home" FAKE_LOG="$sbb/log" FAKE_ARCH="$1" FAKE_NO_DPDK="$2" \
        LAB_SYS_ROOT="$sys/$([[ $1 == aarch64 ]] && echo graviton || echo aws-x86)" \
        bash "$cloud/vm.sh" "$3" >"$sbb/out" 2>&1 || echo "exit $?"
    sed -n 's/^cargo build --release -p \([^ ]*\) --features \([^ ]*\) \(.*\)$/\1 \2 \3/p' "$sbb/log"
}
got="x86:
$(build_sb x86_64 '' build)
arm:
$(build_sb aarch64 '' build)
arm, no dpdk:
$(build_sb aarch64 1 build)
base:
$(build_sb x86_64 '' build-base)"
want="x86:
ergon-md-dpdk static,dpdk --bin media_driver
ergon-md-dpdk-harness static,dpdk --bin rtt --bin load --bin tput --bin floor --bin counters --bin clockcheck
arm:
ergon-md-dpdk static,dpdk --bin media_driver
ergon-md-dpdk-harness static,dpdk --bin rtt --bin load --bin tput --bin floor --bin counters --bin clockcheck
arm, no dpdk:
ergon-md-dpdk static,dpdk --bin media_driver
ergon-md-dpdk static --bin media_driver
ergon-md-dpdk-harness static --bin rtt --bin load --bin tput --bin floor --bin counters --bin clockcheck
base:
ergon-md-dpdk static,dpdk --bin media_driver
ergon-md-dpdk-harness static,dpdk --bin rtt --bin load --bin tput"
if [[ $got != "$want" ]]; then
    fail "vm.sh build: the clients' features"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
else
    pass "vm.sh build and build-base: the clients built with the media driver's features, DPDK's wherever it has them (INVOKER embeds it)"
fi

# vm.sh build-dpdk <build> in a sandbox (vm.sh build runs it for each build
# bench/want_dpdk names: lib.sh, from the modes' @DPDK=): DPDK 25.11 from
# fast.dpdk.org, unpacked afresh; patched-refill with
# dpdk-ena-deferred-refill.patch applied exactly (patch -p1, no fuzz), each
# under its own prefix (stock's the precision-time build's), built as
# build-ptp builds it (meson: the ENA path only), never put in ld.so.conf;
# then the media driver and every benchmark client linked against it in a
# target directory of their own, its library directory their RPATH
# (DT_RPATH, which their transitive libraries and the PMDs the EAL loads
# follow too), every librte_* checked to load from it (fail closed: the build
# directory removed), into ~/bin-dpdk-<build>. Again with the same patch:
# DPDK not rebuilt; with another patch: rebuilt. Only those two builds, on
# AWS only.
sbp=$tmp/vm-build-dpdk
mkdir -p "$sbp/bin" "$sbp/home/ergon/md-dpdk/harness" "$sbp/home/bench" "$sbp/home/dpdk-kmods" "$sbp/opt" "$sbp/tarball/dpdk-25.11"
cp "$sb/bin/uname" "$sb/bin/nproc" "$sbp/bin/"
cp "$cloud/../../harness/Cargo.toml" "$sbp/home/ergon/md-dpdk/harness/Cargo.toml"
echo 'project dpdk' >"$sbp/tarball/dpdk-25.11/meson.build"
(cd "$sbp/tarball" && tar -cJf ../dpdk-25.11.tar.xz dpdk-25.11)
# shellcheck disable=SC2016
{
    printf '#!/usr/bin/env bash\necho "sudo $*" >>"$FAKE_LOG"\ncase $1 in ninja) "$@" ;; tee) case $2 in "$LAB_OPT_ROOT"/*) "$@" >/dev/null ;; *) cat >/dev/null ;; esac ;; esac\nexit 0\n' >"$sbp/bin/sudo"
    printf '#!/bin/sh\necho "curl $*" >>"$FAKE_LOG"\ncat "$FAKE_TARBALL"\n' >"$sbp/bin/curl"
    printf '#!/bin/sh\nif cmp -s - "$FAKE_PATCH"; then p=the-patch; else p=other; fi\necho "patch $* in ${PWD##*/} <$p" >>"$FAKE_LOG"\n' >"$sbp/bin/patch"
    printf '#!/bin/sh\necho "meson $* in ${PWD##*/}" >>"$FAKE_LOG"\nmkdir -p build\nfor a; do case $a in -Dprefix=*) echo "${a#-Dprefix=}" >build/prefix ;; esac; done\n' >"$sbp/bin/meson"
    printf '#!/bin/sh\necho "ninja $*" >>"$FAKE_LOG"\nif [ "$3" = install ]; then d=$(cat build/prefix)/lib/x86_64-linux-gnu; mkdir -p "$d/pkgconfig"; : >"$d/pkgconfig/libdpdk.pc"; fi\n' >"$sbp/bin/ninja"
    printf '#!/bin/sh\necho "cargo $* [PKG_CONFIG_PATH=$PKG_CONFIG_PATH CARGO_TARGET_DIR=$CARGO_TARGET_DIR]" >>"$FAKE_LOG"\nb=; n=; for a; do [ "$n" = 1 ] && b=$a; n=; [ "$a" = --bin ] && n=1; done\nd=${CARGO_TARGET_DIR:-target}/release; mkdir -p "$d"; [ -z "$b" ] || { : >"$d/$b"; chmod +x "$d/$b"; }\nexit 0\n' >"$sbp/bin/cargo"
    printf '#!/bin/sh\nprintf "\\tlinux-vdso.so.1 (0x0)\\n\\tlibrte_eal.so.26 => %%s/librte_eal.so.26 (0x0)\\n\\tlibrte_kvargs.so.26 => %%s/librte_kvargs.so.26 (0x0)\\n\\tlibc.so.6 => /lib/libc.so.6 (0x0)\\n" "$FAKE_EAL_DIR" "${FAKE_KVARGS_DIR:-$FAKE_EAL_DIR}"\n' >"$sbp/bin/ldd"
    printf '#!/bin/sh\nexit 0\n' >"$sbp/bin/make"
    printf '#!/bin/sh\nexit 0\n' >"$sbp/bin/rustup"
}
chmod +x "$sbp/bin/"*
dpdk_sb() { # <cloud> <placement> <eal dir> <vm.sh args>...: "<rc> | <the calls, one a line>" with the sandbox's path as <sb>
    local rc=0
    : >"$sbp/log"
    echo "$1" >"$sbp/home/bench/cloud"
    echo "$2" >"$sbp/home/bench/placement"
    env -i PATH="$sbp/bin:$PATH" HOME="$sbp/home" FAKE_LOG="$sbp/log" FAKE_ARCH=x86_64 FAKE_TARBALL="$sbp/dpdk-25.11.tar.xz" \
        FAKE_PATCH="$cloud/dpdk-ena-deferred-refill.patch" FAKE_EAL_DIR="$3" FAKE_KVARGS_DIR="${FAKE_KVARGS_DIR:-}" \
        LAB_OPT_ROOT="$sbp/opt" LAB_SYS_ROOT="$sys/aws-x86" bash "$cloud/vm.sh" "${@:4}" >"$sbp/out" 2>&1 || rc=$?
    echo "rc $rc$( ((rc)) && echo " ($(tail -1 "$sbp/out"))")"
    sed "s|$sbp|<sb>|g" "$sbp/log"
}
lib=$sbp/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu
patch_sum=$(sha256sum "$cloud/dpdk-ena-deferred-refill.patch" | cut -d' ' -f1)
got="patched:
$(dpdk_sb aws cluster "$lib" build-dpdk patched-refill)
bins: $(ls "$sbp/home/bin-dpdk-patched-refill" 2>/dev/null | tr '\n' ' ')ld.so.conf: $(grep -c ld.so.conf "$sbp/log" || true)
again:
$(dpdk_sb aws cluster "$lib" build-dpdk patched-refill | grep -cE '^(curl|patch|meson|ninja|sudo ninja) ' || true) DPDK steps, $(grep -c '^cargo rustc' "$sbp/log" || true) links"
echo stale >"$sbp/opt/dpdk-25.11-patched-refill/ergon-lab-patch.sha256"
got+="
another patch: $(dpdk_sb aws cluster "$lib" build-dpdk patched-refill | grep -cE '^(curl|patch|meson) ' || true) DPDK steps
stock:
$(dpdk_sb aws cluster "$sbp/opt/dpdk-25.11/lib/x86_64-linux-gnu" build-dpdk stock | grep -vE '^cargo rustc --release -p ergon-md-dpdk-harness ' || true)
host: $(dpdk_sb aws cluster "$lib" host >/dev/null; grep '^dpdk-build ' "$sbp/out" | tr '\n' ';')
bad kvargs: $(FAKE_KVARGS_DIR=/opt/dpdk-25.11/lib/x86_64-linux-gnu dpdk_sb aws cluster "$lib" build-dpdk patched-refill | head -1) $( (ls -d "$sbp/home/bin-dpdk-patched-refill" 2>/dev/null || true) | wc -l | tr -d ' ') bin dir
host after it: $(dpdk_sb aws cluster "$lib" host >/dev/null; grep '^dpdk-build ' "$sbp/out" | tr '\n' ';')
unknown: $(dpdk_sb aws cluster "$lib" build-dpdk refill | head -1)
azure: $(dpdk_sb azure '' "$lib" build-dpdk patched-refill | head -1)"
echo patched-refill >"$sbp/home/bench/want_dpdk"
got+="
build: $(dpdk_sb aws cluster "$lib" build | sed -nE 's/^cargo (build|rustc) --release -p ([^ ]+) .*--bin ([a-z_]+).*/\1 \2 \3/p' | tr '\n' ';')"
rm "$sbp/home/bench/want_dpdk"
got+="
build, none wanted: $(dpdk_sb aws cluster "$lib" build | grep -c '^cargo rustc' || true)"
want="patched:
rc 0
sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq meson ninja-build python3-pyelftools libnuma-dev
curl -fsSL https://fast.dpdk.org/rel/dpdk-25.11.tar.xz
patch -p1 --forward --fuzz=0 in dpdk-25.11-patched-refill <the-patch
sudo rm -rf <sb>/opt/dpdk-25.11-patched-refill
meson setup -Dprefix=<sb>/opt/dpdk-25.11-patched-refill -Dtests=false -Ddisable_apps=* -Denable_drivers=bus/pci,bus/vdev,mempool/ring,net/ena build in dpdk-25.11-patched-refill
ninja -C build
sudo ninja -C build install
ninja -C build install
sudo tee <sb>/opt/dpdk-25.11-patched-refill/ergon-lab-patch.sha256
cargo rustc --release -p ergon-md-dpdk --features static,dpdk --bin media_driver -- -C link-arg=-Wl,--disable-new-dtags,-rpath,<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu [PKG_CONFIG_PATH=<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu/pkgconfig CARGO_TARGET_DIR=<sb>/home/ergon/target-dpdk]
cargo rustc --release -p ergon-md-dpdk-harness --features static,dpdk --bin rtt -- -C link-arg=-Wl,--disable-new-dtags,-rpath,<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu [PKG_CONFIG_PATH=<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu/pkgconfig CARGO_TARGET_DIR=<sb>/home/ergon/target-dpdk]
cargo rustc --release -p ergon-md-dpdk-harness --features static,dpdk --bin load -- -C link-arg=-Wl,--disable-new-dtags,-rpath,<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu [PKG_CONFIG_PATH=<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu/pkgconfig CARGO_TARGET_DIR=<sb>/home/ergon/target-dpdk]
cargo rustc --release -p ergon-md-dpdk-harness --features static,dpdk --bin tput -- -C link-arg=-Wl,--disable-new-dtags,-rpath,<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu [PKG_CONFIG_PATH=<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu/pkgconfig CARGO_TARGET_DIR=<sb>/home/ergon/target-dpdk]
cargo rustc --release -p ergon-md-dpdk-harness --features static,dpdk --bin floor -- -C link-arg=-Wl,--disable-new-dtags,-rpath,<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu [PKG_CONFIG_PATH=<sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu/pkgconfig CARGO_TARGET_DIR=<sb>/home/ergon/target-dpdk]
bins: floor load media_driver rtt tput ld.so.conf: 0
again:
0 DPDK steps, 5 links
another patch: 3 DPDK steps
stock:
rc 0
sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq meson ninja-build python3-pyelftools libnuma-dev
curl -fsSL https://fast.dpdk.org/rel/dpdk-25.11.tar.xz
sudo rm -rf <sb>/opt/dpdk-25.11
meson setup -Dprefix=<sb>/opt/dpdk-25.11 -Dtests=false -Ddisable_apps=* -Denable_drivers=bus/pci,bus/vdev,mempool/ring,net/ena build in dpdk-25.11
ninja -C build
sudo ninja -C build install
ninja -C build install
cargo rustc --release -p ergon-md-dpdk --features static,dpdk --bin media_driver -- -C link-arg=-Wl,--disable-new-dtags,-rpath,<sb>/opt/dpdk-25.11/lib/x86_64-linux-gnu [PKG_CONFIG_PATH=<sb>/opt/dpdk-25.11/lib/x86_64-linux-gnu/pkgconfig CARGO_TARGET_DIR=<sb>/home/ergon/target-dpdk]
host: dpdk-build patched-refill: DPDK 25.11 in <sb>/opt/dpdk-25.11-patched-refill, dpdk-ena-deferred-refill.patch sha256 $patch_sum: <sb>/home/bin-dpdk-patched-refill;dpdk-build stock: DPDK 25.11 in <sb>/opt/dpdk-25.11: <sb>/home/bin-dpdk-stock;
bad kvargs: rc 1 (build-dpdk patched-refill: <sb>/home/bin-dpdk-patched-refill/floor loads librte_kvargs.so.26 from /opt/dpdk-25.11/lib/x86_64-linux-gnu/librte_kvargs.so.26, not <sb>/opt/dpdk-25.11-patched-refill/lib/x86_64-linux-gnu: removed) 0 bin dir
host after it: dpdk-build stock: DPDK 25.11 in <sb>/opt/dpdk-25.11: <sb>/home/bin-dpdk-stock;
unknown: rc 2 (build-dpdk refill: stock (DPDK 25.11 from source) or patched-refill (with dpdk-ena-deferred-refill.patch on ENA's PMD))
azure: rc 2 (build-dpdk patched-refill: AWS only (a build of ENA's PMD))
build: build ergon-md-dpdk media_driver;build ergon-md-dpdk-harness clockcheck;rustc ergon-md-dpdk media_driver;rustc ergon-md-dpdk-harness rtt;rustc ergon-md-dpdk-harness load;rustc ergon-md-dpdk-harness tput;rustc ergon-md-dpdk-harness floor;
build, none wanted: 0"
got=${got//$sbp/<sb>}
if [[ $got != "$want" ]]; then
    fail "vm.sh build-dpdk: DPDK 25.11 from source, patched or stock, and the driver and clients linked against it"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
else
    pass "vm.sh build-dpdk: DPDK 25.11 from source (patched-refill: the patch applied exactly), its own prefix, never in ld.so.conf, idempotent by the patch's hash; the driver and every client linked with its RPATH, each librte_* checked to load from it (fail closed); each build's DPDK and patch in vm.sh host (the env file); run by build for each wanted build, AWS only"
fi

# vm.sh ready <mode>... in a sandbox (lib.sh asks it after the build, before
# any state): what each mode's settings need of the host, refused with why:
# @IRQ=spare and @NAPI_BUSY=spare a fifth core (not c8g.xlarge's four);
# @NAPI_BUSY Linux 6.17 or later and the kernel's ynl (vm.sh build-ynl's,
# with the reason it is missing); nothing asked of the other modes
sbr=$tmp/vm-ready
mkdir -p "$sbr/bin" "$sbr/home/ergon" "$sbr/home/bench"
cp "$sb/bin/"* "$sbr/bin/"
# shellcheck disable=SC2016
printf '#!/bin/sh\ncase "$1" in -m) echo "$FAKE_ARCH" ;; -r) echo "$FAKE_KREL" ;; *) echo Linux ;; esac\n' >"$sbr/bin/uname"
chmod +x "$sbr/bin/uname"
echo aws >"$sbr/home/bench/cloud"
ready_sb() { # <sys root> <kernel release> <vm.sh ready args>...: "<rc> <output on one line>"
    local rc=0 out
    out=$(env -i PATH="$sbr/bin:$PATH" HOME="$sbr/home" FAKE_LOG="$sbr/log" FAKE_ARCH=x86_64 FAKE_KREL="$2" \
        LAB_SYS_ROOT="$sys/$1" bash "$cloud/vm.sh" ready "${@:3}" 2>&1) || rc=$?
    echo "$rc $(tr '\n' ' ' <<<"$out")" | sed 's/ *$//'
}
got="four cores: $(ready_sb c8g-xl 7.2.6+deb13-cloud-arm64 kernel@IRQ=spare dpdk)
five cores: $(ready_sb aws-x86 7.2.6+deb13-cloud-amd64 kernel@IRQ=spare dpdk kernel@IRQ=receiver kernel@NAPI_DEFER=2/200000)
busy, 6.12: $(ready_sb aws-x86 6.12.43+deb13-cloud-amd64 kernel@NAPI_BUSY=spare)"
echo "linux-source-7.2 has no 7.2.6 in the archive (only 7.2.5-1~bpo13+1)" >"$sbr/home/bench/ynl_unavailable"
got+="
busy, no ynl: $(ready_sb aws-x86 7.2.6+deb13-cloud-amd64 kernel@NAPI_BUSY=spare)"
rm "$sbr/home/bench/ynl_unavailable"
printf '#!/bin/sh\nexit 0\n' >"$sbr/home/bench/ynl"
chmod +x "$sbr/home/bench/ynl"
got+="
busy: $(ready_sb aws-x86 7.2.6+deb13-cloud-amd64 kernel-busy@NAPI_BUSY=spare kernel)
none: $(ready_sb c8g-xl 6.12.43+deb13-cloud-arm64 kernel dpdk kernel@IRQ=receiver)"
got+="
dpdk build missing: $(ready_sb aws-x86 7.2.6+deb13-cloud-amd64 dpdk@DPDK=patched-refill dpdk)"
mkdir -p "$sbr/home/bin-dpdk-patched-refill"
printf '#!/bin/sh\nexit 0\n' >"$sbr/home/bin-dpdk-patched-refill/media_driver"
chmod +x "$sbr/home/bin-dpdk-patched-refill/media_driver"
got+="
dpdk build: $(ready_sb aws-x86 7.2.6+deb13-cloud-amd64 dpdk@DPDK=patched-refill+idle dpdk)"
want="four cores: 2 mode kernel@IRQ=spare: spare: no fifth core on this host (its layout 0 1 2 3: four cores), so no spare one: a host with five cores or more (c7i.4xlarge, c8g.2xlarge and up)
five cores: 0
busy, 6.12: 2 mode kernel@NAPI_BUSY=spare: threaded NAPI busy polling needs Linux 6.17 or later (this is 6.12.43+deb13-cloud-amd64)
busy, no ynl: 2 mode kernel@NAPI_BUSY=spare: no ynl, the kernel's netlink tool (vm.sh build-ynl): linux-source-7.2 has no 7.2.6 in the archive (only 7.2.5-1~bpo13+1)
busy: 0
none: 0
dpdk build missing: 2 mode dpdk@DPDK=patched-refill: no DPDK=patched-refill build: no media_driver in $sbr/home/bin-dpdk-patched-refill (vm.sh build-dpdk patched-refill, which vm.sh build runs for each build bench/want_dpdk names)
dpdk build: 0"
if [[ $got != "$want" ]]; then
    fail "vm.sh ready: each mode's needs of the host"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
else
    pass "vm.sh ready: a fifth core for the spare core's modes, Linux 6.17 and the kernel's ynl for NAPI_BUSY, a @DPDK= mode's build, said before any state"
fi

# vm.sh driver's @DPDK=<build> (host_setting): the media driver and clients
# from ~/bin-dpdk-<build> (as @BIN= would take them; `stop` forgets both),
# in DPDK modes only, never beside @BIN=, refused without the build; and
# once the driver has started (dpdk_variant_check), its log shows the ENA
# PMD it loaded: patched-refill's line on its RX queues, or none of them
# under stock, else the run stops (fail closed); nothing asked without
# @DPDK=
dv=$tmp/vm-dpdk-setting
mkdir -p "$dv/home/bench" "$dv/home/bin-dpdk-patched-refill"
printf '#!/bin/sh\nexit 0\n' >"$dv/home/bin-dpdk-patched-refill/media_driver"
chmod +x "$dv/home/bin-dpdk-patched-refill/media_driver"
printf 'ENA_INIT: ena_rx_queue_setup(): ergon-lab deferred-refill ENA PMD: Rx queue 0 refills after a burst that returns frames only at 512 free descriptors (threshold 256)\nENA_INIT: ena_rx_queue_setup(): ergon-lab deferred-refill ENA PMD: Rx queue 0 refills after a burst that returns frames only at 512 free descriptors (threshold 256)\nmedia driver started\n' >"$dv/patched.log"
printf 'ENA_DRIVER: ena_set_queues_placement_policy(): Placement policy: Low latency\nmedia driver started\n' >"$dv/stock.log"
setv=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    eval "$(for f in save_host host_setting restore_host dpdk_variant_check; do vm_fn "$f"; done)"
    HOME=$dv/home run=$dv/home/bench CLOUD=aws ARCH=x86_64
    log() { echo "log: $*" >&2; }
    sudo() { :; }
    for case in "patched-refill dpdk patched.log" "patched-refill dpdk stock.log" "stock dpdk stock.log" "patched-refill kernel -" "refill dpdk -"; do
        read -r v mode logf <<<"$case"
        [[ $v == stock && $logf == stock.log ]] && { mkdir -p "$dv/home/bin-dpdk-stock"; cp "$dv/home/bin-dpdk-patched-refill/media_driver" "$dv/home/bin-dpdk-stock/"; }
        out=$( (host_setting DPDK "$v" "$mode" && { [[ $logf == - ]] || dpdk_variant_check "$dv/$logf"; }) 2>&1)
        rc=$?
        echo "$v $mode $logf: rc $rc bin [$(cat "$run/bin" 2>/dev/null)] build [$(cat "$run/dpdk_variant" 2>/dev/null)] | $(tr '\n' ' ' <<<"$out")"
        restore_host >/dev/null 2>&1
        echo "  undone: [$(cd "$run" && ls bin dpdk_variant restore/* 2>/dev/null | tr '\n' ' ')]"
    done
    echo "stock on the patched PMD: $( (printf 'stock\n' >"$run/dpdk_variant"; dpdk_variant_check "$dv/patched.log") 2>&1; echo "rc $?")"
    rm -f "$run/dpdk_variant"
    echo "none: $(dpdk_variant_check "$dv/patched.log" 2>&1; echo "rc $?")"
    rm -rf "$dv/home/bin-dpdk-stock"
    echo "no build: $( (host_setting DPDK stock dpdk) 2>&1; echo "rc $?")"
    echo "$dv/home/bin-base" >"$run/bin"
    echo "with BIN: $( (host_setting DPDK patched-refill dpdk) 2>&1; echo "rc $?")"
    rm -f "$run/bin"
)
setv=${setv//$dv/<dv>}
want="patched-refill dpdk patched.log: rc 0 bin [<dv>/home/bin-dpdk-patched-refill] build [patched-refill] | log: DPDK=patched-refill: the media driver and clients from <dv>/home/bin-dpdk-patched-refill log: DPDK=patched-refill: the patched ENA PMD on 2 RX queues (<dv>/patched.log) 
  undone: []
patched-refill dpdk stock.log: rc 1 bin [<dv>/home/bin-dpdk-patched-refill] build [patched-refill] | log: DPDK=patched-refill: the media driver and clients from <dv>/home/bin-dpdk-patched-refill DPDK=patched-refill: no RX queue on the patched ENA PMD (no 'ergon-lab deferred-refill ENA PMD' line in <dv>/stock.log): stopping 
  undone: []
stock dpdk stock.log: rc 0 bin [<dv>/home/bin-dpdk-stock] build [stock] | log: DPDK=stock: the media driver and clients from <dv>/home/bin-dpdk-stock log: DPDK=stock: the stock ENA PMD (no patched RX queue in <dv>/stock.log) 
  undone: []
patched-refill kernel -: rc 2 bin [] build [] | DPDK=patched-refill: DPDK modes only (the media driver and clients linked against that DPDK build) 
  undone: []
refill dpdk -: rc 2 bin [] build [] | DPDK=refill: stock or patched-refill 
  undone: []
stock on the patched PMD: DPDK=stock: 2 RX queues on the patched ENA PMD in <dv>/patched.log, not stock's: stopping
rc 1
none: rc 0
no build: DPDK=stock: no media_driver in <dv>/home/bin-dpdk-stock (vm.sh build-dpdk stock)
rc 2
with BIN: DPDK=patched-refill with BIN=: one build a mode
rc 2"
if [[ $setv != "$want" ]]; then
    fail "vm.sh: @DPDK= takes its build's binaries and checks the PMD loaded"
    diff -u <(echo "$want") <(echo "$setv") >&2 || true
else
    pass "vm.sh: @DPDK= runs its build's driver and clients (DPDK modes only, not beside @BIN=, refused without the build), undone by stop; the PMD the driver's log shows is checked against it (fail closed)"
fi
# ...and what a run keeps of it: vm.sh pmd-log lists the patched PMD's line
# with ENA's others, and vm.sh counters keeps its two RX queue counters
# (rx_q<n>_refill_deferred, rx_q<n>_refill_floor; type 92, the PMD's xstats
# the transport mirrors), not ENA's other RX queue counters
cp "$dv/patched.log" "$sbr/home/bench/driver.log"
pmdlog=$(env -i PATH="$sbr/bin:$PATH" HOME="$sbr/home" FAKE_LOG="$sbr/log" FAKE_ARCH=x86_64 FAKE_KREL=7.2.6+deb13-cloud-amd64 \
    LAB_SYS_ROOT="$sys/aws-x86" bash "$cloud/vm.sh" pmd-log 2>&1 | grep -c 'ergon-lab deferred-refill ENA PMD' || true)
rm -f "$sbr/home/bench/driver.log"
pattern=$(sed -n '/^counters)$/,/^    ;;$/s/^    pattern=//p' "$cloud/vm.sh" || true)
eval "pattern=$pattern"
kept=$(printf '%s\n' '  12: 41 92 role=receiver rx_q0_refill_deferred' '  13: 2 92 role=receiver rx_q0_refill_floor' '  14: 0 92 role=receiver rx_q0_refill_partial' '  15: 900 92 role=receiver rx_q0_cnt' | awk -v pat="$pattern" '$0 ~ pat {print $NF}' | tr '\n' ' ' || true)
if [[ $pmdlog != 2 || $kept != "rx_q0_refill_deferred rx_q0_refill_floor " ]]; then
    fail "vm.sh pmd-log and counters: the patched PMD's line ($pmdlog) and counters ($kept)"
else
    pass "vm.sh pmd-log lists the patched PMD's line; counters keeps its RX queue counters (refill_deferred, refill_floor) and no other RX queue counter"
fi

# vm.sh build-ynl in a sandbox (vm.sh build runs it where bench/want_ynl is
# 1): the kernel's own ynl (no Debian package ships it) from Debian's source
# of the running kernel, linux-source-<major>.<minor> at the archive's
# version of exactly the running release (apt-cache madison; fake), so the
# tool and netdev's spec match the kernel's netlink family: installed, the
# tool and the specs taken out of its tarball (a fake one here, its ynl
# printing what it was given), the package purged, a wrapper (bench/ynl)
# running it with netdev's spec, the version recorded. Fails closed, the
# reason in bench/ynl_unavailable for vm.sh ready: no source of that release
# in the archive, or a spec without threaded busy-poll
sby=$tmp/vm-ynl
mkdir -p "$sby/bin" "$sby/home/ergon" "$sby/home/bench" "$sby/src" "$sby/tree/linux-source-7.2/tools/net/ynl/pyynl/lib" \
    "$sby/tree/linux-source-7.2/Documentation/netlink/specs" "$sby/tree/linux-source-7.2/kernel"
cp "$sbr/bin/"* "$sby/bin/"
# shellcheck disable=SC2016
printf '#!/bin/sh\necho "apt-cache $*" >>"$FAKE_LOG"\nprintf "%%s\\n" "$FAKE_MADISON"\n' >"$sby/bin/apt-cache"
chmod +x "$sby/bin/apt-cache"
echo aws >"$sby/home/bench/cloud"
printf 'import sys\nprint("ynl", *sys.argv[1:])\n' >"$sby/tree/linux-source-7.2/tools/net/ynl/pyynl/cli.py"
: >"$sby/tree/linux-source-7.2/tools/net/ynl/pyynl/lib/__init__.py"
printf 'name: genetlink\n' >"$sby/tree/linux-source-7.2/Documentation/netlink/genetlink.yaml"
printf 'name: netdev\n  -\n    name: napi-threaded\n    entries: [disabled, enabled, busy-poll]\n' >"$sby/tree/linux-source-7.2/Documentation/netlink/specs/netdev.yaml"
echo 'not wanted' >"$sby/tree/linux-source-7.2/kernel/sched.c"
tar -C "$sby/tree" -cJf "$sby/src/linux-source-7.2.tar.xz" linux-source-7.2
sed 's/busy-poll//' "$sby/tree/linux-source-7.2/Documentation/netlink/specs/netdev.yaml" >"$sby/old-netdev.yaml"
madison=" linux-source-7.2 | 7.2.6-1~bpo13+1 | http://deb.debian.org/debian trixie-backports/main amd64 Packages"
ynl_sb() { # <madison lines>: vm.sh build-ynl, then "<rc>|<sudo apt calls>|<the wrapper's run>|<version>|<why unavailable>"
    : >"$sby/log"
    rm -rf "$sby/home/ynl" "$sby/home/bench/ynl"*
    local rc=0
    env -i PATH="$sby/bin:$PATH" HOME="$sby/home" FAKE_LOG="$sby/log" FAKE_ARCH=x86_64 FAKE_KREL=7.2.6+deb13-cloud-amd64 \
        FAKE_MADISON="$1" LAB_SYS_ROOT="$sys/aws-x86" LAB_SRC_ROOT="$sby/src" bash "$cloud/vm.sh" build-ynl >"$sby/out" 2>&1 || rc=$?
    echo "$rc|$(grep -E '^sudo .*apt-get (install|purge)' "$sby/log" | tr '\n' ';')|$([[ -x $sby/home/bench/ynl ]] && "$sby/home/bench/ynl" --do napi-get 2>&1 | sed "s#$sby/home#~#g")|$(cat "$sby/home/bench/ynl.version" 2>/dev/null)|$(cat "$sby/home/bench/ynl_unavailable" 2>/dev/null)"
}
got="ok: $(ynl_sb "$madison")
extracted: $(cd "$sby/home/ynl" 2>/dev/null && find . -type f | sort | paste -sd' ' -)
older only: $(ynl_sb " linux-source-7.2 | 7.2.5-1~bpo13+1 | http://deb.debian.org/debian trixie-backports/main amd64 Packages")"
cp "$sby/old-netdev.yaml" "$sby/tree/linux-source-7.2/Documentation/netlink/specs/netdev.yaml"
tar -C "$sby/tree" -cJf "$sby/src/linux-source-7.2.tar.xz" linux-source-7.2
got+="
no busy-poll: $(ynl_sb "$madison")"
want="ok: 0|sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq --no-install-recommends linux-source-7.2=7.2.6-1~bpo13+1 python3-yaml python3-jsonschema;sudo DEBIAN_FRONTEND=noninteractive apt-get purge -yq linux-source-7.2;|ynl --spec ~/ynl/Documentation/netlink/specs/netdev.yaml --do napi-get|linux-source-7.2 7.2.6-1~bpo13+1 (Linux 7.2.6+deb13-cloud-amd64): tools/net/ynl and Documentation/netlink|
extracted: ./Documentation/netlink/genetlink.yaml ./Documentation/netlink/specs/netdev.yaml ./tools/net/ynl/pyynl/cli.py ./tools/net/ynl/pyynl/lib/__init__.py
older only: 1||||linux-source-7.2 has no 7.2.6 (the running kernel's release) in the archive: only 7.2.5-1~bpo13+1
no busy-poll: 1|sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq --no-install-recommends linux-source-7.2=7.2.6-1~bpo13+1 python3-yaml python3-jsonschema;sudo DEBIAN_FRONTEND=noninteractive apt-get purge -yq linux-source-7.2;|||the netdev spec of linux-source-7.2 7.2.6-1~bpo13+1 has no threaded busy-poll (Linux 6.17 and later)"
if [[ $got != "$want" ]]; then
    fail "vm.sh build-ynl: the kernel's ynl from Debian's source of the running kernel"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
else
    pass "vm.sh build-ynl: ynl and netdev's spec from linux-source at exactly the running kernel's release, a wrapper, fail closed with why"
fi

# vm.sh jitter with no media driver process (INVOKER's case): every measured
# CPU's interrupts still listed, from its own column (the fake /proc above)
# (irqs.sh's fake /proc: four CPUs; CPU4, the layout's fifth, not listed)
printf 'cpu  1 2 3 4 5 6 7 8 0 0\ncpu0 1 2 3 4 5 6 7 11 0 0\ncpu2 1 2 3 4 5 6 7 13 0 0\n' >"$tmp/proc/stat"
jit=$(env -i PATH="$sbc/bin:$PATH" HOME="$sbc/home" FAKE_LOG="$sbc/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" \
    LAB_PROC_ROOT="$tmp/proc" bash "$cloud/vm.sh" jitter 2>/dev/null) || true
if [[ $jit == $'cpu 0 irqs=251 timer=100 steal=11\ncpu 1 irqs=216 timer=200 steal=\ncpu 2 irqs=360 timer=300 steal=13\ncpu 3 irqs=424 timer=400 steal=\ncpu 4  steal=' ]]; then
    pass "vm.sh jitter without a media driver: every measured CPU's interrupts"
else
    fail "vm.sh jitter without a media driver: $jit"
fi

# vm.sh counters under INVOKER with DPDK before the mode's first client (AWS
# round A's c8g pair: "vm.sh counters failed"): no media driver process and no
# embedded driver's CnC yet, and the data NICs are DPDK's (igb_uio), so
# ethtool has nothing to read: the snapshot names no driver ("cnc none"),
# which analyse.py counts as zero, and succeeds. A sudo that runs `test` for
# real (the CnC is absent) and logs everything else.
sbk=$tmp/vm-counters
mkdir -p "$sbk/bin" "$sbk/home/ergon" "$sbk/home/bench" "$sbk/sys"
cp "$sb/bin/"* "$sbk/bin/"
cat >"$sbk/bin/sudo" <<'SH'
#!/usr/bin/env bash
echo "sudo $*" >>"$FAKE_LOG"
[[ $1 == test ]] && exec "$@"
exit 0
SH
printf '#!/bin/sh\necho "ethtool $*" >>"$FAKE_LOG"\nexit 1\n' >"$sbk/bin/ethtool"
chmod +x "$sbk/bin/sudo" "$sbk/bin/ethtool"
cp -R "$sys/aws-x86/." "$sbk/sys/"
mkdir -p "$sbk/sys/bus/pci/drivers/igb_uio/0000:00:06.0"
printf "export AERON_THREADING_MODE='INVOKER'\nexport RUSTERON_MEDIA_DRIVER_TRANSPORT='dpdk-ena'\n" >"$sbk/home/bench/driver.env"
echo invoker >"$sbk/home/bench/harness_driver"
echo aws >"$sbk/home/bench/cloud"
: >"$sbk/log"
cnt=$(env -i PATH="$sbk/bin:$PATH" HOME="$sbk/home" FAKE_LOG="$sbk/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sbk/sys" \
    LAB_PROC_ROOT="$tmp/proc" bash "$cloud/vm.sh" counters 2>&1; echo "rc $?")
if [[ $cnt == $'cnc none\n\nrc 0' ]] && ! grep -q '^ethtool' "$sbk/log"; then
    pass "vm.sh counters under INVOKER with DPDK before the first client: no driver yet, \"cnc none\" (zero), no ethtool on DPDK's NICs"
else
    fail "vm.sh counters under INVOKER with DPDK before the first client: [$cnt] $(grep -m2 -E '^(ethtool|sudo)' "$sbk/log" | tr '\n' ' ')"
fi

# vm.sh driver's environment names the socket transport's data NICs
# (RUSTERON_MEDIA_DRIVER_DATA_NICS) in the kernel modes, so the media
# driver's NIC checks run there (T2, T6, N1, N2, N4, and their re-check
# after an ENA reset): each data NIC's kernel interface and, on Azure, its
# VF beneath it, comma-separated, never the management NIC; nothing, and a
# failure, where the data NICs cannot be read. The kernel modes' branch of
# `driver` writes it to driver.env, and nothing else does (under DPDK the
# data NICs are the ports, by PCI address, and the variable is ignored)
datanics=$(
    set +e
    exec 2>&1
    # (data_ifaces is a one-line function)
    eval "$(grep '^data_ifaces() {' "$cloud/vm.sh"; vm_fn data_nics_env)"
    nics() { printf 'sender ens6 ens6 0a:00:00:00:00:01 10.30.1.5 24 0000:00:06.0\nreceiver ens7 ens7 0a:00:00:00:00:02 10.30.1.6 24 0000:00:07.0\n'; }
    echo "aws: $(data_nics_env); rc $?"
    nics() { printf 'sender eth1 enP30832s1 00:0d:3a:00:00:01 10.20.1.4 24 7270:00:02.0\nreceiver eth2 enP30833s2 00:0d:3a:00:00:02 10.20.1.5 24 7271:00:02.0\n'; }
    echo "azure: $(data_nics_env); rc $?"
    nics() { echo "expected two data NICs on 10.20.1.0/24, found: " >&2; return 1; }
    echo "unread: [$(data_nics_env 2>/dev/null)]; rc $?"
    kernel=$(awk '/^    kernel \| kernel-busy\)$/ {on = 1} on {print} on && /^        ;;$/ {exit}' "$cloud/vm.sh")
    echo "kernel modes: $(grep -c '^ *data_nics_env >>"$env_file" ||' <<<"$kernel"); every call: $(grep -c 'data_nics_env >>' "$cloud/vm.sh")"
)
want="aws: export RUSTERON_MEDIA_DRIVER_DATA_NICS='ens6,ens7'; rc 0
azure: export RUSTERON_MEDIA_DRIVER_DATA_NICS='eth1,enP30832s1,eth2,enP30833s2'; rc 0
unread: []; rc 1
kernel modes: 1; every call: 1"
if [[ $datanics == "$want" ]]; then
    pass "vm.sh: the kernel modes' driver environment names the data NICs (RUSTERON_MEDIA_DRIVER_DATA_NICS: each interface, Azure's VF too, never the management NIC), the DPDK mode's does not; unread NICs fail"
else
    fail "vm.sh: RUSTERON_MEDIA_DRIVER_DATA_NICS in the kernel modes' driver environment"
    diff -u <(echo "$want") <(echo "$datanics") >&2 || true
fi

# vm.sh rx-hw-delay: the receiver port's NIC-arrival-to-rx-burst counters
# (type 81, by their exact labels, never a near miss) as "<mean ns>,<max
# ns>,<frames>,<min ns>,<negative>": the mean over the frames in the sum, and
# apart the frames the NIC stamped after the host clock read at their burst
# (PHC and host clock skew). The endpoint is the receive burst that carried
# the frame, so a frame after the burst's first is measured before its own
# dispatch. A driver from before this rename still labels them
# nic-arrival-to-dispatch-*, and one older still rx-hw-delay-*: both parse,
# and the oldest gives its mean, max and frames with min and negative empty.
# Only skewed frames give ",,0,,<negative>" (a NIC clock the host is not kept
# on), and no stamped frame gives nothing. analyse.py's table then has the
# min, mean and max, and the skewed frames summed ("–" where a build did not
# count them).
sbh=$tmp/vm-rxhw
mkdir -p "$sbh/bin" "$sbh/home/ergon/target/release" "$sbh/home/bench" "$sbh/sys"
cp "$sb/bin/"* "$sbh/bin/"
cat >"$sbh/bin/sudo" <<'SH'
#!/usr/bin/env bash
echo "sudo $*" >>"$FAKE_LOG"
[[ $1 == env ]] && exec "$@"
exit 0
SH
# shellcheck disable=SC2016
printf '#!/bin/sh\n[ "$1" = -x ] && [ "$2" = media_driver ]\n' >"$sbh/bin/pgrep"
printf '#!/bin/sh\ncat "$FAKE_COUNTERS"\n' >"$sbh/home/ergon/target/release/counters"
chmod +x "$sbh/bin/sudo" "$sbh/bin/pgrep" "$sbh/home/ergon/target/release/counters"
cp -R "$sys/aws-x86/." "$sbh/sys/"
echo aws >"$sbh/home/bench/cloud"
rx=' 81 rusteron-dpdk role=receiver bdf=0000:00:07.0 port=1 q=0 dir=rx'
cat >"$tmp/rxhw-new" <<EOF
3000$rx nic-arrival-to-rx-burst-max-ns
500$rx nic-arrival-to-rx-burst-min-ns
4800$rx nic-arrival-to-rx-burst-sum-ns
4$rx nic-arrival-to-rx-burst-count
2$rx nic-arrival-to-rx-burst-negative
999$rx nic-arrival-to-rx-burst-max-ns-old
777$rx nic-arrival-to-rx-burst-min-ns-old
7 82 rusteron-dpdk role=receiver bdf=0000:00:07.0 port=1 q=0 dir=rx pkts
EOF
cat >"$tmp/rxhw-one" <<EOF
700$rx nic-arrival-to-rx-burst-max-ns
700$rx nic-arrival-to-rx-burst-min-ns
700$rx nic-arrival-to-rx-burst-sum-ns
1$rx nic-arrival-to-rx-burst-count
0$rx nic-arrival-to-rx-burst-negative
EOF
# a driver from before the rename: the same numbers under the old labels
cat >"$tmp/rxhw-prev" <<EOF
3000$rx nic-arrival-to-dispatch-max-ns
500$rx nic-arrival-to-dispatch-min-ns
4800$rx nic-arrival-to-dispatch-sum-ns
4$rx nic-arrival-to-dispatch-count
2$rx nic-arrival-to-dispatch-negative
EOF
cat >"$tmp/rxhw-old" <<EOF
2000$rx rx-hw-delay-max-ns
3000$rx rx-hw-delay-sum-ns
3$rx rx-hw-timestamped
EOF
cat >"$tmp/rxhw-skewed" <<EOF
0$rx nic-arrival-to-rx-burst-max-ns
0$rx nic-arrival-to-rx-burst-min-ns
0$rx nic-arrival-to-rx-burst-sum-ns
0$rx nic-arrival-to-rx-burst-count
5$rx nic-arrival-to-rx-burst-negative
EOF
sed 's/^5 /0 /' "$tmp/rxhw-skewed" >"$tmp/rxhw-none"
rxhw() { # <counters fixture>: "[<the line it printed> ]rc <status>"
    local said rc=0
    said=$(env -i PATH="$sbh/bin:$PATH" HOME="$sbh/home" FAKE_LOG="$sbh/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sbh/sys" \
        FAKE_COUNTERS="$tmp/rxhw-$1" bash "$cloud/vm.sh" rx-hw-delay 2>&1) || rc=$?
    echo "${said:+$said }rc $rc"
}
got="new: $(rxhw new)
one: $(rxhw one)
prev: $(rxhw prev)
old: $(rxhw old)
skewed: $(rxhw skewed)
none: $(rxhw none)"
want="new: 1200,3000,4,500,2 rc 0
one: 700,700,1,700,0 rc 0
prev: 1200,3000,4,500,2 rc 0
old: 1000,2000,3,, rc 0
skewed: ,,0,,5 rc 0
none: rc 0"
if [[ $got == "$want" ]]; then
    pass "vm.sh rx-hw-delay: NIC arrival to the receive burst by its labels (mean, max, frames, min, the skewed frames apart), the pre-rename and older labels, only skewed frames, nothing without a stamped frame"
else
    fail "vm.sh rx-hw-delay"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
fi
mkdir -p "$tmp/rxhw-run"
cat >"$tmp/rxhw-run/results.csv" <<'EOF'
1,rxhw,isolated:dpdk,1200,3000,4,500,2
2,rxhw,isolated:dpdk,1000,2600,9,700,0
3,rxhw,isolated:dpdk,,,0,,6
1,rxhw,isolated:dpdk@BIN=bin-base,1000,2000,3,,
EOF
python3 "$cloud/analyse.py" "$tmp/rxhw-run/results.csv" >"$tmp/rxhw-run/out.md" 2>&1 || fail "analyse.py with rxhw rows exited $?"
rxhw_table=$(awk '/^## NIC arrival to the receive burst/ {on = 1; next} /^## / {on = 0} on' "$tmp/rxhw-run/out.md" |
    grep '^| isolated' || true)
rxhw_want="| isolated, DPDK              | 3         | 0.60 (0.50–0.70) | 1.10 (1.00–1.20) | 2.80 (2.60–3.00) | 8                              |
| isolated, dpdk@BIN=bin-base | 1         | –                | 1.00 (1.00–1.00) | 2.00 (2.00–2.00) | –                              |"
if [[ $rxhw_table == "$rxhw_want" ]]; then
    pass "analyse.py: NIC arrival to the receive burst with its min, mean and max, and the skewed frames apart (\"–\" for a build that did not count them)"
else
    fail "analyse.py: the NIC arrival to the receive burst table"
    diff -u <(echo "$rxhw_want") <(echo "$rxhw_table") >&2 || cat "$tmp/rxhw-run/out.md" >&2
fi

# analyse.py: INVOKER named readably; and the throughput phase frames of an
# INVOKER run: each client embeds a driver whose counters start at zero, so a
# snapshot of another driver's CnC (vm.sh counters names it: "cnc <pid>-<start>")
# before the run counts as zero, and the same driver's as before
python3 - "$cloud" "$tmp/frames-invoker" <<'PY' && pass "analyse.py: INVOKER named; frames counted from zero over another embedded driver's snapshot" || fail "analyse.py: INVOKER"
import contextlib, io, os, sys
sys.path.insert(0, sys.argv[1])
import analyse
want = "isolated, DPDK, conductor, sender, receiver and the client on one thread (INVOKER)"
assert analyse.name("isolated:dpdk@AERON_THREADING_MODE=INVOKER") == want, analyse.name("isolated:dpdk@AERON_THREADING_MODE=INVOKER")
d = sys.argv[2]
state = os.path.join(d, "isolated")
os.makedirs(state)
label, fm = "isolated:dpdk@AERON_THREADING_MODE=INVOKER", "isolated-dpdk@AERON_THREADING_MODE=INVOKER"
open(os.path.join(d, "results.csv"), "w").write(f"1,tput,{label},udp,1000000,1,2,6000000,0\n")
snap = lambda cnc, pkts, rx: (f"# realtime=1\ncnc {cnc}\n"
                              f" {pkts} 82 rusteron-dpdk role=sender bdf=0000:00:06.0 port=0 q=0 dir=tx pkts; {rx} 82 rusteron-dpdk role=receiver bdf=0000:00:07.0 port=1 q=0 dir=rx pkts;\n")
files = {"a-counters-tput-before": snap("4242-1", 900000, 0), "a-counters-tput-after": snap("4300-2", 600000, 0),
         "b-counters-tput-before": snap("5000-3", 0, 100), "b-counters-tput-after": snap("5000-3", 0, 600100)}
for name, text in files.items():
    open(os.path.join(state, f"{name}-{fm}-1.log"), "w").write(text)
r = analyse.read_results(os.path.join(d, "results.csv"))
out = io.StringIO()
with contextlib.redirect_stdout(out):
    analyse.frames(r, os.path.join(d, "results.csv"))
row = [[c.strip() for c in l.split("|")[1:-1]] for l in out.getvalue().splitlines() if l.startswith("| isolated, DPDK")]
assert row, out.getvalue()
# frames sent by a: 600,000 (from zero: the snapshot before was the last client's driver) over 6 s, 0.100 M/s;
# received by b: 600,000, the same driver throughout
assert row[0][4] == "0.100" and row[0][7] == "0.100" and row[0][8] == "0", row[0]
PY

# irqs.sh against a Graviton's /proc: arm64 gives the trigger type a field of
# its own (GICv3 27 Level arch_timer; ITS-PCI-MSIX-<address> 1 Edge <name>),
# so names are read past it; its IPIs are named rows (IPI0:), not IRQs. A data
# NIC's interrupts are counted by its name and by its address, as on x86.
proc_arm=$tmp/proc-arm
mkdir -p "$proc_arm/irq"
cat >"$proc_arm/interrupts" <<'EOF'
           CPU0       CPU1       CPU2       CPU3
 10:       5000       4000       3000       2000     GICv3  27 Level     arch_timer
 14:        100          2          0          0  ITS-PCI-MSIX-0000:00:05.0   0 Edge      ena-mgmnt@pci:0000:00:05.0
 15:          0          0         50          0  ITS-PCI-MSIX-0000:00:06.0   1 Edge      ens6-Tx-Rx-0
 16:         10          0          0          7  ITS-PCI-MSIX-0000:00:07.0   1 Edge      ens7-Tx-Rx-0
IPI0:        10         20         30         40       Rescheduling interrupts
IPI1:         1          2          3          4       Function call interrupts
EOF
while read -r irq smp; do
    mkdir -p "$proc_arm/irq/$irq"
    echo "$smp" >"$proc_arm/irq/$irq/smp_affinity_list"
    echo "$smp" >"$proc_arm/irq/$irq/effective_affinity_list"
done <<'EOF'
10 0-3
14 0
15 2
16 0
EOF
irqs_arm=$(
    set +e
    exec 2>&1
    # shellcheck disable=SC2034
    LAB_PROC_ROOT=$proc_arm
    # shellcheck source=../irqs.sh
    . "$cloud/irqs.sh" || exit 1
    echo "check: $(irqs_check 0)"
    echo "nic: $(irqs_nic_off 0 ens6,ens6,0000:00:06.0 ens7,ens7,0000:00:07.0)"
) || true
want="check: 2 on CPU 0 alone, 2 elsewhere (10:arch_timer@0-3 15:ens6-Tx-Rx-0@2), default for new ones ?
nic: 57"
if [[ $irqs_arm != "$want" ]]; then
    fail "irqs.sh against a Graviton's /proc"
    diff -u <(echo "$want") <(echo "$irqs_arm") >&2 || true
else
    pass "irqs.sh against a Graviton's /proc: names past the trigger type, NIC interrupts counted"
fi

# lib.sh dry: the isolated-poll state boots isolated with idle polling (vm.sh
# isolate poll), the isolated state without, the pinned state neither; DPDK
# modes stop a run before any state when a host's media driver has no DPDK
# transport (vm.sh build on aarch64 where md-dpdk's dpdk feature does not
# build), kernel modes run on such hosts with that noted
cat >"$tmp/states.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2 repo=$out reps=1
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
python3() { :; }
remote_access() { :; }
both() { echo "both $*" >>"$out/calls"; }
reboot_both() { echo "reboot" >>"$out/calls"; }
remote_state() { echo "state $1" >>"$out/calls"; }
on() { [[ $1 == "$B" && $2 == *dpdk_unavailable* ]] && echo "md-dpdk's dpdk feature did not build on aarch64"; return 0; }
case $3 in
states) states=(pinned isolated-poll isolated) modes=(kernel); run_bench ;;
kernel) modes=(kernel kernel-busy@AERON_THREADING_MODE=SHARED) local_modes=; dpdk_ready ;;
dpdk) modes=(kernel dpdk@AERON_THREADING_MODE=SHARED_NETWORK) local_modes=; dpdk_ready ;;
local) modes=(kernel) local_modes='kernel dpdk'; dpdk_ready ;;
esac
echo "rc $?" >>"$out/calls"
SH
for case in states kernel dpdk local; do
    mkdir -p "$tmp/states-$case"
    bash "$tmp/states.sh" "$cloud" "$tmp/states-$case" "$case" </dev/null 2>"$tmp/states-$case/stderr" || true
done
isolates=$(grep -E '^(both isolate|both tune|reboot|state)' "$tmp/states-states/calls" 2>/dev/null | tr '\n' ';')
if grep -E 'unbound variable|command not found|syntax error' "$tmp"/states-*/stderr; then
    fail "lib.sh states and DPDK readiness: shell errors"
elif [[ $isolates != "state pinned;both isolate-poll isolate poll;reboot;both tune-isolated-poll tune;state isolated-poll;both isolate isolate;reboot;both tune-isolated tune;state isolated;" ]]; then
    fail "lib.sh states: $isolates"
elif [[ $(cat "$tmp/states-kernel/calls") != "rc 0" ]] || ! grep -q "no DPDK transport" "$tmp/states-kernel/log"; then
    fail "lib.sh dpdk_ready with kernel modes only: $(cat "$tmp/states-kernel/calls" "$tmp/states-kernel/log" 2>/dev/null)"
elif grep -qs '^rc 0' "$tmp/states-dpdk/calls" || ! grep -q "ERROR: .*192.0.2.2.*no DPDK transport" "$tmp/states-dpdk/log" ||
    grep -qs '^rc 0' "$tmp/states-local/calls" || ! grep -q "ERROR: .*no DPDK transport" "$tmp/states-local/log"; then
    fail "lib.sh dpdk_ready with DPDK modes: $(cat "$tmp"/states-{dpdk,local}/{calls,log} 2>/dev/null)"
else
    pass "lib.sh: isolated-poll boots with idle polling; DPDK modes stop a run on a host without DPDK, kernel modes run"
fi

# analyse.py names a threading mode and the isolated-poll state readably, and
# sorts that state after isolated; spikes.py attributes a preemption of the
# shared agents (SHARED_NETWORK's "[sender, receiver]" thread, its name cut
# to 15 characters by the kernel; SHARED's, the media driver's main thread)
python3 - "$cloud" "$tmp/diag-shared.log" <<'PY' && pass "analyse.py and spikes.py: threading modes and isolated-poll" || fail "analyse.py and spikes.py: threading modes and isolated-poll"
import sys
sys.path.insert(0, sys.argv[1])
import analyse, spikes
cases = {
    "isolated:dpdk@AERON_THREADING_MODE=SHARED_NETWORK": "isolated, DPDK, sender and receiver on one thread (SHARED_NETWORK)",
    "kernel-busy@AERON_THREADING_MODE=SHARED": "kernel UDP, busy polling, conductor, sender and receiver on one thread (SHARED)",
    "isolated:kernel@AERON_THREADING_MODE=DEDICATED": "isolated, kernel UDP, conductor, sender and receiver on a thread each (DEDICATED)",
    "isolated:kernel-busy@RX_RING=4096@AERON_THREADING_MODE=SHARED_NETWORK":
        "isolated, kernel-busy@RX_RING=4096, sender and receiver on one thread (SHARED_NETWORK)",
    "isolated:local-kernel@AERON_THREADING_MODE=SHARED": "isolated, localhost, kernel UDP, conductor, sender and receiver on one thread (SHARED)",
    "isolated-poll:kernel": "isolated-poll, kernel UDP",
    "isolated:dpdk@BIN=bin-base": "isolated, dpdk@BIN=bin-base",
    "isolated:kernel-busy": "isolated, kernel UDP, busy polling",
}
for label, want in cases.items():
    assert analyse.name(label) == want, (label, analyse.name(label), want)
# a state analyse.py does not know sorts after the ones it does, whatever its
# name ("idle" before "isolated-poll" alphabetically)
order = sorted(["idle:kernel", "isolated-poll:kernel", "isolated:kernel", "pinned:kernel"], key=analyse.key)
assert order == ["pinned:kernel", "isolated:kernel", "isolated-poll:kernel", "idle:kernel"], order
trace = [
    "  [sender, receiv-4242  [003] d..2.  100.000100: sched_switch: prev_comm=[sender, receiv prev_pid=4242 prev_prio=120 prev_state=R+ ==> next_comm=kworker/3:1 next_pid=77 next_prio=120",
    "     kworker/3:1-77     [003] d..2.  100.000150: sched_switch: prev_comm=kworker/3:1 prev_pid=77 prev_prio=120 prev_state=I ==> next_comm=[sender, receiv next_pid=4242 next_prio=120",
    "    media_driver-4100   [002] d..2.  100.000200: sched_switch: prev_comm=media_driver prev_pid=4100 prev_prio=120 prev_state=R ==> next_comm=ksoftirqd/2 next_pid=20 next_prio=120",
    "     ksoftirqd/2-20     [002] d..2.  100.000260: sched_switch: prev_comm=ksoftirqd/2 prev_pid=20 prev_prio=120 prev_state=S ==> next_comm=media_driver next_pid=4100 next_prio=120",
]
open(sys.argv[2], "w").write("== gapspin\n== trace\n" + "\n".join(trace) + "\nUPTIME 1000.0 REALTIME 2000.0\n")
_, preempted = spikes.read_diag(sys.argv[2])
got = sorted((cpu, who, by, round((t1 - t0) * 1e6)) for t0, t1, cpu, who, by in preempted)
assert got == [(2, "media_driver", "ksoftirqd/2", 60), (3, "[sender, receiv", "kworker/3:1", 50)], got
PY

# spikes.py where a host's gap spinner did not run: a four-core Graviton has
# no CPU outside the layout for it (vm.sh diag-start: "# no spinner"), and an
# SMT=off mode pauses it while its CPU is offline ("# paused"/"# resumed").
# A spike nothing else explains then is unexplained with no host stall ruled
# out, said so in its cause, counted as unexplained, and noted per host; one
# outside a pause, or the other host's stall, as ever. Two spikes a round
# trip run: no spinner on a (b's spinner gap explains the first); and b
# paused around the first (the second as before)
python3 - "$cloud" "$tmp/blind" <<'PY' && pass "spikes.py: unexplained spikes where a host ran no gap spinner, or had it paused, say so" || fail "spikes.py: a host without its gap spinner"
import os, subprocess, sys
cloud, d = sys.argv[1], sys.argv[2]
start = 1792032877.0
def run(name, diag_a, diag_b, first=400):
    state = os.path.join(d, name, "isolated")
    os.makedirs(state)
    with open(os.path.join(state, "spikes-isolated-kernel-rtt-1.csv"), "w") as f:
        f.write(f"# start_realtime_ns={int(start * 1e9)}\nstream,due_ns,latency_ns\n0,1000000000,{first}000\n0,3000000000,400000\n")
    for host, gapspin in (("a", diag_a), ("b", diag_b)):
        with open(os.path.join(d, name, f"{host}-diag-isolated.log"), "w") as f:
            f.write("== gapspin\n" + gapspin + "== trace\n")
    return subprocess.run([sys.executable, os.path.join(cloud, "spikes.py"), state], capture_output=True, text=True, check=True).stdout
# a: none; b: a 300 us gap ending 1.0003 s in (the first spike's)
none = run("none", "# no spinner: no CPU outside the layout (no fifth core, no SMT twin)\n", f"{start + 1.0003:.6f} 300\n")
assert "| isolated-kernel-rtt | 2 | 0 | 1 | 0 | 0 | 1 | 0 |" in none, none
assert "400: unexplained (no spinner on a: a host stall not ruled out)" in none, none
assert "Host a ran no gap spinner" in none and "Host b" not in none, none
# b paused from 0.5 s to 1.5 s in: the first spike (450 us, the max)
# unexplained with b blind, the second plain
paused = run("paused", "", f"# paused {start + 0.5:.6f}\n# resumed {start + 1.5:.6f}\n", first=450)
assert "| isolated-kernel-rtt | 2 | 0 | 0 | 0 | 0 | 2 | 0 |" in paused, paused
assert "450: unexplained (no spinner on b: a host stall not ruled out)" in paused, paused
assert "Host b's gap spinner was paused 1 time" in paused and "Host a" not in paused, paused
# both spinners running: as ever, no such cause or note
plain = run("plain", "", "")
assert "no spinner" not in plain and "gap spinner" not in plain and "| isolated-kernel-rtt | 2 | 0 | 0 | 0 | 0 | 2 | 0 |" in plain, plain
PY

# The clock benchmarks were removed: aws/lab.sh refuses LAB_CLOCK_ONLY and
# LAB_DISCIPLINE_SECONDS before any cloud call, so a stale command line never
# becomes a full (billed) run in their place
for refused in "LAB_CLOCK_ONLY=1" "LAB_DISCIPLINE_SECONDS=1800" "LAB_ESTIMATE=1 LAB_CLOCK_ONLY=1"; do
    # shellcheck disable=SC2086
    if lab_aws "$tmp/clock-refused" $refused; then
        fail "aws/lab.sh ran with $refused"
    elif grep -qF "LAB_CLOCK_ONLY and LAB_DISCIPLINE_SECONDS were removed" "$tmp/clock-refused.out" && [[ ! -s $tmp/clock-refused ]]; then
        pass "aws/lab.sh refuses $refused before any cloud call (removed)"
    else
        fail "aws/lab.sh with $refused: $(tail -3 "$tmp/clock-refused.out"; cat "$tmp/clock-refused")"
    fi
done

# modes.sh's pre-flight (lab.sh, before any cloud call), against what vm.sh
# and the DPDK transport would do hours in: an upper-case @KEY that is
# neither a setting the lab knows nor an AERON_* or RUSTERON_* variable is
# refused (vm.sh would export it to a driver that never reads it, the mode
# measuring its baseline under another name: @CONDUCTOR=own did before it
# existed), as are a pair vm.sh refuses at the driver's start and a transport
# it does not know; a DPDK mode's RUSTERON_DPDK_* values as md-dpdk's
# config.rs validates them (rings 64 to 8192 and a power of two, mempool
# cache 512 at most, bursts 1 to 256, enough mbufs for both rings, four
# bursts and two caches, a 32-aligned Aeron MTU up to 8960, a timestamp flag
# of 0, 1, true or false) and as the NIC takes them (ENA: receive 8192,
# transmit 512; MANA: 256 each), NIC RX timestamps only where AWS gives them
# (precision-time); @CONDUCTOR=own (the conductor on the layout's fifth core,
# spinning) once, cross-host only, never beside the spare core's other users
# (@IRQ=spare, @NAPI_BUSY=spare) nor under SHARED (the conductor is the
# shared agent) or INVOKER (no conductor thread), and the spare core's users
# counted as such; each type's physical cores from the table (c7i, c8i:
# vCPUs / 2; Graviton: vCPUs; C7a: vCPUs; Azure's D, E and F v5 and v6
# sizes: vCPUs / 2), a type the table lacks said so, and a mode that takes
# the fifth core refused on a type without one; the families with a PTP
# hardware clock (precision-time: every family AWS lists for precision-time
# placement groups, general purpose, memory and storage as well as compute),
# and under precision-time a DPDK baseline with NIC RX timestamps both on
# and off
pre=$(
    set +e
    exec 2>&1
    # shellcheck source=../modes.sh
    . "$cloud/modes.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    mc() { rc mode_check "$1" "${2:-remote}" x86_64 "${3:-aws}" '' "${4:-cluster}"; }
    echo "keys:$(mc dpdk@CONDUCTUR=own)$(mc kernel@FOO=1)$(mc kernel@coalesce=off)$(mc kernel@JUMBO)$(mc "kernel@SUB_PARAMS=rcv-wnd=1m@X Y=1")$(mc kernel@COALESCE=off@IRQ=spare)$(mc dpdk@AERON_SENDER_IO_VECTOR_CAPACITY=2@AERON_NETWORK_PUBLICATION_MAX_MESSAGES_PER_SEND=2)$(mc dpdk@RUSTERON_DPDK_BURST_SIZE=16)$(mc kernel-busy@BUSY_POLL=50@RX_RING=4096@THP=always@MIN_FREE_PCT=2@CLOCKSOURCE=tsc@RELIABLE=false@CSTATE=1@BIN=bin-base@COALESCE=0/64@NAPI_DEFER=2/200000)"
    echo "transports:$(mc kernal)$(mc dpdk-turbo@JUMBO=1)$(mc dpdk-spin-mono)$(mc kernel-busy-mono@RX_RING=4096)$(mc dpdk-adaptive+idle)$(mc dpdk-epoll@JUMBO=1+idle)"
    mode_check dpdk@CONDUCTUR=own remote x86_64 aws '' cluster
    mode_check kernal remote x86_64 aws '' cluster
    echo "rx:$(mc dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=4096)$(mc dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=8192)$(mc dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=16384)$(mc dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=32)$(mc dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=3000)$(mc dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=many)$(mc dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=256 remote azure '')$(mc dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=512 remote azure '')$(mc kernel@RUSTERON_DPDK_RX_DESCRIPTORS=99999)"
    echo "tx:$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=256)$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=512)$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=512@RUSTERON_DPDK_ENA_DEVARGS=llq_policy=1)$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=512@RUSTERON_DPDK_ENA_DEVARGS=miss_txc_to=0+llq_policy=2)$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=1024@RUSTERON_DPDK_ENA_DEVARGS=llq_policy=1)$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=512@RUSTERON_DPDK_ENA_DEVARGS=llq_policy=3)$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=1024)$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=256 remote azure '')$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=512 remote azure '')$(mc dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=384)"
    echo "cache, burst:$(mc dpdk@RUSTERON_DPDK_MEMPOOL_CACHE=512)$(mc dpdk@RUSTERON_DPDK_MEMPOOL_CACHE=0)$(mc dpdk@RUSTERON_DPDK_MEMPOOL_CACHE=513)$(mc dpdk@RUSTERON_DPDK_BURST_SIZE=256)$(mc dpdk@RUSTERON_DPDK_BURST_SIZE=0)$(mc dpdk@RUSTERON_DPDK_BURST_SIZE=257)"
    echo "mbufs:$(mc dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=8192@RUSTERON_DPDK_MEMPOOL_CACHE=512)$(mc dpdk@RUSTERON_DPDK_MBUFS_PER_PORT=5248)$(mc dpdk@RUSTERON_DPDK_MBUFS_PER_PORT=5247)$(mc dpdk@RUSTERON_DPDK_MBUFS_PER_PORT=9344@RUSTERON_DPDK_RX_DESCRIPTORS=8192)$(mc dpdk@RUSTERON_DPDK_MBUFS_PER_PORT=9343@RUSTERON_DPDK_RX_DESCRIPTORS=8192)$(mc dpdk@RUSTERON_DPDK_MBUFS_PER_PORT=1408 remote azure '')$(mc dpdk@RUSTERON_DPDK_MBUFS_PER_PORT=1407 remote azure '')"
    echo "aeron mtu:$(mc dpdk@RUSTERON_DPDK_MAX_AERON_MTU=1500)$(mc dpdk@RUSTERON_DPDK_MAX_AERON_MTU=9024@JUMBO=1)$(mc dpdk@RUSTERON_DPDK_MAX_AERON_MTU=4096)"
    echo "timestamps:$(mc dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=1)$(mc dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=1 remote aws precision-time)$(mc dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=TRUE remote aws precision-time)$(mc dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=0)$(mc dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=False)$(mc dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=yes remote aws precision-time)$(mc dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=1 remote azure '')"
    mode_check dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=1024 remote x86_64 aws '' cluster
    mode_check dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=512 remote x86_64 azure '' ''
    mode_check dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=512 remote x86_64 aws '' cluster
    mode_check dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=1024@RUSTERON_DPDK_ENA_DEVARGS=llq_policy=1 remote x86_64 aws '' cluster
    mode_check dpdk@RUSTERON_DPDK_MBUFS_PER_PORT=5247 remote x86_64 aws '' cluster
    mode_check dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=1 remote x86_64 aws '' cluster
    echo "conductor:$(mc dpdk@CONDUCTOR=own)$(mc kernel@CONDUCTOR=own@AERON_THREADING_MODE=SHARED_NETWORK)$(mc dpdk@CONDUCTOR=own@AERON_THREADING_MODE=SHARED)$(mc dpdk@CONDUCTOR=own@AERON_THREADING_MODE=INVOKER)$(mc dpdk@CONDUCTOR=own local)$(mc kernel@CONDUCTOR=own@IRQ=spare)$(mc kernel-busy@NAPI_BUSY=spare@CONDUCTOR=own)$(mc dpdk@CONDUCTOR=fast)$(mc dpdk@CONDUCTOR=own@CONDUCTOR=own)$(mc kernel@CONDUCTOR=own@IRQ=receiver)$(mc dpdk@CONDUCTOR=own remote azure '')"
    mode_check kernel@CONDUCTOR=own@IRQ=spare remote x86_64 aws '' cluster
    mode_check dpdk@CONDUCTOR=own@AERON_THREADING_MODE=SHARED remote x86_64 aws '' cluster
    echo "spare:$(rc mode_spare dpdk@CONDUCTOR=own)$(rc mode_spare dpdk@CONDUCTOR=own+idle)$(rc mode_spare dpdk)"
    echo "cores: $(type_cores aws c7i.2xlarge) $(type_cores aws c7i.4xlarge) $(type_cores aws c9g.2xlarge) $(type_cores aws c8g.xlarge) $(type_cores aws c7i.metal-24xl) $(type_cores aws c7i-flex.2xlarge) $(type_cores aws c8i.4xlarge) $(type_cores aws c7a.2xlarge) $(type_cores aws c6in.4xlarge) $(type_cores aws c8gn.xlarge) $(type_cores azure Standard_D8s_v6) $(type_cores azure Standard_D16s_v6) $(type_cores azure Standard_E8ds_v5)$(rc type_cores aws z9.huge)$(rc type_cores aws c7g.metal)$(rc type_cores aws c7i)$(rc type_cores azure Standard_B4ms)"
    type_cores aws z9.huge
    echo "fifth core:$(rc spare_types_check kernel@IRQ=spare aws c7i.2xlarge c7i.4xlarge)$(rc spare_types_check kernel@IRQ=spare aws c7i.4xlarge c9g.2xlarge)$(rc spare_types_check dpdk@CONDUCTOR=own aws c9g.2xlarge c8g.xlarge)$(rc spare_types_check kernel@NAPI_BUSY=spare aws m9z.4xlarge c7i.4xlarge)$(rc spare_types_check dpdk aws z9.huge z9.huge)$(rc spare_types_check kernel@NAPI_BUSY=spare azure Standard_D8s_v6 Standard_D8s_v6)$(rc spare_types_check kernel@NAPI_BUSY=spare azure Standard_D16s_v6 Standard_D16s_v6)"
    spare_types_check kernel@IRQ=spare aws c7i.2xlarge c7i.4xlarge
    spare_types_check dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=4096@CONDUCTOR=own aws c9g.2xlarge m9z.4xlarge
    echo "phc:$(rc phc_type c7i.4xlarge)$(rc phc_type c7i-flex.large)$(rc phc_type c7a.2xlarge)$(rc phc_type c7gd.4xlarge)$(rc phc_type c8a.2xlarge)$(rc phc_type c8g.4xlarge)$(rc phc_type c8gd.xlarge)$(rc phc_type c8i.2xlarge)$(rc phc_type c8i-flex.xlarge)$(rc phc_type c9g.2xlarge)$(rc phc_type c9gd.4xlarge)$(rc phc_type c7g.4xlarge)$(rc phc_type c7gn.4xlarge)$(rc phc_type c8gn.xlarge)$(rc phc_type c6in.4xlarge)$(rc phc_type c9gn.2xlarge)"
    echo "phc, other families:$(rc phc_type m7i.2xlarge)$(rc phc_type m7gd.xlarge)$(rc phc_type m8i-flex.large)$(rc phc_type m9gd.2xlarge)$(rc phc_type r7g.2xlarge)$(rc phc_type r9gd.xlarge)$(rc phc_type x8g.2xlarge)$(rc phc_type x8i.4xlarge)$(rc phc_type i8g.xlarge)$(rc phc_type i8ge.2xlarge)$(rc phc_type m6i.4xlarge)$(rc phc_type m8gd.xlarge)$(rc phc_type r6g.2xlarge)$(rc phc_type i7ie.xlarge)"
    echo "timestamp pair:$(rc precision_modes_check kernel dpdk)$(rc precision_modes_check dpdk dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=0)$(rc precision_modes_check kernel kernel-busy)$(rc precision_modes_check dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=0)$(rc precision_modes_check dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=4096 kernel dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=false@RUSTERON_DPDK_RX_DESCRIPTORS=4096)$(rc precision_modes_check dpdk-spin@RUSTERON_DPDK_RX_HW_TIMESTAMP=1 dpdk-spin@RUSTERON_DPDK_RX_HW_TIMESTAMP=0)"
    precision_modes_check kernel dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=4096
) || true
want="keys: 2 2 2 2 2 0 0 0 0
transports: 2 2 0 0 0 0
mode dpdk@CONDUCTUR=own: CONDUCTUR is neither a setting the lab knows (THP BIN CSTATE BUSY_POLL COALESCE RX_RING MIN_FREE_PCT CLOCKSOURCE RELIABLE AGENTS SMT JUMBO PUB_PARAMS SUB_PARAMS IRQ NAPI_DEFER NAPI_BUSY CONDUCTOR HAIRPIN RTT_GAP_US KEEPALIVE_US FLUSH_KB DPDK) nor an AERON_* or RUSTERON_* variable: vm.sh would export it to a driver that never reads it, and the mode would measure its baseline under another name
mode kernal: transport kernal: kernel, kernel-busy, dpdk, dpdk-spin, dpdk-adaptive, dpdk-epoll, the raw floor's kernel-raw or dpdk-raw, optionally -mono, or the pure floor's dpdk-pure (vm.sh refuses any other at the driver's start)
rx: 0 0 2 2 2 2 0 2 0
tx: 0 2 0 0 2 2 2 0 2 2
cache, burst: 0 0 2 0 2 2
mbufs: 0 0 2 0 2 0 2
aeron mtu: 2 2 0
timestamps: 2 0 0 0 0 2 2
mode dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=1024: RUSTERON_DPDK_TX_DESCRIPTORS=1024: 64 to 256, a power of two (ENA's transmit ring holds 256 at most under large LLQ entries (the transport's llq_policy=3; llq_policy=1 takes 512): the transport refuses more at start)
mode dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=512: RUSTERON_DPDK_RX_DESCRIPTORS=512: 64 to 256, a power of two (MANA's receive ring holds 256 at most: the transport refuses more at start)
mode dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=512: RUSTERON_DPDK_TX_DESCRIPTORS=512: 64 to 256, a power of two (ENA's transmit ring holds 256 at most under large LLQ entries (the transport's llq_policy=3; llq_policy=1 takes 512): the transport refuses more at start)
mode dpdk@RUSTERON_DPDK_TX_DESCRIPTORS=1024@RUSTERON_DPDK_ENA_DEVARGS=llq_policy=1: RUSTERON_DPDK_TX_DESCRIPTORS=1024: 64 to 512, a power of two (ENA's transmit ring holds 512 at most under llq_policy=1: the transport refuses more at start)
mode dpdk@RUSTERON_DPDK_MBUFS_PER_PORT=5247: RUSTERON_DPDK_MBUFS_PER_PORT=5247 is below the 5248 the transport needs: a receive ring of 4096, a transmit ring of 256, four bursts of 32 and two mempool caches of up to 384 each
mode dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=1: RUSTERON_DPDK_RX_HW_TIMESTAMP=1: NIC RX timestamps only in AWS's precision-time placement (LAB_PLACEMENT=precision-time): elsewhere the NIC offers none, and the transport refuses to start
conductor: 0 0 2 2 2 2 2 2 2 0 0
mode kernel@CONDUCTOR=own@IRQ=spare: CONDUCTOR=own with IRQ=spare: both take the layout's fifth core
mode dpdk@CONDUCTOR=own@AERON_THREADING_MODE=SHARED: CONDUCTOR=own under SHARED: the conductor is the one shared agent, on the sender's core
spare: 0 0 1
cores: 4 8 8 4 48 4 8 8 8 4 4 8 4 1 1 1 1
z9.huge: no core count for family z9 in modes.sh's table (Intel's c5, c6i, c7i, c8i and kin two threads a core; Graviton's and AMD's C7a and later one): give a type it knows
fifth core: 2 0 2 2 0 2 0
mode kernel@IRQ=spare takes the layout's fifth core (IRQ=spare): c7i.2xlarge has 4 physical cores (c7i.4xlarge, c8g.2xlarge and up have five or more)
mode dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=4096@CONDUCTOR=own takes the layout's fifth core (CONDUCTOR=own): m9z.4xlarge: no core count for family m9z in modes.sh's table (Intel's c5, c6i, c7i, c8i and kin two threads a core; Graviton's and AMD's C7a and later one): give a type it knows
phc: 0 0 0 0 0 0 0 0 0 0 0 1 1 1 1 1
phc, other families: 0 0 0 0 0 0 0 0 0 0 1 1 1 1
timestamp pair: 2 0 0 2 0 0
precision-time asks every DPDK port for NIC RX timestamps, which changes the ENA's RX descriptors, so the DPDK modes need a baseline with them off too: add dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=4096@RUSTERON_DPDK_RX_HW_TIMESTAMP=0 beside dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=4096"
if [[ $pre != "$want" ]]; then
    fail "modes.sh pre-flight: keys, transports, DPDK values, CONDUCTOR=own, cores by type, PHC families"
    diff -u <(echo "$want") <(echo "$pre") >&2 || true
else
    pass "modes.sh pre-flight: unknown keys and transports, the DPDK transport's values and the NIC's rings, CONDUCTOR=own's rules, each type's cores for the fifth core, the PHC families and precision-time's timestamp pair"
fi

# @DPDK=<build> (modes.sh, before any cloud call): the media driver and
# clients linked against a DPDK 25.11 built from source for the mode (vm.sh
# build-dpdk): stock, or patched-refill (dpdk-ena-deferred-refill.patch on
# ENA's PMD); in DPDK modes only (the raw floor's dpdk-raw too), on AWS only
# (ENA), in any placement, once, never beside @BIN= (one build a mode); and
# the builds the modes ask for, each once, for bench/want_dpdk
dpdkb=$(
    set +e
    exec 2>&1
    # shellcheck source=../modes.sh
    . "$cloud/modes.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    mc() { rc mode_check "$1" remote x86_64 "${2:-aws}" '' "${3:-cluster}"; }
    echo "builds:$(mc dpdk@DPDK=patched-refill)$(mc dpdk@DPDK=stock)$(mc dpdk@DPDK=patched-refill aws precision-time)$(mc dpdk-spin@DPDK=patched-refill@RUSTERON_DPDK_RX_FREE_THRESH=32)$(mc dpdk-raw@DPDK=stock)$(mc dpdk@DPDK=patched-refill+idle)"
    echo "refused:$(mc kernel@DPDK=patched-refill)$(mc kernel-raw@DPDK=stock)$(mc dpdk@DPDK=refill)$(mc dpdk@DPDK=patched-refill azure '')$(mc dpdk@DPDK=stock@BIN=bin-base)$(mc dpdk@DPDK=stock@DPDK=patched-refill)$(mc dpdk-pure@DPDK=stock)"
    mode_check kernel@DPDK=patched-refill remote x86_64 aws '' cluster
    mode_check dpdk@DPDK=refill remote x86_64 aws '' cluster
    mode_check dpdk@DPDK=patched-refill remote x86_64 azure '' ''
    mode_check dpdk@BIN=bin-base@DPDK=stock remote x86_64 aws '' cluster
    mode_check dpdk@DPDK=stock@DPDK=patched-refill remote x86_64 aws '' cluster
    # patched-refill's floor, min(2 x rx_free_thresh, ring - 1): a threshold
    # of half the ring or more would let bursts that return frames drain the
    # ring; stock is not refused (its own refill at the threshold)
    echo "floor:$(mc dpdk@DPDK=patched-refill@RUSTERON_DPDK_RX_FREE_THRESH=2047)$(mc dpdk@DPDK=stock@RUSTERON_DPDK_RX_FREE_THRESH=2048)$(mc dpdk@DPDK=patched-refill@RUSTERON_DPDK_RX_DESCRIPTORS=1024@RUSTERON_DPDK_RX_FREE_THRESH=511)$(mc dpdk@DPDK=patched-refill@RUSTERON_DPDK_RX_DESCRIPTORS=1024)$(mc dpdk@DPDK=patched-refill@RUSTERON_DPDK_RX_FREE_THRESH=2048)$(mc dpdk@DPDK=patched-refill@RUSTERON_DPDK_RX_DESCRIPTORS=1024@RUSTERON_DPDK_RX_FREE_THRESH=512)"
    mode_check dpdk@DPDK=patched-refill@RUSTERON_DPDK_RX_DESCRIPTORS=1024@RUSTERON_DPDK_RX_FREE_THRESH=512 remote x86_64 aws '' cluster
    echo "want: [$(modes_want_dpdk kernel dpdk@DPDK=patched-refill dpdk@DPDK=stock+idle dpdk-spin@DPDK=patched-refill@RUSTERON_DPDK_RX_FREE_THRESH=32)] [$(modes_want_dpdk kernel dpdk)] [$(modes_want_dpdk)]"
) || true
want="builds: 0 0 0 0 0 0
refused: 2 2 2 2 2 2 2
mode kernel@DPDK=patched-refill: DPDK=patched-refill is for DPDK modes only (the media driver and clients linked against that DPDK build)
mode dpdk@DPDK=refill: DPDK=refill: stock (DPDK 25.11 from source) or patched-refill (with dpdk-ena-deferred-refill.patch on ENA's PMD)
mode dpdk@DPDK=patched-refill: DPDK=patched-refill is AWS only (a build of ENA's PMD)
mode dpdk@BIN=bin-base@DPDK=stock: DPDK=stock with BIN=: one build a mode (DPDK= is its own, in ~/bin-dpdk-stock)
mode dpdk@DPDK=stock@DPDK=patched-refill: one DPDK at most
floor: 0 0 0 0 2 2
mode dpdk@DPDK=patched-refill@RUSTERON_DPDK_RX_DESCRIPTORS=1024@RUSTERON_DPDK_RX_FREE_THRESH=512: DPDK=patched-refill with RUSTERON_DPDK_RX_FREE_THRESH=512: below half the receive ring (1024) only: the patch's floor, min(2 x 512, 1023), would let bursts that return frames drain the ring before any refill
want: [patched-refill stock] [] []"
if [[ $dpdkb != "$want" ]]; then
    fail "modes.sh: @DPDK= builds"
    diff -u <(echo "$want") <(echo "$dpdkb") >&2 || true
else
    pass "modes.sh: @DPDK=stock|patched-refill in AWS DPDK modes only, once, never beside @BIN=, patched-refill's threshold below half the receive ring; the builds the modes want, each once"
fi

# both lab.sh apply that pre-flight before any cloud call: aws/lab.sh refuses
# a key nobody reads, a value the DPDK transport or the NIC refuses, NIC RX
# timestamps outside precision-time, a mode taking the fifth core on a type
# without one (or a type the table lacks), precision-time on a type without
# a PTP hardware clock or without the DPDK timestamp pair, and a warm-up
# without a rate (or not a number) or without load sizes (it warms before
# each mode's first ladder: with none it would never run); and it takes the
# fifth core's settings on types with one, and precision-time with the pair
# on PHC types. azure/lab.sh refuses MANA's rings past 256, the fifth core on
# a four-core size and a warm-up without its rate or load sizes. Each
# case: its settings, | between them, then # and what the refusal says.
for refused in "LAB_MODES=dpdk@CONDUCTUR=own#neither a setting the lab knows" \
    "LAB_MODES=dpdk@RUSTERON_DPDK_MEMPOOL_CACHE=600#0 to 512" \
    "LAB_MODES=dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=1#precision-time placement" \
    "LAB_MODES=kernel@COALESCE=off@IRQ=spare#c7i.2xlarge has 4 physical cores" \
    "LAB_ALLOW_ARM=1|LAB_TYPE=c8g.xlarge|LAB_MODES=dpdk@CONDUCTOR=own#c8g.xlarge has 4 physical cores" \
    "LAB_TYPE=c7i.4xlarge|LAB_TYPE_B=c7i.2xlarge|LAB_MODES=kernel dpdk@CONDUCTOR=own#c7i.2xlarge has 4 physical cores" \
    "LAB_TYPE=m9z.4xlarge|LAB_MODES=kernel@NAPI_BUSY=spare#no core count for family m9z" \
    "LAB_PLACEMENT=precision-time|LAB_ALLOW_ARM=1|LAB_TYPE=c7g.4xlarge|LAB_MODES=kernel#no PTP hardware clock" \
    "LAB_PLACEMENT=precision-time|LAB_TYPE=c6in.4xlarge|LAB_MODES=kernel#no PTP hardware clock" \
    "LAB_PLACEMENT=precision-time|LAB_MODES=kernel dpdk#@RUSTERON_DPDK_RX_HW_TIMESTAMP=0" \
    "LAB_WARMUP_SECONDS=30|LAB_LOAD_SIZES=288#LAB_WARMUP_RATE" \
    "LAB_WARMUP_SECONDS=half|LAB_WARMUP_RATE=25000#LAB_WARMUP_SECONDS" \
    "LAB_WARMUP_SECONDS=30|LAB_WARMUP_RATE=fast#LAB_WARMUP_RATE" \
    "LAB_WARMUP_SECONDS=30|LAB_WARMUP_RATE=25000#without LAB_LOAD_SIZES"; do
    IFS='|' read -ra vars <<<"${refused%#*}"
    if lab_aws "$tmp/pre-refused" "${vars[@]}"; then
        fail "aws/lab.sh ran with ${vars[*]}"
    elif grep -qF -- "${refused##*#}" "$tmp/pre-refused.out" && [[ ! -s $tmp/pre-refused ]]; then
        pass "aws/lab.sh refuses ${vars[*]} before any cloud call (${refused##*#})"
    else
        fail "aws/lab.sh with ${vars[*]}: $(tail -3 "$tmp/pre-refused.out"; cat "$tmp/pre-refused")"
    fi
done
pre_accepted=0
for case in "LAB_TYPE=c7i.4xlarge|LAB_MODES=kernel@COALESCE=off@IRQ=spare kernel-busy@NAPI_BUSY=spare@IRQ=spare dpdk@CONDUCTOR=own dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=4096@RUSTERON_DPDK_MEMPOOL_CACHE=512" \
    "LAB_ALLOW_ARM=1|LAB_TYPE=c9g.2xlarge|LAB_MODES=dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=4096@CONDUCTOR=own kernel@COALESCE=0/64@IRQ=spare" \
    "LAB_PLACEMENT=precision-time|LAB_MODES=kernel dpdk dpdk@RUSTERON_DPDK_RX_HW_TIMESTAMP=0" \
    "LAB_PLACEMENT=precision-time|LAB_TYPE=m7i.2xlarge|LAB_MODES=kernel" \
    "LAB_WARMUP_SECONDS=30|LAB_WARMUP_RATE=25000|LAB_LOAD_SIZES=288|LAB_LOAD_LADDER=288:1000|LAB_NO_FLATOUT=1"; do
    IFS='|' read -ra vars <<<"$case"
    lab_aws "$tmp/pre-accepted" "${vars[@]}" || true
    if ! grep -q 'sts get-caller-identity' "$tmp/pre-accepted"; then
        fail "aws/lab.sh refused ${vars[*]}: $(tail -3 "$tmp/pre-accepted.out")"
    else
        pre_accepted=$((pre_accepted + 1))
    fi
done
((pre_accepted == 5)) && pass "aws/lab.sh takes the fifth core's settings on types with five cores or more, precision-time with the DPDK timestamp pair on a PHC type (a general-purpose one too), and a warm-up with its rate and load sizes"
for refused in "LAB_MODES=dpdk@RUSTERON_DPDK_RX_DESCRIPTORS=512#MANA's receive ring holds 256" \
    "LAB_MODES=kernel@NAPI_BUSY=spare#Standard_D8s_v6 has 4 physical cores" \
    "LAB_SIZE=Standard_D16s_v6|LAB_MODES=dpdk@CONDUCTOR=own@CONDUCTOR=own#one CONDUCTOR at most" \
    "LAB_WARMUP_SECONDS=10#LAB_WARMUP_RATE" \
    "LAB_WARMUP_SECONDS=10|LAB_WARMUP_RATE=25000#without LAB_LOAD_SIZES"; do
    IFS='|' read -ra vars <<<"${refused%#*}"
    if env PATH="$tmp/fakebin:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/commarepo" \
        "${vars[@]}" bash "$cloud/azure/lab.sh" >"$tmp/az.out" 2>&1; then
        fail "azure/lab.sh ran with ${vars[*]}"
    elif grep -qF -- "${refused##*#}" "$tmp/az.out" && [[ ! -s $tmp/cloud-calls ]]; then
        pass "azure/lab.sh refuses ${vars[*]} before any cloud call (${refused##*#})"
    else
        fail "azure/lab.sh with ${vars[*]}: $(tail -2 "$tmp/az.out"; cat "$tmp/cloud-calls" 2>/dev/null)"
    fi
    rm -f "$tmp/cloud-calls"
done

# @CONDUCTOR=own in vm.sh (host_setting, as `driver` applies it, and its
# undoing): the conductor pinned to the layout's fifth core (4) and spinning,
# the DPDK transport's housekeeping and local-I/O threads named onto the
# housekeeping CPU (they would follow a conductor that does not spin), the
# core kept for the check after the start and dropped by `stop`; refused
# without a fifth core, under SHARED and INVOKER, and beside @IRQ=spare or
# @NAPI_BUSY=spare. Then the check at the driver's start (host.sh
# conductor_check, through vm.sh's agents_placed): the conductor is the
# media driver's main thread (Aeron's manual main loop runs the first agent
# there) or a thread of its own named conductor; it must be allowed that core
# alone, else the start fails (fail closed), and the check retries while the
# thread pins itself.
cs=$tmp/vm-conductor
mkdir -p "$cs/home/bench"
fake_thread 700 700 media_driver 4
fake_thread 700 701 aeron-md-nra 0
fake_thread 710 710 media_driver 0
fake_thread 720 720 media_driver 0
fake_thread 720 721 conductor 4
fake_thread 730 730 media_driver 0-7
fake_thread 740 740 media_driver 4,6
conductor=$(
    set +e
    exec 2>&1
    LAB_PROC_ROOT=$rt/proc LAB_SYS_ROOT=$rt/sys
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    # shellcheck source=../irqs.sh
    . "$cloud/irqs.sh" || exit 1
    eval "$(for f in save_host pin_irqs ynl_ready host_setting restore_host agents_placed; do vm_fn "$f"; done)"
    run=$cs/home/bench CLOUD=aws HK_CPU=0 APP_CPU=1 SENDER_CPU=2 RECEIVER_CPU=3 APP2_CPU=4 threading=DEDICATED
    env_file=$cs/driver.env
    sudo() { [[ $1 == systemctl ]] && return 0; [[ $* == *"$tmp/"* ]] || { echo "sudo $*" >>"$tmp/sudo-outside"; return 1; }; "$@"; }
    log() { :; }
    sleep() { :; }
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    : >"$env_file"
    overrides=CONDUCTOR=own
    (host_setting CONDUCTOR own dpdk) >/dev/null 2>&1 || echo "CONDUCTOR=own refused"
    echo "env: $(tr '\n' ' ' <"$env_file")| conductor_cpu $(cat "$run/conductor_cpu")"
    restore_host >/dev/null 2>&1
    echo "undone: [$(cd "$run" && ls conductor_cpu restore/* 2>/dev/null | tr '\n' ' ')]"
    : >"$env_file"
    refused=
    for c in "SHARED|CONDUCTOR=own|4" "INVOKER|CONDUCTOR=own|4" "DEDICATED|CONDUCTOR=own@IRQ=spare|4" "DEDICATED|NAPI_BUSY=spare@CONDUCTOR=own|4" "DEDICATED|CONDUCTOR=own|" "DEDICATED|CONDUCTOR=fast|4"; do
        IFS='|' read -r threading overrides APP2_CPU <<<"$c"
        (host_setting CONDUCTOR "$(mode_get "x@$overrides" CONDUCTOR)" kernel) >/dev/null 2>&1
        refused+=" $?"
        restore_host >/dev/null 2>&1
    done
    echo "refused:$refused env [$(cat "$env_file")]"
    echo "check:$(rc conductor_check "$proc_threads/700" 4)$(rc conductor_check "$proc_threads/710" 4)$(rc conductor_check "$proc_threads/720" 4)$(rc conductor_check "$proc_threads/730" 4)$(rc conductor_check "$proc_threads/740" 4)$(rc conductor_check "$proc_threads/799" 4)"
    conductor_check "$proc_threads/710" 4
    conductor_check "$proc_threads/730" 4
    # agents_placed <threading> </proc/<pid>>: the start's checks, retried
    APP2_CPU=4 SENDER_CPU=2 threading=DEDICATED
    echo 4 >"$run/conductor_cpu"
    echo "placed:$(rc agents_placed DEDICATED "$proc_threads/700")$(rc agents_placed DEDICATED "$proc_threads/710")$(rc agents_placed SHARED_NETWORK "$proc_threads/720")"
    rm -f "$run/conductor_cpu"
    echo "placed without:$(rc agents_placed DEDICATED "$proc_threads/710")$(rc agents_placed SHARED_NETWORK "$proc_threads/400")$(rc agents_placed SHARED_NETWORK "$proc_threads/500")"
    echo "sudo outside the fakes: [$(cat "$tmp/sudo-outside" 2>/dev/null)]"
) || true
want="env: export AERON_CONDUCTOR_CPU_AFFINITY='4' export AERON_CONDUCTOR_IDLE_STRATEGY='spin' export RUSTERON_DPDK_HOUSEKEEPING_CPU='0' | conductor_cpu 4
undone: []
refused: 2 2 2 2 2 2 env []
check: 0 2 0 2 2 2
CONDUCTOR=own: the conductor (media_driver/710, the main thread) may run on 0, not on the fifth core 4 alone
CONDUCTOR=own: the conductor (media_driver/730, the main thread) may run on 0-7, not on the fifth core 4 alone
placed: 0 2 2
placed without: 0 0 2
sudo outside the fakes: []"
if [[ $conductor != "$want" ]]; then
    fail "vm.sh: @CONDUCTOR=own set and undone, refused where it cannot be, checked at the start"
    diff -u <(echo "$want") <(echo "$conductor") >&2 || true
else
    pass "vm.sh: @CONDUCTOR=own pins the conductor to the fifth core, spinning, housekeeping named apart; refused without a fifth core, under SHARED or INVOKER, beside IRQ or NAPI_BUSY on that core; the start fails unless the conductor may run there alone"
fi

# Aeron's stall telemetry. The driver's environment (vm.sh common_env) sets
# Aeron 1.53.3's duty-cycle thresholds by their C names and duration syntax
# (aeron_driver_context.c: AERON_DRIVER_{CONDUCTOR,SENDER,RECEIVER}_CYCLE_THRESHOLD,
# aeron_config_parse_duration_ns): the conductor's 1 ms, the sender's and
# receiver's 50 us. host.sh aeron_telemetry reads them back from every counter
# (counters '', "<value> <type> <label>"), matching the system counters' labels
# exactly as aeron_system_counters.c and the conductor's appended suffixes
# write them (": DEDICATED", ": threshold=1ms DEDICATED"): the agents' longest
# cycles, their cycles over the threshold and the threshold itself; the
# sender's flow-control limits (back-pressure), status messages sent and
# received, flow control under and over runs; never a near miss ("Status
# Messages rejected", a label with more after it) nor a counter of another
# type with the same label. vm.sh loss appends them (info- keys) to its line,
# from the running driver or each INVOKER client's embedded one (the ledger
# takes the longest cycle, not the sum). lib.sh loss_row keeps them off the
# loss row (the row as before) and writes them as an aeron row: the longest
# cycles and thresholds as found after the run (since the driver started: one
# a mode and rep), the rest as their growth over it (a counter the line before
# lacks grows from 0 under INVOKER's ledger, and is left out otherwise).
cat >"$tmp/counters-all" <<'EOF'
           3    0 Bytes sent
         120    0 Status Messages sent
          17    0 Status Messages received
           0    0 Flow control under runs
           2    0 Flow control over runs
          40    0 Sender flow control limits, i.e. back-pressure events
      812345    0 Conductor max cycle time doing its work in ns: DEDICATED
           3    0 Conductor work cycle exceeded threshold count: threshold=1ms DEDICATED
       41234    0 Sender max cycle time doing its work in ns: DEDICATED
           1    0 Sender work cycle exceeded threshold count: threshold=50us DEDICATED
       61234    0 Receiver max cycle time doing its work in ns: DEDICATED
           2    0 Receiver work cycle exceeded threshold count: threshold=50us DEDICATED
           9    0 Status Messages rejected
         999    1 Status Messages sent
          77    0 Status Messages sent twice
           5    0 NameResolver max time in ns
           4    0 NameResolver exceeded threshold count: threshold=5s
           1   82 rusteron-dpdk role=sender bdf=0000:00:06.0 port=0 q=0 dir=tx pkts
EOF
tele=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    echo "all: $(aeron_telemetry <"$tmp/counters-all")"
    echo "shared: $(printf '%s\n' '      700000    0 Conductor max cycle time doing its work in ns: SHARED' '           0    0 Conductor work cycle exceeded threshold count: threshold=2s SHARED' | aeron_telemetry)"
    echo "none: [$(printf '%s\n' '           1   82 rusteron-dpdk role=sender dir=tx pkts' | aeron_telemetry)]"
    eval "$(awk -v f="common_env() {" '$0 == f {on = 1} on {print} on && $0 == "}" {exit}' "$cloud/vm.sh")"
    AERON_DIR=/mnt/huge/aeron-bench HK_CPU=0 SENDER_CPU=2 RECEIVER_CPU=3
    echo "env: $(common_env DEDICATED | grep CYCLE_THRESHOLD | tr '\n' ' ' | sed 's/ $//')"
    if common_env DEDICATED | grep -q AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD; then
        echo "env hard-codes the conductor's threshold"
    else
        echo "env leaves the conductor's threshold to be derived"
    fi
    ledger=$tmp/ledger-tele
    rm -rf "$ledger"
    invoker_ledger_record "$ledger" 1-1 "naks-sent=1;info-sender-max-cycle-ns=900;info-sender-cycles-over=2;info-sender-threshold-ns=50000"
    invoker_ledger_record "$ledger" 2-2 "naks-sent=2;info-sender-max-cycle-ns=700;info-sender-cycles-over=3;info-sender-threshold-ns=50000"
    echo "ledger: $(invoker_ledger_total "$ledger")"
    out=$tmp/tele-lib known=$tmp/tele-lib/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
    mkdir -p "$out"
    # shellcheck source=../lib.sh
    . "$cloud/lib.sh"
    results=$out/results.csv
    before="naks-sent=1;short-sends=0;kernel-udp-InErrors=0;nic-kernel-drops=0;routes-missing=0;info-conductor-max-cycle-ns=500000;info-conductor-cycles-over=1;info-conductor-threshold-ns=1000000;info-sms-sent=100;info-fc-over-runs=2"
    after="naks-sent=4;short-sends=0;kernel-udp-InErrors=0;nic-kernel-drops=0;routes-missing=0;info-conductor-max-cycle-ns=812345;info-conductor-cycles-over=3;info-conductor-threshold-ns=1000000;info-sms-sent=120;info-fc-over-runs=2;info-sender-fc-limits=40"
    loss_row 1 isolated:dpdk:288:r100000 a "$before" "$after"
    loss_row 1 isolated:dpdk:rtt b "naks-sent=0;routes-missing=0" "naks-sent=0;routes-missing=0"
    loss_row 2 isolated:dpdk@AERON_THREADING_MODE=INVOKER b "invoker-ledger=0" "invoker-ledger=2;naks-sent=3;info-sender-max-cycle-ns=900;info-sender-cycles-over=5;info-sender-threshold-ns=50000"
    loss_row 3 isolated:dpdk a "" "$after"
    cat "$results"
) || true
want="all: info-conductor-max-cycle-ns=812345;info-conductor-cycles-over=3;info-conductor-threshold-ns=1000000;info-sender-max-cycle-ns=41234;info-sender-cycles-over=1;info-sender-threshold-ns=50000;info-receiver-max-cycle-ns=61234;info-receiver-cycles-over=2;info-receiver-threshold-ns=50000;info-sender-fc-limits=40;info-sms-sent=120;info-sms-received=17;info-fc-under-runs=0;info-fc-over-runs=2
shared: info-conductor-max-cycle-ns=700000;info-conductor-cycles-over=0;info-conductor-threshold-ns=2000000000
none: []
env: export AERON_DRIVER_SENDER_CYCLE_THRESHOLD='50us' export AERON_DRIVER_RECEIVER_CYCLE_THRESHOLD='50us'
env leaves the conductor's threshold to be derived
ledger: invoker-ledger=2;naks-sent=3;info-sender-max-cycle-ns=900;info-sender-cycles-over=5;info-sender-threshold-ns=50000
1,loss,isolated:dpdk:288:r100000,a,naks-sent=3;short-sends=0;kernel-udp-InErrors=0;nic-kernel-drops=0;routes-missing=0
1,aeron,isolated:dpdk:288:r100000,a,info-conductor-max-cycle-ns=812345;info-conductor-cycles-over=2;info-conductor-threshold-ns=1000000;info-sms-sent=20;info-fc-over-runs=0
1,loss,isolated:dpdk:rtt,b,naks-sent=0;routes-missing=0
2,loss,isolated:dpdk@AERON_THREADING_MODE=INVOKER,b,naks-sent=3
2,aeron,isolated:dpdk@AERON_THREADING_MODE=INVOKER,b,info-sender-max-cycle-ns=900;info-sender-cycles-over=5;info-sender-threshold-ns=50000
3,loss,isolated:dpdk,a,invalid=1"
if [[ $tele != "$want" ]]; then
    fail "Aeron's stall telemetry: thresholds set, counters read by their exact labels, kept off the loss rows"
    diff -u <(echo "$want") <(echo "$tele") >&2 || true
else
    pass "Aeron's stall telemetry: the duty-cycle thresholds by Aeron's C names, the agents' longest cycles, cycles over threshold, back-pressure, status messages and flow control by their exact labels, an aeron row beside the loss row (which is as before)"
fi
# The conductor's duty-cycle threshold strictly above its idle strategy's
# longest park. Aeron's tracker fires on cycle_time_ns > threshold_ns
# (aeron_duty_cycle_tracker.h) and backoff parks up to 1 ms
# (aeron_agent.h: AERON_IDLE_STRATEGY_BACKOFF_MAX_PARK_PERIOD_NS), so a 1 ms
# threshold is met by every full park and the counter's max is a nanosleep
# oversleep, not work: the round-11 overrun count, near enough load
# independent. vm.sh derives the threshold from the park (duration_ns,
# conductor_max_park_ns, conductor_cycle_threshold) and applies it after the
# mode's @KEY=value overrides and before the driver or INVOKER's clients
# start (conductor_cycle_threshold_apply), refusing an explicit threshold that
# does not clear the park (Aeron's comparison is `>`, so equal is refused
# too), and an idle strategy, init args or threshold it cannot read.
thr=$(
    set +e
    exec 2>&1
    eval "$(for f in duration_ns format_duration_ns conductor_max_park_ns conductor_cycle_threshold conductor_cycle_threshold_apply; do vm_fn "$f"; done)"
    for f in duration_ns format_duration_ns conductor_max_park_ns conductor_cycle_threshold conductor_cycle_threshold_apply; do
        if declare -F "$f" >/dev/null; then echo "defined $f"; else echo "missing $f"; fi
    done
    echo "duration: $(duration_ns 5), $(duration_ns 20us), $(duration_ns 1ms), $(duration_ns 2s), $(duration_ns 500ns), $(duration_ns 1MS), $(duration_ns 0)"
    for v in '' 1x -1 1m 1.5ms ms 0x10 12s3 9223372036854775807ms; do
        if duration_ns "$v" >/dev/null 2>&1; then echo "duration accepted [$v]"; else echo "duration refused [$v]"; fi
    done
    echo "park: $(conductor_max_park_ns backoff), $(conductor_max_park_ns backoff ''), $(conductor_max_park_ns backoff 100-10-1us-20us), $(conductor_max_park_ns backoff 10-20-1ms-1ms), $(conductor_max_park_ns spin), $(conductor_max_park_ns noop)"
    for s in sleeping yield "backoff 1-2-3" "backoff 100-10-1us" "backoff a-b-c-d" "backoff 100-10-1us-"; do
        if conductor_max_park_ns $s >/dev/null 2>&1; then echo "park accepted [$s]"; else echo "park refused [$s]"; fi
    done
    echo "threshold: $(conductor_cycle_threshold backoff), $(conductor_cycle_threshold backoff ''), $(conductor_cycle_threshold backoff 100-10-1us-20us), $(conductor_cycle_threshold backoff 10-20-1ms-1ms), $(conductor_cycle_threshold spin), $(conductor_cycle_threshold noop)"
    threshold_case() {
        local label=$1 f=$tmp/thr.env err=$tmp/thr.err before after
        shift
        if (($#)); then printf '%s\n' "$@" >"$f"; else : >"$f"; fi
        before=$(wc -l <"$f" | tr -d ' ')
        if conductor_cycle_threshold_apply "$f" 2>"$err"; then
            after=$(wc -l <"$f" | tr -d ' ')
            echo "apply $label: $([[ $after -gt $before ]] && echo appended || echo kept) $(sed -n "s/^export AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='\(.*\)'$/\1/p" "$f" | tail -1)"
        else
            echo "apply $label: refused $(cat "$err")"
        fi
    }
    threshold_case default
    threshold_case park20us "export AERON_CONDUCTOR_IDLE_STRATEGY='backoff'" "export AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS='100-10-1us-20us'"
    threshold_case spin "export AERON_CONDUCTOR_IDLE_STRATEGY='spin'"
    threshold_case keep "export AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='10ms'"
    threshold_case equal "export AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1ms'"
    threshold_case clear-park "export AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1ms'" "export AERON_CONDUCTOR_IDLE_STRATEGY='backoff'" "export AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS='100-10-1us-20us'"
    threshold_case tight "export AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1us'" "export AERON_CONDUCTOR_IDLE_STRATEGY='backoff'" "export AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS='100-10-1us-20us'"
    threshold_case bad-init "export AERON_CONDUCTOR_IDLE_STRATEGY='backoff'" "export AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS='1-2-3'"
    threshold_case bad-threshold "export AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1x'"
    threshold_case later-wins "export AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1ms'" "export AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='10ms'"
    threshold_case unknown-strategy "export AERON_CONDUCTOR_IDLE_STRATEGY='sleeping'"
) || true
want="defined duration_ns
defined format_duration_ns
defined conductor_max_park_ns
defined conductor_cycle_threshold
defined conductor_cycle_threshold_apply
duration: 5, 20000, 1000000, 2000000000, 500, 1000000, 0
duration refused []
duration refused [1x]
duration refused [-1]
duration refused [1m]
duration refused [1.5ms]
duration refused [ms]
duration refused [0x10]
duration refused [12s3]
duration refused [9223372036854775807ms]
park: 1000000, 1000000, 20000, 1000000, 0, 0
park refused [sleeping]
park refused [yield]
park refused [backoff 1-2-3]
park refused [backoff 100-10-1us]
park refused [backoff a-b-c-d]
park refused [backoff 100-10-1us-]
threshold: 2ms, 2ms, 1ms, 2ms, 1ms, 1ms
apply default: appended 2ms
apply park20us: appended 1ms
apply spin: appended 1ms
apply keep: kept 10ms
apply equal: refused conductor: AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1ms' (1000000 ns) is not above backoff's longest park (1000000 ns): every full park would count as a stall. Use '2ms' or leave it unset.
apply clear-park: kept 1ms
apply tight: refused conductor: AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1us' (1000 ns) is not above backoff's longest park (20000 ns): every full park would count as a stall. Use '1ms' or leave it unset.
apply bad-init: refused conductor: AERON_CONDUCTOR_IDLE_STRATEGY='backoff' AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS='1-2-3': the park is not readable (backoff's init args are maxSpins-maxYields-minPark-maxPark, e.g. 100-10-1us-20us)
apply bad-threshold: refused conductor: AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1x': not an Aeron duration (bare ns, or the suffixes s, ms, us, ns)
apply later-wins: kept 10ms
apply unknown-strategy: refused conductor: AERON_CONDUCTOR_IDLE_STRATEGY='sleeping': not one whose longest park this lab knows (backoff, spin, noop)"
if [[ $thr != "$want" ]]; then
    fail "the conductor's duty-cycle threshold above its idle strategy's longest park, derived after the overrides and refused where it does not clear it"
    diff -u <(echo "$want") <(echo "$thr") >&2 || true
else
    pass "the conductor's duty-cycle threshold derived from the idle strategy's longest park (twice it, never below 1 ms), applied after the mode's overrides, and an explicit threshold that does not clear the park refused before the driver starts, naming both values and the one to use"
fi
# vm.sh loss in a sandbox (a media driver running; fake counters print a loss
# line for `counters loss` and every counter for `counters ''`): the line
# carries the telemetry after its loss counters; with no telemetry to read,
# none, and the loss line as before
sbl=$tmp/vm-loss
mkdir -p "$sbl/bin" "$sbl/home/ergon/target/release" "$sbl/home/bench"
cp "$sb/bin/"* "$sbl/bin/"
cat >"$sbl/bin/sudo" <<'SH'
#!/usr/bin/env bash
[[ $1 == env ]] && exec "$@"
exit 1
SH
printf '#!/bin/sh\nshift\nexec "$@"\n' >"$sbl/bin/timeout"
printf '#!/bin/sh\n[ "$*" = "-x media_driver" ]\n' >"$sbl/bin/pgrep"
# shellcheck disable=SC2016
printf '#!/bin/sh\nif [ "$1" = loss ]; then echo "naks-sent=1;short-sends=0"; elif [ -z "$1" ] && [ -s "$FAKE_ALL" ]; then cat "$FAKE_ALL"; fi\n' \
    >"$sbl/home/ergon/target/release/counters"
chmod +x "$sbl/bin/"* "$sbl/home/ergon/target/release/counters"
echo aws >"$sbl/home/bench/cloud"
mkdir -p "$sbl/proc/net"
printf 'Udp: InDatagrams NoPorts InErrors OutDatagrams RcvbufErrors SndbufErrors\nUdp: 100 0 0 100 0 0\n' >"$sbl/proc/net/snmp"
loss_sb() { # <counters '' output file>: vm.sh loss's line
    env -i PATH="$sbl/bin:$PATH" HOME="$sbl/home" FAKE_LOG="$sbl/log" FAKE_ARCH=x86_64 FAKE_ALL="$1" \
        LAB_SYS_ROOT="$sys/aws-x86" LAB_PROC_ROOT="$sbl/proc" bash "$cloud/vm.sh" loss 2>/dev/null || true
}
with=$(loss_sb "$tmp/counters-all")
without=$(loss_sb /dev/null)
if [[ $with != "naks-sent=1;short-sends=0;"*";info-conductor-max-cycle-ns=812345;"*";info-fc-over-runs=2" ]] ||
    [[ $without != "naks-sent=1;short-sends=0;"* || $without == *info-* || ${with%%;info-*} != "$without" ]]; then
    fail "vm.sh loss and the telemetry: [$with] [$without]"
else
    pass "vm.sh loss: Aeron's telemetry after the loss counters, the loss line otherwise as before"
fi

# analyse.py reports the aeron rows as information, never loss: per mode,
# run and host, the agents' longest cycles (the highest over reps, us) and
# their cycles over threshold, back-pressure events, status messages sent and
# received, flow control under and over runs (summed over reps; - where a
# host's rows lack one), the thresholds from the labels beneath; a loss row
# carrying info- keys (none does: loss_row keeps them apart) loses nothing;
# and results without aeron rows print no such table
python3 - "$cloud" "$tmp/aeron.csv" <<'PY' && pass "analyse.py: Aeron's agents and flow control by run and host, never loss" || fail "analyse.py: Aeron's agents and flow control"
import contextlib, io, re, sys
sys.path.insert(0, sys.argv[1])
import analyse
def row(rep, run, host, c, co, s, so, r, ro, fc, sent, rcvd, under, over):
    kv = (f"info-conductor-max-cycle-ns={c};info-conductor-cycles-over={co};info-conductor-threshold-ns=1000000;"
          f"info-sender-max-cycle-ns={s};info-sender-cycles-over={so};info-sender-threshold-ns=50000;"
          f"info-receiver-max-cycle-ns={r};info-receiver-cycles-over={ro};info-receiver-threshold-ns=50000;"
          f"info-sender-fc-limits={fc};info-sms-sent={sent};info-sms-received={rcvd};info-fc-under-runs={under};info-fc-over-runs={over}")
    return f"{rep},aeron,isolated:dpdk{run},{host},{kv}\n"
rows = [row(1, ":rtt", "a", 400000, 0, 30000, 0, 20000, 1, 0, 10, 12, 0, 0),
        row(1, ":rtt", "b", 500000, 1, 25000, 0, 60000, 2, 0, 11, 10, 0, 0),
        row(2, ":rtt", "a", 450000, 0, 35000, 1, 15000, 0, 0, 9, 11, 0, 1),
        row(1, ":288:r100000", "a", 450000, 0, 52000, 3, 20000, 0, 7, 1000, 1200, 2, 0),
        "1,aeron,isolated:dpdk:288:r100000,b,info-sender-max-cycle-ns=25000;info-sms-sent=1100\n",
        "1,load,isolated:dpdk,udp-r100000,288,100000,1,28000,30000,33000,38000,42000,60000,28500,1000000,1100000,1100000,0,100000,9000\n",
        "1,loss,isolated:dpdk:288:r100000,a,naks-sent=0;info-sender-cycles-over=3\n",
        "1,loss,isolated:dpdk:288:r100000,b,naks-sent=0\n"]
open(sys.argv[2], "w").writelines(rows)
r = analyse.read_results(sys.argv[2])
out = io.StringIO()
with contextlib.redirect_stdout(out):
    analyse.aeron_table(r)
want = """
## Aeron's agents and flow control by run (information, never loss)

From Aeron's own counters on each host, read with the loss counters around every run (vm.sh loss). An agent's longest duty cycle counts from its driver's start (one driver a mode and rep), so a run's is the longest up to its end: the highest over reps here. Its cycles over the threshold vm.sh sets (AERON_DRIVER_{CONDUCTOR,SENDER,RECEIVER}_CYCLE_THRESHOLD), the sender's flow-control limits (back-pressure events), the status messages sent and received and the flow control under and over runs are each run's growth, summed over reps. - where a host's rows lack one. None of them is loss.

| state, transport | run | host | reps | conductor max µs | sender max µs | receiver max µs | conductor over | sender over | receiver over | back-pressure events | status messages sent | status messages received | flow control under runs | flow control over runs |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| isolated, DPDK | idle round trip | a | 2 | 450.0 | 35.0 | 20.0 | 0 | 1 | 1 | 0 | 19 | 23 | 0 | 1 |
| isolated, DPDK | idle round trip | b | 1 | 500.0 | 25.0 | 60.0 | 1 | 0 | 2 | 0 | 11 | 10 | 0 | 0 |
| isolated, DPDK | 288 B at 100,000/s | a | 1 | 450.0 | 52.0 | 20.0 | 0 | 3 | 0 | 7 | 1,000 | 1,200 | 2 | 0 |
| isolated, DPDK | 288 B at 100,000/s | b | 1 | - | 25.0 | - | - | - | - | - | 1,100 | - | - | - |

Thresholds, from the counters' labels: conductor 1 ms, sender 50 µs, receiver 50 µs.
"""
norm = lambda s: [re.sub(r"-{3,}", "---", " ".join(l.split())) for l in s.strip().splitlines()]
got = out.getvalue()
assert norm(got) == norm(want), got
assert not analyse.rep_lost(r, "isolated:dpdk", 288, "r100000", 1), "info- keys counted as loss"
none = io.StringIO()
with contextlib.redirect_stdout(none):
    analyse.aeron_table(analyse.read_results(sys.argv[1] + "/test/fixture/results.csv"))
assert none.getvalue() == "", none.getvalue()
PY

# lib.sh dry, the load sweep's warm-up and per-point histograms (every VM
# command faked; a load client prints its row as load does, and the
# histogram it would leave, value_ns,count, is its own rate's, so a fetched
# file names the point it came from). With LAB_WARMUP_SECONDS=3 and
# LAB_WARMUP_RATE=25000: once a mode and rep, before its first ladder (the
# first size the mode runs), a warm-up load at 25,000/s for 3 s at that size,
# labelled <label>:warmup, on fresh endpoints with its pong on the far host,
# recorded nowhere (no results row, no loss row, no spikes, no histogram);
# none for a +idle mode (no sweep); every load point's histogram fetched into
# histogram-<state>-<mode>-<size>-<rep>-<point>.csv, the burst's too, each
# its own point's; and both settings reach VM a's environment. Unset, no
# warm-up and every other call as before.
wu=$tmp/dry-warmup
mkdir -p "$wu"
cat >"$tmp/dry-warmup.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
label() { local l=${1#*client }; echo "${l%% *}"; }
on() {
    local host=$1 cmd rate
    shift
    cmd=$*
    printf '%s %s\n' "$host" "$cmd" >>"$out/calls"
    case $cmd in
    *"vm.sh nics") echo "receiver ens6 ens6v 00:00:00:00:00:00 $([[ $host == "$A" ]] && echo 10.30.1.5 || echo 10.30.1.7) 24 0000:00:07.0" ;;
    *"vm.sh layout") [[ $host == "$A" ]] && echo "0 2 1 3 4" || echo "0 6 1 3 7" ;;
    *"vm.sh loss") echo "naks-sent=0;short-sends=0" ;;
    *" rtt idle "*) echo "idle subscription" ;;
    *" load xping "*)
        read -r _ _ rate _ <<<"${cmd#* load xping }"
        echo "$rate" >"$out/last-rate"
        echo "load,$(label "$cmd"),udp,288,$rate,1,1,2,3,4,5,6,1,100,100,100,0,$rate,10"
        ;;
    "cat bench/rtt-histogram.csv") printf 'value_ns,count\n%s,1\n' "$(cat "$out/last-rate")" ;;
    *"/env") cat >"$out/env-written" ;;
    *"/rc 2>/dev/null") echo 0 ;;
    esac
    return 0
}
reps=1 local_modes=kernel load_sizes='288 1344' load_fractions=50 load_seconds=1 load_ladder='288:1000,2000 1344:500' no_flatout=1
mix= mix_seconds=1 tput_seconds=5 rtt_messages=10 rtt_warmup=1 rtt_align=
[[ $3 == unset ]] || { warmup_seconds=3 warmup_rate=25000; }
modes=(dpdk dpdk+idle)
bench_state isolated
a_private=10.30.0.10 b_private=10.30.0.11
remote_state isolated
SH
bash "$tmp/dry-warmup.sh" "$cloud" "$wu" set </dev/null 2>"$wu/stderr" || fail "lib.sh dry warm-up run exited $?"
wun=$tmp/dry-warmup-unset
mkdir -p "$wun"
bash "$tmp/dry-warmup.sh" "$cloud" "$wun" unset </dev/null 2>"$wun/stderr" || fail "lib.sh dry run without a warm-up exited $?"
# each load client: "<host> <label> <rate> <seconds> <size>", in order
loads() { awk '{for (i = 1; i < NF; i++) if ($i == "load" && $(i + 1) == "xping") print $1, $(i - 1), $(i + 4), $(i + 5), $(i + 6)}' "$1"; }
warm=$(loads "$wu/calls" | grep ':warmup ' | tr '\n' ';') || true
first=$(loads "$wu/calls" | awk '{print $2}' | awk '!seen[$0]++' | tr '\n' ';') || true
# the warm-up's pong, on fresh endpoints (before the first point's, 22202/22203)
pongs=$(grep -c 'rtt xpong 10.30.1.7:22200 10.30.1.5:22201' "$wu/calls" || true)
hists=$(cd "$wu" && for f in histogram-isolated-*-288-1-* histogram-isolated-*-1344-1-*; do printf '%s=%s;' "$f" "$(tail -1 "$f" 2>/dev/null)"; done) || true
want_hists="histogram-isolated-dpdk-288-1-burst.csv=1000,1;histogram-isolated-dpdk-288-1-r1000.csv=1000,1;histogram-isolated-dpdk-288-1-r2000.csv=2000,1;histogram-isolated-local-kernel-288-1-burst.csv=1000,1;histogram-isolated-local-kernel-288-1-r1000.csv=1000,1;histogram-isolated-local-kernel-288-1-r2000.csv=2000,1;histogram-isolated-dpdk-1344-1-burst.csv=500,1;histogram-isolated-dpdk-1344-1-r500.csv=500,1;histogram-isolated-local-kernel-1344-1-burst.csv=500,1;histogram-isolated-local-kernel-1344-1-r500.csv=500,1;"
if grep -E 'unbound variable|command not found|syntax error' "$wu/stderr" "$wun/stderr"; then
    fail "lib.sh dry warm-up: shell errors"
elif [[ $warm != "192.0.2.1 isolated:dpdk:warmup 25000 3 288;192.0.2.1 isolated:local-kernel:warmup 25000 3 288;" ]]; then
    fail "lib.sh dry warm-up: the warm-ups: $warm"
elif [[ $first != "isolated:dpdk:warmup;isolated:dpdk;isolated:local-kernel:warmup;isolated:local-kernel;" ]] || ((pongs != 1)); then
    fail "lib.sh dry warm-up: not first in each sweep on its own endpoints: $first ($pongs pongs)"
elif grep -q 'warmup' "$wu/results.csv" || compgen -G "$wu/*warmup*.csv" >/dev/null; then
    fail "lib.sh dry warm-up: recorded: $(grep warmup "$wu/results.csv" | head -2) $(ls "$wu" | grep warmup | grep csv)"
elif [[ $hists != "$want_hists" ]]; then
    fail "lib.sh dry: per-point histograms: $hists"
elif ! grep -qx "warmup_seconds=3" "$wu/env-written" || ! grep -qx "warmup_rate=25000" "$wu/env-written"; then
    fail "lib.sh dry warm-up: not in VM a's environment: $(grep warmup "$wu/env-written")"
elif grep -q 'warmup' "$wun/calls" || ! diff <(loads "$wu/calls" | grep -v ':warmup ') <(loads "$wun/calls") >"$tmp/diff"; then
    fail "lib.sh dry without a warm-up: not as before"
    cat "$tmp/diff" >&2
else
    pass "lib.sh dry: a warm-up once a mode and rep before its first ladder, at that size, on its own endpoints, recorded nowhere; each load point's histogram fetched as its own; nothing of it without LAB_WARMUP_SECONDS"
fi

# analyse.py merges each load point's histograms (histogram-<state>-<mode>-
# <size>-<rep>-<point>.csv, value_ns,count as the load client writes them)
# over its reps and reports the pooled percentiles as HdrHistogram computes
# them (the first value whose running count reaches ceil(q x samples)): p50
# to p99.99 and max, beside the medians of reps; a rep whose file is missing
# or empty is counted out and said so; results without such files print no
# such table. Two reps of 500 samples at 288 B and 100,000/s merge to
# 10 us x 500, 20 x 150, 25 x 250, 30 x 45, 40 x 44, 60 x 4, 70 x 5, 90 x 1,
# 120 x 1: p50 10, p90 25, p99 60 (40 reaches 989 of 990), p99.9 90,
# p99.99 and max 120. Seven samples, 31 to 37 us: p90 is the 7th (ceil 6.3),
# 37, never the 6th.
mh=$tmp/merged-hist
mkdir -p "$mh/isolated"
python3 - "$cloud" "$mh" <<'PY' && pass "analyse.py: each load point's histograms merged over reps, HdrHistogram's percentiles, missing reps said" || fail "analyse.py: merged load-point histograms"
import contextlib, io, os, re, sys
sys.path.insert(0, sys.argv[1])
import analyse
d = sys.argv[2]
load = "{rep},load,isolated:{m},udp-{p},{size},{rate},1,28000,30000,33000,38000,42000,60000,28500,500,600,600,0,{rate},9000\n"
rows = [load.format(rep=rep, m=m, p=p, size=288, rate=rate) for rep in (1, 2) for m in ("dpdk", "kernel@IRQ=spare")
        for p, rate in (("r100000", 100000), ("burst", 50000))]
open(os.path.join(d, "results.csv"), "w").writelines(rows)
def hist(name, pairs):
    open(os.path.join(d, "isolated", name), "w").write("value_ns,count\n" + "".join(f"{v},{c}\n" for v, c in pairs))
hist("histogram-isolated-dpdk-288-1-r100000.csv", [(10000, 300), (20000, 150), (30000, 45), (60000, 4), (90000, 1)])
hist("histogram-isolated-dpdk-288-2-r100000.csv", [(10000, 200), (25000, 250), (40000, 44), (70000, 5), (120000, 1)])
hist("histogram-isolated-dpdk-288-1-burst.csv", [(50000, 3)])
open(os.path.join(d, "isolated", "histogram-isolated-dpdk-288-2-burst.csv"), "w").write("")
hist("histogram-isolated-kernel@IRQ=spare-288-2-r100000.csv", [(v, 1) for v in range(31000, 38000, 1000)])
r = analyse.read_results(os.path.join(d, "results.csv"))
out = io.StringIO()
with contextlib.redirect_stdout(out):
    analyse.merged_load(r, os.path.join(d, "results.csv"))
want = """
## Round trip under open-loop load, every rep's histogram merged (µs)

Each load point's reps pooled into one histogram (the load client's after its warm-up second, lib.sh fetching it after every point) and its percentiles as HdrHistogram takes them: unlike the medians of reps above, a tail here counts every rep's samples. A rep whose histogram is missing or empty is left out, said so.

| state, transport | size | load | reps merged | samples | p50 | p90 | p99 | p99.9 | p99.99 | max |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| isolated, DPDK | 288 B | 100,000/s | 2 | 1,000 | 10.00 | 25.00 | 60.00 | 90.00 | 120.00 | 120.00 |
| isolated, DPDK | 288 B | burst 1000 | 1 of 2 | 3 | 50.00 | 50.00 | 50.00 | 50.00 | 50.00 | 50.00 |
| isolated, kernel@IRQ=spare | 288 B | 100,000/s | 1 of 2 | 7 | 34.00 | 37.00 | 37.00 | 37.00 | 37.00 | 37.00 |
"""
norm = lambda s: [re.sub(r"-{3,}", "---", " ".join(l.split())) for l in s.strip().splitlines()]
assert norm(out.getvalue()) == norm(want), out.getvalue()
none = io.StringIO()
with contextlib.redirect_stdout(none):
    path = os.path.join(sys.argv[1], "test", "fixture", "results.csv")
    analyse.merged_load(analyse.read_results(path), path)
assert none.getvalue() == "", none.getvalue()
PY
# vm.sh client: a stale histogram never passes for this run's. Before the
# round trip's ping (rtt xping) and before a load client that offers (load
# xping, xmix) the last run's rtt-histogram.csv goes, as the load spikes'
# file does: a client that writes none leaves none to fetch; the pong and the
# echo, which write none, leave it alone
stale=$(
    for args in "rtt xping 10.30.1.7:20123 10.30.1.5:20124 10 1 2" "load xping 10.30.1.7:22200 10.30.1.5:22201 1000 5 288 2" \
        "load xmix 10.30.1.7:22800 10.30.1.5:22801 5 2 o:3001:100:32" "rtt xpong 10.30.1.7:20123 10.30.1.5:20124 2" \
        "load xecho 10.30.1.7:22800 10.30.1.5:22801 2 3001"; do
        echo old >"$sbc/home/bench/rtt-histogram.csv"
        # shellcheck disable=SC2086
        env -i PATH="$sbc/bin:$PATH" HOME="$sbc/home" FAKE_LOG="$sbc/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" \
            bash "$cloud/vm.sh" client isolated:kernel $args >/dev/null 2>&1
        printf '%s %s;' "${args%% 10.*}" "$([[ -e $sbc/home/bench/rtt-histogram.csv ]] && echo kept || echo removed)"
    done
)
rm -f "$sbc/home/bench/rtt-histogram.csv"
if [[ $stale == "rtt xping removed;load xping removed;load xmix removed;rtt xpong kept;load xecho kept;" ]]; then
    pass "vm.sh client: the last run's histogram removed before a ping or a load client, never fetched as this run's"
else
    fail "vm.sh client and a stale histogram: $stale"
fi

# estimate.py counts the warm-up (LAB_WARMUP_SECONDS at LAB_WARMUP_RATE, once
# a mode and rep before its first ladder, at the first size: its seconds and
# a load point's 7 s of start and stop, and (seconds + 1) s of messages each
# way across the network, the client's own first second included), none for
# a +idle mode or a localhost one's bytes, nothing without load sizes; and
# prices c9g.2xlarge, c8gn.xlarge and c6in.4xlarge (us-east-1, on demand).
# The plan above (1,762 s; 162,208,000 B each way) with a 30 s warm-up at
# 25,000/s: 4 mode-reps of 37 s more, 1,910 s; and 4 x 31 s x 25,000 x
# (288 + 60) B = 1,078,800,000 B more each way. A +idle mode added: no more
# warm-up; a localhost mode: its seconds, no bytes.
python3 - "$cloud" <<'PY' && pass "estimate.py: the warm-up's time and bytes; c9g.2xlarge, c8gn.xlarge and c6in.4xlarge priced" || fail "estimate.py: the warm-up and the new prices"
import sys
sys.path.insert(0, sys.argv[1])
import estimate
plan = dict(type_a="c7i.4xlarge", type_b="c7i.4xlarge", placement="cross-az", reps=2, modes=["kernel", "dpdk"],
            local_modes=[], states=["isolated"], load_sizes=[288], load_fractions=[10, 50], load_ladder={288: [1000, 2000]},
            load_seconds=5, mix=[], mix_seconds=10, tput_seconds=5, tput_flows=0, rtt_messages=300000, rtt_warmup=50000,
            no_flatout=True, max_hours=8)
base = estimate.estimate(**plan)
assert base["wall_s"] == 1762 and base["bytes_ab"] == 162208000, base
w = estimate.estimate(**plan, warmup_seconds=30, warmup_rate=25000)
assert w["wall_s"] == 1762 + 4 * 37, w["wall_s"]
assert w["bytes_ab"] == 162208000 + 4 * 31 * 25000 * 348 and w["bytes_ba"] == w["bytes_ab"], (w["bytes_ab"], w["bytes_ba"])
idle = estimate.estimate(**dict(plan, modes=["kernel", "dpdk", "dpdk+idle"]), warmup_seconds=30, warmup_rate=25000)
assert idle["wall_s"] - w["wall_s"] == 2 * 97 and idle["bytes_ab"] - w["bytes_ab"] == 2 * 350000 * 92, idle
local = estimate.estimate(**dict(plan, local_modes=["dpdk"]), warmup_seconds=30, warmup_rate=25000)
assert local["wall_s"] - w["wall_s"] == 2 * (97 + 3 * 12 + 37) and local["bytes_ab"] == w["bytes_ab"], local
assert estimate.estimate(**dict(plan, load_sizes=[]), warmup_seconds=30, warmup_rate=25000)["wall_s"] == \
    estimate.estimate(**dict(plan, load_sizes=[]))["wall_s"]
for t, usd in (("c9g.2xlarge", 0.34776), ("c8gn.xlarge", 0.237), ("c6in.4xlarge", 0.9072)):
    assert estimate.PRICES.get(t) == usd, (t, estimate.PRICES.get(t), usd)
PY
# aws/lab.sh hands the warm-up to the estimate (LAB_ESTIMATE=1: no cloud call)
lab_aws "$tmp/estimate-nowarm" LAB_ESTIMATE=1 LAB_TYPE=c7i.4xlarge LAB_LOAD_SIZES=288 LAB_LOAD_LADDER=288:1000 LAB_NO_FLATOUT=1 || true
lab_aws "$tmp/estimate-warmup" LAB_ESTIMATE=1 LAB_TYPE=c7i.4xlarge LAB_LOAD_SIZES=288 LAB_LOAD_LADDER=288:1000 LAB_NO_FLATOUT=1 \
    LAB_WARMUP_SECONDS=30 LAB_WARMUP_RATE=25000 || true
with=$(sed -n 's/^wall time about \([0-9.]*\) h.*/\1/p' "$tmp/estimate-warmup.out")
without=$(sed -n 's/^wall time about \([0-9.]*\) h.*/\1/p' "$tmp/estimate-nowarm.out")
if [[ -s $tmp/estimate-nowarm || -s $tmp/estimate-warmup ]]; then
    fail "aws/lab.sh LAB_ESTIMATE=1 with a warm-up made cloud calls"
elif [[ -z $with || -z $without ]] || ! python3 -c 'import sys; sys.exit(abs(float(sys.argv[1]) - float(sys.argv[2]) - 3 * 2 * 5 * 37 / 3600) > 0.011)' "$with" "$without"; then
    fail "aws/lab.sh LAB_ESTIMATE=1: the warm-up not counted: $without h without, $with h with"
else
    pass "aws/lab.sh LAB_ESTIMATE=1 counts LAB_WARMUP_SECONDS (once a mode and rep), no cloud call"
fi

# Time sync on AWS. host.sh chrony_conf turns Debian 13's chrony.conf into the
# lab's: the pool and Debian's makestep commented out, and leapseclist too,
# or Debian 12's leapsectz (Amazon's Time Sync Service smears leap seconds,
# and Debian's own comment says to drop it for such a source), then AWS's
# line for the service at its
# link-local address, preferred, polled every 16 s, and a step of any offset
# over 100 us in the first three updates; under precision-time the ENA's PTP
# hardware clock first (refclock PHC /dev/ptp_ena, AWS's line: poll 0, the
# driver's default dpoll of 0, inside ENA's 125 reads a second) and the
# service as a backup, not preferred. Run again on its own output it changes
# nothing. phc_selected reads chronyc -c sources: 0 only with the PHC the
# selected source (#*) and its last offset within the bound, the line kept
# for the run's record. phc_udev_rule names the primary ENI's clock
# /dev/ptp_ena by its PCI address (no data ENI's), and phc_boot_script turns
# Linux's own ENA PHC on there at every boot (devlink enable_phc, a
# driverinit parameter, then a reload), checking the device came.
cat >"$tmp/chrony-debian.conf" <<'EOF'
# Welcome to the chrony configuration file. See chrony.conf(5) for more
# information about usable directives.

# Include configuration files found in /etc/chrony/conf.d.
confdir /etc/chrony/conf.d

# Use Debian vendor zone.
pool 2.debian.pool.ntp.org iburst

# Use time sources from DHCP.
sourcedir /run/chrony-dhcp

# Use NTP sources found in /etc/chrony/sources.d.
sourcedir /etc/chrony/sources.d

# This directive specify the location of the file containing ID/key pairs for
# NTP authentication.
keyfile /etc/chrony/chrony.keys

# This directive specify the file into which chronyd will store the rate
# information.
driftfile /var/lib/chrony/chrony.drift

# Save NTS keys and cookies.
ntsdumpdir /var/lib/chrony

# Uncomment the following line to turn logging on.
#log tracking measurements statistics

# Log files location.
logdir /var/log/chrony

# Stop bad estimates upsetting machine clock.
maxupdateskew 100.0

# This directive enables kernel synchronisation (every 11 minutes) of the
# real-time clock. Note that it can't be used along with the 'rtcfile' directive.
rtcsync

# Step the system clock instead of slewing it if the adjustment is larger than
# one second, but only in the first three clock updates.
makestep 1 3

# Get TAI-UTC offset and leap seconds from the system tz database.
# This directive must be commented out when using time sources serving
# leap-smeared time.
leapseclist /usr/share/zoneinfo/leap-seconds.list
EOF
timesync=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    chrony_conf aws <"$tmp/chrony-debian.conf" >"$tmp/chrony-aws.conf"
    chrony_conf precision-time <"$tmp/chrony-debian.conf" >"$tmp/chrony-pt.conf"
    echo "aws:"
    diff "$tmp/chrony-debian.conf" "$tmp/chrony-aws.conf" | grep '^[<>]'
    echo "precision-time:"
    grep -vxF -f "$tmp/chrony-debian.conf" "$tmp/chrony-pt.conf" | grep -v '^#'
    echo "again:$(chrony_conf aws <"$tmp/chrony-aws.conf" | cmp -s - "$tmp/chrony-aws.conf"; echo -n " $?")$(chrony_conf precision-time <"$tmp/chrony-pt.conf" | cmp -s - "$tmp/chrony-pt.conf"; echo -n " $?")$(chrony_conf aws <"$tmp/chrony-pt.conf" | cmp -s - "$tmp/chrony-aws.conf"; echo -n " $?")$(rc chrony_conf azure <"$tmp/chrony-debian.conf")"
    # Debian 12's chrony.conf took leap seconds from the tz database instead
    echo "leapsectz: $(echo 'leapsectz right/UTC' | chrony_conf aws | grep -cx '#leapsectz right/UTC')"
    selected='#,*,PHC0,0,0,377,0,-0.000000012,-0.000000015,0.000000005
^,-,169.254.169.123,3,4,377,10,0.000012000,0.000012100,0.000262000'
    far='#,*,PHC0,0,0,377,0,0.000012500,0.000012600,0.000000005'
    standby='#,+,PHC0,0,0,377,0,-0.000000012,-0.000000015,0.000000005
^,*,169.254.169.123,3,4,377,10,0.000012000,0.000012100,0.000262000'
    echo "phc:$(rc phc_selected 10000 <<<"$selected")$(rc phc_selected 10000 <<<"$far")$(rc phc_selected 10000 <<<"$standby")$(rc phc_selected 10000 </dev/null)"
    phc_selected 10000 <<<"$selected"
    phc_selected 10000 <<<"$far"
    phc_selected 10000 <<<"$standby"
    echo "udev: $(phc_udev_rule 0000:00:05.0)"
    phc_boot_script 0000:00:05.0 | grep -v '^#'
) || true
want="aws:
< pool 2.debian.pool.ntp.org iburst
> #pool 2.debian.pool.ntp.org iburst
< makestep 1 3
> #makestep 1 3
< leapseclist /usr/share/zoneinfo/leap-seconds.list
> #leapseclist /usr/share/zoneinfo/leap-seconds.list
> # ergon-lab time sync begin (vm.sh bootstrap)
> # The Amazon Time Sync Service at its link-local address, AWS's own line for
> # chrony. It serves leap-smeared time, so no leapseclist (above). Any offset
> # over 100 us stepped in the first three updates, slewed after.
> server 169.254.169.123 prefer iburst minpoll 4 maxpoll 4
> makestep 0.0001 3
> # ergon-lab time sync end
precision-time:
refclock PHC /dev/ptp_ena poll 0 delay 0.000010 prefer
server 169.254.169.123 iburst minpoll 4 maxpoll 4
makestep 0.0001 3
again: 0 0 0 2
leapsectz: 1
phc: 0 2 2 2
phc PHC0 selected, last offset -12 ns (within 10000 ns)
phc PHC0 selected, last offset 12500 ns: over 10000 ns
phc not selected: chrony's selected source is 169.254.169.123 (PHC0 is +)
udev: SUBSYSTEM==\"ptp\", KERNELS==\"0000:00:05.0\", SYMLINK+=\"ptp_ena\"
set -e
for _ in \$(seq 60); do [ -e /sys/bus/pci/devices/0000:00:05.0/net ] && break; sleep 0.5; done
devlink dev param set pci/0000:00:05.0 name enable_phc value true cmode driverinit
devlink dev reload pci/0000:00:05.0
udevadm settle
test -e /dev/ptp_ena"
if [[ $timesync != "$want" ]]; then
    fail "host.sh: chrony's configuration on AWS, the PHC's selection, its udev name and boot-time enabling"
    diff -u <(echo "$want") <(echo "$timesync") >&2 || true
else
    pass "host.sh: chrony on Amazon's Time Sync Service (pool and leapseclist out, makestep 100 us), precision-time's PHC first, idempotent; the PHC selected within its bound or not; /dev/ptp_ena by the primary ENI's address, enabled by devlink at every boot"
fi
# vm.sh's own steps, from vm.sh with fakes (sudo logs and runs only install,
# chmod and tee into the test's tree): timesync_setup (bootstrap, AWS) puts
# chrony in place of systemd-timesyncd, on Amazon's Time Sync Service alone
# under every placement (precision-time's PHC is not there yet, and chrony
# before 4.9 will not start with a refclock whose device is missing), enabled
# and restarted; on Azure it leaves the image's chrony (ptp_hyperv) alone.
# phc_setup (build-ptp, precision-time) finds the primary ENI by the default
# route, never a data ENI, installs the udev rule and the boot-time unit for
# it and enables the unit, and puts chrony on the PHC first, the service its
# backup, from the next boot (installed, never restarted: the device comes
# with that boot); it builds no out-of-tree ENA driver (no amzn-drivers, no
# module options, no initramfs) and fails closed, chrony left alone, where
# the default route's NIC has no PCI address or is a data NIC.
ts=$tmp/vm-timesync
mkdir -p "$ts/etc/chrony" "$ts/home/bench" "$ts/etc/udev/rules.d" "$ts/etc/systemd/system" "$ts/usr/local/sbin"
steps=$(
    set +e
    exec 2>&1
    # shellcheck source=../host.sh
    . "$cloud/host.sh" || exit 1
    eval "$(for f in timesync_setup phc_setup; do vm_fn "$f"; done)"
    run=$ts/home/bench LAB_ETC_ROOT=$ts/etc LAB_USR_ROOT=$ts/usr
    sudo() {
        echo "sudo $*" >>"$ts/sudo"
        case $1 in
        install | chmod | tee) [[ $* == *"$ts/"* ]] && "$@" ;;
        DEBIAN_FRONTEND=* | systemctl) return 0 ;;
        *) echo "sudo $* (not run)" >>"$ts/sudo"; return 0 ;;
        esac
    }
    ip() { echo "default via 10.30.0.1 dev ${FAKE_DEV:-ens5} proto dhcp src 10.30.0.10 metric 100"; }
    ethtool() { [[ $1 == -i && $2 == ens5 ]] && printf 'driver: ena\nbus-info: 0000:00:05.0\n'; [[ $1 == -i && $2 == ens6 ]] && printf 'driver: ena\nbus-info: 0000:00:06.0\n'; return 0; }
    nics() { printf 'sender ens6 ens6 m6 10.30.1.4 24 0000:00:06.0\nreceiver ens7 ens7 m7 10.30.1.5 24 0000:00:07.0\n'; }
    for case in aws precision-time azure; do
        cp "$tmp/chrony-debian.conf" "$ts/etc/chrony/chrony.conf"
        : >"$ts/sudo"
        CLOUD=aws PLACEMENT=$case
        [[ $case == azure ]] && CLOUD=azure PLACEMENT=
        precision_time() { [[ $CLOUD == aws && $PLACEMENT == precision-time ]]; }
        timesync_setup >/dev/null 2>&1
        echo "$case: rc $? $(grep -c '^server 169.254.169.123' "$ts/etc/chrony/chrony.conf") server, $(grep -c '^refclock PHC /dev/ptp_ena' "$ts/etc/chrony/chrony.conf") refclock;" \
            "calls [$(grep -E 'apt-get install|systemctl' "$ts/sudo" | sed 's/^sudo //; s/DEBIAN_FRONTEND=noninteractive //' | tr '\n' ';')]"
    done
    : >"$ts/sudo"
    CLOUD=aws PLACEMENT=precision-time
    # build-ptp runs on bootstrap's chrony.conf
    chrony_conf aws <"$tmp/chrony-debian.conf" >"$ts/etc/chrony/chrony.conf"
    phc_setup >/dev/null 2>&1
    echo "phc: rc $? $(cat "$ts/etc/udev/rules.d/53-ec2-network-interfaces-ptp.rules") | $(grep -c 'pci/0000:00:05.0 name enable_phc' "$ts/usr/local/sbin/ergon-lab-phc") enable_phc | $(grep -E '^(ExecStart|Before)=' "$ts/etc/systemd/system/ergon-lab-phc.service" | tr '\n' ' ')| $(grep -E 'systemctl|amzn|modprobe|initramfs|depmod|git ' "$ts/sudo" | sed 's/^sudo //' | tr '\n' ';')"
    echo "phc chrony: $(grep -cx 'refclock PHC /dev/ptp_ena poll 0 delay 0.000010 prefer' "$ts/etc/chrony/chrony.conf") refclock, $(grep -cx 'server 169.254.169.123 iburst minpoll 4 maxpoll 4' "$ts/etc/chrony/chrony.conf") backup, $(grep -c '^server 169.254.169.123 prefer' "$ts/etc/chrony/chrony.conf") preferred"
    rm -f "$ts/etc/udev/rules.d/53-ec2-network-interfaces-ptp.rules"
    chrony_conf aws <"$tmp/chrony-debian.conf" >"$ts/etc/chrony/chrony.conf"
    FAKE_DEV=ens6 phc_setup >/dev/null 2>&1
    echo "data NIC: rc $? $(ls "$ts/etc/udev/rules.d" | wc -l | tr -d ' ') rules, $(grep -c '^refclock' "$ts/etc/chrony/chrony.conf") refclock"
    FAKE_DEV=ens9 phc_setup >/dev/null 2>&1
    echo "no address: rc $?"
) || true
want="aws: rc 0 1 server, 0 refclock; calls [apt-get install -yq chrony;systemctl disable --now systemd-timesyncd;systemctl enable chrony;systemctl restart chrony;]
precision-time: rc 0 1 server, 0 refclock; calls [apt-get install -yq chrony;systemctl disable --now systemd-timesyncd;systemctl enable chrony;systemctl restart chrony;]
azure: rc 0 0 server, 0 refclock; calls []
phc: rc 0 SUBSYSTEM==\"ptp\", KERNELS==\"0000:00:05.0\", SYMLINK+=\"ptp_ena\" | 1 enable_phc | Before=network-pre.target chrony.service ExecStart=/usr/local/sbin/ergon-lab-phc | systemctl daemon-reload;systemctl enable ergon-lab-phc.service;
phc chrony: 1 refclock, 1 backup, 0 preferred
data NIC: rc 1 0 rules, 0 refclock
no address: rc 1"
if [[ $steps != "$want" ]]; then
    fail "vm.sh: chrony in place of systemd-timesyncd on AWS, the PHC set up on the primary ENI"
    diff -u <(echo "$want") <(echo "$steps") >&2 || true
else
    pass "vm.sh: bootstrap's chrony on AWS (Azure's left), build-ptp's PHC on the primary ENI by devlink at every boot, no out-of-tree driver, fail closed"
fi

# lib.sh, dry: the clocks around every mode and before every state. In each
# state, after the pre-flight and before anything is measured, both hosts
# wait for chrony to settle (vm.sh timesync-wait: chronyc waitsync, bounding
# the correction still to apply, not the error against UTC), and a host that
# never does stops the run there; around each mode both hosts log chrony's
# tracking and sources (vm.sh timesync, <node>-timesync-before-/after-...), the
# one before the drivers start a check (precision-time: the PHC selected
# within its bound) whose failure stops the run before that mode's drivers.
cat >"$tmp/dry-timesync.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2 repo=$out/repo
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
on() {
    local host=$1 cmd
    shift
    cmd=$*
    printf '%s %s\n' "$host" "$cmd" >>"$out/calls"
    case $cmd in
    *"vm.sh nics") echo "receiver ens6 ens6v 00:00:00:00:00:00 $([[ $host == "$A" ]] && echo 10.30.1.5 || echo 10.30.1.7) 24 0000:00:07.0" ;;
    *"vm.sh layout") echo "0 2 1 3 4" ;;
    *"vm.sh timesync-wait") [[ ${FAKE_WAIT_FAIL:-} == "$host" ]] && { echo "waitsync: not synchronised"; return 1; } ;;
    *"vm.sh timesync check") echo "Reference ID : A9FEA97B (169.254.169.123)"; [[ ${FAKE_CHECK_FAIL:-} == "$host" ]] && return 1 ;;
    *"vm.sh timesync") echo "Reference ID : A9FEA97B (169.254.169.123)" ;;
    *"/rc 2>/dev/null") echo 0 ;;
    esac
    return 0
}
reps=1 local_modes= load_sizes= mix= rtt_messages=10 rtt_warmup=1 rtt_align= tput_seconds=5 no_flatout=1 states=(${FAKE_STATE:-pinned})
modes=(kernel dpdk)
if [[ $3 == state ]]; then
    remote_state() { echo "remote_state $1" >>"$out/calls"; return 0; }
    run_bench
else
    bench_state pinned
fi
echo "rc $?" >>"$out/calls"
SH
tsd=$tmp/dry-timesync
for case in mode mode-fail state state-fail state-nodiag; do
    mkdir -p "$tsd-$case/repo/md-dpdk/benches/cloud"
    env FAKE_CHECK_FAIL="$([[ $case == mode-fail ]] && echo 192.0.2.2)" FAKE_WAIT_FAIL="$([[ $case == state-fail ]] && echo 192.0.2.1)" \
        FAKE_STATE="$([[ $case == state-nodiag ]] && echo pinned-nodiag)" \
        bash "$tmp/dry-timesync.sh" "$cloud" "$tsd-$case" "$([[ $case == state-nodiag ]] && echo state || echo "${case%-fail}")" </dev/null >"$tsd-$case/stdout" 2>"$tsd-$case/stderr" || true
done
# a -nodiag state: the same steps without any diagnostics (lib.sh run_bench)
nodiagsteps=$(grep -E '^(192.0.2.1 |remote_state)' "$tsd-state-nodiag/calls" | sed -nE 's#^192.0.2.1 [^ ]*vm.sh (preflight 120|timesync-wait|diag-[a-z]+)$#\1#p; s#^(remote_state .*)#\1#p' | tr '\n' ';') || true
if [[ $nodiagsteps != "preflight 120;timesync-wait;remote_state pinned-nodiag;" ]] || ! grep -q '^rc 0' "$tsd-state-nodiag/calls" ||
    ! grep -q 'state pinned-nodiag: no diagnostics' "$tsd-state-nodiag/log"; then
    fail "lib.sh dry: a -nodiag state ran diagnostics or not its steps: $nodiagsteps"
else
    pass "lib.sh dry: a -nodiag state runs without vm.sh diag-start, diag-fetch or diag-stop, and says so"
fi
# per mode, VM a's: its check, driver start, stop and the log after
modesteps=$(grep '^192.0.2.1 ' "$tsd-mode/calls" | sed -nE 's#^192.0.2.1 [^ ]*vm.sh (timesync check|timesync|driver [a-z]+|stop)$#\1#p' | tr '\n' ';') || true
statesteps=$(grep -E '^(192.0.2.1 |remote_state)' "$tsd-state/calls" | sed -nE 's#^192.0.2.1 [^ ]*vm.sh (preflight 120|timesync-wait|diag-start)$#\1#p; s#^(remote_state .*)#\1#p' | tr '\n' ';') || true
if grep -E 'unbound variable|command not found|syntax error' "$tsd"-*/stderr; then
    fail "lib.sh dry, the clocks: shell errors"
elif [[ $modesteps != "timesync check;driver kernel;stop;timesync;timesync check;driver dpdk;stop;timesync;" &&
    $modesteps != "timesync check;driver dpdk;stop;timesync;timesync check;driver kernel;stop;timesync;" ]] ||
    [[ $(ls "$tsd-mode" | grep -c '^[ab]-timesync-\(before\|after\)-pinned-\(kernel\|dpdk\)-1\.log$') != 8 ]]; then
    fail "lib.sh dry: chrony around each mode: $modesteps $(ls "$tsd-mode" | grep timesync | tr '\n' ' ')"
elif grep -q 'vm.sh driver' "$tsd-mode-fail/calls" || ! grep -q 'ERROR: .*clock' "$tsd-mode-fail/log"; then
    fail "lib.sh dry: a failed clock check did not stop the run before the drivers: $(tail -2 "$tsd-mode-fail/log")"
elif [[ $statesteps != "preflight 120;timesync-wait;diag-start;remote_state pinned;" ]] || ! grep -q '^rc 0' "$tsd-state/calls"; then
    fail "lib.sh dry: chrony's wait before the state: $statesteps"
elif grep -q 'remote_state\|diag-start' "$tsd-state-fail/calls" || ! grep -q 'ERROR: .*192.0.2.1.*\|ERROR: .*synchronis' "$tsd-state-fail/log"; then
    fail "lib.sh dry: a host whose clock never settles did not stop the run: $(tail -2 "$tsd-state-fail/log")"
else
    pass "lib.sh dry: chrony settled on both hosts before every state (the run stops if not), logged around every mode, its check stopping a mode before its drivers"
fi
# vm.sh timesync and timesync-wait in a sandbox, a fake chronyc logging its
# calls: on AWS the wait is chronyc waitsync's (300 tries a second apart:
# chrony's skew after a boot falls below 1 ppm over minutes, polling every
# 16 s, and a run given up on is lost; 10 us of correction, 1 ppm of skew)
# and fails with it; tracking and sources are
# printed, and under precision-time `check` gates on the PHC (selected, within
# 10 us), saying so; on Azure (its image's chrony on the Hyper-V clock) there
# is no wait, and no gate
sbt=$tmp/vm-chrony
mkdir -p "$sbt/bin" "$sbt/home/ergon" "$sbt/home/bench"
cp "$sb/bin/"* "$sbt/bin/"
# shellcheck disable=SC2016
cat >"$sbt/bin/chronyc" <<'SH'
#!/bin/sh
echo "chronyc $*" >>"$FAKE_LOG"
case "$*" in
waitsync*) exit "${FAKE_WAIT_RC:-0}" ;;
"-n tracking") echo "System time     : 0.000000123 seconds fast of NTP time" ;;
"-n sources") echo "#* PHC0  0   0   377     1    -12ns[  -15ns] +/-   5ns" ;;
"-c -n sources") echo "${FAKE_SOURCES:-#,*,PHC0,0,0,377,0,-0.000000012,-0.000000015,0.000000005}" ;;
esac
SH
chmod +x "$sbt/bin/chronyc"
chrony_sb() { # <cloud> <placement> <vm.sh args>...: "rc <rc>: <chronyc calls>; <output's last line>"
    local rc=0
    echo "$1" >"$sbt/home/bench/cloud"
    echo "$2" >"$sbt/home/bench/placement"
    : >"$sbt/log"
    env -i PATH="$sbt/bin:$PATH" HOME="$sbt/home" FAKE_LOG="$sbt/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" \
        FAKE_WAIT_RC="${FAKE_WAIT_RC:-0}" FAKE_SOURCES="${FAKE_SOURCES:-}" bash "$cloud/vm.sh" "${@:3}" >"$sbt/out" 2>&1 || rc=$?
    echo "rc $rc: $(sed -n 's/^chronyc //p' "$sbt/log" | tr '\n' ';') $(tail -1 "$sbt/out")"
}
got="aws wait: $(chrony_sb aws cluster timesync-wait)
aws wait fails: $(FAKE_WAIT_RC=1 chrony_sb aws cluster timesync-wait)
azure wait: $(chrony_sb azure '' timesync-wait)
aws log: $(chrony_sb aws cluster timesync check)
pt check: $(chrony_sb aws precision-time timesync check)
pt check fails: $(FAKE_SOURCES='^,*,169.254.169.123,3,4,377,10,0.000012,0.000012,0.000262' chrony_sb aws precision-time timesync check)
pt log: $(FAKE_SOURCES='^,*,169.254.169.123,3,4,377,10,0.000012,0.000012,0.000262' chrony_sb aws precision-time timesync)"
want="aws wait: rc 0: waitsync 300 0.00001 1 1;-n tracking; System time     : 0.000000123 seconds fast of NTP time
aws wait fails: rc 1: waitsync 300 0.00001 1 1;-n tracking; timesync-wait: chrony did not settle in 5 minutes (waitsync: correction under 10 us, skew under 1 ppm): the run's clocks are not to be trusted
azure wait: rc 0:  timesync-wait: Azure: the image's chrony on the Hyper-V clock (ptp_hyperv), not gated
aws log: rc 0: -n tracking;-n sources; #* PHC0  0   0   377     1    -12ns[  -15ns] +/-   5ns
pt check: rc 0: -n tracking;-n sources;-c -n sources; phc PHC0 selected, last offset -12 ns (within 10000 ns)
pt check fails: rc 1: -n tracking;-n sources;-c -n sources; timesync: precision-time without the PHC as chrony's source: stopping
pt log: rc 0: -n tracking;-n sources; #* PHC0  0   0   377     1    -12ns[  -15ns] +/-   5ns"
if [[ $got != "$want" ]]; then
    fail "vm.sh timesync and timesync-wait"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
else
    pass "vm.sh timesync-wait: chronyc waitsync on AWS (fails with it), none on Azure; timesync: tracking and sources, the PHC gate under precision-time"
fi

# spikes.py notes the hosts' clocks where it matches spikes on one host to the
# other's diagnostics: from chrony's tracking logged around each mode (vm.sh
# timesync, <host>-timesync-*.log), each host's largest system-time offset
# from its source and that source's own bound against UTC (root dispersion
# and half the root delay), and their sum, how far the two clocks may differ;
# without the logs (runs from before them) no such note
python3 - "$cloud" "$tmp/clocks" <<'PY' && pass "spikes.py: the hosts' clock offsets noted where spikes are matched across them" || fail "spikes.py: the hosts' clock offsets"
import os, subprocess, sys
cloud, d = sys.argv[1], sys.argv[2]
start = 1792032877.0
state = os.path.join(d, "isolated")
os.makedirs(state)
with open(os.path.join(state, "spikes-isolated-kernel-rtt-1.csv"), "w") as f:
    f.write(f"# start_realtime_ns={int(start * 1e9)}\nstream,due_ns,latency_ns\n0,1000000000,400000\n")
for host in "ab":
    with open(os.path.join(d, f"{host}-diag-isolated.log"), "w") as f:
        f.write("== gapspin\n== trace\n")
run = lambda: subprocess.run([sys.executable, os.path.join(cloud, "spikes.py"), state], capture_output=True, text=True, check=True).stdout
before = run()
def tracking(name, system, way, delay, dispersion):
    with open(os.path.join(state, name), "w") as f:
        f.write(f"Reference ID    : A9FEA97B (169.254.169.123)\nStratum         : 4\nSystem time     : {system} seconds {way} of NTP time\n"
                f"Last offset     : +0.000000012 seconds\nRoot delay      : {delay} seconds\nRoot dispersion : {dispersion} seconds\n"
                "Leap status     : Normal\nMS Name/IP address         Stratum Poll Reach LastRx Last sample\n"
                "^* 169.254.169.123               3   4   377    10    +12us[  +12us] +/-  262us\n")
tracking("a-timesync-before-isolated-kernel-1.log", "0.000000800", "fast", "0.000200000", "0.000100000")
tracking("a-timesync-after-isolated-kernel-1.log", "0.000001500", "slow", "0.000200000", "0.000120000")
tracking("b-timesync-before-isolated-kernel-1.log", "0.000003000", "fast", "0.000300000", "0.000050000")
after = run()
note = ("Host clocks, from chrony's tracking around each mode (vm.sh timesync): a's system time at most 1.5 µs from its "
        "source, that source within 220.0 µs of UTC (root dispersion and half the root delay; 2 snapshots); b's at most "
        "3.0 µs, its source within 200.0 µs (1 snapshot). Spikes are matched to the other host's diagnostics across these "
        "clocks, which may differ by up to 424.5 µs (the sum).")
assert note in after, after
assert "Host clocks" not in before and after.replace("\n\n" + note, "") == before, (before, after)
PY

# ATTR-LAB and KNOISE in spikes.py: each round trip run's 10 largest round
# trips per rep, each with every event whose interval overlaps it ([sent,
# echoed] on the wall clock), its ping index and its echo's phase in the
# second and in 30 s; and how often each source overlaps them against chance
# (1,000 circular shifts of the rep's round trips within its run, a fixed
# seed). Sources: either client's loop stalls, the DPDK transport's event log
# (RUSTERON_DPDK_EVENT_LOG, saved as <host>-rings-<run>.log: rows
# kind,tsc,realtime_ns,value_ns,port,agent, ended at realtime_ns: agent loop
# gaps, ARP, the housekeeping thread's 1 Hz sample, NIC statistics read and
# PMD timers; the events its full rings dropped noted), the
# gap spinner, and diag-fetch's knoise section (osnoise's irq, softirq,
# thread and NMI noise with their durations, IPIs sent to the agent and app
# CPUs by callsite, compaction and page migration). One fixture spike for
# each label, one with none, an 11th smaller spike left out; each label
# proven by mutation (its event moved 100 s away removes it and only it).
# The agents' and clients' own busy threads are never thread noise; an IPI
# to no agent or app CPU is never labelled. Rows with no ping index (files
# from before it) read as before, in analyse.py too
python3 - "$cloud" "$tmp/attr" <<'PY' && pass "spikes.py: every event overlapping each rep's 10 largest round trips, labelled (each proven by mutation), phases, ping index, a permutation baseline" || fail "spikes.py: the 10 largest round trips and their overlapping events"
import os, re, shutil, subprocess, sys
cloud, d = sys.argv[1], sys.argv[2]
sys.path.insert(0, cloud)
START = 1792032877_000_000_000  # the measured run's CLOCK_REALTIME start (ns): 7 s into a 30 s period
UP0 = 1000.0  # the trace clock (uptime) at START
S = 1_000_000_000

def up(sec):  # the trace clock at START + sec
    return f"{UP0 + sec:.6f}"

def start_of(sec):  # osnoise's "start" field
    return f"{UP0 + sec:.9f}"

def fixture(name, moved=()):
    """A run directory with one isolated round trip rep; each event named in
    `moved` placed 100 s later, out of every spike."""
    off = lambda key: 100.0 if key in moved else 0.0
    root = os.path.join(d, name)
    state = os.path.join(root, "isolated")
    os.makedirs(state)
    spikes = [(i, 76_000 - 1_000 * i, 1000 * i) for i in range(1, 11)]  # (second, latency ns, ping)
    with open(os.path.join(state, "spikes-isolated-dpdk-rtt-1.csv"), "w") as f:
        f.write(f"# start_realtime_ns={START}\n# spikes_total=11\nstream,due_ns,latency_ns,ping\n")
        for sec, lat, ping in spikes:
            f.write(f"1002,{sec * S},{lat},{ping}\n")
        f.write(f"1002,{11 * S},31000,11000\n")  # the 11th, overlapping a stall: left out
    # 20 s measured (histogram: the run's round trips' sum)
    with open(os.path.join(state, "histogram-isolated-dpdk-1.csv"), "w") as f:
        f.write("value_ns,count\n50000,400000\n")
    with open(os.path.join(state, "stalls-ping-isolated-dpdk-1.csv"), "w") as f:
        f.write(f"# start_realtime_ns={START}\nend_ns,gap_ns\n{int((1.00007 + off('stall')) * S)},60000\n{int(11.00002 * S)},15000\n")
    with open(os.path.join(state, "stalls-pong-isolated-dpdk-1.csv"), "w") as f:
        f.write(f"# start_realtime_ns={START}\nend_ns,gap_ns\n")
    with open(os.path.join(state, "b-rings-isolated-dpdk-1.log"), "w") as f:
        f.write("# kind,tsc,realtime_ns,value_ns,port,agent\n")
        f.write(f"gap,77001,{START + int((2.00005 + off('gap')) * S)},30000,1,4242\n")
    with open(os.path.join(state, "a-rings-isolated-dpdk-1.log"), "w") as f:
        f.write("# kind,tsc,realtime_ns,value_ns,port,agent\n")
        f.write(f"arp-request,77002,{START + int((2.00004 + off('arp')) * S)},0,0,4300\n")
        f.write(f"nic-stats,77003,{START + int((3.00003 + off('hk')) * S)},40000,0,4100\n")
        f.write(f"drops,77004,{START + 19 * S},5,0,4100\n")
    knoise = {
        "a": [
            f"  ksoftirqd/2-20     [002] d.h1.  {up(4.00004 + off('irq'))}: irq_noise: ena-mgmnt:24 start {start_of(4.00001 + off('irq'))} duration 20000 ns",
            f"      <idle>-0       [002] ..s1.  {up(6.00002 + off('softirq'))}: softirq_noise:    TIMER:1 start {start_of(6.00001 + off('softirq'))} duration 5000 ns",
            f"      <idle>-0       [002] d.Z1.  {up(7.00002 + off('nmi'))}: nmi_noise: start {start_of(7.00001 + off('nmi'))} duration 3000 ns",
            f"     kworker/0:1-30   [000] d..1.  {up(7.00003 + off('ipi1'))}: ipi_send_cpu: cpu=2 callsite=generic_exec_single+0x30/0x160 callback=nohz_csd_func+0x0/0x90",
            f"     kworker/0:1-30   [000] d..1.  {up(10.00003)}: ipi_send_cpu: cpu=0 callsite=generic_exec_single+0x30/0x160 callback=nohz_csd_func+0x0/0x90",
            f"    kcompactd0-40    [000] .....  {up(9.00001 + off('compaction'))}: mm_compaction_begin: zone_start=0x1000 migrate_pfn=0x1000 free_pfn=0x2000 zone_end=0x3000, mode=async",
            f"    kcompactd0-40    [000] .....  {up(9.00002 + off('migration'))}: mm_migrate_pages: nr_succeeded=3 nr_failed=0 nr_thp_succeeded=0 nr_thp_failed=0 nr_thp_split=0 nr_split=0 mode=MIGRATE_ASYNC reason=compaction",
            # the agent's own thread: busy, never noise
            f"      sender-4242    [002] d..2.  {up(19.0)}: thread_noise:   sender:4242 start {start_of(0.5)} duration 18000000000 ns",
        ],
        "b": [
            f"     kworker/3:1-77   [003] d..2.  {up(5.00003 + off('thread'))}: thread_noise: kworker/3:1:77 start {start_of(5.00001 + off('thread'))} duration 15000 ns",
            f"     kworker/0:1-30   [000] d..1.  {up(8.00002 + off('ipi'))}: ipi_send_cpumask: cpumask={'0c' if 'ipi-mask' not in moved else '01'} callsite=smp_call_function_many_cond+0x1a4/0x5c0 callback=do_sync_core+0x0/0x30",
            # to CPU 0 only (housekeeping): never labelled
            f"     kworker/0:1-30   [000] d..1.  {up(8.00003)}: ipi_send_cpumask: cpumask=01 callsite=on_each_cpu_cond_mask+0x20/0x40 callback=do_flush_tlb_all+0x0/0x30",
        ],
    }
    gapspin = {"a": "", "b": f"{START / S + 6.00003 + off('host'):.6f} 20\n"}
    for host in "ab":
        with open(os.path.join(root, f"{host}-diag-isolated.log"), "w") as f:
            f.write("== gapspin\n" + gapspin[host] + "== trace\n")
            f.write(f"UPTIME {UP0} REALTIME {START / S}\n== drains\n== ipmon\n== knoise\n# cpus 1,2,3,4\n# cpumask hex\n")
            f.write("\n".join(knoise[host]) + "\n")
    return state

def run(state):
    return subprocess.run([sys.executable, os.path.join(cloud, "spikes.py"), state], capture_output=True, text=True, check=True).stdout

def table(out, heading):
    lines = out.split("\n")
    i = lines.index(heading)
    rows = []
    for line in lines[i + 1:]:
        if line.startswith("## "):
            break
        if line.startswith("| ") and not line.startswith("| ---") and not line.startswith("| run "):
            rows.append([c.strip() for c in line.strip("|").split("|")])
    return rows

TOP = "## The 10 largest round trips of each rep and every event overlapping them, isolated"
BASE = "## Overlaps against chance (1,000 circular shifts of each rep's round trips, seed 1), isolated"
want = {
    1000: "ping stall 60.0 µs",
    2000: "ARP request port 0 (a); agent gap port 1 30.0 µs (b tid 4242)",
    3000: "1 Hz NIC stats 40.0 µs (a port 0)",
    4000: "irq_noise ena-mgmnt:24 20.0 µs (a cpu 2)",
    5000: "thread_noise kworker/3:1:77 15.0 µs (b cpu 3)",
    6000: "host stall 20 µs (b); softirq_noise TIMER:1 5.0 µs (a cpu 2)",
    7000: "IPI generic_exec_single to 2 (a); nmi_noise 3.0 µs (a cpu 2)",
    8000: "IPI smp_call_function_many_cond to 2,3 (b)",
    9000: "compaction (a); page migration succeeded=3 reason=compaction (a)",
    10000: "none",
}
out = run(fixture("all"))
rows = table(out, TOP)
assert len(rows) == 10, out
got = {int(r[2]): r[6] for r in rows}
assert got == want, (got, out)
# the largest first; its round trip, its echo's phase in the second (ms) and in 30 s (s)
assert rows[0][:6] == ["isolated-dpdk-rtt", "1", "1000", "75.0", "0.075", "8.000"], rows[0]
assert rows[9][:6] == ["isolated-dpdk-rtt", "1", "10000", "66.0", "0.066", "17.000"], rows[9]
# the baseline: deterministic, one row a source, the observed count of 10
base = table(out, BASE)
assert out == run(fixture("again")), "the baseline is not deterministic"
sources = {r[1]: r for r in base}
assert "a's event log dropped 5 events in isolated-dpdk-1 (full rings): an overlap there may be missing." in out, out
assert set(sources) == {"ping stall", "agent gap", "ARP", "1 Hz NIC stats", "irq_noise", "thread_noise", "host stall", "softirq_noise",
                        "IPI", "nmi_noise", "compaction", "page migration"}, base
assert sources["IPI"][2] == "2 of 10" and sources["ping stall"][2] == "1 of 10", base
for r in base:
    assert float(r[3]) < 0.1 and 0 < float(r[4]) <= 1, r
# each label proven by mutation: its event moved away, it alone goes
for moved, ping, gone in (("stall", 1000, "ping stall 60.0 µs"), ("gap", 2000, "agent gap port 1 30.0 µs (b tid 4242)"),
                          ("arp", 2000, "ARP request port 0 (a)"), ("hk", 3000, "1 Hz NIC stats 40.0 µs (a port 0)"), ("irq", 4000, "irq_noise ena-mgmnt:24 20.0 µs (a cpu 2)"),
                          ("thread", 5000, "thread_noise kworker/3:1:77 15.0 µs (b cpu 3)"), ("host", 6000, "host stall 20 µs (b)"),
                          ("softirq", 6000, "softirq_noise TIMER:1 5.0 µs (a cpu 2)"), ("nmi", 7000, "nmi_noise 3.0 µs (a cpu 2)"),
                          ("ipi1", 7000, "IPI generic_exec_single to 2 (a)"), ("ipi", 8000, "IPI smp_call_function_many_cond to 2,3 (b)"),
                          ("ipi-mask", 8000, "IPI smp_call_function_many_cond to 2,3 (b)"), ("compaction", 9000, "compaction (a)"),
                          ("migration", 9000, "page migration succeeded=3 reason=compaction (a)")):
    mutated = {int(r[2]): r[6] for r in table(run(fixture(f"moved-{moved}", (moved,))), TOP)}
    left = "; ".join(l for l in want[ping].split("; ") if l != gone) or "none"
    assert mutated == {**want, ping: left}, (moved, mutated[ping], left)
# chance: a source at every one of the 10 spikes, spread over 20 s, is far
# above chance (p at its floor, 1/1001): the 1 Hz sample overlapping each
state = fixture("periodic")
with open(os.path.join(state, "a-rings-isolated-dpdk-1.log"), "w") as f:
    for sec in range(1, 11):
        f.write(f"sample,1,{START + int((sec + 0.00002) * S)},10000,0,4100\n")
periodic = {r[1]: r for r in table(run(state), BASE)}
assert periodic["1 Hz sample"][2] == "10 of 10" and periodic["1 Hz sample"][4] == "0.001", periodic["1 Hz sample"]
# a spike file without the ping index (before it): read as before, "-" for its ping
state = fixture("noindex")
path = os.path.join(state, "spikes-isolated-dpdk-rtt-1.csv")
text = open(path).read().replace(",ping\n", "\n")
open(path, "w").write(re.sub(r"^(1002,\d+,\d+),\d+$", r"\1", text, flags=re.M))
old = table(run(state), TOP)
assert [r[2] for r in old] == ["-"] * 10 and [r[6] for r in old] == [want[p] for p in sorted(want)], old
# analyse.py counts both alike (round trips of 100 us or more: none here at
# 66-76 us, so a copy at 150 us)
import analyse, io, contextlib
for name in ("all", "noindex"):
    src = os.path.join(d, name, "isolated", "spikes-isolated-dpdk-rtt-1.csv")
    big = os.path.join(d, name, "isolated", "spikes-isolated-dpdk-rtt-2.csv")
    open(big, "w").write(re.sub(r"^(1002,\d+,)\d+", r"\g<1>150000", open(src).read(), flags=re.M))
    os.remove(src)
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        analyse.spikes(os.path.join(d, name, "isolated"))
    assert re.search(r"isolated-dpdk-rtt +\| 1 +\| 11 ", buf.getvalue()), (name, buf.getvalue())
PY

# every aws/lab.sh run above (refused, estimated, run end to end, failed,
# interrupted, reclaimed, deleted) removed its snapshot as it exited, and the
# teardown still ran first (each run's verification is checked above); a kept
# run's message names the checkout's lab.sh to delete it with, never the
# snapshot it ran from, which is gone by then
left=$(find "$tmp/labtmp" "$tmp/commatmp" "$tmp"/full-*/tmp -mindepth 1 -maxdepth 1 -name 'ergon-lab.*' 2>/dev/null | wc -l | tr -d ' ') || true
if [[ $left != 0 ]]; then
    fail "lab.sh snapshots left behind: $(find "$tmp/labtmp" "$tmp/commatmp" "$tmp"/full-*/tmp -mindepth 1 -maxdepth 1 -name 'ergon-lab.*' 2>/dev/null | head -3)"
elif ! [[ $keep_said =~ \ kept\;\ delete\ it\ with:\ LAB_DELETE=[0-9]{8}-[0-9]{6}\ (.*)$ && ${BASH_REMATCH[1]} == "$cloud/aws/lab.sh" ]]; then
    fail "aws/lab.sh LAB_KEEP: the delete command names the snapshot, not the checkout's lab.sh: $keep_said"
else
    pass "aws/lab.sh removes its snapshot on every exit (refused, estimated, failed, interrupted, reclaimed, deleted), the teardown first; LAB_KEEP's delete command names the checkout's lab.sh"
fi

# vm.sh override_export: a mode's @KEY=value as the driver's export line; the
# ENA device arguments, comma-separated for the PMD, are written with + in a
# mode (whose name cannot hold a comma), which mode_check takes
devargs=$(
    set +e
    exec 2>&1
    eval "$(vm_fn override_export)"
    # shellcheck source=../modes.sh
    . "$cloud/modes.sh" || exit 1
    override_export RUSTERON_DPDK_ENA_DEVARGS=miss_txc_to=0+control_path_poll_interval=1000
    override_export RUSTERON_DPDK_ENA_DEVARGS=llq_policy=3
    override_export SUB_PARAMS=rcv-wnd=8m+so-rcvbuf=8m
    override_export RUSTERON_DPDK_NIC_STATS_MS=0
    m='dpdk@RUSTERON_DPDK_NIC_STATS_MS=0@RUSTERON_DPDK_ENA_DEVARGS=miss_txc_to=0+control_path_poll_interval=1000'
    mode_check "$m" remote arm64 aws '' cluster && echo "checked: 0"
    mode_check 'dpdk@RUSTERON_DPDK_ENA_DEVARGS=miss_txc_to=0,llq_policy=3' remote arm64 aws '' cluster >/dev/null 2>&1; echo "comma: $?"
)
want="export RUSTERON_DPDK_ENA_DEVARGS='miss_txc_to=0,control_path_poll_interval=1000'
export RUSTERON_DPDK_ENA_DEVARGS='llq_policy=3'
export SUB_PARAMS='rcv-wnd=8m+so-rcvbuf=8m'
export RUSTERON_DPDK_NIC_STATS_MS='0'
checked: 0
comma: 2"
if [[ $devargs != "$want" ]]; then
    fail "vm.sh override_export: ENA device arguments with + not exported comma-separated"
    diff -u <(echo "$want") <(echo "$devargs") >&2 || true
else
    pass "vm.sh: a mode's @RUSTERON_DPDK_ENA_DEVARGS takes its device arguments joined with +, exported comma-separated (mode_check takes it; a comma is still refused); every other override as written"
fi

# The raw floor's modes, kernel-raw and dpdk-raw (the harness's floor client:
# rtt's round trip with no Aeron, over kernel UDP or the DPDK transport's
# bindings alone). modes.sh: taken cross-host, -mono too, with host settings
# and RUSTERON_* ones; refused with +idle, on one host, and with anything
# only Aeron reads (AERON_*, PUB_PARAMS, SUB_PARAMS, RELIABLE, CONDUCTOR),
# saying so; their round trip's client floor, every other mode's rtt; their
# host set-up kernel-busy's and dpdk's (raw_base)
rawmodes=$(
    set +e
    exec 2>&1
    # shellcheck source=../modes.sh
    . "$cloud/modes.sh" || exit 1
    for m in kernel-raw dpdk-raw kernel-raw-mono dpdk-raw-mono dpdk-raw@RUSTERON_DPDK_RX_DESCRIPTORS=1024 kernel-raw@RX_RING=4096@THP=always; do
        mode_check "$m" remote x86_64 aws '' cluster >/dev/null
        echo "$m $? $(mode_client "$m") $(raw_base "$m")"
    done
    for m in dpdk kernel-busy dpdk@AERON_THREADING_MODE=INVOKER; do
        mode_check "$m" remote x86_64 aws '' cluster >/dev/null
        echo "$m $? $(mode_client "$m") [$(raw_base "$m")]"
    done
    for m in dpdk-raw+idle kernel-raw@AERON_THREADING_MODE=INVOKER dpdk-raw@AERON_MTU_LENGTH=4096 \
        kernel-raw@SUB_PARAMS=rcv-wnd=1m kernel-raw@PUB_PARAMS=mtu=1408 kernel-raw@RELIABLE=false dpdk-raw@CONDUCTOR=own; do
        why=$(mode_check "$m" remote x86_64 aws '' cluster 2>&1)
        echo "$m $? $(grep -c 'no Aeron' <<<"$why")"
    done
    why=$(mode_check kernel-raw local x86_64 aws '' cluster 2>&1)
    echo "local kernel-raw $? $(grep -c 'cross-host only' <<<"$why")"
)
want="kernel-raw 0 floor kernel-busy
dpdk-raw 0 floor dpdk
kernel-raw-mono 0 floor kernel-busy
dpdk-raw-mono 0 floor dpdk
dpdk-raw@RUSTERON_DPDK_RX_DESCRIPTORS=1024 0 floor dpdk
kernel-raw@RX_RING=4096@THP=always 0 floor kernel-busy
dpdk 0 rtt []
kernel-busy 0 rtt []
dpdk@AERON_THREADING_MODE=INVOKER 0 rtt []
dpdk-raw+idle 2 1
kernel-raw@AERON_THREADING_MODE=INVOKER 2 1
dpdk-raw@AERON_MTU_LENGTH=4096 2 1
kernel-raw@SUB_PARAMS=rcv-wnd=1m 2 1
kernel-raw@PUB_PARAMS=mtu=1408 2 1
kernel-raw@RELIABLE=false 2 1
dpdk-raw@CONDUCTOR=own 2 1
local kernel-raw 2 1"
if [[ $rawmodes == "$want" ]]; then
    pass "modes.sh: the raw floor's modes (kernel-raw, dpdk-raw, -mono too) taken cross-host with host and RUSTERON_* settings, refused with +idle, on one host or with what only Aeron reads; their client floor, kernel-busy's or dpdk's set-up"
else
    fail "modes.sh: the raw floor's modes"
    diff -u <(echo "$want") <(echo "$rawmodes") >&2 || true
fi

# lib.sh dry with the floor modes beside an Aeron one: a floor mode's rep is
# its round trip alone, by the floor client (pong on b, ping on a, the pong
# ended by its path, "/floor xpong"), its rtt row, histogram, spikes and
# stalls fetched as rtt's, its loss rows; no throughput, load sweep, mix,
# several flows or driver host checks (no driver), each said so once a
# rep; the Aeron mode as before
cat >"$tmp/dry-floor.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
label() { local l=${1#*client }; echo "${l%% *}"; }
on() {
    local host=$1 cmd
    shift
    cmd=$*
    printf '%s %s\n' "$host" "$cmd" >>"$out/calls"
    case $cmd in
    *"vm.sh nics") echo "receiver ens7 ens7 0d $([[ $host == "$A" ]] && echo 10.30.1.5 || echo 10.30.1.7) 24 0000:00:07.0" ;;
    *"vm.sh layout") echo "0 2 1 3 4" ;;
    *"vm.sh loss") echo "kernel-udp-InErrors=0;nic-kernel-drops=0" ;;
    *" floor xping "* | *" purefloor xping "* | *" rtt xping "*) echo "rtt,$(label "$cmd"),udp,20000,21000,25000,30000,40000,75000,20500,1000,0" ;;
    *" tput xsub "*) echo "tput,$(label "$cmd"),udp,1000,1,2,3,0,1,2" ;;
    *"cat bench/load-spikes.csv") printf '# start_realtime_ns=1\n# spikes_total=0\nstream,due_ns,latency_ns,ping\n' ;;
    esac
    return 0
}
reps=1 local_modes= load_sizes=288 load_fractions=50 load_seconds=1 load_ladder= mix='p=s:3001:1000:32' mix_seconds=1
tput_seconds=5 rtt_messages=1000 rtt_warmup=100 rtt_align= tput_flows=2
modes=(kernel-raw dpdk-raw dpdk dpdk-pure)
bench_state isolated
SH
dfl=$tmp/dry-floor
mkdir -p "$dfl"
bash "$tmp/dry-floor.sh" "$cloud" "$dfl" </dev/null 2>"$dfl/stderr" || fail "lib.sh dry run with the floor modes exited $?"
floor_calls() { # <mode>: the clients its rep ran on the VMs (sorted: the pong starts in the background), and the client ended after its ping
    grep "client isolated:$1 " "$dfl/calls" | sort |
        sed -E 's/LAB_CLIENT_DEADLINE=[0-9]+ //; s/LOAD_SPIKE_NS=[0-9]+ //; s/LOOP_STALL_NS=[0-9]+ //; s/RTT_SECONDS=[0-9]+ //'
    echo "then $(awk -v m="client isolated:$1 floor xping " 'index($0, m) {on = 1; next} on && /pkill -f/ {print; exit}' "$dfl/calls")"
}
got=$(floor_calls kernel-raw; echo; grep 'client isolated:dpdk-raw ' "$dfl/calls" | grep -c ' tput \| load ' || true)
want="192.0.2.1 ergon/md-dpdk/benches/cloud/vm.sh client isolated:kernel-raw floor xping 10.30.1.7:20123 10.30.1.5:20124 1000 100 2
192.0.2.2 ergon/md-dpdk/benches/cloud/vm.sh client isolated:kernel-raw floor xpong 10.30.1.7:20123 10.30.1.5:20124 2
then 192.0.2.2 sudo pkill -f '/[f]loor xpong'

0"
if grep -E 'unbound variable|command not found|syntax error' "$dfl/stderr"; then
    fail "lib.sh dry with the floor modes: shell errors"
elif [[ $got != "$want" ]]; then
    fail "lib.sh dry with the floor modes: the clients run"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
elif grep -q ERROR "$dfl/log" || [[ $(grep -c '^1,rtt,isolated:\(kernel\|dpdk\)-raw,udp,20000,' "$dfl/results.csv") != 2 ]]; then
    fail "lib.sh dry with the floor modes: their rtt rows: $(grep ',rtt,' "$dfl/results.csv" | tr '\n' ' ') $(tail -2 "$dfl/log")"
elif [[ ! -s $dfl/spikes-isolated-dpdk-raw-rtt-1.csv ]] || ! grep -q "cat bench/stalls-pong.csv" "$dfl/calls" ||
    [[ $(grep -c '^1,loss,isolated:kernel-raw:rtt,[ab],' "$dfl/results.csv") != 2 ]]; then
    fail "lib.sh dry with the floor modes: the spikes, stalls or loss rows not taken as rtt's"
elif grep -q ',hostcheck,isolated:[a-z]*-raw,' "$dfl/results.csv" || ! grep -q ',hostcheck,isolated:dpdk,' "$dfl/results.csv"; then
    fail "lib.sh dry with the floor modes: driver host checks: $(grep ',hostcheck,' "$dfl/results.csv" | tr '\n' ' ')"
elif [[ $(grep -c 'isolated:kernel-raw: the raw floor (no Aeron): its round trip alone' "$dfl/log") != 1 ]] ||
    ! grep -q ' tput xpub ' <(grep 'client isolated:dpdk ' "$dfl/calls") || ! grep -q ' load xping ' <(grep 'client isolated:dpdk ' "$dfl/calls"); then
    fail "lib.sh dry with the floor modes: the skipped phases not said once, or the Aeron mode's phases gone: $(grep -c raw "$dfl/log")"
else
    pass "lib.sh dry with the floor modes: the floor client's round trip alone (no throughput, load, mix, flows or driver host checks: said so), rtt's files and loss rows; the Aeron mode as before"
fi

# vm.sh client for the floor client: only after vm.sh driver set a floor mode
# up (~/bench/floor: kernel or dpdk), then with the driver's environment
# (driver.env: the DPDK transport's, the kernel's selector absent) and the
# lost-round-trip timeout an unreliable channel's (raw UDP repairs nothing:
# 50 ms, or four of the network's round trips), never HARNESS_DRIVER
floor_client() {
    : >"$sbc/log"
    env -i PATH="$sbc/bin:$PATH" HOME="$sbc/home" FAKE_LOG="$sbc/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" \
        bash "$cloud/vm.sh" client isolated:dpdk-raw floor xping 10.30.1.7:20123 10.30.1.5:20124 10 1 2 >"$sbc/out" 2>&1
    echo "rc $? $(grep '^sudo ' "$sbc/log" | tail -1)"
}
rm -f "$sbc/home/bench/harness_driver" "$sbc/home/bench/sub_channel_params" "$sbc/home/bench/pub_channel_params" "$sbc/home/bench/floor"
unset_floor=$(floor_client)
unset_said=$(cat "$sbc/out")
echo dpdk >"$sbc/home/bench/floor"
echo 70000 >"$sbc/home/bench/base_rtt_us"
floored=$(floor_client)
echo 5 >"$sbc/home/bench/base_rtt_us"
floored_near=$(floor_client)
rm -f "$sbc/home/bench/floor"
if [[ $unset_floor != "rc 2 " || $unset_said != *"vm.sh driver kernel-raw or dpdk-raw"* ]]; then
    fail "vm.sh client: the floor client ran without a floor mode's set-up: $unset_floor $unset_said"
elif [[ $floored != "rc 0 sudo bash -c "*"$sbc/home/bench/driver.env"* || $floored != *" RTT_LOST_TIMEOUT_US=280000 "* ||
    $floored != *"/target/release/floor xping 10.30.1.7:20123 10.30.1.5:20124 10 1 2" || $floored == *HARNESS_DRIVER* ]]; then
    fail "vm.sh client for the floor client: $floored $(tail -2 "$sbc/out")"
elif [[ $floored_near != *" RTT_LOST_TIMEOUT_US=50000 "* ]]; then
    fail "vm.sh client for the floor client, a near network: $floored_near"
else
    pass "vm.sh client: the floor client only after a floor mode's set-up, with the driver's environment, an unreliable channel's lost-round-trip timeout, no HARNESS_DRIVER"
fi

# vm.sh client under precision-time: rtt's ping writes each spike's one-way
# split (RTT_ONE_WAY, ~/bench/one-way.csv, the last run's removed first);
# elsewhere none asked for (two hosts' clocks off the PTP hardware clock)
rm -f "$sbc/home/bench/floor" "$sbc/home/bench/driver.env"
cluster_client=$(client_sb)
echo precision-time >"$sbc/home/bench/placement"
: >"$sbc/home/bench/one-way.csv"
precise_client=$(client_sb)
precise_left=$([[ -e $sbc/home/bench/one-way.csv ]] && echo kept || echo removed)
rm -f "$sbc/home/bench/placement"
if [[ $cluster_client != *" RTT_ONE_WAY= "* || $precise_client != *" RTT_ONE_WAY=$sbc/home/bench/one-way.csv "* || $precise_left != removed ]]; then
    fail "vm.sh client: RTT_ONE_WAY: cluster [$cluster_client] precision-time [$precise_client] last file $precise_left"
else
    pass "vm.sh client: under precision-time the ping splits each spike one way (RTT_ONE_WAY, the last run's file removed first), elsewhere not"
fi

# vm.sh driver for a floor mode: kernel-busy's or dpdk's set-up under
# INVOKER's threading (one thread sends and receives), the floor's wire
# recorded for `client` and `stop`, no media driver started (the INVOKER
# branch's exit, before the driver's start), and `stop` ends a floor client
# and forgets the wire
driver_branch=$(awk '/^driver\)$/ {on = 1} on {print} on && /^    ;;$/ {exit}' "$cloud/vm.sh")
stop_branch=$(awk '/^stop\)$/ {on = 1} on {print} on && /^    ;;$/ {exit}' "$cloud/vm.sh")
before_start=${driver_branch%%exec taskset -c \$HK_CPU*}
if [[ $driver_branch != *'raw=$(raw_base "$mode")'* || $driver_branch != *'threading=INVOKER'* ]]; then
    fail "vm.sh driver: a floor mode's set-up (raw_base, INVOKER's threading)"
elif [[ $before_start != *'echo "${floor_wire}" >"$run/floor"'* || $before_start != *'exit 0'* ]]; then
    fail "vm.sh driver: a floor mode's wire not recorded, or a media driver started"
elif [[ $stop_branch != *'pkill -x floor'* || $stop_branch != *'rm -f "$run/floor"'* ]]; then
    fail "vm.sh stop: a floor client not ended, or its wire kept"
elif ! grep -q -- '--bin floor' "$cloud/vm.sh" || ! grep -q "pgrep -x 'media_driver|rtt|load|tput|floor|" "$cloud/vm.sh"; then
    fail "vm.sh: the floor client not built, or diag-start blind to it"
else
    pass "vm.sh driver: a floor mode set up as kernel-busy or dpdk under INVOKER's threading, its wire recorded, no media driver; stop ends its client; build builds it"
fi

# estimate.py: a floor mode's rep is its round trip alone (the mode's 12 s
# and the round trip's 85 s, and its 64-byte datagrams each way); the Aeron
# mode beside it keeps its flat-out, load and mix phases
est=$(python3 - "$cloud" <<'PY'
import sys
sys.path.insert(0, sys.argv[1])
import estimate as e
args = dict(type_a="c8g.xlarge", type_b="c8g.xlarge", placement="cluster", reps=2, local_modes=[], states=["pinned"],
            load_sizes=[288], load_fractions=[50], load_ladder={}, load_seconds=5, mix=[[(1000, 32)]], mix_seconds=5,
            tput_seconds=5, tput_flows=2, rtt_messages=1000, rtt_warmup=100, no_flatout=False, max_hours=8)
floor = e.estimate(modes=["kernel-raw", "dpdk-raw-mono@RUSTERON_DPDK_RX_DESCRIPTORS=1024"], **args)
aeron = e.estimate(modes=["dpdk"], **args)
fixed = e.SETUP_S + e.TEARDOWN_S + e.STATE_S["pinned"]
print(floor["wall_s"] == fixed + 2 * 2 * (e.MODE_S + e.RTT_S),
      floor["bytes_ab"] == 2 * 2 * 1100 * (e.RTT_SIZE + e.WIRE_OVERHEAD) == floor["bytes_ba"], floor["bound"],
      aeron["wall_s"] > fixed + 2 * (e.MODE_S + e.RTT_S), aeron["bound"])
PY
)
if [[ $est == "True True False True True" ]]; then
    pass "estimate.py: a floor mode's rep is its round trip alone; an Aeron mode keeps its other phases"
else
    fail "estimate.py with the floor modes: $est"
fi

# analyse.py: the floor modes named for what they are, after the Aeron modes
# they floor, and a table of what Aeron adds over the floor in the same
# state (each Aeron mode beside its transport's floor: kernel's beside
# kernel-raw, DPDK's beside dpdk-raw, -mono beside -mono), none without a
# floor; spikes.py counts the floor client among the busy threads
cat >"$tmp/floor-results.csv" <<'CSV'
1,rtt,isolated:kernel-busy,udp,30000,31000,35000,40000,50000,79000,30500,1000,0
2,rtt,isolated:kernel-busy,udp,32000,33000,36000,41000,52000,75000,32500,1000,0
1,rtt,isolated:kernel-raw,udp,20000,21000,25000,30000,40000,70000,20500,1000,0
2,rtt,isolated:kernel-raw,udp,22000,23000,26000,31000,42000,74000,22500,1000,0
1,rtt,isolated:dpdk,udp,25000,26000,30000,35000,45000,72000,25500,1000,0
1,rtt,isolated:dpdk-raw,udp,15000,16000,20000,25000,35000,71000,15500,1000,0
1,rtt,pinned:dpdk,udp,25000,26000,30000,35000,45000,72000,25500,1000,0
CSV
floor_md=$(python3 "$cloud/analyse.py" "$tmp/floor-results.csv" 2>&1 || true)
above=$(section "## UDP round trip above the raw floor (µs)" <<<"$floor_md" | tr -s ' ')
busy=$(python3 -c 'import sys; sys.path.insert(0, sys.argv[1]); import spikes; print(bool(spikes.BUSY.match("floor")))' "$cloud")
if [[ $floor_md != *"| isolated, kernel UDP, no Aeron (raw floor) "* || $floor_md != *"| isolated, DPDK packet path, no Aeron (raw floor) "* ]]; then
    fail "analyse.py: the floor modes' names: $(grep -m3 'raw' <<<"$floor_md")"
elif [[ $(grep -c '^| isolated, ' <<<"$above") != 2 || $above != *"| isolated, kernel UDP, busy polling | kernel UDP, no Aeron (raw floor) | 2 | 10.00 (10.00–10.00) "* ||
    $above != *"| isolated, DPDK | DPDK packet path, no Aeron (raw floor) | 1 | 10.00 (10.00–10.00) "* || $above == *pinned* ]]; then
    fail "analyse.py: the table above the raw floor"
    echo "$above" >&2
elif [[ $busy != True ]]; then
    fail "spikes.py: the floor client is not among the busy threads"
else
    pass "analyse.py: the floor modes named, and what each Aeron mode adds over its transport's floor in the same state (none without one); spikes.py counts the floor client busy"
fi

# The pure floor, dpdk-pure: DPDK alone (purefloor.c: rte_eth_rx_burst and
# rte_eth_tx_burst on the receiver port, a 5-tuple-swapping reflector at the
# pong), none of ergon-md-dpdk or Aeron. modes.sh: a raw floor mode on dpdk's
# set-up, its client purefloor; cross-host on AWS only; refused with -mono
# (the CPU's counter is its only clock), +idle, anything only Aeron reads,
# and any RUSTERON_* purefloor does not read; the transport's ring and pool
# limits still apply (they are the NIC's)
puremodes=$(
    set +e
    exec 2>&1
    # shellcheck source=../modes.sh
    . "$cloud/modes.sh" || exit 1
    for m in dpdk-pure dpdk-pure@RUSTERON_DPDK_RX_DESCRIPTORS=1024@RUSTERON_DPDK_BURST_SIZE=16 dpdk-pure@THP=always@JUMBO=1; do
        mode_check "$m" remote arm64 aws '' cluster >/dev/null
        echo "$m $? $(mode_client "$m") $(raw_base "$m") $(mode_raw "$m" && echo raw)"
    done
    for m in dpdk-pure-mono:monotonic dpdk-pure+idle:'no Aeron' dpdk-pure@AERON_MTU_LENGTH=4096:'no Aeron' \
        dpdk-pure@RUSTERON_DPDK_LOCAL_IO=spin:'not one purefloor reads' dpdk-pure@RUSTERON_DPDK_RX_DESCRIPTORS=16384:'8192 at most'; do
        why=$(mode_check "${m%%:*}" remote arm64 aws '' cluster 2>&1)
        echo "${m%%:*} $? $(grep -c -- "${m#*:}" <<<"$why")"
    done
    why=$(mode_check dpdk-pure local arm64 aws '' cluster 2>&1)
    echo "local $? $(grep -c 'cross-host only' <<<"$why")"
    why=$(mode_check dpdk-pure remote x86_64 azure '' cluster 2>&1)
    echo "azure $? $(grep -c 'AWS only' <<<"$why")"
    echo "dpdk-raw: $(mode_client dpdk-raw)"
)
want="dpdk-pure 0 purefloor dpdk raw
dpdk-pure@RUSTERON_DPDK_RX_DESCRIPTORS=1024@RUSTERON_DPDK_BURST_SIZE=16 0 purefloor dpdk raw
dpdk-pure@THP=always@JUMBO=1 0 purefloor dpdk raw
dpdk-pure-mono 2 1
dpdk-pure+idle 2 1
dpdk-pure@AERON_MTU_LENGTH=4096 2 1
dpdk-pure@RUSTERON_DPDK_LOCAL_IO=spin 2 1
dpdk-pure@RUSTERON_DPDK_RX_DESCRIPTORS=16384 2 1
local 2 1
azure 2 1
dpdk-raw: floor"
if [[ $puremodes == "$want" ]]; then
    pass "modes.sh: the pure floor, dpdk-pure: a raw floor mode on dpdk's set-up, its client purefloor; refused with -mono, +idle, Aeron's settings, a RUSTERON_* purefloor does not read, past the NIC's rings, on one host or off AWS"
else
    fail "modes.sh: the pure floor's mode"
    diff -u <(echo "$want") <(echo "$puremodes") >&2 || true
fi

# vm.sh build compiles purefloor on the host, as gapspin, against the DPDK
# the media driver links (pkg-config libdpdk: Debian's, or precision-time's
# 25.11), with -Wall -Wextra -Werror and the architecture's CFLAGS; one that
# does not compile leaves why in ~/bench/purefloor_unavailable and no
# purefloor, and the build goes on (the other modes need none)
# shellcheck disable=SC2016
{
    printf '#!/bin/sh\necho "gcc $*" >>"$FAKE_LOG"\n[ -z "$FAKE_GCC_FAIL" ] || { echo "purefloor.c:1:1: error: boom" >&2; exit 1; }\nwhile [ "$1" != -o ]; do shift; done\nprintf "#!/bin/sh\\n" >"$2"; chmod +x "$2"\n' >"$sbb/bin/gcc"
    printf '#!/bin/sh\ncase "$*" in *--cflags*) echo "-I/usr/include/dpdk -include rte_config.h" ;; *--libs*) echo "-lrte_ethdev -lrte_eal" ;; esac\n' >"$sbb/bin/pkg-config"
}
chmod +x "$sbb/bin/gcc" "$sbb/bin/pkg-config"
pure_build() { # <arch> [FAKE_GCC_FAIL]: the gcc line, then whether purefloor and its marker are there
    rm -f "$sbb/home/bench/purefloor"
    : >"$sbb/log"
    env -i PATH="$sbb/bin:$PATH" HOME="$sbb/home" FAKE_LOG="$sbb/log" FAKE_ARCH="$1" FAKE_NO_DPDK= FAKE_GCC_FAIL="${2:-}" \
        LAB_SYS_ROOT="$sys/$([[ $1 == aarch64 ]] && echo graviton || echo aws-x86)" \
        bash "$cloud/vm.sh" build >"$sbb/out" 2>&1 || echo "exit $?"
    grep '^gcc .*purefloor' "$sbb/log"
    echo "purefloor $([[ -x $sbb/home/bench/purefloor ]] && echo built || echo none), unavailable: $(cat "$sbb/home/bench/purefloor_unavailable" 2>/dev/null)"
}
echo "an earlier build's" >"$sbb/home/bench/purefloor_unavailable"
got="x86:
$(pure_build x86_64)
arm:
$(pure_build aarch64)
fails:
$(pure_build x86_64 1)"
want="x86:
gcc -O2 -Wall -Wextra -Werror -I/usr/include/dpdk -include rte_config.h -o $sbb/home/bench/purefloor $cloud/purefloor.c -lrte_ethdev -lrte_eal
purefloor built, unavailable: 
arm:
gcc -O2 -Wall -Wextra -Werror -mcpu=native -I/usr/include/dpdk -include rte_config.h -o $sbb/home/bench/purefloor $cloud/purefloor.c -lrte_ethdev -lrte_eal
purefloor built, unavailable: 
fails:
gcc -O2 -Wall -Wextra -Werror -I/usr/include/dpdk -include rte_config.h -o $sbb/home/bench/purefloor $cloud/purefloor.c -lrte_ethdev -lrte_eal
purefloor none, unavailable: purefloor.c did not compile: purefloor.c:1:1: error: boom"
if [[ $got != "$want" ]]; then
    fail "vm.sh build: purefloor"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
else
    pass "vm.sh build: purefloor compiled on the host against pkg-config's libdpdk with -Wall -Wextra -Werror and the architecture's CFLAGS; a failure said in purefloor_unavailable, the build going on"
fi

# vm.sh ready: a dpdk-pure mode where purefloor did not build is refused
# before any state, saying why; with purefloor there, taken
rm -f "$sbr/home/bench/purefloor"
echo "purefloor.c did not compile: boom" >"$sbr/home/bench/purefloor_unavailable"
got="none: $(ready_sb aws-x86 7.2.6+deb13-cloud-amd64 dpdk dpdk-pure@THP=always)"
rm -f "$sbr/home/bench/purefloor_unavailable"
printf '#!/bin/sh\n' >"$sbr/home/bench/purefloor"
chmod +x "$sbr/home/bench/purefloor"
got+="
built: $(ready_sb aws-x86 7.2.6+deb13-cloud-amd64 dpdk dpdk-pure)"
rm -f "$sbr/home/bench/purefloor"
want="none: 2 mode dpdk-pure@THP=always: purefloor did not build here (vm.sh build): purefloor.c did not compile: boom
built: 0"
if [[ $got != "$want" ]]; then
    fail "vm.sh ready: dpdk-pure without purefloor"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
else
    pass "vm.sh ready: a dpdk-pure mode refused where purefloor did not build, saying why"
fi

# vm.sh client purefloor: only on the pure floor's wire (vm.sh driver
# dpdk-pure: ~/bench/floor is pure), the build's ~/bench/purefloor with the
# driver's environment and an unreliable channel's lost-round-trip timeout,
# never HARNESS_DRIVER; the floor client never on that wire, nor purefloor
# on dpdk-raw's
pure_client() { # <prog>
    : >"$sbc/log"
    env -i PATH="$sbc/bin:$PATH" HOME="$sbc/home" FAKE_LOG="$sbc/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" \
        bash "$cloud/vm.sh" client isolated:dpdk-pure "$1" xping 10.30.1.7:20123 10.30.1.5:20124 10 1 2 >"$sbc/out" 2>&1
    echo "rc $? $(grep '^sudo ' "$sbc/log" | tail -1)"
}
echo 70000 >"$sbc/home/bench/base_rtt_us"
printf '#!/bin/sh\n' >"$sbc/home/bench/purefloor"
chmod +x "$sbc/home/bench/purefloor"
rm -f "$sbc/home/bench/floor"
p_none=$(pure_client purefloor)
echo dpdk >"$sbc/home/bench/floor"
p_raw=$(pure_client purefloor)
p_raw_said=$(cat "$sbc/out")
echo pure >"$sbc/home/bench/floor"
p_pure=$(pure_client purefloor)
f_pure=$(pure_client floor)
f_pure_said=$(cat "$sbc/out")
rm -f "$sbc/home/bench/floor" "$sbc/home/bench/purefloor"
if [[ $p_none != "rc 2 " || $p_raw != "rc 2 " || $p_raw_said != *"vm.sh driver dpdk-pure"* ]]; then
    fail "vm.sh client purefloor off the pure floor's wire: [$p_none] [$p_raw] $p_raw_said"
elif [[ $f_pure != "rc 2 " || $f_pure_said != *"vm.sh driver kernel-raw or dpdk-raw"* ]]; then
    fail "vm.sh client floor on the pure floor's wire: $f_pure $f_pure_said"
elif [[ $p_pure != "rc 0 sudo bash -c "*"$sbc/home/bench/driver.env"* || $p_pure != *" RTT_LOST_TIMEOUT_US=280000 "* ||
    $p_pure != *" $sbc/home/bench/purefloor xping 10.30.1.7:20123 10.30.1.5:20124 10 1 2" || $p_pure == *HARNESS_DRIVER* ]]; then
    fail "vm.sh client purefloor: $p_pure $(tail -2 "$sbc/out")"
else
    pass "vm.sh client purefloor: only on the pure floor's wire, the build's purefloor with the driver's environment and an unreliable channel's timeout; the floor client never there"
fi

# vm.sh driver dpdk-pure: dpdk's set-up, the wire recorded as pure, refused
# without purefloor; stop ends a purefloor client; diag-start sees it busy
if [[ $before_start != *'floor_wire=pure'* || $before_start != *'purefloor did not build here'* ]]; then
    fail "vm.sh driver: the pure floor's wire, or its refusal without purefloor"
elif [[ $stop_branch != *'pkill -x purefloor'* ]] || ! grep -q "pgrep -x 'media_driver|rtt|load|tput|floor|purefloor'" "$cloud/vm.sh"; then
    fail "vm.sh stop or diag-start: blind to purefloor"
else
    pass "vm.sh driver dpdk-pure: dpdk's set-up, its wire pure, refused without purefloor; stop ends purefloor, diag-start sees it"
fi

# lib.sh dry (the floor modes' run above, dpdk-pure among them): its rep is
# purefloor's round trip alone, pong on b, ping on a, the pong ended by its
# path ("/purefloor xpong"), its rtt row; no throughput or load
got=$(grep "client isolated:dpdk-pure " "$dfl/calls" | sort |
    sed -E 's/LAB_CLIENT_DEADLINE=[0-9]+ //; s/LOAD_SPIKE_NS=[0-9]+ //; s/LOOP_STALL_NS=[0-9]+ //; s/RTT_SECONDS=[0-9]+ //'
    echo "then $(awk 'index($0, "client isolated:dpdk-pure purefloor xping ") {on = 1; next} on && /pkill -f/ {print; exit}' "$dfl/calls")"
    grep 'client isolated:dpdk-pure ' "$dfl/calls" | grep -c ' tput \| load ' || true
    grep -c '^1,rtt,isolated:dpdk-pure,udp,20000,' "$dfl/results.csv")
want="192.0.2.1 ergon/md-dpdk/benches/cloud/vm.sh client isolated:dpdk-pure purefloor xping 10.30.1.7:20123 10.30.1.5:20124 1000 100 2
192.0.2.2 ergon/md-dpdk/benches/cloud/vm.sh client isolated:dpdk-pure purefloor xpong 10.30.1.7:20123 10.30.1.5:20124 2
then 192.0.2.2 sudo pkill -f '/[p]urefloor xpong'
0
1"
if [[ $got != "$want" ]]; then
    fail "lib.sh dry: the pure floor's rep"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
else
    pass "lib.sh dry: the pure floor's rep is purefloor's round trip alone (pong on b, ping on a, ended by its path), its rtt row"
fi

# analyse.py: dpdk-pure named for what it is, after dpdk-raw, and dpdk-raw
# beside it in the table above the floor (what ergon-md-dpdk's packet path
# adds over DPDK alone), the Aeron DPDK mode still beside dpdk-raw; a mode
# with overrides set against the floor with the same ones where that ran
# (precision-time's timestamps-off baselines), else the bare floor;
# estimate.py: its rep the round trip alone; spikes.py: purefloor busy
cat >"$tmp/pure-results.csv" <<'CSV'
1,rtt,isolated:dpdk,udp,25000,26000,30000,35000,45000,72000,25500,1000,0
1,rtt,isolated:dpdk-raw,udp,15000,16000,20000,25000,35000,71000,15500,1000,0
1,rtt,isolated:dpdk-pure,udp,12000,13000,16000,20000,30000,85000,12500,1000,0
1,rtt,isolated:dpdk-raw@RUSTERON_DPDK_RX_HW_TIMESTAMP=0,udp,14000,15000,19000,24000,34000,70000,14500,1000,0
1,rtt,isolated:dpdk-pure@RUSTERON_DPDK_RX_HW_TIMESTAMP=0,udp,12500,13000,17000,21000,31000,90000,12600,1000,0
1,rtt,isolated:dpdk-raw@RUSTERON_DPDK_BURST_SIZE=16,udp,16000,17000,21000,26000,36000,72000,16500,1000,0
CSV
pure_md=$(python3 "$cloud/analyse.py" "$tmp/pure-results.csv" 2>&1 || true)
above=$(section "## UDP round trip above the raw floor (µs)" <<<"$pure_md" | tr -s ' ')
rtt_order=$(grep -o '^| isolated, [^|]*' <<<"$pure_md" | head -3 | tr -s ' ' | tr '\n' ';')
est=$(python3 - "$cloud" <<'PY'
import sys
sys.path.insert(0, sys.argv[1])
import estimate as e
args = dict(type_a="c8g.xlarge", type_b="c8g.xlarge", placement="cluster", reps=2, local_modes=[], states=["pinned"],
            load_sizes=[288], load_fractions=[50], load_ladder={}, load_seconds=5, mix=[[(1000, 32)]], mix_seconds=5,
            tput_seconds=5, tput_flows=2, rtt_messages=1000, rtt_warmup=100, no_flatout=False, max_hours=8)
pure = e.estimate(modes=["dpdk-pure@RUSTERON_DPDK_RX_DESCRIPTORS=1024"], **args)
fixed = e.SETUP_S + e.TEARDOWN_S + e.STATE_S["pinned"]
print(pure["wall_s"] == fixed + 2 * (e.MODE_S + e.RTT_S), pure["bound"])
PY
)
busy=$(python3 -c 'import sys; sys.path.insert(0, sys.argv[1]); import spikes; print(bool(spikes.BUSY.match("purefloor")))' "$cloud")
if [[ $pure_md != *"| isolated, DPDK alone, no ergon-md-dpdk or Aeron (pure floor) "* ]]; then
    fail "analyse.py: the pure floor's name: $(grep -m3 'pure' <<<"$pure_md")"
elif [[ $(grep -c '^| isolated, ' <<<"$above") != 4 ||
    $above != *"| isolated, dpdk-raw@RUSTERON_DPDK_RX_HW_TIMESTAMP=0 | dpdk-pure@RUSTERON_DPDK_RX_HW_TIMESTAMP=0 | 1 | 1.50 (1.50–1.50) | 2.00 (2.00–2.00) | 3.00 (3.00–3.00) | -20.00 (-20.00–-20.00) |"* ||
    $above != *"| isolated, dpdk-raw@RUSTERON_DPDK_BURST_SIZE=16 | DPDK alone, no ergon-md-dpdk or Aeron (pure floor) | 1 | 4.00 (4.00–4.00) | 5.00 (5.00–5.00) | 6.00 (6.00–6.00) | -13.00 (-13.00–-13.00) |"* ||
    $above != *"| isolated, DPDK | DPDK packet path, no Aeron (raw floor) | 1 | 10.00 (10.00–10.00) "* ||
    $above != *"| isolated, DPDK packet path, no Aeron (raw floor) | DPDK alone, no ergon-md-dpdk or Aeron (pure floor) | 1 | 3.00 (3.00–3.00) | 4.00 (4.00–4.00) | 5.00 (5.00–5.00) | -14.00 (-14.00–-14.00) |"* ]]; then
    fail "analyse.py: dpdk-raw above the pure floor"
    echo "$above" >&2
elif [[ $rtt_order != *"DPDK packet path, no Aeron (raw floor) ;| isolated, DPDK alone, no ergon-md-dpdk or Aeron (pure floor) "* ]]; then
    fail "analyse.py: the pure floor not after dpdk-raw: $rtt_order"
elif [[ $est != "True False" || $busy != True ]]; then
    fail "estimate.py or spikes.py with the pure floor: $est $busy"
else
    pass "analyse.py: the pure floor named, after dpdk-raw, and dpdk-raw above it (ergon-md-dpdk's packet path over DPDK alone); estimate.py: its round trip alone; spikes.py: purefloor busy"
fi

# spikes.py: each rep's largest round trips split one way (rtt's RTT_ONE_WAY,
# precision-time: oneway-<state>-<mode>-<rep>.csv): out (sent to the pong's
# NIC), back (the pong's NIC to the ping's) and the ping's host (its NIC to
# the echo's arrival), largest first; printed before the diagnostics are
# read, so a -nodiag state has it too
ow=$tmp/oneway/pinned
mkdir -p "$ow"
printf '# start_realtime_ns=1000000000\nping,sent_realtime_ns,pong_rx_realtime_ns,ping_rx_realtime_ns,echoed_realtime_ns\n7,2000000000,2000100000,2000120000,2000130000\n8,3000000000,3000010000,3000070000,3000075000\n' \
    >"$ow/oneway-pinned-dpdk@SUB_PARAMS=media-rcv-ts-offset=reserved-1.csv"
ow_out=$(python3 "$cloud/spikes.py" "$ow" 2>&1 || true)
ow_table=$(section "## One-way split of the largest round trips, pinned" <<<"$ow_out")
if [[ $ow_table != *"| pinned-dpdk@SUB_PARAMS=media-rcv-ts-offset=reserved | 1 | 7 | 130.0 | 100.0 | 20.0 | 10.0 |"* ||
    $ow_table != *"| pinned-dpdk@SUB_PARAMS=media-rcv-ts-offset=reserved | 1 | 8 | 75.0 | 10.0 | 60.0 | 5.0 |"* ||
    ${ow_table%%| 1 | 8 |*} != *"| 1 | 7 |"* ]]; then
    fail "spikes.py: the one-way split"
    echo "$ow_out" >&2
else
    pass "spikes.py: each rep's largest round trips split one way (out, back, the ping's host), largest first, without diagnostics too"
fi

# The same-host hairpin (@HAIRPIN=1: the raw floor's round trip between host
# a's own two data NICs, never between hosts) and sparse round trips
# (@RTT_GAP_US=<us>: one ping every that many microseconds). modes.sh: HAIRPIN
# taken on the raw floor's modes only (the floor client runs both ends), AWS,
# cross-host lists only, once; RTT_GAP_US 1 to 1000000 us, once, not on
# localhost; each refusal says why. rtt_plan: a paced rep's seconds (20 to 60:
# 10,000 round trips where they fit), its round trips and its warm-up (about
# 2 s of pings); an unpaced one's as before. hairpin_types_check: host a
# needs four cores (housekeeping, ping, the sender's, the pong's)
hsmodes=$(
    set +e
    exec 2>&1
    # shellcheck source=../modes.sh
    . "$cloud/modes.sh" || exit 1
    for m in dpdk-raw@HAIRPIN=1 kernel-raw@HAIRPIN=1 dpdk-raw-mono@HAIRPIN=1 dpdk-raw@HAIRPIN=1@RTT_GAP_US=50000 \
        dpdk@RTT_GAP_US=1000 kernel-raw@RTT_GAP_US=1000000 dpdk-raw; do
        mode_check "$m" remote arm64 aws '' cluster >/dev/null
        echo "$m $? $(mode_hairpin "$m" && echo hairpin || echo cross)"
    done
    for c in 'dpdk@HAIRPIN=1|the raw floor' 'kernel-busy@HAIRPIN=1|the raw floor' 'dpdk-raw@HAIRPIN=2|HAIRPIN=1 or nothing' \
        'dpdk-raw@HAIRPIN=1@HAIRPIN=1|one HAIRPIN at most' 'dpdk@RTT_GAP_US=0|1 to 1000000' 'dpdk@RTT_GAP_US=1000001|1 to 1000000' \
        'dpdk@RTT_GAP_US=5ms|1 to 1000000' 'dpdk@RTT_GAP_US=1000@RTT_GAP_US=2000|one RTT_GAP_US at most' \
        'dpdk-pure@HAIRPIN=1|no hairpin client' 'dpdk-pure@RTT_GAP_US=1000|does not pace'; do
        why=$(mode_check "${c%|*}" remote arm64 aws '' cluster 2>&1)
        echo "${c%|*} $? $(grep -cF -- "${c#*|}" <<<"$why")"
    done
    why=$(mode_check dpdk-raw@HAIRPIN=1 remote x86_64 azure '' 2>&1)
    echo "azure $? $(grep -c 'AWS only' <<<"$why")"
    why=$(mode_check kernel@RTT_GAP_US=1000 local arm64 aws '' cluster 2>&1)
    echo "local $? $(grep -c 'cross-host' <<<"$why")"
    for m in dpdk dpdk@RTT_GAP_US=1000 dpdk@RTT_GAP_US=50000 kernel-raw@RTT_GAP_US=200000 dpdk-raw@HAIRPIN=1@RTT_GAP_US=1000000 dpdk@RTT_GAP_US=100; do
        echo "plan $m $(rtt_plan "$m" 300000 50000)"
    done
    echo "plan few $(rtt_plan dpdk@RTT_GAP_US=50000 500 10)"
    for t in c9g.2xlarge c9g.xlarge c9g.large c8g.medium; do
        hairpin_types_check dpdk-raw@HAIRPIN=1 aws "$t" >/dev/null 2>&1
        echo "type $t $?"
    done
    why=$(hairpin_types_check dpdk-raw@HAIRPIN=1 aws c9g.large 2>&1)
    echo "said $(grep -c 'four' <<<"$why")"
    # host b sets the mode up too (vm.sh driver, ready): its type checked as well
    hairpin_types_check dpdk-raw@HAIRPIN=1 aws c9g.xlarge c9g.large >/dev/null 2>&1
    echo "type b $?"
    hairpin_types_check dpdk-raw aws c9g.large
    echo "cross $?"
)
want="dpdk-raw@HAIRPIN=1 0 hairpin
kernel-raw@HAIRPIN=1 0 hairpin
dpdk-raw-mono@HAIRPIN=1 0 hairpin
dpdk-raw@HAIRPIN=1@RTT_GAP_US=50000 0 hairpin
dpdk@RTT_GAP_US=1000 0 cross
kernel-raw@RTT_GAP_US=1000000 0 cross
dpdk-raw 0 cross
dpdk@HAIRPIN=1 2 1
kernel-busy@HAIRPIN=1 2 1
dpdk-raw@HAIRPIN=2 2 1
dpdk-raw@HAIRPIN=1@HAIRPIN=1 2 1
dpdk@RTT_GAP_US=0 2 1
dpdk@RTT_GAP_US=1000001 2 1
dpdk@RTT_GAP_US=5ms 2 1
dpdk@RTT_GAP_US=1000@RTT_GAP_US=2000 2 1
dpdk-pure@HAIRPIN=1 2 1
dpdk-pure@RTT_GAP_US=1000 2 1
azure 2 1
local 2 1
plan dpdk 20 300000 50000
plan dpdk@RTT_GAP_US=1000 20 20000 2000
plan dpdk@RTT_GAP_US=50000 60 1200 40
plan kernel-raw@RTT_GAP_US=200000 60 300 10
plan dpdk-raw@HAIRPIN=1@RTT_GAP_US=1000000 60 60 2
plan dpdk@RTT_GAP_US=100 20 200000 20000
plan few 60 500 10
type c9g.2xlarge 0
type c9g.xlarge 0
type c9g.large 2
type c8g.medium 2
said 1
type b 2
cross 0"
if [[ $hsmodes == "$want" ]]; then
    pass "modes.sh: @HAIRPIN=1 on the raw floor's modes only (AWS, cross-host lists, once) and @RTT_GAP_US=1 to 1000000 (once, not localhost), each refusal said; a paced rep's seconds, round trips and warm-up; four cores on host a for a hairpin"
else
    fail "modes.sh: the hairpin and sparse round trips"
    diff -u <(echo "$want") <(echo "$hsmodes") >&2 || true
fi

# The raw floor's keep-alives (@KEEPALIVE_US) and cold-cache flush
# (@FLUSH_KB): modes.sh takes them on kernel-raw and dpdk-raw only, each with
# @RTT_GAP_US, once; a keep-alive at most half the gap and over 50 us under
# it, a flush of at most 1 GiB with a gap of 10 ms or more; each refusal says
# why
kfmodes=$(
    set +e
    exec 2>&1
    # shellcheck source=../modes.sh
    . "$cloud/modes.sh" || exit 1
    for m in dpdk-raw@RTT_GAP_US=1000000@KEEPALIVE_US=100 kernel-raw@RTT_GAP_US=10000@KEEPALIVE_US=100 \
        dpdk-raw@RTT_GAP_US=10000@FLUSH_KB=65536 dpdk-raw@HAIRPIN=1@RTT_GAP_US=1000000@KEEPALIVE_US=100; do
        mode_check "$m" remote arm64 aws '' cluster >/dev/null
        echo "$m $?"
    done
    for c in 'dpdk@RTT_GAP_US=1000000@KEEPALIVE_US=100|the raw floor' 'dpdk-raw@KEEPALIVE_US=100|needs @RTT_GAP_US' \
        'dpdk-raw@RTT_GAP_US=150@KEEPALIVE_US=100|at most half the gap' 'dpdk-raw@RTT_GAP_US=1000@FLUSH_KB=1024|10 ms or more' \
        'dpdk-raw@RTT_GAP_US=10000@FLUSH_KB=2000000|at most 1048576' 'dpdk-raw@RTT_GAP_US=10000@FLUSH_KB=0|a whole number from 1' \
        'dpdk-pure@RTT_GAP_US=10000@FLUSH_KB=1024|does not pace' \
        'dpdk-raw@RTT_GAP_US=10000@KEEPALIVE_US=100@KEEPALIVE_US=200|one KEEPALIVE_US at most'; do
        why=$(mode_check "${c%|*}" remote arm64 aws '' cluster 2>&1)
        echo "${c%|*} $? $(grep -cF -- "${c#*|}" <<<"$why")"
    done
)
want="dpdk-raw@RTT_GAP_US=1000000@KEEPALIVE_US=100 0
kernel-raw@RTT_GAP_US=10000@KEEPALIVE_US=100 0
dpdk-raw@RTT_GAP_US=10000@FLUSH_KB=65536 0
dpdk-raw@HAIRPIN=1@RTT_GAP_US=1000000@KEEPALIVE_US=100 0
dpdk@RTT_GAP_US=1000000@KEEPALIVE_US=100 2 1
dpdk-raw@KEEPALIVE_US=100 2 1
dpdk-raw@RTT_GAP_US=150@KEEPALIVE_US=100 2 1
dpdk-raw@RTT_GAP_US=1000@FLUSH_KB=1024 2 1
dpdk-raw@RTT_GAP_US=10000@FLUSH_KB=2000000 2 1
dpdk-raw@RTT_GAP_US=10000@FLUSH_KB=0 2 1
dpdk-pure@RTT_GAP_US=10000@FLUSH_KB=1024 2 1
dpdk-raw@RTT_GAP_US=10000@KEEPALIVE_US=100@KEEPALIVE_US=200 2 1"
if [[ $kfmodes == "$want" ]]; then
    pass "modes.sh: @KEEPALIVE_US and @FLUSH_KB on the raw floor's paced modes only, within the gap, once, each refusal said"
else
    fail "modes.sh: keep-alives and the cache flush"
    diff -u <(echo "$want") <(echo "$kfmodes") >&2 || true
fi

# lib.sh dry with a hairpin and paced modes: a hairpin rep is one floor
# client on VM a (floor xhairpin: the pong listens on a's receiver NIC, the
# ping on a's sender NIC; ping on the app CPU, pong on the receiver's), no
# client on b, both loop-stall files from a; a paced mode's clients get
# RTT_GAP_US, RTT_SECONDS from rtt_plan (60 s at 50 ms), its round trips and
# warm-up, and deadlines stretched to fit; an unpaced mode as before
cat >"$tmp/dry-hs.sh" <<'SH'
set -uo pipefail
cloud=$1 out=$2
known=$out/known_hosts user=admin A=192.0.2.1 B=192.0.2.2
log() { echo "$*" >>"$out/log"; }
die() { log "ERROR: $*"; exit 1; }
. "$cloud/lib.sh"
sleep() { :; }
label() { local l=${1#*client }; echo "${l%% *}"; }
on() {
    local host=$1 cmd
    shift
    cmd=$*
    printf '%s %s\n' "$host" "$cmd" >>"$out/calls"
    case $cmd in
    *"vm.sh nics")
        if [[ $host == "$A" ]]; then
            printf 'sender ens6 ens6 0c 10.30.1.4 24 0000:00:06.0\nreceiver ens7 ens7 0d 10.30.1.5 24 0000:00:07.0\n'
        else
            printf 'sender ens6 ens6 0e 10.30.1.6 24 0000:00:06.0\nreceiver ens7 ens7 0f 10.30.1.7 24 0000:00:07.0\n'
        fi
        ;;
    *"vm.sh layout") echo "0 2 1 3 4" ;;
    *"vm.sh loss") echo "kernel-udp-InErrors=0;nic-kernel-drops=0" ;;
    *" floor xping "* | *" floor xhairpin "* | *" rtt xping "*) echo "rtt,$(label "$cmd"),udp,20000,21000,25000,30000,40000,75000,20500,1000,0" ;;
    *" tput xsub "*) echo "tput,$(label "$cmd"),udp,1000,1,2,3,0,1,2" ;;
    *"cat bench/load-spikes.csv") printf '# start_realtime_ns=1\n# spikes_total=0\nstream,due_ns,latency_ns,ping\n' ;;
    *"cat bench/stalls-pong.csv") printf '# start_realtime_ns=1\nend_ns,gap_ns\n' ;;
    esac
    return 0
}
reps=1 local_modes= load_sizes= load_fractions= load_seconds=1 load_ladder= mix= mix_seconds=1
tput_seconds=5 rtt_messages=1000 rtt_warmup=100 rtt_align= tput_flows=0 no_flatout=1
modes=(dpdk-raw@HAIRPIN=1 kernel-raw@HAIRPIN=1@RTT_GAP_US=50000 dpdk@RTT_GAP_US=1000)
bench_state isolated
SH
dhs=$tmp/dry-hs
mkdir -p "$dhs"
bash "$tmp/dry-hs.sh" "$cloud" "$dhs" </dev/null 2>"$dhs/stderr" || fail "lib.sh dry run with the hairpin and paced modes exited $?"
hs_calls() { # <mode>: the clients its rep ran (sorted)
    grep "client isolated:$1 " "$dhs/calls" | sort | sed -E 's/LOAD_SPIKE_NS=[0-9]+ //; s/LOOP_STALL_NS=[0-9]+ //'
}
got=$(hs_calls dpdk-raw@HAIRPIN=1; hs_calls kernel-raw@HAIRPIN=1@RTT_GAP_US=50000; hs_calls dpdk@RTT_GAP_US=1000)
want="192.0.2.1 RTT_SECONDS=20 LAB_CLIENT_DEADLINE=112 ergon/md-dpdk/benches/cloud/vm.sh client isolated:dpdk-raw@HAIRPIN=1 floor xhairpin 10.30.1.5:20123 10.30.1.4:20124 1000 100 2 3
192.0.2.1 RTT_GAP_US=50000 RTT_SECONDS=60 LAB_CLIENT_DEADLINE=212 ergon/md-dpdk/benches/cloud/vm.sh client isolated:kernel-raw@HAIRPIN=1@RTT_GAP_US=50000 floor xhairpin 10.30.1.5:20123 10.30.1.4:20124 1000 40 2 3
192.0.2.1 RTT_GAP_US=1000 RTT_SECONDS=20 LAB_CLIENT_DEADLINE=112 ergon/md-dpdk/benches/cloud/vm.sh client isolated:dpdk@RTT_GAP_US=1000 rtt xping 10.30.1.7:20123 10.30.1.5:20124 1000 100 2
192.0.2.2 LAB_CLIENT_DEADLINE=142 ergon/md-dpdk/benches/cloud/vm.sh client isolated:dpdk@RTT_GAP_US=1000 rtt xpong 10.30.1.7:20123 10.30.1.5:20124 2"
hairpin_after=$(awk '/client isolated:dpdk-raw@HAIRPIN=1 floor xhairpin /{on = 1; next} on && /cat bench\/stalls-pong.csv/ {print $1; exit}' "$dhs/calls")
if grep -E 'unbound variable|command not found|syntax error' "$dhs/stderr"; then
    fail "lib.sh dry with the hairpin and paced modes: shell errors"
elif [[ $got != "$want" ]]; then
    fail "lib.sh dry with the hairpin and paced modes: the clients run"
    diff -u <(echo "$want") <(echo "$got") >&2 || true
elif grep -q ERROR "$dhs/log" || [[ $(grep -c '^1,rtt,isolated:' "$dhs/results.csv") != 3 ]]; then
    fail "lib.sh dry with the hairpin and paced modes: their rtt rows: $(tail -2 "$dhs/log")"
elif [[ $hairpin_after != 192.0.2.1 ]] || grep -q "192.0.2.2 sudo pkill -f '/\[f\]loor" "$dhs/calls" ||
    [[ ! -s $dhs/stalls-pong-isolated-dpdk-raw@HAIRPIN=1-1.csv ]]; then
    fail "lib.sh dry with a hairpin: the pong's stalls not from a, or a pong ended on b ($hairpin_after)"
else
    pass "lib.sh dry: a hairpin rep is one floor client on a over its own two NICs (pong on the receiver's CPU), its stalls from a; a paced mode's clients get RTT_GAP_US and rtt_plan's seconds, round trips, warm-up and deadlines"
fi

# vm.sh client: RTT_GAP_US reaches the client through sudo (0, unpaced,
# without it); floor xhairpin takes the last run's spikes, histogram and
# both loop-stall files away first, as xping and xpong do theirs
gap_client() { # <environment>... -- <client args>...
    local envs=()
    while [[ $1 != -- ]]; do envs+=("$1"); shift; done
    shift
    : >"$sbc/log"
    env -i PATH="$sbc/bin:$PATH" HOME="$sbc/home" FAKE_LOG="$sbc/log" FAKE_ARCH=x86_64 LAB_SYS_ROOT="$sys/aws-x86" ${envs[@]+"${envs[@]}"} \
        bash "$cloud/vm.sh" client "$@" >"$sbc/out" 2>&1
    echo "rc $? $(grep '^sudo ' "$sbc/log" | tail -1)"
}
rm -f "$sbc/home/bench/harness_driver" "$sbc/home/bench/driver.env"
paced_client=$(gap_client RTT_GAP_US=50000 -- isolated:dpdk@RTT_GAP_US=50000 rtt xping 10.30.1.7:20123 10.30.1.5:20124 10 1 2)
unpaced_client=$(gap_client -- isolated:dpdk rtt xping 10.30.1.7:20123 10.30.1.5:20124 10 1 2)
echo dpdk >"$sbc/home/bench/floor"
for f in stalls-ping.csv stalls-pong.csv load-spikes.csv rtt-histogram.csv; do : >"$sbc/home/bench/$f"; done
hairpin_client=$(gap_client -- isolated:dpdk-raw@HAIRPIN=1 floor xhairpin 10.30.1.5:20123 10.30.1.4:20124 10 1 2 3)
left=$(cd "$sbc/home/bench" && { ls stalls-ping.csv stalls-pong.csv load-spikes.csv rtt-histogram.csv 2>/dev/null || true; } | tr '\n' ' ')
rm -f "$sbc/home/bench/floor"
if [[ $paced_client != *" RTT_GAP_US=50000 "* || $unpaced_client != *" RTT_GAP_US=0 "* ]]; then
    fail "vm.sh client: RTT_GAP_US: paced [$paced_client] unpaced [$unpaced_client]"
elif [[ $hairpin_client != "rc 0 "*"/target/release/floor xhairpin 10.30.1.5:20123 10.30.1.4:20124 10 1 2 3" || -n $left ]]; then
    fail "vm.sh client floor xhairpin: [$hairpin_client], left: $left"
else
    pass "vm.sh client: RTT_GAP_US through sudo (0 without it); floor xhairpin with the last run's files taken away"
fi

# vm.sh host_setting HAIRPIN=1 (vm.sh driver, a raw floor mode): the pong's
# CPU is the layout's receiver core, which the raw floor leaves free
# (recorded for the run); on the kernel wire the kernel would deliver a's
# own address over loopback, so the mode routes each end's sends out of its
# own NIC (a rule for each direction before the local table, which moves
# from priority 0 to 50; accept_local and no reverse-path filter on the data
# NICs, which then receive their own host's packets), checked by ip route
# get both ways (fail closed, everything put back); restore_host puts the
# local table back at 0 before its moved rule goes, the rules and sysctls
# as they were. Refused off the raw floor, off AWS and without a receiver core
hs=$tmp/hairpin-setting
mkdir -p "$hs/bench"
hairpin_sb() { # <floor wire> <cloud> <receiver cpu> <route get answer>: "<rc> | <calls> | <bench/hairpin>"
    (
        set +e
        exec 2>&1
        # shellcheck source=../modes.sh
        . "$cloud/modes.sh" || exit 1
        eval "$(for f in save_host host_setting restore_host; do vm_fn "$f"; done)"
        run=$hs/bench CLOUD=$2 HK_CPU=0 APP_CPU=1 SENDER_CPU=2 RECEIVER_CPU=$3 APP2_CPU= threading=INVOKER floor_wire=$1
        rm -rf "$run"/*
        : >"$hs/calls"
        nics() { printf 'sender ens6 ens6 0c 10.30.1.4 24 0000:00:06.0\nreceiver ens7 ens7 0d 10.30.1.5 24 0000:00:07.0\n'; }
        sudo() { echo "$*" >>"$hs/calls"; }
        sysctl() { echo 2; }
        ip() {
            if [[ $1 == route && $2 == get ]]; then
                [[ $4 == from && $3 == 10.30.1.5 ]] && echo "${FAKE_FWD:-10.30.1.5 from 10.30.1.4 dev ens6 table 100 uid 0}"
                [[ $4 == from && $3 == 10.30.1.4 ]] && echo "10.30.1.4 from 10.30.1.5 dev ens7 table 101 uid 0"
            elif [[ $1 == rule && $2 == show ]]; then
                echo "0:	from all lookup local"
            fi
        }
        log() { :; }
        rc=0
        (FAKE_FWD=$4 host_setting HAIRPIN 1 kernel-busy) 2>"$hs/err" || rc=$?
        echo "$rc | $(tr '\n' ';' <"$hs/calls") | $(cat "$run/hairpin" 2>/dev/null)"
        : >"$hs/calls"
        restore_host
        echo "restored | $(tr '\n' ';' <"$hs/calls") | $(ls "$run/restore" 2>/dev/null | tr '\n' ' ')$(cat "$run/hairpin" 2>/dev/null)"
        echo "said: $(tr '\n' ' ' <"$hs/err")"
    )
}
kernel_on=$(hairpin_sb kernel aws 3 '')
dpdk_on=$(hairpin_sb dpdk aws 3 '')
looped=$(hairpin_sb kernel aws 3 'local 10.30.1.5 from 10.30.1.4 dev lo table local uid 0')
off_floor=$(hairpin_sb '' aws 3 '')
azure=$(hairpin_sb dpdk azure 3 '')
no_cpu=$(hairpin_sb dpdk aws '' '')
want_kernel="0 | sysctl -qw net.ipv4.conf.all.rp_filter=0 net.ipv4.conf.ens6.rp_filter=0 net.ipv4.conf.ens7.rp_filter=0 net.ipv4.conf.ens6.accept_local=1 net.ipv4.conf.ens7.accept_local=1;ip rule add priority 10 iif lo from 10.30.1.4 to 10.30.1.5 lookup 100;ip rule add priority 11 iif lo from 10.30.1.5 to 10.30.1.4 lookup 101;ip rule add priority 50 lookup local;ip rule del priority 0 lookup local; | 3
restored | ip rule add priority 0 lookup local;ip rule del priority 50 lookup local;ip rule del priority 10;ip rule del priority 11;sysctl -qw net.ipv4.conf.all.rp_filter=2 net.ipv4.conf.ens6.rp_filter=2 net.ipv4.conf.ens7.rp_filter=2 net.ipv4.conf.ens6.accept_local=2 net.ipv4.conf.ens7.accept_local=2; | 
said: "
if [[ $kernel_on != "$want_kernel" ]]; then
    fail "vm.sh host_setting HAIRPIN=1 on the kernel wire"
    diff -u <(echo "$want_kernel") <(echo "$kernel_on") >&2 || true
elif [[ $dpdk_on != "0 |  | 3
restored |  | 
said: " ]]; then
    fail "vm.sh host_setting HAIRPIN=1 on the DPDK wire: $dpdk_on"
elif [[ $looped != 1\ \|*"ip rule add priority 0 lookup local"*"| "$'\n'"restored |  | "* || $looped != *"over loopback"* ]]; then
    fail "vm.sh host_setting HAIRPIN=1, a route still over loopback: $looped"
elif [[ $off_floor != 2\ * || $off_floor != *"raw floor"* || $azure != 2\ * || $azure != *"AWS only"* || $no_cpu != 2\ * || $no_cpu != *"receiver core"* ]]; then
    fail "vm.sh host_setting HAIRPIN=1 refusals: [$off_floor] [$azure] [$no_cpu]"
else
    pass "vm.sh host_setting HAIRPIN=1: the pong on the receiver core; on the kernel wire each end routed out of its own NIC, checked both ways (fail closed, put back), restored local table first; refused off the raw floor, off AWS, without a receiver core"
fi

# vm.sh data_kernel_up after HAIRPIN=1 (a later @RX_RING= in the same mode
# brings the NICs up again): it removes an address's stale plain rule (the
# NICs back in another order), never the hairpin's from/to rules, which would
# put the round trip back over loopback unseen
dku=$(
    set +e
    exec 2>&1
    eval "$(vm_fn data_kernel_up)"
    : >"$hs/dku-calls"
    data_unmanaged() { :; }
    nics() { printf 'sender ens6 ens6 0c 10.30.1.4 24 0000:00:06.0\nreceiver ens7 ens7 0d 10.30.1.5 24 0000:00:07.0\n'; }
    policy_routes() { :; }
    pin_irqs() { :; }
    data_routes_missing() { echo 0; }
    sudo() { echo "$*" >>"$hs/dku-calls"; }
    ip() {
        [[ $1 == rule && $2 == show ]] && printf '%s\n' "0:	from all lookup local" \
            "10:	from 10.30.1.4 to 10.30.1.5 iif lo lookup 100" "11:	from 10.30.1.5 to 10.30.1.4 iif lo lookup 101" \
            "50:	from all lookup local" "100:	from 10.30.1.4 lookup 100" "101:	from 10.30.1.5 lookup 101" \
            "102:	from 10.30.1.5 lookup 100"
        return 0
    }
    ARCH=x86_64 PEER_SUBNET=
    data_kernel_up || echo "rc $?"
    grep 'rule' "$hs/dku-calls" | tr '\n' ';'
)
if [[ $dku != "ip rule del priority 102 from 10.30.1.5 lookup 100;" ]]; then
    fail "vm.sh data_kernel_up after HAIRPIN=1: rules changed: [$dku]"
else
    pass "vm.sh data_kernel_up: a stale plain rule removed, the hairpin's from/to rules kept"
fi

# vm.sh ready: a hairpin needs the layout's receiver core on host a (a
# two-core host has none)
# shellcheck disable=SC2016
fake_cpus "$sys/c9g-l" 2 '$cpu'
got="two cores: $(ready_sb c9g-l 7.2.6+deb13-cloud-arm64 dpdk-raw@HAIRPIN=1 dpdk-raw)
four cores: $(ready_sb c8g-xl 7.2.6+deb13-cloud-arm64 dpdk-raw@HAIRPIN=1 kernel-raw@HAIRPIN=1)"
if [[ $got != "two cores: 2 mode dpdk-raw@HAIRPIN=1: no receiver core on this host (its layout 0 1): the hairpin's pong runs there; four cores (c9g.xlarge and up)
four cores: 0" ]]; then
    fail "vm.sh ready with a hairpin: $got"
else
    pass "vm.sh ready: a hairpin refused on a host without the layout's receiver core, said before any state"
fi

# analyse.py: a hairpin and paced modes named for what they are; the hairpin
# beside the cross-host floor of the same mode and state (its maxima there
# too: below the fabric); round trips by gap, each beside the same mode
# unpaced, p99.99 only from 10,000 round trips a rep; a paced Aeron mode's
# floor is the floor paced alike; a hairpin is not set against the round
# trip between the hosts, which it never crosses
cat >"$tmp/hs-results.csv" <<'CSV'
0,netrtt,isolated,40000,50000,60000,5000,200/200
1,rtt,isolated:dpdk-raw,udp,15000,16000,20000,25000,35000,85000,15500,300000,0
2,rtt,isolated:dpdk-raw,udp,15000,16000,20000,25000,35000,87000,15500,300000,0
1,rtt,isolated:dpdk-raw@HAIRPIN=1,udp,9000,9500,11000,15000,30000,84000,9200,300000,0
2,rtt,isolated:dpdk-raw@HAIRPIN=1,udp,9000,9500,11000,15000,30000,86000,9200,300000,0
1,rtt,isolated:dpdk,udp,25000,26000,30000,35000,45000,72000,25500,300000,0
1,rtt,isolated:dpdk@RTT_GAP_US=1000,udp,26000,27000,31000,36000,46000,90000,26500,20000,0
1,rtt,isolated:dpdk@RTT_GAP_US=50000,udp,28000,29000,40000,50000,60000,100000,28500,1200,0
1,rtt,isolated:dpdk-raw@RTT_GAP_US=50000,udp,18000,19000,30000,40000,50000,95000,18500,1200,0
CSV
hs_md=$(python3 "$cloud/analyse.py" "$tmp/hs-results.csv" 2>&1 || true)
hp=$(section "## Same-host hairpin against the cross-host round trip (µs)" <<<"$hs_md" | tr -s ' ')
sp=$(section "## Sparse round trips by gap (µs)" <<<"$hs_md" | tr -s ' ')
fl=$(section "## UDP round trip above the raw floor (µs)" <<<"$hs_md" | tr -s ' ')
net=$(section "## UDP round trip above the network's own (µs)" <<<"$hs_md")
if [[ $hs_md != *"| isolated, DPDK packet path, no Aeron (raw floor), same-host hairpin (HAIRPIN=1) "* ||
    $hs_md != *"| isolated, DPDK, one round trip every 50 ms (RTT_GAP_US=50000) "* ]]; then
    fail "analyse.py: the hairpin's and paced modes' names: $(grep -m3 'HAIRPIN\|RTT_GAP' <<<"$hs_md")"
elif [[ $hp != *"| isolated, DPDK packet path, no Aeron (raw floor) | 2 | 2 | 9.00 (9.00–9.00) | 15.00 (15.00–15.00) | 30.00 (30.00–30.00) | 35.00 (35.00–35.00) | 85.00 (84.00–86.00) | 86.00 (85.00–87.00) |"* ]]; then
    fail "analyse.py: the hairpin table"
    echo "$hp" >&2
elif [[ $sp != *"| isolated, DPDK | none | 1 | 300,000 | 25.00 (25.00–25.00) | 30.00 (30.00–30.00) | 45.00 (45.00–45.00) | 72.00 (72.00–72.00) |"* ||
    $sp != *"| isolated, DPDK | 1 ms | 1 | 20,000 | 26.00 (26.00–26.00) | 31.00 (31.00–31.00) | 46.00 (46.00–46.00) | 90.00 (90.00–90.00) |"* ||
    $sp != *"| isolated, DPDK | 50 ms | 1 | 1,200 | 28.00 (28.00–28.00) | 40.00 (40.00–40.00) | n/a (under 10,000) | 100.00 (100.00–100.00) |"* ||
    $sp != *"| isolated, DPDK packet path, no Aeron (raw floor) | 50 ms | 1 | 1,200 |"* || $sp == *HAIRPIN* ]]; then
    fail "analyse.py: the sparse table"
    echo "$sp" >&2
elif [[ $fl != *"| isolated, DPDK, one round trip every 50 ms (RTT_GAP_US=50000) | DPDK packet path, no Aeron (raw floor), one round trip every 50 ms (RTT_GAP_US=50000) | 1 | 10.00 (10.00–10.00) "* ||
    $fl == *"RTT_GAP_US=1000)"*"| 1 |"* || $fl == *"HAIRPIN"* ]]; then
    fail "analyse.py: a paced mode's floor"
    echo "$fl" >&2
elif [[ $net != *"| isolated, DPDK packet path, no Aeron (raw floor) "* || $net == *HAIRPIN* ]]; then
    fail "analyse.py: a hairpin above the network's own round trip between the hosts, which it never crosses"
    echo "$net" >&2
else
    pass "analyse.py: the hairpin and paced modes named; the hairpin beside its cross-host floor; round trips by gap beside unpaced (p99.99 from 10,000 a rep); a paced mode above its floor paced alike"
fi
# a paced mode with an idle localhost subscription (<mode>@RTT_GAP_US=<us>+idle,
# which modes.sh takes): its gap read, named, and set beside the same +idle
# mode unpaced
cat >"$tmp/hs-idle.csv" <<'CSV'
0,netrtt,isolated,40000,50000,60000,5000,200/200
1,rtt,isolated:dpdk+idle,udp,25000,26000,30000,35000,45000,72000,25500,300000,0
1,rtt,isolated:dpdk@RTT_GAP_US=1000+idle,udp,26000,27000,31000,36000,46000,90000,26500,20000,0
CSV
hs_idle=$(python3 "$cloud/analyse.py" "$tmp/hs-idle.csv" 2>&1 || true)
sp_idle=$(section "## Sparse round trips by gap (µs)" <<<"$hs_idle" | tr -s ' ')
if [[ $sp_idle != *"| isolated, DPDK, idle localhost subscription | none | 1 | 300,000 |"* ||
    $sp_idle != *"| isolated, DPDK, idle localhost subscription | 1 ms | 1 | 20,000 |"* ||
    $hs_idle != *"| isolated, DPDK, idle localhost subscription, one round trip every 1 ms (RTT_GAP_US=1000) "* ]]; then
    fail "analyse.py: a paced +idle mode"
    echo "$sp_idle" >&2
else
    pass "analyse.py: a paced +idle mode named, beside the same +idle mode unpaced"
fi

# spikes.py: a hairpin's spikes are host a's alone (b plays no part: its
# spinner's gap at that moment is no cause), and a paced run's length for the
# chance baseline is its pings times the gap, not the sum of its round trips
hsd=$tmp/hs-spikes/isolated
mkdir -p "$hsd"
python3 - "$hsd" <<'EOF'
import os, sys
d = sys.argv[1]
W0 = 2_000_000_000
for host in "ab":
    with open(os.path.join(d, "..", f"{host}-diag-isolated.log"), "w") as f:
        f.write("== gapspin\n# cpu=5 ns_per_tick=1.000000 threshold_us=10 counter_hz=1000000000\n")
        if host == "b":
            f.write(f"{W0 + 1.000400:.6f} 400\n")
        f.write("== trace\n== drains\n== ipmon\n== knoise\n")
for mode in ("dpdk-raw@HAIRPIN=1", "dpdk-raw"):
    with open(os.path.join(d, f"spikes-isolated-{mode}-rtt-1.csv"), "w") as f:
        f.write(f"# start_realtime_ns={W0 * 1_000_000_000}\n# spikes_total=1\nstream,due_ns,latency_ns,ping\n")
        f.write("1002,1000000000,500000,7\n")
for m in ("dpdk-raw@RTT_GAP_US=50000", "dpdk@RTT_GAP_US=50000+idle"):
    with open(os.path.join(d, f"histogram-isolated-{m}-1.csv"), "w") as f:
        f.write("value_ns,count\n20000,1000\n30000,200\n")
EOF
hs_out=$(python3 "$cloud/spikes.py" "$hsd" 2>&1 || true)
hs_len=$(python3 -c 'import sys; sys.path.insert(0, sys.argv[1]); import spikes
print(spikes.run_length(sys.argv[2], "isolated-dpdk-raw@RTT_GAP_US=50000-1", 0.0, 1.0),
      spikes.run_length(sys.argv[2], "isolated-dpdk@RTT_GAP_US=50000+idle-1", 0.0, 1.0))' "$cloud" "$hsd")
if [[ $hs_out != *"| isolated-dpdk-raw-rtt | 500: host stall (b)"* || $hs_out != *"| isolated-dpdk-raw@HAIRPIN=1-rtt | 500: unexplained"* ]]; then
    fail "spikes.py: a hairpin's spikes judged on host a alone"
    grep 'isolated-dpdk' <<<"$hs_out" >&2 || echo "$hs_out" >&2
elif [[ $hs_len != "60.0 60.0" ]]; then
    fail "spikes.py run_length of a paced run (and a paced +idle one): $hs_len, not 1,200 pings x 50 ms = 60.0 s each"
else
    pass "spikes.py: a hairpin's spikes judged on host a alone; a paced run's length its pings times the gap"
fi

# estimate.py: a paced rep's round trip takes rtt_plan's seconds (60 s at
# 50 ms: 40 s and its warm-up's 10 s more than an unpaced one's) and sends
# its round trips and warm-up; a hairpin sends nothing between the hosts
est=$(python3 - "$cloud" <<'PY'
import sys
sys.path.insert(0, sys.argv[1])
import estimate as e
args = dict(type_a="c9g.2xlarge", type_b="c9g.2xlarge", placement="cluster", reps=1, local_modes=[], states=["pinned"],
            load_sizes=[], load_fractions=[], load_ladder={}, load_seconds=5, mix=[], mix_seconds=5,
            tput_seconds=5, tput_flows=0, rtt_messages=300000, rtt_warmup=50000, no_flatout=True, max_hours=8)
fixed = e.SETUP_S + e.TEARDOWN_S + e.STATE_S["pinned"]
raw = e.estimate(modes=["dpdk-raw"], **args)
paced = e.estimate(modes=["dpdk-raw@RTT_GAP_US=50000"], **args)
hairpin = e.estimate(modes=["dpdk-raw@HAIRPIN=1@RTT_GAP_US=50000"], **args)
print(e.rtt_plan("dpdk@RTT_GAP_US=50000", 300000, 50000), e.rtt_plan("dpdk", 300000, 50000),
      paced["wall_s"] - raw["wall_s"], paced["bytes_ab"] == (1200 + 40) * (e.RTT_SIZE + e.WIRE_OVERHEAD),
      hairpin["bytes_ab"], hairpin["bytes_ba"], hairpin["wall_s"] == paced["wall_s"])
PY
)
if [[ $est == "(60, 1200, 40) (20, 300000, 50000) 50 True 0 0 True" ]]; then
    pass "estimate.py: a paced rep's round trip by rtt_plan (seconds, round trips, warm-up); a hairpin sends nothing between the hosts"
else
    fail "estimate.py with the hairpin and paced modes: $est"
fi

# azure/lab.sh, against a fake az first on PATH that logs every call
# ($FAKE_AZ_LOG) and keeps the one resource group's state in $FAKE_AZ_STATE:
# a group created (its run tag kept), shown, listed and deleted; each
# VM size's quota family (list-skus) and the family's and the region's vCPU
# use and limit (list-usage) printed as azure-cli 2.91 prints a top-level
# list, one value per line (FAKE_AZ_TABS=1: tab-separated, as earlier ones);
# a public address never found (FAKE_AZ_IP_RC), so a run stops right after
# its VMs are created and tears down.
mkdir -p "$tmp/fakeaz" "$tmp/azstate" "$tmp/azrepo"
cat >"$tmp/fakeaz/az" <<'FAKEAZ'
#!/usr/bin/env bash
st=${FAKE_AZ_STATE:?}
echo "az $*" >>"${FAKE_AZ_LOG:?}"
emit() { if [[ ${FAKE_AZ_TABS:-} == 1 ]]; then (IFS=$'\t'; echo "$*"); else printf '%s\n' "$@"; fi; }
case "$*" in
"account show"*) exit 0 ;;
"group exists"*) [[ -f $st/group ]] && echo true || echo false ;;
"group create"*) touch "$st/group"; [[ $* =~ run=([^ ]+) ]] && echo "${BASH_REMATCH[1]}" >"$st/run"; exit "${FAKE_AZ_CREATE_RC:-0}" ;;
"group delete"*) rm -f "$st/group" ;;
"group wait"*) ;;
"group show"*) [[ -f $st/group ]] || exit 3; cat "$st/run" ;;
"vm list-skus"*MaxNetworkInterfaces*) echo "${FAKE_AZ_NICS:-8}" ;;
"vm list-skus"*)
    case "$*" in
    *Standard_D8ps_v6* | *Standard_D16ps_v6*) echo standardDPSv6Family ;;
    *Standard_D8s_v6* | *Standard_D16s_v6*) echo standardDSv6Family ;;
    esac ;;
"vm list-usage"*)
    case "$*" in
    *"=='cores'"*) emit ${FAKE_AZ_CORES:-10 200} ;;
    *standardDPSv6Family*) emit ${FAKE_AZ_FAMILY:-4 100} ;;
    *standardDSv6Family*) emit 0 100 ;;
    esac ;;
"resource list --tag"*) if [[ -f $st/group ]]; then echo 2; else echo "${FAKE_AZ_LEFT:-0}"; fi ;;
"resource list"*) [[ -f $st/group ]] && printf 'Microsoft.Network/virtualNetworks\tvnet\nMicrosoft.Compute/virtualMachines\ta\n' ;;
"network public-ip show"*) exit "${FAKE_AZ_IP_RC:-1}" ;;
"disk show"*) [[ $* =~ -n\ ([^ ]+) ]] && echo "/subscriptions/0/resourceGroups/g/providers/Microsoft.Compute/disks/${BASH_REMATCH[1]}" ;;
esac
exit 0
FAKEAZ
chmod +x "$tmp/fakeaz/az"
# every az below is the fake (a real one would query the account)
saved_path=$PATH
export PATH="$tmp/fakeaz:$PATH" FAKE_AZ_STATE=$tmp/azstate FAKE_AZ_LOG=$tmp/az-calls

# modes.sh type_cores: one thread a core on Azure's sizes without SMT (AMD's
# Fasv6, Cobalt 100's Dpsv6, Dplsv6, Dpdsv6 and Epsv6), two on the rest
az_cores=$(. "$cloud/modes.sh"; echo "$(type_cores azure Standard_F8as_v6) $(type_cores azure Standard_F16as_v6) $(type_cores azure Standard_D8ps_v6) $(type_cores azure Standard_D8pls_v6) $(type_cores azure Standard_D8pds_v6) $(type_cores azure Standard_E8ps_v6) $(type_cores azure Standard_D8s_v6) $(type_cores azure Standard_D8as_v6)")
if [[ $az_cores == "8 16 8 8 8 8 4 4" ]]; then
    pass "modes.sh type_cores: Azure's Fasv6 and Cobalt 100 sizes one thread a core, the D v6 and Dasv6 two"
else
    fail "modes.sh type_cores on Azure: $az_cores, not 8 16 8 8 8 8 4 4"
fi

# azure/az.sh: a size's architecture and its Debian 13 image; az_words joins
# a tsv list printed one value per line (azure-cli 2.91) or tab-separated;
# the quota check reads each size's family and the region's vCPUs either way
# and refuses a run they cannot hold, or a size the region does not offer
az_helpers=$(
    . "$cloud/azure/az.sh"
    rc() { "$@" >/dev/null 2>&1; echo -n " $?"; }
    echo "arch: $(azure_arch Standard_D8ps_v6) $(azure_arch Standard_D8s_v6) $(azure_arch Standard_E8ps_v6) $(azure_arch Standard_F8as_v6) $(azure_arch Standard_D16pls_v6)"
    echo "image: $(azure_image arm64) $(azure_image x86_64)"
    echo "words: [$(printf '4\n100\n' | az_words)] [$(printf '4\t100\n' | az_words)]"
    echo "quota lines:$(rc azure_quota_check northcentralus Standard_D8ps_v6 Standard_D8ps_v6)"
    azure_quota_check northcentralus Standard_D8ps_v6 Standard_D8ps_v6 2>&1
    echo "quota tabs:$(FAKE_AZ_TABS=1 rc azure_quota_check northcentralus Standard_D8ps_v6 Standard_D8ps_v6)"
    echo "quota family full:$(FAKE_AZ_FAMILY='90 100' rc azure_quota_check northcentralus Standard_D8ps_v6 Standard_D8ps_v6)"
    FAKE_AZ_FAMILY='90 100' azure_quota_check northcentralus Standard_D8ps_v6 Standard_D8ps_v6 2>&1
    echo "quota region full:$(FAKE_AZ_CORES='190 200' rc azure_quota_check northcentralus Standard_D8ps_v6 Standard_D8s_v6)"
    echo "not offered:$(rc azure_quota_check northcentralus Standard_D8zz_v6)"
) || true
want_helpers="arch: arm64 x86_64 arm64 x86_64 arm64
image: Debian:debian-13:13-arm64:latest Debian:debian-13:13-gen2:latest
words: [4 100] [4 100]
quota lines: 0
quota: standardDPSv6Family 4 of 100 vCPUs in use, this run 16; the region 10 of 200, this run 16
quota tabs: 0
quota family full: 2
quota: standardDPSv6Family in northcentralus: 90 of 100 vCPUs in use, and this run needs 16 more: delete what runs there or raise the quota
quota region full: 2
not offered: 2"
if [[ $az_helpers == "$want_helpers" ]]; then
    pass "azure/az.sh: the size's architecture and Debian 13 image; quota read from a one-value-a-line or tab-separated list, a full family or region refused"
else
    fail "azure/az.sh helpers"
    diff -u <(echo "$want_helpers") <(echo "$az_helpers") >&2 || true
fi

# the backstop deletes the group at its deadline only while the group is
# this run's (its run tag); the watch logs the group's resources each period
mkdir -p "$tmp/fakedate"
printf '#!/bin/sh\nif [ "$1" = +%%s ]; then n=$(cat "%s/date-calls" 2>/dev/null || echo 0); echo $((n + 1)) >"%s/date-calls"; s=$(/bin/date +%%s); [ "$n" -gt 0 ] && s=$((s + 100000)); echo "$s"; else exec /bin/date "$@"; fi\n' "$tmp" "$tmp" >"$tmp/fakedate/date"
chmod +x "$tmp/fakedate/date"
reap=$(
    . "$cloud/azure/az.sh"
    : >"$FAKE_AZ_LOG"
    touch "$FAKE_AZ_STATE/group"; echo s1 >"$FAKE_AZ_STATE/run"
    azure_reaper g1 s2 1 "$tmp/reaper-other.log" >/dev/null
    for _ in $(seq 40); do grep -q . "$tmp/reaper-other.log" 2>/dev/null && break; sleep 0.25; done
    echo -n "other: $([[ -f $FAKE_AZ_STATE/group ]] && echo kept || echo deleted)"
    azure_reaper g1 s1 1 "$tmp/reaper-own.log" >/dev/null
    for _ in $(seq 40); do [[ -f $FAKE_AZ_STATE/group ]] || break; sleep 0.25; done
    echo -n "; own: $([[ -f $FAKE_AZ_STATE/group ]] && echo kept || echo deleted)"
    echo -n "$(grep -q 'resource group g1 verified gone' "$tmp/reaper-own.log" && echo ', verified' || echo ', unverified')"
    # a deadline in wall-clock time: a Mac asleep stops sleep's clock, so a
    # sleep of the whole period would delete hours late. Here every date +%s
    # after the first is 100000 s later, as after a night asleep.
    touch "$FAKE_AZ_STATE/group"; echo s5 >"$FAKE_AZ_STATE/run"; rm -f "$tmp/date-calls"
    pid=$(PATH="$tmp/fakedate:$PATH" AZ_REAPER_POLL=1 azure_reaper g5 s5 3600 "$tmp/reaper-asleep.log")
    for _ in $(seq 40); do [[ -f $FAKE_AZ_STATE/group ]] || break; sleep 0.25; done
    echo -n "; asleep: $([[ -f $FAKE_AZ_STATE/group ]] && echo kept || echo deleted)"
    kill -TERM "$pid" 2>/dev/null
    pid=$(AZ_REAPER_POLL=7200 azure_reaper g9 s9 3600 "$tmp/reaper-killed.log")
    kill -TERM "$pid"; sleep 0.5
    echo -n "; killed: $(pgrep -f 'ergon-lab-reaper 3600 g9' >/dev/null && echo alive || echo gone)$(pgrep -fx 'sleep 3600' >/dev/null && echo ', its sleep alive' || true)"
    touch "$FAKE_AZ_STATE/group"
    azure_watch g1 1 2>"$tmp/watch.log" &
    w=$!
    sleep 1.6
    kill "$w"
    echo "; watch: $(grep -c 'resources in g1: 2: Microsoft.Network/virtualNetworks/vnet Microsoft.Compute/virtualMachines/a$' "$tmp/watch.log")"
    rm -f "$FAKE_AZ_STATE/group" "$FAKE_AZ_STATE/run"
) || true
if [[ $reap == "other: kept; own: deleted, verified; asleep: deleted; killed: gone; watch: 1" ]]; then
    pass "azure/az.sh: the backstop deletes only its own run's group at the deadline, checks it gone, keeps its deadline in wall-clock time (through a sleep), and dies with its sleep when killed; the watch lists the group's resources"
else
    fail "azure/az.sh backstop and watch: $reap ($(cat "$tmp"/reaper-*.log 2>/dev/null))"
fi

# a whole run on Cobalt 100 (arm64), to the point its VMs exist: the quota
# read before the group, every resource tagged owner=ergon-lab run=<stamp>
# (the OS disks after their VMs), the arm64 Debian 13 image, a DPDK mode
# accepted on arm64 (DPDK builds the MANA and netvsc PMDs for arm64 too),
# each VM's auto-shutdown at the deadline (Azure's own: it stops the compute
# billing with this machine asleep; the group's deletion is the backstop's),
# the backstop started before the group and, once the group is verified gone,
# stopped
: >"$FAKE_AZ_LOG"
rm -rf "$tmp/azrepo/target"
env PATH="$tmp/fakeaz:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/azrepo" \
    LAB_GROUP=ergon-lab-test LAB_SIZE=Standard_D8ps_v6 LAB_MODES="kernel dpdk" LAB_STATES=pinned LAB_REPS=1 \
    LAB_SSH_FROM=203.0.113.9 LAB_DETACH=0 bash "$cloud/azure/lab.sh" >"$tmp/azrun.out" 2>&1 && azrc=0 || azrc=$?
stamp=$(ls "$tmp/azrepo/target/md-dpdk-bench" 2>/dev/null | head -1 || true)
creates=$(grep -E ' create |^az tag update ' "$FAKE_AZ_LOG" | grep -vE 'subnet create|nsg rule create' || true)
untagged=$(grep -vF -- "--tags owner=ergon-lab run=$stamp" <<<"$creates" || true)
reaper_pid=$(sed -n 's/.*backstop.*(pid \([0-9]*\)).*/\1/p' "$tmp/azrun.out" | head -1 || true)
why=
[[ $azrc != 0 ]] || why+=" exit 0;"
[[ -n $stamp && $(grep -c . <<<"$creates") -ge 12 && -z $untagged ]] || why+=" untagged: [$untagged];"
grep -q -- 'tag update --resource-id .*/disks/a-os --operation merge' "$FAKE_AZ_LOG" && grep -q -- 'tag update --resource-id .*/disks/b-os --operation merge' "$FAKE_AZ_LOG" || why+=" OS disks not tagged;"
grep -q 'vm create.*--image Debian:debian-13:13-arm64:latest' "$FAKE_AZ_LOG" || why+=" no arm64 image;"
off=$(date -u -v+8H +%H 2>/dev/null || date -u -d '+8 hours' +%H)
grep -q "vm auto-shutdown -g ergon-lab-test -n a --time $off" "$FAKE_AZ_LOG" && grep -q "vm auto-shutdown -g ergon-lab-test -n b --time $off" "$FAKE_AZ_LOG" || why+=" no auto-shutdown at the deadline;"
[[ -s $tmp/azrepo/target/md-dpdk-bench/$stamp/reaper.log || -f $tmp/azrepo/target/md-dpdk-bench/$stamp/reaper.log ]] || why+=" no backstop log;"
[[ $(grep -n 'backstop:' "$tmp/azrun.out" | head -1 | cut -d: -f1) -lt $(grep -n 'creating ergon-lab-test' "$tmp/azrun.out" | cut -d: -f1) ]] 2>/dev/null || why+=" backstop not started before the group;"
grep -q 'left tagged run=.*: 0$' "$tmp/azrun.out" || why+=" tagged leftovers not counted;"
[[ $(grep -n 'vm list-usage' "$FAKE_AZ_LOG" | head -1 | cut -d: -f1) -lt $(grep -n 'group create' "$FAKE_AZ_LOG" | cut -d: -f1) ]] 2>/dev/null || why+=" quota not read before the group;"
grep -q 'az group delete -n ergon-lab-test' "$FAKE_AZ_LOG" && grep -q 'verified: resource group ergon-lab-test no longer exists' "$tmp/azrun.out" || why+=" no verified delete;"
[[ -n $reaper_pid ]] && ! kill -0 "$reaper_pid" 2>/dev/null || why+=" backstop [$reaper_pid] not started or left running;"
! pgrep -f 'ergon-lab-reaper .* ergon-lab-test' >/dev/null || why+=" a backstop left running;"
grep -q 'no public address' "$tmp/azrun.out" || why+=" no refusal of the missing address;"
if [[ -z $why ]]; then
    pass "azure/lab.sh on Cobalt 100: quota read first, every resource tagged owner=ergon-lab run=<stamp>, the arm64 image, a DPDK mode accepted, auto-shutdown at the deadline, the backstop started before the group and stopped once the group is verified gone"
else
    fail "azure/lab.sh run:$why $(tail -4 "$tmp/azrun.out")"
fi
rm -rf "$tmp/azrepo/target" "$FAKE_AZ_STATE"/*

# fail closed: a group create that fails after making the group is still
# deleted; resources still tagged run=<stamp> after it a WARNING; no backstop
# (nohup fails) refused before the group exists; LAB_MAX_HOURS of a day or
# more no auto-shutdown (a daily schedule would fire before the deadline)
azrun() { # out-file vars...
    local o=$1
    shift
    : >"$FAKE_AZ_LOG"
    rm -rf "$tmp/azrepo/target" "$FAKE_AZ_STATE"/*
    env PATH="$tmp/fakeaz:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/azrepo" \
        LAB_GROUP=ergon-lab-test LAB_SIZE=Standard_D8s_v6 LAB_MODES=kernel LAB_STATES=pinned LAB_REPS=1 \
        LAB_SSH_FROM=203.0.113.9 LAB_DETACH=0 "$@" bash "$cloud/azure/lab.sh" >"$o" 2>&1
}
why=
azrun "$tmp/azrun-create.out" FAKE_AZ_CREATE_RC=1 FAKE_AZ_LEFT=1 && why+=" create failure exit 0;"
grep -q 'az group delete -n ergon-lab-test' "$FAKE_AZ_LOG" && grep -q 'verified: resource group ergon-lab-test no longer exists' "$tmp/azrun-create.out" ||
    why+=" a failed create not deleted;"
grep -q 'WARNING: 1 resource(s) still tagged run=' "$tmp/azrun-create.out" || why+=" tagged leftovers not a WARNING;"
mkdir -p "$tmp/fakenohup"
printf '#!/bin/sh\nexit 1\n' >"$tmp/fakenohup/nohup"
chmod +x "$tmp/fakenohup/nohup"
azrun "$tmp/azrun-nohup.out" PATH="$tmp/fakenohup:$tmp/fakeaz:$PATH" && why+=" no backstop exit 0;"
grep -q 'backstop' "$tmp/azrun-nohup.out" && ! grep -q 'group create' "$FAKE_AZ_LOG" || why+=" no backstop not refused before the group: $(tail -1 "$tmp/azrun-nohup.out");"
azrun "$tmp/azrun-day.out" LAB_MAX_HOURS=30 || true
! grep -q 'auto-shutdown' "$FAKE_AZ_LOG" && grep -q 'no auto-shutdown: LAB_MAX_HOURS=30' "$tmp/azrun-day.out" || why+=" auto-shutdown with LAB_MAX_HOURS=30;"
if [[ -z $why ]]; then
    pass "azure/lab.sh fails closed: a group whose create failed deleted, tagged leftovers a WARNING, no backstop refused before the group, no daily auto-shutdown before a deadline a day or more away"
else
    fail "azure/lab.sh fail-closed:$why"
fi
rm -rf "$tmp/azrepo/target" "$FAKE_AZ_STATE"/*

# refused before any cloud call (the fake that logs to cloud-calls): sizes
# of two architectures (one image for both); a plan longer than the backstop's
# deadline. LAB_ESTIMATE=1 prints the hours and the sizes' prices, no call.
for refused in "LAB_SIZE_A=Standard_D8ps_v6|LAB_SIZE_B=Standard_D8s_v6#one architecture" \
    "LAB_REPS=40|LAB_MAX_HOURS=2#give LAB_MAX_HOURS="; do
    IFS='|' read -ra vars <<<"${refused%#*}"
    if env PATH="$tmp/fakebin:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/commarepo" \
        "${vars[@]}" bash "$cloud/azure/lab.sh" >"$tmp/az.out" 2>&1; then
        fail "azure/lab.sh ran with ${vars[*]}"
    elif grep -qF -- "${refused##*#}" "$tmp/az.out" && [[ ! -s $tmp/cloud-calls ]]; then
        pass "azure/lab.sh refuses ${vars[*]} before any cloud call (${refused##*#})"
    else
        fail "azure/lab.sh with ${vars[*]}: $(tail -2 "$tmp/az.out"; cat "$tmp/cloud-calls" 2>/dev/null)"
    fi
    rm -f "$tmp/cloud-calls"
done
est=$(env PATH="$tmp/fakebin:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/commarepo" \
    LAB_ESTIMATE=1 LAB_SIZE=Standard_D8ps_v6 bash "$cloud/azure/lab.sh" 2>&1) && estrc=0 || estrc=$?
if [[ $estrc == 0 && ! -s $tmp/cloud-calls && $est == *"wall time about "*" h"* && $est == *'Standard_D8ps_v6'* && $est == *'$0.281/h'* && $est == *'total about $'* ]]; then
    pass "azure/lab.sh LAB_ESTIMATE=1: hours and dollars at the sizes' prices, no cloud call"
else
    fail "azure/lab.sh LAB_ESTIMATE=1 (exit $estrc): $est $(cat "$tmp/cloud-calls" 2>/dev/null)"
fi
rm -f "$tmp/cloud-calls"

# a plan estimated over 2 h detaches (a nohup subshell, its parent 1, its
# log in the run directory) and returns; here the detached run goes to its
# teardown in its log (the fake az's missing address). az's sign-in is
# checked before detaching, so a run that cannot start says so here, not in
# a log nobody reads; LAB_DETACH=0 keeps it in the foreground.
rm -rf "$tmp/commarepo/target" "$FAKE_AZ_STATE"/*
det=$(env PATH="$tmp/fakeaz:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/commarepo" \
    LAB_GROUP=ergon-lab-test LAB_SSH_FROM=203.0.113.9 LAB_REPS=12 LAB_MAX_HOURS=24 bash "$cloud/azure/lab.sh" 2>&1) && detrc=0 || detrc=$?
detlog=$(ls "$tmp"/commarepo/target/md-dpdk-bench/*/lab.log 2>/dev/null | head -1 || true)
for _ in $(seq 80); do [[ -n $detlog ]] && grep -q 'results:' "$detlog" && break; sleep 0.25; done
for _ in $(seq 40); do [[ -z $(find "$tmp/commatmp" -mindepth 1 -maxdepth 1 -name 'ergon-lab.*') ]] && break; sleep 0.25; done
detout=$(cat "$detlog" 2>/dev/null || true)
rm -rf "$tmp/commarepo/target"
nolog=$(env PATH="$tmp/fakebin:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/commarepo" \
    LAB_REPS=12 LAB_MAX_HOURS=24 bash "$cloud/azure/lab.sh" 2>&1) && nologrc=0 || nologrc=$?
fg=$(env PATH="$tmp/fakebin:$PATH" HOME="$tmp/commahome" TMPDIR="$tmp/commatmp" LAB_REPO="$tmp/commarepo" \
    LAB_REPS=12 LAB_MAX_HOURS=24 LAB_DETACH=0 bash "$cloud/azure/lab.sh" 2>&1) && fgrc=0 || fgrc=$?
if [[ $detrc == 0 && $det == *"detached"*"lab.log"* && -n $detlog && $detout == *'verified: resource group ergon-lab-test no longer exists'* &&
    -z $(find "$tmp/commatmp" -mindepth 1 -maxdepth 1 -name 'ergon-lab.*') && $nologrc != 0 && $nolog == *'not logged in'* && $nolog != *detached* &&
    -z $(ls "$tmp"/commarepo/target/md-dpdk-bench/*/lab.log 2>/dev/null) && $fgrc != 0 && $fg == *'not logged in'* ]]; then
    pass "azure/lab.sh detaches a plan over 2 h into a nohup subshell logging to the run directory (its snapshot removed as it exits), after checking az's sign-in; LAB_DETACH=0 runs it in the foreground"
else
    fail "azure/lab.sh detach (exit $detrc): $det; log [$detlog] $(tail -2 <<<"$detout"); not signed in (exit $nologrc): $(tail -1 <<<"$nolog"); foreground (exit $fgrc): $(tail -1 <<<"$fg")"
fi
rm -rf "$tmp/commarepo/target" "$FAKE_AZ_STATE"/*
rm -f "$tmp/cloud-calls"
unset FAKE_AZ_STATE FAKE_AZ_LOG
export PATH=$saved_path

for script in vm.sh irqs.sh host.sh modes.sh lib.sh remote.sh transport.sh azure/lab.sh azure/az.sh aws/lab.sh test/purefloor-docker.sh test/test-transport.sh test/test-transport-runner.sh; do
    bash -n "$cloud/$script" && pass "bash -n $script" || fail "bash -n $script"
done
# spikes.py's agent gap table: every agent loop gap the DPDK event log
# stamped (<host>-rings-<run>.log), on its agent's CPU as the trace shows it
# (its tid), labelled by a traced preemption, else the interrupts there (each
# entry to its exit; their time inside the gap), else the spare CPU's spinner
# gap on that host, else nothing the guest saw; a gap of a tid the trace never
# showed not classed. The trace put on the wall clock through its two
# ergon-anchor markers (1 ppm apart here: an offset alone would place the
# second half 50 µs late), not /proc/uptime's line. Then the spinner's gaps
# by size. vm.sh gapspin_us: bench/gapspin_us, else 50.
gt=$tmp/gapcause/isolated
mkdir -p "$gt"
python3 - "$gt" <<'EOF'
import os, sys
root = sys.argv[1]
S = 1_000_000_000
W0 = 2_000_000_000  # the wall clock at trace time 1000 s
def tr(wall):
    """the trace's clock for a wall-clock second (1 ppm slow against it)"""
    return f"{1000.0 + (wall - W0) / (1 + 1e-6):.6f}"
def ns(wall):
    return int(round(wall * S))
trace = [
    f"     python3-900     [001] .....  {tr(W0)}: tracing_mark_write: ergon-anchor {W0 * S}",
    # receiver, tid 4242, on CPU 3: a timer tick of 3 µs, an IPI of 2 µs
    f"    receiver-4242    [003] d.h1.  {tr(W0 + 10.000010)}: irq_handler_entry: irq=11 name=arch_timer",
    f"    receiver-4242    [003] d.h1.  {tr(W0 + 10.000013)}: irq_handler_exit: irq=11 ret=handled",
    f"    receiver-4242    [003] d.h1.  {tr(W0 + 60.000010)}: ipi_entry: (Function call interrupts)",
    f"    receiver-4242    [003] d.h1.  {tr(W0 + 60.000012)}: ipi_exit: (Function call interrupts)",
    # a kworker preempts it for 25 µs
    f"    receiver-4242    [003] d..2.  {tr(W0 + 20.000005)}: sched_switch: prev_comm=receiver prev_pid=4242 prev_prio=120 prev_state=R+ ==> next_comm=kworker/3:1 next_pid=77 next_prio=120",
    f" kworker/3:1-77      [003] d..2.  {tr(W0 + 20.000030)}: sched_switch: prev_comm=kworker/3:1 prev_pid=77 prev_prio=120 prev_state=I ==> next_comm=receiver next_pid=4242 next_prio=120",
    f"     python3-901     [001] .....  {tr(W0 + 100.0)}: tracing_mark_write: ergon-anchor {(W0 + 100) * S}",
]
with open(os.path.join(root, "..", "a-diag-isolated.log"), "w") as f:
    # spinner gaps: 15 µs, 30 µs (with the 40-µs agent gap at 40 s), 120 µs
    f.write("== gapspin\n# cpu=5 ns_per_tick=1.000000 threshold_us=10 counter_hz=1000000000\n")
    f.write(f"{W0 + 5.000015:.6f} 15\n{W0 + 40.000035:.6f} 30\n{W0 + 70.000120:.6f} 120\n")
    # the uptime line 1 s off: used, every event would miss its gap
    f.write("== trace\n" + "\n".join(trace) + f"\nUPTIME 1001.0 REALTIME {W0 + 0.0}\n== drains\n== ipmon\n== knoise\n")
gaps = [
    (W0 + 10.000020, 20000),  # [10.000000, 10.000020]: the tick
    (W0 + 20.000040, 40000),  # [20.000000, 20.000040]: the preemption
    (W0 + 40.000040, 40000),  # [40.000000, 40.000040]: the spinner's 30-µs gap
    (W0 + 50.000025, 25000),  # nothing
    (W0 + 60.000030, 30000),  # the IPI, 2 µs of it: an anchor's offset alone puts it 60 µs late
]
with open(os.path.join(root, "a-rings-isolated-dpdk@RUSTERON_DPDK_EVENT_LOG=x-1.log"), "w") as f:
    f.write("# kind,tsc,realtime_ns,value_ns,port,agent\n")
    for end, v in gaps:
        f.write(f"gap,1,{ns(end)},{v},1,4242\n")
    f.write(f"gap,1,{ns(W0 + 80.0)},22000,0,5555\n")  # a tid the trace never showed
    f.write(f"arp-request,1,{ns(W0 + 30.0)},0,0,4242\n")
EOF
gapcause=$(python3 "$cloud/spikes.py" "$gt" 2>&1 | awk '/^## Agent loop gaps/{on=1} on')
want='## Agent loop gaps by what overlapped them, isolated

Every gap of an agent'"'"'s loop the DPDK event log stamped (modes with @RUSTERON_DPDK_EVENT_LOG), on the
agent'"'"'s CPU: a traced preemption; else interrupts there (their time inside the gap); else a gap of the
spare CPU'"'"'s spinner on the same host (the whole VM paused); else nothing the guest saw: the agent'"'"'s own
work, or the hypervisor taking this vCPU alone. Lengths in µs: median (largest).

| host | class | gaps | length | interrupt time inside | what |
| --- | --- | --- | --- | --- | --- |
| a | guest preemption | 1 | 40.0 (40.0) | 0.0 (0.0) | kworker/3:1 (1) |
| a | interrupts | 2 | 30.0 (30.0) | 3.1 (3.1) | arch_timer (1), Function call interrupts (1) |
| a | VM paused | 1 | 40.0 (40.0) | 0.0 (0.0) | spinner gap (1) |
| a | nothing in the guest | 1 | 25.0 (25.0) | 0.0 (0.0) | - |

a: 1 gaps of an agent the trace never showed (no CPU known): not classed.

The spare CPU'"'"'s gap spinner, each host, the whole state (vm.sh diag-start; its threshold is the
diag log'"'"'s threshold_us):

| host | gaps | 10-20 µs | 20-50 µs | 50-100 µs | 100 µs or more | largest |
| --- | --- | --- | --- | --- | --- | --- |
| a | 3 | 1 | 1 | 0 | 1 | 120 |'
spin_us=$(
    run=$tmp/gapcause
    eval "$(vm_fn gapspin_us)"
    echo -n "$(gapspin_us)"
    echo 10 >"$run/gapspin_us"; echo -n " $(gapspin_us)"
    echo 0 >"$run/gapspin_us"; echo -n " $(gapspin_us)"
    echo x >"$run/gapspin_us"; echo -n " $(gapspin_us)"
)
if [[ $gapcause != "$want" ]]; then
    fail "spikes.py: agent loop gaps by cause"
    diff -u <(echo "$want") <(echo "$gapcause") >&2 || true
elif [[ $spin_us != "50 10 50 50" ]]; then
    fail "vm.sh gapspin_us: $spin_us, not 50 10 50 50"
else
    pass "spikes.py: each agent loop gap labelled on its agent's CPU (preemption, interrupts timed entry to exit, the spinner's VM pause, nothing), the trace on the wall clock through its two anchors; the spinner's gaps by size; vm.sh gapspin_us"
fi

for script in analyse.py spikes.py estimate.py transport_plan.py test/fake_aws.py; do
    python3 -c 'import ast, sys; ast.parse(open(sys.argv[1]).read(), sys.argv[1])' "$cloud/$script" &&
        pass "$script compiles" || fail "$script does not compile"
done

if ((failed)); then
    echo "test-lab: FAILED" >&2
    exit 1
fi
echo "test-lab: all passed"
