#!/usr/bin/env bash
# The transport suite's analysis with no cloud: fairness.py against a clean
# fixture and one fixture per violated rule, spikes.py --transport's labels,
# analyse.py --transport against expected-transport.md, transport_events.py's
# events file from a host's log and transport_tune.py's rule (test_transport.py).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
cloud=$(dirname "$here")
for script in fairness.py transport_points.py transport_events.py transport_tune.py transport_collect.py spikes.py test/transport_fixture.py test/test_transport.py; do
    python3 -c 'import ast, sys; ast.parse(open(sys.argv[1]).read(), sys.argv[1])' "$cloud/$script"
done
python3 -W error::ResourceWarning "$here/test_transport.py"
echo "test-transport: all passed"
