#!/usr/bin/env python3
"""Whether a transport run's comparisons are fair (transport-format.md).

fairness.py <run dir>    prints every violation; exit 1 if there is one

check(run_dir) returns [Violation]: rule, set (None: the whole run), the
point tags it concerns (empty: its whole set, or run) and what is wrong.
analyse.py --transport refuses every table holding a point a violation
concerns. Rules:

  manifest         every row has a manifest that agrees with it, every manifest a row
  payload          one payload (sha256, size) per round; wire bytes as the row says,
                   the A/A's its arm's, each arm's the same in every round of its set
  build            one core build per round
  schedule         one rate, duration, warm-up, stream count and size per round
  placement        every thread over 90 % CPU alone on a hot CPU; none on a
                   housekeeping or IRQ CPU (but the IRQ CPU's own ksoftirqd:
                   the IRQ core saturated, a flag of the point,
                   transport_points.flags); samples from every host
  irq              no data-NIC IRQ on a hot CPU; IRQs read on every host
  hostcheck        every host's report clean (no warn, no unproven blocking check),
                   and no warning of a binary's own checks in the point's logs
                   (<run>/logs/<runner point>.out on a, .pong on its echo's host:
                   ergon-io's `warning: <id> warn:` lines)
  settings         every knob as transport-settings.env gives it, none it does not
  shared-knob      a per-arm knob the round's arms run at different values swept
                   for every arm of the round that has it (a declared arm's own
                   knob excepted, and an arm at its technology's declared best:
                   best.knob.<name>, the data NICs' interrupt deferral); never
                   also shared
  napi             a socket arm's point (sockets set-up, or a tcp/udp transport),
                   and an Onload arm's (onload set-up: Onload polls the AF_XDP
                   ring the NIC's NAPI fills at each interrupt),
                   with interrupt deferral off on every data NIC of every host
                   (knob.napi_defer read back 0, each NIC 0/0 where the runner
                   recorded them): its epoll never busy-polls, so a deferred
                   interrupt leaves each packet waiting for the gro_flush timer;
                   only a tuning session's frozen choice (the arm swept for it)
                   may run it on
  onload           an Onload arm's point (onload set-up) had an Onload stack of
                   its process (its binary, or the media driver) on every host
                   mid-window (vm.sh onload-stacks: onload_<host> ok), ran
                   under onload (onload_cmd), and in a round one onload_cmd
                   and one Onload build a host (onload_build_<host>)
  declared         an arm declared another's twin but for one knob (aeron-ipc-huge:
                   aeron-ipc on 2 MiB pages) runs in every round with it, and
                   nothing else differs: its other per-arm knobs, transport, wire
                   bytes, binary, arguments and media driver mode are the twin's
                   (a driver knob's own part of the mode apart: aeron_threading's
                   AERON_THREADING_MODE and the agents it drops, aeron_driver's
                   DRIVER). A twin may be another's (dpdk-aeron-rel-invoker:
                   dpdk-aeron-rel-emb's, which is dpdk-aeron-rel's): each knob
                   along the chain is the arm's own difference
  order            a seed and the round's arms in its recorded order; seeds
                   distinct and orders not all the same across reps. An arm
                   the order names with no row in the round is absent, not a
                   violation, only where failures.csv records it failed there
                   (that rep, netem and stream count, or its set-up): it is
                   left out of that round's comparison (check's absences,
                   which fairness.py prints and analyse.py states)
  aa               an A/A arm in every round, repeating an arm of the round
  loss-accounting  loss columns filled (retrans where the transport retransmits:
                   TCP, Aeron); counters from every host (TCP: tcp_info
                   and nstat; Aeron: aeron; UDP: nstat; a transport joined by +,
                   such as udp+tcp, every part's)
  loss             a TCP point's real loss (retrans_not_dsack, lost_end) on a row
                   without loss (no netem, none, 0 %) where the guest's own drop
                   counters grew is a bug to trace; with none grown it was lost
                   below the guest's stack (the NIC or the network), a note
                   (path_loss), as an Aeron row's retransmits there are;
                   DSACK-reported (spurious) retransmits are reported, not loss
  lateness         scheduled and late counted, as the manifest says
  sink             a sink arm's run (aeron_lat --mode spy-udp) has its sink's manifest,
                   the sink read every ping the run sent (and in a round trip every
                   echo), in order, and host a's driver sent at least every frame's
                   bytes (driver.bytes-sent): the spy read a stream that crossed the wire
  max              every max labelled (spikes.py); unexplained blocks
  netem            a loss row's netem on every data NIC of every host at its
                   loss, its drops inside a 99.9 % binomial interval; none on
                   other rows; no DPDK in a loss row
  vm-pair          one VM pair per set
"""
import csv
import math
import os
import re
import sys
from collections import defaultdict, namedtuple

from spikes import transport_maxima
from transport_points import hosts_of_point, irq_core_thread, read_points, tcp_loss

Violation = namedtuple("Violation", "rule set tags detail")
Violation.__str__ = lambda v: f"{v.rule}: {v.set or 'run'}: {v.detail}"
# an arm a round's order names that has no row there, excused by a recorded failure
Absence = namedtuple("Absence", "set rep netem streams arm detail")
Absence.__str__ = lambda a: (f"absent: {a.set}: rep {a.rep}, {a.streams} stream{'s' if a.streams != 1 else ''}, "
                             f"netem {a.netem or '-'}: {a.arm} {a.detail}")
# a binary's own check warning (ergon-io's host checks): "<who> warning: <id> warn: <finding>. Fix: ..."
BINARY_WARN = re.compile(r"warning: (\w+) warn: (.*)$")
# a binary's periodic checks whose first warning covers the process's start (io/src/warn.rs FAULTS_EVERY)
START_CHECKS = {"T19"}
# what the run reports beside its violations: a path's loss, say
Note = namedtuple("Note", "rule set tags detail")
Note.__str__ = lambda n: f"note: {n.rule}: {n.set or 'run'}: {n.detail}"
# the guest's own drop counters (nstat's names): a segment the guest's
# stack received and threw away
GUEST_DROPS = {"TcpExtTCPBacklogDrop", "TcpExtTCPRcvQDrop", "TcpExtTCPOFODrop", "TcpExtTCPZeroWindowDrop",
               "TcpExtPFMemallocDrop", "TcpExtTCPMinTTLDrop", "TcpExtTCPDeferAcceptDrop", "TcpExtLockDroppedIcmps",
               "TcpExtListenDrops", "TcpExtTCPReqQFullDrop", "IpInDiscards", "IpInHdrErrors", "IpInAddrErrors",
               "TcpInErrs", "TcpInCsumErrors", "UdpRcvbufErrors", "UdpInErrors", "UdpMemErrors"}

BUSY = 0.9  # a thread over this share of a CPU is a busy thread
ALONE = 0.05  # another thread over this share of the busy thread's CPU shares it
Z = 3.29  # the binomial interval's half-width in standard deviations (99.9 %, two-sided)
# ergon-tune's ids whose unknown does not stop a require start (tune/src/report.rs)
SUBSYSTEM_CHECKS = {"T3", "T4", "T5", "T6", "T11", "T13", "T14", "T15", "T16", "T17", "T18", "T19", "K6", "K7", "N1",
                    "N2", "N3", "N4", "M4"}
FINDING = re.compile(r"^(\w+) (ok|info|warn|unknown):")
REQUIRED = ("set", "arm", "round", "rep", "rate", "streams", "size", "duration_s", "warmup_s", "payload_sha256",
            "wire_bytes", "core_build", "arm_order", "vms", "cpus", "threads", "irqs", "hostcheck", "settings",
            "loss_counters", "transport")
LOSS_SOURCES = {"tcp": ("tcp_info", "nstat"), "udp": ("nstat",), "aeron-udp": ("aeron",), "aeron-dpdk": ("aeron",),
                "aeron-ipc": ("aeron",), "aeron-spy": ("aeron",), "aeron-spy-udp": ("aeron",),
                "aeron-spy-dpdk": ("aeron",), "mpsc": ()}
# Aeron's data frame: the message and its 32-byte header (aeron_lat.rs)
AERON_FRAME = 96



def loss_sources(transport):
    """The counter sources a transport's loss rows need: LOSS_SOURCES', and
    for one of several joined by + (udp+tcp) the union of its parts';
    None for a transport LOSS_SOURCES does not know (or one part of it)."""
    parts = (transport or "").split("+")
    if not all(p in LOSS_SOURCES for p in parts):
        return None
    return tuple(sorted({s for p in parts for s in LOSS_SOURCES[p]}))


def retransmits(transport):
    """Whether a transport (or a part of a +-joined one) retransmits, so its
    rows must count it: TCP and Aeron's."""
    return any(p == "tcp" or p.startswith("aeron") for p in (transport or "").split("+"))


def sink_problems(p):
    """A sink arm's run (the runner's arm_hosts=sink): what its sink's
    manifest (collected as "sink") says against the run's own: a sink that
    read fewer pings than the run sent (or, in a round trip, fewer echoes
    than it echoed), out of order, or a driver on host a that sent fewer
    bytes than the frames; [] for any other point."""
    m = p.manifest or {}
    if (m.get("runner") or {}).get("arm_hosts") != "sink":
        return []
    bs = (m.get("binary") or {}).get("settings") or {}
    ss = (m.get("sink") or {}).get("settings")
    if ss is None:
        return [f"{p.tag}: no sink manifest: its frames' arrival on b unproven"]
    num = lambda d, k: int(d[k]) if str(d.get(k, "")).isdigit() else None
    sent, echoes = num(bs, "sent"), num(bs, "echoes_sent") if p.kind == "load" else 0
    ping, echo = num(ss, "sink.ping_frames"), num(ss, "sink.echo_frames")
    out = []
    if None in (sent, echoes, ping, echo):
        return [f"{p.tag}: sink or run counts not recorded (sent {sent}, echoes {echoes}, sink {ping}/{echo})"]
    if ping < sent:
        out.append(f"{p.tag}: the sink received {ping:,} of {sent:,} pings")
    if echo < echoes:
        out.append(f"{p.tag}: the sink received {echo:,} of {echoes:,} echoes")
    for k in ("sink.gaps", "sink.behind", "sink.refused"):
        if num(ss, k) != 0:
            out.append(f"{p.tag}: the sink's {k[len('sink.'):]} {ss.get(k)}")
    sent_bytes = num(bs, "driver.bytes-sent")
    if sent_bytes is None or sent_bytes < (sent + echoes) * AERON_FRAME:
        out.append(f"{p.tag}: host a's driver sent {sent_bytes} bytes for {sent + echoes:,} frames")
    return out


def guest_drops(p):
    """{counter: growth} of the guest's own drop counters (GUEST_DROPS) over
    every host of a point (its loss_counters' nstat; nstat leaves zero ones
    out), the counters that grew."""
    out = defaultdict(int)
    for src in ((p.manifest or {}).get("loss_counters") or {}).values():
        for k, v in ((src or {}).get("nstat") or {}).items():
            if k in GUEST_DROPS and isinstance(v, int) and v > 0:
                out[k] += v
    return dict(out)


def lossless_row(p):
    """Whether a point's row has no loss of its own: no netem, the no-qdisc
    control, or netem at 0 %."""
    return p.netem in (None, "none") or float(p.netem) == 0


def tcp_loss_problem(p):
    """What is wrong with a TCP point's loss: real loss (retransmits not
    reported spurious, segments lost) on a row without loss (no netem, the
    no-qdisc control, or netem at 0 %) where the guest's own drop counters
    grew (a socket's backlog or receive queue full, an IP discard...) is a
    bug to trace; with none of them grown it was lost below the guest's
    stack, the NIC or the network (path_loss: a note); spurious
    retransmits are reported, never loss. None when nothing is."""
    loss = tcp_loss(p)
    if loss is None or not lossless_row(p):
        return None
    real, spurious = loss
    drops = guest_drops(p)
    if real and drops:
        return (f"{p.tag}: {real} segments lost (retransmits not DSACK-reported, or lost at the end) "
                f"on a row without loss ({spurious} spurious), and the guest dropped some "
                f"({', '.join(f'{k} {v}' for k, v in sorted(drops.items()))}): trace it")
    return None


def path_loss(p):
    """A note of a point's loss below the guest's stack on a row without
    loss: TCP's real loss (tcp_loss) with no guest drop counter grown, or
    the Aeron retransmits its row counts (NAKed gaps); None when there is
    none. The path's, as every arm's on it: reported, not a violation."""
    if not lossless_row(p):
        return None
    loss = tcp_loss(p)
    if loss is not None:
        real, spurious = loss
        if not real or guest_drops(p):
            return None
        per = ", ".join(f"{h} {((s or {}).get('tcp_info') or {}).get('retrans_not_dsack') or 0}"
                        for h, s in sorted(((p.manifest or {}).get("loss_counters") or {}).items())
                        if (s or {}).get("tcp_info"))
        return (f"{p.tag}: {real} segments lost below the guest's TCP stack (the NIC or the network: no guest drop "
                f"counter grew on either host; retransmits not DSACK-reported: {per}), {spurious} spurious, on a row "
                "without netem")
    if (p.manifest or {}).get("transport", "").startswith("aeron") and (p.row.get("retrans") or 0) > 0:
        return (f"{p.tag}: Aeron retransmitted {p.row['retrans']} (NAKed gaps) on a row without netem: lost below "
                "the guest's stack")
    return None


# a driver knob's own part of a media driver mode (transport.sh
# tr_driver_mode): the @KEY= overrides it sets or drops
DRIVER_MODE_KEYS = {
    "knob.aeron_threading": ("AERON_THREADING_MODE", "CONDUCTOR", "AERON_CONDUCTOR_IDLE_STRATEGY",
                             "AERON_SENDER_IDLE_STRATEGY", "AERON_RECEIVER_IDLE_STRATEGY"),
    "knob.aeron_driver": ("DRIVER",),
}


def driver_mode_less(mode, knob):
    """A media driver mode (<transport>@KEY=value@...) without the
    overrides `knob` itself sets or drops (DRIVER_MODE_KEYS); as it is for
    any other knob."""
    keys = DRIVER_MODE_KEYS.get(knob)
    if not keys or not isinstance(mode, str):
        return mode
    head, *pairs = mode.split("@")
    return "@".join([head] + [p for p in pairs if p.partition("=")[0] not in keys])


def read_env(path):
    """transport-settings.env as (shared {knob: value}, per arm {arm: {knob:
    value}}, sweep {knob: {arm}}, declared {arm: (twin, knob)}, best {knob:
    {arm: value}}), or None when missing."""
    if not os.path.exists(path):
        return None
    shared, per_arm, sweep, declared, best = {}, defaultdict(dict), {}, {}, {}
    with open(path) as f:
        text = f.read().splitlines()
    for line in text:
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        k, v = line.split("=", 1)
        prefix, _, knob = k.partition(".")
        if prefix == "shared":
            shared[knob] = v
        elif prefix == "sweep":
            sweep[knob] = {a.strip() for a in v.split(",") if a.strip()}
        elif prefix == "declared":
            twin, _, k = v.partition(":")
            declared[knob] = (twin, k)
        elif prefix == "best":
            best[knob] = dict(e.partition(":")[::2] for e in v.split(",") if ":" in e)
        else:
            per_arm[prefix][knob] = v
    return shared, per_arm, sweep, declared, best


SOCKET_TRANSPORTS = {"tcp", "udp"}


def socket_point(p):
    """Whether a point is a socket arm's: the runner's sockets set-up, else
    (no set-up recorded) a transport of the kernel's sockets alone."""
    setup = ((p.manifest or {}).get("runner") or {}).get("setup")
    if setup:
        return setup == "sockets"
    parts = (p.manifest or {}).get("transport", "").split("+")
    return bool(parts) and all(x in SOCKET_TRANSPORTS for x in parts)


def onload_point(p):
    """Whether a point is an Onload arm's: the runner's onload set-up."""
    return ((p.manifest or {}).get("runner") or {}).get("setup") == "onload"


def onload_problems(p, hosts):
    """What an Onload arm's point lacks (the onload rule): its run under
    onload recorded, an Onload stack of its process on every host it used
    (vm.sh onload-stacks, the runner's onload_<host>), its build read."""
    runner = (p.manifest or {}).get("runner") or {}
    out = []
    if not runner.get("onload_cmd"):
        out.append(f"{p.tag}: no onload_cmd: not shown to have run under onload")
    for host in hosts:
        real = ((p.manifest or {}).get("hosts") or {}).get(host, host)
        v = runner.get(f"onload_{real}") or "unread"
        if not v.startswith("ok"):
            out.append(f"{p.tag}: {host}'s Onload stack mid-window: {v}")
        if (runner.get(f"onload_build_{real}") or "unread") == "unread":
            out.append(f"{p.tag}: {host}'s Onload build not read")
    return out


def napi_on(p, host):
    """What shows a point's data NICs on host deferring interrupts: the
    knob read back (knob.napi_defer) not 0, or a NIC the runner read
    (napi_<host>: <nic>:<defer>/<gro_flush ns>,...) not 0/0; None when
    neither does; "unread" when neither was read."""
    got = ((p.manifest.get("settings") or {}).get(host) or {}).get("knob.napi_defer")
    host_of = (p.manifest.get("hosts") or {}).get(host, host)
    raw = ((p.manifest.get("runner") or {}).get(f"napi_{host_of}") or "")
    nics = [e for e in raw.split(",") if e]
    if got is None and not nics:
        return "unread"
    if got is not None and got != "0":
        return f"napi_defer {got}"
    if any(e.rpartition(":")[2] != "0/0" for e in nics):
        return raw
    return None


# findings the hostcheck binary reports without blocking (REPORTED_ONLY in
# md-dpdk/harness/src/bin/hostcheck.rs): T11, CLOCK_REALTIME's discipline,
# bounds only wall-clock stamps compared across hosts; every latency here is
# on one host's counter
REPORTED_ONLY = {"T11"}


def hostcheck_problems(path):
    """What keeps a hostcheck report from being clean ([] when clean)."""
    try:
        with open(path) as f:
            lines = f.read().splitlines()
    except OSError:
        return ["missing"]
    found = [FINDING.match(l) for l in lines]
    if not any(found):
        return ["no findings"]
    out = []
    for m, line in zip(found, lines):
        if m and m.group(1) in REPORTED_ONLY:
            continue
        if m and m.group(2) == "warn":
            out.append(line)
        elif m and m.group(2) == "unknown" and m.group(1) not in SUBSYSTEM_CHECKS:
            out.append(line)
    return out


def read_failures(run_dir):
    """failures.csv (transport.sh tr_set: set, rep, netem, arm, aa, then the
    stream count that failed or `setup`) as {(set, rep, netem, arm): [what]},
    the A/A arm as `aa`; {} when there is none."""
    out = defaultdict(list)
    path = os.path.join(run_dir, "failures.csv")
    if os.path.exists(path):
        with open(path, newline="") as f:
            for row in csv.reader(f):
                if len(row) >= 6 and row[1].isdigit():
                    out[(row[0], int(row[1]), row[2] or "-", "aa" if row[4] == "1" else row[3])].append(row[5])
    return out


def binary_warnings(run_dir, p):
    """{host: {check id: [finding]}}: the warnings of the point's binaries'
    own checks in the runner's logs (logs/<runner point>.out: the measuring
    run, host a; .pong: its echo, host b, or a on a one-host set), each
    finding cut at its first sentence; {} where the logs are not kept. The
    first of a log's T19 warnings (the loop thread's page faults since the
    last check, every 5 s) covers the process's start, set-up and warm-up,
    not the window: left out."""
    point = ((p.manifest or {}).get("runner") or {}).get("point")
    out = {}
    if not point:
        return out
    for suffix, host in ((".out", "a"), (".pong", hosts_of_point(p)[-1])):
        path = os.path.join(run_dir, "logs", point + suffix)
        if not os.path.exists(path):
            continue
        seen = set()
        with open(path, errors="replace") as f:
            for line in f:
                m = BINARY_WARN.search(line.rstrip("\n"))
                if not m:
                    continue
                if m.group(1) in START_CHECKS and m.group(1) not in seen:
                    seen.add(m.group(1))
                    continue
                finding = m.group(2).split(". Fix:")[0].split(". ")[0]
                out.setdefault(host, {}).setdefault(m.group(1), []).append(finding)
    return out


def within_binomial(offered, dropped, pct):
    """Whether `dropped` of `offered` is inside the Z interval of loss `pct` %."""
    p = pct / 100
    if p == 0:
        return dropped == 0
    if offered <= 0:
        return False
    mean = offered * p
    return abs(dropped - mean) <= Z * math.sqrt(offered * p * (1 - p)) + 1


def check(run_dir, absences=None, notes=None):
    """[Violation]; with absences (a list), every arm a round's order names
    that failed there (failures.csv) appended to it as an Absence; with
    notes (a list), what is reported and breaks no rule (a path's loss) as
    Notes."""
    out = []
    note = lambda rule, set_, tags, detail: notes is not None and notes.append(
        Note(rule, set_, tuple(sorted(tags)), detail))
    add = lambda rule, set_, tags, detail: out.append(Violation(rule, set_, tuple(sorted(tags)), detail))
    points = read_points(run_dir)
    failed = read_failures(run_dir)
    env = read_env(os.path.join(run_dir, "transport-settings.env"))
    if env is None:
        add("settings", None, (), "transport-settings.env missing")

    # manifests without a row
    tags = {p.tag for p in points}
    for set_ in sorted({p.set for p in points} | {d for d in os.listdir(run_dir)
                                                  if os.path.isdir(os.path.join(run_dir, d))}):
        folder = os.path.join(run_dir, set_)
        if not os.path.isdir(folder):
            continue
        for name in sorted(os.listdir(folder)):
            if name.startswith("manifest-") and name.endswith(".json"):
                tag = name[len("manifest-"):-len(".json")]
                if tag not in tags:
                    add("manifest", set_, (), f"{name} has no row in results.csv")

    good = []
    for p in points:
        m = p.manifest
        if m is None:
            add("manifest", p.set, (p.tag,), f"{p.tag}: no manifest")
            continue
        missing = [k for k in REQUIRED if k not in m]
        if missing:
            add("manifest", p.set, (p.tag,), f"{p.tag}: missing {', '.join(missing)}")
            continue
        wrong = [k for k, v in (("set", p.set), ("rep", p.rep), ("rate", p.rate), ("streams", p.streams),
                                ("size", p.size), ("label", p.label), ("kind", p.kind)) if k in m and m[k] != v]
        if (m.get("netem") or None) != (p.netem or None):
            wrong.append("netem")
        if wrong:
            add("manifest", p.set, (p.tag,), f"{p.tag}: manifest disagrees with the row on {', '.join(wrong)}")
            continue
        good.append(p)

    rounds = defaultdict(list)
    for p in good:
        rounds[p.manifest["round"]].append(p)
    arm_key = lambda p: p.manifest.get("aa_of") if p.manifest.get("aa") else p.arm

    for rid, ps in sorted(rounds.items()):
        set_ = ps[0].set
        tags = [p.tag for p in ps]

        def same(rule, what, value):
            seen = defaultdict(list)
            for p in ps:
                seen[repr(value(p))].append(p.arm)
            if len(seen) > 1:
                add(rule, set_, tags, f"round {rid}: {what} differs: "
                    + "; ".join(f"{v} ({', '.join(sorted(set(a)))})" for v, a in sorted(seen.items())))

        same("payload", "payload sha256", lambda p: p.manifest["payload_sha256"])
        same("payload", "payload size", lambda p: p.manifest["size"])
        same("build", "core build", lambda p: p.manifest["core_build"])
        for k in ("rate", "duration_s", "warmup_s", "streams", "size"):
            same("schedule", k, lambda p, k=k: p.manifest[k])
        # the round's Onload arms: one onload command and one build a host
        ol = [p for p in ps if onload_point(p)]
        for k in ("onload_cmd", "onload_build_a", "onload_build_b"):
            seen = defaultdict(list)
            for p in ol:
                seen[(p.manifest.get("runner") or {}).get(k)].append(p.arm)
            if len(seen) > 1:
                add("onload", set_, [p.tag for p in ol], f"round {rid}: {k} differs across its Onload arms: "
                    + "; ".join(f"{v} ({', '.join(sorted(set(a)))})" for v, a in sorted(seen.items(), key=str)))

        arms = {p.arm for p in ps}
        aas = [p for p in ps if p.manifest.get("aa")]
        if not aas:
            add("aa", set_, tags, f"round {rid}: no A/A arm")
        for p in aas:
            if p.manifest.get("aa_of") not in arms - {p.arm}:
                add("aa", set_, tags, f"round {rid}: A/A arm {p.arm} repeats {p.manifest.get('aa_of')!r}, "
                    "not an arm of the round")
            ref = [q for q in ps if q.arm == p.manifest.get("aa_of")]
            if ref and ref[0].manifest["wire_bytes"] != p.manifest["wire_bytes"]:
                add("payload", set_, tags, f"round {rid}: A/A {p.arm} sends {p.manifest['wire_bytes']} wire bytes, "
                    f"its arm {ref[0].arm} {ref[0].manifest['wire_bytes']}")

        orders = {tuple(p.manifest["arm_order"]) for p in ps}
        seeds = {p.manifest.get("seed") for p in ps}
        if None in seeds:
            add("order", set_, tags, f"round {rid}: no seed recorded")
        elif len(seeds) > 1:
            add("order", set_, tags, f"round {rid}: points record different seeds")
        if len(orders) > 1:
            add("order", set_, tags, f"round {rid}: points record different arm orders")
        p0 = ps[0]
        for order in orders:
            # an arm of the order with no row here: absent where its failure
            # in this round (its rep, netem and stream count, or its set-up)
            # is recorded, and left out of the round's comparison
            why = {a: failed.get((set_, p0.rep, p0.netem or "-", a), []) for a in set(order) - arms}
            excused = {a for a, what in why.items() if str(p0.streams) in what or "setup" in what}
            if sorted(a for a in order if a not in excused) != sorted(arms):
                unrecorded = sorted(set(why) - excused)
                add("order", set_, tags, f"round {rid}: arm order {', '.join(order)} is not the round's arms "
                    f"{', '.join(sorted(arms))}"
                    + (f" (no failure recorded for {', '.join(unrecorded)})" if unrecorded else ""))
            elif absences is not None:
                for a in sorted(excused):
                    what = sorted(set(why[a]))
                    absences.append(Absence(set_, p0.rep, p0.netem, p0.streams, a,
                                            "failed (" + ", ".join(w if w == "setup" else f"{w} streams" for w in what)
                                            + f": failures.csv), no row: left out of round {rid}'s comparison"))

        if env is not None:
            shared, per_arm, sweep, declared, best = env
            reported = defaultdict(set)  # knob: arms reporting it
            for p in ps:
                for host, knobs in p.manifest["settings"].items():
                    for k in knobs:
                        reported[k].add(arm_key(p))
            for p in ps:
                arm = arm_key(p)
                want = dict(shared, **per_arm.get(arm, {}))
                for host in hosts_of_point(p):
                    got = p.manifest["settings"].get(host)
                    if got is None:
                        add("settings", set_, [p.tag], f"{p.tag}: no settings read back on {host}")
                        continue
                    for k, v in sorted(want.items()):
                        if k not in got:
                            add("settings", set_, [p.tag], f"{p.tag}: {host} {k} not read back (want {v})")
                        elif str(got[k]) != v:
                            add("settings", set_, [p.tag], f"{p.tag}: {host} {k}={got[k]}, the env gives {v}")
                    for k in sorted(set(got) - set(want)):
                        add("settings", set_, [p.tag], f"{p.tag}: {host} {k}={got[k]} is not in the env for {arm}")
            for k in sorted(set(shared) & {k for knobs in per_arm.values() for k in knobs}):
                add("shared-knob", set_, tags, f"round {rid}: {k} is both shared and per arm")
            # a per-arm knob the round's arms run at different values: every
            # arm of the round that has it swept for it (one at the same
            # value everywhere is no arm's own tweak)
            round_arms = {arm_key(p) for p in ps}

            def own(arm, k):
                """k is arm's declared difference from a twin in the round,
                or its twin's from that twin's (a chain of twins): its value
                is what the pair compares (the declared rule checks the
                rest)."""
                seen = set()
                while arm in declared and arm not in seen and declared[arm][0] in round_arms:
                    seen.add(arm)
                    if declared[arm][1] == k:
                        return True
                    arm = declared[arm][0]
                return False

            for k in sorted({k for arm in round_arms for k in per_arm.get(arm, {})}):
                have = (reported[k] | {arm for arm in round_arms if k in per_arm.get(arm, {})})
                have = {arm for arm in have if not own(arm, k)}
                values = {per_arm[arm][k] for arm in have if k in per_arm.get(arm, {})}
                # an arm at its technology's declared best (best.<knob>): its
                # own value, needing no sweep; the others still do
                have = {arm for arm in have if best.get(k, {}).get(arm) != per_arm.get(arm, {}).get(k, "?")}
                swept = sweep.get(k, set())
                if len(values) > 1 and not have <= swept:
                    add("shared-knob", set_, tags, f"round {rid}: {k} differs across its arms "
                        f"({', '.join(a + ' ' + per_arm.get(a, {}).get(k, '?') for a in sorted(have))}), and "
                        f"{', '.join(sorted(have - swept))} was not swept for it "
                        f"(sweep.{k}={','.join(sorted(swept)) or 'none'})")
            for arm in sorted(round_arms & set(declared)):
                twin, k = declared[arm]
                if twin not in round_arms:
                    add("declared", set_, tags, f"round {rid}: {arm} is declared {twin}'s twin but for {k}, "
                        f"and {twin} is not in the round")
                    continue
                mine, theirs = per_arm.get(arm, {}), per_arm.get(twin, {})
                if k not in mine or k not in theirs:
                    add("declared", set_, tags, f"round {rid}: {arm}'s declared difference {k} is not a per-arm "
                        f"knob of both it and {twin}")
                diff = sorted(x for x in (set(mine) | set(theirs)) - {k} if mine.get(x) != theirs.get(x))
                if diff:
                    add("declared", set_, tags, f"round {rid}: {arm} differs from its twin {twin} beyond {k}: "
                        + ", ".join(f"{x} {mine.get(x, '-')} against {theirs.get(x, '-')}" for x in diff))
                pair = [p for p in ps if p.arm in (arm, twin) and not p.manifest.get("aa")]
                for what, value in (("transport", lambda p: p.manifest.get("transport")),
                                    ("wire bytes", lambda p: p.manifest.get("wire_bytes")),
                                    ("binary", lambda p: (p.manifest.get("runner") or {}).get("bin")),
                                    ("arguments", lambda p: (p.manifest.get("runner") or {}).get("args")),
                                    ("media driver mode",
                                     lambda p, k=k: driver_mode_less(
                                         (p.manifest.get("runner") or {}).get("driver_mode"), k))):
                    seen = sorted({repr(value(p)) for p in pair})
                    if len(seen) > 1:
                        add("declared", set_, tags, f"round {rid}: {arm} and its twin {twin} differ in {what}: "
                            + ", ".join(seen))

        for p in ps:
            m, hosts = p.manifest, hosts_of_point(p)
            for why in sink_problems(p):
                add("sink", set_, [p.tag], why)
            # a socket arm's NICs: interrupts not deferred (epoll never
            # busy-polls), but where a tuning session froze it on
            if socket_point(p) or onload_point(p):
                arm = arm_key(p)
                tuned = (env is not None and env[1].get(arm, {}).get("knob.napi_defer", "0") != "0"
                         and arm in env[2].get("knob.napi_defer", set()))
                for host in hosts:
                    on = napi_on(p, host)
                    if on == "unread":
                        add("napi", set_, [p.tag], f"{p.tag}: {host}'s data NICs' interrupt deferral not read back")
                    elif on and not tuned:
                        add("napi", set_, [p.tag], f"{p.tag}: {host}'s data NICs defer interrupts ({on}) under a "
                            + ("socket arm: its epoll never busy-polls, so each packet waits for the gro_flush timer"
                               if not onload_point(p) else "an Onload arm: its AF_XDP ring is filled by the NIC's "
                               "NAPI at each interrupt, so each packet waits for the gro_flush timer"))
            # an Onload arm's: accelerated on every host, under onload
            if onload_point(p):
                for why in onload_problems(p, hosts):
                    add("onload", set_, [p.tag], why)
            # wire bytes
            if p.row["wire_bytes"] != m["wire_bytes"]:
                add("payload", set_, [p.tag], f"{p.tag}: row says {p.row['wire_bytes']} wire bytes, the manifest "
                    f"{m['wire_bytes']}")
            # placement
            for host in hosts:
                c = m["cpus"].get(host) or {}
                hot, quiet = set(c.get("hot") or []), set(c.get("housekeeping") or []) | set(c.get("irq") or [])
                samples = m["threads"].get(host)
                if not samples:
                    add("placement", set_, [p.tag], f"{p.tag}: no thread placement samples on {host}")
                    continue
                for i, sample in enumerate(samples):
                    for t in sample:
                        if t.get("util", 0) <= BUSY:
                            continue
                        if irq_core_thread(t, c):
                            continue  # the IRQ core saturated: the point's flag (transport_points.flags)
                        who = f"{host} {t.get('comm')} ({t.get('tid')}) at {t.get('util', 0):.0%} on CPU {t.get('cpu')}"
                        if t.get("cpu") in quiet:
                            add("placement", set_, [p.tag], f"{p.tag}: sample {i}: {who}, a housekeeping/IRQ CPU")
                        elif t.get("cpu") not in hot:
                            add("placement", set_, [p.tag], f"{p.tag}: sample {i}: {who}, not a hot CPU")
                        others = [o for o in sample if o is not t and o.get("cpu") == t.get("cpu")
                                  and o.get("util", 0) > ALONE]
                        if others:
                            add("placement", set_, [p.tag], f"{p.tag}: sample {i}: {who} shares it with "
                                + ", ".join(f"{o.get('comm')} {o.get('util', 0):.0%}" for o in others))
                # IRQs
                irqs = m["irqs"].get(host)
                if irqs is None:
                    add("irq", set_, [p.tag], f"{p.tag}: IRQs not read on {host}")
                else:
                    for q in irqs:
                        if q.get("data_nic") and q.get("delta", 0) > 0 and q.get("cpu") in hot:
                            add("irq", set_, [p.tag], f"{p.tag}: {host} data NIC IRQ {q.get('irq')} {q.get('name')} "
                                f"x{q.get('delta')} on hot CPU {q.get('cpu')}")
                # hostcheck
                rel = m["hostcheck"].get(host)
                problems = (["not recorded"] if not rel
                            else hostcheck_problems(os.path.join(run_dir, p.set, rel)))
                if problems:
                    add("hostcheck", set_, [p.tag], f"{p.tag}: {host} hostcheck {rel}: {'; '.join(problems)}")
            for host, checks in sorted(binary_warnings(run_dir, p).items()):
                for cid, findings in sorted(checks.items()):
                    add("hostcheck", set_, [p.tag], f"{p.tag}: {host} {p.arm}'s own check {cid} warned "
                        f"{len(findings)}x: {findings[0]}")
            for host in hosts:
                # loss counters
                sources = (m["loss_counters"].get(host) or {})
                for s in loss_sources(m["transport"]) or ():
                    if s not in sources:
                        add("loss-accounting", set_, [p.tag], f"{p.tag}: no {s} counters on {host}")
                if "tcp_info" in sources and "retrans_not_dsack" not in (sources["tcp_info"] or {}):
                    add("loss-accounting", set_, [p.tag], f"{p.tag}: {host}'s tcp_info has no DSACK split "
                        "(retrans_not_dsack): spurious and real retransmits cannot be told apart")
            if loss_sources(m["transport"]) is None:
                add("loss-accounting", set_, [p.tag], f"{p.tag}: unknown transport {m['transport']!r}")
            # retrans: a count only where the transport retransmits (TCP, Aeron);
            # empty is the row's honest "no such counter" for UDP and mpsc
            need = ("lost", "reordered", "dup") + (("retrans",) if retransmits(m["transport"]) else ())
            empty = [c for c in need if p.row[c] is None]
            if empty:
                add("loss-accounting", set_, [p.tag], f"{p.tag}: row has no {', '.join(empty)}")
            why = tcp_loss_problem(p)
            if why:
                add("loss", set_, [p.tag], why)
            elif path_loss(p):
                note("loss", set_, [p.tag], path_loss(p))
            # lateness
            if p.row["scheduled"] is None or p.row["late"] is None:
                add("lateness", set_, [p.tag], f"{p.tag}: row has no scheduled or late count")
            else:
                for k in ("scheduled", "late"):
                    if k in m and m[k] != p.row[k]:
                        add("lateness", set_, [p.tag], f"{p.tag}: row {k} {p.row[k]}, manifest {m[k]}")
            # netem
            counters = m.get("netem_counters") or {}
            if p.netem and p.netem != "none":
                if m["transport"] == "aeron-dpdk":
                    add("netem", set_, [p.tag], f"{p.tag}: DPDK in a loss row")
                try:
                    pct = float(p.netem)
                except ValueError:
                    add("netem", set_, [p.tag], f"{p.tag}: netem {p.netem!r} is not a percentage")
                    continue
                for host in hosts:
                    nics = counters.get(host)
                    if not nics:
                        add("netem", set_, [p.tag], f"{p.tag}: no tc -s counters on {host}")
                        continue
                    offered = dropped = 0
                    for nic, c in sorted(nics.items()):
                        if c.get("qdisc") != "netem" or float(c.get("loss_pct", -1)) != pct:
                            add("netem", set_, [p.tag], f"{p.tag}: {host} {nic} qdisc {c.get('qdisc')} loss "
                                f"{c.get('loss_pct')}, not netem {p.netem} %")
                        offered += c.get("sent_pkts", 0) + c.get("dropped", 0)
                        dropped += c.get("dropped", 0)
                    if not within_binomial(offered, dropped, pct):
                        add("netem", set_, [p.tag], f"{p.tag}: {host} netem dropped {dropped:,} of {offered:,}, "
                            f"outside the 99.9 % interval of {pct} %")
            else:
                if p.netem == "none":  # the 3c control: shown to have no qdisc on any data NIC
                    for host in hosts:
                        if not counters.get(host):
                            add("netem", set_, [p.tag], f"{p.tag}: no tc -s counters on {host} to show no qdisc")
                for host, nics in sorted(counters.items()):
                    for nic, c in sorted((nics or {}).items()):
                        if c.get("qdisc") == "netem":
                            add("netem", set_, [p.tag], f"{p.tag}: {host} {nic} has netem on a row without loss")

    # across rounds: seeds and orders randomised, wire bytes per arm, the VM pair
    reps = defaultdict(list)
    for rid, ps in rounds.items():
        p = ps[0]
        reps[(p.set, p.streams, p.netem or "", p.rate)].append(ps)
    for (set_, streams, netem, rate), rs in sorted(reps.items()):
        tags = [p.tag for ps in rs for p in ps]
        seeds = [ps[0].manifest.get("seed") for ps in rs]
        orders = [tuple(ps[0].manifest["arm_order"]) for ps in rs]
        what = f"set {set_}, {streams} streams, netem {netem or '-'}, {rate:,}/s"
        if len(rs) >= 2 and None not in seeds and len(set(seeds)) < len(seeds):
            add("order", set_, tags, f"{what}: reps share a seed")
        if len(rs) >= 3 and len(set(orders)) == 1:
            add("order", set_, tags, f"{what}: every rep ran its arms in the same order")
    by_set = defaultdict(list)
    for p in good:
        by_set[p.set].append(p)
    for set_, ps in sorted(by_set.items()):
        wire = defaultdict(set)
        for p in ps:
            wire[p.arm].add(p.manifest["wire_bytes"])
        for arm, w in sorted(wire.items()):
            if len(w) > 1:
                add("payload", set_, [p.tag for p in ps if p.arm == arm],
                    f"{arm}'s wire bytes differ across rounds: {', '.join(map(str, sorted(w)))}")
        # each host on one VM: a one-host arm's points (host a alone) beside
        # a sink arm's (a and b) in one set are one pair
        pairs, by_host = defaultdict(list), defaultdict(set)
        for p in ps:
            vms = p.manifest["vms"] or {}
            pairs[tuple(sorted(vms.items()))].append(p.tag)
            for h, v in vms.items():
                by_host[h].add(v)
        if any(len(v) > 1 for v in by_host.values()):
            add("vm-pair", set_, [], "points on more than one VM pair: "
                + "; ".join(f"{dict(k)} ({len(v)})" for k, v in sorted(pairs.items())))
        for p in ps:
            missing = [h for h in hosts_of_point(p) if not (p.manifest["vms"] or {}).get(h)]
            if missing:
                add("vm-pair", set_, [p.tag], f"{p.tag}: no VM id for {', '.join(missing)}")

    for p, _, cause in transport_maxima(run_dir):
        if cause.startswith("unexplained"):
            add("max", p.set, [p.tag], f"{p.tag}: max {cause}")
    return out


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: fairness.py <run dir>")
    absent, notes = [], []
    found = check(sys.argv[1], absences=absent, notes=notes)
    for v in found:
        print(v)
    for a in absent + notes:
        print(a)
    print(f"{len(found)} violation{'s' if len(found) != 1 else ''}")
    sys.exit(1 if found else 0)


if __name__ == "__main__":
    main()
