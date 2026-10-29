#!/usr/bin/env python3
"""A clean transport run directory (transport-format.md), built the same
every time: three sets, three reps, every fairness rule met and every max
labelled, with one sender-late and one over-capacity point.

  2a  cross-host UDP: aeron-kernel, aeron-dpdk, mio-udp, eio-udp, A/A of
      aeron-kernel; 1 and 8 streams
  3b  0.1 % netem: aeron-kernel, mio-tcp, A/A of mio-tcp
  2b  same host, one way: aeron-spy, aeron-self-udp, A/A of aeron-spy

make(run_dir) writes it; the tests mutate one thing at a time."""
import hashlib
import json
import os
import random
import sys

REPS = 3
RATE = 10_000
DURATION = 30
WARMUP = 30
SIZE = 64
BASE_S = 1_790_000_040  # a minute boundary on CLOCK_REALTIME
PAYLOAD = hashlib.sha256(bytes(SIZE)).hexdigest()
BUILD = "core-0123abcd"
# per rep, shared by every arm (the host's drift), and the A/A's own noise
DRIFT = [0, 100, 200]
AA_NOISE = [0.0015, -0.002, 0.003]
HOT = [4, 6, 8, 10]

# arm: (transport, wire bytes, busy threads)
ARMS = {
    "aeron-kernel": ("aeron-udp", 138, 4),
    "aeron-dpdk": ("aeron-dpdk", 138, 4),
    "mio-udp": ("udp", 106, 2),
    "eio-udp": ("udp", 106, 2),
    "mio-tcp": ("tcp", 110, 2),
    "aeron-spy": ("aeron-spy", 96, 3),
    "aeron-self-udp": ("aeron-udp", 138, 4),
}
# set: (vms, [(arm, aa_of or None)], stream counts, netem, kind)
SETS = {
    "2a": ({"a": "vm-a1", "b": "vm-b1"},
           [("aeron-kernel", None), ("aeron-dpdk", None), ("mio-udp", None), ("eio-udp", None), ("aa", "aeron-kernel")],
           [1, 8], None, "load"),
    "3b": ({"a": "vm-a2", "b": "vm-b2"}, [("aeron-kernel", None), ("mio-tcp", None), ("aa", "mio-tcp")], [1], "0.1",
           "load"),
    "2b": ({"a": "vm-a2"}, [("aeron-spy", None), ("aeron-self-udp", None), ("aa", "aeron-spy")], [1], None, "oneway"),
}
# p50 in ns by (set, arm, streams); the A/A takes its arm's
P50 = {("2a", "aeron-kernel", 1): 20_000, ("2a", "aeron-dpdk", 1): 12_000, ("2a", "mio-udp", 1): 15_000,
       ("2a", "eio-udp", 1): 15_040, ("2a", "aeron-kernel", 8): 24_000, ("2a", "aeron-dpdk", 8): 13_000,
       ("2a", "mio-udp", 8): 17_000, ("2a", "eio-udp", 8): 17_500, ("3b", "aeron-kernel", 1): 25_000,
       ("3b", "mio-tcp", 1): 30_000, ("2b", "aeron-spy", 1): 300, ("2b", "aeron-self-udp", 1): 9_000}
MULT = {"min": 0.8, "p50": 1.0, "p90": 1.1, "p99": 1.5, "p99.9": 2.0, "p99.99": 3.0, "max": 10.0}
# what explains each arm's max, by set and arm (cycled over its points)
CAUSES = {("2a", "aeron-kernel"): ["nak", "retransmit"], ("2a", "aa"): ["host-pause"],
          ("2a", "aeron-dpdk"): ["dpdk-refill"], ("2a", "mio-udp"): ["irq", "softirq"],
          ("2a", "eio-udp"): ["page-fault"], ("3b", "aeron-kernel"): ["retransmit"],
          ("3b", "mio-tcp"): ["tcp-rto"], ("3b", "aa"): ["tcp-tlp"], ("2b", "aeron-spy"): ["page-fault"],
          ("2b", "aeron-self-udp"): ["irq"], ("2b", "aa"): ["host-pause"]}
# (set, arm, streams, rep): the flagged points
LATE = {("2a", "eio-udp", 8, 2)}
OVER = {("2a", "mio-udp", 8, 3)}

SETTINGS_ENV = """\
# every arm, both hosts
shared.net.core.busy_poll=50
shared.net.core.busy_read=50
shared.kernel.numa_balancing=0
# per arm, each swept for every arm that has it
aeron-kernel.aeron.threading.mode=DEDICATED
aeron-dpdk.aeron.threading.mode=DEDICATED
aeron-spy.aeron.threading.mode=DEDICATED
aeron-self-udp.aeron.threading.mode=DEDICATED
sweep.aeron.threading.mode=aeron-kernel,aeron-dpdk,aeron-spy,aeron-self-udp
mio-udp.socket.busy_poll_us=50
eio-udp.socket.busy_poll_us=50
sweep.socket.busy_poll_us=mio-udp,eio-udp
mio-tcp.tcp.quickack=1
sweep.tcp.quickack=mio-tcp
# the data NICs' interrupt deferral (napi_defer_hard_irqs, read back per
# point): each technology at its own best (best.knob.napi_defer), Aeron's
# busy-polling kernel path deferred, the sockets not
aeron-kernel.knob.napi_defer=2
aeron-self-udp.knob.napi_defer=2
mio-udp.knob.napi_defer=0
eio-udp.knob.napi_defer=0
mio-tcp.knob.napi_defer=0
best.knob.napi_defer=aeron-kernel:2,aeron-self-udp:2,eio-udp:0,mio-tcp:0,mio-udp:0
"""

HOSTCHECK = """\
T1 ok: the busy-spinning CPUs are isolated
T10 ok: SMT is off
T19 unknown: FaultWatch: no hot thread registered yet: not started
"""


def settings_of(arm):
    out = {}
    for line in SETTINGS_ENV.splitlines():
        if not line or line.startswith("#") or "=" not in line:
            continue
        k, v = line.split("=", 1)
        prefix, _, knob = k.partition(".")
        if prefix == "shared" or prefix == arm:
            out[knob] = v
    return out


def loss_sources(transport):
    if transport == "tcp":
        # the traffic's retransmits split as tcp_lat reports them: those the
        # peer reported duplicates (DSACK: spurious) and the rest (lost)
        return {"tcp_info": {"retrans": 3, "dsack_dups": 1, "retrans_not_dsack": 2, "lost_end": 0, "rtos": 0},
                "nstat": {"TcpRetransSegs": 3}}
    if transport.startswith("aeron"):
        return {"aeron": {"naks-sent": 1, "retransmits-sent": 1}}
    if transport == "udp":
        return {"nstat": {"UdpRcvbufErrors": 0}}
    return {}


def make(run):
    os.makedirs(run, exist_ok=True)
    with open(os.path.join(run, "transport-settings.env"), "w") as f:
        f.write(SETTINGS_ENV)
    rows = []
    index = 0
    for set_, (vms, arms, stream_counts, netem, kind) in SETS.items():
        folder = os.path.join(run, set_)
        os.makedirs(folder, exist_ok=True)
        hosts = sorted(vms)
        for h in hosts:
            with open(os.path.join(folder, f"{h}-hostcheck.txt"), "w") as f:
                f.write(HOSTCHECK)
        gaps = {h: [] for h in hosts}
        events = {}
        seen = {}
        for streams in stream_counts:
            for rep in range(1, REPS + 1):
                seed = 1000 * streams + rep
                order = sorted(a for a, _ in arms)
                random.Random(seed).shuffle(order)
                round_id = f"{set_}-s{streams}-n{netem or '-'}-r{RATE}-rep{rep}"
                for arm, ref in arms:
                    base_arm = ref or arm
                    transport, wire, busy = ARMS[base_arm]
                    label = f"{set_}:{arm}" + (f"@STREAMS={streams}" if streams > 1 else "") + (
                        f"@NETEM={netem}" if netem else "")
                    tag = f"{set_}-{label.partition(':')[2]}-{SIZE}-{rep}-r{RATE}" + (
                        "-oneway" if kind == "oneway" else "")
                    p50 = P50[(set_, base_arm, streams)] + DRIFT[rep - 1]
                    noise = AA_NOISE[rep - 1] if ref else 0.0
                    lat = {k: round(p50 * m * (1 + noise)) for k, m in MULT.items()}
                    lat["max"] += rep * 1000
                    late = (set_, arm, streams, rep) in LATE
                    over = (set_, arm, streams, rep) in OVER
                    scheduled = RATE * DURATION
                    achieved = 9_000 if over else RATE
                    sent = achieved * DURATION
                    retrans = 3 if transport in ("tcp",) or (netem and transport.startswith("aeron")) else 0
                    point = ("tcp" if transport == "tcp" else "ipc" if transport == "aeron-spy" else "udp") + f"-r{RATE}"
                    rows.append([rep, kind, label, point, SIZE, RATE, 1, lat["p50"], lat["p90"], lat["p99"],
                                 lat["p99.9"], lat["p99.99"], lat["max"], lat["p50"], sent, sent, sent, 0, achieved,
                                 2_000, scheduled, scheduled // 50 if late else 0, 0, lat["min"], 0, 0, 0, retrans,
                                 wire, busy, lat["p50"], lat["p99"], lat["p99.99"], lat["max"]])
                    start_s = BASE_S + index * 120 + 20
                    index += 1
                    causes = CAUSES[(set_, arm)]
                    n = seen.get((set_, arm), 0)
                    seen[(set_, arm)] = n + 1
                    cause = causes[n % len(causes)]
                    # each max at its own phase of the minute: none recurs
                    # (spikes.py session_periodic)
                    due_s = 5.0 + rep + round(index * 7.31 % 20, 2)
                    t0 = start_s + due_s
                    t1 = t0 + lat["max"] / 1e9
                    mid_ns = int((t0 + (t1 - t0) / 2) * 1e9)
                    excess_ns = lat["max"] - lat["p50"]
                    with open(os.path.join(folder, f"spikes-{tag}.csv"), "w") as f:
                        f.write(f"# start_realtime_ns={start_s * 10**9}\n# spikes_total=2\n"
                                f"# max={int(due_s * 1e9)},{lat['max']},0\nstream,due_ns,latency_ns\n"
                                f"0,{int(due_s * 1e9) - 10**9},{lat['max'] // 2}\n0,{int(due_s * 1e9)},{lat['max']}\n")
                    host_ev = []
                    span = (mid_ns - int(excess_ns * 0.4), mid_ns + int(excess_ns * 0.4))
                    if cause == "host-pause":
                        gaps["a"].append(((span[1]) / 1e9, (span[1] - span[0]) / 1e3))
                    elif cause in ("irq", "softirq"):
                        host_ev.append(("a", cause, span[0], span[1], HOT[0], "mana_q1" if cause == "irq" else "NET_RX"))
                    elif cause == "page-fault":
                        host_ev.append(("a", cause, span[0], span[1], HOT[1], "1"))
                    elif cause == "dpdk-refill":
                        host_ev.append(("a", cause, span[0], span[1], HOT[1], "256"))
                    elif cause in ("nak", "retransmit"):
                        host_ev.append(("b", cause, mid_ns, mid_ns, "-", "1"))
                    elif cause in ("tcp-rto", "tcp-tlp"):
                        host_ev.append(("a", cause, mid_ns, mid_ns, "-", "1"))
                        sec = mid_ns // 10**9 * 10**9
                        host_ev.append(("b", "tc-drop", sec, sec + 10**9, "-", "1"))
                    for h, k, s, e, cpu, detail in host_ev:
                        events.setdefault((h, tag), []).append(f"{k},{s},{e},{cpu},{detail}")
                    threads = {}
                    for h in hosts:
                        busy_list = [{"tid": 100 + i, "comm": f"busy{i}", "cpu": HOT[i], "util": 0.99}
                                     for i in range(busy)]
                        idle = [{"tid": 1, "comm": "sshd", "cpu": 0, "util": 0.02},
                                {"tid": 2, "comm": "ksoftirqd/2", "cpu": 2, "util": 0.01}]
                        threads[h] = [busy_list + idle for _ in range(3)]
                    netem_counters = None
                    if netem:
                        netem_counters = {h: {nic: {"qdisc": "netem", "loss_pct": netem, "sent_pkts": 299_700,
                                                    "dropped": 300} for nic in ("eth1", "eth2")} for h in hosts}
                    manifest = {
                        "format": 1, "set": set_, "arm": arm, "label": label, "kind": kind, "transport": transport,
                        "round": round_id, "rep": rep, "rate": RATE, "streams": streams, "size": SIZE,
                        "duration_s": DURATION, "warmup_s": WARMUP, "netem": netem, "aa": bool(ref), "aa_of": ref,
                        "payload_sha256": PAYLOAD, "wire_bytes": wire, "core_build": BUILD, "seed": seed,
                        "arm_order": order, "scheduled": scheduled, "late": scheduled // 50 if late else 0,
                        "max_send_lag_ns": 2_000, "vms": vms,
                        "cpus": {h: {"housekeeping": [0], "irq": [2], "hot": HOT} for h in hosts},
                        "threads": threads,
                        "irqs": {h: [{"irq": "45", "name": "mana_q0", "data_nic": True, "cpu": 2, "delta": 1200},
                                     {"irq": "12", "name": "timer", "data_nic": False, "cpu": 4, "delta": 3}]
                                 for h in hosts},
                        "hostcheck": {h: f"{h}-hostcheck.txt" for h in hosts},
                        "settings": {h: settings_of(base_arm) for h in hosts},
                        "netem_counters": netem_counters,
                        "loss_counters": {h: loss_sources(transport) for h in hosts},
                    }
                    with open(os.path.join(folder, f"manifest-{tag}.json"), "w") as f:
                        json.dump(manifest, f, indent=1, sort_keys=True)
        for h in hosts:
            with open(os.path.join(folder, f"{h}-diag-{set_}.log"), "w") as f:
                f.write("== gapspin\n# threshold_us 10\n")
                for end, us in gaps[h]:
                    f.write(f"{end:.6f} {us:.0f}\n")
        for (h, tag), lines in events.items():
            with open(os.path.join(folder, f"{h}-events-{tag}.csv"), "w") as f:
                f.write("kind,start_ns,end_ns,cpu,detail\n" + "\n".join(lines) + "\n")
    with open(os.path.join(run, "results.csv"), "w") as f:
        for r in rows:
            f.write(",".join(str(v) for v in r) + "\n")
    return run


if __name__ == "__main__":
    print(make(sys.argv[1]))
