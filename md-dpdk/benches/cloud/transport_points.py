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


def hosts_of_point(p):
    """The hosts point p used, as its manifest keys them (a one-host arm's
    one; a two-host or sink arm's a and b), else its set's (hosts_of): a set
    may hold one-host and sink arms (transport_plan.py)."""
    used = (p.manifest or {}).get("hosts")
    return tuple(sorted(used)) if used else hosts_of(p.set)


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


# the background's message sizes (mixed.rs SIZES) and the hot stream's frame
MIXED_SIZES = (64, 288, 1344, 4096)
AERON_HEADER = 32
HOT_FRAME = 96


def _ints(s, key):
    try:
        return [int(x) for x in str(s[key]).split(",")]
    except (KeyError, TypeError, ValueError):
        return None


def _int(s, key):
    try:
        return int(s[key])
    except (KeyError, TypeError, ValueError):
        return None


def aeron_frames_bytes(size, mtu):
    """A message's bytes in a term: its frames (fragments at the MTU), each
    with Aeron's header and 32-byte aligned."""
    per, out = mtu - AERON_HEADER, 0
    while True:
        k = min(size, per)
        out += (k + AERON_HEADER + 31) // 32 * 32
        size -= k
        if size <= 0:
            return out


def term_rotation_band(payload, messages, publishers, term, mtu):
    """(fewest, most) terms a background stream's publications filled, from
    its payload bytes and messages (the sizes' mix unknown, so a band): its
    frames' bytes at the least and most overhead the sizes allow, a term's
    end padded by up to a message's frames, its publications' share
    unknown (each rotates its own terms)."""
    extra = {s: aeron_frames_bytes(s, mtu) - s - AERON_HEADER for s in MIXED_SIZES}
    least = min(extra.values())
    big = max(MIXED_SIZES, key=lambda s: (extra[s], s))
    small = MIXED_SIZES[0]
    most_big = messages if big == small else min(messages, max(0, (payload - messages * small) // (big - small)))
    lo = payload + messages * (AERON_HEADER + least)
    hi = payload + messages * AERON_HEADER + least * (messages - most_big) + extra[big] * most_big
    pad = max(aeron_frames_bytes(s, mtu) for s in MIXED_SIZES)
    return max(0, lo // term - (publishers - 1)), hi // max(term - pad, 1)


def hot_rotations(sent, term):
    """The hot stream's terms filled by `sent` 96-byte frames: a term holds
    term // 96 of them, and the next offer after a full term rotates."""
    return max(sent - 1, 0) // (term // HOT_FRAME)


def legacy_admin_actions(s, row):
    """Whether a mixed point from a binary that counted Aeron's ADMIN_ACTION
    (a term rotation) as back-pressure (no mixed.admin) back-pressured only
    by term rotations: the hot stream's count its terms filled and every
    background stream's within its band (term_rotation_band)."""
    term, mtu = _int(s, "term_length"), _int(s, "mtu")
    bp, sent = _ints(s, "mixed.bp"), _ints(s, "mixed.sent")
    pay, pubs = _ints(s, "mixed.payload_bytes"), _ints(s, "mixed.publishers")
    hot = row.get("sent")
    if None in (term, mtu, bp, sent, pay, pubs, hot) or not term or not (len(bp) == len(sent) == len(pay)
                                                                            == len(pubs)):
        return False
    if (row.get("backpressured") or 0) != hot_rotations(hot, term):
        return False
    for b, n, y, k in zip(bp, sent, pay, pubs):
        lo, hi = term_rotation_band(y, n, k, term, mtu)
        if not lo <= b <= hi:
            return False
    return True


def mixed_offers(p):
    """A mixed-load point's (aeron_lat --load mixed) retryable offer results:
    {bp: offers back-pressured (BACK_PRESSURED, NOT_CONNECTED) in its
    measured window, the hot stream's (its row) and the background's
    (mixed.bp_measured); admin: Aeron's ADMIN_ACTIONs (term rotations, not
    back-pressure; the hot stream's admin_actions and the background's
    mixed.admin, whole runs); how: "recorded" where the binary counted
    them apart, "term rotations" where an older binary's counts are the
    terms its publications filled (legacy_admin_actions), "" where they
    are not and so stay back-pressure}; None for any other point."""
    s = (((p.manifest or {}).get("binary") or {}).get("settings")) or {}
    if s.get("load") != "mixed":
        return None
    hot = p.row.get("backpressured") or 0
    bg = sum(_ints(s, "mixed.bp_measured") or [])
    if "mixed.admin" in s:
        return {"bp": hot + bg, "admin": sum(_ints(s, "mixed.admin") or []) + (_int(s, "admin_actions") or 0),
                "how": "recorded"}
    if legacy_admin_actions(s, p.row):
        return {"bp": 0, "admin": hot + sum(_ints(s, "mixed.bp") or []), "how": "term rotations"}
    return {"bp": hot + bg, "admin": 0, "how": ""}


def mixed_backpressured(p):
    """A mixed-load point's offers back-pressured in its measured window
    (mixed_offers' bp); 0 for any other point."""
    o = mixed_offers(p)
    return o["bp"] if o else 0


def flags(p):
    """A point's flags: shown, never compared. The row's (row_flags),
    `IRQ core saturated` where a host's IRQ CPU's ksoftirqd ran over
    SATURATED of it (irq_core_saturated): the point measures the IRQ core's
    queue, not the transport; and a mixed-load point's `back-pressured`
    (mixed_backpressured): its offers queued behind flow control, so its
    latency is the queue's (fairness.py's backpressure rule requires the
    saturation that explains it)."""
    out = row_flags(p.row) + (["IRQ core saturated"] if irq_core_saturated(p.manifest) else [])
    if mixed_backpressured(p):
        out.append("back-pressured")
    return out


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
