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
                   housekeeping or IRQ CPU; samples from every host
  irq              no data-NIC IRQ on a hot CPU; IRQs read on every host
  hostcheck        every host's report clean (no warn, no unproven blocking check)
  settings         every knob as transport-settings.env gives it, none it does not
  shared-knob      a per-arm knob the round's arms run at different values swept
                   for every arm of the round that has it (a declared arm's own
                   knob excepted); never also shared
  declared         an arm declared another's twin but for one knob (aeron-ipc-huge:
                   aeron-ipc on 2 MiB pages) runs in every round with it, and
                   nothing else differs: its other per-arm knobs, transport, wire
                   bytes, binary, arguments and media driver mode are the twin's
  order            a seed and the round's arms in its recorded order; seeds
                   distinct and orders not all the same across reps
  aa               an A/A arm in every round, repeating an arm of the round
  loss-accounting  loss columns filled (retrans where the transport retransmits:
                   TCP, Aeron); counters from every host (TCP: tcp_info
                   and nstat; Aeron: aeron; UDP: nstat; a transport joined by +,
                   such as udp+tcp, every part's)
  loss             a TCP point's real loss (retrans_not_dsack, lost_end) only on a
                   loss row: on one without (no netem, none, 0 %) it is a bug to
                   trace; DSACK-reported (spurious) retransmits are reported, not loss
  lateness         scheduled and late counted, as the manifest says
  max              every max labelled (spikes.py); unexplained blocks
  netem            a loss row's netem on every data NIC of every host at its
                   loss, its drops inside a 99.9 % binomial interval; none on
                   other rows; no DPDK in a loss row
  vm-pair          one VM pair per set
"""
import math
import os
import re
import sys
from collections import defaultdict, namedtuple

from spikes import transport_maxima
from transport_points import hosts_of, read_points, tcp_loss

Violation = namedtuple("Violation", "rule set tags detail")
Violation.__str__ = lambda v: f"{v.rule}: {v.set or 'run'}: {v.detail}"

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
                "aeron-ipc": ("aeron",), "aeron-spy": ("aeron",), "mpsc": ()}



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


def tcp_loss_problem(p):
    """What is wrong with a TCP point's loss: real loss (retransmits not
    reported spurious, segments lost) on a row without loss (no netem, the
    no-qdisc control, or netem at 0 %) is a bug to trace; spurious
    retransmits are reported, never loss. None when nothing is."""
    loss = tcp_loss(p)
    if loss is None or (p.netem not in (None, "none") and float(p.netem) != 0):
        return None
    real, spurious = loss
    if real:
        return (f"{p.tag}: {real} segments lost (retransmits not DSACK-reported, or lost at the end) "
                f"on a row without loss ({spurious} spurious): trace it")
    return None


def read_env(path):
    """transport-settings.env as (shared {knob: value}, per arm {arm: {knob:
    value}}, sweep {knob: {arm}}, declared {arm: (twin, knob)}), or None
    when missing."""
    if not os.path.exists(path):
        return None
    shared, per_arm, sweep, declared = {}, defaultdict(dict), {}, {}
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
        else:
            per_arm[prefix][knob] = v
    return shared, per_arm, sweep, declared


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
        if m and m.group(2) == "warn":
            out.append(line)
        elif m and m.group(2) == "unknown" and m.group(1) not in SUBSYSTEM_CHECKS:
            out.append(line)
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


def check(run_dir):
    out = []
    add = lambda rule, set_, tags, detail: out.append(Violation(rule, set_, tuple(sorted(tags)), detail))
    points = read_points(run_dir)
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
        for order in orders:
            if sorted(order) != sorted(arms):
                add("order", set_, tags, f"round {rid}: arm order {', '.join(order)} is not the round's arms "
                    f"{', '.join(sorted(arms))}")

        if env is not None:
            shared, per_arm, sweep, declared = env
            reported = defaultdict(set)  # knob: arms reporting it
            for p in ps:
                for host, knobs in p.manifest["settings"].items():
                    for k in knobs:
                        reported[k].add(arm_key(p))
            for p in ps:
                arm = arm_key(p)
                want = dict(shared, **per_arm.get(arm, {}))
                for host in hosts_of(p.set):
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
                """k is arm's declared difference from a twin in the round:
                its value is what the pair compares (the declared rule
                checks the rest)."""
                return declared.get(arm, (None, None))[1] == k and declared[arm][0] in round_arms

            for k in sorted({k for arm in round_arms for k in per_arm.get(arm, {})}):
                have = (reported[k] | {arm for arm in round_arms if k in per_arm.get(arm, {})})
                have = {arm for arm in have if not own(arm, k)}
                values = {per_arm[arm][k] for arm in have if k in per_arm.get(arm, {})}
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
                                     lambda p: (p.manifest.get("runner") or {}).get("driver_mode"))):
                    seen = sorted({repr(value(p)) for p in pair})
                    if len(seen) > 1:
                        add("declared", set_, tags, f"round {rid}: {arm} and its twin {twin} differ in {what}: "
                            + ", ".join(seen))

        for p in ps:
            m, hosts = p.manifest, hosts_of(p.set)
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
        pairs = defaultdict(list)
        for p in ps:
            pairs[tuple(sorted((p.manifest["vms"] or {}).items()))].append(p.tag)
        if len(pairs) > 1:
            add("vm-pair", set_, [], "points on more than one VM pair: "
                + "; ".join(f"{dict(k)} ({len(v)})" for k, v in sorted(pairs.items())))
        for p in ps:
            missing = [h for h in hosts_of(p.set) if not (p.manifest["vms"] or {}).get(h)]
            if missing:
                add("vm-pair", set_, [p.tag], f"{p.tag}: no VM id for {', '.join(missing)}")

    for p, _, cause in transport_maxima(run_dir):
        if cause.startswith("unexplained"):
            add("max", p.set, [p.tag], f"{p.tag}: max {cause}")
    return out


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: fairness.py <run dir>")
    found = check(sys.argv[1])
    for v in found:
        print(v)
    print(f"{len(found)} violation{'s' if len(found) != 1 else ''}")
    sys.exit(1 if found else 0)


if __name__ == "__main__":
    main()
