#!/usr/bin/env python3
"""Where each latency spike, and each rep's max, came from, for one state of a lab run.

spikes.py <run dir>/<state> [<threshold us>]

Reads the spike files the clients wrote (spikes-*.csv: round trips of 100 us
or more, with their run's CLOCK_REALTIME start) and the diagnostics vm.sh
diag-fetch saved for each host (a-diag-<state>.log, b-diag-<state>.log: the gap
spinner on a spare CPU, a trace of the app and agent CPUs). A spike of at least
<threshold us> (default 200) overlapping:

- a spinner gap on either host is a host stall: the hypervisor paused the VM
  (the spinner's CPU is idle, so nothing in the guest can explain its gap);
- a traced preemption of a busy thread (an agent, ping or pong) is a guest
  preemption, named by what ran instead;
- a stall of the ping or pong client's own loop (stalls-ping/pong-*.csv) is
  a client stall;
- a window of the media driver's own errors (a-/b-errors-<run>.log: failed
  sends, NAKs or status messages, and loss reports), when nothing above
  explains it, is driver errors, named by the first error;
- none of them is unexplained: the NIC, the fabric, or a stall of one vCPU
  the spinner does not share. For each round trip run the receive agents'
  poll gaps of 100 us or more (the jitter snapshots before and after) say
  whether an agent stalled: with none, an unexplained spike was in flight.
  Where a host ran no spinner (no CPU outside its layout: a four-core
  Graviton) or had it paused (an SMT=off mode takes its CPU offline), a host
  stall there cannot be ruled out: such a spike's cause says so, and a note
  names the host.

Then, for every round trip run, each rep's 10 largest round trips (at any
size: the clients stamp idle round trips from LOAD_SPIKE_NS, 30 us cross-host)
with every event whose interval overlaps it ([sent, echoed]), its ping index
in the measured run and its echo's phase in the second and in 30 s. Events:
either client's loop stalls, the gap spinner and traced preemptions as above,
the DPDK transport's event log where a mode turned it on
(RUSTERON_DPDK_EVENT_LOG, saved per round trip run as <host>-rings-<run>.log:
the agents' loop gaps of 10 us or more, ARP, the housekeeping thread's 1 Hz
sample, NIC statistics read and PMD timers, each with its length), and KNOISE (diag-fetch's knoise section: osnoise's irq,
softirq, thread and NMI noise on the agent and app CPUs with their durations,
IPIs sent to those CPUs by callsite, compaction and page migration). Each
source's overlaps are set against chance: the rep's round trips shifted
circularly by a random offset within the run (its round trips' sum, from the
histogram), 1,000 times with a fixed seed, how many overlap then; p is the
share of shifts overlapping at least as many.

Under precision-time, each rep's largest round trips split one way first
(oneway-<state>-<mode>-<rep>.csv, rtt's RTT_ONE_WAY: both hosts' NIC
receive timestamps, the clocks on the PTP hardware clock): out (sent to the
pong's NIC), back (the pong's NIC to the ping's) and the ping's host (its NIC
to the echo's arrival), so a spike is placed on one direction or one host.

The match across hosts is on their wall clocks: where chrony's tracking was
logged around each mode (vm.sh timesync, <host>-timesync-*.log), a note gives
each host's largest offset from its source and that source's own bound
against UTC, and their sum, how far apart the two clocks may be.
"""
import bisect
import collections
import csv
import glob
import gzip
import os
import random
import re
import sys

# the threads that busy-spin on traced CPUs: the dedicated agents, the shared
# network agent ("[sender, receiver]", which the kernel cuts to 15 characters),
# SHARED's one agent (the media driver's main thread; under the other modes the
# conductor, on the untraced housekeeping CPU) and the clients
BUSY = re.compile(r"^(sender|receiver|\[sender, |\[conductor, |media_driver|rtt|load|tput|floor)")
EVENT = re.compile(r"\[(\d+)\].*?\s(\d+\.\d+): (\w+): (.*)")
# a thread name may hold spaces ("[sender, receiv")
SWITCH = re.compile(r"prev_comm=(.+?) prev_pid=\d+ prev_prio=\d+ prev_state=(\S+) ==> next_comm=(.+?) next_pid=")


def spike_files(folder):
    """The clients' spike files in <folder>, plain (spikes-*.csv) or gzipped
    (spikes-*.csv.gz, as large ones are kept), with each one's stem: the name
    between spikes- and the suffix."""
    paths = glob.glob(os.path.join(folder, "spikes-*.csv")) + glob.glob(os.path.join(folder, "spikes-*.csv.gz"))
    return sorted((p, os.path.basename(p)[len("spikes-"):].removesuffix(".gz").removesuffix(".csv")) for p in paths)


def open_spikes(path):
    """A spike file opened as text, gzipped or not."""
    return gzip.open(path, "rt", newline="") if path.endswith(".gz") else open(path, newline="")


# vm.sh diag-start and diag-fetch write "ergon-anchor <CLOCK_REALTIME ns>" to
# the trace (trace_marker, on a traced CPU, the trace on CLOCK_MONOTONIC):
# each a pair of the trace's clock and the wall clock, a few µs apart
ANCHOR = re.compile(r"\s(\d+\.\d+): tracing_mark_write: ergon-anchor (\d+)")
TASK = re.compile(r"^\s*(.+?)-(\d+)\s+\[(\d+)\]")


def trace_clock(lines, uptime):
    """The trace's clock onto the wall clock (seconds): through the first and
    last of the trace's ergon-anchor markers (a line between them, so drift
    between the two clocks cancels), one marker an offset; without one,
    diag-fetch's UPTIME line (/proc/uptime, 10 ms resolution), an offset;
    None with neither."""
    anchors = []
    for line in lines:
        m = ANCHOR.search(line)
        if m:
            anchors.append((float(m.group(1)), int(m.group(2)) / 1e9))
    if len(anchors) >= 2 and anchors[-1][0] > anchors[0][0]:
        (t0, w0), (t1, w1) = anchors[0], anchors[-1]
        rate = (w1 - w0) / (t1 - t0)
        return lambda ts: w0 + (ts - t0) * rate
    if anchors:
        t0, w0 = anchors[0]
        return lambda ts: w0 + (ts - t0)
    if uptime:
        return lambda ts: uptime[1] - (uptime[0] - ts)
    return None


def read_diag(path):
    """(spinner gaps [(start_s, end_s)], preemptions [(start_s, end_s, cpu, who, by)]) on the wall clock."""
    gaps, lines, anchor, section = [], [], None, None
    for line in open(path, errors="replace"):
        line = line.rstrip("\n")
        if line.startswith("== "):
            section = line[3:]
            continue
        if section == "gapspin" and line and not line.startswith("#"):
            end, us = line.split()[:2]
            gaps.append((float(end) - float(us) / 1e6, float(end)))
        elif section == "trace":
            if line.startswith("UPTIME"):
                f = line.split()
                anchor = (float(f[1]), float(f[3]))
            else:
                lines.append(line)
    preempted, away, spinner_out = [], {}, []
    wall = trace_clock(lines, anchor)
    for line in lines if wall else []:
        m = EVENT.search(line)
        if not m or m.group(3) != "sched_switch":
            continue
        s = SWITCH.search(m.group(4))
        if not s:
            continue
        cpu, ts = int(m.group(1)), wall(float(m.group(2)))
        prev, state, nxt = s.groups()
        # the spinner itself taken off its CPU by the guest: not a host stall
        if prev == "gapspin" and state.startswith("R"):
            away[("spin", cpu)] = (ts, prev, nxt)
        elif nxt == "gapspin" and ("spin", cpu) in away:
            spinner_out.append((away.pop(("spin", cpu))[0], ts))
        if BUSY.match(prev) and state.startswith("R"):
            away[cpu] = (ts, prev, nxt)
        elif BUSY.match(nxt) and cpu in away:
            t0, who, by = away.pop(cpu)
            preempted.append((t0, ts, cpu, who, by))
    # a spinner gap the trace explains (the guest ran something else there) is
    # not the host's
    gaps = [g for g in gaps if not any(o0 < g[1] and o1 > g[0] for o0, o1 in spinner_out)]
    return gaps, preempted


def spinner_blind(path):
    """[(start_s, end_s)] on the wall clock when the host's gap spinner did
    not run (vm.sh diag-start's "# no spinner": the whole state; SMT=off's
    "# paused <s>" to "# resumed <s>", or to the end)."""
    blind, paused, section = [], None, None
    for line in open(path, errors="replace"):
        line = line.rstrip("\n")
        if line.startswith("== "):
            section = line[3:]
        elif section == "gapspin" and line.startswith("# no spinner"):
            blind.append((float("-inf"), float("inf")))
        elif section == "gapspin" and line.startswith("# paused "):
            paused = float(line.split()[2])
        elif section == "gapspin" and line.startswith("# resumed ") and paused is not None:
            blind.append((paused, float(line.split()[2])))
            paused = None
    if paused is not None:
        blind.append((paused, float("inf")))
    return blind


# osnoise's noise events (Linux include/trace/events/osnoise.h): the source
# (a thread's "comm:pid", an IRQ's "desc:vector", a softirq's "NAME:vector";
# none for an NMI), its start on the trace clock and its net duration
NOISE = re.compile(r"^\s*(?:(.*):(\d+) )?start (\d+)\.(\d+) duration (\d+) ns")
IPI_MASK = re.compile(r"cpumask=(\S+) callsite=(\S+)")
IPI_CPU = re.compile(r"cpu=(\d+) callsite=(\S+)")


def cpu_list(text):
    """{cpu} of a CPU list ("1-3,5")."""
    out = set()
    for part in text.split(","):
        if part.strip():
            lo, _, hi = part.partition("-")
            out.update(range(int(lo), int(hi or lo) + 1))
    return out


def cpu_mask(text, form):
    """{cpu} of a trace's cpumask: a list where the instance prints one
    (its bitmask-list option), else hex in 32-bit groups, the highest first."""
    if form == "list":
        return cpu_list(text)
    bits = int(text.replace(",", ""), 16)
    return {c for c in range(bits.bit_length()) if bits >> c & 1}


def read_knoise(path, host):
    """KNOISE's events in a host's diagnostics (vm.sh diag-fetch's knoise
    section) on the wall clock: [(start_s, end_s, source, label)]. Noise has
    its duration; an IPI, a compaction or a page migration is an instant. The
    busy threads' own runs (an agent, a client) are not noise, nor an IPI to
    none of the agent and app CPUs (the section's "# cpus")."""
    anchor, section, lines, cpus, form = None, None, [], set(), "hex"
    for line in open(path, errors="replace"):
        line = line.rstrip("\n")
        if line.startswith("== "):
            section = line[3:]
        elif section == "trace" and line.startswith("UPTIME"):
            f = line.split()
            anchor = (float(f[1]), float(f[3]))
        elif section == "knoise":
            if line.startswith("# cpus "):
                cpus = cpu_list(line[len("# cpus "):])
            elif line.startswith("# cpumask "):
                form = line.split()[2]
            elif not line.startswith("#"):
                lines.append(line)
    if anchor is None:
        return []
    wall = lambda ts: anchor[1] - (anchor[0] - ts)
    out = []
    for line in lines:
        m = EVENT.search(line)
        if not m:
            continue
        cpu, ts, event, rest = int(m.group(1)), float(m.group(2)), m.group(3), m.group(4)
        if event.endswith("_noise"):
            n = NOISE.match(rest)
            if not n:
                continue
            who = (n.group(1) or "").strip()
            if event == "thread_noise" and (BUSY.match(who) or who.startswith("swapper")):
                continue
            t0 = wall(int(n.group(3)) + int(n.group(4).ljust(9, "0")[:9]) / 1e9)
            dur = int(n.group(5)) / 1e9
            name = f"{event} {who}:{n.group(2)}" if who else event
            out.append((t0, t0 + dur, event, f"{name} {dur * 1e6:.1f} µs ({host} cpu {cpu})"))
        elif event in ("ipi_send_cpumask", "ipi_send_cpu"):
            i = IPI_MASK.search(rest) if event == "ipi_send_cpumask" else IPI_CPU.search(rest)
            if not i:
                continue
            to = cpu_mask(i.group(1), form) if event == "ipi_send_cpumask" else {int(i.group(1))}
            hit = to & cpus if cpus else to
            if hit:
                t = wall(ts)
                out.append((t, t, "IPI", f"IPI {i.group(2).split('+')[0]} to {','.join(map(str, sorted(hit)))} ({host})"))
        elif event == "mm_compaction_begin":
            out.append((wall(ts), wall(ts), "compaction", f"compaction ({host})"))
        elif event == "mm_migrate_pages":
            # (include/trace/events/migrate.h: nr_succeeded=, older kernels succeeded=)
            f = dict(kv.split("=", 1) for kv in rest.split() if "=" in kv)
            done = f.get("nr_succeeded", f.get("succeeded", "?"))
            out.append((wall(ts), wall(ts), "page migration",
                        f"page migration succeeded={done} reason={f.get('reason', '?')} ({host})"))
    return sorted(out)


# the DPDK transport's event log's housekeeping kinds (rusteron_dpdk_events.h)
RING_SOURCES = {"sample": "1 Hz sample", "nic-stats": "1 Hz NIC stats", "timers": "PMD timers"}


def read_rings(path, host):
    """The DPDK transport's event log for a run (RUSTERON_DPDK_EVENT_LOG,
    saved as <host>-rings-<run>.log; rows kind,tsc,realtime_ns,value_ns,port,
    agent, realtime_ns when the event ended, value_ns its length) on the wall
    clock: ([(start_s, end_s, source, label)], events the rings dropped). An
    agent's loop gap, a housekeeping call (the 1 Hz sample, the NIC
    statistics read, the PMDs' timers) over its length; ARP an instant."""
    out, dropped = [], 0
    for row in csv.reader(open(path, errors="replace")):
        if len(row) != 6 or row[0].startswith("#") or not (row[2].isdigit() and row[3].isdigit()):
            continue
        kind, end, value, port, agent = row[0], int(row[2]), int(row[3]), row[4], row[5]
        on_port = f" port {port}" if port != "-" else ""
        if kind == "drops":
            dropped += value
        elif kind == "gap":
            out.append(((end - value) / 1e9, end / 1e9, "agent gap", f"agent gap{on_port} {value / 1e3:.1f} µs ({host} tid {agent})"))
        elif kind.startswith("arp-"):
            out.append((end / 1e9, end / 1e9, "ARP", f"ARP {kind[4:]}{on_port} ({host})"))
        else:
            source = RING_SOURCES.get(kind, kind)
            out.append(((end - value) / 1e9, end / 1e9, source, f"{source} {value / 1e3:.1f} µs ({host}{on_port})"))
    return sorted(out), dropped


TOP_RTT = 10  # each rep's largest round trips labelled with every overlapping event
SHIFTS = 1000  # circular shifts for the chance baseline
SEED = 1


def overlaps(events, t0, t1):
    """The events of [(start, end, ...)] sorted by start, `longest` their
    longest duration, that overlap [t0, t1]."""
    starts, longest, rows = events
    lo = bisect.bisect_left(starts, t0 - longest)
    hi = bisect.bisect_right(starts, t1)
    return [r for r in rows[lo:hi] if r[1] >= t0]


def by_source(events):
    """{source: (starts, longest, rows)} of [(start, end, source, label)]."""
    groups = collections.defaultdict(list)
    for e in events:
        groups[e[2]].append(e)
    out = {}
    for source, rows in groups.items():
        rows.sort()
        out[source] = ([r[0] for r in rows], max(r[1] - r[0] for r in rows), rows)
    return out


def chance(reps, sources, shifts=SHIFTS, seed=SEED):
    """Per source: (observed, of, expected, p) over `reps`, each (round trips
    [(t0, t1)], run start, run length s, its events by_source): how many round
    trips an event of the source overlaps, and the same with each rep's round
    trips shifted circularly by a random offset within its run, `shifts` times."""
    rng = random.Random(seed)
    offsets = [[rng.uniform(0, length) for _ in range(shifts)] for _, _, length, _ in reps]
    out = {}
    for source in sources:
        def hits(shift_of):
            n = 0
            for r, (trips, start, length, events) in enumerate(reps):
                if source not in events:
                    continue
                for t0, t1 in trips:
                    s0 = start + (t0 - start + shift_of(r)) % length if shift_of else t0
                    if overlaps(events[source], s0, s0 + (t1 - t0)):
                        n += 1
            return n
        observed = hits(None)
        counts = [hits(lambda r, k=k: offsets[r][k]) for k in range(shifts)]
        out[source] = (observed, sum(len(t) for t, _, _, _ in reps), sum(counts) / shifts,
                       (1 + sum(c >= observed for c in counts)) / (1 + shifts))
    return out


def run_length(folder, run_tag, start, last):
    """A round trip run's measured length in seconds: its round trips' sum
    (histogram-<run>.csv, value_ns,count), else to its last event seen."""
    path = os.path.join(folder, f"histogram-{run_tag}.csv")
    if os.path.exists(path):
        total = 0
        for row in csv.reader(open(path)):
            if len(row) == 2 and row[0].isdigit():
                total += int(row[0]) * int(row[1])
        if total:
            return total / 1e9
    return max(last - start, 1.0)


BUCKETS = ("poll-cycle-gaps-20-50us", "poll-cycle-gaps-50-100us", "poll-cycle-gaps-over-100us")


def agent_gaps(dirs, run_tag):
    """The agents' loop gaps during a round trip run, per host: how many of
    20-50, 50-100 and over 100 us (both ports summed), from the jitter
    snapshots taken before and after it (the first of `dirs` holding them);
    None where they are missing. Also analyse.py --ab's."""
    out = {}
    for host in "ab":
        snaps, drivers = {}, {}
        for when in ("before", "after"):
            path = next((p for p in (os.path.join(d, f"{host}-jitter-{when}-rtt-{run_tag}.log") for d in dirs)
                         if os.path.exists(p)), None)
            if path is None:
                break
            text = open(path).read().splitlines()
            drivers[when] = next((l.split()[1] for l in text if l.startswith("thread media_driver/")), None)
            lines = [l for l in text if l.startswith("counter ")]
            if any(b in l for l in lines for b in BUCKETS):
                snaps[when] = [sum(int(l.split()[1]) for l in lines if b in l) for b in BUCKETS]
        # the ports' counters appear with the first transport, at zero: a
        # snapshot of the same driver taken before then counts as zero
        # (a build without the counters stays unknown)
        if "after" in snaps and "before" not in snaps and drivers.get("before") and drivers.get("before") == drivers.get("after"):
            snaps["before"] = [0] * len(BUCKETS)
        out[host] = [a - b for b, a in zip(snaps["before"], snaps["after"])] if len(snaps) == 2 else None
    return out



BURSTS = ("rx-burst-max-ns", "rx-bursts-over-1us", "rx-bursts-over-5us",
          "tx-burst-max-ns", "tx-bursts-over-1us", "tx-bursts-over-5us")


def pmd_bursts(dirs, run_tag):
    """The PMD burst timing during a round trip run, per host
    (RUSTERON_DPDK_GAP_ATTRIB=1 or burst), in BURSTS order, from the same jitter
    snapshots as agent_gaps: each max-ns is the after-snapshot's largest over
    the ports (a lifetime max, not differenced); each over-1us/5us count is
    summed over the ports and differenced against the before-snapshot. None
    where missing or all zero (GAP_ATTRIB off). Counter names are matched as
    whole words: nic-arrival-to-rx-burst-max-ns is another counter."""
    out = {}
    for host in "ab":
        snaps = {}
        for when in ("before", "after"):
            path = next((p for p in (os.path.join(d, f"{host}-jitter-{when}-rtt-{run_tag}.log") for d in dirs)
                         if os.path.exists(p)), None)
            if path is None:
                break
            values = {b: [] for b in BURSTS}
            for line in open(path).read().splitlines():
                words = line.split()
                if line.startswith("counter ") and len(words) > 1 and words[1].lstrip("-").isdigit():
                    for b in BURSTS:
                        if b in words[2:]:
                            values[b].append(int(words[1]))
            if any(values.values()):
                snaps[when] = values
        if "after" not in snaps:
            out[host] = None
            continue
        after, before = snaps["after"], snaps.get("before")
        row = []
        for b in BURSTS:
            if b.endswith("-max-ns"):
                row.append(max(after[b], default=0))
            elif before is not None:
                row.append(sum(after[b]) - sum(before[b]))
            else:
                row.append(None)
        out[host] = None if all(v in (0, None) for v in row) else row
    return out

TRACKING = re.compile(r"^(System time|Root delay|Root dispersion)\s*:\s*([0-9.]+) seconds( (fast|slow))?")


def clock_offsets(dirs):
    """Per host, from chrony's tracking logged around each mode (vm.sh
    timesync: <host>-timesync-*.log in the first of `dirs` holding any): the
    largest system-time offset from its source, the largest bound of that
    source against UTC (root dispersion and half the root delay), both in
    seconds, and how many snapshots; {} without them."""
    out = {}
    for host in "ab":
        paths = next((p for p in (sorted(glob.glob(os.path.join(d, f"{host}-timesync-*.log"))) for d in dirs) if p), [])
        offset, bound, n = 0.0, 0.0, 0
        for path in paths:
            got = {}
            for line in open(path, errors="replace"):
                m = TRACKING.match(line)
                if m:
                    got[m.group(1)] = float(m.group(2))
            if "System time" not in got:
                continue
            n += 1
            offset = max(offset, got["System time"])
            bound = max(bound, got.get("Root dispersion", 0.0) + got.get("Root delay", 0.0) / 2)
        if n:
            out[host] = (offset, bound, n)
    return out


def clocks_note(offsets):
    """The note spikes.py prints from clock_offsets'."""
    snaps = lambda n: f"{n} snapshot{'s' if n != 1 else ''}"
    parts = []
    for i, (host, (offset, bound, n)) in enumerate(sorted(offsets.items())):
        if i == 0:
            parts.append(f"{host}'s system time at most {offset * 1e6:.1f} µs from its source, that source within "
                         f"{bound * 1e6:.1f} µs of UTC (root dispersion and half the root delay; {snaps(n)})")
        else:
            parts.append(f"{host}'s at most {offset * 1e6:.1f} µs, its source within {bound * 1e6:.1f} µs ({snaps(n)})")
    total = sum(o + b for o, b, _ in offsets.values())
    return ("Host clocks, from chrony's tracking around each mode (vm.sh timesync): " + "; ".join(parts)
            + f". Spikes are matched to the other host's diagnostics across these clocks, which may differ by up to "
            f"{total * 1e6:.1f} µs (the sum).")


ONE_WAY_TOP = 10  # a rep's largest round trips split one way


def one_way(folder, state):
    """Each rep's ONE_WAY_TOP largest round trips split one way (rtt's
    RTT_ONE_WAY files), largest first: µs out, back and on the ping's host."""
    rows = []
    for path in sorted(glob.glob(os.path.join(folder, f"oneway-{state}-*.csv"))):
        stem = os.path.basename(path)[len("oneway-"):].removesuffix(".csv")
        m = re.match(r"^(.*)-(\d+)$", stem)
        if not m:
            continue
        run, rep = m.group(1), int(m.group(2))
        trips = []
        with open(path, newline="") as f:
            for row in csv.reader(f):
                if len(row) == 5 and all(v.isdigit() for v in row):
                    ping, sent, pong_rx, ping_rx, echoed = (int(v) for v in row)
                    trips.append((echoed - sent, ping, pong_rx - sent, ping_rx - pong_rx, echoed - ping_rx))
        for rtt, ping, out, back, host in sorted(trips, reverse=True)[:ONE_WAY_TOP]:
            rows.append((run, rep, -rtt, ping, rtt, out, back, host))
    if not rows:
        return
    print(f"\n## One-way split of the largest round trips, {state}\n")
    print("Each rep's largest round trips (rtt's RTT_ONE_WAY, the hosts' NIC receive timestamps on CLOCK_REALTIME, "
          "both on the PTP hardware clock): out is the ping's send to the pong host's NIC, back the pong host's NIC "
          "to the ping host's (the pong's host and the way back), the ping's host its NIC to the echo's arrival. The "
          "two hosts' clocks may differ by their sync error (timesync logs), which moves between out and back.\n")
    print("| run | rep | ping | round trip (µs) | out (µs) | back (µs) | ping's host (µs) |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for run, rep, _, ping, rtt, out, back, host in sorted(rows):
        print(f"| {run} | {rep} | {ping} | {rtt / 1e3:.1f} | {out / 1e3:.1f} | {back / 1e3:.1f} | {host / 1e3:.1f} |")


def main():
    folder = sys.argv[1]
    threshold = float(sys.argv[2]) if len(sys.argv) > 2 else 200.0
    state = os.path.basename(os.path.normpath(folder))
    run_dir = os.path.dirname(os.path.normpath(folder))
    one_way(folder, state)
    diag, blind, knoise = {}, {}, {}
    for host in "ab":
        for path in (os.path.join(run_dir, f"{host}-diag-{state}.log"), os.path.join(folder, f"{host}-diag-{state}.log")):
            if os.path.exists(path):
                diag[host] = read_diag(path)
                blind[host] = spinner_blind(path)
                knoise[host] = read_knoise(path, host)
                break
    if not diag:
        sys.exit(f"no {state} diagnostics (a-diag-{state}.log) next to {folder}")
    # intervals sorted by start, for a binary search: anything overlapping
    # [t0, t1] starts before t1 and, as none here lasts over a second, after
    # t0 - 1 s
    gap_idx = {h: sorted(g) for h, (g, _) in diag.items()}
    pre_idx = {h: sorted(p) for h, (_, p) in diag.items()}

    def overlapping(intervals, t0, t1):
        lo = bisect.bisect_left(intervals, (t0 - 1.0,))
        hi = bisect.bisect_left(intervals, (t1,))
        return [iv for iv in intervals[lo:hi] if iv[1] > t0]

    BASE = 50e-6  # an ordinary round trip: a cause must account for half of what a spike adds to it

    def read_stalls(path):
        """[(start_s, end_s)] of a client's loop stalls on the wall clock."""
        if not os.path.exists(path):
            return []
        with open(path) as f:
            first = f.readline()
            if not first.startswith("# start_realtime_ns="):
                return []
            base = int(first.split("=", 1)[1]) / 1e9
            out = []
            for line in f:
                parts = line.strip().split(",")
                if len(parts) == 2 and parts[0].isdigit():
                    end = base + int(parts[0]) / 1e9
                    out.append((end - int(parts[1]) / 1e9, end))
        return sorted(out)

    def run_gaps(run_tag):
        """agent_gaps for a run of this state, as "20-50/50-100/100+" per host."""
        return {h: "/".join(map(str, g)) if g is not None else "n/a"
                for h, g in agent_gaps((run_dir, folder), run_tag).items()}

    ERROR = re.compile(r"count=(\d+)(?: bytes=\d+)? first=(\d+) last=(\d+)(?:: (\(\d+\) [^|]*))?")

    def driver_errors(run_tag):
        """[(start_s, end_s, host, what, count)] of each distinct error and loss
        report in both drivers' error logs for a run (vm.sh errors)."""
        out = []
        for host in "ab":
            for base in (folder, run_dir):
                path = os.path.join(base, f"{host}-errors-{run_tag}.log")
                if os.path.exists(path):
                    break
            else:
                continue
            for line in open(path):
                m = ERROR.search(line)
                if not m:
                    continue
                what = "loss report" if line.startswith("loss ") else (m.group(4) or "error").strip()
                # the log's times are whole milliseconds
                out.append((int(m.group(2)) / 1e3, (int(m.group(3)) + 1) / 1e3, host, what, int(m.group(1))))
        return sorted(out)

    clients = {}
    errors = []

    def cause_of(t0, t1, base=BASE):
        need = max(t1 - t0 - base, 0.0) / 2
        for who, stalls in clients.items():
            if any(g1 - g0 >= need for g0, g1 in overlapping(stalls, t0, t1)):
                return f"{who} stall"
        for host in diag:
            if any(g1 - g0 >= need for g0, g1 in overlapping(gap_idx[host], t0, t1)):
                return f"host stall ({host})"
        for host in diag:
            hit = [p for p in overlapping(pre_idx[host], t0, t1) if p[1] - p[0] >= need]
            if hit:
                return f"guest preemption ({host}: {hit[0][3]} by {hit[0][4]})"
        hit = [e for e in errors if e[0] < t1 and e[1] > t0]
        if hit:
            return f"driver errors ({hit[0][2]}: {hit[0][3]} x{hit[0][4]:,})"
        # a host whose spinner did not run then: its stall not ruled out
        unseen = [h for h in sorted(blind) if any(b0 < t1 and b1 > t0 for b0, b1 in blind[h])]
        if unseen:
            return f"unexplained (no spinner on {', '.join(unseen)}: a host stall not ruled out)"
        return "unexplained"

    # load points past capacity (achieved below 98% of offered, or more than
    # 0.1% of messages back-pressured): their round trips queue, so their spikes
    # are queueing, not stalls. Clients before October 2026 also counted the
    # admin action each term rotation returns (about 2 in 100,000 messages of
    # 288 B), which the 0.1% leaves out.
    saturated = set()
    results = os.path.join(folder, "results.csv")
    if os.path.exists(results):
        for r in csv.reader(open(results)):
            if len(r) >= 20 and r[1] in ("load", "mix") and int(r[5]) > 0:
                if float(r[18]) < 0.98 * int(r[5]) or int(r[17]) > 0.001 * int(r[15]):
                    # as the file names spell it (lib.sh: a / in a mode, BIN's, is _)
                    mode = r[2].split(":", 1)[-1].replace("/", "_")
                    point = r[3].removeprefix("udp-")
                    saturated.add((mode, r[4], r[0], point))

    TOP = 50  # the largest spikes of a file attributed one by one; the rest counted
    capped = collections.Counter()  # spikes past a file's first 100,000, not written
    mid = collections.defaultdict(list)  # round trip runs: (rep, how many of 100-250 us)
    totals = collections.defaultdict(collections.Counter)
    maxima = collections.defaultdict(list)
    for path, stem in spike_files(folder):
        start = None
        with open_spikes(path) as f:
            first = f.readline()
            if first.startswith("# start_realtime_ns="):
                start = int(first.split("=", 1)[1]) / 1e9
            rows = []
            recorded_total = None  # the clients keep the first 100,000 and count the rest
            for line in f:
                if line.startswith("# spikes_total="):
                    recorded_total = int(line.split("=", 1)[1])
                    continue
                parts = line.rstrip().split(",")
                if len(parts) in (3, 4) and parts[1].isdigit():
                    rows.append((int(parts[2]) / 1e3, int(parts[1])))
        if start is None:
            continue
        run = re.sub(r"-\d+(-[^-]+)?$", lambda m: m.group(1) or "", stem)
        m = re.match(rf"^{re.escape(state)}-(.*)-(\d+)-(\d+)-([^-]+)$", stem)
        if m and (m.group(1), m.group(2), m.group(3), m.group(4)) in saturated:
            maxima[run].append((0.0, "past capacity (queueing)", "", None, False))
            continue
        # a round trip run: its clients' own stalls, and its agents' poll gaps
        rt = re.match(rf"^({re.escape(state)}-.*)-rtt-(\d+)$", stem)
        clients.clear()
        errors.clear()
        if m:
            errors.extend(driver_errors(f"{state}-{m.group(1)}-{m.group(3)}"))
        gaps_note = ""
        if rt:
            tag = f"{rt.group(1)}-{rt.group(2)}"
            errors.extend(driver_errors(tag))
            for who in ("ping", "pong"):
                clients[who] = read_stalls(os.path.join(folder, f"stalls-{who}-{tag}.csv"))
            gaps = run_gaps(tag)
            if gaps:
                gaps_note = (" (agent loop gaps 20-50/50-100/100+ us: "
                             + ", ".join(f"{h} {n}" for h, n in sorted(gaps.items())) + ")")
        rows.sort(reverse=True)
        if rt and f"{state}-local-" not in stem:
            mid[run].append((int(rt.group(2)), sum(1 for lat, _ in rows if 100 <= lat < 250)))
        # a localhost round trip is ~6 us, not ~50: its causes are judged against that
        base = 6e-6 if f"{state}-local-" in stem else BASE
        over = sum(1 for lat, _ in rows if lat >= threshold)
        for lat, due in rows[:TOP]:
            if lat >= threshold:
                t0 = start + due / 1e9
                totals[run][cause_of(t0, t0 + lat / 1e6, base)] += 1
        if over > TOP:
            totals[run]["not attributed (past the largest 50)"] += over - TOP
        if recorded_total is not None and recorded_total > len(rows):
            capped[run] += recorded_total - len(rows)
        if rows:
            lat, due = rows[0]
            t0 = start + due / 1e9
            maxima[run].append((lat, cause_of(t0, t0 + lat / 1e6, base), gaps_note, t0, bool(rt)))
        else:
            maxima[run].append((0.0, "under 100 us", gaps_note, None, bool(rt)))
    print(f"## Spikes of {threshold:.0f} us or more by cause, {state}\n")
    print("A load point past its capacity queues for seconds: its spikes are queueing, not stalls,")
    print("so only each file's 50 largest are attributed one by one.\n")
    print("| run | spikes | client stall | host stall | guest preemption | driver errors | unexplained | not attributed |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- |")
    for run in sorted(set(totals) | set(maxima)):
        c = totals[run]
        client = sum(v for k, v in c.items() if k.endswith(" stall") and k.split()[0] in ("ping", "pong"))
        host = sum(v for k, v in c.items() if k.startswith("host"))
        guest = sum(v for k, v in c.items() if k.startswith("guest"))
        rest = sum(v for k, v in c.items() if k.startswith("not attributed"))
        driver = sum(v for k, v in c.items() if k.startswith("driver"))
        unexplained = sum(v for k, v in c.items() if k.startswith("unexplained"))
        print(f"| {run} | {sum(c.values())} | {client} | {host} | {guest} | {driver} | {unexplained} | {rest} |")
    offsets = clock_offsets((folder, run_dir))
    if offsets:
        print("\n" + clocks_note(offsets))
    for h in sorted(blind):
        if (float("-inf"), float("inf")) in blind[h]:
            print(f"\nHost {h} ran no gap spinner (no CPU outside its layout): a host stall there cannot be told from "
                  "an unexplained spike, whose cause says so.")
        elif blind[h]:
            print(f"\nHost {h}'s gap spinner was paused {len(blind[h])} time(s) (an SMT=off mode takes its CPU offline): "
                  "a host stall there then cannot be told from an unexplained spike, whose cause says so.")
    for run, n in sorted(capped.items()):
        print(f"\n{run}: {n:,} more round trips over the client's spike threshold were counted but not written "
              "(past capacity, nearly every message queues).")
    print(f"\n## Each rep's max round trip and its cause, {state}\n")
    print("| run | max per rep (us): cause |")
    print("| --- | --- |")
    # A host event recurring at one second of each minute (Azure: a whole-VM
    # pause near :57) shows as unexplained maxima clustered at that second
    # across runs. Only idle round trip runs' maxima are candidates (a burst or
    # a loaded point has queueing maxima of its own at any second), and a
    # second marks the event when three or more of them, from two or more
    # runs, fall within a second of it, and at least three times as many as
    # chance would put in that two-second window. The unexplained maxima of
    # 500 us or more at that second, in any run, are labelled with it.
    big = [(t0 % 60, run) for run, reps in maxima.items() for lat, cause, _, t0, idle in reps
           if idle and t0 is not None and lat >= 500 and cause == "unexplained"]
    need = max(3, 3 * len(big) * 2 / 60)
    recurring = []
    for sec, _ in big:
        near = [r for s2, r in big if min(abs(s2 - sec), 60 - abs(s2 - sec)) <= 1.0]
        if len(near) >= need and len(set(near)) >= 2 and not any(min(abs(sec - x), 60 - abs(sec - x)) <= 1.0 for x in recurring):
            recurring.append(sec)

    def labelled(lat, cause, t0):
        if cause == "unexplained" and t0 is not None and lat >= 500:
            for sec in recurring:
                if min(abs(t0 % 60 - sec), 60 - abs(t0 % 60 - sec)) <= 1.0:
                    return f"recurring host event (:{sec:04.1f} each minute)"
        return cause

    for run, reps in sorted(maxima.items()):
        cells = "; ".join(f"{lat:.0f}: {labelled(lat, cause, t0)}{gaps}" if lat else f"{cause}{gaps}"
                          for lat, cause, gaps, t0, _ in reps)
        print(f"| {run} | {cells} |")
    # A slow-path episode (seen on Azure in about a quarter of runs, whatever
    # the setting): round trips of 100-250 us several times as frequent as
    # usual, spread over the run, with no loop stall, lifting p99.99. A rep
    # with over three times its mode's median count (and over 20) may sit in
    # one, so an A/B decided by its p99.99 should be rerun, not believed.
    if mid:
        print(f"\n## Round trips of 100-250 us per rep, {state}\n")
        print("| run | per rep | possible slow-path episode |")
        print("| --- | --- | --- |")
        for run, reps in sorted(mid.items()):
            counts = sorted(n for _, n in reps)
            median = counts[len(counts) // 2]
            odd = [f"rep {r} ({n})" for r, n in sorted(reps) if n > 20 and n > 3 * max(median, 1)]
            print(f"| {run} | {', '.join(str(n) for _, n in sorted(reps))} | {', '.join(odd) or 'none'} |")

    if recurring:
        print("\nMaxima of 500 us or more cluster at second " + ", ".join(f":{s:04.1f}" for s in sorted(recurring))
              + " of the minute: a host event recurring once a minute. A run catches it only if it spans that second "
              "(LAB_RTT_ALIGN starts every round trip just after it).")

    largest(folder, state, run_dir, diag, knoise)
    traces = {}
    for host in "ab":
        for path in (os.path.join(run_dir, f"{host}-diag-{state}.log"), os.path.join(folder, f"{host}-diag-{state}.log")):
            if os.path.exists(path):
                traces[host] = read_trace(path)
                break
    agent_gap_causes(folder, state, run_dir, diag, traces)


def stall_events(path, who):
    """A client's loop stalls (stalls-<who>-<run>.csv) as [(start_s, end_s,
    source, label)] on the wall clock, each labelled with its exact length."""
    if not os.path.exists(path):
        return []
    with open(path) as f:
        first = f.readline()
        if not first.startswith("# start_realtime_ns="):
            return []
        base = int(first.split("=", 1)[1])
        out = []
        for line in f:
            parts = line.strip().split(",")
            if len(parts) == 2 and parts[0].isdigit() and parts[1].isdigit():
                end, gap = base + int(parts[0]), int(parts[1])
                out.append(((end - gap) / 1e9, end / 1e9, f"{who} stall", f"{who} stall {gap / 1e3:.1f} µs"))
    return sorted(out)


def read_trace(path):
    """A host's trace on the wall clock (diag-fetch's trace section):
    ({pid: the CPU it ran on most}, {cpu: sorted [(start_s, end_s, name)]}
    of its interrupts: each irq_handler_entry to its CPU's next
    irq_handler_exit, each ipi_entry to its ipi_exit, an instant where the
    exit is missing), or None without a trace clock."""
    lines, uptime, section = [], None, None
    for line in open(path, errors="replace"):
        line = line.rstrip("\n")
        if line.startswith("== "):
            section = line[3:]
        elif section == "trace":
            if line.startswith("UPTIME"):
                f = line.split()
                uptime = (float(f[1]), float(f[3]))
            else:
                lines.append(line)
    wall = trace_clock(lines, uptime)
    if wall is None:
        return None
    seen = collections.defaultdict(collections.Counter)
    irqs, open_irq = collections.defaultdict(list), {}
    for line in lines:
        m = EVENT.search(line)
        if not m:
            continue
        cpu, ts, kind, rest = int(m.group(1)), wall(float(m.group(2))), m.group(3), m.group(4)
        t = TASK.match(line)
        if t and t.group(2) != "0":
            seen[t.group(2)][cpu] += 1
        if kind in ("irq_handler_entry", "ipi_entry"):
            name = re.search(r"name=(\S+)", rest)
            name = name.group(1) if name else rest.strip("() ")
            if (cpu, kind) in open_irq:  # its exit never traced: an instant
                t0, n = open_irq.pop((cpu, kind))
                irqs[cpu].append((t0, t0, n))
            open_irq[(cpu, kind)] = (ts, name)
        elif kind in ("irq_handler_exit", "ipi_exit"):
            entry = "irq_handler_entry" if kind == "irq_handler_exit" else "ipi_entry"
            if (cpu, entry) in open_irq:
                t0, n = open_irq.pop((cpu, entry))
                irqs[cpu].append((t0, ts, n))
    for (cpu, _), (t0, n) in open_irq.items():
        irqs[cpu].append((t0, t0, n))
    return ({pid: c.most_common(1)[0][0] for pid, c in seen.items()},
            {cpu: sorted(v) for cpu, v in irqs.items()})


GAP_CLASSES = ("guest preemption", "interrupts", "VM paused", "nothing in the guest")


def agent_gap_causes(folder, state, run_dir, diag, traces):
    """Every agent loop gap the DPDK event log stamped (<host>-rings-*.log,
    10 us or more), on its agent's CPU, labelled by what overlapped it: a
    traced preemption of the agent; interrupts on its CPU (their time inside
    the gap); a gap of the spare CPU's spinner on the same host (the whole VM
    paused); else nothing the guest saw (the agent's own work, or the
    hypervisor taking this vCPU alone). Then each host's spinner gaps by
    size."""
    rows = collections.defaultdict(lambda: collections.defaultdict(list))  # host: class: [(len us, irq us, label)]
    unmapped = collections.Counter()
    for host in "ab":
        paths = sorted(set(glob.glob(os.path.join(folder, f"{host}-rings-{state}-*.log"))
                           + glob.glob(os.path.join(run_dir, f"{host}-rings-{state}-*.log"))))
        if not paths or host not in diag:
            continue
        spin = sorted(diag[host][0])
        pre = collections.defaultdict(list)
        for t0, t1, cpu, who, by in diag[host][1]:
            pre[cpu].append((t0, t1, by))
        trace = traces.get(host)
        cpu_of, irqs = trace if trace else ({}, {})
        for path in paths:
            for row in csv.reader(open(path, errors="replace")):
                if len(row) != 6 or row[0] != "gap" or not (row[2].isdigit() and row[3].isdigit()):
                    continue
                end, value, tid = int(row[2]) / 1e9, int(row[3]), row[5]
                start = end - value / 1e9
                cpu = cpu_of.get(tid)
                if cpu is None:
                    unmapped[host] += 1
                    continue
                by = [b for t0, t1, b in pre.get(cpu, []) if t0 < end and t1 > start]
                hit = [(max(t0, start), min(t1, end), n) for t0, t1, n in irqs.get(cpu, [])
                       if t0 <= end and t1 >= start]
                irq_us = sum(t1 - t0 for t0, t1, _ in hit) * 1e6
                if by:
                    cls, label = "guest preemption", by[0]
                elif hit:
                    cls = "interrupts"
                    label = ", ".join(sorted(collections.Counter(n for _, _, n in hit)))
                elif any(g0 < end and g1 > start for g0, g1 in spin):
                    cls, label = "VM paused", "spinner gap"
                else:
                    cls, label = "nothing in the guest", ""
                rows[host][cls].append((value / 1e3, irq_us, label))
    spinner = {h: [(g1 - g0) * 1e6 for g0, g1 in g] for h, (g, _) in diag.items()}
    if not rows and not any(spinner.values()):
        return
    print(f"\n## Agent loop gaps by what overlapped them, {state}\n")
    print("Every gap of an agent's loop the DPDK event log stamped (modes with @RUSTERON_DPDK_EVENT_LOG), on the")
    print("agent's CPU: a traced preemption; else interrupts there (their time inside the gap); else a gap of the")
    print("spare CPU's spinner on the same host (the whole VM paused); else nothing the guest saw: the agent's own")
    print("work, or the hypervisor taking this vCPU alone. Lengths in µs: median (largest).\n")
    print("| host | class | gaps | length | interrupt time inside | what |")
    print("| --- | --- | --- | --- | --- | --- |")
    for host in sorted(rows):
        for cls in GAP_CLASSES:
            g = rows[host].get(cls)
            if not g:
                continue
            lens = sorted(x[0] for x in g)
            irq = sorted(x[1] for x in g)
            what = collections.Counter(x[2] for x in g if x[2]).most_common(3)
            print(f"| {host} | {cls} | {len(g):,} | {lens[len(lens) // 2]:.1f} ({lens[-1]:.1f}) | "
                  f"{irq[len(irq) // 2]:.1f} ({irq[-1]:.1f}) | {', '.join(f'{n} ({c})' for n, c in what) or '-'} |")
    for host, n in sorted(unmapped.items()):
        print(f"\n{host}: {n:,} gaps of an agent the trace never showed (no CPU known): not classed.")
    print("\nThe spare CPU's gap spinner, each host, the whole state (vm.sh diag-start; its threshold is the")
    print("diag log's threshold_us):\n")
    print("| host | gaps | 10-20 µs | 20-50 µs | 50-100 µs | 100 µs or more | largest |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for host, g in sorted(spinner.items()):
        b = [sum(1 for x in g if lo <= x < hi) for lo, hi in ((10, 20), (20, 50), (50, 100), (100, float("inf")))]
        print(f"| {host} | {len(g):,} | {' | '.join(f'{n:,}' for n in b)} | {max(g):.0f} |" if g else
              f"| {host} | 0 | 0 | 0 | 0 | 0 | - |")


def largest(folder, state, run_dir, diag, knoise):
    """Each round trip run's TOP_RTT largest round trips per rep, every event
    overlapping each, and each source's overlaps against chance."""
    # the state's events on either host, whatever the run: spinner gaps,
    # traced preemptions, KNOISE
    host_events = []
    for host, (gaps, preempted) in sorted(diag.items()):
        # (their sources stamp whole microseconds)
        host_events += [(g0, g1, "host stall", f"host stall {(g1 - g0) * 1e6:.0f} µs ({host})") for g0, g1 in gaps]
        host_events += [(t0, t1, "guest preemption", f"{who} preempted by {by} {(t1 - t0) * 1e6:.0f} µs ({host} cpu {cpu})")
                        for t0, t1, cpu, who, by in preempted]
        host_events += knoise.get(host, [])
    table = []  # (run, rep, ping, latency ns, end ns, labels)
    reps = collections.defaultdict(list)  # run: [(round trips, start, length, events by source)]
    drops = []
    for path, stem in spike_files(folder):
        rt = re.match(rf"^({re.escape(state)}-.*)-rtt-(\d+)$", stem)
        if not rt:
            continue
        run, rep = f"{rt.group(1)}-rtt", int(rt.group(2))
        tag = f"{rt.group(1)}-{rep}"
        start_ns, rows = None, []
        with open_spikes(path) as f:
            for line in f:
                if line.startswith("# start_realtime_ns="):
                    start_ns = int(line.split("=", 1)[1])
                    continue
                parts = line.rstrip().split(",")
                if len(parts) in (3, 4) and parts[1].isdigit() and parts[2].isdigit():
                    ping = parts[3] if len(parts) == 4 else "-"
                    rows.append((int(parts[2]), int(parts[1]), ping))
        if start_ns is None:
            continue
        rows.sort(key=lambda r: -r[0])
        top = rows[:TOP_RTT]
        events = []
        for who in ("ping", "pong"):
            events += stall_events(os.path.join(folder, f"stalls-{who}-{tag}.csv"), who)
        for host in "ab":
            for base in (folder, run_dir):
                ring = os.path.join(base, f"{host}-rings-{tag}.log")
                if os.path.exists(ring):
                    ring_events, dropped = read_rings(ring, host)
                    events += ring_events
                    if dropped:
                        drops.append(f"{host}'s event log dropped {dropped:,} events in {tag} (full rings): an overlap there may be missing")
                    break
        start = start_ns / 1e9
        last = max([start + (due + lat) / 1e9 for lat, due, _ in rows] + [e[1] for e in events if e[1] >= start] + [start])
        length = run_length(folder, tag, start, last)
        # what the run's window holds, for the baseline
        window = [e for e in events + host_events if e[1] >= start and e[0] <= start + length]
        sources = by_source(window)
        trips = []
        for lat, due, ping in top:
            t0, t1 = start + due / 1e9, start + (due + lat) / 1e9
            trips.append((t0, t1))
            labels = sorted({e[3] for src in sources.values() for e in overlaps(src, t0, t1)})
            table.append((run, rep, ping, lat, start_ns + due + lat, labels))
        reps[run].append((trips, start, length, sources))
    if not table:
        return
    print(f"\n## The {TOP_RTT} largest round trips of each rep and every event overlapping them, {state}\n")
    print("A round trip is [sent, echoed] on the wall clock; an event overlaps it if its interval does (an IPI, a")
    print("compaction or a page migration is an instant). Its phase is its echo's, in the second and in 30 s.\n")
    print("| run | rep | ping | round trip (µs) | mod 1 s (ms) | mod 30 s (s) | overlapping events |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for run, rep, ping, lat, end_ns, labels in sorted(table, key=lambda r: (r[0], r[1], -r[3])):
        print(f"| {run} | {rep} | {ping} | {lat / 1e3:.1f} | {end_ns % 1_000_000_000 / 1e6:.3f} | "
              f"{end_ns % 30_000_000_000 / 1e9:.3f} | {'; '.join(labels) or 'none'} |")
    for note in drops:
        print(f"\n{note}.")
    print(f"\n## Overlaps against chance ({SHIFTS:,} circular shifts of each rep's round trips, seed {SEED}), {state}\n")
    print("Each rep's round trips shifted together by a random offset within its run (its round trips' sum), so")
    print("their spacing and every source's own timing stay; p is the share of shifts overlapping at least as many.\n")
    print("| run | source | of the largest round trips | expected by chance | p |")
    print("| --- | --- | --- | --- | --- |")
    for run, run_reps in sorted(reps.items()):
        names = sorted({src for _, _, _, sources in run_reps for src in sources})
        for src, (observed, of, expected, p) in sorted(chance(run_reps, names).items(), key=lambda kv: (kv[1][3], kv[0])):
            print(f"| {run} | {src} | {observed} of {of} | {expected:.2f} | {p:.3f} |")


if __name__ == "__main__":
    main()
