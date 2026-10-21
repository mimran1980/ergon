#!/usr/bin/env python3
"""What a run of aws/lab.sh would cost, before it creates anything
(LAB_ESTIMATE=1: lab.sh prints this and exits, with no cloud call).

Wall time, from the run of 2026-10-15 on two c7i.4xlarge (aws-20261015-185813:
its files' times), per step:
  setup        600 s  launch to the first state: bootstrap, reboot, build, tune,
                      the clock checks (9.7 minutes there)
  state        pinned 150 s (pre-flight 120 s, diagnostics); isolated and
                      isolated-poll 450 s (kernel command line, reboot, tune,
                      clockcheck 120 s, Criterion, pre-flight 120 s: 7.3 minutes)
  mode and rep 12 s (drivers started and stopped, counters, loss reads) plus the
                      round trip's 85 s (RTT_SECONDS 20, its warm-up, the pong's start)
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
out. The round trip: its warm-up and messages of 32 bytes, each way. A load
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
    "c8g.xlarge": 0.15952, "c9g.xlarge": 0.17388,
    "c7g.4xlarge": 0.58, "c8g.4xlarge": 0.63808, "c8g.metal-24xl": 3.82848,
    "c7gn.4xlarge": 0.9984, "c8gn.4xlarge": 0.948, "c9g.4xlarge": 0.69552,
    "c8i.4xlarge": 0.74968,
    "c9g.2xlarge": 0.34776, "c8gn.xlarge": 0.237, "c6in.4xlarge": 0.9072,
}
# us-east-1 Linux spot prices ($/h) for the types the final rounds use, as
# of SPOT_AS_OF (instances.vantage.sh, its data of 2026-10-15 16:35): spot
# moves by the hour and the zone, so refresh them before a round
SPOT_PRICES = {"c9g.2xlarge": 0.153, "c8g.xlarge": 0.077, "c7i.2xlarge": 0.200}
SPOT_AS_OF, SPOT_SOURCE = "2026-10-15", "instances.vantage.sh"
SETUP_S, TEARDOWN_S = 600, 180
STATE_S = {"pinned": 150, "isolated": 450, "isolated-poll": 450}
MODE_S, RTT_S = 12, 85
POINT_EXTRA_S, MIX_EXTRA_S, TPUT_EXTRA_S, MAX_EXTRA_S = 7, 7, 9, 11
WIRE_OVERHEAD = 32 + 28  # Aeron's data header, IPv4 and UDP
# One AWS flow's ceiling, rounded up: md-dpdk/BENCHMARKS.md measured 7.75-7.77 Gbit/s
# of Aeron frames (about 7.9 as the IP packets counted here) on c7i.4xlarge, so a
# flat-out phase's bytes at 8.0 are an upper bound.
FLOW_CAP_BYTES_S = 8.0e9 / 8
RTT_SIZE = 32
TRANSFER_USD_GB = {"cluster": 0.0, "precision-time": 0.0, "cross-az": 0.02, "cross-region": 0.02}


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
             warmup_seconds=0, warmup_rate=0):
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
            secs, to_b, to_a = MODE_S + RTT_S, 0, 0
            if not local:
                rtt = (rtt_messages + rtt_warmup) * (RTT_SIZE + WIRE_OVERHEAD)
                to_b, to_a = rtt, rtt
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
    unknown = [t for t in (type_a, type_b) if t not in PRICES]
    hourly = None if unknown else PRICES[type_a] + PRICES[type_b]
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


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    for name in ("type-a", "type-b", "placement", "modes", "local-modes", "states", "load-sizes", "load-fractions",
                 "load-ladder", "mix"):
        p.add_argument("--" + name, default="")
    for name in ("reps", "load-seconds", "mix-seconds", "tput-seconds", "tput-flows", "rtt-messages", "rtt-warmup",
                 "max-hours", "warmup-seconds", "warmup-rate"):
        p.add_argument("--" + name, type=int, default=0)
    p.add_argument("--no-flatout", default="")
    p.add_argument("--market", choices=("on-demand", "spot"), default="on-demand")
    p.add_argument("--refuse-over-deadline", action="store_true")
    a = p.parse_args()
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
                 warmup_rate=a.warmup_rate)
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


if __name__ == "__main__":
    sys.exit(main())
