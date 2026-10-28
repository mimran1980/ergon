#!/usr/bin/env python3
"""A point's events on one host, as transport-format.md's events file has them.

transport_events.py <events log> [<stalls file> <cpu> <who>]
    prints the events file (kind,start_ns,end_ns,cpu,detail) and its notes

transport_collect.py calls events() for every point and host: the log is what
vm.sh (or local-vm.sh) events-stop printed (host.sh events_stop), in sections:

  == clock      "<CLOCK_MONOTONIC ns> <CLOCK_REALTIME ns>" at the start and the end
  == notes      "events: ..." lines: what ran, and what this kernel lacks
  == gapspin <cpu>  gapspin.c's gaps on a hot CPU the point left idle
  == trace      the tracefs instance's events on the hot CPUs (CLOCK_MONOTONIC)
  == osnoise    osnoise's noise events, where the kernel has the tracer
  == samples    the sampler's lines, one a second (host.sh events_sampler)

Every time becomes CLOCK_REALTIME ns: the trace's through the two clock
readings (a line between them). Rows, each on a hot CPU unless it says -:

  irq          an IRQ handler, entry to exit (x86's local timer and other
               vectors too)
  softirq      a softirq, entry to exit
  ipi          an IPI's handler (x86's reschedule, call-function and irq_work
               vectors; arm64's ipi_entry to ipi_exit)
  page-fault   a page fault (x86's exceptions tracepoints: an instant; else
               the kprobe on handle_mm_fault to its return)
  preempt      the CPU's own busy thread (the one the trace shows on it most)
               off it: switched out to its return, what ran instead named
  host-pause   a gap spinner's gap no traced switch on its CPU explains: the
               hypervisor did not run the vCPU
  nak, retransmit   the media driver's NAKs (sent and received) and
               retransmits in a second (-)
  tc-drop      packets netem dropped in a second, every data NIC (-)
  tcp-rto, tcp-tlp, tcp-dsack, tcp-retrans   the host's TCP RTOs,
               tail-loss probes, DSACKs received and retransmitted segments
               in a second (-)
  stall        the measuring loop's own stall (its stalls file: the point's
               binary on host a), on the CPU given

A note says what could not be recorded ("... unavailable: why"), a traced
CPU whose buffer overran, and a log that is missing or holds no trace.
"""
import bisect
import collections
import functools
import csv
import gzip
import re
import sys

EVENT = re.compile(r"\[(\d+)\].*?\s(\d+\.\d+): ([\w]+): ?(.*)$")
SWITCH = re.compile(r"prev_comm=(.+?) prev_pid=(\d+) prev_prio=\d+ prev_state=(\S+) ==> next_comm=(.+?) next_pid=(\d+)")
NOISE = re.compile(r"^\s*(?:(.*):(\d+) )?start (\d+)\.(\d+) duration (\d+) ns")
# x86's irq_vectors that are IPIs (arch/x86/include/asm/trace/irq_vectors.h)
IPI_VECTORS = {"reschedule", "call_function", "call_function_single", "irq_work"}
COUNTERS = {
    "tcp-retrans": ("tcp", ("RetransSegs",)),
    "tcp-rto": ("tcp", ("TCPTimeouts",)),
    "tcp-tlp": ("tcp", ("TCPLossProbes",)),
    "tcp-dsack": ("tcp", ("TCPDSACKRecv", "TCPDSACKOfoRecv")),
    "nak": ("aeron", ("naks-sent", "naks-received")),
    "retransmit": ("aeron", ("retransmits-sent",)),
    # a media driver agent's duty cycles over the driver's threshold
    # (counters loss' info- keys): a stall of the driver's loop, placed in time
    "agent-cycle": ("aeron", ("info-cycle-conductor-exceeded", "info-cycle-sender-exceeded",
                              "info-cycle-receiver-exceeded")),
}
# a driver agent's longest duty cycle (since its driver started): a second
# where it rose holds the stall that raised it ("<agent> <ns>")
AGENT_MAX = ("conductor", "sender", "receiver")
SPINNER = "gapspin"
TASK = re.compile(r"^\s*(.+?)-(\d+)\s+(?:\(\s*\S+\)\s+)?\[\d+\]")
# the kinds that happen on one CPU (the rest are the host's, per second)
ON_CPU = {"irq", "softirq", "ipi", "page-fault", "preempt"}


def read_text(path):
    """A log's text, gzipped or not; None when missing or empty."""
    try:
        if path.endswith(".gz"):
            with gzip.open(path, "rt", errors="replace") as f:
                text = f.read()
        else:
            with open(path, errors="replace") as f:
                text = f.read()
    except (OSError, EOFError):
        return None
    return text or None


def sections(text):
    """{section name: [lines]} (a gapspin section's name holds its CPU)."""
    out, name = collections.defaultdict(list), None
    for line in text.splitlines():
        if line.startswith("== "):
            name = line[3:].strip()
            out[name]
            continue
        if name is not None:
            out[name].append(line)
    return out


def wall_clock(lines):
    """The trace's clock (mono, s) onto CLOCK_REALTIME ns: through the first
    and last reading (drift between them cancels), one an offset; None without."""
    pts = []
    for line in lines:
        w = line.split()
        if len(w) == 2 and w[0].isdigit() and w[1].isdigit():
            pts.append((int(w[0]), int(w[1])))
    if not pts:
        return None
    (m0, r0), (m1, r1) = pts[0], pts[-1]
    if m1 > m0:
        rate = (r1 - r0) / (m1 - m0)
        return lambda s: r0 + int(round((s * 1e9 - m0) * rate))
    return lambda s: r0 + int(round(s * 1e9 - m0))


def trace_events(lines, wall):
    """(rows, notes) from the trace section."""
    rows, notes = [], []
    parsed = []
    for line in lines:
        if line.startswith("# overrun "):
            w = line.split()
            notes.append(f"trace overran on {w[2]}: {w[3]} events lost (its buffer full): an event there may be missing")
            continue
        m = EVENT.search(line)
        if m:
            task = TASK.match(line)
            parsed.append((int(m.group(1)), float(m.group(2)), m.group(3), m.group(4), task.group(2) if task else None))
    if not parsed:
        return rows, notes, {}
    t_first, t_last = min(p[1] for p in parsed), max(p[1] for p in parsed)
    # who each CPU's busy thread is: the task the switches show on it longest
    on_cpu, since, ran = {}, {}, collections.defaultdict(collections.Counter)
    names = {}
    for cpu, ts, ev, rest, _ in parsed:
        if ev != "sched_switch":
            continue
        s = SWITCH.search(rest)
        if not s:
            continue
        prev, ppid, _, nxt, npid = s.groups()
        names[ppid], names[npid] = prev, nxt
        ran[cpu][ppid] += ts - since.get(cpu, t_first)
        on_cpu[cpu], since[cpu] = npid, ts
    for cpu, pid in on_cpu.items():
        ran[cpu][pid] += t_last - since[cpu]
    owner = {}
    for cpu, c in ran.items():
        pid, secs = c.most_common(1)[0]
        if pid != "0" and secs > (t_last - t_first) / 4:
            owner[cpu] = pid
    away = {}  # cpu: (start, prev state, [what ran])
    opened = {}  # (cpu, kind, key): (start, detail)
    for cpu, ts, ev, rest, task in parsed:
        t = wall(ts)
        if ev == "sched_switch":
            s = SWITCH.search(rest)
            if not s:
                continue
            prev, ppid, state, nxt, npid = s.groups()
            own = owner.get(cpu)
            if own is None:
                continue
            if ppid == own and npid != own:
                away[cpu] = (t, state, [nxt])
            elif cpu in away and npid != own:
                if len(away[cpu][2]) < 4 and nxt not in away[cpu][2]:
                    away[cpu][2].append(nxt)
            elif npid == own and cpu in away:
                t0, state0, ran_by = away.pop(cpu)
                how = "preempted" if state0.startswith("R") else f"blocked (state {state0})"
                rows.append(("preempt", t0, t, cpu, f"{names.get(own, '?')}:{own} {how}, ran {' '.join(ran_by)}"))
        elif ev in ("irq_handler_entry", "softirq_entry", "ipi_entry") or ev.endswith("_entry") and "vector=" in rest:
            if ev == "irq_handler_entry":
                n = re.search(r"name=(\S+)", rest)
                kind, key, detail = "irq", "irq", n.group(1) if n else rest
            elif ev == "softirq_entry":
                n = re.search(r"action=(\w+)", rest)
                kind, key, detail = "softirq", "softirq", n.group(1) if n else rest
            elif ev == "ipi_entry":
                kind, key, detail = "ipi", "ipi", rest.strip("() ") or "ipi"
            else:
                vec = ev[:-len("_entry")]
                kind = "ipi" if vec in IPI_VECTORS else "irq"
                key, detail = vec, vec
            opened.setdefault((cpu, kind, key), []).append((t, detail))
        elif ev in ("irq_handler_exit", "softirq_exit", "ipi_exit") or ev.endswith("_exit") and "vector=" in rest:
            if ev == "irq_handler_exit":
                kind, key = "irq", "irq"
            elif ev == "softirq_exit":
                kind, key = "softirq", "softirq"
            elif ev == "ipi_exit":
                kind, key = "ipi", "ipi"
            else:
                vec = ev[:-len("_exit")]
                kind, key = ("ipi" if vec in IPI_VECTORS else "irq"), vec
            stack = opened.get((cpu, kind, key))
            if stack:
                t0, detail = stack.pop()
                rows.append((kind, t0, t, cpu, detail))
        elif ev in ("page_fault_user", "page_fault_kernel"):
            if task != owner.get(cpu):
                continue
            a = re.search(r"address=(\S+)", rest)
            rows.append(("page-fault", t, t, cpu, f"{ev[len('page_fault_'):]} {a.group(1) if a else ''}".strip()))
        elif ev.startswith("ergon_pf_"):
            if task != owner.get(cpu):
                continue
            if ev.endswith("_ret"):
                stack = opened.get((cpu, "page-fault", "pf"))
                if stack:
                    t0, _ = stack.pop()
                    rows.append(("page-fault", t0, t, cpu, "handle_mm_fault"))
            else:
                opened.setdefault((cpu, "page-fault", "pf"), []).append((t, "handle_mm_fault"))
    end = wall(t_last)
    for cpu, (t0, state0, ran_by) in away.items():
        rows.append(("preempt", t0, end, cpu, f"{names.get(owner[cpu], '?')}:{owner[cpu]} off the CPU to the trace's end, "
                     f"ran {' '.join(ran_by)}"))
    for (cpu, kind, _), stack in opened.items():
        for t0, detail in stack:  # no exit traced: an instant
            rows.append((kind, t0, t0, cpu, detail))
    return rows, notes, {cpu: names.get(pid, "?") for cpu, pid in owner.items()}


def osnoise_events(lines, wall):
    """osnoise's noise events: each from its trace time less its duration."""
    rows = []
    kinds = {"irq_noise": "irq", "softirq_noise": "softirq", "nmi_noise": "irq", "thread_noise": "preempt"}
    for line in lines:
        m = EVENT.search(line)
        if not m or m.group(3) not in kinds:
            continue
        n = NOISE.match(m.group(4))
        if not n:
            continue
        t1 = wall(float(m.group(2)))
        dur = int(n.group(5))
        who = (n.group(1) or "NMI").strip()
        if m.group(3) == "thread_noise" and (who.startswith(SPINNER) or who.startswith("swapper")):
            continue
        rows.append((kinds[m.group(3)], t1 - dur, t1, int(m.group(1)), f"osnoise {m.group(3)} {who}"))
    return rows


def spinner_events(gaps, traced, complete=None, min_ns=20_000):
    """A gap spinner's gaps the guest does not explain: host-pause, its
    length what no traced event on its CPU covers (a switch away from the
    spinner, an IRQ, a softirq, an IPI, a page fault), when that is at least
    min_ns and half the gap. A traced trace overrun on the CPU is not checked
    here: the note says so."""
    rows = []
    for cpu, lines in gaps.items():
        own = sorted((a, b) for k, a, b, c, d in traced if c == cpu)
        starts = [a for a, _ in own]
        longest = max((b - a for a, b in own), default=0)
        for line in lines:
            w = line.split()
            if len(w) != 2 or line.startswith("#"):
                continue
            try:
                end, us = int(round(float(w[0]) * 1e9)), float(w[1])
            except ValueError:
                continue
            start = end - int(us * 1000)
            if start < (complete or {}).get(cpu, start):
                continue  # the trace there overwritten: the gap not judged
            covered, last = 0, start
            lo, hi = bisect.bisect_left(starts, start - longest), bisect.bisect_left(starts, end)
            for a, b in own[lo:hi]:
                a, b = max(a, last), min(b, end)
                if b > a:
                    covered += b - a
                    last = b
            left = (end - start) - covered
            if left >= min_ns and left * 2 >= end - start:
                rows.append(("host-pause", start, end, cpu, f"{left / 1e3:.1f} µs of a {us:.1f} µs spinner gap untraced"))
    return rows


def sample_events(lines):
    """Per-second counter growth: [(kind, start, end, None, count)]."""
    rows, prev = [], None
    for line in lines:
        f = dict(w.split("=", 1) for w in line.split() if "=" in w)
        if not f.get("t", "").isdigit():
            continue
        cur = {"t": int(f["t"]), "tc": {}, "tcp": {}, "aeron": {}}
        for nic in (f.get("tc") or "-").split(","):
            p = nic.split(":")
            if len(p) == 3 and p[2].isdigit():
                cur["tc"][p[0]] = int(p[2])
        for src in ("tcp", "aeron"):
            for kv in (f.get(src) or "-").split(";"):
                k, _, v = kv.partition("=")
                if v.lstrip("-").isdigit():
                    cur[src][k] = int(v)
        if prev is not None:
            t0, t1 = prev["t"], cur["t"]
            drop = sum(v - prev["tc"].get(n, v) for n, v in cur["tc"].items())
            if drop > 0:
                rows.append(("tc-drop", t0, t1, None, str(drop)))
            for kind, (src, keys) in COUNTERS.items():
                d = sum(cur[src].get(k, 0) - prev[src].get(k, cur[src].get(k, 0)) for k in keys)
                if d > 0:
                    rows.append((kind, t0, t1, None, str(d)))
            for agent in AGENT_MAX:
                k = f"info-cycle-{agent}-max-ns"
                if k in cur["aeron"] and k in prev["aeron"] and cur["aeron"][k] > prev["aeron"][k]:
                    rows.append(("agent-cycle-max", t0, t1, None, f"{agent} {cur['aeron'][k]}"))
        prev = cur
    return rows


def stall_events(path, cpu, who):
    """The measuring loop's stalls (stalls-<label>.csv: start_realtime_ns,
    then end_ns,gap_ns from it)."""
    rows = []
    text = read_text(path)
    if text is None:
        return rows
    base = None
    for line in text.splitlines():
        if line.startswith("# start_realtime_ns="):
            base = int(line.split("=", 1)[1])
            continue
        p = line.strip().split(",")
        if base is not None and len(p) == 2 and p[0].isdigit() and p[1].isdigit():
            end, gap = base + int(p[0]), int(p[1])
            rows.append(("stall", end - gap, end, cpu, f"{who} loop {gap / 1e3:.1f} µs"))
    return rows


def events(log_path, stalls=None):
    """(rows sorted by start, notes) for one host's point. stalls: (path,
    cpu, who) of the measuring loop's stalls file, or None."""
    rows, notes = log_events(log_path)
    rows, notes = list(rows), list(notes)
    if stalls:
        rows += stall_events(*stalls)
    return sorted(rows, key=lambda r: (r[1], r[2], r[0])), notes


@functools.lru_cache(maxsize=1)
def log_events(log_path):
    """events()'s rows and notes from the log alone (a one-host point's two
    runs share it: read once)."""
    rows, notes = [], []
    text = read_text(log_path) if log_path else None
    if text is None:
        notes.append("events unavailable: no events log for this host and point (events-start or -stop failed)")
    else:
        sec = sections(text)
        notes += [l[len("events: "):] for l in sec.get("notes", []) if l.startswith("events: ")]
        wall = wall_clock(sec.get("clock", []))
        trace = sec.get("trace", [])
        if wall is None:
            notes.append("trace unavailable: no clock readings, so its events have no wall-clock time")
        else:
            got, more, busy = trace_events(trace, wall)
            rows += got
            notes += more
            if not any(EVENT.search(l) for l in trace):
                notes.append("trace empty: no event recorded on the hot CPUs")
            rows += osnoise_events(sec.get("osnoise", []), wall)
        gaps = {int(k.split()[1]): v for k, v in sec.items() if k.startswith("gapspin ") and k.split()[1].isdigit()}
        rows += spinner_events(gaps, rows, complete_from(trace, wall) if wall else {})
        # what happened on a CPU no busy thread owned (idle, or a gap
        # spinner's) is not on the measured path: only its host pauses stay
        if wall is not None:
            rows = [r for r in rows if r[0] not in ON_CPU or (r[3] in busy and busy[r[3]] != SPINNER)]
        rows += sample_events(sec.get("samples", []))
    return tuple(rows), tuple(notes)


def complete_from(trace, wall):
    """{cpu: wall ns} for each CPU whose buffer overran: its trace's first
    event, before which (overwritten) nothing it shows is complete."""
    over = {int(l.split()[2][3:]) for l in trace if l.startswith("# overrun cpu")}
    first = {}
    for line in trace:
        m = EVENT.search(line)
        if m and int(m.group(1)) in over and int(m.group(1)) not in first:
            first[int(m.group(1))] = wall(float(m.group(2)))
    return first


def write(f, rows, notes):
    """The events file (transport-format.md) into the open file f, its notes
    as # lines first."""
    for n in notes:
        f.write(f"# note: {n}\n")
    w = csv.writer(f, lineterminator="\n")
    w.writerow(["kind", "start_ns", "end_ns", "cpu", "detail"])
    for kind, a, b, cpu, detail in rows:
        w.writerow([kind, a, b, "-" if cpu is None else cpu, detail])


def main():
    if len(sys.argv) not in (2, 5):
        sys.exit("usage: transport_events.py <events log> [<stalls file> <cpu> <who>]")
    stalls = (sys.argv[2], int(sys.argv[3]), sys.argv[4]) if len(sys.argv) == 5 else None
    rows, notes = events(sys.argv[1], stalls)
    write(sys.stdout, rows, notes)

if __name__ == "__main__":
    main()
