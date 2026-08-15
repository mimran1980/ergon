#!/usr/bin/env bash
# Stamp a cluster Criterion tree with run provenance for packaging.
set -euo pipefail
dir="${1:?criterion dir}"
profile="${2:?profile}"
run_id="${3:?run_id}"
commit="${4:?commit}"
rustc_v="${5:?rustc}"
target="${6:?target}"
count="$(find "$dir" -name estimates.json -path '*/new/*' 2>/dev/null | wc -l | tr -d ' ')"
python3 -c "
import json
with open('${dir}/run-manifest.json', 'w') as f:
    json.dump({
        'run_id': '''${run_id}''',
        'profile': '''${profile}''',
        'commit': '''${commit}''',
        'rustc': '''${rustc_v}''',
        'target': '''${target}''',
        'estimates': ${count},
    }, f, indent=2)
"
echo "stamped cluster $profile run_id=$run_id estimates=$count"
