#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-test-policy.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

git -C "$fixture" init -q
git -C "$fixture" config user.email test@example.invalid
git -C "$fixture" config user.name "Policy Test"
mkdir -p "$fixture/tests" "$fixture/.github/workflows"

write_valid_fixture() {
    printf '%s\n' \
        '# pattern<TAB>lane<TAB>command' \
        $'tests/*.rs\tpr\tcargo test --tests' \
        >"$fixture/test-lanes.tsv"
    printf '%s\n' \
        '#[test]' \
        'fn ordinary_test() {}' \
        >"$fixture/tests/ordinary.rs"
    printf '%s\n' \
        'test:' \
        '    cargo test --tests' \
        >"$fixture/justfile"
    printf '%s\n' \
        'name: CI' \
        'jobs: {}' \
        >"$fixture/.github/workflows/ci.yml"
    git -C "$fixture" add .
}

expect_failure() {
    local expected=$1
    if output=$("$checker" --root "$fixture" --manifest test-lanes.tsv 2>&1); then
        echo "expected policy failure containing: $expected" >&2
        exit 1
    fi
    if [[ "$output" != *"$expected"* ]]; then
        echo "wrong policy failure; expected '$expected', got:" >&2
        echo "$output" >&2
        exit 1
    fi
}

write_valid_fixture
"$checker" --root "$fixture" --manifest test-lanes.tsv

printf '%s\n' \
    '#[test]' \
    '#[ignore = "later"]' \
    'fn hidden_test() {}' \
    >"$fixture/tests/ignored.rs"
git -C "$fixture" add tests/ignored.rs
expect_failure '#[ignore]'
git -C "$fixture" rm -qf tests/ignored.rs

printf '%s\n' \
    '/// ```rust,ignore' \
    '/// let value = 1;' \
    '/// ```' \
    'pub fn ignored_docs() {}' \
    >"$fixture/tests/ignored_docs.rs"
git -C "$fixture" add tests/ignored_docs.rs
expect_failure 'ignored Rust documentation fence'
git -C "$fixture" rm -qf tests/ignored_docs.rs

printf '%s\n' \
    '/// ```rust, ignore' \
    '/// let value = 1;' \
    '/// ```' \
    'pub fn spaced_ignored_docs() {}' \
    >"$fixture/tests/spaced_ignored_docs.rs"
git -C "$fixture" add tests/spaced_ignored_docs.rs
expect_failure 'ignored Rust documentation fence'
git -C "$fixture" rm -qf tests/spaced_ignored_docs.rs

printf '%s\n' \
    '#[test]' \
    'fn runtime_skip() {' \
    '    eprintln!("SKIP missing fixture");' \
    '}' \
    >"$fixture/tests/runtime_skip.rs"
git -C "$fixture" add tests/runtime_skip.rs
expect_failure 'returning success'
git -C "$fixture" rm -qf tests/runtime_skip.rs

printf '%s\n' \
    '#[test]' \
    'fn unowned_test() {}' \
    >"$fixture/unowned.rs"
git -C "$fixture" add unowned.rs
expect_failure 'no test lane owns'
git -C "$fixture" rm -qf unowned.rs

printf '%s\n' \
    'test:' \
    '    cargo test -- --skip broken_test' \
    >"$fixture/justfile"
git -C "$fixture" add justfile
expect_failure 'test-selection bypass'

printf '%s\n' \
    '# pattern<TAB>lane<TAB>command' \
    $'tests/*.rs\tpr\tcargo test -- --ignored' \
    >"$fixture/test-lanes.tsv"
git -C "$fixture" add test-lanes.tsv
expect_failure 'manifest lane'
write_valid_fixture

printf '%s\n' \
    'test:' \
    '    cargo test --tests' \
    >"$fixture/justfile"
printf '%s\n' \
    'name: CI' \
    'jobs:' \
    '  test:' \
    "    if: !contains(github.event.head_commit.message, '[skip ci]')" \
    '    runs-on: ubuntu-latest' \
    >"$fixture/.github/workflows/ci.yml"
git -C "$fixture" add justfile .github/workflows/ci.yml
expect_failure 'custom skip-CI condition'

printf '%s\n' \
    'test:' \
    '    cargo test --tests' \
    >"$fixture/justfile"
printf '%s\n' \
    'name: CI' \
    'jobs:' \
    '  test:' \
    '    if: github.event_name == '\''workflow_dispatch'\''' \
    '    runs-on: ubuntu-latest' \
    >"$fixture/.github/workflows/ci.yml"
git -C "$fixture" add justfile .github/workflows/ci.yml
expect_failure 'workflow conditions may silently suppress a lane'

printf '%s\n' \
    'test:' \
    '    if command -v java >/dev/null; then' \
    '        cargo test --tests' \
    '    fi' \
    >"$fixture/justfile"
printf '%s\n' \
    'name: CI' \
    'jobs: {}' \
    >"$fixture/.github/workflows/ci.yml"
git -C "$fixture" add justfile .github/workflows/ci.yml
expect_failure 'conditionally executed'

printf '%s\n' \
    'test:' \
    '    cargo test --tests || true' \
    >"$fixture/justfile"
printf '%s\n' \
    'name: CI' \
    'jobs: {}' \
    >"$fixture/.github/workflows/ci.yml"
git -C "$fixture" add justfile .github/workflows/ci.yml
expect_failure 'failures may not be converted to success'

printf '%s\n' \
    'test:' \
    '    cargo test --tests' \
    >"$fixture/justfile"
printf '%s\n' \
    'name: CI' \
    'jobs:' \
    '  test:' \
    '    steps:' \
    '      - run: cargo test --tests' \
    '        continue-on-error: true' \
    >"$fixture/.github/workflows/ci.yml"
git -C "$fixture" add justfile .github/workflows/ci.yml
expect_failure 'continue-on-error'

# Back to a clean slate: the cases below assert on shell scripts, so nothing
# left over from the justfile/workflow cases above may still be failing.
write_valid_fixture
"$checker" --root "$fixture" --manifest test-lanes.tsv

# A conditional that genuinely gates a benchmark must still be caught.
mkdir -p "$fixture/scripts"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'if [ "$MODE" = fast ]; then' \
    '    cargo bench -p thing' \
    'fi' \
    >"$fixture/scripts/conditional.sh"
git -C "$fixture" add scripts/conditional.sh
expect_failure 'may not be conditionally executed'
git -C "$fixture" rm -qf scripts/conditional.sh

# …but an `if` inside a heredoc belongs to the embedded language, not the
# shell. It has no `fi`, so a naive scanner leaves the block open and flags
# every later command in the file.
mkdir -p "$fixture/scripts"  # `git rm` prunes the now-empty directory above
cat >"$fixture/scripts/heredoc.sh" <<'FIXTURE'
#!/usr/bin/env bash
python3 - <<'PY'
if 1 == 0:
    raise SystemExit("unreachable")
PY
cargo bench -p thing
FIXTURE
git -C "$fixture" add scripts/heredoc.sh
"$checker" --root "$fixture" --manifest test-lanes.tsv
git -C "$fixture" rm -qf scripts/heredoc.sh

echo "test policy self-test: PASS"
