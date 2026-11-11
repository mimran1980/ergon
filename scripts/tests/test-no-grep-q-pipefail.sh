#!/usr/bin/env bash
# Under `set -o pipefail`, `producer | grep -q X` fails spuriously when grep
# exits at its first match and the producer dies of SIGPIPE, and
# `! producer | grep -q X` can pass while X is present. Capture the output and
# use a here-string (`grep -q X <<<"$out"`), or read to EOF (`grep X >/dev/null`).
#
# Flags `| grep -<flags with q>` in every tracked shell file that sets pipefail,
# except a line inside a `bash -c` / `sh -c` string (the line, or the one before
# it ending in `-c \`) that does not itself set pipefail: that inner shell has
# none. ALLOW lists any other proven-safe line by fixed text.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)

# "path<TAB>fixed text": inside a `sh -c '...'` run in a pod, without pipefail.
ALLOW=$'k8s/nri-shared-cpus/test/e2e.sh\tergon-cpus ready --add[r]'

# scan <file>...: print `file:line:text` for each offending line.
scan() {
    local f
    for f in "$@"; do
        grep -q 'pipefail' "$f" || continue
        awk -v f="$f" -v allow="$ALLOW" '
            BEGIN { n = split(allow, rows, "\n") }
            {
                bad = ($0 !~ /^ *#/ && $0 ~ /\| *grep +(-[A-Za-z]*q|--quiet)/)
                inner = ($0 ~ /(ba)?sh -c/ || prev ~ /(ba)?sh -c *\\$/) && $0 !~ /pipefail/
                ok = inner
                for (i = 1; i <= n; i++) {
                    split(rows[i], kv, "\t")
                    if (kv[1] == f && index($0, kv[2])) ok = 1
                }
                if (bad && !ok) printf "%s:%d:%s\n", f, NR, $0
                prev = $0
            }' "$f"
    done
}

fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

# Self-test: the old pattern must be flagged, the safe forms must not.
{
    printf '%s\n' 'set -euo pipefail'
    printf '%s\n' 'printf "%s\n" "$x" | grep -q foo || fail'
    printf '%s\n' '! logs | grep -qE VIOLATION'
} >"$fixture/bad.sh"
{
    printf '%s\n' 'set -euo pipefail'
    printf '%s\n' 'grep -q foo <<<"$x" || fail'
    printf '%s\n' 'check x bash -c "logs | grep -q foo"'
    printf '%s\n' 'check x bash -c \'
    printf '%s\n' '    "! logs | grep -q foo"'
} >"$fixture/good.sh"
printf '%s\n' 'x | grep -q foo' >"$fixture/nopipefail.sh"

flagged=$(scan "$fixture/bad.sh")
[[ $(grep -c . <<<"$flagged") -eq 2 ]] || { echo "self-test: bad.sh should give 2 hits, got: $flagged" >&2; exit 1; }
[[ -z $(scan "$fixture/good.sh" "$fixture/nopipefail.sh") ]] || { echo "self-test: safe forms were flagged" >&2; exit 1; }

cd "$repo_root"
# This file's fixtures quote the pattern on purpose, so it is not scanned.
files=$(git ls-files -- '*.sh' '*.just' justfile ':!scripts/tests/test-no-grep-q-pipefail.sh')
# shellcheck disable=SC2086
hits=$(scan $files)
if [[ -n $hits ]]; then
    echo "pipefail + 'grep -q' on a pipe loses matches to SIGPIPE; use a here-string or drop -q:" >&2
    echo "$hits" >&2
    exit 1
fi
echo "test-no-grep-q-pipefail: PASS"
