#!/usr/bin/env python3
"""The transport suite's plan, from transport-matrix.tsv and any LAB_ARMS_FILE.

transport.sh calls it for the plan and estimate.py imports it for the
timings. Commands:

  arms <files>...                    TRANSPORT_ARMS for modes.sh (" name=setup/hosts ...")
  arm <name> <files>...              "<binary> <setup> <hosts> <arguments...>"
  modes <pair> <files>...            every mode a point runs, one a line (for mode_check)
  rounds <pair> <seed> <reps> <files>...
                                     the session's rounds in run order, one a line, tab-separated:
                                     group set host rep netem round-seed loads streams arms
                                     (arms: the round's order; aa as <first arm>@AA=1)
  count <pair> <reps> <files>...     the points the pair runs
  smt <files>...                     the session's SMT, off or on (a sets file's `smt on`
                                     row; off without one)
  binom <offered> <dropped> <loss %> "<lo> <hi> ok|FAIL": the dropped count's
                                     interval at that loss (two-sided, 1 in 10,000)

A sets file may hold one `smt <off|on>` row: the session's SMT (vm.sh
transport-isolate). With `on`, SMT stays on, each isolated core is listed
whole in the CPU map, and every arm of the session is a two-host socket arm
(transport.sh tr_check): a loop on one hyperthread, its sibling idle or
running that loop's uring-sqpoll SQPOLL thread.

A round is one pass over a set's arms at one rep and netem level, in an order
drawn from that round's own seed: sha256 of "<session seed>:<set>:<rep>:<netem>",
so a session's seed reproduces every order. The netem levels of a rep are
ordered by the seed of "<session seed>:<set>:<rep>".
"""
import hashlib
import math
import random
import sys

SETUPS = ("none", "kernel", "dpdk", "dpdk-raw", "sockets", "fstack", "onload")
# setups whose NICs DPDK holds: netem (a kernel qdisc) cannot reach them
DPDK_SETUPS = ("dpdk", "dpdk-raw", "fstack")
# one: both ends in one process on the set's host; two: ping on a, pong on
# b; sink: one's run on host a and a sink on b reading every frame it sends
HOSTS = ("one", "two", "sink")


class PlanError(Exception):
    pass


def load(paths):
    """(arms {name: (binary, setup, hosts, [args])}, sets [dict]) from the files, in order;
    a path ending :arms gives its arm rows alone (LAB_SETS_FILE's sets in place of the matrix's)."""
    arms, sets, seen = {}, [], set()
    for path in paths:
        if not path:
            continue
        arms_only = path.endswith(":arms")
        path = path.removesuffix(":arms")
        try:
            with open(path) as f:
                text = f.read()
        except OSError as e:
            raise PlanError(f"{path}: {e.strerror}") from None
        for n, line in enumerate(text.splitlines(), 1):
            words = line.split("#", 1)[0].split()
            if not words:
                continue
            where = f"{path}:{n}"
            if words[0] == "arm":
                if len(words) < 5:
                    raise PlanError(f"{where}: arm <name> <binary> <setup> <hosts> [<arguments>...]")
                name, binary, setup, hosts = words[1:5]
                if name in arms or name == "aa":
                    raise PlanError(f"{where}: arm {name} given twice (or named aa, the A/A arm's)")
                if setup not in SETUPS or hosts not in HOSTS:
                    raise PlanError(f"{where}: arm {name}: setup is one of {', '.join(SETUPS)}, hosts one, two or sink")
                if not all(c.isalnum() or c in "-_" for c in name + binary):
                    raise PlanError(f"{where}: arm {name}: names and binaries are letters, digits, - and _")
                args = words[5:]
                if "--nics" in args and args[args.index("--nics") + 1:args.index("--nics") + 2] != ["alt"]:
                    raise PlanError(f"{where}: arm {name}: --nics takes alt (each host's other data NIC)")
                arms[name] = (binary, setup, hosts, args)
            elif words[0] in ("set", "smt") and arms_only:
                continue
            elif words[0] == "smt":
                if len(words) != 2 or words[1] not in ("off", "on"):
                    raise PlanError(f"{where}: smt off or smt on")
            elif words[0] == "set":
                if len(words) != 9:
                    raise PlanError(f"{where}: set <id> <pair> <par> <host> <loads> <streams> <netem> <arms>")
                sid, pair, par, host, loads, streams, netem, names = words[1:]
                if sid in seen:
                    raise PlanError(f"{where}: set {sid} given twice")
                seen.add(sid)
                if pair not in ("a", "b", "tune") or host not in ("a", "b", "ab"):
                    raise PlanError(f"{where}: set {sid}: pair a or b (tune: the tuning session's), host a, b or ab")
                try:
                    loads = [int(v) for v in loads.split(",")]
                    streams = [int(v) for v in streams.split(",")]
                    levels = ["-"] if netem == "-" else [v for v in netem.split(",")]
                    [float(v) for v in levels if v not in ("-", "none")]
                except ValueError:
                    raise PlanError(f"{where}: set {sid}: loads and streams are whole numbers, netem - or loss %") from None
                names = names.split(",")
                sets.append(dict(id=sid, pair=pair, par=par, host=host, loads=loads, streams=streams,
                                 netem=levels, arms=names, where=where))
            else:
                raise PlanError(f"{where}: a row is arm, set or smt, not {words[0]}")
    for s in sets:
        if s["arms"][0] == "aa":
            raise PlanError(f"{s['where']}: set {s['id']}: aa repeats the set's first arm, so it cannot be first")
        for a in s["arms"]:
            if a != "aa" and a not in arms:
                raise PlanError(f"{s['where']}: set {s['id']}: no arm {a}")
        held = sorted(a for a in s["arms"] if a != "aa" and arms[a][1] in DPDK_SETUPS)
        if held and s["netem"] != ["-"]:
            raise PlanError(f"{s['where']}: set {s['id']}: {', '.join(held)} run on DPDK, which netem "
                            "cannot reach: such a set has netem -")
        # Onload's AF_XDP socket sends by __dev_direct_xmit: past the qdisc too
        xdp = sorted(a for a in s["arms"] if a != "aa" and arms[a][1] == "onload")
        if xdp and s["netem"] != ["-"]:
            raise PlanError(f"{s['where']}: set {s['id']}: {', '.join(xdp)} run on Onload, whose AF_XDP sends "
                            "netem cannot reach (they skip the qdisc): such a set has netem -")
        kinds = {arms[a][2] for a in s["arms"] if a != "aa"}
        if s["host"] == "ab" and kinds != {"two"} or s["host"] != "ab" and not kinds <= {"one", "sink"}:
            raise PlanError(f"{s['where']}: set {s['id']}: host ab takes two-host arms, a or b one-host arms")
        # a sink arm measures on host a and reads on b: its set holds both
        # hosts (never run beside another), with no netem (set on a alone)
        s["sink"] = "sink" in kinds
        if s["sink"] and (s["host"] != "a" or s["netem"] != ["-"]):
            raise PlanError(f"{s['where']}: set {s['id']}: a sink arm (its sink on b) runs in a set on host a, "
                            "netem -")
    return arms, sets


def smt(paths):
    """The session's SMT, "off" or "on": the files' `smt` rows (a path
    ending :arms gives none), which agree; "off" without one."""
    load(paths)
    seen = {}
    for path in paths:
        if not path or path.endswith(":arms"):
            continue
        with open(path) as f:
            for n, line in enumerate(f, 1):
                words = line.split("#", 1)[0].split()
                if words and words[0] == "smt":
                    seen[f"{path}:{n}"] = words[1]
    if len(set(seen.values())) > 1:
        raise PlanError("smt rows disagree: " + ", ".join(f"{w} {v}" for w, v in seen.items()))
    return next(iter(seen.values()), "off")


def seed_of(text):
    return int(hashlib.sha256(text.encode()).hexdigest()[:12], 16)


def mode(arm, streams, netem, aa):
    m = f"{arm}@STREAMS={streams}"
    if netem != "-":
        m += f"@NETEM={netem}"
    return m + ("@AA=1" if aa else "")


def groups(sets, pair):
    """The pair's sets as run groups, in file order: a group is one set, or
    every set of the pair sharing a par tag (run at once)."""
    out, tags = [], {}
    for s in sets:
        if s["pair"] != pair:
            continue
        if s["par"] == "-":
            out.append([s])
        elif s["par"] in tags:
            out[tags[s["par"]]].append(s)
        else:
            tags[s["par"]] = len(out)
            out.append([s])
    for g in out:
        hosts = ["ab" if s.get("sink") else s["host"] for s in g]
        if len(g) > 1 and ("ab" in hosts or len(set(hosts)) != len(hosts)):
            raise PlanError(f"sets {', '.join(s['id'] for s in g)} run at once: one-host sets on different hosts only")
    return out


def rounds(sets, pair, seed, reps):
    """[(group, set, rep, netem, round seed, [arm modes' (arm, aa)])] in run order."""
    out = []
    for gi, g in enumerate(groups(sets, pair)):
        for s in g:
            for rep in range(1, reps + 1):
                levels = list(s["netem"])
                random.Random(seed_of(f"{seed}:{s['id']}:{rep}")).shuffle(levels)
                for netem in levels:
                    rs = seed_of(f"{seed}:{s['id']}:{rep}:{netem}")
                    order = [(s["arms"][0], True) if a == "aa" else (a, False) for a in s["arms"]]
                    random.Random(rs).shuffle(order)
                    out.append((gi, s, rep, netem, rs, order))
    return out


def count(sets, pair, reps):
    return sum(len(s["arms"]) * len(s["loads"]) * len(s["streams"]) * len(s["netem"]) * reps
               for s in sets if s["pair"] == pair)


def set_seconds(s, reps, warmup_s, point_s, switch_s):
    """A set's measuring time: each arm x stream count a warm-up, a switch
    and its points (each its window and a switch); a one-host set's points
    (a sink arm's too) twice (transport.sh runs one way, then the round trip)."""
    runs = 1 if s["host"] == "ab" else 2
    return (runs * len(s["arms"]) * len(s["streams"]) * len(s["netem"]) * reps
            * (warmup_s + switch_s + len(s["loads"]) * (point_s + switch_s)))


def seconds(sets, pair, reps, warmup_s, point_s, switch_s):
    """The pair's measuring time; sets run at once take the longest of them."""
    return sum(max(set_seconds(s, reps, warmup_s, point_s, switch_s) for s in g) for g in groups(sets, pair))


def binom(n, d, pct):
    """(lo, hi, ok): dropped d of n offered at loss pct %, against the binomial
    interval holding 1 - 1e-4 of it (two-sided): exact sums below a variance of
    25, the normal approximation above."""
    p = pct / 100
    if p <= 0:
        return 0, 0, d == 0
    if p >= 1:
        return n, n, d == n
    var = n * p * (1 - p)
    if var >= 25:
        z = 3.8906  # two-sided 1e-4
        lo, hi = max(0, math.floor(n * p - z * math.sqrt(var))), min(n, math.ceil(n * p + z * math.sqrt(var)))
        return lo, hi, lo <= d <= hi
    tail = 0.5e-4
    pmf, cdf, k, lo, hi = math.exp(n * math.log1p(-p)), 0.0, 0, None, n
    while k <= n:
        cdf += pmf
        if lo is None and cdf > tail:
            lo = k
        if cdf >= 1 - tail:
            hi = k
            break
        pmf *= (n - k) / (k + 1) * p / (1 - p)
        k += 1
    lo = 0 if lo is None else lo
    return lo, hi, lo <= d <= hi


def main(argv):
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    cmd, args = argv[0], argv[1:]
    try:
        if cmd == "binom":
            lo, hi, ok = binom(int(args[0]), int(args[1]), float(args[2]))
            print(lo, hi, "ok" if ok else "FAIL")
            return 0
        if cmd == "arms":
            arms, _ = load(args)
            print(" " + " ".join(f"{a}={v[1]}/{v[2]}" for a, v in arms.items()) + " ")
        elif cmd == "arm":
            arms, _ = load(args[1:])
            if args[0] not in arms:
                raise PlanError(f"no arm {args[0]}")
            b, setup, hosts, rest = arms[args[0]]
            print(" ".join([b, setup, hosts] + rest))
        elif cmd == "modes":
            arms, sets = load(args[1:])
            seen = []
            for s in sets:
                if s["pair"] != args[0]:
                    continue
                for a in s["arms"]:
                    for n in s["streams"]:
                        for lv in s["netem"]:
                            m = mode(s["arms"][0], n, lv, True) if a == "aa" else mode(a, n, lv, False)
                            if m not in seen:
                                seen.append(m)
            groups(sets, args[0])
            print("\n".join(seen))
        elif cmd == "rounds":
            pair, seed, reps = args[0], args[1], int(args[2])
            _, sets = load(args[3:])
            for gi, s, rep, netem, rs, order in rounds(sets, pair, seed, reps):
                print("\t".join([str(gi), s["id"], s["host"], str(rep), netem, str(rs),
                                 ",".join(map(str, s["loads"])), ",".join(map(str, s["streams"])),
                                 ",".join(a + ("@AA=1" if aa else "") for a, aa in order)]))
        elif cmd == "smt":
            print(smt(args))
        elif cmd == "count":
            _, sets = load(args[2:])
            print(count(sets, args[0], int(args[1])))
        else:
            print(f"transport_plan.py: no command {cmd}", file=sys.stderr)
            return 2
    except PlanError as e:
        print(f"transport plan: {e}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
