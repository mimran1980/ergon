#!/usr/bin/env python3
"""Copy the completeness counts out of a run into a small tracked file.

`tables.md` is gitignored (it is generated and large), but the completeness
counts cited in BENCHMARK.md have to be checkable from the repository. This
writes `work-counts.md` beside the run's `fair.md`, which is tracked.

    python3 scripts/extract_work_counts.py results/<stamp>

Two accounts, and they are not the same one:

* the work behind each percentile — scheduled, issued, completed, late,
  rejected, censored — copied out of the generated `tables.md`, counts the
  analyser already reconciled against `records.bin` before any warm-up was
  dropped (that file is long gone by then);
* the sender's batch schedule — sent, batches, late >100 µs, latest — read
  from each point's `meta.txt`, which says whether the offered traffic was
  the planned traffic. `late` above cannot say that: it counts the paced
  order slots filled when the next was already due, and a point can report 2
  of those and tens of thousands of late batches at once.

A point over 1% of batches late is named in the summary, so the runs that
measured a load they were not offered on purpose cannot be read as clean.

Both accounts follow `Work` and `SendSchedule` in `src/stats.rs`. A count a
run reported is read or named: unreadable values, minus counts, a partial
sender schedule and counts that contradict each other stop the extractor
rather than write a table that reads as clean. A count the run did not report
is "not yet measured" — never invented, never a zero.
"""

import sys
from pathlib import Path

SEND_KEYS = ("send.sent", "send.batches", "send.late_100us", "send.late_max_us")
WORK_KEYS = (
    "work.scheduled",
    "work.issued",
    "work.completed",
    "work.late",
    "work.rejected",
    "work.censored",
)
NOT_MEASURED = "not yet measured"


class MetaError(Exception):
    """A count a run reported but that cannot be read, or counts that
    contradict each other: named, and fatal. Not an honest hole."""


def parse_count(key: str, raw: str) -> int:
    """One count as `src/stats.rs` parses it: a `u64`.

    A present key with anything else — a minus count included — is malformed
    metadata, not missing data, and is named before the extractor goes on.
    """
    body = raw[1:] if raw.startswith("+") else raw
    if not body or not body.isascii() or not body.isdigit() or int(raw) >= 2**64:
        raise MetaError(f"{key}={raw} is not a count")
    return int(raw)


def check_work_row(row: str) -> None:
    """One work row on its way into the tracked file, as `Work` reads it.

    A cell is "not yet measured" or a count. A count the row carries is read
    or named; a count it does not carry is absent data and the row stays —
    `Work::from_meta` keeps a partial account. Where recorded counts can
    contradict each other they are checked, as `Work::check` does.
    """
    cells = [c.strip() for c in row.strip().strip("|").split("|")]
    if len(cells) > 1 and cells[1] == "scheduled":
        return  # the header
    if cells and all(c and set(c) <= {"-"} for c in cells):
        return  # its separator
    label, *raw_counts = cells
    vals = []
    for k, raw in zip(WORK_KEYS, raw_counts):
        if raw == NOT_MEASURED:
            vals.append(None)
        else:
            try:
                vals.append(parse_count(k, raw))
            except MetaError as e:
                raise MetaError(f"{label}: {e}") from None
    # A count the row does not carry is "not yet measured", never a zero and
    # never a reason to drop the row.
    vals.extend([None] * (len(WORK_KEYS) - len(vals)))
    scheduled, issued, completed, _late, _rejected, censored = vals
    if None not in (issued, completed, censored) and issued != completed + censored:
        raise MetaError(
            f"{label}: work.issued {issued} is not work.completed {completed}"
            f" + work.censored {censored}"
        )
    if None not in (scheduled, issued) and scheduled < issued:
        raise MetaError(f"{label}: work.scheduled {scheduled} is below work.issued {issued}")


def extract(run_dir: Path) -> int:
    src_path = run_dir / "tables.md"
    if not src_path.is_file():
        print(f"{src_path}: no tables.md", file=sys.stderr)
        return 1
    src = src_path.read_text().splitlines()
    out = [
        f"# Work behind the percentiles — run {run_dir.name}",
        "",
        "Extracted from the run's generated `tables.md` (gitignored by policy) so the",
        "completeness counts cited in `BENCHMARK.md` are checkable from the repository.",
        "Counts are the run's own account, reconciled against `records.bin` before any",
        "warm-up is dropped. Every percentile in `BENCHMARK.md` is over `completed`",
        "alone, after the warm-up cut, so `n` is smaller still.",
        "",
        "`scheduled` `issued` `completed` `late` `rejected` `censored`",
        "",
    ]
    cur = "unknown"
    i = 0
    n = 0
    while i < len(src):
        line = src[i]
        if line.startswith("## "):
            cur = line[3:].strip()
        if line.startswith("### Work behind the percentiles"):
            out.append(f"## {cur}")
            out.append("")
            j = i + 1
            while j < len(src) and not src[j].startswith("| "):
                j += 1
            while j < len(src) and src[j].startswith("|"):
                check_work_row(src[j])
                out.append(src[j])
                j += 1
            out.append("")
            n += 1
            i = j
            continue
        i += 1
    out.extend(send_schedule_section(run_dir))
    (run_dir / "work-counts.md").write_text("\n".join(out) + "\n")
    print(f"{run_dir / 'work-counts.md'}: {n} tables")
    return 0


def send_schedule_section(run_dir: Path):
    """The sender's batch schedule per point, from the `send.*` meta keys.

    Matches `SendSchedule::from_meta` in `src/stats.rs`. A point that wrote
    none of the keys has no sender account and is left out rather than shown
    as zeros (an honest hole). Anything it did write is evidence: every key
    present is read or the extractor stops, and all four are required — a
    sender schedule is all or none. Reported separately from the work counts
    because it counts a different population (see the module docstring).
    """
    rows = []
    for meta in sorted((run_dir / "res").rglob("meta.txt")):
        kv = {}
        for line in meta.read_text().splitlines():
            if "=" in line:
                k, v = line.split("=", 1)
                kv[k.strip()] = v.strip()
        if all(k not in kv for k in SEND_KEYS):
            continue
        # Unreadable first, then incomplete: a run that wrote `late_100us=lots`
        # and forgot `sent` is lying about a number, which outranks the hole.
        vals = {}
        for k in SEND_KEYS:
            if k in kv:
                try:
                    vals[k] = parse_count(k, kv[k])
                except MetaError as e:
                    raise MetaError(f"{meta}: {e}") from None
        for k in SEND_KEYS:
            if k not in vals:
                raise MetaError(
                    f"{meta}: {k} is missing: a sender schedule is all or none"
                )
        sent, batches, late, late_max = (vals[k] for k in SEND_KEYS)
        rel = meta.parent.relative_to(run_dir / "res")
        pct = (100.0 * late / batches) if batches else None
        verdict = (
            "not yet measured"
            if pct is None
            else ("over 1%: not the planned traffic" if pct > 1.0 else "ok")
        )
        share = f"{late} ({pct:.2f}%)" if pct is not None else str(late)
        rows.append((str(rel), sent, batches, share, late_max, verdict))
    if not rows:
        return []
    slipped = [r for r in rows if r[5].startswith("over")]
    out = [
        "",
        "## Sender schedule behind those percentiles",
        "",
        "`batches` are the due times the sender had traffic for — the order connection and the",
        "feeds together, the first message of each — and `late >100 µs` is the part that began",
        "over 100 µs after its due time. This is not `late` above: that counts the paced order",
        "slots filled when the next slot was already due. Traffic sent late is not the traffic",
        "planned (a saturated sender also slows down with a slow receiver), so a row over 1% is",
        "measuring a load the run was not offered on purpose.",
        "",
        "| point | sent | batches | late >100 µs | latest µs | verdict |",
        "|---|---|---|---|---|---|",
    ]
    for rel, sent, batches, share, late_max, verdict in rows:
        out.append(f"| {rel} | {sent} | {batches} | {share} | {late_max} | {verdict} |")
    out.append("")
    if slipped:
        out.append(
            f"{len(slipped)} of {len(rows)} points over 1% of batches late: "
            + ", ".join(f"`{r[0]}`" for r in slipped)
            + "."
        )
    else:
        out.append(f"all {len(rows)} points within 1% of batches late.")
    out.append("")
    # A `latest µs` far past the rest of the run is a stall, not a ranking.
    # Name it here so the column cannot be read as a like-for-like ordering;
    # the investigation itself is narrative and lives in BENCHMARK.md.
    maxima = sorted(r[4] for r in rows)
    mid = maxima[len(maxima) // 2]
    if mid > 0:
        stalls = [r for r in rows if r[4] > 100 * mid]
        if stalls:
            out.append(
                "`latest µs` is the worst batch of the whole run. "
                + ", ".join(f"`{r[0]}` at {r[4]:,}" for r in stalls)
                + " sit over 100× the median of the column"
                + (f" ({mid:,})" if mid else "")
                + ". Each is investigated in `io/benches/BENCHMARK.md`; no latency"
                " sample saw them, so the published figures stand. Do not rank a"
                " row on `latest µs`."
            )
            out.append("")
    return out


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        return extract(Path(sys.argv[1]))
    except MetaError as e:
        print(e, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
