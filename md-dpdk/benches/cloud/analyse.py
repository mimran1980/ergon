#!/usr/bin/env python3
"""Tables for md-dpdk/BENCHMARKS.md from lab.sh's results.csv, in rusteron's form:
each cell is the median across reps, the range across reps in brackets.

results.csv rows (lab.sh prefixes the rep):
  <rep>,rtt,<mode>,udp,p50,p90,p99,p99.9,p99.99,max,mean,samples   (ns)
  <rep>,tput,<mode>,udp,<median msgs/s>,<min>,<max>,<received>,<lost>,<window start ns>,<window end ns>
        (the measured window on the subscriber's wall clock; older clients end at <max> or <lost>)
  <rep>,rxhw,<mode>,<mean ns>,<max ns>,<frames>[,<min ns>,<negative>]   (one per host; NIC RX
        timestamps only: NIC arrival to the receive burst that carried the frame over <frames>;
        <negative>, frames stamped after the host clock read at their burst, PHC/host clock skew;
        the read is once a burst and before any frame of it is dispatched, so a frame after the
        burst's first is measured before its own dispatch; mean and max empty when only those came,
        min and negative empty from a driver built before they were counted)
  <rep>,tputmax,<mode>,<size>,<median msgs/s>,<min>,<max>   (the load sweep's one-way maximum)
  <rep>,tputflows,<mode>,<size>,<flows>,<sum msgs/s>,<flow 1 msgs/s>,...,<flow n>,<overlap>
        (several flows' sustained maxima at once; each flow's mean over its own measured window,
        and the sum of those -- the concurrent aggregate, not a sum of per-flow medians; a flow
        without a result, or without a window, leaves the sum empty;
        overlap: the span every flow's measured window shares over the shortest window, truncated to
        the thousandth, empty when a flow reported no window and absent in rows from before it. A run
        counts from MIN_OVERLAP; one below it, or with it unknown, is left out and reported)
  <rep>,load,<mode>,udp-<fraction|burst>,<size>,<rate>,<burst>,p50,...,max,mean,samples,sent,received,
        backpressured,achieved msgs/s,max send lag ns[,scheduled,late,censored]
        (scheduled is what the run owed, late the sends that found the following slot already due,
        censored the round trips at the histogram's bound whose true value is only known to be at
        least that; all three absent in rows from before they were counted)
  <rep>,mix,<mode>,udp-<profile>/<stream>,<size>,<rate>,<burst>,... (as load: one row per stream of a mix)
  <rep>,loss,<mode>[:rtt|:<size>:<fraction>|:<size>:max|:<size>:flows<n>|:mix:<profile>],<host>,<key>=<growth>;...
        (routes-missing as found after the run; VALIDITY's counters check the run and are not loss)
  0,netrtt,<state>,<min ns>,<avg ns>,<max ns>,<mdev ns>,<replies>/<sent>
        (the network's own round trip, ping between the hosts' data NICs at the state's start)
  <rep>,mtu,<mode>,<host>,<link MTU>,<Aeron MTU>   (the MTU each host's run used)
  <rep>,aeron,<mode>[:rtt|:<size>:<fraction>|...],<host>,info-<key>=<value>;...
        (Aeron's own telemetry over the run, as loss rows are labelled: each agent's longest duty
        cycle in ns since its driver started, its cycles over the threshold and the threshold; the
        sender's flow-control limits, status messages sent and received, flow control under and
        over runs; information, never loss)

Next to results.csv, each load point's histograms (histogram-<state>-<mode>-<size>-<rep>-<point>.csv,
value_ns,count) are merged over its reps into percentiles of every sample.

Next to results.csv, spikes-*.csv (stream,due_ns,latency_ns[,ping]: round trips of 100 µs
or more) are summarised: whether spikes recur at one phase of each second, the
signature of once-a-second work. Where lib.sh saved the counters around the
flat-out runs (<host>-counters-{tput,max,flows}-{before,after}-*.log, in the run
directory or a state's below it), a table of their wire frames per message.

analyse.py <results.csv> [<run dir>]
analyse.py --ab <mode A> <mode B> <results.csv>
    one mode against another (each bare, compared within every state that has
    both, or as <state>:<mode>): per metric both medians and ranges and whether
    the ranges overlap, the runs with NAKs pooled with Fisher's exact test, and
    the agents' poll-cycle gaps in each rep's idle round trip
"""
import csv
import glob
import math
import os
import re
import statistics
import sys
from collections import defaultdict
from types import SimpleNamespace

from spikes import agent_gaps, open_spikes, pmd_bursts, spike_files

ORDER = ["kernel", "kernel-mono", "kernel-busy", "kernel-busy-mono", "kernel-raw", "kernel-raw-mono", "dpdk",
         "dpdk-mono", "dpdk+idle", "dpdk-spin", "dpdk-spin+idle", "dpdk-adaptive+idle", "dpdk-raw", "dpdk-raw-mono",
         "dpdk-pure", "local-kernel", "local-dpdk", "local-dpdk-adaptive", "local-dpdk-spin"]
# isolated-poll: isolated, with idle CPUs polling; -nodiag: the state without
# the diagnostics (lib.sh run_bench)
STATES = ["", "pinned", "pinned-nodiag", "isolated", "isolated-nodiag", "isolated-poll", "isolated-poll-nodiag"]
NAMES = {
    "kernel": "kernel UDP",
    "kernel-busy": "kernel UDP, busy polling",
    "dpdk": "DPDK",
    "dpdk-spin": "DPDK, agents poll local sockets",
    "dpdk+idle": "DPDK, idle localhost subscription",
    "dpdk-spin+idle": "DPDK, idle localhost subscription, agents poll it",
    "local-kernel": "localhost, kernel UDP",
    # the build's default local-I/O: epoll up to sweep 2 (2026-10-14), adaptive since
    "local-dpdk": "localhost, DPDK driver",
    "local-dpdk-spin": "localhost, DPDK driver, agents poll",
    "local-dpdk-adaptive": "localhost, DPDK driver, adaptive local-I/O",
    "local-dpdk-epoll": "localhost, DPDK driver, epoll local-I/O",
    "dpdk-adaptive+idle": "DPDK, idle localhost subscription, adaptive local-I/O",
    # the raw floor (modes.sh mode_raw): the floor client, rtt's round trip with no Aeron
    "kernel-raw": "kernel UDP, no Aeron (raw floor)",
    "dpdk-raw": "DPDK packet path, no Aeron (raw floor)",
    # the pure floor (modes.sh mode_pure): purefloor.c, DPDK alone
    "dpdk-pure": "DPDK alone, no ergon-md-dpdk or Aeron (pure floor)",
}
# a -mono mode: the transport and the clients clock with CLOCK_MONOTONIC, not TSC
NAMES.update({f"{m}-mono": f"{n}, monotonic clock" for m, n in list(NAMES.items())})
# <mode>@AERON_THREADING_MODE=<it>: the media driver's agents (vm.sh, host.sh threading_env)
THREADING = {
    "DEDICATED": "conductor, sender and receiver on a thread each",
    "SHARED_NETWORK": "sender and receiver on one thread",
    "SHARED": "conductor, sender and receiver on one thread",
    # no media driver process: each client embeds the driver and runs it on its own thread
    "INVOKER": "conductor, sender, receiver and the client on one thread",
}
RTT_COLUMNS = ["p50", "p90", "p99", "p99.9", "p99.99", "max"]


def mode_name(mode):
    """A mode as tables name it: its name, or the mode as given; a threading
    mode override named after the rest, with Aeron's name for it."""
    parts = mode.split("@")
    threading = [p for p in parts[1:] if p.startswith("AERON_THREADING_MODE=")]
    if not threading:
        return NAMES.get(mode, mode)
    rest = "@".join([parts[0]] + [p for p in parts[1:] if p not in threading])
    value = threading[-1].partition("=")[2]
    return f"{NAMES.get(rest, rest)}, {THREADING.get(value, value)} ({value})"


def name(label):
    state, _, mode = label.rpartition(":")
    return f"{state}, {mode_name(mode)}" if state else mode_name(mode)


def cell(values, scale, digits):
    values = sorted(v / scale for v in values)
    return f"{statistics.median(values):.{digits}f} ({values[0]:.{digits}f}–{values[-1]:.{digits}f})"


def table(header, rows):
    widths = [max(len(r[i]) for r in [header] + rows) for i in range(len(header))]
    line = lambda r: "| " + " | ".join(c.ljust(w) for c, w in zip(r, widths)) + " |"
    return "\n".join([line(header), "| " + " | ".join("-" * w for w in widths) + " |"] + [line(r) for r in rows])


# labels are <state>:<mode> (lab.sh), or a bare mode
def key(label):
    state, _, mode = label.rpartition(":")
    return (STATES.index(state) if state in STATES else len(STATES), state,
            ORDER.index(mode) if mode in ORDER else len(ORDER), mode)


def file_name(label):
    """A label as lib.sh's file names spell it: <state>-<mode>, a / (BIN's) as _."""
    state, _, mode = label.rpartition(":")
    return f"{state}-{mode}".replace("/", "_") if state else mode.replace("/", "_")


def rep_key(rep):
    """Reps in numeric order (a rep that is not a number after them)."""
    return (0, rep, "") if isinstance(rep, int) else (1, 0, str(rep))


def run_dirs(path):
    """Where a run's files are: results.csv's directory, then the state
    directories below it (lib.sh fetches each state's files into its own)."""
    folder = os.path.dirname(os.path.abspath(path))
    return [folder] + sorted(d for d in glob.glob(os.path.join(folder, "*")) if os.path.isdir(d))


# Several flows' sustained maxima count only where their measured windows
# overlapped this much of the shortest: below it, a flow had the path to itself
# for a while and their sum overstates what they carry at once.
MIN_OVERLAP = 0.9


def overlap_of(field):
    """A tputflows row's overlap, or None when it is unknown: empty (a flow
    reported no window) or not a number."""
    try:
        overlap = float(field)
    except ValueError:
        return None
    return overlap if math.isfinite(overlap) else None


def overlap_name(overlap):
    return "unknown" if overlap is None else f"{overlap:.3f}"


def work_of(row):
    """A load or mix row's scheduled, late and censored counts -- the work
    behind its percentiles -- always three, `None` each when the row is from
    before they were counted: there is nothing to report, which is not a
    measured zero."""
    return [int(v) for v in row[20:23]] if len(row) >= 23 else [None, None, None]


def work_columns(reps):
    """A point's `echoed` and `late, censored` cells: completed, issued and
    scheduled together, and how often the run fell more than one slot behind
    plus how many round trips the histogram's bound censored. A rep that
    predates the three counts has none to add, so the point is reported
    without them rather than as a zero that would read as measured."""
    sent = sum(r[6] for r in reps)
    received = sum(r[7] for r in reps)
    if any(r[-3] is None for r in reps):
        return f"{received:,}/{sent:,}", "—"
    scheduled, late, censored = (sum(r[i] for r in reps) for i in (-3, -2, -1))
    work = f"late {late:,}, censored {censored:,}" if late or censored else "none"
    return f"{received:,}/{sent:,}/{scheduled:,}", work


def read_results(path):
    """Every row of results.csv, by mode, and by mode and rep (the A/B's and
    the frames table's)."""
    r = SimpleNamespace(
        rtt=defaultdict(list), rtt_lost=defaultdict(list), tput=defaultdict(list), tput_lost=defaultdict(list),
        rxhw=defaultdict(list), tputmax=defaultdict(list), load=defaultdict(list), load_failed=defaultdict(int),
        mix=defaultdict(list), loss=defaultdict(lambda: defaultdict(int)),
        # [(label, size, flows)]: (rep, sum, [each flow], overlap) per run counted, the
        # runs a flow failed, and (rep, overlap or None: unknown) per run left out
        # for its flows' overlap
        tputflows=defaultdict(list), tputflows_failed=defaultdict(int), tputflows_apart=defaultdict(list),
        # by rep: rtt [(label, rep)], tput [(label, rep)], tputmax [(label, size, rep)],
        # tputflows [(label, size, flows, rep)], load [(label, size, point, rep)],
        # burst [(label, size, point)], loss [(loss label, rep, host)]
        rtt_rep={}, tput_rep={}, tputmax_rep={}, tputflows_rep={}, load_rep={}, load_echo={}, burst={}, loss_rep={},
        # the network's own round trip by state: [(min, avg, max) ns]
        netrtt=defaultdict(list),
        # Aeron's telemetry by (loss label, host): [(rep, {key: value})]
        aeron=defaultdict(list),
        # the media driver's start-up host checks by mode: [(rep, host, severity, what it said)]
        hostcheck=defaultdict(list))
    with open(path, newline="") as f:
        for row in csv.reader(f):
            if len(row) < 5:
                continue
            kind, mode = row[1], row[2]
            rep = int(row[0]) if row[0].isdigit() else row[0]
            if kind == "rtt" and len(row) >= 12:
                r.rtt[mode].append([float(v) for v in row[4:10]] + [float(row[11])])
                r.rtt_rep[(mode, rep)] = [float(v) for v in row[4:10]]
                if len(row) >= 13:  # lost round trips (unreliable channels; 0 otherwise)
                    r.rtt_lost[mode].append(int(row[12]))
            elif kind == "tput" and len(row) >= 7:
                r.tput[mode].append(float(row[4]))
                r.tput_rep[(mode, rep)] = float(row[4])
                if len(row) >= 9:  # received and lost (by the publisher's sequence numbers)
                    r.tput_lost[mode].append((int(row[7]), int(row[8])))
            elif kind == "rxhw" and len(row) >= 6:
                num = lambda i: float(row[i]) if len(row) > i and row[i] != "" else None
                r.rxhw[mode].append((num(3), num(4), num(6), num(7)))
            elif kind == "tputmax" and len(row) >= 5:
                r.tputmax[(mode, int(row[3]))].append(float(row[4]))
                r.tputmax_rep[(mode, int(row[3]), rep)] = float(row[4])
            elif kind == "tputflows" and len(row) >= 7:
                size, n = int(row[3]), int(row[4])
                flows = row[6:6 + n]
                if len(flows) != n or not all(flows):  # a flow without a result
                    r.tputflows_failed[(mode, size, n)] += 1
                    continue
                # the flows' overlap; a row from before it was recorded has none.
                # An empty sum goes the same way: a flow reported no window, so
                # there is no mean to add and no aggregate to report (lib.sh
                # leaves the sum empty rather than summing medians)
                overlap = overlap_of(row[6 + n] if len(row) > 6 + n else "")
                if not row[5] or overlap is None or overlap < MIN_OVERLAP:
                    r.tputflows_apart[(mode, size, n)].append((rep, overlap))
                    continue
                r.tputflows[(mode, size, n)].append((rep, float(row[5]), [float(v) for v in flows], overlap))
                r.tputflows_rep[(mode, size, n, rep)] = float(row[5])
            elif kind == "load" and len(row) >= 20:
                point = (mode, int(row[4]), row[3].removeprefix("udp-"))
                if int(row[14]) == 0:  # no echo after warm-up: a failed rep, no percentiles
                    r.load_failed[point] += 1
                    continue
                r.load[point].append([float(v) for v in row[7:13]]
                                     + [int(row[15]), int(row[16]), int(row[17]), int(row[5]), float(row[18]), int(row[19])]
                                     + work_of(row))
                r.load_rep[point + (rep,)] = [float(v) for v in row[7:13]]
                r.load_echo[point + (rep,)] = (int(row[15]), int(row[16]))  # sent, received
                r.burst[point] = int(row[6])
            elif kind == "mix" and len(row) >= 20 and int(row[14]) > 0:
                profile, _, stream = row[3].removeprefix("udp-").partition("/")
                r.mix[(mode, profile, stream, int(row[4]), int(row[5]), int(row[6]))].append(
                    [float(v) for v in row[7:13]] + [int(row[15]), int(row[16]), int(row[17]), float(row[18])]
                    + work_of(row))
            elif kind == "netrtt" and len(row) >= 6:
                r.netrtt[mode].append(tuple(float(v) for v in row[3:6]))
            elif kind == "hostcheck" and len(row) >= 5:
                r.hostcheck[mode].append((rep, row[3], row[4], row[5] if len(row) > 5 else ""))
            elif kind == "aeron" and len(row) >= 5:
                kv = dict(x.split("=", 1) for x in row[4].split(";") if "=" in x)
                r.aeron[(mode, row[3])].append((rep, {k.removeprefix("info-"): int(v) for k, v in kv.items()
                                                      if k.startswith("info-") and v.lstrip("-").isdigit()}))
            elif kind == "loss" and len(row) >= 5:
                grew = r.loss_rep.setdefault((mode, rep, row[3]), {})
                for kv in row[4].split(";"):
                    if "=" in kv:
                        k, v = kv.split("=", 1)
                        # runs before the counters tool named it noport-
                        k = "noport-dpdk-queue-drop" if k == "dpdk-queue-drop" else k
                        # the kernel's routes, meaningless while DPDK owns the NICs (a
                        # first version counted the NICs it could not find as missing)
                        if k == "routes-missing" and "dpdk" in (mode.split(":") + [""])[1]:
                            continue
                        r.loss[mode][k] += int(v)
                        grew[k] = grew.get(k, 0) + int(v)
    return r


# The transport's send-side counters: the TX ring full, no ARP answer yet,
# no mbuf free, or the kernel socket's send buffer full. A send one of them
# refuses is also an Aeron short send, which counts as loss; on its own a
# full ring may only have queued a vector, refusing nothing, so these do not.
SEND_CAUSES = {"dpdk-tx-eagain", "dpdk-arp-miss", "dpdk-nobufs", "kernel-udp-SndbufErrors"}

# Counters that check the run rather than count loss: listed with the loss
# counters, never loss (loss_free, the multi-flow table). isolated-nic-irqs is
# the data NICs' interrupts on CPUs other than the housekeeping one (vm.sh
# loss, irqs.sh; under DPDK, those of the IRQs the kernel keeps on the NICs):
# vm.sh pins every IRQ there before each driver starts, so where it grows the
# kernel placed a NIC's IRQs anew (ENA does, bringing a NIC up or resetting
# it) and the run took interrupts off the housekeeping CPU (nic_irqs). Under a
# mode's @IRQ= vm.sh pins the NICs' queue IRQs to the CPU it chose instead,
# and counts theirs off that CPU.
VALIDITY = {"isolated-nic-irqs"}


def ladder_limit(r, m, size, holds, failed_word):
    """The highest load point's rate (median offered) for <m> at <size> below
    the first one where `holds(point, reps)` is false or a rep failed (no echo
    after warm-up). Bursts are left out (they are not a rate the path
    sustains)."""
    points = [(statistics.median(x[9] for x in r.load[p]), p) for p in r.load
              if p[0] == m and p[1] == size and p[2] != "burst"]
    points += [(int(p[2][1:]), p) for p in r.load_failed if p[0] == m and p[1] == size and p[2].startswith("r")
               and p not in r.load]
    points.sort(key=lambda x: x[0])
    if not points:
        return "no ladder"
    best = None
    for rate, p in points:
        reps = sorted(k[3] for k in r.load_rep if k[:3] == p)
        if not reps or r.load_failed.get(p) or not holds(p, reps):
            break
        best = rate
    return f"{best:,.0f}/s" if best is not None else f"none ({points[0][0]:,.0f}/s {failed_word})"


def counts_as_loss(k, v):
    """Whether a loss row's counter growth is loss: short sends included;
    `sat-` and `noport-` counters, VALIDITY's, SEND_CAUSES on their own and
    Aeron's telemetry (`info-`: lib.sh loss_row keeps it to aeron rows) are
    not."""
    return (bool(v) and not k.startswith(("sat-", "noport-", "info-")) and k not in SEND_CAUSES
            and k not in VALIDITY)


def loss_growth(grew):
    """A loss row's growth with the short sends an ARP miss explains taken
    out: a send refused while a next hop's first ARP resolution is in flight
    is retried by Aeron, start-up rather than loss (each INVOKER client's
    embedded driver resolves anew, so it shows at every load point)."""
    if "short-sends" in grew and "dpdk-arp-miss" in grew:
        grew = dict(grew, **{"short-sends": max(0, grew["short-sends"] - grew["dpdk-arp-miss"])})
    return grew


def rep_lost(r, m, size, point, rep):
    """Whether a rep of a load point lost anything: a loss counter grew on a
    host (counts_as_loss), a loss read failed or is missing, or a message
    went unechoed."""
    hosts = ("a",) if m.split(":")[-1].startswith("local-") else ("a", "b")
    sent, received = r.load_echo.get((m, size, point, rep), (0, 0))
    if received < sent:
        return True
    for host in hosts:
        grew = r.loss_rep.get((f"{m}:{size}:{point}", rep, host))
        if grew is None or any(counts_as_loss(k, v) for k, v in loss_growth(grew).items()):
            return True
    return False


def loss_by_point(r):
    """Per mode and size, each load point's reps that lost anything (rep_lost)
    of the reps run, a failed rep (no echo) counted as lost, and the NAKs
    both hosts sent over them."""
    points = sorted(set(r.load) | set(r.load_failed), key=lambda p: (key(p[0]), p[1], point_rank(p[2])))
    if not points:
        return
    cols = sorted({p[2] for p in points}, key=point_rank)
    head = {c: "bursts" if c == "burst" else (f"{int(c[1:]):,}/s" if c.startswith("r") else f"{c}%") for c in cols}
    print("\n## Loss by load point (reps that lost anything, of the reps run; NAKs sent)\n")
    print("A rep lost something when a loss counter grew on either host (short sends included, those an ARP miss "
          "explains aside; `sat-`, `noport-`, the run-validity counters and the send-side causes on their own aside), a message went unechoed, or a "
          "loss read failed or was not taken, as `loss-free` counts it. In brackets, the NAKs both hosts sent over "
          "those reps. Compare variants on these counts: one rare event moves the strict `loss-free` rate by whole "
          "ladder steps.\n")
    rows = []
    for m, size in sorted({(p[0], p[1]) for p in points}, key=lambda x: (key(x[0]), x[1])):
        row = [name(m), f"{size} B"]
        for c in cols:
            p = (m, size, c)
            reps = sorted(k[3] for k in r.load_rep if k[:3] == p)
            failed = r.load_failed.get(p, 0)
            if not reps and not failed:
                row.append("-")
                continue
            lost = sum(rep_lost(r, m, size, c, rep) for rep in reps) + failed
            naks = sum(v.get("naks-sent", 0) for (lab, _, _), v in r.loss_rep.items() if lab == f"{m}:{size}:{c}")
            row.append(f"{lost}/{len(reps) + failed}" + (f" ({naks:,})" if naks else ""))
        rows.append(row)
    print(table(["state, transport", "size"] + [head[c] for c in cols], rows))


def loss_free(r, m, size):
    """The highest ladder rate below the first point that lost anything in
    any rep: a loss counter that grew on a host, short sends included
    (`sat-` and `noport-` ones are not loss, nor VALIDITY's, nor SEND_CAUSES
    on their own), a loss read that failed or is missing, or a message not
    echoed. A rep that could not keep the offered rate is saturation, not
    loss: keeps_up."""
    def holds(p, reps):
        return all(x[7] >= x[6] for x in r.load[p]) and not any(rep_lost(r, m, size, p[2], rep) for rep in reps)

    return ladder_limit(r, m, size, holds, "lost")


def keeps_up(r, m, size):
    """The highest ladder rate below the first point where a rep achieved
    under 98% of its offered rate: the generator or the path fell behind
    (saturation), whether or not anything was lost."""
    return ladder_limit(r, m, size, lambda p, reps: all(x[10] >= 0.98 * x[9] for x in r.load[p]), "behind")


def sustainable(r, m, size):
    """The lower of loss_free and keeps_up."""
    limits = [loss_free(r, m, size), keeps_up(r, m, size)]
    if "no ladder" in limits:
        return "no ladder"
    rates = [int(v[:-2].replace(",", "")) if v.endswith("/s") else 0 for v in limits]
    return limits[rates.index(min(rates))] if min(rates) else "none"


def nic_irqs(r):
    """The note under the loss tables: the runs (loss labels and reps), of
    every mode, where isolated-nic-irqs grew on a host, with each host's
    growth, in results.csv's order, and those where a host's row lacks it (a
    failed read, or vm.sh could not find the NICs or their IRQs: unknown
    there, never none); None where no run has it (results from before it). A
    fall counts as growth: the NIC's IRQs were made anew mid-run (a device
    reset), which places them anew."""
    runs = {}
    for (label, rep, host), grew in r.loss_rep.items():
        runs.setdefault((label, rep), []).append((host, grew.get("isolated-nic-irqs")))
    if not any(v is not None for hosts in runs.values() for _, v in hosts):
        return None
    hit = [f"`{label}` rep {rep} ({', '.join(f'{h} {v:,}' for h, v in hosts if v)})"
           for (label, rep), hosts in runs.items() if any(v for _, v in hosts)]
    unread = [f"`{label}` rep {rep} ({', '.join(h for h, v in hosts if v is None)})"
              for (label, rep), hosts in runs.items() if any(v is None for _, v in hosts)]
    verdict = (f"{len(hit)} of {len(runs)} runs grew: " + "; ".join(hit) + "." if hit
               else f"none of {len(runs)} runs grew.")
    if unread:
        verdict += " Not read on a host, so unknown there: " + "; ".join(unread) + "."
    if any("@IRQ=" in label for label, _ in runs):
        verdict += (" Under a mode's @IRQ= its data NICs' queue IRQs count on CPUs other than the one it chose (vm.sh "
                    "pins them there; its driver log names it), and their other IRQs on CPUs other than the housekeeping one.")
    return ("Data NIC interrupts on CPUs other than the housekeeping one (`isolated-nic-irqs`: on each host, the growth "
            "over the run of the data NICs' IRQ counts, under DPDK those of the IRQs the kernel keeps on their PCI "
            "addresses, in those CPUs' columns of /proc/interrupts; a check on the run, not loss. vm.sh pins every IRQ "
            "to the housekeeping CPU before each driver starts, and the kernel places a NIC's IRQs anew whenever it "
            "brings the NIC up or resets it): " + verdict)


def main():
    if len(sys.argv) >= 2 and sys.argv[1] == "--ab":
        if len(sys.argv) != 5:
            sys.exit("usage: analyse.py --ab <mode A> <mode B> <results.csv>")
        ab(sys.argv[2], sys.argv[3], sys.argv[4])
        return
    if len(sys.argv) < 2:
        sys.exit("usage: analyse.py <results.csv> [<run dir>] | analyse.py --ab <mode A> <mode B> <results.csv>")
    r = read_results(sys.argv[1])
    rtt, rtt_lost, tput, tput_lost, rxhw = r.rtt, r.rtt_lost, r.tput, r.tput_lost, r.rxhw
    tputmax, load, load_failed, mix, loss = r.tputmax, r.load, r.load_failed, r.mix, r.loss

    modes = sorted(set(rtt) | set(tput), key=key)

    print("## UDP round trip (µs): between the two hosts, or both ends on one (localhost)\n")
    rows = []
    for m in modes:
        reps = rtt.get(m, [])
        if not reps:
            continue
        rows.append([name(m), str(len(reps))] + [cell([r[i] for r in reps], 1000, 2) for i in range(6)])
    print(table(["state, transport", "reps"] + RTT_COLUMNS, rows))
    samples = {m: [int(r[6]) for r in rtt[m]] for m in rtt}
    print("\nRound trips measured per rep: " + ", ".join(f"{name(m)} {min(s):,}–{max(s):,}" for m, s in samples.items()) + ".\n")
    lost_rtt = {m: v for m, v in rtt_lost.items() if any(v)}
    if lost_rtt:
        print("Round trips lost (no echo within 50 ms on an unreliable channel), per rep: "
              + ", ".join(f"{name(m)} {min(v):,}–{max(v):,} of {min(samples[m]) + min(v):,}" for m, v in lost_rtt.items()) + ".\n")

    above_network(r, modes)
    above_floor(r, modes)

    print("## UDP throughput, 32-byte messages (M msgs/s)\n")
    def dropped(m):
        v = tput_lost.get(m)
        if not v:
            return "-"
        rates = [lost / (received + lost) * 100 if received + lost else 0.0 for received, lost in v]
        return f"{sorted(lost for _, lost in v)[len(v) // 2]:,} ({min(rates):.3f}–{max(rates):.3f}%)"
    rows = [[name(m), str(len(tput[m])), cell(tput[m], 1e6, 2), dropped(m)] for m in modes if tput.get(m)]
    print(table(["state, transport", "reps", "msgs/s", "dropped (median, % range)"], rows))

    if tputmax:
        print("\n## Sustained one-way maximum by message size (M msgs/s)\n")
        print("`flat-out` is the most the publisher sustained with nothing held back: a ceiling, lossy when its "
              "loss counters grew. `loss-free` is the highest rate of the ladder below the first point that lost "
              "anything in any rep: a loss counter grew on either host, short sends included (those an ARP miss explains aside); a message went unechoed; or a loss "
              "read failed or was not taken. The transport's own send-side counters (a full TX ring, an ARP miss, no mbuf, a full "
              "kernel send buffer) do not count on their own: each send they refuse is also a short send, and a full ring that "
              "queues a vector refuses nothing. Nor does `isolated-nic-irqs`, the data NICs' interrupts on CPUs other than the "
              "housekeeping one: a check on the run (under the loss counters' tables), listed with the loss counters but not loss. "
              "`keeps up` is the highest rate below the first point where a rep achieved under 98% "
              "of the offered rate (saturation, not loss), and `sustainable` the lower of the two.\n")
        def max_loss(m, size):
            grew = {k: v for k, v in loss.get(f"{m}:{size}:max", {}).items() if v}
            if f"{m}:{size}:max" not in loss:
                return "not sampled"
            return ", ".join(f"{k} {v:,}" for k, v in sorted(grew.items())) or "none"
        rows = [[name(m), f"{size} B", str(len(v)), cell(v, 1e6, 2), max_loss(m, size), loss_free(r, m, size),
                 keeps_up(r, m, size), sustainable(r, m, size)]
                for (m, size), v in sorted(tputmax.items(), key=lambda x: (key(x[0][0]), x[0][1]))]
        print(table(["state, transport", "size", "reps", "flat-out msgs/s", "flat-out loss counters", "loss-free",
                     "keeps up", "sustainable"],
                    rows))

    flows(r)

    frames(r, sys.argv[1])

    if load:
        print("\n## Round trip under open-loop load (µs, from each message's intended send time)\n")
        print("`offered` is the scheduled rate; `achieved` the rate actually offered after warm-up. "
              "Below 98% of offered (marked *), the generator or the path could not keep the schedule, "
              "and the latencies include that queueing. `echoed` is completed/issued/scheduled; "
              "`late, censored` is how often the run fell more than one slot behind, and how many round "
              "trips the histogram's bound censored (a max at the bound beside a censored count is a floor, "
              "not the slowest round trip). Both are absent (`—`) at a point whose reps predate them. "
              "`max send lag` is the longest a measured message "
              "waited past its due time to be offered (median of the reps, range in brackets): how far the "
              "generator itself fell behind, which is already inside the latencies beside it. "
              "`loss counters` lists every loss counter that grew "
              "on either host (`sat-` ones are saturation, `noport-` ones frames to a port nobody listens on, "
              "such as an endpoint the previous point closed, and `isolated-nic-irqs` the data NICs' interrupts on CPUs "
              "other than the housekeeping one, a check on the run: none of them loss).\n")
        order = {"burst": 10**12}
        rows = []
        rank = lambda f: order.get(f, 0) or int(f.removeprefix("r"))
        points = sorted(set(load) | set(load_failed), key=lambda p: (key(p[0]), p[1], rank(p[2])))
        for (m, size, frac) in points:
            reps = load.get((m, size, frac), [])
            if not reps:
                rows.append([name(m), f"{size} B", "burst 1000" if frac == "burst" else (f"{int(frac[1:]):,}/s" if frac.startswith("r") else f"{frac}%"), "", "",
                             f"0 (+{load_failed[(m, size, frac)]} failed)"] + ["no echo"] * 6 + ["", "", "", "", ""])
                continue
            echoed, work = work_columns(reps)
            lost = loss.get(f"{m}:{size}:{frac}", {})
            bad = {k: v for k, v in lost.items() if v}
            load_name = "burst 1000" if frac == "burst" else (f"{int(frac[1:]):,}/s" if frac.startswith("r") else f"{frac}%")
            offered = statistics.median(r[9] for r in reps)
            achieved = statistics.median(r[10] for r in reps)
            limited = "*" if achieved < 0.98 * offered else ""
            failed = load_failed.get((m, size, frac), 0)
            rows.append([name(m), f"{size} B", load_name, f"{offered:,.0f}", f"{achieved:,.0f}{limited}",
                         str(len(reps)) + (f" (+{failed} failed)" if failed else "")]
                        + [cell([r[i] for r in reps], 1000, 2) for i in range(6)]
                        + [echoed, work, f"{sum(r[8] for r in reps):,}",
                           cell([r[11] for r in reps], 1000, 2),
                           ", ".join(f"{k} {v:,}" for k, v in sorted(bad.items())) or "none"])
        print(table(["state, transport", "size", "load", "offered", "achieved", "reps"] + RTT_COLUMNS
                    + ["echoed", "late, censored", "back-pressured", "max send lag", "loss counters"], rows))
        merged_load(r, sys.argv[1])
        loss_by_point(r)

    if mix:
        print("\n## Mixed streams at once, open loop (µs, from each message's intended send time)\n")
        print("Every stream of a profile runs at the same time on one pair of channels, each on its own "
              "schedule; one row per stream. `achieved` below 98% of `offered` is marked *. "
              "`echoed` is completed/issued/scheduled and `late, censored` as in the load table, "
              "each stream's own schedule.\n")
        rows = []
        for (m, profile, stream, size, rate, burst), reps in sorted(mix.items(), key=lambda x: (key(x[0][0]), x[0][1], x[0][2])):
            achieved = statistics.median(r[9] for r in reps)
            lost = {k: v for k, v in loss.get(f"{m}:mix:{profile}", {}).items() if v}
            echoed, work = work_columns(reps)
            rows.append([name(m), profile, stream, f"{size} B", f"{rate:,}" + (f" ×{burst}" if burst > 1 else ""),
                         f"{achieved:,.0f}" + ("*" if achieved < 0.98 * rate else ""), str(len(reps))]
                        + [cell([r[i] for r in reps], 1000, 2) for i in range(6)]
                        + [echoed, work,
                           ", ".join(f"{k} {v:,}" for k, v in sorted(lost.items())) or "none"])
        print(table(["state, transport", "profile", "stream", "size", "offered/s", "achieved", "reps"] + RTT_COLUMNS
                    + ["echoed", "late, censored", "loss counters (profile)"], rows))

    spikes(os.path.dirname(os.path.abspath(sys.argv[1])))

    if loss:
        flagged = {m: {k: v for k, v in ks.items() if v} for m, ks in loss.items() if ":" not in m.split(":", 1)[-1]}
        flagged = {m: ks for m, ks in flagged.items() if ks}
        print("\n## Loss counters over the round-trip and throughput runs (both hosts, all reps; none = every one 0; "
              "`sat-` counters are saturation, `noport-` ones frames to a port nobody listens on and `isolated-nic-irqs` "
              "the data NICs' interrupts on CPUs other than the housekeeping one, a check on the run: none of them loss; "
              "localhost base runs are not sampled)\n")
        rows = [[name(m), ", ".join(f"{k} {v:,}" for k, v in sorted(flagged.get(m, {}).items())) or "none"]
                for m in modes if m in loss]
        print(table(["state, transport", "grew"], rows))
        # the idle round trip alone (lab.sh snapshots between it and the throughput run)
        alone = {m[:-len(":rtt")]: {k: v for k, v in ks.items() if v} for m, ks in loss.items() if m.endswith(":rtt")}
        if alone:
            print("\n## Loss counters over the idle round trip alone (both hosts, all reps; none = every one 0)\n")
            rows = [[name(m), ", ".join(f"{k} {v:,}" for k, v in sorted(alone[m].items())) or "none"]
                    for m in modes if m in alone]
            print(table(["state, transport", "grew"], rows))
        note = nic_irqs(r)
        if note:
            print("\n" + note)

    aeron_table(r)

    host_checks(r)

    if rxhw:
        print("\n## NIC arrival to the receive burst, receiver port (µs)\n")
        def stat(runs, i):
            values = [r[i] for r in runs if r[i] is not None]
            return cell(values, 1000, 2) if values else "–"

        def skewed(runs):
            counted = [r[3] for r in runs if r[3] is not None]
            return f"{sum(counted):,.0f}" if counted else "–"

        rows = [[name(m), str(len(rxhw[m])), stat(rxhw[m], 2), stat(rxhw[m], 0), stat(rxhw[m], 1), skewed(rxhw[m])]
                for m in sorted(rxhw, key=key)]
        print(table(["state, transport", "host-runs", "min", "mean", "max", "skewed frames (PHC/host clock)"], rows))


def host_checks(r):
    """The media driver's start-up host checks per mode and host (hostcheck
    rows, lib.sh hostcheck_rows): each line a host's driver printed, its
    severity, in how many of the mode's reps; warnings first."""
    if not r.hostcheck:
        return
    print("\n## Media driver host checks at start-up (per mode and host)\n")
    print("What each host's media driver said of the host as it started (its `ergon-md-dpdk warning:` and `info:` "
          "lines: ConfiguredDriver::from_env, md-dpdk/src/checks.rs; under INVOKER each client's embedded driver's), "
          "in how many of the mode's reps; `none`: it said nothing; `unknown`: its checks could not be read.\n")
    rank = {"warning": 0, "info": 1, "none": 2, "unknown": 3}
    rows = []
    for m in sorted(r.hostcheck, key=key):
        reps = {rep for rep, _, _, _ in r.hostcheck[m]}
        seen = defaultdict(set)
        for rep, host, severity, text in r.hostcheck[m]:
            seen[(host, severity, text)].add(rep)
        for (host, severity, text), in_reps in sorted(seen.items(), key=lambda x: (x[0][0], rank.get(x[0][1], 3), x[0][2])):
            rows.append([name(m), host, severity, text.replace("|", "\\|"), f"{len(in_reps)} of {len(reps)}"])
    print(table(["state, transport", "host", "severity", "finding", "reps"], rows))


# The load client's percentiles (HdrHistogram's value_at_quantile), as the
# load table's columns name them, then the max
HIST_QUANTILES = (0.5, 0.9, 0.99, 0.999, 0.9999)


def histogram_counts(path):
    """A histogram file the clients write (value_ns,count rows, each value
    its bucket's highest equivalent) as {value: count}; None when it is
    missing, empty or holds no rows."""
    try:
        with open(path, newline="") as f:
            rows = list(csv.reader(f))
    except OSError:
        return None
    counts = {}
    for row in rows:
        if len(row) == 2 and row[0].isdigit() and row[1].isdigit():
            counts[int(row[0])] = counts.get(int(row[0]), 0) + int(row[1])
    return counts or None


def merged_quantiles(counts):
    """HdrHistogram's percentiles of merged counts (its value_at_quantile:
    the first value whose running count reaches ceil(q x samples), at least
    1) for HIST_QUANTILES, then the max; and the samples."""
    total = sum(counts.values())
    values = sorted(counts)
    out = []
    for q in HIST_QUANTILES:
        want, running = max(1, math.ceil(q * total)), 0
        for v in values:
            running += counts[v]
            if running >= want:
                out.append(v)
                break
    return out + [values[-1]], total


def merged_load(r, path):
    """Each load point's histograms (lib.sh fetches one after every point:
    histogram-<state>-<mode>-<size>-<rep>-<point>.csv) merged over its reps,
    and the pooled percentiles: beside the medians of reps, a tail of every
    sample. A rep without its histogram (missing or empty) is left out, said
    so; nothing where no point has one (runs from before they were fetched)."""
    dirs = run_dirs(path)
    rows = []
    rank = lambda p: 10**12 if p == "burst" else int(p.removeprefix("r"))
    for m, size, point in sorted(set(r.load) | set(r.load_failed), key=lambda p: (key(p[0]), p[1], rank(p[2]))):
        reps = sorted({k[3] for k in r.load_rep if k[:3] == (m, size, point)}, key=rep_key)
        merged, used = {}, 0
        for rep in reps:
            fname = f"histogram-{file_name(m)}-{size}-{rep}-{point}.csv"
            counts = next((c for c in (histogram_counts(os.path.join(d, fname)) for d in dirs) if c), None)
            if counts:
                used += 1
                for v, c in counts.items():
                    merged[v] = merged.get(v, 0) + c
        if not used:
            continue
        values, total = merged_quantiles(merged)
        load_name = "burst 1000" if point == "burst" else (f"{int(point[1:]):,}/s" if point.startswith("r") else f"{point}%")
        rows.append([name(m), f"{size} B", load_name, str(used) if used == len(reps) else f"{used} of {len(reps)}",
                     f"{total:,}"] + [f"{v / 1000:.2f}" for v in values])
    if not rows:
        return
    print("\n## Round trip under open-loop load, every rep's histogram merged (µs)\n")
    print("Each load point's reps pooled into one histogram (the load client's after its warm-up second, lib.sh "
          "fetching it after every point) and its percentiles as HdrHistogram takes them: unlike the medians of reps "
          "above, a tail here counts every rep's samples. A rep whose histogram is missing or empty is left out, "
          "said so.\n")
    print(table(["state, transport", "size", "load", "reps merged", "samples"] + RTT_COLUMNS, rows))


# Aeron's telemetry (lib.sh's aeron rows): (key, column, how reps combine)
AERON_COLUMNS = (("conductor-max-cycle-ns", "conductor max µs", max), ("sender-max-cycle-ns", "sender max µs", max),
                 ("receiver-max-cycle-ns", "receiver max µs", max), ("conductor-cycles-over", "conductor over", sum),
                 ("sender-cycles-over", "sender over", sum), ("receiver-cycles-over", "receiver over", sum),
                 ("sender-fc-limits", "back-pressure events", sum), ("sms-sent", "status messages sent", sum),
                 ("sms-received", "status messages received", sum), ("fc-under-runs", "flow control under runs", sum),
                 ("fc-over-runs", "flow control over runs", sum))


def run_of(label):
    """A loss or aeron row's label as (<state>:<mode>, the run it covers, its
    rank among the mode's runs)."""
    parts = label.split(":")
    mode, rest = ":".join(parts[:2]), parts[2:]
    if not rest:
        return mode, "round trip and throughput", (1,)
    if rest == ["rtt"]:
        return mode, "idle round trip", (0,)
    if rest[0] == "mix":
        return mode, f"mix {':'.join(rest[1:])}", (3, ":".join(rest[1:]))
    if len(rest) == 2 and rest[0].isdigit():
        size, point = int(rest[0]), rest[1]
        if point == "max":
            return mode, f"{size} B sustained maximum", (2, size, -2)
        if point.startswith("flows"):
            return mode, f"{size} B, {point.removeprefix('flows')} flows at once", (2, size, -1)
        return mode, point_name(size, point), (2, size, point_rank(point))
    return mode, ":".join(rest), (4, ":".join(rest))


def duration(ns):
    """A threshold as Aeron's labels give it, in the largest whole unit."""
    for unit, scale in (("s", 10**9), ("ms", 10**6), ("µs", 10**3)):
        if ns >= scale and ns % scale == 0:
            return f"{ns // scale:,} {unit}"
    return f"{ns:,} ns"


def aeron_table(r):
    """Aeron's own telemetry by mode, run and host (lib.sh's aeron rows),
    as information: never loss."""
    if not r.aeron:
        return
    rows = []
    for (label, host), runs in sorted(r.aeron.items(), key=lambda x: (key(run_of(x[0][0])[0]), run_of(x[0][0])[2], x[0][1])):
        mode, run, _ = run_of(label)
        cells = []
        for k, _, combine in AERON_COLUMNS:
            values = [kv[k] for _, kv in runs if k in kv]
            if not values:
                cells.append("-")
            elif combine is max:
                cells.append(f"{max(values) / 1000:.1f}")
            else:
                cells.append(f"{sum(values):,}")
        rows.append([name(mode), run, host, str(len({rep for rep, _ in runs}))] + cells)
    print("\n## Aeron's agents and flow control by run (information, never loss)\n")
    print("From Aeron's own counters on each host, read with the loss counters around every run (vm.sh loss). An "
          "agent's longest duty cycle counts from its driver's start (one driver a mode and rep), so a run's is the "
          "longest up to its end: the highest over reps here. Its cycles over the threshold vm.sh sets "
          "(AERON_DRIVER_{CONDUCTOR,SENDER,RECEIVER}_CYCLE_THRESHOLD), the sender's flow-control limits "
          "(back-pressure events), the status messages sent and received and the flow control under and over runs "
          "are each run's growth, summed over reps. - where a host's rows lack one. None of them is loss.\n")
    print(table(["state, transport", "run", "host", "reps"] + [c for _, c, _ in AERON_COLUMNS], rows))
    thresholds = []
    for agent in ("conductor", "sender", "receiver"):
        seen = sorted({kv[f"{agent}-threshold-ns"] for runs in r.aeron.values() for _, kv in runs
                       if f"{agent}-threshold-ns" in kv})
        if seen:
            thresholds.append(f"{agent} {' and '.join(duration(v) for v in seen)}")
    if thresholds:
        print(f"\nThresholds, from the counters' labels: {', '.join(thresholds)}.")


def above_network(r, modes):
    """Each cross-host mode's round trip less the network's own (netrtt: ping's
    minimum between the hosts in that state), per rep, where a state has it:
    what the transport and the driver add over the wire's round trip."""
    rows = []
    for m in modes:
        state, _, mode = m.rpartition(":")
        base = r.netrtt.get(state)
        if not base or not r.rtt.get(m) or mode.startswith("local-"):
            continue
        low = min(b[0] for b in base)
        net = f"{low / 1000:.2f}/{statistics.mean(b[1] for b in base) / 1000:.2f}/{max(b[2] for b in base) / 1000:.2f}"
        reps = r.rtt[m]
        rows.append([name(m), net, str(len(reps))] + [cell([x[i] - low for x in reps], 1000, 2) for i in (0, 2, 4, 5)])
    if not rows:
        return
    print("## UDP round trip above the network's own (µs)\n")
    print("The network's own round trip is ping's between the hosts' data NICs at the start of each state (vm.sh "
          "base-rtt, from host a's sender to host b's receiver: min/avg/max). Each mode's round trip less that "
          "minimum, per rep: the median across reps, the range in brackets; what the transport, the driver and the "
          "clients add over the wire's own round trip, whatever the distance.\n")
    print(table(["state, transport", "network min/avg/max", "reps", "p50", "p99", "p99.99", "max"], rows) + "\n")


def floor_of(mode):
    """The floor beneath a cross-host mode's transport: kernel-raw for the
    kernel's Aeron modes, dpdk-raw for DPDK's (-mono for -mono), the pure
    floor dpdk-pure for dpdk-raw itself; None for the other floors and
    localhost modes."""
    transport = mode.split("@")[0].removesuffix("+idle")
    if transport == "dpdk-raw":
        return "dpdk-pure"
    mono = "-mono" if transport.endswith("-mono") else ""
    transport = transport.removesuffix("-mono")
    if transport.endswith("-raw") or transport == "dpdk-pure" or mode.startswith("local-"):
        return None
    if transport.startswith("kernel"):
        return f"kernel-raw{mono}"
    return f"dpdk-raw{mono}" if transport.startswith("dpdk") else None


def above_floor(r, modes):
    """Each cross-host Aeron mode's round trip less its transport's raw floor
    (the floor client, no Aeron) in the same state, rep by rep where both ran:
    what Aeron's driver and client add over the NIC, PMD or kernel, host and
    fabric beneath them."""
    rows = []
    for m in modes:
        state, _, mode = m.rpartition(":")
        floor = floor_of(mode)
        base = f"{state}:{floor}" if state else floor
        if floor is None or not r.rtt.get(m) or not r.rtt.get(base):
            continue
        reps = sorted({rep for (label, rep) in r.rtt_rep if label == m} &
                      {rep for (label, rep) in r.rtt_rep if label == base}, key=rep_key)
        if not reps:
            continue
        diff = lambda i: cell([r.rtt_rep[(m, rep)][i] - r.rtt_rep[(base, rep)][i] for rep in reps], 1000, 2)
        rows.append([name(m), mode_name(floor), str(len(reps))] + [diff(i) for i in (0, 2, 4, 5)])
    if not rows:
        return
    print("## UDP round trip above the raw floor (µs)\n")
    print("The raw floor is the floor client's round trip in the same state (md-dpdk/harness/src/bin/floor.rs: rtt's "
          "pacing, warm-up, clock, CPUs and 64-byte datagrams, with no Aeron driver or client: kernel UDP sockets "
          "for the kernel's modes, the DPDK transport's bindings alone for DPDK's). Each Aeron mode's round trip less "
          "its floor's, rep by rep where both ran: the median across reps, the range in brackets; what Aeron's "
          "driver and client add. A max no higher than the floor's lies beneath Aeron. dpdk-raw is set against the "
          "pure floor (purefloor.c: DPDK alone, a 5-tuple-swapping reflector at the pong): what ergon-md-dpdk's "
          "packet path adds; a max no higher than the pure floor's lies beneath the transport.\n")
    print(table(["state, transport", "floor", "reps", "p50", "p99", "p99.99", "max"], rows) + "\n")


def one_flow(r, m, size, rep):
    """<m>'s one-flow sustained maximum at <size> in <rep>: the load sweep's,
    or at 32 B the flat-out throughput run's (the same run). None if not run."""
    v = r.tputmax_rep.get((m, size, rep))
    return r.tput_rep.get((m, rep)) if v is None and size == 32 else v


def flows(r):
    """Several flows' sustained maxima at once (lib.sh multi_flow_max), against
    one flow's in the same rep, with the runs' loss counters. Only runs whose
    flows overlapped at least MIN_OVERLAP count; the rest are listed."""
    if not r.tputflows and not r.tputflows_failed and not r.tputflows_apart:
        return
    print("\n## Sustained maximum over several flows at once (M msgs/s)\n")
    print("Several publications on host a at once, each to its own subscription on host b over a channel of its own, "
          "so on a UDP port of its own: each is a flow of its own as a cloud network counts flows (one 5-tuple). "
          "They start together and every client has an app CPU to itself; each host's one driver, its one sender "
          "and one receiver thread, carries them all. Starting together does not make their measured seconds "
          "overlap (a client started late, by an SSH retry say, has the path to itself for a while and raises the "
          "sum), so `overlap` checks it: the span every flow's measured window shares over the shortest window, "
          "each from its subscriber's wall clock (all on host b), the median across the runs counted and the range "
          f"in brackets. A run whose overlap is below {MIN_OVERLAP}, or unknown (a client that reports no window, or "
          "a row from before the overlap was recorded), counts in none of `aggregate`, `per flow` and `× one flow`: "
          "`left out` counts it, each one's overlap in brackets. `aggregate` sums the flows' "
          "means over their own measured windows -- the rate the flows carried at once, which a sum of per-flow "
          "medians is not -- and `per flow` is each flow's mean. `× one flow` is the aggregate over the same mode's one-flow "
          "sustained maximum at that size in the same rep (at 32 B, the flat-out throughput run): the median of those "
          "ratios, their range in brackets, and how many reps paired where not all did. Near 1, every flow shares "
          "one ceiling (the NIC's or the instance's, or the drivers' threads'); near the number of flows, the ceiling "
          "is per flow, up to the instance's own bandwidth. `loss counters` lists every loss counter that grew on "
          "either host in any run, left out or not (`sat-` and `noport-` ones are not loss, nor is `isolated-nic-irqs`, "
          "a check on the run under the loss counters' tables; short sends are); the "
          "transport's send-side counters, whose refusals show as short sends, are listed apart.\n")
    rows = []
    for m, size, n in sorted(set(r.tputflows) | set(r.tputflows_failed) | set(r.tputflows_apart),
                             key=lambda x: (key(x[0]), x[1], x[2])):
        runs = r.tputflows.get((m, size, n), [])
        failed = r.tputflows_failed.get((m, size, n), 0)
        reps = str(len(runs)) + (f" (+{failed} failed)" if failed else "")
        apart = sorted(r.tputflows_apart.get((m, size, n), []), key=lambda x: rep_key(x[0]))
        left_out = f"{len(apart)} ({', '.join(overlap_name(o) for _, o in apart)})" if apart else "none"
        # paired by rep: a rep without its one-flow run is left out, and said so
        ratios = [total / one for rep, total, _, _ in runs if (one := one_flow(r, m, size, rep))]
        ratio = cell(ratios, 1, 2) if ratios else "-"
        if 0 < len(ratios) < len(runs):
            ratio += f", {len(ratios)} of {len(runs)} reps"
        label = f"{m}:{size}:flows{n}"
        grew = {k: v for k, v in r.loss.get(label, {}).items()
                if v and not k.startswith(("sat-", "noport-", "info-")) and k not in VALIDITY}
        lost = ", ".join(f"{k} {v:,}" for k, v in sorted(grew.items()) if k not in SEND_CAUSES) or "none"
        causes = ", ".join(f"{k} {v:,}" for k, v in sorted(grew.items()) if k in SEND_CAUSES) or "none"
        if label not in r.loss:
            lost = causes = "not sampled"
        rows.append([name(m), f"{size} B", str(n), reps,
                     cell([o for _, _, _, o in runs], 1, 3) if runs else "-", left_out,
                     cell([total for _, total, _, _ in runs], 1e6, 2) if runs else "-",
                     " / ".join(f"{statistics.median(f[i] for _, _, f, _ in runs) / 1e6:.2f}" for i in range(n))
                     if runs else "-",
                     ratio, lost, causes])
    print(table(["state, transport", "size", "flows", "reps", "overlap", "left out", "aggregate msgs/s", "per flow",
                 "× one flow", "loss counters", "send-side causes"], rows))


# the DPDK transport's counters a frames table reads, by how their labels end
TRANSPORT = {"dir=tx pkts": "tx-pkts", "dir=rx pkts": "rx-pkts", "dir=tx bytes": "tx-bytes",
             "dir=tx tx-queued": "tx-queued", "dir=tx tx-eagain": "tx-eagain"}


def counters_snapshot(path):
    """One host's vm.sh counters (lib.sh snap), summed over its ports: the DPDK
    transport's TRANSPORT counters, or in a kernel mode the data NICs' packets
    (nic-tx, nic-rx). A counter the snapshot lacks is absent. Under INVOKER the
    snapshot names the embedded driver whose CnC it read ("cnc <pid>-<start>"):
    as "_cnc"."""
    out = {}
    for line in open(path, errors="replace"):
        if line.startswith("#"):
            continue
        words = line.split()
        if len(words) == 2 and words[0] == "cnc":
            out["_cnc"] = words[1]
            continue
        if words and words[0] in ("sender", "receiver"):
            for k, v in re.findall(r"(\w+): (\d+)", line):
                if k in ("tx_packets", "rx_packets"):
                    out[f"nic-{k[:2]}"] = out.get(f"nic-{k[:2]}", 0) + int(v)
            continue
        for entry in line.split(";"):
            f = entry.split()
            if len(f) >= 5 and f[0].isdigit() and f[2] == "rusteron-dpdk" and " ".join(f[-2:]) in TRANSPORT:
                k = TRANSPORT[" ".join(f[-2:])]
                out[k] = out.get(k, 0) + int(f[0])
    return out


def grew(before, after, k):
    """A counter's growth between two snapshots; a counter that appeared in
    between was registered at zero. None when the later one lacks it."""
    if after is None or k not in after:
        return None
    return after[k] - (before or {}).get(k, 0)


def frames(r, path):
    """Wire frames per message in the flat-out runs, from the counters lib.sh
    saved on both hosts just before and after each (32 B throughput, and each
    size's sustained maximum, one flow and several at once)."""
    labels = {file_name(l): l for l in set(r.tput) | {m for m, _ in r.tputmax} | {k[0] for k in r.tputflows_rep}
              | {k[0] for k in r.tputflows_failed} | {k[0] for k in r.tputflows_apart} | set(r.rtt)}
    runs = []
    for d in run_dirs(path):
        seconds = 5  # lab.sh's default TPUT_SECONDS
        env = os.path.join(d, "env")
        if os.path.exists(env):
            m = re.search(r"^tput_seconds=(\d+)", open(env).read(), re.M)
            seconds = int(m.group(1)) if m else seconds
        for rank, phase in enumerate(("tput", "max", "flows")):
            prefix = f"a-counters-{phase}-before-"
            for before in glob.glob(os.path.join(d, prefix + "*.log")):
                rest = os.path.basename(before)[len(prefix):-len(".log")]
                m = re.match(r"^(.*)-(\d+)$" if phase == "tput" else r"^(.*)-(\d+)-(\d+)$", rest)
                if not m:
                    continue
                label = labels.get(m.group(1), m.group(1))
                # a localhost run's frames go through the kernel's loopback,
                # which neither the transport's nor the NICs' counters see
                if label.rpartition(":")[2].startswith("local-"):
                    continue
                size, rep = (32, int(m.group(2))) if phase == "tput" else (int(m.group(2)), int(m.group(3)))
                run = {"tput": "32 B flat out", "max": f"{size} B sustained maximum"}.get(phase)
                snaps = {}
                for node in "ab":
                    for when in ("before", "after"):
                        p = os.path.join(d, f"{node}-counters-{phase}-{when}-{rest}.log")
                        snaps[node, when] = counters_snapshot(p) if os.path.exists(p) else None
                msgs = r.tput_rep.get((label, rep)) if phase == "tput" else r.tputmax_rep.get((label, size, rep))
                if phase == "flows":
                    # the flows' sum; how many from the run's row, a failed run's or a missing one's
                    # unknown; a run left out for its flows' overlap named, with no sum
                    msgs, run = next(((v, f"{size} B, {k[2]} flows at once") for k, v in r.tputflows_rep.items()
                                      if k[0] == label and k[1] == size and k[3] == rep),
                                     next(((None, f"{size} B, {k[2]} flows at once, left out (overlap "
                                                  f"{overlap_name(o)})")
                                           for k, apart in r.tputflows_apart.items() if k[0] == label and k[1] == size
                                           for run_rep, o in apart if run_rep == rep),
                                          (None, f"{size} B, several flows at once")))
                runs.append((key(label), rank, size, rep, label, msgs, seconds + 1, snaps, run))
    if not runs:
        return
    print("\n## Throughput phase frames\n")
    print("From vm.sh counters on both hosts just before and after each flat-out run. Frames are host a's sent "
          "(the transport's `pkts` over both ports; the data NICs' `tx_packets` in kernel modes) and host b's "
          "received, as rates over the subscriber's warm-up second and samples (`tput_seconds` + 1 s), the only "
          "time the publisher has a receiver window. Messages per frame divides the run's msgs/s (over several "
          "flows, their aggregate) by the frames sent, so frames sent twice lower it. `tx-queued` (frames a full "
          "ring refused and the transport queued to go first) and `tx-eagain` (frames refused back to Aeron) are "
          "host a's growth over the run.\n")
    rows = []
    fmt = lambda v, f: "-" if v is None else format(v, f)
    for _, _, size, rep, label, msgs, flow, s, run in sorted(runs, key=lambda x: x[:4]):
        a0, a1, b0, b1 = s["a", "before"], s["a", "after"], s["b", "before"], s["b", "after"]
        # INVOKER: a snapshot of another embedded driver (the last client's,
        # before this run's started) counts as zero: this one's started there
        a0 = {} if a0 and a1 and a0.get("_cnc") != a1.get("_cnc") else a0
        b0 = {} if b0 and b1 and b0.get("_cnc") != b1.get("_cnc") else b0
        sent = grew(a0, a1, "tx-pkts")
        sent = grew(a0, a1, "nic-tx") if sent is None else sent
        received = grew(b0, b1, "rx-pkts")
        received = grew(b0, b1, "nic-rx") if received is None else received
        sent_bytes = grew(a0, a1, "tx-bytes")
        rows.append([name(label), run, str(rep),
                     fmt(msgs and msgs / 1e6, ".2f"), fmt(sent and sent / flow / 1e6, ".3f"),
                     fmt(msgs / (sent / flow) if msgs and sent else None, ".1f"),
                     fmt(sent_bytes / sent if sent_bytes is not None and sent else None, ",.0f"),
                     fmt(received and received / flow / 1e6, ".3f"),
                     fmt(sent - received if sent is not None and received is not None else None, ","),
                     fmt(grew(a0, a1, "tx-queued"), ","), fmt(grew(a0, a1, "tx-eagain"), ",")])
    print(table(["state, transport", "run", "rep", "msgs/s (M)", "frames/s sent by a (M)", "msgs per frame",
                 "bytes per frame", "frames/s received by b (M)", "sent − received", "tx-queued", "tx-eagain"], rows))


def fisher_exact(a, b, c, d):
    """Fisher's exact test, two-sided, of the 2x2 table [[a, b], [c, d]]: with
    its margins fixed, the probability of a table no more likely than this one."""
    n1, n2, k = a + b, c + d, a + c
    total = math.comb(n1 + n2, k)
    p = lambda x: math.comb(n1, x) * math.comb(n2, k - x) / total
    observed = p(a)
    return min(1.0, sum(p(x) for x in range(max(0, k - n2), min(k, n1) + 1) if p(x) <= observed * (1 + 1e-9)))


def point_name(size, point, burst=None):
    """A load point as the A/B names it (a burst's size from its load rows)."""
    if point == "burst":
        return f"{size} B, bursts of {burst:,}" if burst else f"{size} B, bursts"
    if point.startswith("r"):
        return f"{size} B at {int(point[1:]):,}/s"
    return f"{size} B at {point}% of the sustained maximum"


def point_rank(point):
    return 10**12 if point == "burst" else int(point.removeprefix("r"))


def ab(arg_a, arg_b, path):
    """analyse.py --ab: mode A against mode B, within each state that has both."""
    r = read_results(path)
    labels = sorted(set(r.rtt) | set(r.tput) | {m for m, _ in r.tputmax} | {k[0] for k in r.tputflows_rep}
                    | {p[0] for p in r.load}, key=key)
    match = lambda arg: [l for l in labels if l == arg or l.rpartition(":")[2] == arg]
    found_a, found_b = match(arg_a), match(arg_b)
    pairs = [(a, b) for a in found_a for b in found_b
             if len(found_a) == len(found_b) == 1 or a.rpartition(":")[0] == b.rpartition(":")[0]]
    if not pairs:
        missing = " and ".join(a for a, f in ((arg_a, found_a), (arg_b, found_b)) if not f) or f"{arg_a} and {arg_b} in one state"
        sys.exit(f"analyse.py --ab: no runs of {missing} in {path}; the modes there: {', '.join(labels)}")
    dirs = run_dirs(path)
    for i, (la, lb) in enumerate(pairs):
        print(("\n" if i else "") + f"## A/B: A = `{la}`, B = `{lb}`\n")
        print("Each cell is the median across reps, the range in brackets. A verdict names the better side only "
              "where the two ranges do not overlap. NAKs are those either host sent during the run; the 32 B "
              "flat-out run's are the round trip and throughput snapshot's less the idle round trip's.\n")
        ab_pair(r, la, lb, dirs)


def ab_pair(r, la, lb, dirs):
    rows = []

    def metric(title, va, vb, scale, digits, higher=False):
        if not va and not vb:
            return
        if not va or not vb:
            verdict = "missing on one side"
        elif max(va) < min(vb) or max(vb) < min(va):
            lower = "A" if max(va) < min(vb) else "B"
            verdict = f"{lower if not higher else 'AB'.replace(lower, '')} better, ranges apart"
        else:
            verdict = "ranges overlap"
        rows.append([title, str(len(va)), cell(va, scale, digits) if va else "-",
                     str(len(vb)), cell(vb, scale, digits) if vb else "-", verdict])

    by_rep = lambda d, prefix: [v for k, v in sorted(d.items(), key=lambda kv: rep_key(kv[0][-1])) if k[:-1] == prefix]
    for i, title in ((0, "p50"), (2, "p99"), (4, "p99.99"), (5, "max")):
        metric(f"idle round trip {title} (µs)", [v[i] for v in by_rep(r.rtt_rep, (la,))],
               [v[i] for v in by_rep(r.rtt_rep, (lb,))], 1000, 2)
    metric("32 B flat out (M msgs/s)", by_rep(r.tput_rep, (la,)), by_rep(r.tput_rep, (lb,)), 1e6, 2, higher=True)
    sizes = sorted({k[1] for k in r.tputmax_rep if k[0] in (la, lb)}
                   | {k[1] for k in r.tputflows_rep if k[0] in (la, lb)})
    for size in sizes:
        metric(f"{size} B sustained maximum (M msgs/s)", by_rep(r.tputmax_rep, (la, size)),
               by_rep(r.tputmax_rep, (lb, size)), 1e6, 2, higher=True)
        for n in sorted({k[2] for k in r.tputflows_rep if k[0] in (la, lb) and k[1] == size}):
            metric(f"{size} B, {n} flows at once (M msgs/s)", by_rep(r.tputflows_rep, (la, size, n)),
                   by_rep(r.tputflows_rep, (lb, size, n)), 1e6, 2, higher=True)
    points = sorted({k[1:3] for k in r.load_rep if k[0] in (la, lb)}, key=lambda p: (p[0], point_rank(p[1])))
    for size, point in points:
        title = point_name(size, point, r.burst.get((la, size, point), r.burst.get((lb, size, point))))
        for i, pct in ((4, "p99.99"), (5, "max")):
            metric(f"{title}, {pct} (µs)", [v[i] for v in by_rep(r.load_rep, (la, size, point))],
                   [v[i] for v in by_rep(r.load_rep, (lb, size, point))], 1000, 2)

    # NAKs sent by either host, per run: one value per rep of each run that
    # has loss rows (the idle round trip's own, the throughput run's as the
    # snapshot spanning both less the round trip's, each size's sustained
    # maximum, each load point, each mix profile)
    def naks(loss_label, rep):
        """Either host's NAKs over one run; None when no host has its row, or
        one's lacks the count (a failed read: invalid=1)."""
        rows_ = [x for x in (r.loss_rep.get((loss_label, rep, h)) for h in "ab") if x is not None]
        if not rows_ or any("naks-sent" not in x for x in rows_):
            return None
        return sum(x["naks-sent"] for x in rows_)

    def units(label):
        reps = sorted({k[1] for k in r.loss_rep if k[0] == label or k[0].startswith(label + ":")}, key=rep_key)
        out = []
        rtt_alone = [naks(f"{label}:rtt", rep) for rep in reps]
        both = [naks(label, rep) for rep in reps]
        if any(v is not None for v in rtt_alone):
            out.append(("idle round trip", [v for v in rtt_alone if v is not None]))
            out.append(("32 B flat out", [t - a for t, a in zip(both, rtt_alone) if t is not None and a is not None]))
        else:
            out.append(("round trip and 32 B flat out", [v for v in both if v is not None]))
        suffixes = {k[0][len(label) + 1:] for k in r.loss_rep if k[0].startswith(label + ":")} - {"rtt"}
        # each size's sustained maximum, its several flows at once, then its load points
        sized = sorted((s.split(":") for s in suffixes if re.match(r"^\d+:", s)),
                       key=lambda p: (int(p[0]), -2 if p[1] == "max" else -1 if p[1].startswith("flows")
                                      else point_rank(p[1])))
        for size, point in sized:
            title = (f"{size} B sustained maximum" if point == "max"
                     else f"{size} B, {point.removeprefix('flows')} flows at once" if point.startswith("flows")
                     else point_name(int(size), point, r.burst.get((label, int(size), point))))
            out.append((title, [v for v in (naks(f"{label}:{size}:{point}", rep) for rep in reps) if v is not None]))
        for s in sorted(x for x in suffixes if x.startswith("mix:")):
            out.append((f"mix {s[4:]}", [v for v in (naks(f"{label}:{s}", rep) for rep in reps) if v is not None]))
        return out

    ua, ub = dict(units(la)), dict(units(lb))
    for title in list(ua) + [t for t in ub if t not in ua]:
        metric(f"NAKs, {title}", ua.get(title, []), ub.get(title, []), 1, 0)
    print(table(["metric", "A reps", "A", "B reps", "B", "verdict"], rows))
    with_a = sum(v > 0 for vs in ua.values() for v in vs)
    with_b = sum(v > 0 for vs in ub.values() for v in vs)
    runs_a, runs_b = sum(map(len, ua.values())), sum(map(len, ub.values()))
    if runs_a and runs_b:
        print(f"\nRuns with NAKs, every run above pooled: A {with_a} of {runs_a}, B {with_b} of {runs_b}; "
              f"Fisher's exact test p = {fisher_exact(with_a, runs_a - with_a, with_b, runs_b - with_b):.3f} (two-sided).")
    # several flows' rates count only where the flows overlapped (flows()); never dropped unsaid
    apart = [f"{side}, {size} B, {n} flows, rep {rep} ({overlap_name(o)})"
             for side, label in (("A", la), ("B", lb))
             for (m, size, n), runs in sorted(r.tputflows_apart.items(), key=lambda x: x[0][1:]) if m == label
             for rep, o in sorted(runs, key=lambda x: rep_key(x[0]))]
    if apart:
        print(f"\nLeft out of the rates over several flows at once, their flows' measured windows overlapping less "
              f"than {MIN_OVERLAP} of the shortest or by an amount not recorded: {'; '.join(apart)}.")

    reps = sorted({k[1] for k in r.rtt_rep if k[0] in (la, lb)}, key=rep_key)
    gaps = {(l, rep): agent_gaps(dirs, f"{file_name(l)}-{rep}") for l in (la, lb) for rep in reps}
    if any(g is not None for hosts in gaps.values() for g in hosts.values()):
        def side(hosts):
            if all(g is None for g in hosts.values()):
                return "n/a"
            return ", ".join(f"{h} {'/'.join(map(str, g)) if g is not None else 'n/a'}" for h, g in sorted(hosts.items()))
        print("\n### Agent poll-cycle gaps during the idle round trip, per rep (20–50/50–100/over 100 µs, both ports)\n")
        print(table(["rep", "A", "B"], [[str(rep), side(gaps[la, rep]), side(gaps[lb, rep])] for rep in reps]))
    bursts = {(l, rep): pmd_bursts(dirs, f"{file_name(l)}-{rep}") for l in (la, lb) for rep in reps}
    if any(b is not None for hosts in bursts.values() for b in hosts.values()):
        def burst_side(hosts):
            def one(b):
                if b is None:
                    return "n/a"
                f = ["n/a" if v is None else str(v) for v in b]
                return f"rx {'/'.join(f[:3])}, tx {'/'.join(f[3:])}"
            if all(b is None for b in hosts.values()):
                return "n/a"
            return "; ".join(f"{h} {one(b)}" for h, b in sorted(hosts.items()))
        print("\n### PMD burst timing during the idle round trip, per rep "
              "(RUSTERON_DPDK_GAP_ATTRIB=1: max ns / ≥1 µs / ≥5 µs)\n")
        print(table(["rep", "A", "B"], [[str(rep), burst_side(bursts[la, rep]), burst_side(bursts[lb, rep])] for rep in reps]))


def spikes(folder):
    """Per run (reps pooled): round trips of 100 µs or more, and the share of them
    in the busiest 10 ms of the second (by intended send time). Spread at random
    that share is about 1%; work that runs once a second pushes it towards 100%."""
    groups = defaultdict(lambda: [0, [0] * 100, 0])
    for path, stem in spike_files(folder):
        # <state>-<mode>-rtt-<rep>, <state>-<mode>-mix-<profile>-<rep>, <state>-<mode>-<size>-<rep>-<point>
        m = re.match(r"^(.*-(?:rtt|mix-.*))-\d+$", stem) or re.match(r"^(.*-\d+)-\d+(-[^-]+)$", stem)
        run = "".join(m.groups()) if m else stem
        g = groups[run]
        g[2] += 1
        with open_spikes(path) as f:
            for row in csv.reader(f):
                if len(row) in (3, 4) and row[1].isdigit():
                    g[0] += 1
                    g[1][int(row[1]) % 1_000_000_000 // 10_000_000] += 1
    if not groups:
        return
    print("\n## Round trips of 100 µs or more, by run (reps pooled)\n")
    rows = []
    for run, (count, phase, files) in sorted(groups.items()):
        share = f"{100 * max(phase) / count:.0f}%" if count >= 20 else "too few"
        rows.append([run, str(files), f"{count:,}", share])
    print(table(["run", "reps", "spikes", "in busiest 10 ms of the second"], rows))


if __name__ == "__main__":
    main()
