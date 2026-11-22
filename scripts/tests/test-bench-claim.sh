#!/usr/bin/env bash
# Proof for scripts/check-bench-claim.py: a <= 1.00 claim is made only from a
# complete, resolved, exception-free run, and refused for an over-1.00 ratio,
# an INCONCLUSIVE pair (over 1.00 inside the A/A band), an unresolved pair, a
# documented exception, a missing arm, a missing profile, an unmeasured control
# and a wrong run id. Several cases pass the gate on purpose: they show the
# gate accepting what the claim refuses.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
claim="$repo_root/scripts/check-bench-claim.py"
gate="$repo_root/scripts/check-bench-gate.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
run=claim-test-run

# A copy of the gate with its two documented exceptions set to 1.00, so a
# clean run can show the claim holding.
python3 -I - "$gate" "$work/gate-no-exceptions.sh" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
text = text.replace('|1.01"', '|1.00"').replace('ceiling="1.05"', 'ceiling="1.00"')
open(sys.argv[2], "w", encoding="utf-8").write(text)
PY

# fixture <suite> <dir> <profile> [ratio=<key>:<r>] [ci=<w>] [control_ci=<w>] [drop=<key>] [control_noci=1]
fixture() {
    python3 -I - "$gate" "$run" "$@" <<'PY'
import json, os, re, sys
gate, run, suite, out, profile, *opts = sys.argv[1:]
o = dict(x.split("=", 1) for x in opts)
ratio = {}
for spec in opts:
    if spec.startswith("ratio="):
        k, v = spec[6:].rsplit(":", 1)
        ratio[k] = float(v)
text = open(gate, encoding="utf-8").read()
def entries(name):
    block = re.search(rf"(?m)^\s*{name}=\((.*?)\n\s*\)", text, re.S).group(1)
    return [m.split("|") for l in block.splitlines() if not l.lstrip().startswith("#")
            for m in re.findall(r'"([^"]*\|[^"]*)"', l)]
arms = []  # (key_ergo, key_ref)
if suite == "sbe":
    for _, g, e, r, _ in entries("pairs"):
        g = "parity_" + g.replace("/", "_")
        arms.append((f"{g}/{e}", f"{g}/{r}"))
else:
    for g, e, r, _ in entries("cluster_pairs"):
        arms.append((f"{g}/{e}", f"{g}/{r}"))
controls = [(f"{g}/{a}", f"{g}/{b}") for g, a, b in
            re.findall(r'aa_band "([^"]+)" "([^"]+)" "([^"]+)"', text)
            if g.startswith("parity_" if suite == "sbe" else "cluster_")]
ci, cci, drop = float(o.get("ci", 0.005)), float(o.get("control_ci", 0.005)), o.get("drop")
def put(base, key, point, width, interval=True):
    if key == drop:
        return
    d = os.path.join(base, key); os.makedirs(d, exist_ok=True)
    mean = {"point_estimate": point}
    if interval:
        mean["confidence_interval"] = {"lower_bound": point * (1 - width), "upper_bound": point * (1 + width)}
    est = {"mean": mean, "slope": None}
    json.dump(est, open(os.path.join(d, "estimates.json"), "w"))
    open(os.path.join(d, "run-id.txt"), "w").write(run)
trees = ["repeats", "repeats-reversed"] if suite == "cluster" else ["repeats"]
for tree in trees:
    for k in "123":
        base = os.path.join(out, tree, k)
        for e, r in arms:
            put(base, e, 100.0 * ratio.get(e, 0.97), ci); put(base, r, 100.0, ci)
        for a, b in controls:
            put(base, a, 100.0, cci, "control_noci" not in o); put(base, b, 100.0, cci, "control_noci" not in o)
manifest = {"run_id": run, "profile": profile, "commit": "c", "rustc": "r", "target": "t", "host": "h", "repeats": 3}
if suite == "cluster":
    manifest["arm_order"] = "paired"
json.dump(manifest, open(os.path.join(out, "run-manifest.json"), "w"))
PY
}

# both <suite> <name> [options for both profiles]: work/<name>/{lto,no-lto}.
both() {
    local suite=$1 name=$2; shift 2
    for p in lto no-lto; do
        mkdir -p "$work/$name/$p"
        fixture "$suite" "$work/$name/$p" "$p" "$@"
    done
}

# expect <name> <want rc> <want text> <claim args...>
expect() {
    local name=$1 want=$2 text=$3 out rc=0; shift 3
    out=$(python3 -I "$claim" "$@" 2>&1) || rc=$?
    if [ "$rc" -ne "$want" ] || ! grep -qF -- "$text" <<<"$out"; then
        echo "FAIL $name: want rc $want and '$text', got rc $rc:" >&2
        echo "$out" >&2
        exit 1
    fi
    echo "ok $name"
}

# gate_rc <suite> <dir>: the real gate's verdict on a profile directory.
gate_rc() {
    local out
    if out=$(bash "$gate" "$2" 0 "$1" --run-id "$run" 2>&1); then echo 0; else echo "$out" >&2; echo 1; fi
}
patched=(--gate "$work/gate-no-exceptions.sh")

both cluster clean
expect "a clean exception-free cluster run holds the claim" 0 "CLAIM OK" \
    --suite cluster --run-id "$run" "${patched[@]}" "$work/clean/lto" "$work/clean/no-lto"
expect "the shipped gate's documented exceptions refuse it" 1 "documented exception" \
    --suite cluster --run-id "$run" "$work/clean/lto" "$work/clean/no-lto"
expect "a missing profile refuses it" 1 "needs exactly the lto and no-lto profiles" \
    --suite cluster --run-id "$run" "${patched[@]}" "$work/clean/lto"
expect "a wrong run id refuses it" 1 "the gate itself fails" \
    --suite cluster --run-id other "${patched[@]}" "$work/clean/lto" "$work/clean/no-lto"

# The table at 1.00 but the no-lto 1.05 override left in: the profile override alone refuses it.
python3 -I - "$gate" "$work/gate-profile-exception.sh" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(text.replace('|1.01"', '|1.00"'))
PY
expect "a profile-only documented exception refuses it" 1 "no-lto: cluster_decode_session_event: a documented exception (ceiling 1.05)" \
    --suite cluster --run-id "$run" --gate "$work/gate-profile-exception.sh" "$work/clean/lto" "$work/clean/no-lto"

both cluster over "ratio=cluster_encode_session_keep_alive/ergo-sbe:1.004"
expect "an over-1.00 ratio refuses it" 1 "over 1.00" \
    --suite cluster --run-id "$run" "${patched[@]}" "$work/over/lto" "$work/over/no-lto"

# Ratio 1.01 with a +-5% A/A control: the gate calls it inconclusive and exits 0.
both cluster inconclusive "ratio=cluster_encode_session_keep_alive/ergo-sbe:1.01" control_ci=0.05
[ "$(gate_rc cluster "$work/inconclusive/lto")" = 0 ] || { echo "fixture: the gate should accept an inconclusive run" >&2; exit 1; }
expect "an INCONCLUSIVE pair refuses it" 1 "INCONCLUSIVE" \
    --suite cluster --run-id "$run" "${patched[@]}" "$work/inconclusive/lto" "$work/inconclusive/no-lto"

# Ratio 0.998 with +-1% CIs: under the ceiling in every invocation, but not resolved.
both cluster straddle "ratio=cluster_encode_session_keep_alive/ergo-sbe:0.998" ci=0.01
[ "$(gate_rc cluster "$work/straddle/lto")" = 0 ] || { echo "fixture: the gate should accept a straddling run" >&2; exit 1; }
expect "an unresolved pair refuses it" 1 "unresolved" \
    --suite cluster --run-id "$run" "${patched[@]}" "$work/straddle/lto" "$work/straddle/no-lto"

both cluster missing drop=cluster_decode_session_event/sbe-tool
expect "a missing arm refuses it" 1 "the gate itself fails" \
    --suite cluster --run-id "$run" "${patched[@]}" "$work/missing/lto" "$work/missing/no-lto"

both cluster nocontrol control_noci=1
[ "$(gate_rc cluster "$work/nocontrol/lto")" = 0 ] || { echo "fixture: the gate should accept a run whose control has no interval" >&2; exit 1; }
expect "an unmeasured equal-work control refuses it" 1 "no confidence interval" \
    --suite cluster --run-id "$run" "${patched[@]}" "$work/nocontrol/lto" "$work/nocontrol/no-lto"

both sbe sbe-clean
expect "a clean sbe run holds the claim" 0 "CLAIM OK" \
    --suite sbe --run-id "$run" "$work/sbe-clean/lto" "$work/sbe-clean/no-lto"
both sbe sbe-over "ratio=parity_decode_scalar/ergo-sbe:1.002"
expect "an over-1.00 sbe ratio refuses it" 1 "over 1.00" \
    --suite sbe --run-id "$run" "$work/sbe-over/lto" "$work/sbe-over/no-lto"

echo "bench claim: the <= 1.00 claim is refused for every case above and held for the clean runs"
