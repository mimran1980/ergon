#!/usr/bin/env python3
"""The transport suite's points: its results.csv rows joined to their
manifests (transport-format.md). Shared by analyse.py --transport,
spikes.py --transport and fairness.py."""
import csv
import json
import os
from collections import namedtuple

# the sets that run on one host (transport-format.md)
ONE_HOST_SETS = {"1", "2b"}

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
    return ("a",) if set_ in ONE_HOST_SETS else ("a", "b")


def flags(p):
    """A point's flags: shown, never compared."""
    return row_flags(p.row)


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


def group_key(p):
    """The points one table compares: one set, stream count, netem, rate and kind."""
    return (p.set, p.streams, p.netem or "", p.rate, p.kind)


def is_aa(p):
    return bool(p.manifest and p.manifest.get("aa"))


def aa_of(p):
    return p.manifest.get("aa_of") if p.manifest else None
