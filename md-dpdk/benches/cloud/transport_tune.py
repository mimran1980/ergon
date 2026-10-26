#!/usr/bin/env python3
"""The transport suite's tuning session (azure/lab.sh LAB_TUNE=1; transport.sh
run tune): every arm's knobs swept, the better values confirmed, the choice
frozen into transport-settings.env for the comparison sessions.

  transport_tune.py sweep --settings <env> --out <dir> [--arms a,b] [--knobs k,l]
                          [--absent "<knob> <knob>=<value> ..."] <plan files>...
        the sweep plan into <dir>: tune-arms.tsv (one arm row a variant: an arm
        at one value of one knob), tune-sweep.tsv (one set row an arm and
        knob: its values, the current one first, and the A/A), variants.txt
        ("<variant> <arm> <knob> <value> <1: the current value> <set>")
  transport_tune.py confirm <run dir>
        the confirmation's set rows on stdout (each knob whose sweep found a
        value better than the current one: the current value, the top two and
        the A/A); the sweep's decisions into <run dir>/tune-sweep.json
  transport_tune.py freeze <run dir> <stamp>
        the tuned section on stdout (transport-settings.env's generated
        block) and <run dir>/tune-report.md, the evidence
  transport_tune.py apply <settings env> <section file>
        the section into the settings file, in place of the one there
  transport_tune.py count --settings <env> [--arms] [--knobs] [--absent] <plan files>...
        "<points> <runs> <sets> <one-host sets>" of the sweep (estimate.py)

The sweeps are transport-settings.env's TR_SWEEP_<KNOB>="<arms> <values>
<profile>". Each arm of a sweep runs in the context of the matrix's sets that
hold it: their middle load (the first set's), 1 and 128 streams where they
have them (TR_TUNE_STREAMS), its own host or hosts; profile loss also under
netem TR_TUNE_NETEM % where one of its sets is a loss set. The current value
(the tuned section's TR_ARM_KNOBS, else the knob's default) is swept first
and repeated as the round's A/A arm, so each round measures its own
no-difference band.

A value beats another (better()): measured in each of the round's
dimensions (row kind, streams, netem), it is lower by more than the band at
p99.99 in at least one and higher by more than it in none; with no
dimension separated at p99.99 either way, the same at the max. The sweep's
band is the A/A difference itself (one rep); the confirmation's is the
largest per-rep A/A difference and, besides, the reps' ranges must not
overlap. A tie keeps the current value. The congestion control is the
hosts' (one value for every TCP arm): a new one is frozen only when every
TCP arm swept chose it.
"""
import argparse
import collections
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import transport_plan as tp  # noqa: E402
import transport_points as pts  # noqa: E402

# knob: its default's TR_ variable (transport-settings.env's per-arm knob list)
KNOBS = {
    "busy_read_us": "TR_BUSY_READ", "epoll_events": "TR_EPOLL_EVENTS", "datagram_burst": "TR_DATAGRAM_BURST",
    "sockbuf": "TR_SOCKBUF", "quickack_per_read": "TR_QUICKACK_PER_READ", "rto_min_us": "TR_TCP_RTO_MIN_US",
    "congestion": "TR_TCP_CONGESTION", "term_length": "TR_TERM_LENGTH", "aeron_path": "TR_AERON_PATH",
    "aeron_so_rcvbuf": "TR_AERON_SO_RCVBUF", "nak_delay_ns": "TR_AERON_NAK_DELAY_NS",
    "rx_free_thresh": "TR_DPDK_RX_FREE_THRESH", "aeron_threading": "TR_AERON_IPC_THREADING",
    "capacity": "TR_MPSC_CAPACITY", "napi_defer": "TR_SOCKET_NAPI_DEFER",
}
# the hosts' own, one value for every arm that has it (frozen as its TR_ variable)
HOST_WIDE = {"congestion"}
BEGIN = "# ==== tuned:"
END = "# ==== end tuned ===="
SWEEP_SET = re.compile(r"\.\d+$")
CONFIRM_SET = re.compile(r"\.c\d+$")


class TuneError(Exception):
    pass


def read_settings(path):
    """transport-settings.env's TR_ assignments, the last of each winning (the
    tuned section's over the defaults), quotes removed."""
    out = {}
    with open(path) as f:
        for line in f:
            m = re.match(r"^(TR_[A-Z0-9_]+)=(.*)$", line.strip())
            if m:
                v = m.group(2).strip()
                if len(v) >= 2 and v[0] == v[-1] and v[0] in "\"'":
                    v = v[1:-1]
                out[m.group(1)] = v
    return out


def arm_knobs(tr):
    """{(arm, knob): value} of TR_ARM_KNOBS."""
    out = {}
    for kv in tr.get("TR_ARM_KNOBS", "").split():
        a, _, rest = kv.partition(":")
        k, _, v = rest.partition("=")
        if a and k and v:
            out[(a, k)] = v
    return out


def current(tr, arm, knob):
    """An arm's value of a knob now: the tuned section's, else the default
    (a host-wide knob's is its TR_ variable alone)."""
    if knob not in HOST_WIDE:
        v = arm_knobs(tr).get((arm, knob))
        if v is not None:
            return v
    return tr[KNOBS[knob]]


def sweeps(tr):
    """[(knob, [arms], [values], profile)] in the file's order."""
    out = []
    for k, v in tr.items():
        if not k.startswith("TR_SWEEP_"):
            continue
        knob = k[len("TR_SWEEP_"):].lower()
        w = v.split()
        if knob not in KNOBS or len(w) != 3 or w[2] not in ("load", "loss"):
            raise TuneError(f"{k}={v!r}: a knob of {', '.join(KNOBS)} and \"<arms> <values> load|loss\"")
        out.append((knob, w[0].split(","), w[1].split(","), w[2]))
    return out


def token(v):
    return re.sub(r"[^A-Za-z0-9_-]", "_", v)


def variant(arm, knob, value):
    return f"{arm}--{knob}-{token(value)}"


def plan(a):
    """The sweep: (arm rows, set dicts, variants rows, notes)."""
    tr = read_settings(a.settings)
    arms, sets = tp.load(a.files)
    only_arms = {x for x in (a.arms or "").replace(",", " ").split() if x}
    only_knobs = {x for x in (a.knobs or "").replace(",", " ").split() if x}
    absent = (a.absent or "").split()
    want_streams = [int(x) for x in tr.get("TR_TUNE_STREAMS", "1 128").split()]
    loss = tr.get("TR_TUNE_NETEM", "0.1")
    arm_rows, out_sets, rows, notes, k = [], [], [], [], 0
    for knob, karms, values, profile in sweeps(tr):
        if only_knobs and knob not in only_knobs:
            continue
        if knob in absent:
            notes.append(f"{knob}: not swept: the hosts' kernels lack it")
            continue
        for arm in karms:
            if only_arms and arm not in only_arms:
                continue
            ctx = [s for s in sets if arm in s["arms"]]
            if arm not in arms or not ctx:
                notes.append(f"{knob} {arm}: not swept: no set of the plan runs it")
                continue
            # the session that tunes it (LAB_TUNE=1 on both pairs at once): an
            # arm of a loss set (the TCP arms, aeron-rel) on pair b, every
            # other on pair a, which halves the wall time about evenly
            lossy = any(lv not in ("-", "none") for s in ctx for lv in s["netem"])
            if a.pair and ("b" if lossy else "a") != a.pair:
                continue
            have = sorted({n for s in ctx for n in s["streams"]})
            streams = [n for n in want_streams if n in have] or have[:1]
            loads = sorted(ctx[0]["loads"])
            netem = ["-"]
            if profile == "loss":
                if not any(lv not in ("-", "none") for s in ctx for lv in s["netem"]):
                    notes.append(f"{knob} {arm}: swept without loss: none of its sets is a loss set")
                else:
                    netem = ["-", loss]
            cur = current(tr, arm, knob)
            vals = [cur] + [v for v in values if v != cur and f"{knob}={v}" not in absent]
            gone = [v for v in values if f"{knob}={v}" in absent]
            if gone:
                notes.append(f"{knob} {arm}: {', '.join(gone)} not swept: the hosts lack it")
            if len(vals) < 2:
                notes.append(f"{knob} {arm}: one value left ({cur}): nothing to sweep")
                continue
            k += 1
            sid = f"{ctx[0]['id']}.{k}"
            binary, setup, hosts, args = arms[arm]
            names = []
            for v in vals:
                name = variant(arm, knob, v)
                arm_rows.append(" ".join(["arm", name, binary, setup, hosts] + list(args)))
                rows.append((name, arm, knob, v, 1 if v == cur else 0, sid))
                names.append(name)
            out_sets.append(dict(id=sid, host=ctx[0]["host"], loads=[loads[len(loads) // 2]], streams=streams,
                                 netem=netem, arms=names + ["aa"], par="-"))
    pair_one_host(out_sets)
    return arm_rows, out_sets, rows, notes


def pair_one_host(sets):
    """One-host sets on host a and on host b run two at a time (par tags)."""
    by_host = {"a": [s for s in sets if s["host"] == "a"], "b": [s for s in sets if s["host"] == "b"]}
    for i, (x, y) in enumerate(zip(by_host["a"], by_host["b"])):
        x["par"] = y["par"] = f"q{i + 1}"


def set_row(s):
    return " ".join(["set", s["id"], "tune", s["par"], s["host"], ",".join(map(str, s["loads"])),
                     ",".join(map(str, s["streams"])), ",".join(s["netem"]), ",".join(s["arms"])])


def cmd_sweep(a):
    arm_rows, sets, rows, notes = plan(a)
    if not sets:
        raise TuneError("nothing to sweep" + (f": {'; '.join(notes)}" if notes else ""))
    os.makedirs(a.out, exist_ok=True)
    with open(os.path.join(a.out, "tune-arms.tsv"), "w") as f:
        f.write("# the tuning session's variants (transport_tune.py sweep): an arm at one value of one knob\n")
        f.write("\n".join(arm_rows) + "\n")
    with open(os.path.join(a.out, "tune-sweep.tsv"), "w") as f:
        f.write("# the tuning session's sweep (transport_tune.py sweep): an arm and knob a set\n")
        f.write("\n".join(set_row(s) for s in sets) + "\n")
    with open(os.path.join(a.out, "variants.txt"), "w") as f:
        f.write("".join(" ".join(map(str, r)) + "\n" for r in rows))
    for n in notes:
        print(f"note: {n}")
    print(f"sweep: {len(sets)} sets, {len(rows)} variants")
    return 0


def cmd_count(a):
    """The sweep's points and runs, and the confirmation's at most (every
    set confirmed: its current value, its top two and the A/A, over
    TR_TUNE_CONFIRM_REPS reps), as estimate.py times them: two-host runs
    one after another; one-host runs of a set paired with one on the
    other host (a par tag) counted once."""
    tr = read_settings(a.settings)
    reps = int(tr.get("TR_TUNE_CONFIRM_REPS", "3"))
    _, sets, _, _ = plan(a)

    def runs(s, n_arms, r=1):
        return (1 if s["host"] == "ab" else 2) * n_arms * len(s["streams"]) * len(s["netem"]) * r

    def timed(arms_of, r):
        # a par pair runs at once: the longer of the two
        total, seen = 0, set()
        for s in sets:
            if s["par"] != "-":
                if s["par"] in seen:
                    continue
                seen.add(s["par"])
                total += max(runs(x, arms_of(x), r) for x in sets if x["par"] == s["par"])
            else:
                total += runs(s, arms_of(s), r)
        return total

    points = sum(len(s["arms"]) * len(s["streams"]) * len(s["netem"]) for s in sets)
    sweep = timed(lambda s: len(s["arms"]), 1)
    confirm = timed(lambda s: min(len(s["arms"]) - 1, 3) + 1, reps)
    cpoints = sum((min(len(s["arms"]) - 1, 3) + 1) * len(s["streams"]) * len(s["netem"]) * reps for s in sets)
    print(f"sets={len(sets)} sweep_points={points} sweep_runs={sweep} confirm_points_max={cpoints} confirm_runs_max={confirm}")
    return 0


def read_variants(d):
    out = {}
    with open(os.path.join(d, "variants.txt")) as f:
        for line in f:
            w = line.split()
            if len(w) == 6:
                out[w[0]] = dict(arm=w[1], knob=w[2], value=w[3], current=w[4] == "1", set=w[5])
    return out


def measured(run_dir, pattern):
    """{set: {name: {dim: [(rep, p99.99, max)]}}} of the tuning sets whose id
    matches pattern; name a variant or aa; dim (kind, streams, netem)."""
    out = collections.defaultdict(lambda: collections.defaultdict(lambda: collections.defaultdict(list)))
    for p in pts.read_points(run_dir):
        if not pattern.search(p.set):
            continue
        name = "aa" if pts.is_aa(p) else p.arm
        out[p.set][name][(p.kind, p.streams, p.netem or "-")].append(
            (p.rep, p.row.get("p99.99"), p.row.get("max")))
    return out


def median(v):
    v = sorted(v)
    n = len(v)
    return None if not n else (v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) / 2)


def separated(x, ref, band, ranges):
    """x is lower than ref: by more than the band, and (ranges) with no
    overlap of their reps. x, ref: lists of values."""
    if not x or not ref or None in x or None in ref:
        return False
    if median(ref) - median(x) <= band:
        return False
    return not ranges or max(x) < min(ref)


def better(cand, ref, aa, ranges):
    """Whether cand beats ref (the module's rule), and how: (bool, why).
    cand, ref, aa: {dim: [(rep, p99.99, max)]}."""
    dims = sorted(set(cand) & set(ref))
    if not dims:
        return False, "not measured"
    for col, what in ((1, "p99.99"), (2, "max")):
        wins, losses = [], []
        for dim in dims:
            c, r = [x[col] for x in cand[dim]], [x[col] for x in ref[dim]]
            band = aa_band(ref[dim], aa.get(dim, []), col)
            if band is None:
                return False, f"no A/A at {dim_name(dim)}"
            if separated(c, r, band, ranges):
                wins.append(dim)
            elif separated(r, c, band, ranges):
                losses.append(dim)
        if wins or losses:
            if losses:
                return False, f"{what} worse at {', '.join(map(dim_name, losses))}"
            return True, f"{what} better at {', '.join(map(dim_name, wins))}"
    return False, "within the A/A band at p99.99 and max"


def aa_band(ref, aa, col):
    """The no-difference band: the largest per-rep |ref - its A/A|; None
    without an A/A rep."""
    by_rep = {x[0]: x[col] for x in aa}
    d = [abs(x[col] - by_rep[x[0]]) for x in ref if x[0] in by_rep and x[col] is not None and by_rep[x[0]] is not None]
    return max(d) if d else None


def dim_name(dim):
    kind, n, netem = dim
    return f"{kind} N={n}" + ("" if netem == "-" else f" netem {netem}%")


def gain(cand, ref):
    """The mean relative p99.99 gain of cand over ref (ranking only)."""
    g = []
    for dim in set(cand) & set(ref):
        c, r = median([x[1] for x in cand[dim]]), median([x[1] for x in ref[dim]])
        if c is not None and r:
            g.append((r - c) / r)
    return sum(g) / len(g) if g else 0.0


def summary(data):
    """{dim name: "p99.99 µs / max µs"} (medians) of one name's points."""
    return {dim_name(d): f"{median([x[1] for x in v]) / 1e3:.1f}/{median([x[2] for x in v]) / 1e3:.1f}"
            for d, v in sorted(data.items()) if v and None not in [x[1] for x in v] + [x[2] for x in v]}


def cmd_confirm(a):
    d = a.run_dir
    var = read_variants(d)
    data = measured(d, SWEEP_SET)
    arms, sets = tp.load([os.path.join(d, "tune-arms.tsv"), os.path.join(d, "tune-sweep.tsv")])
    decisions, out, k = {}, [], 0
    for s in sets:
        names = [x for x in s["arms"] if x != "aa"]
        cur = next(x for x in names if var[x]["current"])
        got = data.get(s["id"], {})
        base, knob = var[cur]["arm"], var[cur]["knob"]
        dec = dict(set=s["id"], arm=base, knob=knob, current=var[cur]["value"],
                   measured={var[x]["value"] if x in var else x: summary(got.get(x, {})) for x in names + ["aa"]})
        missing = [x for x in names + ["aa"] if not got.get(x)]
        if missing:
            dec.update(decision="keep", why=f"incomplete: no rows of {', '.join(missing)}")
            decisions[s["id"]] = dec
            continue
        ranked = []
        for x in names:
            if x == cur:
                continue
            ok, why = better(got[x], got[cur], got["aa"], ranges=False)
            dec.setdefault("versus", {})[var[x]["value"]] = why
            if ok:
                ranked.append((gain(got[x], got[cur]), x))
        if not ranked:
            dec.update(decision="keep", why="no value beat the current one outside the A/A band")
            decisions[s["id"]] = dec
            continue
        ranked.sort(reverse=True)
        top = [x for _, x in ranked[:2]]
        k += 1
        cid = re.sub(r"\.\d+$", f".c{k}", s["id"])
        out.append(dict(s, id=cid, arms=[cur] + top + ["aa"]))
        dec.update(decision="confirm", confirm_set=cid, top=[var[x]["value"] for x in top])
        decisions[s["id"]] = dec
    pair_one_host(out)
    with open(os.path.join(d, "tune-sweep.json"), "w") as f:
        json.dump(decisions, f, indent=1, sort_keys=True)
    print("# the tuning session's confirmation (transport_tune.py confirm): each knob whose sweep found a value")
    print("# better than the current one, the current value first (its A/A), then the sweep's top two")
    for s in out:
        print(set_row(s))
    return 0


def cmd_freeze(a):
    d = a.run_dir
    var = read_variants(d)
    tr = read_settings(os.path.join(d, "transport-settings.tr.env"))
    with open(os.path.join(d, "tune-sweep.json")) as f:
        decisions = json.load(f)
    conf = measured(d, CONFIRM_SET)
    confirm_sets = {}
    if os.path.exists(os.path.join(d, "tune-confirm.tsv")):
        _, csets = tp.load([os.path.join(d, "tune-arms.tsv"), os.path.join(d, "tune-confirm.tsv")])
        confirm_sets = {s["id"]: s for s in csets}
    frozen = {}  # (arm, knob): (value, changed, why)
    for sid, dec in sorted(decisions.items(), key=lambda kv: (kv[1]["knob"], kv[1]["arm"])):
        arm, knob, cur = dec["arm"], dec["knob"], dec["current"]
        if dec["decision"] != "confirm":
            frozen[(arm, knob)] = (cur, False, dec["why"])
            continue
        cs = confirm_sets.get(dec["confirm_set"])
        got = conf.get(dec["confirm_set"], {})
        names = [x for x in (cs or {}).get("arms", []) if x != "aa"]
        if not cs or any(not got.get(x) for x in names + ["aa"]):
            frozen[(arm, knob)] = (cur, False, "the confirmation did not complete: the current value kept")
            dec["confirmed"] = "incomplete"
            continue
        curn = names[0]
        wins = []
        for x in names[1:]:
            ok, why = better(got[x], got[curn], got["aa"], ranges=True)
            dec.setdefault("confirm_versus", {})[var[x]["value"]] = why
            if ok:
                wins.append(x)
        dec["confirm_measured"] = {var[x]["value"]: summary(got[x]) for x in names}
        dec["confirm_measured"]["aa"] = summary(got["aa"])
        if not wins:
            frozen[(arm, knob)] = (cur, False, f"the confirmation's {len(names) - 1} best did not beat it over "
                                   f"{tr.get('TR_TUNE_CONFIRM_REPS', 3)} reps")
            continue
        pick = wins[0]
        if len(wins) == 2 and better(got[wins[1]], got[wins[0]], got["aa"], ranges=True)[0]:
            pick = wins[1]
        frozen[(arm, knob)] = (var[pick]["value"], True, dec["confirm_versus"][var[pick]["value"]])
    # a host-wide knob: one value for every arm swept
    host = {}
    declared = {k: set(arms_) for k, arms_, _, _ in sweeps(tr)}
    for knob in HOST_WIDE:
        picks = {arm: v for (arm, k), v in frozen.items() if k == knob}
        if not picks:
            continue
        vals = {v[0] for v in picks.values()}
        cur = tr[KNOBS[knob]]
        unswept = sorted(declared.get(knob, set()) - set(picks))
        if len(vals) == 1 and vals != {cur} and not unswept:
            host[knob] = (vals.pop(), f"every arm swept chose it ({', '.join(sorted(picks))})")
        elif len(vals) == 1 and vals != {cur}:
            host[knob] = (cur, f"kept: {', '.join(sorted(picks))} chose {vals.pop()}, but {', '.join(unswept)} "
                               "(the hosts' one value too) was not swept")
        else:
            host[knob] = (cur, "kept: " + ("the arms disagreed: " + ", ".join(f"{a} {v[0]}" for a, v in
                                                                                   sorted(picks.items()))
                                           if len(vals) > 1 else "no arm's sweep beat it"))
    # this run's choices over the earlier tuned section's (a knob swept now replaces its entry)
    knobs = arm_knobs(tr)
    swept = collections.defaultdict(set)
    for kv in tr.get("TR_TUNED_SWEPT", "").split():
        k, _, arms_ = kv.partition(":")
        swept[k].update(x for x in arms_.split(",") if x)
    for (arm, knob), (v, _, _) in frozen.items():
        swept[knob].add(arm)
        if knob not in HOST_WIDE:
            knobs[(arm, knob)] = v
    runs = " ".join(x for x in (tr.get("TR_TUNED_RUN", "").split() + [a.stamp]) if x)
    changed = [(arm, knob, v, why) for (arm, knob), (v, ch, why) in sorted(frozen.items())
               if ch and knob not in HOST_WIDE]
    lines = [f"{BEGIN} generated by transport_tune.py (LAB_TUNE=1); edit by rerunning the tuning, not by hand ====",
             f"# Run {a.stamp}: {len(frozen)} arm knobs swept, {len(changed)} changed"
             + "".join(f", {k} {'changed' if v[0] != tr[KNOBS[k]] else 'kept'}" for k, v in sorted(host.items()))
             + "; the evidence: that run's tune-report.md.",
             f'TR_TUNED_RUN="{runs}"',
             'TR_TUNED_SWEPT="' + " ".join(f"{k}:{','.join(sorted(v))}" for k, v in sorted(swept.items())) + '"',
             'TR_ARM_KNOBS="' + " ".join(f"{a_}:{k}={v}" for (a_, k), v in sorted(knobs.items())) + '"']
    for knob, (v, why) in sorted(host.items()):
        lines.append(f"{KNOBS[knob]}={v}")
        lines.append(f"# {'changed' if v != tr[KNOBS[knob]] else 'kept'} {knob} (the hosts'): {v}: {why}")
    for arm, knob, v, why in changed:
        lines.append(f"# changed {arm} {knob}: {current(tr, arm, knob)} -> {v}: {why}")
    for (arm, knob), (v, ch, why) in sorted(frozen.items()):
        if not ch and knob not in HOST_WIDE:
            lines.append(f"# kept {arm} {knob} {v}: {why}")
    lines.append(END)
    print("\n".join(lines))
    report(d, a.stamp, decisions, frozen, host)
    with open(os.path.join(d, "tune-sweep.json"), "w") as f:
        json.dump(decisions, f, indent=1, sort_keys=True)
    return 0


def report(d, stamp, decisions, frozen, host):
    """tune-report.md: each arm and knob, what each value measured, the decision."""
    out = [f"# Tuning session {stamp}", "",
           "Each arm and knob: one round of its values and an A/A of the current one (the sweep), then for a value "
           "that beat it outside the A/A band, the current value and the top two again over the confirmation's reps. "
           "p99.99/max in µs, medians, by row kind, streams and netem. A tie keeps the current value.", "",
           "| arm | knob | current | sweep: value p99.99/max | sweep verdicts | confirmation | frozen |",
           "| --- | --- | --- | --- | --- | --- | --- |"]
    for sid, dec in sorted(decisions.items(), key=lambda kv: (kv[1]["arm"], kv[1]["knob"])):
        arm, knob = dec["arm"], dec["knob"]
        meas = "; ".join(f"{v}: " + ", ".join(f"{dn} {x}" for dn, x in m.items()) for v, m in dec["measured"].items())
        verd = "; ".join(f"{v}: {w}" for v, w in (dec.get("versus") or {}).items()) or dec.get("why", "")
        conf = "; ".join(f"{v}: {w}" for v, w in (dec.get("confirm_versus") or {}).items()) or "-"
        v, ch, why = frozen.get((arm, knob), (dec["current"], False, ""))
        if knob in host:
            v, why = host[knob][0], host[knob][1]
        out.append(f"| {arm} | {knob} | {dec['current']} | {meas} | {verd} | {conf} | "
                   f"{'**' + v + '**' if ch else v}: {why} |")
    with open(os.path.join(d, "tune-report.md"), "w") as f:
        f.write("\n".join(out) + "\n")


def parse_section(lines):
    """A tuned section's (assignments {TR_...: value}, comment lines)."""
    vals, notes = {}, []
    for line in lines[1:-1]:
        m = re.match(r"^(TR_[A-Z0-9_]+)=(.*)$", line)
        if m:
            v = m.group(2)
            vals[m.group(1)] = v[1:-1] if len(v) >= 2 and v[0] == v[-1] == '"' else v
        elif line.startswith("# changed ") or line.startswith("# kept "):
            notes.append(line)
    return vals, notes


def merge(old, new):
    """The section the settings file keeps: the new one's choices over the
    old one's (both pairs' tuning sessions start from the same file, each
    tuning its own arms): runs and swept arms joined, each arm's knob the new
    value where the new session swept it, a host-wide knob the new one's."""
    (ov, on), (nv, nn) = parse_section(old), parse_section(new)
    knobs = {}
    for src in (ov, nv):
        for kv in src.get("TR_ARM_KNOBS", "").split():
            a, _, rest = kv.partition(":")
            knobs[(a, rest.partition("=")[0])] = kv
    swept = collections.defaultdict(set)
    for src in (ov, nv):
        for kv in src.get("TR_TUNED_SWEPT", "").split():
            k, _, arms_ = kv.partition(":")
            swept[k].update(x for x in arms_.split(",") if x)
    runs = []
    for src in (ov, nv):
        runs += [r for r in src.get("TR_TUNED_RUN", "").split() if r not in runs]
    key = lambda l: tuple(x.rstrip(":") for x in l.split()[2:4])
    redone = {key(l) for l in nn}
    out = [new[0], new[1] if len(new) > 1 and new[1].startswith("# Run") else "# merged",
           f'TR_TUNED_RUN="{" ".join(runs)}"',
           'TR_TUNED_SWEPT="' + " ".join(f"{k}:{','.join(sorted(v))}" for k, v in sorted(swept.items())) + '"',
           'TR_ARM_KNOBS="' + " ".join(v for _, v in sorted(knobs.items())) + '"']
    host = {k: v for k, v in ov.items() if k not in ("TR_TUNED_RUN", "TR_TUNED_SWEPT", "TR_ARM_KNOBS")}
    host.update({k: v for k, v in nv.items() if k not in ("TR_TUNED_RUN", "TR_TUNED_SWEPT", "TR_ARM_KNOBS")})
    out += [f"{k}={v}" for k, v in sorted(host.items())]
    out += nn + [l for l in on if key(l) not in redone]
    return out + [END]


def cmd_apply(a):
    with open(a.section) as f:
        section = f.read().strip("\n").splitlines()
    if not section or not section[0].startswith(BEGIN) or section[-1] != END:
        raise TuneError(f"{a.section}: not a tuned section ({BEGIN} ... {END})")
    with open(a.settings) as f:
        lines = f.read().splitlines()
    try:
        i = next(n for n, l in enumerate(lines) if l.startswith(BEGIN))
        j = next(n for n, l in enumerate(lines) if n > i and l == END)
        lines[i:j + 1] = merge(lines[i:j + 1], section)
    except StopIteration:
        lines += [""] + section
    with open(a.settings, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"apply: {a.settings}: the tuned section of {a.section}")
    return 0


def main(argv):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    for name in ("sweep", "count"):
        s = sub.add_parser(name)
        s.add_argument("--settings", required=True)
        s.add_argument("--out", default=".")
        s.add_argument("--arms", default="")
        s.add_argument("--knobs", default="")
        s.add_argument("--absent", default="")
        s.add_argument("--pair", default="", help="a or b: the arms this pair's tuning session sweeps (none: all)")
        s.add_argument("files", nargs="+")
    s = sub.add_parser("confirm")
    s.add_argument("run_dir")
    s = sub.add_parser("freeze")
    s.add_argument("run_dir")
    s.add_argument("stamp")
    s = sub.add_parser("apply")
    s.add_argument("settings")
    s.add_argument("section")
    a = p.parse_args(argv)
    try:
        return {"sweep": cmd_sweep, "count": cmd_count, "confirm": cmd_confirm, "freeze": cmd_freeze,
                "apply": cmd_apply}[a.cmd](a)
    except (TuneError, tp.PlanError, OSError, KeyError, StopIteration) as e:
        print(f"transport_tune.py {a.cmd}: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
