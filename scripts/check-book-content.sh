#!/usr/bin/env bash
# check-book-content.sh — grep book code fences for known stale/incorrect API
# patterns that the allowlist alone doesn't catch. Called from release-check.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
book_src=${BOOK_CONTENT_ROOT:-$root/book/src}
rc=0

check_pattern() {
    local pattern="$1"
    local message="$2"
    if grep -rn "$pattern" "$book_src/" --include='*.md' | grep -v 'check-book-content\|allowlist' > /dev/null 2>&1; then
        echo "  FAIL: $message"
        grep -rn "$pattern" "$book_src/" --include='*.md' | grep -v 'check-book-content\|allowlist'
        rc=1
    fi
}

echo "=== book content check ==="

# Deprecated chrono: NaiveDateTime::from_timestamp_opt
check_pattern 'NaiveDateTime::from_timestamp_opt' \
    'NaiveDateTime::from_timestamp_opt is deprecated — use DateTime::from_timestamp(...)'

# Deprecated chrono: NaiveDateTime::timestamp (not DateTime<Utc>::timestamp)
check_pattern 'naive.*\.timestamp()' \
    'NaiveDateTime::timestamp() is deprecated — use .and_utc().timestamp()'

# Old API inside a fence. A line-oriented grep cannot see this: the call is
# not required to sit on the line after the opening fence, and macOS grep
# has no multiline mode. Prose mentioning the call is not a failure.
if ! python3 - "$book_src" <<'PY'
import pathlib, sys

root = pathlib.Path(sys.argv[1])
failed = False
for path in sorted(root.rglob("*.md")):
    lines = path.read_text(errors="replace").splitlines()
    in_fence = False
    body = []
    start = 0
    for number, line in enumerate(lines, 1):
        if line.lstrip().startswith("```"):
            if in_fence:
                if "with_domain_objects(true)" in "\n".join(body):
                    print(f"  FAIL: {path}:{start}: with_domain_objects(true) inside a fence")
                    failed = True
                body = []
                in_fence = False
            else:
                in_fence = True
                start = number
                body = []
        elif in_fence:
            body.append(line)
    if in_fence and "with_domain_objects(true)" in "\n".join(body):
        print(f"  FAIL: {path}:{start}: with_domain_objects(true) inside a fence")
        failed = True
sys.exit(1 if failed else 0)
PY
then
    rc=1
fi

# CString::new in code fences — must use c"…" literals
check_pattern 'CString::new' \
    'CString::new is forbidden — use c"…" literals or cformat!'

# Stale generation-config default model (T-8).
check_pattern 'All knobs default to `true`' \
    'GenerationConfig knobs do not all default to true — list enabled vs disabled defaults'

# Stale ParseError variants that were never shipped (T-7).
check_pattern 'ParseError::Unsupported' \
    'ParseError has no Unsupported variant'
check_pattern 'schema_parse::unsupported' \
    'ParseError has no unsupported diagnostic code'

# Trust-boundary drift: wrap does not return Result.
check_pattern 'wrap` return `Result' \
    'bare wrap panics if short — it does not return Result'

# Copied point estimates (quote a run id instead).
check_pattern '22-23%' \
    'do not copy bulk_add latency percentages into the book'


if [ $rc -eq 0 ]; then
    echo "check-book-content: PASS"
else
    echo "check-book-content: FAIL — fix the patterns above"
fi
exit $rc
