#!/usr/bin/env python3
"""The transport suite's analysis against fixtures: fairness.py (a clean run,
then one fixture per violated rule), spikes.py --transport (one fixture per
label) and analyse.py --transport (expected Markdown, refusal).

python3 test_transport.py [-v]; UPDATE_EXPECTED=1 rewrites
expected-transport.md from the clean fixture."""
import csv
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
CLOUD = os.path.dirname(HERE)
sys.path[:0] = [CLOUD, HERE]

import analyse  # noqa: E402
import fairness  # noqa: E402
import spikes  # noqa: E402
import transport_fixture as fx  # noqa: E402
from transport_points import COLUMNS, read_points, tag_of, tcp_loss  # noqa: E402

CLEAN = None


def read(path):
    with open(path) as f:
        return f.read()


def write(path, text, mode="w"):
    with open(path, mode) as f:
        f.write(text)


def setUpModule():
    global CLEAN
    CLEAN = tempfile.mkdtemp(prefix="transport-clean-")
    fx.make(os.path.join(CLEAN, "run"))


def tearDownModule():
    shutil.rmtree(CLEAN, ignore_errors=True)


class Run:
    """A copy of the clean fixture to mutate."""

    def __init__(self, test):
        self.dir = tempfile.mkdtemp(prefix="transport-")
        test.addCleanup(shutil.rmtree, self.dir, True)
        self.run = os.path.join(self.dir, "run")
        shutil.copytree(os.path.join(CLEAN, "run"), self.run)

    def manifests(self, pred=lambda m: True):
        out = []
        for set_ in sorted(os.listdir(self.run)):
            folder = os.path.join(self.run, set_)
            if not os.path.isdir(folder):
                continue
            for name in sorted(os.listdir(folder)):
                if name.startswith("manifest-"):
                    path = os.path.join(folder, name)
                    m = json.loads(read(path))
                    if pred(m):
                        out.append((path, m))
        return out

    def edit(self, pred, fn, first=True):
        """fn(manifest) on the first (or every) manifest pred matches."""
        hits = self.manifests(pred)
        assert hits, "no manifest matched"
        for path, m in hits[:1] if first else hits:
            fn(m)
            write(path, json.dumps(m))
        return hits[0][1]

    def rows(self, pred, fn):
        path = os.path.join(self.run, "results.csv")
        rows = list(csv.reader(read(path).splitlines()))
        n = 0
        for r in rows:
            if len(r) == len(COLUMNS) and pred(dict(zip(COLUMNS, r))):
                fn(r)
                n += 1
        assert n, "no row matched"
        with open(path, "w", newline="") as f:
            csv.writer(f).writerows(rows)

    def drop_rows(self, pred):
        path = os.path.join(self.run, "results.csv")
        rows = [r for r in csv.reader(read(path).splitlines()) if not (len(r) == len(COLUMNS) and pred(dict(zip(COLUMNS, r))))]
        with open(path, "w", newline="") as f:
            csv.writer(f).writerows(rows)

    def path(self, *parts):
        return os.path.join(self.run, *parts)

    def add_twin(self, set_, arm, twin, knob, value, default, declare=True):
        """A copy of arm in every round of set_ as twin (aeron-ipc-huge's
        shape: its rows, manifests, spikes and events files), the two apart
        only in knob: arm at default, twin at value; with declare, the
        settings file's declared line naming the difference."""
        env = read(self.path("transport-settings.env"))
        own = [l for l in env.splitlines() if l.startswith(f"{arm}.")]
        env += "".join(f"{twin}.{l[len(arm) + 1:]}\n" for l in own)
        env += f"{arm}.{knob}={default}\n{twin}.{knob}={value}\n"
        if declare:
            env += f"declared.{twin}={arm}:{knob}\n"
        write(self.path("transport-settings.env"), env)
        folder = self.path(set_)
        for path, m in self.manifests(lambda m: m["set"] == set_):
            m["arm_order"] = m["arm_order"] + [twin]
            if m["arm"] == arm or m.get("aa_of") == arm:
                for s in m["settings"].values():
                    s[knob] = default
            if m["arm"] == arm:
                t = dict(m, arm=twin, label=m["label"].replace(f"{set_}:{arm}", f"{set_}:{twin}", 1),
                         settings={h: dict(s, **{knob: value}) for h, s in m["settings"].items()})
                old = tag_of(m["label"], m["size"], m["rep"], m["rate"], m["kind"])
                new = tag_of(t["label"], t["size"], t["rep"], t["rate"], t["kind"])
                write(os.path.join(folder, f"manifest-{new}.json"), json.dumps(t))
                for name in ("spikes-{}.csv", "a-events-{}.csv", "b-events-{}.csv"):
                    if os.path.exists(os.path.join(folder, name.format(old))):
                        shutil.copyfile(os.path.join(folder, name.format(old)), os.path.join(folder, name.format(new)))
            write(path, json.dumps(m))
        path = self.path("results.csv")
        rows = list(csv.reader(read(path).splitlines()))
        lab = COLUMNS.index("label")
        add = [r[:lab] + [r[lab].replace(f"{set_}:{arm}", f"{set_}:{twin}", 1)] + r[lab + 1:] for r in rows
               if len(r) == len(COLUMNS) and (r[lab] == f"{set_}:{arm}" or r[lab].startswith(f"{set_}:{arm}@"))]
        assert add, "no row of the arm"
        with open(path, "w", newline="") as f:
            csv.writer(f).writerows(rows + add)

    def rules(self):
        return sorted({v.rule for v in fairness.check(self.run)})


ONE = lambda m: m["set"] == "2a" and m["arm"] == "mio-udp" and m["streams"] == 1 and m["rep"] == 1


class FairnessTest(unittest.TestCase):
    def assertOnly(self, run, rule):
        got = run.rules()
        self.assertEqual(got, [rule], "\n".join(str(v) for v in fairness.check(run.run)))

    def test_clean_fixture_has_no_violations(self):
        self.assertEqual(fairness.check(os.path.join(CLEAN, "run")), [])

    def test_missing_manifest(self):
        run = Run(self)
        os.remove(run.manifests(ONE)[0][0])
        self.assertIn("manifest", run.rules())

    def test_manifest_without_its_row(self):
        run = Run(self)
        run.drop_rows(lambda r: r["label"] == "2a:mio-udp" and r["rep"] == "1")
        self.assertIn("manifest", run.rules())

    def test_manifest_disagrees_with_row(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(rate=20_000))
        self.assertIn("manifest", run.rules())

    def test_payload_sha_differs_in_round(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(payload_sha256="00" * 32))
        self.assertOnly(run, "payload")

    def test_payload_size_differs_in_round(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(size=128))
        run.rows(lambda r: r["label"] == "2a:mio-udp" and r["rep"] == "1", lambda r: r.__setitem__(4, "128"))
        os.rename(run.path("2a", "manifest-2a-mio-udp-64-1-r10000.json"),
                  run.path("2a", "manifest-2a-mio-udp-128-1-r10000.json"))
        os.rename(run.path("2a", "spikes-2a-mio-udp-64-1-r10000.csv"),
                  run.path("2a", "spikes-2a-mio-udp-128-1-r10000.csv"))
        os.rename(run.path("2a", "a-events-2a-mio-udp-64-1-r10000.csv"),
                  run.path("2a", "a-events-2a-mio-udp-128-1-r10000.csv"))
        self.assertIn("payload", run.rules())

    def test_wire_bytes_row_against_manifest(self):
        run = Run(self)
        run.rows(lambda r: r["label"] == "2a:mio-udp" and r["rep"] == "1", lambda r: r.__setitem__(28, "120"))
        self.assertOnly(run, "payload")

    def test_aa_wire_bytes_differ_from_its_arm(self):
        run = Run(self)
        aa = lambda m: m["set"] == "2a" and m["arm"] == "aa" and m["streams"] == 1 and m["rep"] == 1
        run.edit(aa, lambda m: m.update(wire_bytes=106))
        run.rows(lambda r: r["label"] == "2a:aa" and r["rep"] == "1", lambda r: r.__setitem__(28, "106"))
        self.assertOnly(run, "payload")

    def test_core_build_differs(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(core_build="core-ffff"))
        self.assertOnly(run, "build")

    def test_schedule_differs(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(duration_s=20))
        self.assertOnly(run, "schedule")

    def test_busy_thread_off_the_hot_cpus(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["threads"]["a"][0][0].update(cpu=12))
        self.assertOnly(run, "placement")

    def test_busy_thread_not_alone(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["threads"]["b"][1].append({"tid": 9, "comm": "kworker/4:1", "cpu": 4, "util": 0.2}))
        self.assertOnly(run, "placement")

    def test_busy_thread_on_cpu0(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["threads"]["a"][2].append({"tid": 9, "comm": "spinner", "cpu": 0, "util": 0.95}))
        self.assertOnly(run, "placement")

    def test_no_placement_samples(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["threads"].pop("b"))
        self.assertOnly(run, "placement")

    def test_data_nic_irq_on_a_hot_cpu(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["irqs"]["a"].append(
            {"irq": "46", "name": "mana_q1", "data_nic": True, "cpu": 6, "delta": 1}))
        self.assertOnly(run, "irq")

    def test_irqs_not_read(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["irqs"].pop("a"))
        self.assertOnly(run, "irq")

    def test_hostcheck_warns(self):
        run = Run(self)
        with open(run.path("2a", "b-hostcheck.txt"), "a") as f:
            f.write("T15 warn: another process may run on CPU 4 (evidence: /proc/77/status)\n")
        self.assertOnly(run, "hostcheck")

    def test_hostcheck_unproven_blocking_check(self):
        run = Run(self)
        with open(run.path("2a", "a-hostcheck.txt"), "a") as f:
            f.write("T2 unknown: no CPU quota: cgroup not readable\n")
        self.assertOnly(run, "hostcheck")

    def test_hostcheck_missing(self):
        run = Run(self)
        os.remove(run.path("3b", "a-hostcheck.txt"))
        self.assertOnly(run, "hostcheck")

    def test_setting_differs_from_env(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["settings"]["a"].update({"net.core.busy_poll": "0"}))
        self.assertOnly(run, "settings")

    def test_setting_not_in_env(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["settings"]["b"].update({"socket.rcvbuf": "4194304"}))
        self.assertOnly(run, "settings")

    def test_settings_env_missing(self):
        run = Run(self)
        os.remove(run.path("transport-settings.env"))
        self.assertIn("settings", run.rules())

    def test_per_arm_knob_not_swept_for_a_comparable_arm(self):
        # mio-udp at its own value, swept; eio-udp, at another, not
        run = Run(self)
        env = read(run.path("transport-settings.env"))
        env = env.replace("sweep.socket.busy_poll_us=mio-udp,eio-udp", "sweep.socket.busy_poll_us=mio-udp")
        env = env.replace("mio-udp.socket.busy_poll_us=50", "mio-udp.socket.busy_poll_us=0")
        write(run.path("transport-settings.env"), env)
        run.edit(lambda m: m.get("arm") == "mio-udp",
                 lambda m: [s.update({"socket.busy_poll_us": "0"}) for s in m["settings"].values()], first=False)
        self.assertOnly(run, "shared-knob")

    def test_per_arm_knob_at_one_value_needs_no_sweep(self):
        # every arm that has it at the same value: no arm's own tweak
        run = Run(self)
        env = read(run.path("transport-settings.env"))
        write(run.path("transport-settings.env"), env.replace("sweep.socket.busy_poll_us=mio-udp,eio-udp\n", ""))
        self.assertEqual(run.rules(), [])

    # a declared technology difference (aeron-ipc-huge: aeron-ipc on 2 MiB
    # pages): the twin arm differs from its arm in the declared knob alone
    PAGE = "knob.aeron_page"

    def twin(self, declare=True):
        run = Run(self)
        run.add_twin("2b", "aeron-spy", "aeron-spy-huge", self.PAGE, "2097152", "4096", declare)
        return run

    def test_declared_difference_is_no_violation(self):
        run = self.twin()
        self.assertEqual(run.rules(), [], "\n".join(str(v) for v in fairness.check(run.run)))

    def test_undeclared_difference_is_an_unswept_knob(self):
        self.assertOnly(self.twin(declare=False), "shared-knob")

    def test_declared_value_not_read_back(self):
        run = self.twin()
        run.edit(lambda m: m["arm"] == "aeron-spy-huge", lambda m: m["settings"]["a"].update({self.PAGE: "4096"}))
        self.assertOnly(run, "settings")

    def test_declared_twin_differs_in_another_knob(self):
        run = self.twin()
        env = read(run.path("transport-settings.env"))
        write(run.path("transport-settings.env"), env.replace("aeron-spy-huge.aeron.threading.mode=DEDICATED",
                                                               "aeron-spy-huge.aeron.threading.mode=SHARED"))
        run.edit(lambda m: m["arm"] == "aeron-spy-huge",
                 lambda m: [s.update({"aeron.threading.mode": "SHARED"}) for s in m["settings"].values()], first=False)
        self.assertIn("declared", run.rules())

    def test_declared_twin_runs_another_driver(self):
        run = self.twin()
        run.edit(lambda m: m["set"] == "2b",
                 lambda m: m.update(runner={"bin": "aeron_lat", "args": "--mode spy", "driver_mode": (
                     "kernel-busy@AERON_THREADING_MODE=SHARED" if m["arm"] == "aeron-spy-huge" else "kernel-busy")}),
                 first=False)
        self.assertOnly(run, "declared")

    def test_declared_twin_runs_another_binary_or_arguments(self):
        run = self.twin()
        run.edit(lambda m: m["set"] == "2b",
                 lambda m: m.update(runner={"bin": "aeron_lat", "driver_mode": "kernel-busy", "args": (
                     "--mode ipc" if m["arm"] == "aeron-spy-huge" else "--mode spy")}), first=False)
        self.assertOnly(run, "declared")

    def test_declared_twin_without_its_arm_in_the_round(self):
        run = self.twin()
        env = read(run.path("transport-settings.env"))
        write(run.path("transport-settings.env"), env.replace("declared.aeron-spy-huge=aeron-spy:",
                                                               "declared.aeron-spy-huge=aeron-self-udp:"))
        self.assertIn("declared", run.rules())

    # the data NICs' interrupt deferral: each technology at its own best
    # (best.knob.napi_defer: Aeron's busy-polling kernel path deferred, the
    # sockets, waiting in epoll, not), read back per point on every host
    NAPI = "knob.napi_defer"

    def test_declared_best_napi_difference_is_no_violation(self):
        # the clean fixture's round 2a: aeron-kernel at 2, the socket arms at 0
        run = Run(self)
        self.assertEqual(run.rules(), [], "\n".join(str(v) for v in fairness.check(run.run)))

    def test_napi_difference_without_its_best_line_is_an_unswept_knob(self):
        run = Run(self)
        env = read(run.path("transport-settings.env"))
        write(run.path("transport-settings.env"), "".join(
            l + "\n" for l in env.splitlines() if not l.startswith("best.")))
        self.assertOnly(run, "shared-knob")

    def test_arm_off_its_declared_best_is_an_unswept_knob(self):
        # mio-udp declared and run at 2, its best 0, not swept: refused
        run = Run(self)
        env = read(run.path("transport-settings.env"))
        write(run.path("transport-settings.env"), env.replace("mio-udp.knob.napi_defer=0", "mio-udp.knob.napi_defer=2"))
        run.edit(lambda m: m.get("arm") == "mio-udp",
                 lambda m: [s.update({self.NAPI: "2"}) for s in m["settings"].values()], first=False)
        self.assertEqual(run.rules(), ["napi", "shared-knob"])

    def test_socket_point_with_deferral_on_is_refused(self):
        # a socket arm declared 0 whose NICs read back 2: the settings rule
        # and the napi rule's own refusal (its packets waited for the timer)
        run = Run(self)
        run.edit(ONE, lambda m: m["settings"]["b"].update({self.NAPI: "2"}))
        found = [str(v) for v in fairness.check(run.run) if v.rule == "napi"]
        self.assertEqual(found, ["napi: 2a: 2a-mio-udp-64-1-r10000: b's data NICs defer interrupts (napi_defer 2) "
                                 "under a socket arm: its epoll never busy-polls, so each packet waits for the "
                                 "gro_flush timer"])
        self.assertEqual(run.rules(), ["napi", "settings"])

    def test_socket_point_nic_read_back_with_deferral_is_refused(self):
        # the knob read back 0, but one NIC's own values (the runner's raw
        # napi_<host>) show a gro_flush timer left on
        run = Run(self)
        run.edit(ONE, lambda m: m.update(runner={"setup": "sockets", "napi_a": "eth1:0/0,eth2:0/200000",
                                                 "napi_b": "eth1:0/0,eth2:0/0"}))
        found = [str(v) for v in fairness.check(run.run) if v.rule == "napi"]
        self.assertEqual(found, ["napi: 2a: 2a-mio-udp-64-1-r10000: a's data NICs defer interrupts "
                                 "(eth1:0/0,eth2:0/200000) under a socket arm: its epoll never busy-polls, so each "
                                 "packet waits for the gro_flush timer"])

    def test_socket_point_without_deferral_read_back_is_refused(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["settings"]["a"].pop(self.NAPI))
        self.assertIn("napi", run.rules())

    def test_socket_arm_swept_to_deferral_is_its_tuned_best(self):
        # a tuning session chose 2 for mio-udp (swept, with eio-udp at 0):
        # its deferral is its frozen best, not refused
        run = Run(self)
        env = read(run.path("transport-settings.env")).replace("mio-udp.knob.napi_defer=0", "mio-udp.knob.napi_defer=2")
        write(run.path("transport-settings.env"), env + "sweep.knob.napi_defer=mio-udp,eio-udp\n")
        run.edit(lambda m: m.get("arm") == "mio-udp",
                 lambda m: [s.update({self.NAPI: "2"}) for s in m["settings"].values()], first=False)
        self.assertEqual(run.rules(), [], "\n".join(str(v) for v in fairness.check(run.run)))

    def test_knob_both_shared_and_per_arm(self):
        run = Run(self)
        with open(run.path("transport-settings.env"), "a") as f:
            f.write("shared.tcp.quickack=1\n")
        self.assertIn("shared-knob", run.rules())

    def test_order_not_randomised(self):
        run = Run(self)
        fixed = ["aa", "aeron-dpdk", "aeron-kernel", "eio-udp", "mio-udp"]
        run.edit(lambda m: m["set"] == "2a" and m["streams"] == 1, lambda m: m.update(arm_order=fixed), first=False)
        self.assertOnly(run, "order")

    def test_order_misses_an_arm(self):
        run = Run(self)
        run.edit(lambda m: m["round"] == "2a-s1-n--r10000-rep1",
                 lambda m: m.update(arm_order=[a for a in m["arm_order"] if a != "eio-udp"]), first=False)
        self.assertOnly(run, "order")

    def test_no_seed(self):
        run = Run(self)
        run.edit(lambda m: m["round"] == "2a-s1-n--r10000-rep1", lambda m: m.pop("seed"), first=False)
        self.assertOnly(run, "order")

    def test_round_without_aa(self):
        run = Run(self)
        run.drop_rows(lambda r: r["label"] == "3b:aa@NETEM=0.1" and r["rep"] == "2")
        os.remove(run.path("3b", "manifest-3b-aa@NETEM=0.1-64-2-r10000.json"))
        run.edit(lambda m: m["round"] == "3b-s1-n0.1-r10000-rep2",
                 lambda m: m.update(arm_order=[a for a in m["arm_order"] if a != "aa"]), first=False)
        self.assertOnly(run, "aa")

    def test_loss_columns_empty(self):
        run = Run(self)
        run.rows(lambda r: r["label"] == "2a:mio-udp" and r["rep"] == "1", lambda r: r.__setitem__(24, ""))
        self.assertOnly(run, "loss-accounting")

    def test_retrans_empty_where_the_transport_has_none(self):
        # UDP and mpsc have no retransmit counter: an empty retrans is no
        # violation there, and is one for TCP
        run = Run(self)
        run.rows(lambda r: r["label"] == "2a:mio-udp" and r["rep"] == "1", lambda r: r.__setitem__(27, ""))
        self.assertEqual(fairness.check(run.run), [])
        run.rows(lambda r: r["label"] == "3b:mio-tcp@NETEM=0.1" and r["rep"] == "1", lambda r: r.__setitem__(27, ""))
        self.assertOnly(run, "loss-accounting")

    def test_tcp_loss_is_retrans_not_dsack_and_lost(self):
        # real loss: retransmits the peer did not report duplicates, and
        # segments lost at the end, on every host; DSACK-reported ones are
        # spurious. On a row without loss real loss is a bug to trace, while
        # spurious ones are reported, not counted
        run = Run(self)
        p = [p for p in read_points(run.run) if p.arm == "mio-tcp"][0]
        self.assertEqual(tcp_loss(p), (4, 2))
        self.assertIsNone(fairness.tcp_loss_problem(p))
        # with no guest drop counter grown on either host the loss is below
        # the guest's stack (the NIC or the network): a note, not a violation
        self.assertIsNone(fairness.tcp_loss_problem(p._replace(netem=None)))
        self.assertIn("4 segments lost below the guest", fairness.path_loss(p._replace(netem=None)))
        self.assertIsNone(fairness.path_loss(p))
        # a drop the guest counted (its socket backlog full) is a bug to trace
        dropped = {h: dict(src, nstat=dict(src["nstat"], TcpExtTCPBacklogDrop=2))
                   for h, src in p.manifest["loss_counters"].items()}
        d = p._replace(manifest=dict(p.manifest, loss_counters=dropped))
        self.assertIn("4 segments lost", fairness.tcp_loss_problem(d._replace(netem=None)))
        self.assertIn("TcpExtTCPBacklogDrop 4", fairness.tcp_loss_problem(d._replace(netem="0")))
        spurious = {h: dict(src, tcp_info=dict(src["tcp_info"], retrans_not_dsack=0, dsack_dups=3))
                    for h, src in p.manifest["loss_counters"].items()}
        q = p._replace(netem=None, manifest=dict(p.manifest, loss_counters=spurious))
        self.assertEqual(tcp_loss(q), (0, 6))
        self.assertIsNone(fairness.tcp_loss_problem(q))

    def test_tcp_info_without_its_dsack_split(self):
        run = Run(self)
        run.edit(lambda m: m["arm"] == "mio-tcp",
                 lambda m: m["loss_counters"]["a"]["tcp_info"].pop("retrans_not_dsack"))
        self.assertOnly(run, "loss-accounting")

    def test_tcp_without_tcp_info(self):
        run = Run(self)
        run.edit(lambda m: m["arm"] == "mio-tcp", lambda m: m["loss_counters"]["b"].pop("tcp_info"))
        self.assertOnly(run, "loss-accounting")

    def test_joined_transport_takes_its_parts_loss_sources(self):
        # a transport of several (udp+tcp) needs every part's sources: tcp's
        # tcp_info and nstat, udp's nstat
        run = Run(self)
        run.edit(lambda m: m["arm"] == "mio-tcp", lambda m: m.update(transport="udp+tcp"), first=False)
        self.assertEqual(fairness.check(run.run), [])
        run.edit(lambda m: m["arm"] == "mio-tcp", lambda m: m["loss_counters"]["b"].pop("tcp_info"))
        self.assertOnly(run, "loss-accounting")
        self.assertEqual(fairness.loss_sources("udp+tcp"), ("nstat", "tcp_info"))
        self.assertIsNone(fairness.loss_sources("udp+nosuch"))

    def test_lateness_not_counted(self):
        run = Run(self)
        run.rows(lambda r: r["label"] == "2a:mio-udp" and r["rep"] == "1", lambda r: r.__setitem__(21, ""))
        self.assertOnly(run, "lateness")

    def test_lateness_row_against_manifest(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(late=7))
        self.assertOnly(run, "lateness")

    def test_unexplained_max(self):
        run = Run(self)
        os.remove(run.path("2a", "a-events-2a-mio-udp-64-1-r10000.csv"))
        self.assertOnly(run, "max")

    def test_netem_outside_its_binomial_interval(self):
        run = Run(self)
        run.edit(lambda m: m["arm"] == "mio-tcp",
                 lambda m: m["netem_counters"]["a"]["eth1"].update(dropped=900, sent_pkts=299_100))
        self.assertOnly(run, "netem")

    def test_netem_missing_on_a_data_nic(self):
        run = Run(self)
        run.edit(lambda m: m["arm"] == "mio-tcp", lambda m: m["netem_counters"]["b"]["eth2"].update(qdisc="none"))
        self.assertOnly(run, "netem")

    def test_stray_netem_on_a_no_loss_row(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(netem_counters={"a": {"eth1": {"qdisc": "netem", "loss_pct": "1",
                                                                       "sent_pkts": 10, "dropped": 0}}}))
        self.assertOnly(run, "netem")

    def test_no_qdisc_control_unverified(self):
        run = Run(self)
        run.rows(lambda r: r["label"] == "2a:mio-udp" and r["rep"] == "1",
                 lambda r: r.__setitem__(2, "2a:mio-udp@NETEM=none"))
        for old in os.listdir(run.path("2a")):
            if "-2a-mio-udp-64-1-" in old:
                os.rename(run.path("2a", old), run.path("2a", old.replace("mio-udp-", "mio-udp@NETEM=none-")))
        run.edit(lambda m: m["label"] == "2a:mio-udp" and m["rep"] == 1 and m["streams"] == 1,
                 lambda m: m.update(label="2a:mio-udp@NETEM=none", netem="none"))
        self.assertIn("netem", run.rules())
        self.assertIn("no tc -s counters on a to show no qdisc", "\n".join(map(str, fairness.check(run.run))))

    def test_dpdk_in_a_loss_row(self):
        run = Run(self)
        run.edit(lambda m: m["set"] == "3b" and m["arm"] == "aeron-kernel",
                 lambda m: m.update(transport="aeron-dpdk"), first=False)
        self.assertIn("netem", run.rules())

    def test_vm_pair_changes_within_a_set(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(vms={"a": "vm-a1", "b": "vm-b9"}))
        self.assertOnly(run, "vm-pair")

    def test_cli_exits_nonzero_on_violations(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(core_build="core-ffff"))
        got = subprocess.run([sys.executable, os.path.join(CLOUD, "fairness.py"), run.run], capture_output=True,
                             text=True)
        self.assertEqual(got.returncode, 1)
        self.assertIn("build", got.stdout)
        clean = subprocess.run([sys.executable, os.path.join(CLOUD, "fairness.py"), os.path.join(CLEAN, "run")],
                               capture_output=True, text=True)
        self.assertEqual(clean.returncode, 0, clean.stdout + clean.stderr)


class SpikesLabelTest(unittest.TestCase):
    """One point, its max at a known time, one source of evidence at a time."""
    START = fx.BASE_S + 20
    DUE = 6.0
    P50 = 20_000
    MAX = 200_000

    def setUp(self):
        self.dir = tempfile.mkdtemp(prefix="transport-spikes-")
        self.addCleanup(shutil.rmtree, self.dir, True)
        self.folder = os.path.join(self.dir, "2a")
        os.makedirs(self.folder)
        self.tag = "2a-mio-udp-64-1-r10000"
        self.t0 = int((self.START + self.DUE) * 1e9)
        self.mid = self.t0 + self.MAX // 2
        self.write_spikes()

    def write_spikes(self, due=None, with_max=True):
        due_ns = int((self.DUE if due is None else due) * 1e9)
        with open(os.path.join(self.folder, f"spikes-{self.tag}.csv"), "w") as f:
            f.write(f"# start_realtime_ns={self.START * 10**9}\n# spikes_total=1\n")
            if with_max:
                f.write(f"# max={due_ns},{self.MAX},0\n")
            f.write(f"stream,due_ns,latency_ns\n0,{due_ns - 10**9},{self.MAX // 2}\n")

    def events(self, host, *lines):
        with open(os.path.join(self.folder, f"{host}-events-{self.tag}.csv"), "w") as f:
            f.write("kind,start_ns,end_ns,cpu,detail\n" + "".join(l + "\n" for l in lines))

    def cause(self, achieved=10_000, late=0, transport="udp"):
        row = dict.fromkeys(COLUMNS)
        row.update(p50=self.P50, max=self.MAX, rate=10_000, achieved=achieved, scheduled=300_000, late=late,
                   samples=300_000)
        manifest = {"transport": transport, "cpus": {h: {"housekeeping": [0], "irq": [2], "hot": [4, 6]}
                                                     for h in "ab"}}
        point = spikes.TransportPoint(set="2a", tag=self.tag, row=row, manifest=manifest, hosts=("a", "b"))
        return spikes.transport_cause(self.folder, point)

    def test_host_pause_from_the_gap_spinner(self):
        with open(os.path.join(self.folder, "b-diag-2a.log"), "w") as f:
            f.write(f"== gapspin\n{(self.mid + 80_000) / 1e9:.6f} 160\n")
        self.assertTrue(self.cause().startswith("host pause (gap spinner 160"), self.cause())

    def test_a_spinner_gap_too_short_explains_nothing(self):
        with open(os.path.join(self.folder, "b-diag-2a.log"), "w") as f:
            f.write(f"== gapspin\n{(self.mid + 10_000) / 1e9:.6f} 20\n")
        self.assertTrue(self.cause().startswith("unexplained"))

    def test_per_minute_cluster(self):
        self.write_spikes(due=30.0)  # :50 of the minute
        self.assertTrue(self.cause().startswith("per-minute stall"), self.cause())

    def test_outside_the_per_minute_window(self):
        self.write_spikes(due=33.0)  # :53
        self.assertTrue(self.cause().startswith("unexplained"))

    def test_irq_on_a_hot_cpu(self):
        self.events("a", f"irq,{self.mid - 50_000},{self.mid + 50_000},4,mana_q1")
        self.assertTrue(self.cause().startswith("IRQ mana_q1 on hot CPU 4"), self.cause())

    def test_irq_on_a_housekeeping_cpu_explains_nothing(self):
        self.events("a", f"irq,{self.mid - 50_000},{self.mid + 50_000},2,mana_q1")
        self.assertTrue(self.cause().startswith("unexplained"))

    def test_irq_from_the_trace(self):
        up = 1000.0  # the trace clock at the UPTIME line, CLOCK_REALTIME there: the max's time
        wall = self.mid / 1e9
        with open(os.path.join(self.folder, "a-diag-2a.log"), "w") as f:
            f.write("== trace\n")
            f.write(f"UPTIME {up:.6f} REALTIME {wall:.9f}\n")
            f.write(f"  <idle>-0 [006] d.h1. {up - 0.00005:.6f}: irq_handler_entry: irq=45 name=mana_q2\n")
            f.write(f"  <idle>-0 [006] d.h1. {up + 0.00005:.6f}: irq_handler_exit: irq=45 ret=handled\n")
        self.assertTrue(self.cause().startswith("IRQ mana_q2 on hot CPU 6"), self.cause())

    def test_softirq(self):
        self.events("b", f"softirq,{self.mid - 50_000},{self.mid + 50_000},6,NET_RX")
        self.assertTrue(self.cause().startswith("softirq NET_RX"), self.cause())

    def test_nak(self):
        self.events("b", f"nak,{self.mid},{self.mid},-,2")
        self.assertTrue(self.cause(transport="aeron-udp").startswith("NAK/retransmit"), self.cause(transport="aeron-udp"))

    def test_retransmit(self):
        self.events("a", f"retransmit,{self.mid},{self.mid},-,1")
        self.assertTrue(self.cause(transport="aeron-udp").startswith("NAK/retransmit"),
                        self.cause(transport="aeron-udp"))

    def test_a_nak_explains_no_socket_arm(self):
        # the media driver's counters are no UDP socket's
        self.events("a", f"nak,{self.mid},{self.mid},-,2")
        self.assertTrue(self.cause().startswith("unexplained"), self.cause())

    def test_busy_thread_preempted(self):
        self.events("a", f"preempt,{self.mid - 50_000},{self.mid + 50_000},4,\"ping:101 preempted, ran kworker/4:1\"")
        self.assertTrue(self.cause().startswith("busy thread off its CPU 100 µs (a cpu 4: ping:101"), self.cause())

    def test_a_spinner_preempted_explains_nothing(self):
        self.events("a", f"preempt,{self.mid - 50_000},{self.mid + 50_000},6,\"gapspin:77 preempted, ran kworker/6:1\"")
        self.assertTrue(self.cause().startswith("unexplained"), self.cause())

    def test_ipi(self):
        self.events("b", f"ipi,{self.mid - 50_000},{self.mid + 50_000},6,call_function")
        self.assertTrue(self.cause().startswith("IPI call_function on hot CPU 6 (b"), self.cause())

    def test_tcp_retransmit_matched_to_a_tc_drop(self):
        sec = self.mid // 10**9 * 10**9
        self.events("a", f"tcp-retrans,{self.mid},{self.mid},-,3", f"tc-drop,{sec},{sec + 10**9},-,2")
        self.assertTrue(self.cause(transport="tcp").startswith("TCP retransmit after a netem drop"),
                        self.cause(transport="tcp"))

    def test_a_stall_names_why_the_max_is_unexplained(self):
        self.events("a", f"stall,{self.mid - 60_000},{self.mid + 60_000},4,ping receive loop 120.0 µs")
        self.events("b")
        got = self.cause()
        self.assertTrue(got.startswith("unexplained (the ping receive loop 120.0 µs on a cpu 4 with no traced event"),
                        got)

    def test_what_was_not_recorded_is_said(self):
        with open(os.path.join(self.folder, f"a-events-{self.tag}.csv"), "w") as f:
            f.write("# note: trace unavailable: no tracefs at /sys/kernel/tracing\nkind,start_ns,end_ns,cpu,detail\n")
        self.assertEqual(self.cause(), "unexplained (a: trace unavailable: no tracefs at /sys/kernel/tracing; "
                         "b: no events file)")

    def test_tcp_rto_matched_to_a_tc_drop_in_its_second(self):
        sec = self.mid // 10**9 * 10**9
        self.events("a", f"tcp-rto,{self.mid},{self.mid},-,1")
        self.events("b", f"tc-drop,{sec},{sec + 10**9},-,1")
        self.assertTrue(self.cause(transport="tcp").startswith("TCP RTO after a netem drop"),
                        self.cause(transport="tcp"))

    def test_tcp_tlp_matched(self):
        sec = self.mid // 10**9 * 10**9
        self.events("a", f"tcp-tlp,{self.mid},{self.mid},-,1", f"tc-drop,{sec},{sec + 10**9},-,2")
        self.assertTrue(self.cause(transport="tcp").startswith("TCP TLP after a netem drop"))

    def test_tcp_rto_after_a_tc_drop_in_the_second_before(self):
        # the timer fires at least rto_min (or a probe timeout) after the
        # drop it recovers: across a second boundary as often as not
        sec = self.mid // 10**9 * 10**9
        self.events("a", f"tcp-rto,{self.mid},{self.mid},-,1")
        self.events("b", f"tc-drop,{sec - 10**9},{sec},-,1")
        self.assertTrue(self.cause(transport="tcp").startswith("TCP RTO after a netem drop"), self.cause(transport="tcp"))

    def test_tcp_rto_without_a_tc_drop_is_unexplained(self):
        sec = self.mid // 10**9 * 10**9
        self.events("a", f"tcp-rto,{self.mid},{self.mid},-,1")
        self.events("b", f"tc-drop,{sec - 2 * 10**9},{sec - 10**9},-,1")  # two seconds before
        self.assertTrue(self.cause(transport="tcp").startswith("unexplained (TCP RTO"), self.cause(transport="tcp"))

    def test_dpdk_refill_from_the_event_log(self):
        with open(os.path.join(self.folder, f"a-rings-{self.tag}.log"), "w") as f:
            f.write("# kind,tsc,realtime_ns,value_ns,port,agent\n")
            f.write(f"refill,1,{self.mid + 50_000},100000,0,77\n")
        self.assertTrue(self.cause(transport="aeron-dpdk").startswith("DPDK refill"))

    def test_dpdk_refill_from_events(self):
        self.events("b", f"dpdk-refill,{self.mid - 50_000},{self.mid + 50_000},4,256")
        self.assertTrue(self.cause(transport="aeron-dpdk").startswith("DPDK refill"))

    def test_page_fault(self):
        self.events("a", f"page-fault,{self.mid - 50_000},{self.mid + 50_000},4,3")
        self.assertTrue(self.cause().startswith("page fault (a cpu 4"), self.cause())

    def test_host_pause_comes_before_an_irq(self):
        with open(os.path.join(self.folder, "a-diag-2a.log"), "w") as f:
            f.write(f"== gapspin\n{(self.mid + 80_000) / 1e9:.6f} 160\n")
        self.events("a", f"irq,{self.mid - 50_000},{self.mid + 50_000},4,mana_q1")
        self.assertTrue(self.cause().startswith("host pause"))

    def test_over_capacity_is_queueing(self):
        self.assertEqual(self.cause(achieved=9_000), "past capacity (queueing)")

    def test_sender_late_is_queueing(self):
        self.assertEqual(self.cause(late=6_000), "sender late (queueing)")

    def test_nothing_is_unexplained(self):
        self.events("a")
        self.events("b")
        self.assertEqual(self.cause(), "unexplained (no recorded event overlaps it on either host)")

    def test_max_not_located(self):
        self.write_spikes(with_max=False)
        self.assertTrue(self.cause().startswith("unexplained (max not located"))

    def test_largest_row_is_the_max_without_the_max_line(self):
        with open(os.path.join(self.folder, f"spikes-{self.tag}.csv"), "w") as f:
            f.write(f"# start_realtime_ns={self.START * 10**9}\nstream,due_ns,latency_ns\n"
                    f"0,{int(self.DUE * 1e9)},{self.MAX}\n")
        self.events("a", f"page-fault,{self.mid - 50_000},{self.mid + 50_000},4,3")
        self.assertTrue(self.cause().startswith("page fault"))

    def test_cli_lists_every_max(self):
        got = subprocess.run([sys.executable, os.path.join(CLOUD, "spikes.py"), "--transport",
                              os.path.join(CLEAN, "run")], capture_output=True, text=True)
        self.assertEqual(got.returncode, 0, got.stderr)
        self.assertIn("host pause", got.stdout)
        self.assertIn("unexplained: none", got.stdout)


A2 = os.path.join(HERE, "fixture", "transport-a2")


class A2SessionTest(unittest.TestCase):
    """Cases cut from a real Azure session (20261026-120451-transport-a, sets
    fan and 3a): each point's spikes header and max, its events files
    trimmed to their notes, each CPU's first trace rows and the rows around
    the max, and the manifest fields the labels read; values as recorded."""

    def point(self, tag):
        with open(os.path.join(A2, "fan", f"point-{tag}.json")) as f:
            p = json.load(f)
        return spikes.TransportPoint(set=p["set"], tag=tag, row=p["row"], manifest=p["manifest"], hosts=("a", "b"))

    def copy(self, *tags):
        """The fan fixture's files of tags in a folder of their own, to edit."""
        d = tempfile.mkdtemp(prefix="transport-a2-")
        self.addCleanup(shutil.rmtree, d, True)
        folder = os.path.join(d, "fan")
        os.makedirs(folder)
        for name in os.listdir(os.path.join(A2, "fan")):
            if any(t in name for t in tags):
                shutil.copyfile(os.path.join(A2, "fan", name), os.path.join(folder, name))
        return folder

    def test_an_overrun_before_the_max_leaves_it_traced(self):
        # both hosts' CPU 2 overran, but its trace on each runs from long
        # before the max (a: 0.9 s into the 41 s; b: 28 s): the overrun lost
        # warm-up events only, so it is not what left the max unexplained
        tag = "fan-aa@STREAMS=128-64-1-r10000"
        self.assertEqual(spikes.transport_cause(os.path.join(A2, "fan"), self.point(tag)),
                         "unexplained (no recorded event overlaps it on either host)")

    def test_an_overrun_after_the_max_is_said(self):
        tag = "fan-aa@STREAMS=128-64-1-r10000"
        folder = self.copy(tag)
        path = os.path.join(folder, f"spikes-{tag}.csv")
        # the max moved to half a second into the run: before host a's first
        # surviving trace row on CPU 2 (0.88 s), after nothing on b's (28 s)
        text = read(path).replace("# max=41148699978,", "# max=500000000,").replace(",41148699978,", ",500000000,")
        write(path, text)
        self.assertEqual(spikes.transport_cause(folder, self.point(tag)),
                         "unexplained (a: trace overran on cpu2: 29884 events lost (its buffer full): an event there "
                         "may be missing; b: trace overran on cpu2: 954962 events lost (its buffer full): an event "
                         "there may be missing)")

    def test_a_host_tcp_counter_without_the_points_own_retransmits_is_another_connections(self):
        # a's TCP counters grew in the max's second (a retransmit, a DSACK,
        # two probes), but the point's connections retransmitted nothing on
        # either host in the window (tcp_info): the counters are the host's,
        # the runner's SSH session among them, so they explain nothing
        tag = "fan-mio-tcp@STREAMS=8-64-2-r10000"
        self.assertEqual(spikes.transport_cause(os.path.join(A2, "fan"), self.point(tag)),
                         "unexplained (no recorded event overlaps it on either host; a TCP counter grew in that second "
                         "on a, but the point's connections there retransmitted nothing in the window: another "
                         "connection's)")

    def test_a_retransmit_of_the_points_own_with_no_netem_is_a_loss_elsewhere(self):
        # a's probes in the max's second, and a's own connection retransmitted
        # one segment the peer did not report as a duplicate: a real loss with
        # no netem on the row, recovered by a probe ~12 ms later (2 x SRTT
        # rounded up to jiffies plus two jiffies at HZ=250)
        tag = "fan-aa@STREAMS=32-64-2-r50000"
        got = spikes.transport_cause(os.path.join(A2, "fan"), self.point(tag))
        self.assertEqual(got, "TCP loss recovery (a: TLP x4 in that second; the point's connections there "
                              "retransmitted 1 in the window, 1 not DSACK-reported; no netem drop: a loss on the path)")

    def test_path_loss_on_a_row_without_netem_is_a_note(self):
        # a real 3a point (mio-tcp, 50k msg/s, rep 1, no netem): each end's
        # connection retransmitted one segment the peer never reported a
        # duplicate (each sender fast-retransmitted after SACKs, each
        # receiver queued out of order: a hole), and no guest drop counter
        # grew on either host: lost below the guest's stack
        run = Run(self)
        real = {"a": {"tcp_info": {"dsack_dups": 0, "lost_end": 0, "retrans": 1, "retrans_not_dsack": 1, "rtos": 0},
                      "nstat": {"TcpExtTCPSackRecovery": 1, "TcpExtTCPFastRetrans": 1, "TcpExtTCPOFOQueue": 9,
                                "TcpRetransSegs": 10}},
                "b": {"tcp_info": {"dsack_dups": 0, "lost_end": 0, "retrans": 1, "retrans_not_dsack": 1, "rtos": 0},
                      "nstat": {"TcpExtTCPSackRecovery": 1, "TcpExtTCPFastRetrans": 1, "TcpExtTCPOFOQueue": 6,
                                "TcpRetransSegs": 1}}}
        mio = lambda m: m["arm"] == "mio-tcp" and m["rep"] == 1
        run.edit(mio, lambda m: m.update(loss_counters=real, netem=None, netem_counters=None))
        run.rows(lambda r: r["label"] == "3b:mio-tcp@NETEM=0.1" and r["rep"] == "1",
                 lambda r: r.__setitem__(2, "3b:mio-tcp"))
        os.rename(run.path("3b", "manifest-3b-mio-tcp@NETEM=0.1-64-1-r10000.json"),
                  run.path("3b", "manifest-3b-mio-tcp-64-1-r10000.json"))
        run.edit(mio, lambda m: m.update(label="3b:mio-tcp"))
        notes = []
        found = [v for v in fairness.check(run.run, notes=notes) if v.rule in ("loss", "netem")]
        self.assertEqual([v.rule for v in found if v.rule == "loss"], [])
        self.assertEqual([str(n) for n in notes if n.rule == "loss"], [
            "note: loss: 3b: 3b-mio-tcp-64-1-r10000: 2 segments lost below the guest's TCP stack (the NIC or the "
            "network: no guest drop counter grew on either host; retransmits not DSACK-reported: a 1, b 1), 0 "
            "spurious, on a row without netem"])

    def test_failed_arm_absent_from_its_rounds(self):
        # 3a's dpdk-aeron-rel failed in every rep (failures.csv as the runner
        # wrote it): its rounds' orders name it, no row has it
        run = Run(self)
        shutil.copyfile(os.path.join(A2, "failures.csv"), run.path("failures.csv"))
        with open(run.path("failures.csv"), "a") as f:
            f.write("2a,1,-,aeron-x,0,1\n2a,2,-,aeron-x,0,setup\n2a,3,-,aeron-x,0,1\n")
        run.edit(lambda m: m["set"] == "2a" and m["streams"] == 1,
                 lambda m: m.update(arm_order=m["arm_order"] + ["aeron-x"]), first=False)
        notes = []
        self.assertEqual(fairness.check(run.run, absences=notes), [])
        self.assertEqual(sorted({(a.set, a.rep, a.streams, a.arm) for a in notes}),
                         [("2a", 1, 1, "aeron-x"), ("2a", 2, 1, "aeron-x"), ("2a", 3, 1, "aeron-x")])
        self.assertIn("failures.csv", notes[0].detail)

    def test_an_arm_absent_without_a_recorded_failure_breaks_the_order(self):
        run = Run(self)
        shutil.copyfile(os.path.join(A2, "failures.csv"), run.path("failures.csv"))
        with open(run.path("failures.csv"), "a") as f:
            f.write("2a,1,-,aeron-x,0,8\n")  # another stream count's
        run.edit(lambda m: m["round"] == "2a-s1-n--r10000-rep1",
                 lambda m: m.update(arm_order=m["arm_order"] + ["aeron-x"]), first=False)
        self.assertEqual(run.rules(), ["order"])

    def test_analysis_says_which_arm_is_absent(self):
        run = Run(self)
        with open(run.path("failures.csv"), "w") as f:
            f.write("2a,1,-,aeron-x,0,1\n2a,2,-,aeron-x,0,1\n2a,3,-,aeron-x,0,1\n")
        run.edit(lambda m: m["set"] == "2a" and m["streams"] == 1,
                 lambda m: m.update(arm_order=m["arm_order"] + ["aeron-x"]), first=False)
        out = io.StringIO()
        with redirect_stdout(out):
            analyse.transport([run.run])
        s1 = out.getvalue().split("### Set 2a, 1 stream, 10,000 msg/s")[1].split("\n### ")[0]
        self.assertIn("Absent: aeron-x (failed in reps 1, 2, 3: failures.csv): left out of this comparison.", s1)
        self.assertNotIn("Refused", s1)

    def test_irq_core_saturated_is_a_flag_not_a_placement_fault(self):
        # fan-eio-tcp, 128 connections at 200k msg/s: host a's ksoftirqd/1 at
        # 102 % on CPU 1, the IRQ CPU (its softirq work handed to its thread)
        with open(os.path.join(A2, "placement-fan-eio-tcp@STREAMS=128-64-1-r200000.json")) as f:
            real = json.load(f)
        run = Run(self)
        irq_cpu = 2  # the fixture's IRQ CPU; the real one was 1
        sat = dict(next(t for t in real["threads"]["a"][0] if t["comm"] == "ksoftirqd/1"),
                   comm=f"ksoftirqd/{irq_cpu}", cpu=irq_cpu)
        run.edit(ONE, lambda m: [s.append(dict(sat)) for s in m["threads"]["a"]])
        self.assertEqual(run.rules(), [])
        p = next(p for p in read_points(run.run) if p.manifest and ONE(p.manifest))
        from transport_points import flags
        self.assertIn("IRQ core saturated", flags(p))

    def test_ksoftirqd_on_a_hot_cpu_is_still_a_placement_fault(self):
        run = Run(self)
        run.edit(ONE, lambda m: m["threads"]["a"][0].append({"comm": "ksoftirqd/6", "cpu": 6, "tid": 27, "util": 1.02}))
        self.assertEqual(run.rules(), ["placement"])

    def test_a_binarys_own_warning_breaks_hostcheck(self):
        # ergon-io's checks at start and through the run: N2 (interrupts
        # deferred for busy polling that nothing does) and T19 (the loop
        # thread's page faults), from a real eio-tcp point's log
        run = Run(self)
        os.makedirs(run.path("logs"))
        shutil.copyfile(os.path.join(A2, "eio-tcp.out"), run.path("logs", "2a-r1-eio-udp-10000.out"))
        run.edit(lambda m: m["set"] == "2a" and m["arm"] == "eio-udp" and m["streams"] == 1 and m["rep"] == 1,
                 lambda m: m.update(runner={"point": "2a-r1-eio-udp-10000"}))
        found = [v for v in fairness.check(run.run) if v.rule == "hostcheck"]
        self.assertEqual([str(v) for v in found], [
            "hostcheck: 2a: 2a-eio-udp-64-1-r10000: a eio-udp's own check N2 warned 1x: enP44106s2: interrupts are "
            "deferred (napi_defer_hard_irqs 2, gro_flush_timeout 200000 ns) but no socket on it busy-polls: packets "
            "wait for the 200 µs timer",
            # T19's first covers the start (1085 faults: set-up and warm-up), left out
            "hostcheck: 2a: 2a-eio-udp-64-1-r10000: a eio-udp's own check T19 warned 1x: the loop thread "
            "(/proc/thread-self/stat) took 5 minor and 0 major page faults since the last check (minflt 1108 → "
            "1113, majflt 0 → 0): each stops the thread while the kernel maps the page in, and a major one waits for "
            "I/O too (not measured here)"])


class AnalyseTest(unittest.TestCase):
    def analyse(self, *args):
        out = io.StringIO()
        with redirect_stdout(out):
            analyse.transport(list(args))
        return out.getvalue()

    def test_expected_markdown(self):
        got = self.analyse(os.path.join(CLEAN, "run"))
        path = os.path.join(HERE, "expected-transport.md")
        if os.environ.get("UPDATE_EXPECTED"):
            write(path, got)
        self.maxDiff = None
        self.assertEqual(read(path), got)

    def test_cli_matches(self):
        got = subprocess.run([sys.executable, os.path.join(CLOUD, "analyse.py"), "--transport",
                              os.path.join(CLEAN, "run")], capture_output=True, text=True)
        self.assertEqual(got.returncode, 0, got.stderr)
        self.assertEqual(got.stdout, self.analyse(os.path.join(CLEAN, "run")))

    def test_refuses_a_round_with_a_violation(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(core_build="core-ffff"))
        got = self.analyse(run.run)
        section = got.split("### Set 2a, 1 stream, 10,000 msg/s")[1].split("\n### ")[0]
        self.assertIn("Refused", section)
        self.assertIn("build", section)
        self.assertNotIn("| arm ", section)
        # the other groups still compare
        other = got.split("### Set 2a, 8 streams, 10,000 msg/s")[1].split("\n### ")[0]
        self.assertIn("| arm ", other)

    def test_show_invalid_prints_it_marked(self):
        run = Run(self)
        run.edit(ONE, lambda m: m.update(core_build="core-ffff"))
        got = self.analyse("--show-invalid", run.run)
        section = got.split("### Set 2a, 1 stream, 10,000 msg/s")[1].split("\n### ")[0]
        self.assertIn("Invalid", section)
        self.assertIn("| arm ", section)

    def test_verdicts(self):
        got = self.analyse(os.path.join(CLEAN, "run"))
        s1 = got.split("### Set 2a, 1 stream, 10,000 msg/s")[1].split("\n### ")[0]
        # 40 ns apart, the A/A spread 60 ns: not separated
        self.assertRegex(s1, r"\| eio-udp vs mio-udp +\| not separated \(within A/A 0\.05\)")
        # 4 µs apart at the min, 5 µs at the p50: separated
        self.assertRegex(s1, r"\| aeron-kernel vs mio-udp +\| mio-udp lower by 4\.00 +\| mio-udp lower by 5\.00")
        s8 = got.split("### Set 2a, 8 streams, 10,000 msg/s")[1].split("\n### ")[0]
        # eio-udp late in rep 2 and mio-udp over capacity in rep 3: shown, never compared
        self.assertIn("sender late", s8)
        self.assertIn("over capacity", s8)
        self.assertNotIn("vs eio-udp", s8)
        self.assertNotIn("eio-udp vs", s8)
        self.assertNotIn("mio-udp vs", s8)
        self.assertNotIn("vs mio-udp", s8)

    def test_apart_by_more_than_the_spread_but_ranges_overlap(self):
        run = Run(self)
        p99 = {"1": "22700", "2": "22760", "3": "23000"}
        run.rows(lambda r: r["label"] == "2a:eio-udp", lambda r: r.__setitem__(9, p99[r[0]]))
        got = self.analyse(run.run)
        s1 = got.split("### Set 2a, 1 stream, 10,000 msg/s")[1].split("\n### ")[0]
        line = next(l for l in s1.splitlines() if l.startswith("| eio-udp vs mio-udp"))
        self.assertEqual(line.split("|")[4].strip(), "not separated (ranges overlap)")

    def test_unexplained_max_blocks_the_max_verdict(self):
        run = Run(self)
        os.remove(run.path("2a", "a-events-2a-mio-udp-64-1-r10000.csv"))
        got = self.analyse("--show-invalid", run.run)
        s1 = got.split("### Set 2a, 1 stream, 10,000 msg/s")[1].split("\n### ")[0]
        self.assertIn("blocked: unexplained max", s1)

    def test_stream_count_table(self):
        got = self.analyse(os.path.join(CLEAN, "run"))
        self.assertIn("### Set 2a by stream count", got)


class EventsTest(unittest.TestCase):
    """transport_events.py: vm.sh events-stop's log as the events file."""
    # the trace's clock (mono, s) 100.0 is CLOCK_REALTIME 1,800,000,000 s
    RT = 1_800_000_000 * 10**9

    def log(self, trace=(), gaps=(), samples=(), notes=("events: trace on cpus 4,6 (test)",)):
        d = tempfile.mkdtemp(prefix="transport-events-")
        self.addCleanup(shutil.rmtree, d, True)
        path = os.path.join(d, "p-a.log")
        text = ["== clock", f"{100 * 10**9} {self.RT}", f"{200 * 10**9} {self.RT + 100 * 10**9}", "== notes", *notes]
        text += ["== gapspin 6", *gaps, "== trace", *trace, "== osnoise", "== samples", *samples]
        write(path, "\n".join(text) + "\n")
        return path

    def at(self, s):
        return self.RT + int(round((s - 100) * 1e9))

    def test_busy_thread_irq_softirq_ipi_preemption_and_page_faults(self):
        import transport_events as te
        trace = [
            "  <idle>-0 [004] d..2. 100.000000: sched_switch: prev_comm=swapper/4 prev_pid=0 prev_prio=120 prev_state=R ==> next_comm=udp_lat next_pid=77 next_prio=120",
            "  udp_lat-77 [004] d.h1. 101.000000: irq_handler_entry: irq=24 name=mana_q0",
            "  udp_lat-77 [004] d.h1. 101.000040: irq_handler_exit: irq=24 ret=handled",
            "  udp_lat-77 [004] ..s1. 101.000041: softirq_entry: vec=3 [action=NET_RX]",
            "  udp_lat-77 [004] ..s1. 101.000071: softirq_exit: vec=3 [action=NET_RX]",
            "  udp_lat-77 [004] d.h1. 102.000000: call_function_entry: vector=251",
            "  udp_lat-77 [004] d.h1. 102.000005: call_function_exit: vector=251",
            "  udp_lat-77 [004] d.... 103.000000: page_fault_user: address=0x7f00 ip=0x55 error_code=0x6",
            "  udp_lat-77 [004] d..2. 104.000000: sched_switch: prev_comm=udp_lat prev_pid=77 prev_prio=120 prev_state=R+ ==> next_comm=kworker/4:1 next_pid=9 next_prio=120",
            "  kworker/4:1-9 [004] d..2. 104.000300: sched_switch: prev_comm=kworker/4:1 prev_pid=9 prev_prio=120 prev_state=I ==> next_comm=udp_lat next_pid=77 next_prio=120",
            # another task's fault on CPU 6, which no busy thread owns: dropped
            "  bash-5 [006] d.... 105.000000: page_fault_user: address=0x1 ip=0x2 error_code=0x6",
            "  udp_lat-77 [004] d..2. 199.000000: sched_switch: prev_comm=udp_lat prev_pid=77 prev_prio=120 prev_state=S ==> next_comm=swapper/4 next_pid=0 next_prio=120",
            "# overrun cpu4 12",
        ]
        rows, notes = te.events(self.log(trace=trace))
        got = {(k, cpu, d) for k, a, b, cpu, d in rows}
        self.assertIn(("irq", 4, "mana_q0"), got)
        self.assertIn(("softirq", 4, "NET_RX"), got)
        self.assertIn(("ipi", 4, "call_function"), got)
        self.assertIn(("page-fault", 4, "user 0x7f00"), got)
        self.assertIn(("preempt", 4, "udp_lat:77 preempted, ran kworker/4:1"), got)
        self.assertFalse([r for r in rows if r[3] == 6], rows)
        irq = next(r for r in rows if r[0] == "irq")
        self.assertEqual((irq[1], irq[2]), (self.at(101.0), self.at(101.00004)))
        pre = next(r for r in rows if r[0] == "preempt")
        self.assertEqual(pre[2] - pre[1], 300_000)
        self.assertTrue(any("overran on cpu4: 12 events lost" in n for n in notes), notes)

    def test_a_spinner_gap_the_trace_explains_is_no_host_pause(self):
        import transport_events as te
        trace = [
            "  <idle>-0 [006] d..2. 100.000000: sched_switch: prev_comm=swapper/6 prev_pid=0 prev_prio=120 prev_state=R ==> next_comm=gapspin next_pid=50 next_prio=120",
            "  gapspin-50 [006] d.h1. 110.000000: irq_handler_entry: irq=11 name=arch_timer",
            "  gapspin-50 [006] d.h1. 110.000090: irq_handler_exit: irq=11 ret=handled",
            "  gapspin-50 [006] d..2. 199.000000: sched_switch: prev_comm=gapspin prev_pid=50 prev_prio=120 prev_state=S ==> next_comm=swapper/6 next_pid=0 next_prio=120",
        ]
        # a 100 us gap the IRQ covers 90 us of, and one at 120 s nothing does
        gaps = [f"{1_800_000_010.000095:.6f} 100.0", f"{1_800_000_020.000100:.6f} 100.0"]
        rows, _ = te.events(self.log(trace=trace, gaps=gaps))
        pauses = [r for r in rows if r[0] == "host-pause"]
        self.assertEqual(len(pauses), 1, pauses)
        self.assertEqual(pauses[0][3], 6)
        self.assertTrue(pauses[0][4].startswith("100.0 µs of a 100.0 µs spinner gap untraced"), pauses)
        # the spinner's CPU's own IRQ is no busy thread's: not an event
        self.assertFalse([r for r in rows if r[0] == "irq"], rows)

    def test_per_second_counters(self):
        import transport_events as te
        t0 = self.RT
        samples = [f"t={t0} tc=eth1:100:0,eth2:50:0 tcp=RetransSegs=5;TCPTimeouts=1;TCPLossProbes=0;TCPDSACKRecv=0;TCPDSACKOfoRecv=0 aeron=naks-sent=3;naks-received=0;retransmits-sent=0",
                   f"t={t0 + 10**9} tc=eth1:200:2,eth2:90:1 tcp=RetransSegs=9;TCPTimeouts=2;TCPLossProbes=1;TCPDSACKRecv=1;TCPDSACKOfoRecv=0 aeron=naks-sent=5;naks-received=1;retransmits-sent=2"]
        rows, _ = te.events(self.log(samples=samples))
        got = {r[0]: r[4] for r in rows}
        self.assertEqual(got, {"tc-drop": "3", "tcp-retrans": "4", "tcp-rto": "1", "tcp-tlp": "1", "tcp-dsack": "1",
                               "nak": "3", "retransmit": "2"})
        self.assertTrue(all((r[1], r[2], r[3]) == (t0, t0 + 10**9, None) for r in rows))

    def test_stalls_and_what_is_missing(self):
        import transport_events as te
        d = tempfile.mkdtemp(prefix="transport-events-")
        self.addCleanup(shutil.rmtree, d, True)
        stalls = os.path.join(d, "stalls-x.csv")
        write(stalls, f"# start_realtime_ns={self.RT}\nend_ns,gap_ns\n2000000,150000\n")
        rows, notes = te.events(os.path.join(d, "missing.log.gz"), (stalls, 4, "ping receive"))
        self.assertEqual(rows, [("stall", self.RT + 1_850_000, self.RT + 2_000_000, 4, "ping receive loop 150.0 µs")])
        self.assertTrue(notes[0].startswith("events unavailable: no events log"), notes)
        rows, notes = te.events(self.log(notes=("events: trace unavailable: no tracefs at /sys/kernel/tracing",)))
        self.assertIn("trace unavailable: no tracefs at /sys/kernel/tracing", notes)


class TuneTest(unittest.TestCase):
    """transport_tune.py's rule: better by more than the A/A band, ties keep the current value."""

    def test_better_outside_the_band_and_ties(self):
        import transport_tune as tt
        dim = ("load", 1, "-")
        cur, aa = {dim: [(1, 50_000, 300_000)]}, {dim: [(1, 52_000, 310_000)]}
        self.assertTrue(tt.better({dim: [(1, 40_000, 300_000)]}, cur, aa, ranges=False)[0])
        # within the 2 us band at p99.99, and the max no better outside its 10 us
        self.assertFalse(tt.better({dim: [(1, 49_000, 295_000)]}, cur, aa, ranges=False)[0])
        # p99.99 tied: the max decides
        ok, why = tt.better({dim: [(1, 49_000, 200_000)]}, cur, aa, ranges=False)
        self.assertTrue(ok and why.startswith("max better"), why)
        # better at N=1, worse at N=128: not better
        d2 = ("load", 128, "-")
        two = lambda a, b: {dim: [(1, a, 300_000)], d2: [(1, b, 300_000)]}
        self.assertFalse(tt.better(two(40_000, 90_000), two(50_000, 60_000), two(50_500, 60_500), ranges=False)[0])

    def test_confirmation_needs_ranges_apart(self):
        import transport_tune as tt
        dim = ("load", 1, "-")
        cur = {dim: [(1, 50_000, 1), (2, 51_000, 1), (3, 50_500, 1)]}
        aa = {dim: [(1, 50_200, 1), (2, 50_900, 1), (3, 50_400, 1)]}
        self.assertTrue(tt.better({dim: [(1, 45_000, 1), (2, 46_000, 1), (3, 45_500, 1)]}, cur, aa, ranges=True)[0])
        self.assertFalse(tt.better({dim: [(1, 45_000, 1), (2, 50_100, 1), (3, 45_500, 1)]}, cur, aa, ranges=True)[0])

    def test_two_sessions_merge_into_one_section(self):
        import transport_tune as tt
        d = tempfile.mkdtemp(prefix="transport-tune-")
        self.addCleanup(shutil.rmtree, d, True)
        env = os.path.join(d, "settings.env")
        shutil.copyfile(os.path.join(CLOUD, "transport-settings.env"), env)
        a = [tt.BEGIN + " x ====", "# Run 1", 'TR_TUNED_RUN="1"', 'TR_TUNED_SWEPT="capacity:mpsc"',
             'TR_ARM_KNOBS="mpsc:capacity=1024"', "# changed mpsc capacity: 4096 -> 1024: p99.99 better", tt.END]
        b = [tt.BEGIN + " x ====", "# Run 2", 'TR_TUNED_RUN="2"', 'TR_TUNED_SWEPT="congestion:eio-tcp,mio-tcp"',
             'TR_ARM_KNOBS=""', "TR_TCP_CONGESTION=bbr", "# changed congestion (the hosts'): bbr: every arm", tt.END]
        for n, sec in enumerate((a, b)):
            write(os.path.join(d, f"s{n}"), "\n".join(sec) + "\n")
            self.assertEqual(tt.main(["apply", env, os.path.join(d, f"s{n}")]), 0)
        tr = tt.read_settings(env)
        self.assertEqual(tr["TR_TUNED_RUN"], "1 2")
        self.assertEqual(tr["TR_ARM_KNOBS"], "mpsc:capacity=1024")
        self.assertEqual(tr["TR_TUNED_SWEPT"], "capacity:mpsc congestion:eio-tcp,mio-tcp")
        self.assertEqual(tr["TR_TCP_CONGESTION"], "bbr")
        self.assertEqual(read(env).count(tt.BEGIN), 1)
        self.assertIn("# changed mpsc capacity", read(env))

    def test_sweep_plan(self):
        import transport_tune as tt
        d = tempfile.mkdtemp(prefix="transport-tune-")
        self.addCleanup(shutil.rmtree, d, True)
        self.assertEqual(tt.main(["sweep", "--settings", os.path.join(CLOUD, "transport-settings.env"), "--out", d,
                                  "--pair", "b", "--knobs", "rto_min_us,congestion", "--absent", "congestion=bbr",
                                  os.path.join(CLOUD, "transport-matrix.tsv")]), 0)
        sets = [l for l in read(os.path.join(d, "tune-sweep.tsv")).splitlines() if l.startswith("set ")]
        # rto_min_us for both TCP arms (pair b: they are in loss sets), at 1
        # and 128 connections, without and with netem; no bbr to sweep
        self.assertEqual(sets, [
            "set fan.1 tune - ab 50000 1,128 -,0.1 mio-tcp--rto_min_us-8000,mio-tcp--rto_min_us-12000,mio-tcp--rto_min_us-16000,aa",
            "set fan.2 tune - ab 50000 1,128 -,0.1 eio-tcp--rto_min_us-8000,eio-tcp--rto_min_us-12000,eio-tcp--rto_min_us-16000,aa"])


if __name__ == "__main__":
    unittest.main()
