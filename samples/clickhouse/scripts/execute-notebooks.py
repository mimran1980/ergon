#!/usr/bin/env python3
"""Execute every notebook in a fresh kernel against the fixture dataset.

Uses nbclient with finite timeouts and allow_errors=False; saves executed
copies + rendered HTML under artifacts/<run-id>/ and fails on exceptions,
empty required datasets, or failing assertions.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

import nbformat
from nbclient import NotebookClient


class Exporter:
    """The archive-agent's `/metrics` surface, up for the duration of a run.

    Notebook 06 scrapes a real Prometheus-format endpoint; there is no
    Prometheus in the local fixture stack, so the exporter itself is the
    thing under test.
    """

    def __init__(self, root: Path, addr: str = "127.0.0.1:9101") -> None:
        self.root = root
        self.addr = addr
        self.url = f"http://{addr}/metrics"
        self.proc: subprocess.Popen[bytes] | None = None
        self._tmp: tempfile.TemporaryDirectory[str] | None = None
        self._external = os.environ.get("ERGO_METRICS_URL", "")

    def __enter__(self) -> "Exporter":
        # A caller-supplied endpoint wins: that is how a real Prometheus or a
        # deliberate negative test replaces the local exporter.
        if self._external:
            self.url = self._external
            return self
        binary = self.root / "target" / "debug" / "archive-agent"
        if not binary.exists():
            raise RuntimeError(f"{binary} missing; run `cargo build -p archive-agent` first")
        # rusteron links libaeron out of the build tree; there is no install
        # step, so children need the dyld fallback path.
        libdirs = sorted(
            {str(p.parent) for p in (self.root / "target" / "debug" / "build").glob("**/libaeron*.dylib")}
        )
        os.environ["DYLD_FALLBACK_LIBRARY_PATH"] = ":".join(libdirs)
        self._tmp = tempfile.TemporaryDirectory()
        self.proc = subprocess.Popen(
            [
                str(binary),
                "--mode",
                "metrics-only",
                "--metrics-addr",
                self.addr,
                "--catalog",
                str(Path(self._tmp.name) / "metrics-catalog.db"),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        os.environ["ERGO_METRICS_URL"] = self.url
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(f"archive-agent exited {self.proc.returncode} before serving {self.url}")
            try:
                with urllib.request.urlopen(self.url, timeout=1) as resp:
                    resp.read()
                return self
            except OSError:
                time.sleep(0.25)
        raise RuntimeError(f"archive-agent did not serve {self.url} within 15s")

    def __exit__(self, *_exc: object) -> None:
        if self.proc is not None:
            self.proc.terminate()
            self.proc.wait(timeout=10)
        if self._tmp is not None:
            self._tmp.cleanup()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=["fixture", "live"], default="fixture")
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--timeout", type=int, default=300)
    args = parser.parse_args()

    # Kernels inherit this process's environment; the notebooks read the run id
    # and ClickHouse connection from it.
    os.environ["ERGO_RUN_ID"] = args.run_id

    root = Path(__file__).resolve().parent.parent
    notebooks_dir = root / "notebooks"
    out_dir = root / "artifacts" / args.run_id
    out_dir.mkdir(parents=True, exist_ok=True)

    notebooks = sorted(notebooks_dir.glob("*.ipynb"))
    # An empty glob used to report "all notebooks executed cleanly"; a run that
    # executed nothing is not a passing run.
    if not notebooks:
        print(f"[notebooks] no notebooks found under {notebooks_dir}", file=sys.stderr)
        return 1

    failed = []
    with Exporter(root):
        for nb_path in notebooks:
            print(f"[notebooks] executing {nb_path.name}", flush=True)
            nb = nbformat.read(nb_path, as_version=4)
            client = NotebookClient(
                nb,
                timeout=args.timeout,
                allow_errors=False,
                kernel_name="python3",
                resources={"metadata": {"path": str(root)}},
            )
            try:
                client.execute()
            except Exception as e:  # noqa: BLE001 — report every failure
                print(f"[notebooks] FAIL {nb_path.name}: {e}", file=sys.stderr)
                failed.append(nb_path.name)
                continue
            executed = out_dir / f"executed-{nb_path.name}"
            nbformat.write(nb, executed)
            try:
                from nbconvert import HTMLExporter

                body, _ = HTMLExporter().from_notebook_node(nb)
                executed.with_suffix(".html").write_text(body)
            except Exception as e:  # noqa: BLE001 — HTML rendering is best-effort
                print(f"[notebooks] WARN {nb_path.name}: html render failed: {e}")

    if failed:
        print(f"[notebooks] failed: {failed}", file=sys.stderr)
        return 1
    manifest = {
        "mode": args.mode,
        "run_id": args.run_id,
        "executed": [p.name for p in notebooks],
        "finished_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    (out_dir / "notebooks-manifest.json").write_text(json.dumps(manifest, indent=1))
    print(f"[notebooks] all notebooks executed cleanly; artifacts in {out_dir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
