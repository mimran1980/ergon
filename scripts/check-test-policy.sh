#!/usr/bin/env bash
set -euo pipefail

root=.
manifest=test-lanes.tsv
while [[ $# -gt 0 ]]; do
    case "$1" in
        --root)
            root=$2
            shift 2
            ;;
        --manifest)
            manifest=$2
            shift 2
            ;;
        *)
            echo "usage: $0 [--root PATH] [--manifest PATH]" >&2
            exit 2
            ;;
    esac
done

root=$(cd "$root" && pwd)
if [[ "$manifest" != /* ]]; then
    manifest="$root/$manifest"
fi
if [[ ! -f "$manifest" ]]; then
    echo "test policy: manifest not found: $manifest" >&2
    exit 1
fi
if ! git -C "$root" rev-parse --git-dir >/dev/null 2>&1; then
    echo "test policy: root is not a Git worktree: $root" >&2
    exit 1
fi
if ! command -v rg >/dev/null 2>&1; then
    echo "test policy: ripgrep (rg) is required" >&2
    exit 1
fi

tracked=$(mktemp)
test_sources=$(mktemp)
manifest_rows=$(mktemp)
trap 'rm -f "$tracked" "$test_sources" "$manifest_rows"' EXIT
git -C "$root" ls-files --cached --others --exclude-standard >"$tracked"

failed=0
report_matches() {
    local label=$1
    local pattern=$2
    shift 2
    local output
    if output=$(cd "$root" && rg -n "$pattern" "$@" 2>/dev/null); then
        :
    elif [[ $? -eq 1 ]]; then
        output=
    else
        echo "test policy: scan failed while checking $label" >&2
        failed=1
        return
    fi
    if [[ -n "$output" ]]; then
        echo "test policy: $label" >&2
        echo "$output" >&2
        failed=1
    fi
}

rust_source_files=()
documentation_files=()
control_files=()
workflow_files=()
while IFS= read -r file; do
    case "$file" in
        *.rs)
            rust_source_files+=("$file")
            documentation_files+=("$file")
            ;;
        *.md)
            documentation_files+=("$file")
            ;;
    esac
    case "$file" in
        scripts/check-test-policy.sh|scripts/tests/*.sh)
            ;;
        justfile|just/*.just|scripts/*.sh|.github/workflows/*.yml|.github/workflows/*.yaml)
            control_files+=("$file")
            ;;
    esac
    case "$file" in
        .github/workflows/*.yml|.github/workflows/*.yaml)
            workflow_files+=("$file")
            ;;
    esac
    if [[ "$file" == *.rs ]] &&
        (cd "$root" && rg -q '#\[(tokio::)?test\]|proptest!|criterion_(group|main)!|fuzz_target!|```rust' "$file"); then
        printf '%s\n' "$file" >>"$test_sources"
    fi
done <"$tracked"

if [[ ${#rust_source_files[@]} -gt 0 ]]; then
    report_matches '#[ignore] is forbidden; every test must belong to an executable lane' \
        '#\[[[:space:]]*ignore([[:space:]=\(\]]|$)' "${rust_source_files[@]}"
    report_matches 'tests may not report a case as skipped while returning success' \
        '(e?println!\([^\n]*\bSKIP\b|[^\n]*Skipped:)' "${rust_source_files[@]}"
fi
if [[ ${#documentation_files[@]} -gt 0 ]]; then
    # In .rs files: any `ignore` fence in a doc comment is forbidden (old rule).
    rs_files=()
    md_files=()
    for f in "${documentation_files[@]}"; do
        case "$f" in
            *.rs) rs_files+=("$f") ;;
            *.md) md_files+=("$f") ;;
        esac
    done
    if [[ ${#rs_files[@]} -gt 0 ]]; then
        # Rust doc comment fences start with `///` — plain regex still covers them.
        report_matches 'ignored Rust documentation fence is forbidden; use compile-checked rust/no_run or honest text' \
            '```[[:alnum:]_,.:{}=+ -]*\bignore\b' "${rs_files[@]}"
    fi
    # In .md files: `rust,ignore` is always allowed — gives syntax highlighting
    # without CI compilation requirement. Hand-written schematics and
    # Aeron/rusteron-dependent examples cannot compile in the book-fence harness.
    :  # no-op: do not flag ignored fences in .md files
fi
if [[ ${#control_files[@]} -gt 0 ]]; then
    report_matches 'test-selection bypass is forbidden in test control files' \
        '(^|[[:space:]])--(skip|ignored|include-ignored)([=[:space:]]|$)' "${control_files[@]}"
    report_matches 'conditional cargo test/bench execution is forbidden' \
        '^[[:space:]]*@?if[[:space:]].*cargo[[:space:]]+(test|bench)' "${control_files[@]}"
    report_matches 'test/benchmark failures may not be converted to success' \
        '(cargo[[:space:]]+(test|bench)|just[[:space:]]+(test|check))[^#]*(\|\||;[[:space:]]*(exit[[:space:]]+0|true))' \
        "${control_files[@]}"

    conditional_output=$(
        cd "$root"
        awk '
            FNR == 1 {
                in_if = 0
                reported = 0
            }
            /^[[:space:]]*@?if[[:space:]]/ {
                in_if = 1
                if_line = FNR
            }
            in_if && /(cargo[[:space:]]+(test|bench)|just[[:space:]]+(test|check))/ && !reported {
                printf "%s:%d: conditional block contains test command at line %d\n",
                    FILENAME, if_line, FNR
                reported = 1
            }
            in_if && /^[[:space:]]*fi([;[:space:]]|$)/ {
                in_if = 0
                reported = 0
            }
        ' "${control_files[@]}"
    )
    if [[ -n "$conditional_output" ]]; then
        echo "test policy: tests and benchmarks may not be conditionally executed" >&2
        echo "$conditional_output" >&2
        failed=1
    fi
fi
if [[ ${#workflow_files[@]} -gt 0 ]]; then
    report_matches 'custom skip-CI condition is forbidden' \
        '\[(skip ci|ci skip|no ci|skip actions|actions skip)\]' "${workflow_files[@]}"
    report_matches 'continue-on-error is forbidden for fail-closed workflows' \
        '^[[:space:]]*continue-on-error:[[:space:]]*true([[:space:]]|$)' "${workflow_files[@]}"

    if workflow_conditions=$(
        cd "$root" && rg -n '^[[:space:]]*if:' "${workflow_files[@]}"
    ); then
        :
    elif [[ $? -eq 1 ]]; then
        workflow_conditions=
    else
        echo "test policy: workflow condition scan failed" >&2
        failed=1
        workflow_conditions=
    fi
    while IFS= read -r condition; do
        [[ -z "$condition" ]] && continue
        if [[ ! "$condition" =~ if:[[:space:]]*always\(\)[[:space:]]*$ ]]; then
            echo "test policy: workflow conditions may silently suppress a lane" >&2
            echo "$condition" >&2
            failed=1
        fi
    done <<<"$workflow_conditions"
fi

while IFS= read -r row || [[ -n "$row" ]]; do
    [[ -z "$row" || "$row" == \#* ]] && continue
    IFS=$'\t' read -r pattern lane command extra <<<"$row"
    if [[ -z "${pattern:-}" || -z "${lane:-}" || -z "${command:-}" || -n "${extra:-}" ]]; then
        echo "test policy: malformed manifest row (expected pattern<TAB>lane<TAB>command): $row" >&2
        failed=1
        continue
    fi
    if [[ "$command" =~ (^|[[:space:]])--(skip|ignored|include-ignored)(=|[[:space:]]|$) ]]; then
        echo "test policy: manifest lane '$lane' contains a test-selection bypass: $command" >&2
        failed=1
    fi
    printf '%s\t%s\t%s\n' "$pattern" "$lane" "$command" >>"$manifest_rows"
done <"$manifest"

owned=0
while IFS= read -r file; do
    matches=0
    owners=
    while IFS=$'\t' read -r pattern lane command; do
        if [[ "$file" == $pattern ]]; then
            matches=$((matches + 1))
            owners="${owners}${owners:+, }${lane}"
        fi
    done <"$manifest_rows"
    if [[ $matches -eq 0 ]]; then
        echo "test policy: no test lane owns $file" >&2
        failed=1
    elif [[ $matches -gt 1 ]]; then
        echo "test policy: multiple test lanes own $file: $owners" >&2
        failed=1
    else
        owned=$((owned + 1))
    fi
done <"$test_sources"

while IFS=$'\t' read -r pattern lane command; do
    pattern_matches=0
    while IFS= read -r file; do
        if [[ "$file" == $pattern ]]; then
            pattern_matches=$((pattern_matches + 1))
        fi
    done <"$test_sources"
    if [[ $pattern_matches -eq 0 ]]; then
        echo "test policy: lane '$lane' pattern matches no tracked test source: $pattern" >&2
        failed=1
    fi
done <"$manifest_rows"

if [[ $failed -ne 0 ]]; then
    exit 1
fi
echo "test policy: PASS ($owned tracked test-bearing sources, zero suppressions)"
