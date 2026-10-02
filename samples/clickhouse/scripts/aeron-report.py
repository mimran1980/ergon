#!/usr/bin/env python3
"""Charts and tables from scripts/aeron-matrix.sh results.

    aeron-report.py OUT_DIR RESULTS.jsonl...

Writes OUT_DIR/*.png and OUT_DIR/results.md: round-trip percentiles per
driver mode and CPU policy, idle strategies, and the archive's cost and
replay rate. Needs matplotlib.
"""
import json
import sys
from collections import defaultdict
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

POLICIES = ["shared", "pinned", "exclusive", "isolated"]
MODES = ["dedicated", "shared-network", "shared", "invoker"]
PCTS = [("min_ns", "min"), ("p50_ns", "p50"), ("p99_ns", "p99"), ("p9999_ns", "p99.99"), ("max_ns", "max")]


def load(paths):
    rows = []
    for path in paths:
        for line in Path(path).read_text().splitlines():
            if line.strip():
                rows.append(json.loads(line))
    return rows


def us(ns):
    return ns / 1000.0


def fmt(ns):
    return f"{ns / 1000:.1f}" if ns >= 1000 else f"{ns / 1000:.2f}"


def load_name(rate):
    return "closed loop" if rate == 0 else f"{rate // 1000}k msg/s open loop"


def chart_modes(rows, out, transport, rate):
    """One panel per percentile: bars per driver mode, grouped by policy."""
    pick = [r for r in rows if r["transport"] == transport and r["rate"] == rate
            and r["idle"] == "Spin" and r["size"] == 64 and r["archive"] == "none"]
    policies = [p for p in POLICIES if any(r["policy"] == p for r in pick)]
    if not pick:
        return None
    fig, axes = plt.subplots(1, len(PCTS), figsize=(4.2 * len(PCTS), 4.2), sharey=False)
    width = 0.8 / max(len(policies), 1)
    for ax, (key, label) in zip(axes, PCTS):
        for i, policy in enumerate(policies):
            values = []
            for mode in MODES:
                hit = [r for r in pick if r["policy"] == policy and r["mode"] == mode]
                values.append(us(hit[0][key]) if hit else 0)
            ax.bar([m + i * width for m in range(len(MODES))], values, width, label=policy)
        ax.set_title(label)
        ax.set_yscale("log")
        ax.set_xticks([m + width * (len(policies) - 1) / 2 for m in range(len(MODES))])
        ax.set_xticklabels(MODES, rotation=30, ha="right", fontsize=8)
        ax.set_ylabel("round trip (µs, log)")
        ax.grid(axis="y", alpha=0.3)
    axes[0].legend(title="CPU policy", fontsize=8)
    fig.suptitle(f"Aeron {transport.upper()} round trip, {load_name(rate)}, 64-byte messages")
    fig.tight_layout()
    name = f"modes-{transport}-{rate}.png"
    fig.savefig(out / name, dpi=110)
    plt.close(fig)
    return name


def chart_idle(rows, out):
    pick = [r for r in rows if r["transport"] == "ipc" and r["rate"] == 0 and r["size"] == 64
            and r["archive"] == "none" and r["mode"] in ("dedicated", "invoker")]
    if not pick:
        return None
    policies = [p for p in POLICIES if any(r["policy"] == p for r in pick)]
    fig, axes = plt.subplots(1, len(policies), figsize=(5 * len(policies), 4.2), squeeze=False)
    for ax, policy in zip(axes[0], policies):
        labels, p50, p99, p999 = [], [], [], []
        for mode in ("dedicated", "invoker"):
            for idle in ("Spin", "Backoff", "Sleep"):
                hit = [r for r in pick if r["policy"] == policy and r["mode"] == mode and r["idle"] == idle]
                if hit:
                    labels.append(f"{mode}\n{idle.lower()}")
                    p50.append(us(hit[0]["p50_ns"]))
                    p99.append(us(hit[0]["p99_ns"]))
                    p999.append(us(hit[0]["p999_ns"]))
        x = range(len(labels))
        ax.bar([i - 0.27 for i in x], p50, 0.27, label="p50")
        ax.bar(list(x), p99, 0.27, label="p99")
        ax.bar([i + 0.27 for i in x], p999, 0.27, label="p99.9")
        ax.set_xticks(list(x))
        ax.set_xticklabels(labels, fontsize=8)
        ax.set_yscale("log")
        ax.set_title(f"policy: {policy}")
        ax.set_ylabel("round trip (µs, log)")
        ax.grid(axis="y", alpha=0.3)
    axes[0][0].legend(fontsize=8)
    fig.suptitle("Idle strategy of the polling threads (IPC, closed loop)")
    fig.tight_layout()
    fig.savefig(out / "idle.png", dpi=110)
    plt.close(fig)
    return "idle.png"


def chart_archive(rows, out):
    pick = [r for r in rows if r["archive"] != "none"]
    if not pick:
        return None
    base = [r for r in rows if r["archive"] == "none" and r["transport"] == "ipc" and r["idle"] == "Spin"
            and r["size"] == 64 and r["mode"] in ("dedicated", "invoker")]
    fig, axes = plt.subplots(1, 2, figsize=(13, 4.4))
    groups = defaultdict(dict)
    for r in base + pick:
        groups[(r["policy"], r["mode"], r["rate"])][r["archive"]] = r
    keys = sorted(groups, key=lambda k: (POLICIES.index(k[0]) if k[0] in POLICIES else 9, k[1], k[2]))
    labels = [f"{p}\n{m}\n{load_name(rate).split(' ')[0]}" for p, m, rate in keys]
    for j, archive in enumerate(("none", "dedicated", "shared")):
        values = [us(groups[k][archive]["p999_ns"]) if archive in groups[k] else 0 for k in keys]
        axes[0].bar([i + (j - 1) * 0.27 for i in range(len(keys))], values, 0.27,
                    label={"none": "not recorded"}.get(archive, f"archive {archive}"))
    axes[0].set_xticks(range(len(keys)))
    axes[0].set_xticklabels(labels, fontsize=7)
    axes[0].set_yscale("log")
    axes[0].set_ylabel("p99.9 round trip (µs, log)")
    axes[0].set_title("Recording the ping stream: p99.9 cost")
    axes[0].legend(fontsize=8)
    axes[0].grid(axis="y", alpha=0.3)
    rep = [r for r in pick if r.get("replay_msgs_per_s")]
    if rep:
        lab = [f"{r['policy']}\n{r['archive']}\n{r['mode']}\n{load_name(r['rate']).split(' ')[0]}" for r in rep]
        axes[1].bar(range(len(rep)), [r["replay_msgs_per_s"] / 1e6 for r in rep])
        axes[1].set_xticks(range(len(rep)))
        axes[1].set_xticklabels(lab, fontsize=6)
        axes[1].set_ylabel("replay catch-up (million msgs/s)")
        axes[1].set_title("Replay rate (an engine's resync)")
        axes[1].grid(axis="y", alpha=0.3)
    fig.tight_layout()
    fig.savefig(out / "archive.png", dpi=110)
    plt.close(fig)
    return "archive.png"


def table(rows, keys):
    head = "| policy | archive | mode | transport | load | idle | size | min µs | p50 µs | p99 µs | p99.99 µs | max µs | load1 |"
    lines = [head, "|" + "---|" * 13]
    for r in sorted(rows, key=keys):
        lines.append(
            f"| {r['policy']} | {r['archive']} | {r['mode']} | {r['transport']} | {load_name(r['rate'])} | "
            f"{r['idle'].lower()} | {r['size']} | {fmt(r.get('min_ns', 0))} | {fmt(r['p50_ns'])} | {fmt(r['p99_ns'])} | "
            f"{fmt(r['p9999_ns'])} | {fmt(r['max_ns'])} | {r.get('load1', '')} |"
        )
    return "\n".join(lines)


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    rows = load(sys.argv[2:])
    charts = []
    for transport in ("ipc", "udp"):
        for rate in (0, 100000, 500000):
            name = chart_modes(rows, out, transport, rate)
            if name:
                charts.append((f"{transport.upper()}, {load_name(rate)}", name))
    for title, maker in (("Idle strategies", chart_idle), ("Archive", chart_archive)):
        name = maker(rows, out)
        if name:
            charts.append((title, name))
    hosts = sorted({(r["host"], r["cpu"]) for r in rows})
    order = lambda r: (POLICIES.index(r["policy"]) if r["policy"] in POLICIES else 9, r["archive"],
                       MODES.index(r["mode"]) if r["mode"] in MODES else 9, r["transport"], r["rate"], r["idle"], r["size"])
    md = ["# Aeron latency by driver mode and CPU policy (results)", "",
          "Generated by `scripts/aeron-report.py` from `scripts/aeron-matrix.sh` runs. Round trips in microseconds;",
          "one-way latency is about half. Hosts: " + "; ".join(f"{h} ({c})" for h, c in hosts) + ".", ""]
    for title, name in charts:
        md += [f"## {title}", "", f"![{title}]({name})", ""]
    md += ["## Every run", "", table(rows, order), ""]
    (out / "results.md").write_text("\n".join(md))
    print(f"{len(rows)} runs -> {out / 'results.md'} and {len(charts)} charts")


if __name__ == "__main__":
    main()
