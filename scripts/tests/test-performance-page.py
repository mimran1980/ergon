#!/usr/bin/env python3
"""Negative and positive proof for scripts/generate-performance-page.py.

- --check passes on the committed page and fails on an edited copy, and on a
  changed artifact (a gap max the page does not show);
- codec rows come from a real check-bench-gate.sh run: PASS, INCONCLUSIVE and
  FAIL land in the verdict column, with the ergon arm's mean;
- an arm with no reviewed cause prints "unexplained", never a guess;
- codec ratios carry their ceiling; a profile laid out as `just bench` leaves
  it (gate.txt beside criterion/) is read; an empty gate.txt gives an
  "unreadable" row, not a missing suite;
- the ftrace arm gets its own host-gaps table;
- every gate outcome lands in one verdict (unproven and malformed are FAIL,
  an unmeasured interval is a PASS that says so);
- an SBE label finds its Criterion key through the gate's pairs table;
- with no artifacts every table reads "not measured".
"""
import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
GEN = ROOT / "scripts" / "generate-performance-page.py"
GAPS = ROOT / "k8s/nri-shared-cpus/lab/results/azure-20261024234841"
PAGE = ROOT / "book/src/project/performance.md"
CLUSTER = [
    "cluster_encode_session_message_header",
    "cluster_encode_session_keep_alive",
    "cluster_decode_session_message_header",
    "cluster_decode_session_event",
    "cluster_encode_claim_shaped_header_plus_app",
]


def run(*args):
    return subprocess.run([sys.executable, "-I", str(GEN), *map(str, args)], capture_output=True, text=True)


def expect(condition, message, output=""):
    if not condition:
        sys.exit(f"test-performance-page: FAIL: {message}\n{output}")


def estimate(path, slope, lo, hi):
    path.mkdir(parents=True, exist_ok=True)
    est = {"point_estimate": slope, "confidence_interval": {"lower_bound": lo, "upper_bound": hi}}
    (path / "estimates.json").write_text(json.dumps({"slope": est, "mean": est, "median": {"point_estimate": slope}}))


def cluster_run(crit):
    """A cluster profile with a wide A/A band, one pair each way, real gate output."""
    for group in ("cluster_control_aa_encode_keep_alive", "cluster_control_aa_encode_claim_shaped"):
        for arm in ("arm_a", "arm_b"):
            estimate(crit / group / arm / "new", 100, 50, 200)
    cases = {
        "cluster_encode_session_message_header": (90, 89.9, 90.1),
        "cluster_encode_session_keep_alive": (200, 50, 400),
        "cluster_decode_session_message_header": (100, 99.9, 100.1),
        "cluster_decode_session_event": (100, 99.9, 100.1),
        "cluster_encode_claim_shaped_header_plus_app": (500, 499, 501),
    }
    for group, ergo in cases.items():
        estimate(crit / group / "sbe-tool" / "new", 100, 99.9, 100.1)
        estimate(crit / group / "ergo-sbe" / "new", *ergo)
    gate = subprocess.run(
        [str(ROOT / "scripts/check-bench-gate.sh"), str(crit), "0", "cluster"], capture_output=True, text=True
    )
    expect(gate.returncode != 0, "the fixture's breach must fail the gate", gate.stdout)
    (crit / "gate.txt").write_text(gate.stdout)


def gate(crit, suite, log=None):
    out = subprocess.run(
        [str(ROOT / "scripts/check-bench-gate.sh"), str(crit), "0", suite], capture_output=True, text=True
    )
    (log or crit / "gate.txt").write_text(out.stdout)
    return out.stdout


def verdicts(text):
    spec = importlib.util.spec_from_file_location("report", ROOT / "scripts" / "bench-cluster-report.py")
    report = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(report)
    return {item["name"]: (item["verdict"], item["note"]) for item in report.parse_gate(text)}


def check_verdict_branches(tmp):
    """The gate outcomes the cluster fixture above does not reach, from real gate output."""
    # No A/A control: an over-ceiling pair whose interval straddles the ceiling
    # is unproven (FAIL, not INCONCLUSIVE); a pair with no interval is a pass
    # whose uncertainty is not measured.
    crit = tmp / "branches"
    for group in CLUSTER:
        estimate(crit / group / "sbe-tool" / "new", 100, 90, 110)
        estimate(crit / group / "ergo-sbe" / "new", 100, 99.9, 100.1)
    estimate(crit / CLUSTER[0] / "ergo-sbe" / "new", 200, 50, 400)
    (crit / CLUSTER[1] / "ergo-sbe" / "new" / "estimates.json").write_text('{"mean":{"point_estimate":90}}')
    (crit / CLUSTER[1] / "sbe-tool" / "new" / "estimates.json").write_text('{"mean":{"point_estimate":100}}')
    text = gate(crit, "cluster")
    found = verdicts(text)
    expect(found[CLUSTER[0]] == ("FAIL", "unproven: no measured A/A band reaches it"), "unproven is FAIL", text)
    expect(found[CLUSTER[1]] == ("PASS", "uncertainty not measured"), "no interval is said", text)

    # A malformed repeat: the gate refuses the pair in its "<label>: FAIL" form.
    crit = tmp / "malformed"
    for group in CLUSTER:
        for arm in ("ergo-sbe", "sbe-tool"):
            estimate(crit / "repeats" / "1" / group / arm, 100, 99.9, 100.1)
            estimate(crit / group / arm / "new", 100, 99.9, 100.1)
    (crit / "repeats" / "1" / CLUSTER[2] / "ergo-sbe" / "estimates.json").write_text('{"mean":')
    text = gate(crit, "cluster")
    found = verdicts(text)
    expect(found.get(CLUSTER[2], ("",))[0] == "FAIL", "malformed evidence is FAIL", text)
    expect("incomplete or malformed" in found[CLUSTER[2]][1], "the refusal is said", text)


def main():
    result = run("--check")
    expect(result.returncode == 0, "--check must pass on the committed page", result.stderr)

    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        edited = tmp / "edited.md"
        edited.write_text(PAGE.read_text().replace("| plugin-on | 3 |", "| plugin-on | 4 |"))
        result = run("--check", "--out", edited)
        expect(result.returncode != 0, "--check must fail on a hand-edited page", result.stdout)

        gaps = tmp / GAPS.name
        shutil.copytree(GAPS, gaps)
        before = tmp / "before.md"
        expect(run("--gaps", gaps, "--out", before).returncode == 0, "generate from the copy")
        result = run("--check", "--gaps", gaps, "--out", before)
        expect(result.returncode == 0, "--check must pass on what the copy gives", result.stderr)
        summary = gaps / "plugin-on-1.summary.tsv"
        head, row = summary.read_text().splitlines()
        cells = row.split("\t")
        cells[10] = "9999999"  # max_ns
        summary.write_text(head + "\n" + "\t".join(cells) + "\n")
        result = run("--check", "--gaps", gaps, "--out", before)
        expect(result.returncode != 0, "--check must fail when an artifact changes", result.stdout)
        out = tmp / "gaps.md"
        expect(run("--gaps", gaps, "--causes", tmp / "none.tsv", "--out", out).returncode == 0, "generate")
        text = out.read_text()
        expect("| 9,999,999 |" in text, "the max comes from summary.tsv", text)
        expect("| unexplained (rep 1: 0 involuntary switches (rep total)" in text, "no reviewed cause is unexplained", text)
        # The ftrace arm is not comparable: its own table, after the untraced one.
        untraced, _, traced = text.partition("Under ftrace (trace):")
        expect("| plugin-on | 3 |" in untraced and "| trace |" not in untraced, "trace has its own table", text)
        expect("| trace | 1 |" in traced and "1 involuntary switch (rep total)" in traced, "one switch", text)

        check_verdict_branches(tmp)

        artifacts = tmp / "bench-results"
        cluster_run(artifacts / "cluster-lto")
        (artifacts / "cluster-lto" / "run-manifest.json").write_text(
            json.dumps(
                {
                    "run_id": "r1",
                    "rustc": "rustc 1.99.0 (abc 2026-09-28)",
                    "uptime_start": " 10:00:00 up 1 day, load average: 0.10, 0.20, 0.30",
                    "uptime_end": " 10:30:00 up 1 day, load average: 0.40, 0.30, 0.30",
                }
            )
        )
        # An SBE label that is not its Criterion directory: the generator finds
        # the key in check-bench-gate.sh's pairs table. Laid out as `just bench`
        # leaves a profile: gate.txt beside criterion/.
        sbe = artifacts / "sbe-lto" / "criterion"
        estimate(sbe / "parity_encode_scalar" / "ergo-sbe_header_and_body" / "new", 90, 89.9, 90.1)
        estimate(sbe / "parity_encode_scalar" / "sbe-tool_header_and_body" / "new", 100, 99.9, 100.1)
        gate(sbe, "sbe", artifacts / "sbe-lto" / "gate.txt")
        # Samples of 1 iteration each: the pooled percentiles are the sorted times.
        ergo_new = sbe / "parity_encode_scalar" / "ergo-sbe_header_and_body" / "new"
        (ergo_new / "sample.json").write_text(json.dumps({"iters": [1, 1, 1, 1], "times": [40, 10, 30, 20]}))
        # A gate output with no graded pair must not drop the suite silently.
        (artifacts / "cluster-no-lto").mkdir()
        (artifacts / "cluster-no-lto" / "gate.txt").write_text("")
        out = tmp / "codec.md"
        result = run("--artifacts", artifacts, "--out", out)
        expect(result.returncode == 0, "generate with codec artifacts", result.stderr)
        text = out.read_text()
        for row in (
            "| cluster_encode_session_message_header | lto | 0.009 ns per op | 0.9000 (ceiling 1.00) |",
            "| cluster_encode_session_keep_alive | lto | 0.020 ns per op | 2.0000 (ceiling 1.00) |",
            "| cluster_encode_claim_shaped_header_plus_app | lto | 0.050 ns per op | 5.0000 (ceiling 1.00) |",
            "| encode_scalar_header_and_body | lto | 90.000 ns per iteration | 0.9000 (ceiling 1.00) |",
            "| encode_scalar_header_and_body | lto | ergo-sbe_header_and_body | 4 | 10.000 | 20.000 | 40.000 | 40.000 | 40.000 |",
            "| encode_scalar_header_and_body | lto | sbe-tool_header_and_body | 0 | n/a | n/a | n/a | n/a | n/a |",
            "| sbe codecs | no-lto | not measured |",
            "| cluster codecs | no-lto | not measured | not measured | not measured |"
            " gate output unreadable: no graded pair in the gate output",
        ):
            expect(row in text, f"missing codec row {row!r}", text)
        expect("rustc 1.99.0 (abc 2026-09-28)" in text and "rustc rustc" not in text, "rustc is printed once", text)
        expect("load at start: 10:00:00 up 1 day, load average: 0.10" in text, "the start load is on the page", text)
        expect("load at end: 10:30:00 up 1 day, load average: 0.40" in text, "the end load is on the page", text)
        for verdict in (
            "| PASS: resolved |",
            "| INCONCLUSIVE: no winner: inside the measured A/A band |",
            "| FAIL: resolved breach |",
        ):
            expect(verdict in text, f"missing verdict {verdict!r}", text)

        out = tmp / "empty.md"
        result = run("--artifacts", tmp / "nothing", "--gaps", tmp / "nothing", "--out", out)
        expect(result.returncode == 0, "generate with no artifacts", result.stderr)
        text = out.read_text()
        expect("| host gaps | not measured |" in text, "no gap run reads not measured", text)
        expect(text.count("not measured") >= 20, "every table reads not measured", text)
    print("test-performance-page: PASS")


if __name__ == "__main__":
    main()
