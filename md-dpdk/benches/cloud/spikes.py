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

The match across hosts is on their wall clocks: where chrony's tracking was
logged around each mode (vm.sh timesync, <host>-timesync-*.log), a note gives
each host's largest offset from its source and that source's own bound
against UTC, and their sum, how far apart the two clocks may be.
"""
import collections
import csv
import glob
import gzip
import os
import re
import sys

# the threads that busy-spin on traced CPUs: the dedicated agents, the shared
# network agent ("[sender, receiver]", which the kernel cuts to 15 characters),
# SHARED's one agent (the media driver's main thread; under the other modes the
# conductor, on the untraced housekeeping CPU) and the clients
BUSY = re.compile(r"^(sender|receiver|\[sender, |\[conductor, |media_driver|rtt|load|tput)")
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
    for line in lines if anchor else []:
        m = EVENT.search(line)
        if not m or m.group(3) != "sched_switch":
            continue
        s = SWITCH.search(m.group(4))
        if not s:
            continue
        cpu, ts = int(m.group(1)), anchor[1] - (anchor[0] - float(m.group(2)))
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


def main():
    folder = sys.argv[1]
    threshold = float(sys.argv[2]) if len(sys.argv) > 2 else 200.0
    state = os.path.basename(os.path.normpath(folder))
    run_dir = os.path.dirname(os.path.normpath(folder))
    diag, blind = {}, {}
    for host in "ab":
        for path in (os.path.join(run_dir, f"{host}-diag-{state}.log"), os.path.join(folder, f"{host}-diag-{state}.log")):
            if os.path.exists(path):
                diag[host] = read_diag(path)
                blind[host] = spinner_blind(path)
                break
    if not diag:
        sys.exit(f"no {state} diagnostics (a-diag-{state}.log) next to {folder}")
    # intervals sorted by start, for a binary search: anything overlapping
    # [t0, t1] starts before t1 and, as none here lasts over a second, after
    # t0 - 1 s
    import bisect
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
                if len(parts) == 3 and parts[1].isdigit():
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

if __name__ == "__main__":
    main()
