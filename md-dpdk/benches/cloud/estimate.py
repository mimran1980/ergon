#!/usr/bin/env python3
"""What a run of aws/lab.sh or azure/lab.sh would cost, before it creates anything
(LAB_ESTIMATE=1: lab.sh prints this and exits, with no cloud call).

Azure (--cloud azure): the same steps and timings (measured on AWS), the
VMs' pay-as-you-go Linux prices in northcentralus (AZURE_PRICES, from the
Azure Retail Prices API on AZURE_AS_OF), no data transfer charge (both VMs
in one VNet and proximity placement group: placement ppg), and the deadline
the backstop's (it deletes the resource group LAB_MAX_HOURS after creation).

Wall time, from the run of 2026-10-15 on two c7i.4xlarge (aws-20261015-185813:
its files' times), per step:
  setup        600 s  launch to the first state: bootstrap, reboot, build, tune,
                      the clock checks (9.7 minutes there)
  state        pinned 150 s (pre-flight 120 s, diagnostics); isolated and
                      isolated-poll 450 s (kernel command line, reboot, tune,
                      clockcheck 120 s, Criterion, pre-flight 120 s: 7.3 minutes)
  mode and rep 12 s (drivers started and stopped, counters, loss reads) plus the
                      round trip's 85 s (RTT_SECONDS 20, its warm-up, the pong's start);
                      a raw floor mode (kernel-raw, dpdk-raw, dpdk-pure: no Aeron) only that, its
                      round trip alone; a paced mode (@RTT_GAP_US) its round trip's
                      rtt_plan seconds (20 to 60) and a quarter more of warm-up in
                      place of the 20 and 5
  32 B flat out          tput_seconds + 9 s (14 s at 5)
  sustained maximum      tput_seconds + 11 s a size (16 s at 5)
  several flows at once  tput_seconds + 11 s a size
  load point             load_seconds + 7 s (17 s at 10), the burst point alike
  warm-up                warmup_seconds + 7 s (LAB_WARMUP_SECONDS, a load point's
                         start and stop), once a mode and rep where a load sweep
                         runs (not a +idle mode), before its first ladder
  mix profile            mix_seconds + 7 s
  teardown     180 s
Both instances run the whole time: instance-hours are twice the wall time.

Bytes between the hosts (cross-host modes only; a localhost run crosses no
network), as IP packets: each message plus Aeron's 32-byte data header and 28
bytes of IPv4 and UDP; Aeron's status messages, NAKs and heartbeats are left
out. The round trip: its warm-up and messages of 32 bytes, each way (a paced
mode's as rtt_plan has them; a same-host hairpin, @HAIRPIN=1, none: it stays on
host a). A load
point: (load_seconds + 1) s (the first second is warm-up) at its rate, echoed
back the same size; a warm-up likewise, (warmup_seconds + 1) s at
LAB_WARMUP_RATE, at the sweep's first size. A flat-out phase (none with LAB_NO_FLATOUT=1) is counted
at 8.0 Gbit/s of IP packets, above one AWS flow's measured ceiling (7.75–7.77
Gbit/s of Aeron frames, about 7.9 as IP packets: md-dpdk/BENCHMARKS.md), for the
publisher's tput_seconds + 6 s: an upper bound, as is a fraction of a flat-out
maximum (LAB_LOAD_FRACTIONS) taken of that ceiling. GB are 10^9 bytes.

Dollars: on-demand prices in us-east-1 ($/h) for the types the lab uses (a
type not in the table is reported, its dollars unknown). With --market spot
(LAB_SPOT=1's one-time spot instances) also the lab's spot prices
(SPOT_PRICES: us-east-1, as of SPOT_AS_OF, from SPOT_SOURCE; no cloud call
looks them up): AWS bills each hour at the spot price of the run's zone then,
which moves, so those dollars are an estimate; the on-demand figure is the
ceiling, as a one-time spot instance's default maximum price is the
on-demand price (lab.sh sets none). A type without a spot price is priced at
on-demand alone, as an upper bound. Data transfer:
none within one zone (cluster, precision-time: private addresses); across
zones $0.01/GB out of one zone and $0.01/GB into the other, so $0.02 a GB
crossing either way; across regions $0.02/GB out of the sending region.

A plan longer than LAB_MAX_HOURS is cut short by the instances' own
deadline (each powers off, and so terminates, that long after launch): the
estimate says so, with the hours to give it (the estimate and a fifth more,
rounded up), and --refuse-over-deadline (lab.sh, before any cloud call)
prints only that and exits 3.
"""
import argparse
import math
import statistics
import sys

PRICES = {
    "c7i.2xlarge": 0.357, "c7i.4xlarge": 0.714, "c7i.metal-24xl": 4.284,
    "c8g.xlarge": 0.15952, "c8g.2xlarge": 0.31904, "c9g.xlarge": 0.17388,
    "c7g.4xlarge": 0.58, "c8g.4xlarge": 0.63808, "c8g.metal-24xl": 3.82848,
    "c7gn.4xlarge": 0.9984, "c8gn.4xlarge": 0.948, "c9g.4xlarge": 0.69552,
    "c8i.4xlarge": 0.74968,
    "c9g.2xlarge": 0.34776, "c8gn.xlarge": 0.237, "c6in.4xlarge": 0.9072,
    "c7a.2xlarge": 0.41056,
}
# us-east-1 Linux spot prices ($/h) for the types the final rounds use, as
# of SPOT_AS_OF (instances.vantage.sh, its data of 2026-10-15 16:35): spot
# moves by the hour and the zone, so refresh them before a round
# (c8g.2xlarge: its page of 2026-10-23 16:36)
SPOT_PRICES = {"c9g.2xlarge": 0.153, "c8g.xlarge": 0.077, "c8g.2xlarge": 0.147, "c7i.2xlarge": 0.200,
               "c7a.2xlarge": 0.1907}
SPOT_AS_OF, SPOT_SOURCE = "2026-10-15", "instances.vantage.sh"
# a type whose spot price was read elsewhere than SPOT_SOURCE: c7a.2xlarge
# (the AWS transport session) from aws ec2 describe-spot-price-history in
# us-east-1 on 2026-10-27, us-east-1a's (the zones then 0.1495 to 0.2267);
# its on-demand price from the AWS Price List API (effective 2026-10-01)
SPOT_READ = {"c7a.2xlarge": ("2026-10-27", "aws ec2 describe-spot-price-history, us-east-1a")}
# Azure pay-as-you-go Linux prices ($/h) in northcentralus, the lab's
# default region (prices.azure.com/api/retail/prices on AZURE_AS_OF)
AZURE_PRICES = {
    "Standard_D4s_v6": 0.202, "Standard_D8s_v6": 0.403, "Standard_D16s_v6": 0.806,
    "Standard_D4ps_v6": 0.14, "Standard_D8ps_v6": 0.281, "Standard_D16ps_v6": 0.562,
    "Standard_D8pls_v6": 0.248, "Standard_D16pls_v6": 0.496, "Standard_E8ps_v6": 0.369,
    "Standard_F8as_v6": 0.546, "Standard_F16as_v6": 1.093,
}
AZURE_AS_OF, AZURE_REGION = "2026-10-24", "northcentralus"
SETUP_S, TEARDOWN_S = 600, 180
STATE_S = {"pinned": 150, "isolated": 450, "isolated-poll": 450,
           "pinned-nodiag": 150, "isolated-nodiag": 450, "isolated-poll-nodiag": 450}
MODE_S, RTT_S = 12, 85
POINT_EXTRA_S, MIX_EXTRA_S, TPUT_EXTRA_S, MAX_EXTRA_S = 7, 7, 9, 11
WIRE_OVERHEAD = 32 + 28  # Aeron's data header, IPv4 and UDP
# One AWS flow's ceiling, rounded up: md-dpdk/BENCHMARKS.md measured 7.75-7.77 Gbit/s
# of Aeron frames (about 7.9 as the IP packets counted here) on c7i.4xlarge, so a
# flat-out phase's bytes at 8.0 are an upper bound.
FLOW_CAP_BYTES_S = 8.0e9 / 8
RTT_SIZE = 32
# the raw floor's transports (modes.sh mode_raw): the round trip alone
RAW = ("kernel-raw", "dpdk-raw", "dpdk-pure")
TRANSFER_USD_GB = {"cluster": 0.0, "precision-time": 0.0, "cross-az": 0.02, "cross-region": 0.02, "ppg": 0.0}


def rtt_plan(mode, messages, warmup):
    """(seconds, round trips, warm-up) of a mode's round trip, as modes.sh
    rtt_plan has them: 20 s and as given, or for a paced mode (@RTT_GAP_US) as
    long as 10,000 round trips take, 20 to 60 s, the round trips that fit
    (at most those given) and about 2 s of warm-up (at least one)."""
    gaps = [p.partition("=")[2] for p in mode.removesuffix("+idle").split("@")[1:] if p.startswith("RTT_GAP_US=")]
    if not gaps:
        return 20, messages, warmup
    gap = int(gaps[-1])
    seconds = min(60, max(20, -(-10000 * gap // 1_000_000)))
    return seconds, min(messages, seconds * 1_000_000 // gap), min(warmup, -(-2_000_000 // gap))


def points(size, fractions, ladder):
    """The load sweep's points at <size>: (rate msgs/s, is it a bound) each,
    the burst's last: the ladder's rates and its median, or fractions of the
    flat-out maximum (at most the flow's ceiling) and half of it."""
    rates = ladder.get(size)
    if rates:
        ordered = sorted(rates)
        return [(r, False) for r in rates] + [(ordered[(len(ordered) + 1) // 2 - 1], False)]
    most = FLOW_CAP_BYTES_S / (size + WIRE_OVERHEAD)
    return [(most * f / 100, True) for f in fractions] + [(most / 2, True)]


def estimate(type_a, type_b, placement, reps, modes, local_modes, states, load_sizes, load_fractions, load_ladder,
             load_seconds, mix, mix_seconds, tput_seconds, tput_flows, rtt_messages, rtt_warmup, no_flatout, max_hours,
             warmup_seconds=0, warmup_rate=0, prices=None):
    """The plan's wall time (s), instance-hours, dollars (on demand, and at
    the lab's spot prices where it has them) and bytes each way.
    mix: [[(rate, size), ...] a profile]; load_ladder: {size: [rates]};
    warmup_seconds at warmup_rate (LAB_WARMUP_*): a warm-up load before each
    mode's first ladder, in each rep."""
    flat = not no_flatout
    flows = tput_flows if flat and tput_flows >= 2 else 0
    per_mode, ab, ba, bound = 0, 0, 0, False
    for local in (False, True):
        for mode in (local_modes if local else modes):
            idle = mode.endswith("+idle")
            seconds, messages, warmup = rtt_plan(mode, rtt_messages, rtt_warmup)
            secs, to_b, to_a = MODE_S + RTT_S + (seconds - 20) + (-(-seconds // 4) - 5), 0, 0
            if not local and "@HAIRPIN=1@" not in mode.removesuffix("+idle") + "@":
                rtt = (messages + warmup) * (RTT_SIZE + WIRE_OVERHEAD)
                to_b, to_a = rtt, rtt
            if mode.split("@")[0].removesuffix("-mono") in RAW:
                per_mode += secs
                ab, ba = ab + to_b, ba + to_a
                continue
            if flat:
                secs += tput_seconds + TPUT_EXTRA_S
                if not local:
                    to_b += FLOW_CAP_BYTES_S * (tput_seconds + 6)
                    bound = True
            if not idle:
                warmed = False
                for size in load_sizes:
                    if flat:
                        secs += tput_seconds + MAX_EXTRA_S
                        if not local:
                            to_b += FLOW_CAP_BYTES_S * (tput_seconds + 6)
                    elif not load_ladder.get(size):
                        continue
                    if warmup_seconds > 0 and not warmed:
                        warmed = True
                        secs += warmup_seconds + POINT_EXTRA_S
                        if not local:
                            sent = (warmup_seconds + 1) * warmup_rate * (size + WIRE_OVERHEAD)
                            to_b, to_a = to_b + sent, to_a + sent
                    for rate, is_bound in points(size, load_fractions, load_ladder):
                        secs += load_seconds + POINT_EXTRA_S
                        if not local:
                            sent = (load_seconds + 1) * rate * (size + WIRE_OVERHEAD)
                            to_b, to_a, bound = to_b + sent, to_a + sent, bound or is_bound
                for profile in mix:
                    secs += mix_seconds + MIX_EXTRA_S
                    if not local:
                        sent = sum((mix_seconds + 1) * rate * (size + WIRE_OVERHEAD) for rate, size in profile)
                        to_b, to_a = to_b + sent, to_a + sent
                if flows and not local:
                    for _ in sorted({32} | set(load_sizes)):
                        secs += tput_seconds + MAX_EXTRA_S
                        to_b += flows * FLOW_CAP_BYTES_S * (tput_seconds + 6)
            per_mode += secs
            ab, ba = ab + to_b, ba + to_a
    wall = SETUP_S + TEARDOWN_S + sum(STATE_S.get(s, 450) + reps * per_mode for s in states)
    n = reps * len(states)
    prices = PRICES if prices is None else prices
    unknown = [t for t in (type_a, type_b) if t not in prices]
    hourly = None if unknown else prices[type_a] + prices[type_b]
    spot_hourly = (SPOT_PRICES[type_a] + SPOT_PRICES[type_b]
                   if type_a in SPOT_PRICES and type_b in SPOT_PRICES else None)
    bytes_ab, bytes_ba = ab * n, ba * n
    gb = (bytes_ab + bytes_ba) / 1e9
    return {
        "wall_s": wall, "instance_hours": 2 * wall / 3600,
        "instance_usd": None if hourly is None else wall / 3600 * hourly,
        "deadline_usd": None if hourly is None else max_hours * hourly,
        "spot_usd": None if spot_hourly is None else wall / 3600 * spot_hourly,
        "spot_deadline_usd": None if spot_hourly is None else max_hours * spot_hourly,
        "bytes_ab": bytes_ab, "bytes_ba": bytes_ba, "bound": bound,
        "transfer_usd": gb * TRANSFER_USD_GB[placement], "unknown": unknown,
        # the instances power off at their deadline: a plan past it is cut short
        "over_deadline": wall > max_hours * 3600, "hours_needed": math.ceil(wall * 1.2 / 3600),
    }


# The transport suite (azure/lab.sh LAB_SUITE=transport, transport.sh): a
# pair session's set-up is the latency lab's SETUP_S, then the bench
# binaries' build, the CPU map's reboot and transport-tune
# (TRANSPORT_PREP_S); each run of a binary (an arm at one stream count)
# costs TRANSPORT_RUN_S besides its warm-up and points (about 5 s a point: the
# pong started and given 2 s to bind, the snapshots, its output fetched; the driver
# or kernel path set up and stopped), each round TRANSPORT_ROUND_S (netem set, checked, cleared).
TRANSPORT_PREP_S, TRANSPORT_RUN_S, TRANSPORT_ROUND_S = 900, 15, 5
# a session with an F-Stack arm (setup fstack) also builds F-Stack and its
# DPDK on both hosts at once (vm.sh transport-build fstack): not yet timed on
# a run: DPDK's meson build and F-Stack's make on 8 cores, about 10 minutes
TRANSPORT_FSTACK_BUILD_S = 600
# each point's events on its hosts (vm.sh events-start and events-stop: the
# trace set up, read back and gzipped, the spinners and sampler started and
# stopped), measured in the local smoke
TRANSPORT_EVENTS_S = 6
# a sink arm's point (hosts sink: aeron_lat --mode spy-udp) starts a sink on
# host b for each of its two runs and stops it after (its ready line, then
# TERM and up to 4 s of reading on): not yet timed on a run, taken at 5 s each
TRANSPORT_SINK_S = 5


def transport_estimate(pair, paths, reps, warmup_s, point_s, switch_s):
    """A pair session's points, sets, measuring seconds, run and round
    overheads and wall seconds (transport_plan.py's plan)."""
    import os
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import transport_plan as tp
    arms, sets = tp.load(paths)
    measuring = tp.seconds(sets, pair, reps, warmup_s, point_s, switch_s)
    runs = rounds = 0
    for g in tp.groups(sets, pair):
        # sets run at once overlap: the longest one's runs and rounds count
        longest = max(g, key=lambda s: tp.set_seconds(s, reps, warmup_s, point_s, switch_s))
        runs += ((1 if longest["host"] == "ab" else 2) * len(longest["arms"]) * len(longest["streams"])
                 * len(longest["netem"]) * reps)
        rounds += len(longest["netem"]) * reps
    overhead = runs * TRANSPORT_RUN_S + rounds * TRANSPORT_ROUND_S + tp.count(sets, pair, reps) * TRANSPORT_EVENTS_S
    # sink arms' points (the A/A of one too): two sinks each
    sink_points = sum(len(s["loads"]) * len(s["streams"]) * len(s["netem"]) * reps
                      for s in sets if s["pair"] == pair
                      for a in s["arms"] if arms[s["arms"][0] if a == "aa" else a][2] == "sink")
    overhead += sink_points * 2 * TRANSPORT_SINK_S
    if any(arms[a][1] == "fstack" for s in sets if s["pair"] == pair for a in s["arms"] if a != "aa"):
        overhead += TRANSPORT_FSTACK_BUILD_S
    wall =SETUP_S + TRANSPORT_PREP_S + measuring + overhead + TEARDOWN_S
    return {"points": tp.count(sets, pair, reps), "sets": [s["id"] for s in sets if s["pair"] == pair],
            "measuring_s": measuring, "overhead_s": overhead, "wall_s": wall,
            "over_deadline": False, "hours_needed": math.ceil(wall * 1.2 / 3600)}


def tune_estimate(pair, a):
    """A tuning session's (LAB_TUNE=1, pair tune-a or tune-b) sweep and
    confirmation (transport_tune.py's plan): the sweep's runs exactly, the
    confirmation's at most (every knob confirmed) and likely (a third of
    them: the share a sweep beats its current value is not known before it
    runs); wall_s the at-most one, which the deadline must cover."""
    import argparse as ap
    import os
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import transport_tune as tt
    ns = ap.Namespace(settings=a.settings, arms=a.tune_arms, knobs=a.tune_knobs, absent="", pair=pair[len("tune-"):],
                      files=a.matrix)
    tr = tt.read_settings(a.settings)
    reps = int(tr.get("TR_TUNE_CONFIRM_REPS", "3"))
    _, sets, _, _ = tt.plan(ns)
    run_s = a.transport_warmup_s + 2 * a.transport_switch_s + a.transport_point_s + TRANSPORT_RUN_S + TRANSPORT_EVENTS_S

    def runs(s, n_arms, r):
        return (1 if s["host"] == "ab" else 2) * n_arms * len(s["streams"]) * len(s["netem"]) * r

    def timed(arms_of, r):
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

    sweep = timed(lambda s: len(s["arms"]), 1)
    confirm = timed(lambda s: min(len(s["arms"]) - 1, 3) + 1, reps)
    rounds = sum(len(s["netem"]) for s in sets) * (1 + reps)
    fixed = SETUP_S + TRANSPORT_PREP_S + TEARDOWN_S + rounds * TRANSPORT_ROUND_S
    sweep_s, confirm_s = sweep * run_s, confirm * run_s
    points = sum(len(s["arms"]) * len(s["streams"]) * len(s["netem"]) for s in sets)
    wall = fixed + sweep_s + confirm_s
    return {"points": points, "sets": [f"{len(sets)} arm-knob sweeps"], "measuring_s": sweep_s + confirm_s,
            "overhead_s": fixed, "wall_s": wall, "likely_s": fixed + sweep_s + confirm_s / 3,
            "sweep_s": sweep_s, "confirm_s": confirm_s, "over_deadline": False,
            "hours_needed": math.ceil(wall * 1.2 / 3600)}


def transport_report(a):
    """azure/lab.sh and aws/lab.sh LAB_ESTIMATE=1 (and their deadline
    checks) for the transport suite: the pair given, or both pairs (both
    tuning sessions with --tune); AWS's own report transport_aws_report."""
    pairs = [a.pair] if a.pair else (["tune-a", "tune-b"] if a.tune else ["a", "b"])
    try:
        est = {p: (tune_estimate(p, a) if p.startswith("tune-") else
                   transport_estimate(p, a.matrix, a.reps, a.transport_warmup_s, a.transport_point_s,
                                      a.transport_switch_s)) for p in pairs}
    except Exception as e:  # the plan's own errors, said once
        print(f"estimate.py: {e}", file=sys.stderr)
        return 2
    for e in est.values():
        e["over_deadline"] = e["wall_s"] > a.max_hours * 3600
    if a.print_hours:
        print(f"{est[pairs[0]]['wall_s'] / 3600:.2f}")
        return 0
    cut = (f"each instance powers off (and so terminates) {a.max_hours} h after launch" if a.cloud == "aws" else
           f"the backstop deletes the resource group {a.max_hours} h after creation")
    if a.refuse_over_deadline:
        for p, e in est.items():
            if e["over_deadline"]:
                print(f"the plan takes pair {p} about {e['wall_s'] / 3600:.2f} h, longer than LAB_MAX_HOURS="
                      f"{a.max_hours}: {cut}, "
                      f"cutting the session short: give LAB_MAX_HOURS={e['hours_needed']} or trim the plan", file=sys.stderr)
                return 3
        return 0
    if a.cloud == "aws":
        return transport_aws_report(a, est)
    hourly = None
    if a.type_a in AZURE_PRICES and a.type_b in AZURE_PRICES:
        hourly = AZURE_PRICES[a.type_a] + AZURE_PRICES[a.type_b]
    print(f"LAB_ESTIMATE: nothing created, no cloud call made. Transport suite, a {a.type_a}, b {a.type_b}, "
          f"{a.priority}, {a.reps} reps; {a.transport_warmup_s} s warm-up a run, {a.transport_point_s} s a point, "
          f"{a.transport_switch_s} s a switch")
    total_usd = 0.0
    for p, e in est.items():
        line = (f"pair {p}: {e['points']} points (sets {', '.join(e['sets'])}): measuring {e['measuring_s'] / 3600:.2f} h, "
                f"set-up, runs and teardown {(e['wall_s'] - e['measuring_s']) / 3600:.2f} h: "
                f"wall about {e['wall_s'] / 3600:.2f} h, {2 * e['wall_s'] / 3600:.2f} VM-hours")
        if hourly is not None:
            usd = e["wall_s"] / 3600 * hourly
            total_usd += usd
            line += f", about ${usd:.2f} (at most ${a.max_hours * hourly:.2f} to LAB_MAX_HOURS={a.max_hours})"
        print(line)
        if "likely_s" in e:
            likely = f"{e['likely_s'] / 3600:.2f} h" + (f", about ${e['likely_s'] / 3600 * hourly:.2f}" if hourly else "")
            print(f"  tuning {p}: the sweep {e['sweep_s'] / 3600:.2f} h, the confirmation at most "
                  f"{e['confirm_s'] / 3600:.2f} h (every knob confirmed; the wall above); likely about {likely} "
                  "(a third of the knobs confirmed)")
        if e["over_deadline"]:
            print(f"WARNING: pair {p} is longer than LAB_MAX_HOURS={a.max_hours}: give LAB_MAX_HOURS={e['hours_needed']}")
    if hourly is None:
        unknown = [t for t in (a.type_a, a.type_b) if t not in AZURE_PRICES]
        print(f"VMs: no price for {', '.join(unknown)} in estimate.py's AZURE_PRICES: their dollars unknown")
        return 0
    spot = " (pay-as-you-go: Spot bills less, by the hour's Spot price, not looked up)" if a.priority == "Spot" else ""
    print(f"prices: pay-as-you-go Linux, {AZURE_REGION} on {AZURE_AS_OF}: {a.type_a} ${AZURE_PRICES[a.type_a]:.3f}/h, "
          f"{a.type_b} ${AZURE_PRICES[a.type_b]:.3f}/h{spot}; disks and public addresses extra, cents an hour; "
          "no data transfer charge (one VNet, one proximity placement group)")
    if len(est) > 1:
        print(f"both pairs: about ${total_usd:.2f}, wall about {max(e['wall_s'] for e in est.values()) / 3600:.2f} h "
              "run side by side")
    return 0


def transport_aws_report(a, est):
    """aws/lab.sh LAB_SUITE=transport LAB_ESTIMATE=1: each pair session's
    points, wall time, instance-hours and dollars (on-demand, and spot with
    --market spot); no data transfer charge (one zone, cluster placement)."""
    print(f"LAB_ESTIMATE: nothing created, no cloud call made. Transport suite on AWS, one zone (cluster placement), "
          f"a {a.type_a}, b {a.type_b}, {a.market}, {a.reps} reps; {a.transport_warmup_s} s warm-up a run, "
          f"{a.transport_point_s} s a point, {a.transport_switch_s} s a switch")
    od = PRICES[a.type_a] + PRICES[a.type_b] if a.type_a in PRICES and a.type_b in PRICES else None
    spot = (SPOT_PRICES[a.type_a] + SPOT_PRICES[a.type_b]
            if od is not None and a.market == "spot" and a.type_a in SPOT_PRICES and a.type_b in SPOT_PRICES else None)
    for p, e in est.items():
        h = e["wall_s"] / 3600
        line = (f"pair {p}: {e['points']} points (sets {', '.join(e['sets'])}): measuring {e['measuring_s'] / 3600:.2f} h, "
                f"set-up, runs and teardown {(e['wall_s'] - e['measuring_s']) / 3600:.2f} h: "
                f"wall about {h:.2f} h{' (over 2 h: lab.sh detaches it)' if e['wall_s'] > 7200 else ''}, "
                f"{2 * h:.2f} instance-hours")
        if spot is not None:
            line += (f", about ${h * spot:.2f} at spot prices, at most ${h * od:.2f} on demand (at most "
                     f"${a.max_hours * od:.2f} to LAB_MAX_HOURS={a.max_hours})")
        elif od is not None:
            line += f", about ${h * od:.2f} on demand (at most ${a.max_hours * od:.2f} to LAB_MAX_HOURS={a.max_hours})"
        print(line)
        if e["over_deadline"]:
            print(f"WARNING: pair {p} is longer than LAB_MAX_HOURS={a.max_hours}: give LAB_MAX_HOURS={e['hours_needed']}")
    if od is None:
        unknown = [t for t in (a.type_a, a.type_b) if t not in PRICES]
        print(f"instances: no price for {', '.join(unknown)} in estimate.py's table: their dollars unknown")
        return 0
    prices = f"on-demand, us-east-1: {a.type_a} ${PRICES[a.type_a]}/h, {a.type_b} ${PRICES[a.type_b]}/h"
    if spot is not None:
        read = sorted({SPOT_READ.get(t, (SPOT_AS_OF, SPOT_SOURCE)) for t in (a.type_a, a.type_b)})
        prices += (f"; spot {a.type_a} ${SPOT_PRICES[a.type_a]}/h, {a.type_b} ${SPOT_PRICES[a.type_b]}/h ("
                   + "; ".join(f"{src} on {day}" for day, src in read)
                   + ": AWS bills each hour at the zone's spot price then, which moves; a one-time spot "
                   "instance's maximum is the on-demand price)")
    elif a.market == "spot":
        prices += " (spot bills less, at a price not in estimate.py's table)"
    print(f"prices: {prices}; EBS (40 GB gp3 each) and public IPv4 addresses extra, cents an hour; "
          "no data transfer charge (one zone, private addresses)")
    return 0


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--suite", choices=("latency", "transport"), default="latency")
    p.add_argument("--pair", default="")
    p.add_argument("--matrix", action="append", default=[])
    p.add_argument("--priority", choices=("Regular", "Spot"), default="Regular")
    p.add_argument("--tune", action="store_true", help="the tuning sessions (LAB_TUNE=1)")
    p.add_argument("--settings", default="")
    p.add_argument("--tune-arms", default="")
    p.add_argument("--tune-knobs", default="")
    for name in ("transport-warmup-s", "transport-point-s", "transport-switch-s"):
        p.add_argument("--" + name, type=int, default={"transport-switch-s": 5}.get(name, 30))
    for name in ("type-a", "type-b", "placement", "modes", "local-modes", "states", "load-sizes", "load-fractions",
                 "load-ladder", "mix"):
        p.add_argument("--" + name, default="")
    for name in ("reps", "load-seconds", "mix-seconds", "tput-seconds", "tput-flows", "rtt-messages", "rtt-warmup",
                 "max-hours", "warmup-seconds", "warmup-rate"):
        p.add_argument("--" + name, type=int, default=0)
    p.add_argument("--no-flatout", default="")
    p.add_argument("--market", choices=("on-demand", "spot"), default="on-demand")
    p.add_argument("--refuse-over-deadline", action="store_true")
    p.add_argument("--cloud", choices=("aws", "azure"), default="aws")
    p.add_argument("--print-hours", action="store_true", help="the wall time alone, in hours")
    a = p.parse_args()
    if a.suite == "transport":
        return transport_report(a)
    ladder = {}
    for entry in a.load_ladder.split():
        size, _, rates = entry.partition(":")
        ladder[int(size)] = [int(r) for r in rates.split(",") if r]
    mix = []
    for profile in a.mix.split(";"):
        specs = profile.partition("=")[2].split()
        streams = [(int(f[2]), int(f[3])) for f in (s.split(":") for s in specs) if len(f) >= 4]
        if streams:
            mix.append(streams)
    e = estimate(a.type_a, a.type_b, a.placement, a.reps, a.modes.split(), a.local_modes.split(), a.states.split(),
                 [int(s) for s in a.load_sizes.split()], [int(f) for f in a.load_fractions.split()], ladder,
                 a.load_seconds, mix, a.mix_seconds, a.tput_seconds, a.tput_flows, a.rtt_messages, a.rtt_warmup,
                 a.no_flatout == "1", a.max_hours, warmup_seconds=a.warmup_seconds,
                 warmup_rate=a.warmup_rate, prices=AZURE_PRICES if a.cloud == "azure" else PRICES)
    if a.print_hours:
        print(f"{e['wall_s'] / 3600:.2f}")
        return 0
    if a.cloud == "azure":
        return azure_report(a, e)
    over = (f"about {e['wall_s'] / 3600:.2f} h, longer than LAB_MAX_HOURS={a.max_hours}: each instance powers off "
            f"(and so terminates) {a.max_hours} h after launch, cutting the run short: give LAB_MAX_HOURS="
            f"{e['hours_needed']} (the estimate and a fifth more) or trim the plan (fewer modes, reps or states)")
    if a.refuse_over_deadline:
        if e["over_deadline"]:
            print(f"the plan takes {over}", file=sys.stderr)
            return 3
        return 0
    where = {"cluster": "one zone (cluster placement)", "precision-time": "one zone (precision-time placement)",
             "cross-az": "cross-AZ", "cross-region": "cross-region"}[a.placement]
    print(f"LAB_ESTIMATE: nothing created, no cloud call made. {where}, a {a.type_a}, b {a.type_b}")
    if a.warmup_seconds > 0 and a.load_sizes.split():
        print(f"warm-up: {a.warmup_seconds} s at {a.warmup_rate:,}/s before each mode's first ladder, in each rep "
              "(counted below)")
    print(f"wall time about {e['wall_s'] / 3600:.2f} h: {2 * e['wall_s'] / 3600:.2f} instance-hours")
    if e["over_deadline"]:
        print(f"WARNING: {over}")
    if e["unknown"]:
        print(f"instances: no price for {', '.join(e['unknown'])} in estimate.py's table: their dollars unknown")
    elif a.market == "spot" and e["spot_usd"] is not None:
        print(f"instances about ${e['spot_usd']:.2f} at spot prices (us-east-1 on {SPOT_AS_OF}, {SPOT_SOURCE}: "
              f"${SPOT_PRICES[a.type_a]:.3f}/h and ${SPOT_PRICES[a.type_b]:.3f}/h; AWS bills each hour at the spot price "
              f"of the run's zone then, which moves), at most ${e['instance_usd']:.2f} at on-demand prices "
              f"(${PRICES[a.type_a]}/h and ${PRICES[a.type_b]}/h: a one-time spot instance's default maximum); "
              f"should they run to LAB_MAX_HOURS={a.max_hours}, about ${e['spot_deadline_usd']:.2f} at spot "
              f"prices, at most ${e['deadline_usd']:.2f}")
    elif a.market == "spot":
        print(f"instances at most ${e['instance_usd']:.2f} (on-demand, us-east-1: ${PRICES[a.type_a]}/h and "
              f"${PRICES[a.type_b]}/h): an upper bound: LAB_SPOT=1 runs spot instances, billed at the spot price "
              f"of the hour, which is lower (not looked up); at most ${e['deadline_usd']:.2f} should they run to "
              f"LAB_MAX_HOURS={a.max_hours}")
    else:
        print(f"instances about ${e['instance_usd']:.2f} (on-demand, us-east-1: ${PRICES[a.type_a]}/h and "
              f"${PRICES[a.type_b]}/h); at most ${e['deadline_usd']:.2f} should they run to LAB_MAX_HOURS={a.max_hours}")
    bound = " (flat-out phases at 8.0 Gbit/s, above one flow's measured ceiling: an upper bound)" if e["bound"] else ""
    print(f"between the hosts about {e['bytes_ab'] / 1e9:.3f} GB a to b and {e['bytes_ba'] / 1e9:.3f} GB b to a{bound}")
    rule = {"cross-az": "cross-AZ: $0.01/GB out of one zone and $0.01/GB into the other",
            "cross-region": "cross-region: $0.02/GB out of the sending region"}.get(a.placement, "none within one zone")
    print(f"data transfer about ${e['transfer_usd']:.2f} ({rule})")
    if e["unknown"]:
        pass
    elif a.market == "spot" and e["spot_usd"] is not None:
        print(f"total about ${e['spot_usd'] + e['transfer_usd']:.2f} at spot prices, at most "
              f"${e['instance_usd'] + e['transfer_usd']:.2f} at on-demand prices")
    elif a.market == "spot":
        print(f"total at most ${e['instance_usd'] + e['transfer_usd']:.2f} (the instances at on-demand prices: "
              f"spot is lower)")
    else:
        print(f"total about ${e['instance_usd'] + e['transfer_usd']:.2f}")


def azure_report(a, e):
    """azure/lab.sh's LAB_ESTIMATE=1 and its deadline check."""
    over = (f"about {e['wall_s'] / 3600:.2f} h, longer than LAB_MAX_HOURS={a.max_hours}: the backstop deletes the "
            f"resource group {a.max_hours} h after creation, cutting the run short: give LAB_MAX_HOURS="
            f"{e['hours_needed']} (the estimate and a fifth more) or trim the plan (fewer modes, reps or states)")
    if a.refuse_over_deadline:
        if e["over_deadline"]:
            print(f"the plan takes {over}", file=sys.stderr)
            return 3
        return 0
    print(f"LAB_ESTIMATE: nothing created, no cloud call made. One VNet and proximity placement group, "
          f"a {a.type_a}, b {a.type_b}")
    print(f"wall time about {e['wall_s'] / 3600:.2f} h: {2 * e['wall_s'] / 3600:.2f} VM-hours"
          f"{' (over 2 h: lab.sh detaches it)' if e['wall_s'] > 7200 else ''}")
    if e["over_deadline"]:
        print(f"WARNING: {over}")
    if e["unknown"]:
        print(f"VMs: no price for {', '.join(e['unknown'])} in estimate.py's AZURE_PRICES: their dollars unknown")
        return 0
    pa, pb = AZURE_PRICES[a.type_a], AZURE_PRICES[a.type_b]
    print(f"VMs about ${e['instance_usd']:.2f} (pay-as-you-go Linux, {AZURE_REGION} on {AZURE_AS_OF}: "
          f"{a.type_a} ${pa:.3f}/h, {a.type_b} ${pb:.3f}/h); at most ${e['deadline_usd']:.2f} should they run to "
          f"LAB_MAX_HOURS={a.max_hours}; disks and public addresses extra, cents an hour")
    print("data transfer none (one VNet, one proximity placement group)")
    print(f"total about ${e['instance_usd']:.2f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
