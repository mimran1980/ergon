#!/usr/bin/env python3
"""The transport suite's points: its results.csv rows joined to their
manifests (transport-format.md). Shared by analyse.py --transport,
spikes.py --transport and fairness.py."""
import csv
import json
import os
from collections import namedtuple

# the sets that run on one host (transport-format.md)
ONE_HOST_SETS = {"1", "2b"}  # and their tuning sets, <set>.<n> (transport_tune.py)

# a transport row: the load row's 23 columns, then these
TRAILING = ["min", "lost", "reordered", "dup", "retrans", "wire_bytes", "busy_threads",
            "stream0_p50", "stream0_p99", "stream0_p99.99", "stream0_max"]
COLUMNS = (["rep", "kind", "label", "point", "size", "rate", "burst", "p50", "p90", "p99", "p99.9", "p99.99", "max",
            "mean", "samples", "sent", "received", "backpressured", "achieved", "max_send_lag", "scheduled", "late",
            "censored"] + TRAILING)
ROW_LEN = len(COLUMNS)  # 34
KINDS = ("load", "oneway")

# a point past this share of sends more than 100 µs late is flagged "sender late"
LATE_SHARE = 0.01
# achieved below this share of the offered rate is flagged "over capacity"
CAPACITY_SHARE = 0.98
# the IRQ CPU's ksoftirqd over this share of it is flagged "IRQ core saturated"
# (fairness.py's busy-thread threshold)
SATURATED = 0.9

Point = namedtuple("Point", "set arm label kind streams netem rate size rep tag row manifest")


def parse_label(label):
    """<set>:<arm>[@STREAMS=n][@NETEM=pct] as (set, arm, streams, netem):
    streams 1 when absent, netem None when absent (not a loss row)."""
    set_, _, mode = label.partition(":")
    parts = mode.split("@")
    streams, netem = 1, None
    for p in parts[1:]:
        k, _, v = p.partition("=")
        if k == "STREAMS" and v.isdigit():
            streams = int(v)
        elif k == "NETEM":
            netem = v
    return set_, parts[0], streams, netem


def file_name(label):
    """A label as lib.sh's file names spell it: <set>-<mode>, a / as _."""
    set_, _, mode = label.partition(":")
    return f"{set_}-{mode}".replace("/", "_")


def tag_of(label, size, rep, rate, kind):
    """A point's file tag (transport-format.md)."""
    return f"{file_name(label)}-{size}-{rep}-r{rate}" + ("-oneway" if kind == "oneway" else "")


def number(v):
    """A column as an int, or None when empty or not a number."""
    try:
        return int(v)
    except ValueError:
        try:
            return int(float(v))
        except ValueError:
            return None


def is_transport_row(row):
    return len(row) == ROW_LEN and row[1] in KINDS and ":" in row[2]


def read_points(run_dir):
    """Every transport row of <run_dir>/results.csv as a Point, its manifest
    (<run_dir>/<set>/manifest-<tag>.json) loaded, or None when missing or
    unreadable. Row columns by name; an empty or non-numeric column None."""
    path = os.path.join(run_dir, "results.csv")
    points = []
    if not os.path.exists(path):
        return points
    with open(path, newline="") as f:
        for raw in csv.reader(f):
            if not is_transport_row(raw):
                continue
            row = {c: (raw[i] if c in ("kind", "label", "point") else number(raw[i])) for i, c in enumerate(COLUMNS)}
            set_, arm, streams, netem = parse_label(row["label"])
            size, rate, rep = row["size"], row["rate"], row["rep"]
            tag = tag_of(row["label"], size, rep, rate, row["kind"])
            manifest = None
            mpath = os.path.join(run_dir, set_, f"manifest-{tag}.json")
            try:
                with open(mpath) as m:
                    manifest = json.load(m)
            except (OSError, ValueError):
                manifest = None
            points.append(Point(set_, arm, row["label"], row["kind"], streams, netem, rate, size, rep, tag, row,
                                manifest))
    return points


def hosts_of(set_):
    return ("a",) if set_.split(".")[0] in ONE_HOST_SETS else ("a", "b")


def irq_core_thread(t, cpus):
    """Whether placement sample entry t is the IRQ CPU's own softirq thread
    (ksoftirqd/<n> on IRQ CPU n, cpus a manifest's cpus.<host>) over
    SATURATED of it: the softirq work of the data NICs' interrupts more than
    their CPU keeps up with in interrupt context, handed to its thread. The
    IRQ core saturated: a capacity limit of the one-IRQ-CPU layout, the
    point's flag, never a placement fault."""
    cpu = t.get("cpu")
    return (cpu in set((cpus or {}).get("irq") or []) and t.get("comm") == f"ksoftirqd/{cpu}"
            and t.get("util", 0) > SATURATED)


def irq_core_saturated(manifest):
    """The hosts whose IRQ core a placement sample shows saturated (irq_core_thread)."""
    m = manifest or {}
    return sorted(h for h, samples in (m.get("threads") or {}).items()
                  if any(irq_core_thread(t, (m.get("cpus") or {}).get(h)) for s in samples or [] for t in s))


def flags(p):
    """A point's flags: shown, never compared. The row's (row_flags), and
    `IRQ core saturated` where a host's IRQ CPU's ksoftirqd ran over
    SATURATED of it (irq_core_saturated): the point measures the IRQ core's
    queue, not the transport."""
    return row_flags(p.row) + (["IRQ core saturated"] if irq_core_saturated(p.manifest) else [])


def row_flags(r):
    """A transport row's flags: `sender late` (over LATE_SHARE of sends more
    than 100 µs late), `over capacity` (achieved under CAPACITY_SHARE of the
    rate), `no samples`."""
    out = []
    if r["scheduled"] and r["late"] is not None and r["late"] > LATE_SHARE * r["scheduled"]:
        out.append("sender late")
    if r["achieved"] is not None and r["rate"] and r["achieved"] < CAPACITY_SHARE * r["rate"]:
        out.append("over capacity")
    if not r["samples"]:
        out.append("no samples")
    return out


def tcp_loss(p):
    """A TCP point's (real, spurious) segments over every host's traffic:
    real the retransmits the peer did not report as duplicates
    (retrans_not_dsack) plus those lost at the end (lost_end), spurious the
    DSACK-reported ones (a late ACK, not loss). None where no host has a
    tcp_info with the split."""
    real = spurious = 0
    found = False
    for src in ((p.manifest or {}).get("loss_counters") or {}).values():
        t = (src or {}).get("tcp_info")
        if not t or t.get("retrans_not_dsack") is None:
            continue
        found = True
        real += (t.get("retrans_not_dsack") or 0) + (t.get("lost_end") or 0)
        spurious += t.get("dsack_dups") or 0
    return (real, spurious) if found else None


def group_key(p):
    """The points one table compares: one set, stream count, netem, rate and kind."""
    return (p.set, p.streams, p.netem or "", p.rate, p.kind)


def is_aa(p):
    return bool(p.manifest and p.manifest.get("aa"))


def aa_of(p):
    return p.manifest.get("aa_of") if p.manifest else None
