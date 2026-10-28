#!/usr/bin/env python3
"""A transport.sh session directory laid out as transport-format.md has it,
for fairness.py, spikes.py --transport and analyse.py --transport.

transport_collect.py <session dir>

transport.sh run calls it as the session ends (and it may be run again on a
fetched copy). From what the runner kept per point (manifest/<point>.env,
the binaries' --out directories under points/<point>/, the hosts' snapshots
under irq/, nstat/, netem/, samples/ and settings/, and the session's map-*,
nics-*, hostcheck-* and declared-sysctls.txt) it writes, for every run with a
row:

  <set>/manifest-<tag>.json   the fields transport-format.md lists, plus the
                              runner's point manifest ("runner") and the
                              binaries' own ("binary", "pong"; a sink arm's
                              run's sink as "sink": points/<point>/sink<run>)
  <set>/spikes-<tag>.csv      the measuring run's spikes file
  <set>/<host>-events-<tag>.csv  each host's events over the point
                              (transport_events.py: events/<point>-<host>.log.gz,
                              and on host a the measuring loop's stalls)
  <set>/hostcheck-<host>.txt  each host's session report (hostcheck-<host>-hugepages.txt:
                              the one on the hugetlbfs Aeron directory, its arm's points')
  transport-settings.env      the declared settings in fairness.py's form:
                              shared.<knob>=<value> for every transport sysctl
                              (host.sh transport_sysctls); an optional one that
                              no host's kernel has at any point is declared
                              `absent`; one an arm's knob sets otherwise per
                              arm; <arm>.knob.<name> for each per-arm knob an
                              arm ran (manifest/<point>.env's knob.*), and a
                              sweep line for each the tuning swept
                              (TR_TUNED_SWEPT, a tuning session's variants);
                              declared.<arm>=<twin>:knob.<name> for each
                              TR_DECLARED arm that ran

Hosts: a two-host set's are a and b; a one-host set's one host is `a` in the
manifest (transport-format.md), whichever VM it ran on. A run of a failed point
(one the runner kept no row of) is left out and counted. Exit 1, saying
which, when a row of results.csv has no run or a run is missing its
manifest or spikes file; 2 on a usage error.
"""
import csv
import json
import os
import re
import shutil
import sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import transport_events  # noqa: E402
from transport_points import COLUMNS, tag_of  # noqa: E402

# nstat's counters a loss row reads (the growth of every Tcp, TcpExt and Udp
# counter that grew is kept too)
NSTAT_ALWAYS = ("TcpRetransSegs", "TcpExtTCPLossProbes", "TcpExtTCPTimeouts", "UdpInErrors", "UdpRcvbufErrors",
                "UdpSndbufErrors", "UdpNoPorts")
NSTAT_KEEP = ("Tcp", "TcpExt", "Udp")
# a per-arm knob (transport-settings.env's list, transport.sh tr_arm_knobs):
# the binary's manifest key it reads back from, where the binary reports it
# (the rest, the media driver's, as the runner recorded them)
READBACK = {"busy_read_us": "busy_read_us", "epoll_events": "epoll_events", "datagram_burst": "datagram_burst",
            "sockbuf": "rcvbuf", "quickack_per_read": "quickack_per_read", "rto_min_us": "rto_min_us",
            "term_length": "term_length", "capacity": "capacity"}
BOOL = {"true": "on", "false": "off"}
# a per-arm knob read back on each host after the point (manifest/<point>.env's
# <name>_<host>: vm.sh aeron-page, the Aeron directory's page size)
# vm.sh napi, the data NICs' interrupt deferral (transport.sh tr_napi_value),
# and vm.sh idle, the IRQ core's idle depth (transport.sh tr_idle_value)
HOST_READBACK = {"aeron_page": "aeron_page", "napi_defer": "napi_defer", "irq_idle": "irq_idle"}
# the hosts' own knob (a sysctl, read back as one): never a per-arm knob.* line
HOST_WIDE = {"congestion"}
# what a knob's value implies of the hosts' sysctls (vm.sh driver kernel: no
# busy poll), beyond transport_sysctls' own
IMPLIED = {("aeron_path", "kernel"): {"net.core.busy_read": "0", "net.core.busy_poll": "0"}}


def best_of(knob, knobs, tr):
    """An arm's technology's best value of a TR_BEST_BY_TECHNOLOGY knob,
    from its knobs (transport-settings.env): napi_defer, an Aeron kernel
    UDP arm's driver's (kernel-busy: TR_AERON_NAPI_DEFER; kernel: 0), a socket
    arm's TR_SOCKET_NAPI_DEFER; None for a knob without one."""
    if knob == "napi_defer":
        if "aeron_path" in knobs:
            return tr.get("TR_AERON_NAPI_DEFER", "0") if knobs["aeron_path"] == "kernel-busy" else "0"
        return tr.get("TR_SOCKET_NAPI_DEFER", "0")
    return None


def implied(knobs):
    """{sysctl: value} an arm's knobs imply."""
    out = {}
    for k, v in knobs.items():
        out.update(IMPLIED.get((k, v), {}))
        if k == "congestion":
            out["net.ipv4.tcp_congestion_control"] = v
    return out


def swept_of(d, tr):
    """{knob: {arm}}: the tuned section's (TR_TUNED_SWEPT, base arms) and, in
    a tuning session, its variants (variants.txt: each the arm swept)."""
    out = defaultdict(set)
    for kv in tr.get("TR_TUNED_SWEPT", "").strip('"').split():
        k, _, arms = kv.partition(":")
        out[k].update(a for a in arms.split(",") if a)
    for line in (read(os.path.join(d, "variants.txt")) or "").splitlines():
        w = line.split()
        if len(w) == 6:
            out[w[2]].add(w[0])
    return out


def read(path):
    try:
        with open(path) as f:
            return f.read()
    except OSError:
        return None


def read_env(path):
    out = {}
    for line in (read(path) or "").splitlines():
        if "=" in line and not line.startswith("#"):
            k, v = line.split("=", 1)
            out[k] = v
    return out


def read_json(path):
    text = read(path)
    if text is None:
        return None
    try:
        return json.loads(text)
    except ValueError:
        return None


def ints(v):
    try:
        return int(v)
    except (TypeError, ValueError):
        return None


def cpu_map(path):
    """map-<host>.txt "<hk> <irq> <iso,...>" as {housekeeping, irq, hot}."""
    words = (read(path) or "").split()
    if len(words) != 3:
        return None
    return {"housekeeping": [int(words[0])], "irq": [int(words[1])],
            "hot": [int(c) for c in words[2].split(",") if c.isdigit()]}


def nic_names(path):
    """Every name a data NIC's interrupts may carry: interface, VF, PCI address."""
    names = set()
    for line in (read(path) or "").splitlines():
        w = line.split()
        if len(w) >= 7:
            names.update(n for n in (w[1], w[2], w[6]) if n not in ("-", "lo"))
    return names


def interrupts(text):
    """/proc/interrupts as {irq: (name, {cpu: count})} for the numbered rows."""
    out = {}
    lines = (text or "").splitlines()
    if not lines:
        return out
    cpus = [int(c[3:]) for c in lines[0].split() if c.startswith("CPU") and c[3:].isdigit()]
    for line in lines[1:]:
        w = line.split()
        if not w or not re.fullmatch(r"\d+:", w[0]):
            continue
        counts = {}
        for i, cpu in enumerate(cpus):
            if i + 1 < len(w) and w[i + 1].isdigit():
                counts[cpu] = int(w[i + 1])
        out[w[0][:-1]] = (" ".join(w[1 + len(counts):]), counts)
    return out


def irq_deltas(before, after, nics):
    """[{irq, name, data_nic, cpu, delta}] for every numbered IRQ and CPU that grew; None when unread."""
    if before is None or after is None:
        return None
    b, a = interrupts(before), interrupts(after)
    out = []
    for irq, (name, counts) in sorted(a.items(), key=lambda kv: int(kv[0])):
        prev = b.get(irq, ("", {}))[1]
        for cpu, n in sorted(counts.items()):
            d = n - prev.get(cpu, 0)
            if d > 0:
                out.append({"irq": irq, "name": name, "data_nic": any(x in name for x in nics) or "mana" in name,
                            "cpu": cpu, "delta": d})
    return out


def nstat(text):
    """/proc/net/snmp and netstat as {<Proto><Counter>: value}."""
    out, header = {}, {}
    for line in (text or "").splitlines():
        proto, _, rest = line.partition(":")
        words = rest.split()
        if not words:
            continue
        if not all(w.lstrip("-").isdigit() for w in words):
            header[proto] = words
        elif proto in header:
            for k, v in zip(header[proto], words):
                out[proto + k] = int(v)
    return out


def nstat_growth(before, after):
    if before is None or after is None:
        return None
    b, a = nstat(before), nstat(after)
    if not a:
        return None
    out = {}
    for k, v in a.items():
        d = v - b.get(k, 0)
        if k in NSTAT_ALWAYS or (d and any(k.startswith(p) and k[len(p):][:1].isupper() for p in NSTAT_KEEP)):
            out[k] = d
    return out


# a data NIC's counters a mixed-load point reads (vm.sh nicstats; the DPDK
# transport's own, nic.* in the binaries' manifests)
NIC_KEYS = ("tx_bytes", "rx_bytes", "tx_packets", "rx_packets")
DPDK_NIC_KEYS = ("tx_bytes", "rx_bytes", "tx_pkts", "rx_pkts", "rx_missed", "rx_nombuf", "queue_drop", "tx_eagain",
                 "allowance_exceeded")


def nicstats(text):
    """vm.sh nicstats' traffic lines ("<role> <iface> tx_bytes=<n> ...") as
    {iface: {counter: n}}, a NIC DPDK holds as {iface: "dpdk"}; None when
    unread. Its moderation and driver counter lines ("<role> <nic> coalesce
    ..." and "<role> <nic> stat <name> <n>", the c7i diagnosis's) are left
    to the raw nicstats/ files."""
    if not text:
        return None
    out = {}
    for line in text.splitlines():
        w = line.split()
        if len(w) < 3 or w[2] in ("coalesce", "stat"):
            continue
        if w[2] == "dpdk":
            out[w[1]] = "dpdk"
            continue
        out[w[1]] = {k: int(v) for k, _, v in (x.partition("=") for x in w[2:]) if v.lstrip("-").isdigit()}
    return out


def nic_growth(before, after, binary_settings):
    """A host's data NICs over a point, every NIC summed: {source: kernel,
    tx_bytes, rx_bytes, tx_packets, rx_packets, allowance_exceeded} from
    vm.sh nicstats, or where DPDK holds them {source: dpdk, ...} from the
    binary's own nic.* counters; None when unread."""
    b, a = nicstats(before), nicstats(after)
    if b is None or a is None:
        return None
    if any(v == "dpdk" for v in a.values()):
        s = binary_settings or {}
        if not any(f"nic.{k}" in s for k in DPDK_NIC_KEYS):
            return None
        return dict({"source": "dpdk"}, **{k: ints(s.get(f"nic.{k}")) or 0 for k in DPDK_NIC_KEYS})
    out = {"source": "kernel", "allowance_exceeded": 0}
    for k in NIC_KEYS:
        out[k] = 0
    for iface, c in a.items():
        was = b.get(iface) if isinstance(b.get(iface), dict) else {}
        for k, v in c.items():
            d = v - was.get(k, 0)
            if k in NIC_KEYS:
                out[k] += d
            elif k.endswith("allowance_exceeded"):
                out["allowance_exceeded"] += d
    return out


def netem_growth(before, after, pct):
    """{nic: {qdisc, loss_pct, sent_pkts, dropped}} from vm.sh netem stats before and after."""
    if before is None or after is None:
        return None
    parse = lambda t: {w[0]: (int(w[1]), int(w[2])) for w in (l.split() for l in t.splitlines())
                       if len(w) == 3 and w[1].isdigit() and w[2].isdigit()}
    b, a = parse(before), parse(after)
    return {nic: {"qdisc": "netem", "loss_pct": pct, "sent_pkts": s - b.get(nic, (0, 0))[0],
                  "dropped": d - b.get(nic, (0, 0))[1]} for nic, (s, d) in sorted(a.items())}


def root_qdiscs(text):
    """vm.sh netem show's lines as {nic: {qdisc}}: a no-qdisc round's proof."""
    if not text:
        return None
    return {w[0]: {"qdisc": w[1]} for w in (l.split() for l in text.splitlines()) if len(w) == 2}


def threads(text):
    """vm.sh sample's ps lines (after its verdict) as [{tid, comm, cpu, util}]."""
    if text is None:
        return None
    out = []
    for line in text.splitlines()[1:]:
        w = line.split(None, 3)
        if len(w) == 4 and w[0].isdigit() and w[1].isdigit():
            try:
                util = float(w[2]) / 100
            except ValueError:
                continue
            out.append({"tid": int(w[0]), "comm": w[3], "cpu": int(w[1]), "util": util})
    return out


def settings_of(text):
    if text is None:
        return None
    return dict(l.split("=", 1) for l in text.splitlines() if "=" in l)


def row_of(path):
    """A binary's results.csv line as (kind, label, channel, size, rate), or None."""
    text = read(path)
    if not text or not text.strip():
        return None
    cols = text.strip().splitlines()[0].split(",")
    if len(cols) != len(COLUMNS) - 1:
        return None
    return cols[0], cols[1], cols[2], ints(cols[3]), ints(cols[4])


def collect(d):
    env = read_env(os.path.join(d, "env"))
    problems, written, by_tag, skipped = [], 0, {}, []
    # the rows: the runner writes a point's only when every run of it had one
    rows = set()
    path = os.path.join(d, "results.csv")
    if os.path.exists(path):
        with open(path, newline="") as f:
            for raw in csv.reader(f):
                if len(raw) == len(COLUMNS):
                    rows.add(tag_of(raw[2], ints(raw[4]), ints(raw[0]), ints(raw[5]), raw[1]))
    knobs_seen = defaultdict(set)  # knob: values read back anywhere
    arm_knobs = defaultdict(dict)  # arm: {knob: value} as the runner ran it
    hostcheck_done = set()
    for name in sorted(os.listdir(os.path.join(d, "manifest"))):
        if not name.endswith(".env"):
            continue
        pid = name[:-4]
        pe = read_env(os.path.join(d, "manifest", name))
        set_, rep, netem = pe.get("set"), ints(pe.get("rep")), pe.get("netem", "-")
        # the hosts the point used (the runner marks the other of a one-host set unused)
        real = [h for h in ("a", "b") if pe.get(f"placement_{h}") != "unused"] or ["a"]
        keys = dict(zip(("a", "b"), real))  # manifest key: the real host
        folder = os.path.join(d, set_)
        os.makedirs(folder, exist_ok=True)
        # each host's session report that covers the arm (the runner's
        # hostcheck_<host>: on the hugetlbfs, the one on its directory)
        reports = {key: pe.get(f"hostcheck_{h}") or f"hostcheck-{h}.txt" for key, h in keys.items()}
        for key, h in keys.items():
            src, dst = os.path.join(d, reports[key]), os.path.join(folder, reports[key])
            if (set_, reports[key]) not in hostcheck_done and os.path.exists(src):
                shutil.copyfile(src, dst)
                hostcheck_done.add((set_, reports[key]))
        snap = lambda kind, h, when: read(os.path.join(d, kind, f"{pid}-{h}-{when}"))
        pong = read_json(os.path.join(d, "points", pid, "pong", "manifest.json"))
        settings = {}
        for key, h in keys.items():
            s = settings_of(read(os.path.join(d, "settings", f"{pid}-{h}")))
            if s is not None:
                settings[key] = s
                for k, v in s.items():
                    knobs_seen[k].add(v)
        order = [("aa" if a.endswith("@AA=1") else a) for a in pe.get("order", "").split(",") if a]
        runs = sorted(r for r in os.listdir(os.path.join(d, "points", pid)) if r.isdigit()) \
            if os.path.isdir(os.path.join(d, "points", pid)) else []
        for r in runs:
            out = os.path.join(d, "points", pid, r)
            row = row_of(os.path.join(out, "results.csv"))
            bm = read_json(os.path.join(out, "manifest.json"))
            if row is None or bm is None:
                # a run of a failed point that wrote nothing (failures.csv)
                skipped.append(f"{pid} run {r}")
                continue
            kind, label, channel, size, rate = row
            bs = bm.get("settings", {})
            tag = tag_of(label, size, rep, rate, kind)
            if tag not in rows:
                # a run of a point that failed (failures.csv): no row, so nothing to compare
                skipped.append(f"{pid} run {r}")
                continue
            aa = pe.get("aa") == "1"
            # a sink arm's run: its own sink on b (None: none written)
            sink = (read_json(os.path.join(d, "points", pid, f"sink{r}", "manifest.json"))
                    if pe.get("arm_hosts") == "sink" else None)
            # each host's own binary manifest: the measuring run's on a, the pong's (or sink's) on b
            own = {"a": bm, "b": pong if pe.get("arm_hosts") != "sink" else sink}
            # the arm's per-arm knobs (the runner's knob.* lines), each read
            # back from the binary that reports it, else as the runner set it
            run_settings = {k: dict(v) for k, v in settings.items()}
            knobs = {k[len("knob."):]: v for k, v in pe.items() if k.startswith("knob.")}
            arm_knobs[pe.get("arm")].update(knobs)
            for key in run_settings:
                ms = (own.get(key) or {}).get("settings") or {}
                for k, v in knobs.items():
                    if k in HOST_WIDE:
                        continue
                    got = ms.get(READBACK[k]) if k in READBACK else None
                    if k in HOST_READBACK:
                        # read back on the host, never taken as set
                        got = pe.get(f"{HOST_READBACK[k]}_{keys[key]}") or "unread"
                    run_settings[key][f"knob.{k}"] = v if got is None else BOOL.get(got, got)
            loss = {}
            for key, h in keys.items():
                src = {}
                g = nstat_growth(snap("nstat", h, "before"), snap("nstat", h, "after"))
                if g is not None:
                    src["nstat"] = g
                m = own.get(key) or {}
                ms = m.get("settings", {})
                if bs.get("transport") == "tcp" and ints(ms.get("retrans")) is not None:
                    # the traffic's counters as tcp_lat splits them (sock.rs): real
                    # loss is retrans_not_dsack and lost_end, DSACK dups spurious
                    src["tcp_info"] = {"retrans": ints(ms.get("tcp.retrans", ms.get("retrans")))}
                    for k in ("retrans_setup", "dsack_dups", "dsack_dups_setup", "retrans_not_dsack", "lost_end",
                              "rtos", "rtos_setup"):
                        if f"tcp.{k}" in ms:
                            src["tcp_info"][k] = ints(ms[f"tcp.{k}"])
                    win = {k[len("netstat.window."):]: ints(v) for k, v in ms.items() if k.startswith("netstat.window.")}
                    if win:
                        src["netstat_window"] = win
                aeron = {k[len("aeron."):]: ints(v) for k, v in ms.items() if k.startswith("aeron.")}
                if aeron:
                    src["aeron"] = aeron
                loss[key] = src
            nics = {key: nic_names(os.path.join(d, f"nics-{h}.txt")) for key, h in keys.items()}
            # the point's data NIC traffic (vm.sh nicstats), each host's: a
            # mixed-load point's against its driver (fairness.py mixed-nic)
            nic = {key: g for key, h in keys.items()
                   if (g := nic_growth(snap("nicstats", h, "before"), snap("nicstats", h, "after"),
                                       (own.get(key) or {}).get("settings"))) is not None}
            manifest = {
                "format": 1, "set": set_, "arm": label.partition(":")[2].split("@")[0], "label": label,
                "kind": kind, "transport": bs.get("transport"), "channel": channel,
                "round": f"{set_}:rep{rep}:netem{netem}:streams{pe.get('streams')}:rate{rate}",
                "rep": rep, "rate": rate, "streams": ints(bs.get("streams")), "size": bm.get("payload_size"),
                "duration_s": ints(bs.get("duration_s")), "warmup_s": ints(bs.get("warmup_s")),
                "netem": None if netem == "-" else netem, "aa": aa, "aa_of": pe.get("arm") if aa else None,
                "payload_sha256": bm.get("payload_sha256"), "wire_bytes": bm.get("wire_bytes"),
                "core_build": bm.get("core_build_hash"), "seed": ints(pe.get("seed")), "arm_order": order,
                "scheduled": ints(bs.get("scheduled")), "late": ints(bs.get("late")),
                "max_send_lag_ns": ints(bs.get("max_send_lag_ns")),
                "vms": {key: env.get(f"vm_{h}") for key, h in keys.items()},
                "cpus": {key: cpu_map(os.path.join(d, f"map-{h}.txt")) for key, h in keys.items()},
                "threads": {key: [t] for key, h in keys.items()
                            if (t := threads(read(os.path.join(d, "samples", f"{pid}-{h}")))) is not None},
                "irqs": {key: q for key, h in keys.items()
                         if (q := irq_deltas(snap("irq", h, "before"), snap("irq", h, "after"), nics[key])) is not None},
                "hostcheck": {key: reports[key] for key in keys if os.path.exists(os.path.join(d, reports[key]))},
                "settings": run_settings,
                "netem_counters": None if netem == "-" else {
                    key: g for key, h in keys.items()
                    if (g := (root_qdiscs(snap("netem", h, "after")) if netem == "none" else
                              netem_growth(snap("netem", h, "before"), snap("netem", h, "after"), netem))) is not None},
                "loss_counters": loss,
                "nic_counters": nic or None,
                "hosts": keys, "point": pid, "runner": pe, "binary": bm, "pong": pong, "sink": sink,
            }
            # each host's events over the point (transport_events.py): the
            # measuring host's with its loop's stalls (its last busy CPU)
            manifest["events"] = {}
            for key, h in keys.items():
                stalls = None
                if key == "a":
                    cpus_used = [c for c in str(bs.get("cpu_list", "")).split(",") if c.isdigit()]
                    stalls = (os.path.join(out, f"stalls-{label}.csv"), int(cpus_used[-1]) if cpus_used else None,
                              bs.get("role", "measuring") + " receive")
                log = os.path.join(d, "events", f"{pid}-{h}.log.gz")
                name = f"{key}-events-{tag}.csv"
                dst = os.path.join(folder, name)
                if os.path.exists(dst) and os.path.exists(log) and os.path.getmtime(dst) >= os.path.getmtime(log):
                    # laid out by an earlier pass (the tuning session collects
                    # after each phase): its rows and notes as written
                    text = read(dst) or ""
                    notes = [l[len("# note: "):] for l in text.splitlines() if l.startswith("# note: ")]
                    n = sum(1 for l in text.splitlines() if l and not l.startswith("#")) - 1
                    manifest["events"][key] = {"file": name, "rows": max(n, 0), "notes": notes}
                    continue
                rows_, notes = transport_events.events(log, stalls)
                with open(dst, "w", newline="") as f:
                    transport_events.write(f, rows_, notes)
                manifest["events"][key] = {"file": name, "rows": len(rows_), "notes": notes}
            with open(os.path.join(folder, f"manifest-{tag}.json"), "w") as f:
                json.dump(manifest, f, indent=1, sort_keys=True)
            spikes = os.path.join(out, f"spikes-{label}.csv")
            if os.path.exists(spikes):
                shutil.copyfile(spikes, os.path.join(folder, f"spikes-{tag}.csv"))
            else:
                problems.append(f"{pid} run {r}: no spikes-{label}.csv")
            by_tag[tag] = pid
            written += 1

    for t in sorted(rows - set(by_tag)):
        problems.append(f"results.csv row {t}: no run")

    # the declared settings, as fairness.py reads them
    lines = ["# transport_collect.py: transport-settings.tr.env's sysctls (host.sh transport_sysctls),",
             "# each the value every host must read back at every point; an optional one no host's",
             "# kernel had at any point is declared absent"]
    tr = read_env(os.path.join(d, "transport-settings.tr.env"))
    swept = swept_of(d, tr)
    # a sysctl an arm's knob sets otherwise (a tuning variant's congestion
    # control, the kernel path's busy poll) is every arm's own, each its value
    shared = {}
    for line in (read(os.path.join(d, "declared-sysctls.txt")) or "").splitlines():
        w = line.split()
        if len(w) < 3:
            continue
        knob, value, need = w[0].replace("/", "."), " ".join(w[1:-1]), w[-1]
        if need == "optional" and knobs_seen.get(knob) == {"absent"}:
            value = "absent"
        shared[knob] = value
    own = {arm: implied(k) for arm, k in arm_knobs.items()}
    per_arm_sysctls = sorted({s for arm in own for s, v in own[arm].items() if s in shared and v != shared[s]})
    for knob, value in shared.items():
        if knob not in per_arm_sysctls:
            lines.append(f"shared.{knob}={value}")
    if per_arm_sysctls:
        lines.append("# sysctls an arm's knob sets: each arm's own (transport_collect.py IMPLIED)")
        for sysctl in per_arm_sysctls:
            for arm in sorted(arm_knobs):
                lines.append(f"{arm}.{sysctl}={own[arm].get(sysctl, shared[sysctl])}")
            by = {"net.ipv4.tcp_congestion_control": ("congestion",)}.get(sysctl, ("aeron_path",))
            arms = sorted(set().union(*(swept.get(k, set()) for k in by)) & set(arm_knobs))
            if arms:
                lines.append(f"sweep.{sysctl}={','.join(arms)}")
    names = sorted({k for kn in arm_knobs.values() for k in kn if k not in HOST_WIDE})
    if names:
        lines += ["# each arm's per-arm knobs as it ran them (transport-settings.env's list); a sweep",
                  "# line names the arms a tuning session swept for one (TR_TUNED_SWEPT, its variants)"]
        for arm in sorted(arm_knobs):
            for k in sorted(arm_knobs[arm]):
                if k not in HOST_WIDE:
                    lines.append(f"{arm}.knob.{k}={arm_knobs[arm][k]}")
        for k in names:
            arms = sorted(swept.get(k, set()) & set(arm_knobs))
            if arms:
                lines.append(f"sweep.knob.{k}={','.join(arms)}")
    # each technology's own best (TR_BEST_BY_TECHNOLOGY): fairness.py's
    # shared-knob rule takes an arm at it as needing no sweep
    for k in tr.get("TR_BEST_BY_TECHNOLOGY", "").strip('"').split():
        best = sorted((arm, b) for arm in arm_knobs if k in arm_knobs[arm]
                      and (b := best_of(k, arm_knobs[arm], tr)) is not None)
        if best:
            lines.append(f"# each arm's technology's best {k} (TR_BEST_BY_TECHNOLOGY)")
            lines.append(f"best.knob.{k}=" + ",".join(f"{arm}:{b}" for arm, b in best))
    # declared technology differences (TR_DECLARED "<arm>:<twin>:<knob>=<value>"):
    # fairness.py's declared rule, each arm that ran
    declared = [e.split(":") for e in tr.get("TR_DECLARED", "").strip('"').split() if e.count(":") == 2]
    declared = [(a, twin, kv.partition("=")[0]) for a, twin, kv in declared if a in arm_knobs]
    if declared:
        lines.append("# each declared twin and the one knob it differs from its twin in (TR_DECLARED)")
        lines += [f"declared.{a}={twin}:knob.{k}" for a, twin, k in declared]
    with open(os.path.join(d, "transport-settings.env"), "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"transport_collect: {written} runs with a row, {len(rows)} rows in results.csv, "
          f"{len(skipped)} runs of failed points left out, "
          f"{len(problems)} problem{'s' if len(problems) != 1 else ''}")
    for p in problems:
        print(f"  {p}")
    return 1 if problems else 0


def main():
    if len(sys.argv) != 2 or not os.path.isdir(os.path.join(sys.argv[1], "manifest")):
        sys.exit("usage: transport_collect.py <transport.sh session dir> (with manifest/)")
    sys.exit(collect(sys.argv[1]))


if __name__ == "__main__":
    main()
