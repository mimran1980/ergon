#!/usr/bin/env bash
# Fuzz corpus replay for `just test-all` and the release: every libFuzzer
# target replays its corpus, then fuzzes for 30 s.
#
# One `cargo fuzz build` compiles every target with the flags `cargo fuzz run`
# uses, then the six targets run at the same time (each libFuzzer process is
# single-threaded). Each keeps its own corpus and artifact directory and its
# full 30 s budget; any failing target fails the run (scripts/run-parallel.sh).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
targets=(
    generated_verify
    nested_group_decode
    bulk_decode
    flat_group_decode
    any_message_frame_cursor
    schema_parse
)

cd "$root/sbe/fuzz"
cargo +nightly fuzz build

commands=()
for target in "${targets[@]}"; do
    commands+=("cargo +nightly fuzz run $target -- -max_total_time=30")
done
exec bash "$root/scripts/run-parallel.sh" "${commands[@]}"
