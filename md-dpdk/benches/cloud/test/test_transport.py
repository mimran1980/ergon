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
from transport_points import COLUMNS, read_points  # noqa: E402

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
        run = Run(self)
        env = read(run.path("transport-settings.env"))
        env = env.replace("sweep.socket.busy_poll_us=mio-udp,eio-udp", "sweep.socket.busy_poll_us=mio-udp")
        write(run.path("transport-settings.env"), env)
        self.assertOnly(run, "shared-knob")

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

    def test_tcp_without_tcp_info(self):
        run = Run(self)
        run.edit(lambda m: m["arm"] == "mio-tcp", lambda m: m["loss_counters"]["b"].pop("tcp_info"))
        self.assertOnly(run, "loss-accounting")

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
        self.assertTrue(self.cause().startswith("Azure host pause"), self.cause())

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
        self.assertTrue(self.cause().startswith("NAK/retransmit"), self.cause())

    def test_retransmit(self):
        self.events("a", f"retransmit,{self.mid},{self.mid},-,1")
        self.assertTrue(self.cause().startswith("NAK/retransmit"), self.cause())

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

    def test_tcp_rto_without_a_tc_drop_is_unexplained(self):
        sec = self.mid // 10**9 * 10**9
        self.events("a", f"tcp-rto,{self.mid},{self.mid},-,1")
        self.events("b", f"tc-drop,{sec - 10**9},{sec},-,1")  # the second before
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
        self.assertTrue(self.cause().startswith("page fault (T19"), self.cause())

    def test_host_pause_comes_before_an_irq(self):
        with open(os.path.join(self.folder, "a-diag-2a.log"), "w") as f:
            f.write(f"== gapspin\n{(self.mid + 80_000) / 1e9:.6f} 160\n")
        self.events("a", f"irq,{self.mid - 50_000},{self.mid + 50_000},4,mana_q1")
        self.assertTrue(self.cause().startswith("Azure host pause"))

    def test_over_capacity_is_queueing(self):
        self.assertEqual(self.cause(achieved=9_000), "past capacity (queueing)")

    def test_sender_late_is_queueing(self):
        self.assertEqual(self.cause(late=6_000), "sender late (queueing)")

    def test_nothing_is_unexplained(self):
        self.assertEqual(self.cause(), "unexplained")

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
        self.assertIn("Azure host pause", got.stdout)
        self.assertIn("unexplained: none", got.stdout)


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


if __name__ == "__main__":
    unittest.main()
