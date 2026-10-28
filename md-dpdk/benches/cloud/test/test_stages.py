#!/usr/bin/env python3
"""transport_stages.py against synthetic files: one point's breakdown (each
stage, the wire as what the round trip leaves, flags), its notes (a missing
stage, a resend, a full ring), the A/A overhead check, and a capture's
accounting (NAKs, retransmits, ARP, the unexplained).

python3 test_stages.py [-v]"""
import gzip
import os
import struct
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))

import transport_stages as ts  # noqa: E402

S, E = 1001, 11001
HEADER = "stage,tsc,stream_id,seq,term_offset,value,ring\n"


def read(path):
    with open(path) as f:
        return f.read()


def write(path, text):
    with open(path, "w") as f:
        f.write(text)


def app_file(path, role, rows, start=0, tpn=2.0, dropped=0):
    with open(path, "w") as f:
        f.write("# ergon aeron_lat stage trace v1\n")
        f.write(f"# role={role} every=1 streams=1 rate=1000 warmup_s=1 duration_s=1 ticks_per_ns={tpn} "
                f"start_tsc=0 measure_from_tsc={start} stream_id={S} echo_offset=10000 dropped={dropped}\n")
        f.write(HEADER)
        for r in rows:
            f.write(",".join(map(str, r)) + "\n")


def driver_file(path, rows, gz=True, drops=0, tpn=2.0):
    text = ["# ergon md-dpdk stage trace v1", "# every=1 pid=1 transport=dpdk",
            "# clock open tsc=0 mono_raw_ns=0 realtime_ns=0", HEADER.strip()]
    text += [",".join(map(str, r)) for r in rows]
    if drops:
        text.append(f"# drops ring 0 {drops}")
    text.append(f"# clock close tsc={int(1e9 * tpn)} mono_raw_ns=1000000000 realtime_ns=1")
    data = "\n".join(text) + "\n"
    if gz:
        with gzip.open(path + ".gz", "wt") as f:
            f.write(data)
    else:
        with open(path, "w") as f:
            f.write(data)


def point(d, seqs, hw=False, b_dup=False, drop_rx=None, dropped_b=0):
    """A point whose every message takes the same ticks (2 a ns on each
    host): host a's offer at 10000 + 100000*seq."""
    os.makedirs(os.path.join(d, "0"))
    os.makedirs(os.path.join(d, "pong"))
    a_app, b_app, a_drv, b_drv = [], [], [], []
    for q in seqs:
        t = 10000 + 100000 * q
        a_app += [("app-offer", t, S, q, 0, 0, 0), ("app-offered", t + 200, S, q, 0, 0, 0),
                  ("app-read", t + 60000, E, q, 0, t, 1)]
        a_drv += [("tx-send", t + 2000, S, q, 0, 0, 0), ("tx-call", t + 2400, S, q, 0, 0, 0),
                  ("tx-done", t + 2600, S, q, 0, 0, 0),
                  ("rx-burst", t + 50000, E, q, 0, 0, 1), ("rx-dispatch", t + 50200, E, q, 0, 0, 1),
                  ("rx-inserted", t + 50600, E, q, 0, 0, 1)]
        if hw:
            # the realtime read at the dispatch, 1500 ns after the NIC's arrival
            a_drv.append(("rx-hw", t + 50200, E, q, 0, 1500, 1))
        # host b's counter is another: its own base
        u = 7_000_000 + 100000 * q
        if drop_rx != q:
            b_drv += [("rx-burst", u, S, q, 0, 0, 1)]
        b_drv += [("rx-dispatch", u + 100, S, q, 0, 0, 1), ("rx-inserted", u + 500, S, q, 0, 0, 1),
                  ("tx-send", u + 3000, E, q, 0, 0, 0), ("tx-call", u + 3200, E, q, 0, 0, 0),
                  ("tx-done", u + 3400, E, q, 0, 0, 0)]
        if b_dup:
            b_drv.append(("tx-send", u + 9000, E, q, 0, 0, 0))
        if hw:
            b_drv.append(("rx-hw", u + 100, S, q, 0, 1000, 1))
        b_app += [("app-read", u + 1500, S, q, 0, t, 1), ("app-offer", u + 1700, E, q, 0, 0, 0),
                  ("app-offered", u + 1900, E, q, 0, 0, 0)]
    app_file(os.path.join(d, "0", "stages-app.csv"), "ping", a_app)
    app_file(os.path.join(d, "pong", "stages-app.csv"), "pong", b_app, dropped=dropped_b)
    driver_file(os.path.join(d, "stages-driver-a.csv"), a_drv)
    driver_file(os.path.join(d, "stages-driver-b.csv"), b_drv, gz=False)


def table(bd):
    return {row[0]: row for row in bd.table()}


class Breakdown(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = os.path.join(self.tmp.name, "p")

    def tearDown(self):
        self.tmp.cleanup()

    def test_each_stage_and_the_wire_from_the_hosts_own_clocks(self):
        point(self.d, range(5))
        bd = ts.breakdown_of(*ts.point_files(self.d))
        t = table(bd)
        self.assertEqual(bd.messages, 5)
        want = {
            "a offer call": 100, "a app->sender": 1000, "a sender->tx": 200, "a tx burst": 100,
            "b rx->dispatch": 50, "b insert": 200, "b insert->app": 500, "b turnaround": 100,
            "b offer call": 100, "b app->sender": 650, "b sender->tx": 100, "b tx burst": 100,
            "a rx->dispatch": 100, "a insert": 200, "a insert->app": 4700,
            # a: (2600 + 10000) / 2; b: 3400 / 2; rtt 60000 / 2
            "host a": 6300, "host b": 1700, "wire": 30000 - 6300 - 1700, "round trip": 30000,
        }
        for stage, ns in want.items():
            self.assertEqual(t[stage][1], 5, stage)
            self.assertAlmostEqual(t[stage][2], ns, msg=stage)
            self.assertAlmostEqual(t[stage][4], ns, msg=stage)
        self.assertNotIn("a rx ring wait", t)
        self.assertIn("p50 4700 ns > 2000 ns", t["a insert->app"][6])
        self.assertEqual(t["a app->sender"][6], "")
        self.assertTrue(any("no rx-hw" in n for n in bd.notes()), bd.notes())

    def test_nic_arrival_gives_each_ring_wait(self):
        point(self.d, range(3), hw=True)
        t = table(ts.breakdown_of(*ts.point_files(self.d)))
        # a: the read at 50200 ticks, 1500 ns after arrival (3000 ticks): arrival 47200, burst 50000
        self.assertAlmostEqual(t["a rx ring wait"][2], 1400)
        self.assertAlmostEqual(t["b rx ring wait"][2], 950)
        self.assertAlmostEqual(t["wire less ring waits"][2], 22000 - 1400 - 950)

    def test_missing_stamps_resends_and_drops_are_noted(self):
        point(self.d, range(10), b_dup=True, drop_rx=3, dropped_b=7)
        bd = ts.breakdown_of(*ts.point_files(self.d))
        notes = bd.notes()
        self.assertEqual(bd.messages, 10)
        self.assertEqual(table(bd)["round trip"][1], 9, "the one without b's rx-burst left out")
        self.assertIn("1 of 10 messages without b/out/rx-burst FLAG", notes)
        self.assertTrue(any("10 messages stamped twice at tx-send (a resend: FLAG)" in n for n in notes), notes)
        self.assertTrue(any("host b: 7 stamps dropped" in n for n in notes), notes)

    def test_messages_before_the_window_are_left_out(self):
        point(self.d, range(4))
        p = os.path.join(self.d, "0", "stages-app.csv")
        write(p, read(p).replace("measure_from_tsc=0", "measure_from_tsc=210000"))
        self.assertEqual(ts.breakdown_of(*ts.point_files(self.d)).messages, 2)

    def test_a_large_max_is_named(self):
        point(self.d, range(3))
        p = os.path.join(self.d, "pong", "stages-app.csv")
        lines = read(p).splitlines()
        # message 2's echo offer 150 us after its read
        lines = [l.replace("app-offer,7201700,", "app-offer,7501700,").replace("app-offered,7201900,", "app-offered,7501900,")
                 for l in lines]
        write(p, "\n".join(lines) + "\n")
        t = table(ts.breakdown_of(*ts.point_files(self.d)))
        self.assertIn("max 150.1 us at stream 1001 seq 2", t["b turnaround"][6])


def results(path, rows):
    with open(path, "w") as f:
        for rep, arm, rate, p50, p99 in rows:
            cols = [str(rep), "load", f"t1:{arm}", f"udp-r{rate}", "64", str(rate), "1", str(p50), "0", str(p99)]
            cols += ["0"] * (34 - len(cols))
            f.write(",".join(cols) + "\n")


class Overhead(unittest.TestCase):
    def test_a_shift_past_the_aa_spread_is_flagged(self):
        with tempfile.TemporaryDirectory() as d:
            rows = []
            for rep in (1, 2, 3):
                rows += [(rep, "dpdk-aeron-rel", 10000, 25000, 30000), (rep, "aa", 10000, 25000 + 100 * rep, 30200),
                         (rep, "dpdk-aeron-rel-trace", 10000, 25050, 33000)]
            results(os.path.join(d, "results.csv"), rows)
            got = {r[3]: r for r in ts.overhead(d)}
            self.assertEqual(got["p50"][5:8], (300, 50, 3))
            self.assertEqual(got["p50"][8], "")
            self.assertEqual(got["p99"][5:7], (200, 3000))
            self.assertIn("FLAG", got["p99"][8])


def pcapng(path, packets):
    """packets: (iface, outbound, ns, frame bytes)."""
    out = bytearray()

    def block(t, body):
        n = 12 + len(body)
        out.extend(struct.pack("<II", t, n) + body + struct.pack("<I", n))

    block(0x0A0D0D0A, struct.pack("<IHHq", 0x1A2B3C4D, 1, 0, -1))
    for _ in range(2):
        block(1, struct.pack("<HHI", 1, 0, 160) + struct.pack("<HHB3x", 9, 1, 9) + struct.pack("<I", 0))
    for iface, outbound, ns, frame in packets:
        cap = min(len(frame), 160)
        body = struct.pack("<IIIII", iface, ns >> 32, ns & 0xffffffff, cap, len(frame))
        body += frame[:cap] + b"\0" * (((cap + 3) & ~3) - cap)
        body += struct.pack("<HHI", 2, 4, 2 if outbound else 1) + struct.pack("<I", 0)
        block(6, body)
    with open(path, "wb") as f:
        f.write(out)


def udp(payload):
    ip = bytes([0x45, 0, 0, 0, 0, 0, 0, 0, 64, 17, 0, 0]) + bytes(8)
    return bytes(12) + b"\x08\x00" + ip + bytes(8) + payload


def data(stream, term_offset, seq):
    h = struct.pack("<iBBHiiii", 96, 0, 0xc0, 1, term_offset, 7, stream, 3) + bytes(8)
    return h + struct.pack("<QQQII", seq, 0, 0, 0, 1) + bytes(32)


def ctl(kind, length):
    return struct.pack("<iBBH", length, 0, 0, kind) + bytes(length - 8)


class Capture(unittest.TestCase):
    def test_the_capture_is_accounted_for(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "c.pcapng")
            pk = []
            for q in range(10):
                pk.append((0, True, 1_000_000_000 + q * 100_000, udp(data(S, q * 96, q))))
                pk.append((1, False, 1_000_050_000 + q * 100_000, udp(data(E, q * 96, q))))
                pk.append((1, True, 1_000_060_000 + q * 100_000, udp(ctl(3, 36))))
            pk.append((0, True, 1_002_000_000, udp(data(S, 3 * 96, 3))))  # a retransmit
            pk.append((1, True, 1_002_000_000, udp(ctl(2, 28))))  # a NAK
            pk.append((0, True, 1_002_000_000, bytes(12) + b"\x08\x06" + bytes(28)))  # ARP
            pk.append((0, False, 1_002_000_000, bytes(12) + b"\x86\xdd" + bytes(40)))  # IPv6
            pcapng(p, pk)
            lines, flags = ts.capture_report(p, every=2)
            text = "\n".join(lines)
            self.assertIn("34 frames", text)
            self.assertIn("sender tx      data       11", text)
            self.assertIn("receiver tx    sm         10", text)
            self.assertIn("sampled messages sent (seq % 2 == 0): 5", text)
            self.assertIn("1 NAKs: a gap the receiver saw", flags)
            self.assertIn("1 data frames sent twice: retransmits", flags)
            self.assertIn("1 unexplained frames: sender rx ethertype 0x86dd", flags)
            with open(p, "rb") as f, gzip.open(p + ".gz", "wb") as g:
                g.write(f.read())
            self.assertEqual(ts.capture_report(p + ".gz")[1][:2], flags[:2])


class Run(unittest.TestCase):
    def test_the_session_report(self):
        with tempfile.TemporaryDirectory() as d:
            point(os.path.join(d, "points", "t1-r1-dpdk-aeron-rel-trace@STREAMS=1-10000"), range(3))
            os.makedirs(os.path.join(d, "points", "t1-r1-dpdk-aeron-rel@STREAMS=1-10000", "0"))
            pcapng(os.path.join(d, "points", "t1-r1-dpdk-aeron-rel-trace@STREAMS=1-10000", "capture-a.pcapng"),
                   [(0, True, 1, udp(data(S, 0, 0)))])
            out = ts.run(d)
            self.assertIn("### t1-r1-dpdk-aeron-rel-trace@STREAMS=1-10000", out)
            self.assertIn("capture, host a", out)
            self.assertIn("- nothing unexplained", out)
            rows = read(os.path.join(d, "stages.csv")).splitlines()
            self.assertEqual(rows[0], "point,stage,n,p50_ns,p99_ns,max_ns,max_at,max_at_s,flag")
            self.assertTrue(any(",round trip,3,30000," in r for r in rows), rows)
            self.assertTrue(os.path.exists(os.path.join(d, "stages.md")))
            self.assertEqual(ts.main(["point", os.path.join(d, "points", "t1-r1-dpdk-aeron-rel@STREAMS=1-10000")]), 1)


if __name__ == "__main__":
    unittest.main()
