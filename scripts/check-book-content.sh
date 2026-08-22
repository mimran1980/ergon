#!/usr/bin/env bash
# check-book-content.sh — grep book code fences for known stale/incorrect API
# patterns that the allowlist alone doesn't catch. Called from release-check.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
rc=0

check_pattern() {
    local pattern="$1"
    local message="$2"
    if grep -rn "$pattern" "$root/book/src/" --include='*.md' | grep -v 'check-book-content\|allowlist' > /dev/null 2>&1; then
        echo "  FAIL: $message"
        grep -rn "$pattern" "$root/book/src/" --include='*.md' | grep -v 'check-book-content\|allowlist'
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

# Old API: with_domain_objects(bool) inside code fences (not prose)
check_pattern '\`\`\`.*\n.*with_domain_objects(true)' \
    'with_domain_objects(true) is obsolete — use with_domain_objects(DomainVarData::Bytes)'

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


if [ $rc -eq 0 ]; then
    echo "check-book-content: PASS"
else
    echo "check-book-content: FAIL — fix the patterns above"
fi
exit $rc
