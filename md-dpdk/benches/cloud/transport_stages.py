#!/usr/bin/env python3
"""Where a round trip's time goes: md-dpdk's stage trace per point, and its
packet capture's accounting.

transport_stages.py point <point dir>          one point's breakdown (stdout)
transport_stages.py run <session dir>          every traced point, the A/A
                                               overhead check and every
                                               capture: <session>/stages.md
                                               and stages.csv (and stdout)
transport_stages.py join <ping app> <pong app> <ping driver> <pong driver>
                                               the breakdown from four files
transport_stages.py capture <pcapng[.gz]> [<every>]
                                               a capture's accounting

A point directory is the runner's points/<point>/: the ping's
0/stages-app.csv (aeron_lat --stage-trace, host a), the pong's
pong/stages-app.csv (host b), each host's driver file
stages-driver-<a|b>.csv[.gz] (RUSTERON_DPDK_STAGE_TRACE_FILE) and, on an
arm's first point, capture-<a|b>.pcapng[.gz] (RUSTERON_DPDK_CAPTURE_FILE).
Every file holds `stage,tsc,stream_id,seq,term_offset,value,ring` rows; a
message is keyed by (stream id, seq) and its echo by (stream id + 10000,
seq): the ping's on stream ids from S (the app file's stream_id), its echo's
from S + echo_offset. Counter ticks become ns at each host's own rate (its
app file's ticks_per_ns; else its driver file's two clock lines). No time is
ever compared across hosts: the wire is what the round trip leaves.

The stages, per sampled measured message (a = ping host, b = pong host):

  offer call       app-offered - app-offer      the publication's offer
  app->sender      tx-send - app-offer          the driver's sender picks it up
  sender->tx       tx-call - tx-send            the send path builds the frame
  tx burst         tx-done - tx-call            rte_eth_tx_burst (kernel: sendmmsg)
  rx ring wait     rx-burst - NIC arrival       with NIC RX timestamps (rx-hw) only
  rx->dispatch     rx-dispatch - rx-burst       classify, the frames before it
  insert           rx-inserted - rx-dispatch    Aeron's receive callback: into the image
  insert->app      app-read - rx-inserted       the subscriber's poll finds it
  turnaround       b app-offer(echo) - b app-read(ping)
  host a           (a tx-done - a app-offer) + (a app-read(echo) - a rx-burst(echo))
  host b           b tx-done(echo) - b rx-burst(ping)
  wire             round trip - host a - host b: both NICs' DMA, the network
                   both ways and (without rx-hw) each receive ring's wait
  round trip       a app-read(echo) - a app-offer(ping)

Each stage's n, p50, p99 and max, the max's message (stream, seq, seconds
into the window), and a flag where it is above what it should take
(EXPECT_NS: p50, or p99 for the bursts), any max over MAX_NS, messages
missing a stage, and stamps seen twice (a resend). The A/A check (run):
for each set, stream count and load, the traced arm's p50 and p99 against
its untraced arm's, beside the untraced arm's A/A spread (aa against it);
a shift past the spread is flagged.
"""
import collections
import csv
import gzip
import io
import os
import re
import struct
import sys

# what a stage should take (ns); a p50 above it is flagged, a p99 for "tx
# burst" (rte_eth_tx_burst is the NIC doorbell, ~100 ns on ENA's LLQ)
EXPECT_NS = {
    "offer call": 1000,
    "app->sender": 2000,  # one sender duty cycle and its hand-off
    "sender->tx": 1000,
    "tx burst": 1000,
    "rx ring wait": 2000,
    "rx->dispatch": 1000,
    "insert": 1000,
    "insert->app": 2000,
    "turnaround": 2000,
}
P99_FLAGGED = {"tx burst"}
# any stage's max above this is named with its message
MAX_NS = 50000
# more than this share of messages missing a stage is flagged
MISSING_SHARE = 0.01
HOST_STAGES = ["offer call", "app->sender", "sender->tx", "tx burst",
               "rx ring wait", "rx->dispatch", "insert", "insert->app"]


def opener(path):
    return gzip.open(path, "rt") if path.endswith(".gz") else open(path)


def find(base, name):
    """base/name, or base/name.gz, or None."""
    for p in (os.path.join(base, name), os.path.join(base, name + ".gz")):
        if os.path.exists(p):
            return p
    return None


class StageFile:
    """One stage file: its `#` metadata, clock lines and rows by key."""

    def __init__(self, path):
        self.path = path
        self.meta = {}
        self.clocks = []
        self.drops = 0
        self.rows = collections.defaultdict(lambda: collections.defaultdict(list))
        self.count = 0
        with opener(path) as f:
            for line in f:
                line = line.rstrip("\n")
                if not line:
                    continue
                if line.startswith("#"):
                    self._note(line)
                    continue
                if line.startswith("stage,"):
                    continue
                p = line.split(",")
                if len(p) != 7:
                    raise ValueError(f"{path}: not a stage row: {line}")
                stage, tsc, stream, seq, _, value, _ = p
                self.rows[(int(stream), int(seq))][stage].append((int(tsc), int(value)))
                self.count += 1

    def _note(self, line):
        m = re.match(r"# clock \w+ tsc=(\d+) mono_raw_ns=(\d+)", line)
        if m:
            self.clocks.append((int(m.group(1)), int(m.group(2))))
            return
        m = re.match(r"# drops ring \d+ (\d+)", line)
        if m:
            self.drops += int(m.group(1))
            return
        for k, v in re.findall(r"(\w+)=(\S+)", line):
            self.meta[k] = v

    def ticks_per_ns(self):
        if "ticks_per_ns" in self.meta:
            return float(self.meta["ticks_per_ns"])
        if len(self.clocks) >= 2:
            (t0, n0), (t1, n1) = self.clocks[0], self.clocks[-1]
            if n1 > n0 and t1 > t0:
                return (t1 - t0) / (n1 - n0)
        return None


class Host:
    """One host's stamps: its app file's and its driver file's."""

    def __init__(self, app, driver):
        self.app, self.driver = app, driver
        tpn = (app.ticks_per_ns() if app else None) or (driver.ticks_per_ns() if driver else None)
        if not tpn:
            raise ValueError("no counter rate: neither the app file's ticks_per_ns nor the driver file's clock lines")
        self.tpn = tpn
        self.dups = collections.Counter()

    def first(self, key, stage):
        """(tsc, value) of a message's first stamp at a stage, or None."""
        for f in (self.app, self.driver):
            if f is None:
                continue
            got = f.rows.get(key, {}).get(stage)
            if got:
                if len(got) > 1:
                    self.dups[stage] += 1
                return min(got)
        return None

    def ns(self, ticks):
        return ticks / self.tpn


def pct(sorted_values, q):
    if not sorted_values:
        return None
    return sorted_values[min(len(sorted_values) - 1, int(round((len(sorted_values) - 1) * q)))]


class Breakdown:
    """The stages of every complete measured message of a point."""

    def __init__(self, a, b):
        self.a, self.b = a, b
        meta = a.app.meta
        s = int(meta.get("stream_id", 0))
        off = int(meta.get("echo_offset", 10000))
        streams = int(meta.get("streams", 1))
        start = int(meta.get("measure_from_tsc", 0))
        self.values = collections.defaultdict(list)  # stage: [(ns, key, t_s)]
        self.missing = collections.Counter()
        self.messages = 0
        self.hw = False
        offers = [(k, v) for k, v in a.app.rows.items()
                  if s <= k[0] < s + streams and v.get("app-offer")]
        for ping, _ in sorted(offers):
            echo = (ping[0] + off, ping[1])
            offer = a.first(ping, "app-offer")
            if offer is None or offer[0] < start:
                continue
            self.messages += 1
            t_s = a.ns(offer[0] - start) / 1e9 if start else 0.0
            st = {}
            for host, name, key, stages in (
                (a, "a", ping, ("app-offered", "tx-send", "tx-call", "tx-done")),
                (b, "b", ping, ("rx-burst", "rx-hw", "rx-dispatch", "rx-inserted", "app-read")),
                (b, "b", echo, ("app-offer", "app-offered", "tx-send", "tx-call", "tx-done")),
                (a, "a", echo, ("rx-burst", "rx-hw", "rx-dispatch", "rx-inserted", "app-read")),
            ):
                leg = "out" if key == ping else "back"
                for stage in stages:
                    got = host.first(key, stage)
                    if got is not None:
                        st[(name, leg, stage)] = got
            st[("a", "out", "app-offer")] = offer
            need = [("a", "out", "tx-done"), ("b", "out", "rx-burst"), ("b", "out", "app-read"),
                    ("b", "back", "app-offer"), ("b", "back", "tx-done"), ("a", "back", "rx-burst"),
                    ("a", "back", "app-read")]
            gone = [k for k in need if k not in st]
            for k in gone:
                self.missing["/".join(k)] += 1
            if gone:
                continue
            self._message(st, ping, t_s)

    def _add(self, stage, ns, key, t_s):
        self.values[stage].append((ns, key, t_s))

    def _message(self, st, key, t_s):
        a, b = self.a, self.b
        hosts = {"a": a, "b": b}

        def d(h, leg, s1, leg2, s2):
            x, y = st.get((h, leg, s1)), st.get((h, leg2, s2))
            if x is None or y is None:
                return None
            return hosts[h].ns(y[0] - x[0])

        for h, leg in (("a", "out"), ("b", "back")):
            for stage, s1, s2 in (("offer call", "app-offer", "app-offered"), ("app->sender", "app-offer", "tx-send"),
                                  ("sender->tx", "tx-send", "tx-call"), ("tx burst", "tx-call", "tx-done")):
                v = d(h, leg, s1, leg, s2)
                if v is not None:
                    self._add(f"{h} {stage}", v, key, t_s)
        waits = []
        for h, leg in (("b", "out"), ("a", "back")):
            for stage, s1, s2 in (("rx->dispatch", "rx-burst", "rx-dispatch"), ("insert", "rx-dispatch", "rx-inserted"),
                                  ("insert->app", "rx-inserted", "app-read")):
                v = d(h, leg, s1, leg, s2)
                if v is not None:
                    self._add(f"{h} {stage}", v, key, t_s)
            hw = st.get((h, leg, "rx-hw"))
            if hw is not None:
                self.hw = True
                # the NIC's arrival: the counter at the realtime read less
                # the ns from the NIC's stamp to that read
                arrival = hw[0] - hw[1] * hosts[h].tpn
                wait = hosts[h].ns(st[(h, leg, "rx-burst")][0] - arrival)
                waits.append(wait)
                self._add(f"{h} rx ring wait", wait, key, t_s)
        self._add("b turnaround", d("b", "out", "app-read", "back", "app-offer"), key, t_s)
        rtt = d("a", "out", "app-offer", "back", "app-read")
        host_a = d("a", "out", "app-offer", "out", "tx-done") + d("a", "back", "rx-burst", "back", "app-read")
        host_b = d("b", "out", "rx-burst", "back", "tx-done")
        self._add("host a", host_a, key, t_s)
        self._add("host b", host_b, key, t_s)
        self._add("wire", rtt - host_a - host_b, key, t_s)
        if len(waits) == 2:
            self._add("wire less ring waits", rtt - host_a - host_b - sum(waits), key, t_s)
        self._add("round trip", rtt, key, t_s)

    def table(self):
        """[(stage, n, p50, p99, max, (key, t_s) of max, flag)] in stage order."""
        order = []
        for leg in (("a", "out"), ("b", "out"), ("b", "back"), ("a", "back")):
            h = leg[0]
            names = ["offer call", "app->sender", "sender->tx", "tx burst"] if (leg in (("a", "out"), ("b", "back"))) \
                else ["rx ring wait", "rx->dispatch", "insert", "insert->app"]
            if leg == ("b", "back"):
                order.append("b turnaround")
            order += [f"{h} {n}" for n in names]
        order += ["host a", "host b", "wire", "wire less ring waits", "round trip"]
        out = []
        for stage in order:
            vals = self.values.get(stage)
            if not vals:
                continue
            ns = sorted(v[0] for v in vals)
            top = max(vals, key=lambda v: v[0])
            p50, p99, mx = pct(ns, 0.5), pct(ns, 0.99), ns[-1]
            flags = []
            base = stage.split(" ", 1)[1] if stage[:2] in ("a ", "b ") else stage
            limit = EXPECT_NS.get(base)
            if limit is not None:
                v = p99 if base in P99_FLAGGED else p50
                if v > limit:
                    flags.append(f"{'p99' if base in P99_FLAGGED else 'p50'} {v:.0f} ns > {limit} ns")
            if stage == "wire" and p50 < 0:
                flags.append("negative: a host's stamps out of order")
            if mx > MAX_NS and stage not in ("round trip",):
                flags.append(f"max {mx / 1000:.1f} us at stream {top[1][0]} seq {top[1][1]} ({top[2]:.3f} s in)")
            out.append((stage, len(ns), p50, p99, mx, (top[1], top[2]), "; ".join(flags)))
        return out

    def notes(self):
        n = []
        if self.messages == 0:
            n.append("no measured sampled message: no app-offer past measure_from_tsc")
        for k, c in sorted(self.missing.items()):
            share = c / self.messages if self.messages else 1
            flag = " FLAG" if share > MISSING_SHARE else ""
            n.append(f"{c} of {self.messages} messages without {k}{flag}")
        for name, h in (("a", self.a), ("b", self.b)):
            for stage, c in sorted(h.dups.items()):
                n.append(f"host {name}: {c} messages stamped twice at {stage} (a resend: FLAG)")
            for f in (h.app, h.driver):
                if f is not None and (f.drops or int(f.meta.get("dropped", 0) or 0)):
                    n.append(f"host {name}: {f.drops or f.meta.get('dropped')} stamps dropped by a full ring "
                             f"({os.path.basename(f.path)}) FLAG")
        if not self.hw:
            n.append("no rx-hw (NIC RX timestamps off): each receive ring's wait is in the wire")
        return n


def breakdown_of(ping_app, pong_app, ping_driver, pong_driver):
    a = Host(StageFile(ping_app), StageFile(ping_driver) if ping_driver else None)
    b = Host(StageFile(pong_app), StageFile(pong_driver) if pong_driver else None)
    return Breakdown(a, b)


def point_files(d):
    return (find(os.path.join(d, "0"), "stages-app.csv"), find(os.path.join(d, "pong"), "stages-app.csv"),
            find(d, "stages-driver-a.csv"), find(d, "stages-driver-b.csv"))


def render(title, bd):
    lines = [f"### {title}", "", f"{bd.messages} measured sampled messages", "",
             "| stage | n | p50 ns | p99 ns | max ns | max at | flag |", "|---|---:|---:|---:|---:|---|---|"]
    for stage, n, p50, p99, mx, (key, t_s), flag in bd.table():
        lines.append(f"| {stage} | {n} | {p50:.0f} | {p99:.0f} | {mx:.0f} | {key[0]}/{key[1]} {t_s:.3f} s | {flag} |")
    lines.append("")
    lines += [f"- {n}" for n in bd.notes()]
    return "\n".join(lines) + "\n"


# ---- capture ----

AERON_TYPES = {0: "pad", 1: "data", 2: "nak", 3: "sm", 4: "err", 5: "setup", 6: "rttm", 7: "res",
               8: "ats-data", 9: "ats-setup", 10: "ats-sm", 11: "rsp-setup"}


def pcapng_packets(path):
    """(interface, outbound, ns, wire len, bytes) of each enhanced packet."""
    with (gzip.open(path, "rb") if path.endswith(".gz") else open(path, "rb")) as f:
        data = f.read()
    at, out, names = 0, [], []
    while at + 12 <= len(data):
        t, ln = struct.unpack_from("<II", data, at)
        if ln < 12 or at + ln > len(data):
            raise ValueError(f"{path}: block at {at} runs past the file")
        body = data[at + 8:at + ln - 4]
        if t == 1:
            names.append(len(names))
        elif t == 6:
            iface, hi, lo, cap, orig = struct.unpack_from("<IIIII", body, 0)
            pkt = body[20:20 + cap]
            opts = body[20 + ((cap + 3) & ~3):]
            outbound = None
            while len(opts) >= 4:
                code, olen = struct.unpack_from("<HH", opts, 0)
                if code == 0:
                    break
                if code == 2 and olen == 4:
                    outbound = (struct.unpack_from("<I", opts, 4)[0] & 3) == 2
                opts = opts[4 + ((olen + 3) & ~3):]
            out.append((iface, outbound, (hi << 32) | lo, orig, pkt))
        at += ln
    return out


def capture_report(path, every=None):
    """A capture's accounting: (lines, flags)."""
    pkts = pcapng_packets(path)
    count = collections.Counter()
    sizes = collections.Counter()
    data_seen = collections.Counter()  # (dir, stream, term_offset) -> frames
    per_message = collections.Counter()  # (dir, stream, seq) -> data frames
    frames_per_dgram = collections.Counter()
    unexplained = collections.Counter()
    first = last = None
    for iface, outbound, ns, orig, p in pkts:
        first = ns if first is None else min(first, ns)
        last = ns if last is None else max(last, ns)
        d = "tx" if outbound else "rx"
        port = ("sender", "receiver")[iface] if iface < 2 else str(iface)
        where = f"{port} {d}"
        if len(p) < 14:
            unexplained[f"{where} runt"] += 1
            continue
        et = struct.unpack_from(">H", p, 12)[0]
        if et == 0x0806:
            count[(where, "arp")] += 1
            continue
        if et != 0x0800 or len(p) < 34:
            unexplained[f"{where} ethertype 0x{et:04x}"] += 1
            continue
        proto = p[23]
        if proto == 1:
            count[(where, "icmp")] += 1
            continue
        if proto != 17:
            unexplained[f"{where} ip proto {proto}"] += 1
            continue
        ihl = (p[14] & 0x0f) * 4
        payload = p[14 + ihl + 8:]
        sizes[(where, orig)] += 1
        off, frames = 0, 0
        while off + 8 <= len(payload):
            flen, = struct.unpack_from("<i", payload, off)
            ftype, = struct.unpack_from("<H", payload, off + 6)
            name = AERON_TYPES.get(ftype)
            if name is None:
                unexplained[f"{where} aeron type 0x{ftype:04x}"] += 1
                break
            if name == "data" and flen <= 0:
                count[(where, "heartbeat")] += 1
                break
            if name in ("data", "pad") and off + 32 <= len(payload):
                frames += 1
                count[(where, name)] += 1
                if name == "data":
                    stream, = struct.unpack_from("<i", payload, off + 16)
                    term_off, = struct.unpack_from("<i", payload, off + 8)
                    data_seen[(d, stream, term_off)] += 1
                    if off + 40 <= len(payload):
                        seq, = struct.unpack_from("<Q", payload, off + 32)
                        per_message[(d, stream, seq)] += 1
                off += (flen + 31) & ~31
                if flen <= 0:
                    break
                continue
            count[(where, name)] += 1
            break
        if frames:
            frames_per_dgram[frames] += 1
    span = (last - first) / 1e9 if first is not None and last is not None and last > first else 0.0
    lines = [f"capture {os.path.basename(path)}: {len(pkts)} frames over {span:.3f} s"]
    for (where, kind), c in sorted(count.items()):
        rate = f" ({c / span:.0f}/s)" if span else ""
        lines.append(f"  {where:<14} {kind:<10} {c}{rate}")
    for (where, size), c in sorted(sizes.items()):
        lines.append(f"  {where:<14} size {size:<6} {c}")
    lines.append("  data frames a datagram: " + ", ".join(f"{k}: {v}" for k, v in sorted(frames_per_dgram.items())))
    flags = []
    resent = sum(1 for (d, _, _), c in data_seen.items() if d == "tx" and c > 1)
    dup_msgs = sum(1 for (d, _, _), c in per_message.items() if c > 1)
    naks = sum(c for (w, k), c in count.items() if k == "nak")
    arps = sum(c for (w, k), c in count.items() if k == "arp")
    icmp = sum(c for (w, k), c in count.items() if k == "icmp")
    lines.append(f"  retransmitted data frames (a term offset sent twice): {resent}")
    lines.append(f"  messages seen in more than one data frame: {dup_msgs}")
    if naks:
        flags.append(f"{naks} NAKs: a gap the receiver saw")
    if resent:
        flags.append(f"{resent} data frames sent twice: retransmits")
    if dup_msgs:
        flags.append(f"{dup_msgs} messages in more than one frame")
    # md-dpdk refreshes each next hop's ARP about once a minute and answers
    # the peer's: more than a few a second is something else
    if span and arps / span > 4:
        flags.append(f"{arps} ARP frames ({arps / span:.1f}/s): more than refreshes and replies")
    if icmp:
        flags.append(f"{icmp} ICMP frames")
    for k, c in sorted(unexplained.items()):
        flags.append(f"{c} unexplained frames: {k}")
    if every:
        sampled = sum(1 for (d, _, seq) in per_message if d == "tx" and seq % every == 0)
        lines.append(f"  sampled messages sent (seq % {every} == 0): {sampled}")
    return lines, flags


# ---- the A/A overhead check ----

def overhead(session, base="dpdk-aeron-rel", traced="dpdk-aeron-rel-trace"):
    """[(set, streams, rate, metric, base, aa, traced, spread, shift, flag)]:
    per rep the traced arm's and the A/A arm's p50 and p99 less the base
    arm's; the spread is the A/A's largest, the shift the traced arm's
    median."""
    path = os.path.join(session, "results.csv")
    if not os.path.exists(path):
        return []
    by = collections.defaultdict(dict)  # (set, streams, rate, rep) -> arm -> row
    with open(path) as f:
        for row in csv.reader(f):
            if len(row) < 13 or ":" not in row[2]:
                continue
            set_, _, mode = row[2].partition(":")
            parts = mode.split("@")
            streams = next((int(p[8:]) for p in parts[1:] if p.startswith("STREAMS=")), 1)
            if any(p.startswith("NETEM=") for p in parts[1:]):
                continue
            rate = int(row[5])
            by[(set_, streams, rate, row[0])][parts[0]] = row
    out = []
    groups = collections.defaultdict(list)
    for (set_, streams, rate, rep), arms in by.items():
        if base in arms and traced in arms and "aa" in arms:
            groups[(set_, streams, rate)].append(arms)
    for (set_, streams, rate), reps in sorted(groups.items()):
        for metric, col in (("p50", 7), ("p99", 9)):
            b = [float(r[base][col]) for r in reps]
            aa = [float(r["aa"][col]) - float(r[base][col]) for r in reps]
            tr = [float(r[traced][col]) - float(r[base][col]) for r in reps]
            spread = max(abs(x) for x in aa)
            shift = sorted(tr)[len(tr) // 2]
            flag = "FLAG: past the A/A spread" if abs(shift) > spread else ""
            out.append((set_, streams, rate, metric, sorted(b)[len(b) // 2], spread, shift, len(reps), flag))
    return out


def run(session):
    md, rows = ["# Stage trace", ""], []
    oh = overhead(session)
    if oh:
        md += ["## Tracing overhead (A/A)", "",
               "| set | streams | rate | metric | base median ns | A/A spread ns | traced shift ns | reps | flag |",
               "|---|---:|---:|---|---:|---:|---:|---:|---|"]
        for set_, streams, rate, metric, b, spread, shift, n, flag in oh:
            md.append(f"| {set_} | {streams} | {rate} | {metric} | {b:.0f} | {spread:.0f} | {shift:+.0f} | {n} | {flag} |")
        md.append("")
    pts = os.path.join(session, "points")
    for pid in sorted(os.listdir(pts)) if os.path.isdir(pts) else []:
        d = os.path.join(pts, pid)
        files = point_files(d)
        if files[0] and files[1]:
            try:
                bd = breakdown_of(*files)
            except (OSError, ValueError) as e:
                md += [f"### {pid}", "", f"unreadable: {e}", ""]
                continue
            md.append(render(pid, bd))
            for stage, n, p50, p99, mx, (key, t_s), flag in bd.table():
                rows.append([pid, stage, n, f"{p50:.0f}", f"{p99:.0f}", f"{mx:.0f}", f"{key[0]}/{key[1]}",
                             f"{t_s:.3f}", flag])
        for h in ("a", "b"):
            cap = find(d, f"capture-{h}.pcapng")
            if cap:
                lines, flags = capture_report(cap)
                md += [f"#### {pid} capture, host {h}", "", "```"] + lines + ["```", ""]
                md += [f"- FLAG {x}" for x in flags] or ["- nothing unexplained"]
                md.append("")
    with open(os.path.join(session, "stages.md"), "w") as f:
        f.write("\n".join(md) + "\n")
    with open(os.path.join(session, "stages.csv"), "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["point", "stage", "n", "p50_ns", "p99_ns", "max_ns", "max_at", "max_at_s", "flag"])
        w.writerows(rows)
    return "\n".join(md) + "\n"


def main(argv):
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    cmd, args = argv[0], argv[1:]
    try:
        if cmd == "point":
            files = point_files(args[0])
            if not (files[0] and files[1]):
                print(f"{args[0]}: no 0/stages-app.csv and pong/stages-app.csv", file=sys.stderr)
                return 1
            sys.stdout.write(render(os.path.basename(os.path.normpath(args[0])), breakdown_of(*files)))
        elif cmd == "join" and len(args) == 4:
            sys.stdout.write(render("join", breakdown_of(*args)))
        elif cmd == "run":
            sys.stdout.write(run(args[0]))
        elif cmd == "capture":
            lines, flags = capture_report(args[0], int(args[1]) if len(args) > 1 else None)
            print("\n".join(lines))
            print("\n".join(f"FLAG {x}" for x in flags) or "nothing unexplained")
        else:
            print(__doc__, file=sys.stderr)
            return 2
    except (OSError, ValueError) as e:
        print(f"transport_stages.py: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
