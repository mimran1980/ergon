#!/usr/bin/env bash
# check-book-anim.py must pass a well-formed fixture book and fail, naming the
# problem, for each way an animation can rot: an embed with no module, a module
# no page embeds, a syntax error, an external URL, a missing source citation,
# a register() slug that differs from the file name, and an unregistered runtime.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-book-anim.py"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

# make_book DIR: a minimal book that passes.
make_book() {
    local dir=$1
    mkdir -p "$dir/src/anim"
    cat > "$dir/book.toml" <<'EOF'
[output.html]
additional-js = ["src/anim/anim.js"]
additional-css = ["src/anim/anim.css"]
EOF
    printf '(() => { window.Anim = { register() {} }; })();\n' > "$dir/src/anim/anim.js"
    printf '.ergon-anim { color: var(--fg); }\n' > "$dir/src/anim/anim.css"
    cat > "$dir/src/anim/demo.js" <<'EOF'
// Demo.
// Sources:
//   sbe/src/lib.rs:1   what it shows
Anim.register('demo', (stage, api) => ({ steps: [{ caption: 'one', show() {} }] }));
EOF
    cat > "$dir/src/page.md" <<'EOF'
# Page

<div class="ergon-anim" data-anim="demo" data-title="Demo"></div>

Text alternative.
EOF
}

# expect_fail NAME MESSAGE: the checker must fail on $fixture/NAME and print MESSAGE.
expect_fail() {
    local name=$1 message=$2
    if python3 "$checker" "$fixture/$name" >"$fixture/$name.out" 2>&1; then
        echo "expected '$name' to fail" >&2
        cat "$fixture/$name.out" >&2
        exit 1
    fi
    if ! grep -F -q -- "$message" "$fixture/$name.out"; then
        echo "'$name' failed without naming the problem ('$message'):" >&2
        cat "$fixture/$name.out" >&2
        exit 1
    fi
}

make_book "$fixture/good"
python3 "$checker" "$fixture/good" >"$fixture/good.out"
grep -F -q 'check-book-anim: PASS (1 module(s), 1 embedded)' "$fixture/good.out"

make_book "$fixture/missing"
printf '\n<div class="ergon-anim" data-anim="nope"></div>\n' >> "$fixture/missing/src/page.md"
expect_fail missing "embeds 'nope' but"

# An embed tag split over lines is still an embed.
make_book "$fixture/multiline"
printf '\n<div class="ergon-anim"\n     data-anim="gone"></div>\n' >> "$fixture/multiline/src/page.md"
expect_fail multiline "embeds 'gone' but"

make_book "$fixture/orphan"
sed 's/demo/orphan/g' "$fixture/orphan/src/anim/demo.js" > "$fixture/orphan/src/anim/orphan.js"
expect_fail orphan "module 'orphan' is not embedded on any page"

make_book "$fixture/syntax"
printf 'Anim.register("demo", () => {\n' >> "$fixture/syntax/src/anim/demo.js"
expect_fail syntax 'node --check failed'

make_book "$fixture/url"
printf "const font = 'https://fonts.example.com/x.woff2';\n" >> "$fixture/url/src/anim/demo.js"
expect_fail url 'external URL'

make_book "$fixture/cssurl"
printf ".x { background: url(//cdn.example.com/a.png); }\n" >> "$fixture/cssurl/src/anim/anim.css"
expect_fail cssurl 'external URL'

make_book "$fixture/nocite"
sed -i.bak '/Sources:/,/sbe\/src/d' "$fixture/nocite/src/anim/demo.js"
rm "$fixture/nocite/src/anim/demo.js.bak"
expect_fail nocite "needs a 'Sources:' line"

make_book "$fixture/slug"
sed -i.bak "s/register('demo'/register('other'/" "$fixture/slug/src/anim/demo.js"
rm "$fixture/slug/src/anim/demo.js.bak"
expect_fail slug "must call Anim.register('demo'"

make_book "$fixture/toml"
printf '[output.html]\n' > "$fixture/toml/book.toml"
expect_fail toml 'src/anim/anim.js is not registered'

make_book "$fixture/disabled"
printf 'const b = document.createElement("button"); b.disabled = true;\n' >> "$fixture/disabled/src/anim/anim.js"
expect_fail disabled 'use aria-disabled'

# The SVG namespace URI is never fetched and stays allowed.
make_book "$fixture/ns"
printf "const NS = 'http://www.w3.org/2000/svg';\n" >> "$fixture/ns/src/anim/demo.js"
python3 "$checker" "$fixture/ns" >"$fixture/ns.out"

echo "test-book-anim: PASS"
