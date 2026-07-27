#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
corpus_root="${1:-${repo_root}/sbe/fuzz/corpus}"

mkdir -p \
  "${corpus_root}/schema_parse" \
  "${corpus_root}/generated_verify" \
  "${corpus_root}/any_message_frame_cursor" \
  "${corpus_root}/nested_group_decode"

while IFS= read -r schema; do
  cp "${schema}" "${corpus_root}/schema_parse/$(basename "${schema}")"
done < <(find "${repo_root}/sbe/tests/fixtures/schemas" -type f -name '*.xml' | sort)

while IFS= read -r frame; do
  name="$(basename "${frame}")"
  cp "${frame}" "${corpus_root}/generated_verify/${name}"
  cp "${frame}" "${corpus_root}/any_message_frame_cursor/${name}"
  cp "${frame}" "${corpus_root}/nested_group_decode/${name}"
done < <(find "${repo_root}/sbe/tests/fixtures" -maxdepth 1 -type f -name '*.sbe' | sort)

echo "seeded fuzz corpora under ${corpus_root}"
