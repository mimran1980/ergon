#!/usr/bin/env python3
"""ergon-clock's benchmark (clock/bench, clockbench) on several hosts, as one comparison.

clockbench.py <run dir or CSV>...

Reads every clockbench CSV given, or found in each run directory given
(<node>-clockbench-<state>.csv, as lib.sh's clockbench_both saves them, from
any number of lab runs: c7i, c8g, c9g, a Mac's own run), each file one or more
runs (a run starts at `meta,tool,clockbench`; LAB_CLOCK_ONLY=1's full variant
is the LTO build's run then the build without LTO's). Prints Markdown:

- the hosts' comparison: each clock read's median cost (and the slowest
  batch's mean) on each host, one column a host, in its most isolated state
  (isolated, isolated-poll, pinned, boot), the LTO build, node a, with the
  instruction each host's ergon-clock ordered read took;
- the conversions alone, and ergon against quanta built without LTO;
- each clock's resolution, gaps and backwards steps, its cross-core
  inversions, its drift against CLOCK_MONOTONIC_RAW and against the
  NTP-steered CLOCK_MONOTONIC, the calibrations in
  fresh processes (a calibration that fell back has no counter rate: why,
  instead), the gates (with the bare row each compared against) and
  comparisons, per host;
- every run (node, state, build: what ergon-clock chose, its ordered read,
  and the default calibration's choice and why), with any clock that failed
  its check;
- each read's longest gap in each run, where it is 10 us or more,
  attributed where the run directory has the state's diagnostics
  (<node>-diag-<state>.log, vm.sh diag-fetch): a host stall (the gap spinner
  stalled across it: its stalls are on the wall clock exactly), else a
  preemption of the benchmark (the trace switched it out on its CPU) or an
  interrupt on its CPU within TRACE_SLACK_S of the gap (the trace reaches the
  wall clock through diag-fetch's /proc/uptime anchor, of 10 ms resolution),
  else unexplained;
- ergon-clock's discipline of the counter to CLOCK_REALTIME (clockbench
  --discipline: a run's meta mode is discipline; in a run directory
  <node>-discipline-<state>.csv, as lib.sh's discipline_both saves them,
  LAB_DISCIPLINE_SECONDS), per host and state: the absolute residual,
  CLOCK_REALTIME less the clock, p50 / p99 / max at the owner's samples and
  from the verifier (its interrupted brackets apart), where the verifier's
  max fell (after which update, its action and flags), what the start
  update did (its forward step, by design, apart), the samples rejected,
  the clock's steps after the start and the wall clock's steps, the
  update's cost; and
  chrony's tracking before and after it (<node>-discipline-<state>.log,
  vm.sh discipline's "== chrony before" and "== chrony after").

A run directory may hold clockbench runs, discipline runs or both; given
none of either, it is an error.
"""
import collections
import glob
import os
import re
import sys

STATE_ORDER = ["isolated", "isolated-poll", "pinned", "boot"]
# what drift is measured against (a drift record's last field; none, in an
# older CSV, is the raw clock): the key, the clock, what drift from it means
DRIFT_REFERENCES = [
    ("raw", "CLOCK_MONOTONIC_RAW",
     "the kernel's unadjusted scaling of its clock source, which ergon-clock calibrates against; where that source "
     "is the counter, a counter clock's drift from it is how its conversion differs from the kernel's, not its "
     "accuracy"),
    ("monotonic", "CLOCK_MONOTONIC",
     "steered by NTP (chrony) to its sources' rate, so independent of the counter's oscillator; a clock at the "
     "counter's raw rate drifts from it by the oscillator's error as NTP sees it, plus any slewing during the run"),
]
# how far a traced event may be from a gap and still be matched with it: the
# resolution of the anchor that puts the trace on the wall clock
TRACE_SLACK_S = 0.01
EVENT = re.compile(r"\[(\d+)\].*?\s(\d+\.\d+): (\w+): (.*)")
SWITCH = re.compile(r"prev_comm=(.+?) prev_pid=\d+ prev_prio=\d+ prev_state=(\S+) ==> next_comm=(.+?) next_pid=")


class Run:
    """One clockbench run: its metadata and records by kind."""

    def __init__(self, path, node):
        self.path, self.node = path, node
        self.meta = {}
        self.rows = collections.defaultdict(list)

    @property
    def host(self):
        label = self.meta.get("label", "-")
        return label.split(" ")[0] if label not in ("", "-") else self.meta.get("cpu_model", "-")

    @property
    def state(self):
        label = self.meta.get("label", "-").split(" ")
        return label[1] if len(label) > 1 else "-"

    @property
    def build(self):
        return self.meta.get("build", "-")

    def by_name(self, kind):
        return {r[0]: r for r in self.rows[kind]}


def read_runs(path):
    """Every run in one clockbench CSV."""
    node = os.path.basename(path).split("-", 1)[0]
    node = node if node in ("a", "b") else "-"
    runs = []
    for line in open(path, errors="replace"):
        f = line.rstrip("\n").split(",")
        if f[0] == "meta" and len(f) >= 3 and f[1] == "tool":
            runs.append(Run(path, node))
        if not runs or len(f) < 2:
            continue
        if f[0] == "meta":
            runs[-1].meta[f[1]] = ",".join(f[2:])
        elif f[0] in ("check", "cost", "delta", "gap", "skew", "convert", "calib", "drift", "gate", "cmp",
                      "discipline", "truth", "discipline_summary"):
            runs[-1].rows[f[0]].append(f[1:])
    return runs


def inputs(args):
    """(csv path, the run directory it is in) for each CSV given or found."""
    out = []
    for a in args:
        if os.path.isdir(a):
            out += [(p, a) for p in sorted(glob.glob(os.path.join(a, "*-clockbench-*.csv")))]
            out += [(p, a) for p in sorted(glob.glob(os.path.join(a, "*-discipline-*.csv")))]
        else:
            out.append((a, os.path.dirname(a) or "."))
    return out


def num(v):
    try:
        return float(v)
    except (TypeError, ValueError):
        return None


def fmt(v, digits=2):
    return "-" if v is None else f"{v:.{digits}f}"


def table(header, rows):
    print("| " + " | ".join(header) + " |")
    print("|" + "|".join("---" for _ in header) + "|")
    for r in rows:
        print("| " + " | ".join(r) + " |")
    print()


def pick(runs, build="lto"):
    """Per host, the run of its most isolated state, of this build, node a
    first: {host: run}. A run with no build named (clockbench by hand) is the
    release profile's, the workspace's LTO one."""
    best = {}
    for r in runs:
        if r.build != build and not (build == "lto" and r.build == "release"):
            continue
        rank = (STATE_ORDER.index(r.state) if r.state in STATE_ORDER else len(STATE_ORDER), r.node != "a")
        if r.host not in best or rank < best[r.host][0]:
            best[r.host] = (rank, r)
    return {h: v[1] for h, v in sorted(best.items())}


def ordered_read(run):
    """The instruction the run's ergon-clock ordered read took, as its report
    names it, or -. The report writes its spaces as _ (isb;_mrs_cntvct_el0):
    those before an instruction or register name are spaces again, the
    register's own (cntvct_el0) are not."""
    return re.sub(r"_(?=mrs|cntvct)", " ", run.meta.get("ergon_ordered_read", "-"))


def calib_cell(c):
    """A calib record's cell: the times, and the counter rate's spread with its
    source, or, where no process chose the counter (ergon-clock's fallback,
    which has no counter rate), why."""
    rate = f"{fmt(num(c[8]), 3)} ppm" if num(c[8]) is not None else "no counter rate"
    return f"{c[2]} / {c[3]} / {c[4]} ms; {rate} ({c[9]})"


def drift_reference(d):
    """A drift record's reference key (its last field; the raw clock in an older CSV)."""
    return d[5] if len(d) > 5 else "raw"


def names(runs, kind):
    """Row names of a kind, in first-seen order."""
    seen = []
    for r in runs:
        for row in r.rows[kind]:
            if row[0] not in seen:
                seen.append(row[0])
    return seen


def comparison(chosen):
    hosts = list(chosen)
    print("## Read cost, ns: median of the batch means (the slowest batch's mean)\n")
    print("Each host in its most isolated state, the LTO build, node a: "
          + "; ".join(f"{h}: {chosen[h].state}, node {chosen[h].node}, {chosen[h].meta.get('arch', '-')}, "
                      f"{chosen[h].meta.get('cpu_model', '-')}" for h in hosts) + ".\n")
    print("ergon-clock's ordered read (`ergon_ticks_ordered`, `ergon_now_ns_ordered`): "
          + "; ".join(f"{h} `{ordered_read(chosen[h])}`" for h in hosts) + ".\n")
    rows = []
    for name in names(chosen.values(), "check"):
        cells = []
        for h in hosts:
            run = chosen[h]
            cost, check = run.by_name("cost").get(name), run.by_name("check").get(name)
            if cost:
                cells.append(f"{fmt(num(cost[4]))} ({fmt(num(cost[6]))})")
            elif check and check[3] == "fail":
                cells.append("FAILED its check")
            else:
                cells.append("-")
        rows.append([f"`{name}`"] + cells)
    table(["read"] + hosts, rows)
    print("## Conversion alone, ns: median (slowest batch)\n")
    rows = []
    for name in names(chosen.values(), "convert"):
        rows.append([f"`{name}`"] + [
            (lambda c: f"{fmt(num(c[3]))} ({fmt(num(c[5]))})" if c else "-")(chosen[h].by_name("convert").get(name))
            for h in hosts])
    table(["conversion"] + hosts, rows)


def nolto(runs):
    chosen = pick(runs, "nolto")
    if not chosen:
        return
    hosts = list(chosen)
    print("## Built without LTO: ergon-clock's reads are #[inline], quanta's are calls\n")
    rows = []
    for name in ("ergon_ticks", "quanta_raw", "ergon_now_ns", "quanta_now_ns", "quanta_now", "ergon_now_ns_ordered",
                 "convert_ergon_ticks_to_ns", "convert_quanta_delta_as_nanos"):
        cells = []
        for h in hosts:
            c = chosen[h].by_name("cost").get(name) or chosen[h].by_name("convert").get(name)
            if c and name.startswith("convert_"):
                cells.append(f"{fmt(num(c[3]))} ({fmt(num(c[5]))})")
            elif c:
                cells.append(f"{fmt(num(c[4]))} ({fmt(num(c[6]))})")
            else:
                cells.append("-")
        rows.append([f"`{name}`"] + cells)
    table(["read, ns"] + hosts, rows)


def per_host(chosen):
    hosts = list(chosen)
    print("## Gaps between consecutive reads, ns: resolution, p50, p99.99, max (backwards steps)\n")
    rows = []
    for name in names(chosen.values(), "delta"):
        cells = []
        for h in hosts:
            d = chosen[h].by_name("delta").get(name)
            cells.append("-" if not d else
                         f"{fmt(num(d[2]), 1)}, {fmt(num(d[3]), 1)}, {fmt(num(d[5]), 1)}, {fmt(num(d[6]), 1)} ({d[8]})")
        rows.append([f"`{name}`"] + cells)
    table(["read"] + hosts, rows)
    print("## Across two threads on two CPUs: second reads lower than the first, of the handoffs (largest step back, ns; "
          "offset b over a, ns)\n")
    rows = []
    for name in names(chosen.values(), "skew"):
        cells = []
        for h in hosts:
            s = chosen[h].by_name("skew").get(name)
            cells.append("-" if not s else
                         f"{int(s[5]) + int(s[6])} of {2 * int(s[1])} ({fmt(num(s[7]), 1)}; {fmt(num(s[4]), 1)})")
        rows.append([f"`{name}`"] + cells)
    table(["read"] + hosts, rows)
    for key, clock, about in DRIFT_REFERENCES:
        rows = []
        for name in names(chosen.values(), "drift"):
            cells = []
            for h in hosts:
                pts = [d for d in chosen[h].rows["drift"] if d[0] == name and drift_reference(d) == key]
                if not pts:
                    cells.append("-")
                    continue
                last = max(pts, key=lambda d: int(d[1]))
                worst = max(abs(num(d[3]) or 0) for d in pts)
                cells.append(f"{fmt(num(last[4]), 3)} at {last[1]} s ({worst:.0f})")
            if any(c != "-" for c in cells):
                rows.append([f"`{name}`"] + cells)
        if rows:
            print(f"## Drift against {clock}: ppm at the last checkpoint (largest error, ns)\n")
            print(f"{clock}: {about}.\n")
            table(["clock"] + hosts, rows)
    print("## Calibration in fresh processes: ms min / median / max; the rate's spread, ppm\n")
    rows = []
    for impl in names(chosen.values(), "calib"):
        cells = []
        for h in hosts:
            c = chosen[h].by_name("calib").get(impl)
            cells.append("-" if not c else calib_cell(c))
        rows.append([impl] + cells)
    if rows:
        table(["calibration"] + hosts, rows)
    print("## Gates and comparisons, ns\n")
    rows = []
    for name in names(chosen.values(), "gate"):
        rows.append([f"gate `{name}`"] + [
            (lambda g: "-" if not g else f"{g[1]} (limit {g[2]}): {g[3]}" + (f", against `{g[5]}`" if len(g) > 5 else ""))(
                chosen[h].by_name("gate").get(name)) for h in hosts])
    for name in names(chosen.values(), "cmp"):
        rows.append([f"`{name}`"] + [(lambda c: c[1] if c else "-")(chosen[h].by_name("cmp").get(name)) for h in hosts])
    table(["", *hosts], rows)


def every_run(runs):
    print("## Every run\n")
    rows = []
    for r in runs:
        m = r.meta
        failed = [c[0] for c in r.rows["check"] if c[3] == "fail"]
        rows.append([r.host, r.node, r.state, r.build, m.get("arch", "-"), m.get("ergon_clocksource", "-"),
                     f"{m.get('ergon_kind', '-')} {m.get('ergon_rate_source', '-')} {m.get('ergon_hz', '-')} Hz",
                     f"{m.get('default_kind', '-')} ({m.get('default_fallback', '-')})", m.get("ergon_ecv", "-"),
                     ordered_read(r), ", ".join(failed) or "none"])
    table(["host", "node", "state", "build", "arch", "clocksource", "ergon-clock (cross-check)",
           "default (kernel)", "ECV", "ordered read", "failed checks"], rows)


def diag(path):
    """(spinner gaps [(start s, end s)], [(t s, cpu, kind, detail)]) on the wall clock, from vm.sh diag-fetch."""
    gaps, events, lines, anchor, section = [], [], [], None, None
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
    for line in lines if anchor else []:
        m = EVENT.search(line)
        if not m:
            continue
        cpu, t = int(m.group(1)), anchor[1] - (anchor[0] - float(m.group(2)))
        if m.group(3) == "sched_switch":
            s = SWITCH.search(m.group(4))
            if s and s.group(1).startswith("clockbench") and s.group(2).startswith("R"):
                events.append((t, cpu, "preempted", f"by {s.group(3)}"))
        elif m.group(3) == "irq_handler_entry":
            events.append((t, cpu, "interrupt", m.group(4).strip()))
    return gaps, events


def attributed(runs):
    # each read's longest gap in each run (clockbench lists the longest 20 of
    # 10 us or more, longest first)
    found = []
    for r in runs:
        seen = set()
        for g in r.rows["gap"]:
            if g[0] not in seen:
                seen.add(g[0])
                found.append((r, g))
    if not found:
        return
    print("## Each read's longest gap (10 us or more), attributed\n")
    print(f"A traced preemption or interrupt is matched within {TRACE_SLACK_S * 1e3:.0f} ms of a gap (the trace's "
          "wall-clock anchor, /proc/uptime, has that resolution); the gap spinner's stalls exactly.\n")
    rows = []
    for r, (name, at, gap) in found:
        end = int(at) / 1e9
        start = end - float(gap) / 1e9
        path = os.path.join(os.path.dirname(r.path), f"{r.node}-diag-{r.state}.log")
        if not os.path.exists(path):
            cause = "no diagnostics"
        else:
            spins, events = diag(path)
            cpu = int(r.meta.get("cpu", "-1")) if r.meta.get("cpu", "-").isdigit() else -1
            mine = [e for e in events if e[1] == cpu and start - TRACE_SLACK_S <= e[0] <= end + TRACE_SLACK_S]
            if any(s0 < end and s1 > start for s0, s1 in spins):
                cause = "host stall (the gap spinner stalled across it)"
            elif mine:
                kinds = collections.Counter(e[2] for e in mine)
                cause = "; ".join(f"{k} x{n} ({next(e[3] for e in mine if e[2] == k)})" for k, n in kinds.items())
            else:
                cause = "unexplained"
        rows.append([r.host, r.node, r.state, r.build, f"`{name}`", f"{float(gap) / 1000:.1f}", cause])
    table(["host", "node", "state", "build", "read", "gap, us", "cause"], rows)


# the discipline's flags (ergon_clock::advanced::discipline::flags), by bit
DISCIPLINE_FLAGS = ["rejected", "holdover", "slew-limited", "converging", "wall-step", "stepped", "start",
                    "leap-pending", "unsynchronised", "fast-slew-or-jump", "d-glitch"]


def flag_names(flags):
    """A discipline record's flags field as names, or "no flags"."""
    try:
        bits = int(flags)
    except ValueError:
        return flags
    names = [n for i, n in enumerate(DISCIPLINE_FLAGS) if bits >> i & 1]
    return ", ".join(names) or "no flags"


def spread(run, series):
    """A discipline_summary series as "p50 / p99 / max", or -."""
    for row in run.rows["discipline_summary"]:
        if row[0] == series and len(row) >= 6:
            return f"{row[2]} / {row[3]} / {row[5]}"
    return "-"


def verifier_max(run):
    """Where the verifier's largest residual fell (its uninterrupted
    brackets: at most ten times the median width, as clockbench counts
    them), after which update: "<t> s, after the update at <u> s
    (<action>; <flags>)", or -."""
    truths = [(float(t[0]), int(t[1]), int(t[2])) for t in run.rows["truth"] if len(t) >= 3]
    if not truths:
        return "-"
    widths = sorted(w for _, _, w in truths)
    wide = max(widths[(len(widths) - 1) * 500 // 1000] * 10, 1)
    at, residual, _ = max(((t, r, w) for t, r, w in truths if w <= wide), key=lambda x: abs(x[1]))
    before = [u for u in run.rows["discipline"] if len(u) >= 9 and float(u[0]) <= at]
    if not before:
        return f"{at:.2f} s, before the first update"
    u = before[-1]
    return f"{at:.2f} s, after the update at {float(u[0]):.2f} s ({u[1]}; {flag_names(u[8])})"


def interrupted(run):
    """The verifier's interrupted brackets: how many, with the widest."""
    for row in run.rows["discipline_summary"]:
        if row[0] == "verifier_interrupted_width_ns" and len(row) >= 6:
            return f"{row[1]} (widest {row[5]})"
    return "0"


def chrony(path):
    """{"before": cell, "after": cell} from a vm.sh discipline log's chrony
    sections (chronyc -n tracking): the reference, its stratum, the system
    time's offset, the last and RMS offsets (ns), the frequency, the skew and
    the leap status."""
    sections, section = {}, None
    for line in open(path, errors="replace") if os.path.exists(path) else []:
        line = line.rstrip("\n")
        if line.startswith("== chrony "):
            section = line[len("== chrony "):]
            sections[section] = {}
        elif line.startswith("== ") or not line.strip():
            section = None
        elif section and ":" in line:
            key, _, value = line.partition(":")
            sections[section][key.strip()] = value.strip()

    def ns(value):
        """chrony's "0.000000123 seconds fast of NTP time" or "+0.000000045 seconds" as signed ns."""
        f = value.split()
        try:
            v = float(f[0]) * 1e9
        except (IndexError, ValueError):
            return value
        if "slow" in f:
            v = -v
        return f"{v:+.0f} ns" if "fast" in f or "slow" in f or f[0][0] in "+-" else f"{v:.0f} ns"

    cells = {}
    for name, t in sections.items():
        ref = t.get("Reference ID", "-")
        ref = ref[ref.find("(") + 1:ref.rfind(")")] if "(" in ref else ref
        parts = []
        for key, label, conv in (("System time", "system time", ns), ("Last offset", "last offset", ns),
                                 ("RMS offset", "RMS offset", ns), ("Frequency", "frequency", str),
                                 ("Skew", "skew", str), ("Leap status", "leap", str)):
            if key in t:
                parts.append(f"{label} {conv(t[key])}")
        stratum = f" (stratum {t['Stratum']})" if "Stratum" in t else ""
        cells[name] = f"{ref}{stratum}: " + ", ".join(parts) if parts else f"{ref}{stratum}"
    return cells


def discipline(runs):
    """The discipline's residual per host and state, and chrony around it."""
    runs = sorted(runs, key=lambda r: (r.host, r.node, STATE_ORDER.index(r.state) if r.state in STATE_ORDER else 99))
    print("## ergon-clock's discipline to CLOCK_REALTIME (clockbench --discipline): the residual, ns\n")
    print("The absolute residual, CLOCK_REALTIME less the disciplined clock, p50 / p99 / max: at the owner's samples, "
          "one a second (what each update found before it re-anchored the clock; held updates left out), and from the "
          "verifier, a follower's copy of the clock bracketing CLOCK_REALTIME between two of its ordered reads 100 times "
          "a second (a bracket over ten times the median width was interrupted: counted apart, with the widest). The "
          "verifier's max is placed after the last update before it, with that update's action and flags. `start`: what "
          "the first update did (a clock behind the wall clock is stepped forward then, by design, so a start step is no "
          "anomaly); `rejected`: samples whose bracket was too wide, the clock held at its frequency estimate; `steps "
          "after start`: the clock stepped forward after the first update; `wall steps`: CLOCK_REALTIME stepped under "
          "it (chrony).\n")
    rows = []
    for r in runs:
        updates = [u for u in r.rows["discipline"] if len(u) >= 9]
        flags = [int(u[8]) if u[8].isdigit() else 0 for u in updates]
        # the start update (its flags' start bit) apart: ergon-clock steps a
        # clock behind the wall clock forward there by design, so it is
        # never counted as a step
        starts = [u for u, f in zip(updates, flags) if f >> 6 & 1]
        start = "-"
        if starts:
            action = starts[0][1]
            start = f"stepped {action.split(':', 1)[1]} ns" if action.startswith("stepped:") else action
        stepped = sum(u[1].startswith("stepped") and not f >> 6 & 1 for u, f in zip(updates, flags))
        secs = re.search(r"discipline_secs=(\d+)", r.meta.get("settings", ""))
        rows.append([r.host, r.node, r.state, secs.group(1) if secs else "-", start,
                     spread(r, "residual_at_samples_ns"), spread(r, "residual_verifier_ns"), verifier_max(r),
                     interrupted(r), str(sum(f & 1 for f in flags)), str(stepped), str(sum(f >> 4 & 1 for f in flags)),
                     spread(r, "update_cost_ns")])
    table(["host", "node", "state", "s", "start", "at samples", "verifier", "verifier max at", "interrupted",
           "rejected", "steps after start", "wall steps", "update cost"], rows)
    print("## chrony's tracking around each discipline run (vm.sh discipline: chronyc tracking before and after)\n")
    rows = []
    for r in runs:
        cells = chrony(os.path.splitext(r.path)[0] + ".log")
        rows.append([r.host, r.node, r.state, cells.get("before", "-"), cells.get("after", "-")])
    table(["host", "node", "state", "before", "after"], rows)


def main(argv):
    if not argv:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    every = [run for path, _ in inputs(argv) for run in read_runs(path)]
    disciplined = [r for r in every if r.meta.get("mode") == "discipline"]
    runs = [r for r in every if r.meta.get("mode") != "discipline"]
    if not runs and not disciplined:
        print("clockbench.py: no clockbench runs in " + " ".join(argv), file=sys.stderr)
        return 1
    print("# Clock benchmarks (clockbench)\n")
    chosen = pick(runs)
    if chosen:
        comparison(chosen)
    nolto(runs)
    if chosen:
        per_host(chosen)
    if runs:
        every_run(runs)
        attributed(runs)
    if disciplined:
        discipline(disciplined)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
