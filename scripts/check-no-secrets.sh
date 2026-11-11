#!/usr/bin/env bash
# Fails if a repository tracks (or, with --staged, is about to commit) a
# working note or a secret:
#   - plans, backlogs, design specs and handovers (docs/plans/, docs/backlog/,
#     docs/specs/, HANDOVER*.md, TODO-PLAN*.md): working notes live outside
#     every repository;
#   - build output (any target/ directory);
#   - secret-shaped text: cloud and GitHub keys, private keys, Slack and
#     OpenAI tokens, "sudo password is ...", a password assigned a literal;
#   - any literal listed one per line in a local denylist kept outside the
#     repository (default ~/.config/ergon/secret-denylist, mode 600), so a
#     known password is caught without the repository ever naming it.
#   check-no-secrets.sh [--staged] [repo dir]     default: this repository
set -euo pipefail
staged=0
if [[ ${1:-} == --staged ]]; then staged=1; shift; fi
repo=${1:-$(cd "$(dirname "$0")/.." && pwd)}
denylist=${SECRET_DENYLIST:-$HOME/.config/ergon/secret-denylist}

paths='(^|/)(docs/(plans|backlog|specs)/|target/)|(^|/)(HANDOVER|TODO-PLAN)[^/]*\.md$'
shapes='AKIA[0-9A-Z]{16}|ASIA[0-9A-Z]{16}|-----BEGIN ([A-Z]+ )?PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{40,}|xox[abpr]-[A-Za-z0-9-]{10,}|sk-[A-Za-z0-9]{32,}|aws_secret_access_key *= *[A-Za-z0-9/+]{30,}|sudo password (for [^ ]+ )?is [^ ]+|[Pp][Aa][Ss][Ss][Ww][Oo][Rr][Dd] *[:=] *"[^"$<{ ]{8,}"'

if ((staged)); then
    files=$(git -C "$repo" diff --cached --name-only --diff-filter=ACMR)
    grep_base=(git -C "$repo" grep --cached -I)
else
    files=$(git -C "$repo" ls-files)
    grep_base=(git -C "$repo" grep -I)
fi

bad=0
while IFS= read -r f; do
    [[ -z $f ]] && continue
    if [[ $f =~ $paths ]]; then
        echo "check-no-secrets: working note or build output must not be committed: $f" >&2
        bad=1
    fi
done <<< "$files"

# The pattern list itself and its test name the shapes; nothing else may.
self=(':!scripts/check-no-secrets.sh' ':!scripts/tests/test-no-secrets.sh' ':!scripts/no-secrets-allow.tsv')
if [[ -n $files ]]; then
    hits=$("${grep_base[@]}" -n -E -e "$shapes" -- . "${self[@]}" 2>/dev/null || true)
    # A PEM private-key header is allowed only in the files listed, with why,
    # in scripts/no-secrets-allow.tsv (test vectors, generated test keys).
    allow="$repo/scripts/no-secrets-allow.tsv"
    if [[ -n $hits && -r $allow ]]; then
        kept=""
        while IFS= read -r h; do
            p=${h%%:*}
            if [[ $h =~ BEGIN\ ([A-Z]+\ )?PRIVATE\ KEY ]] && grep -qxF "$p" <(cut -f1 "$allow" | grep -v '^#'); then continue; fi
            kept+="$h"$'\n'
        done <<< "$hits"
        hits=${kept%$'\n'}
    fi
    if [[ -n $hits ]]; then
        # Print where, never what: the match may be a live secret.
        cut -d: -f1,2 <<< "$hits" | sed 's/^/check-no-secrets: secret-shaped text at /' >&2
        bad=1
    fi
    if [[ -r $denylist ]]; then
        while IFS= read -r lit; do
            [[ -z $lit || $lit == \#* ]] && continue
            where=$("${grep_base[@]}" -l -F -e "$lit" -- . "${self[@]}" 2>/dev/null || true)
            if [[ -n $where ]]; then
                sed 's/^/check-no-secrets: denylisted secret in /' <<< "$where" >&2
                bad=1
            fi
        done < "$denylist"
    fi
fi

if ((bad)); then
    echo "check-no-secrets: refused. Keep notes outside the repository and secrets in .env or a secret store." >&2
    exit 1
fi
echo "check-no-secrets: clean"
