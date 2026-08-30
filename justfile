# ergon — reproducible workspace gates
#
# `ergo-aeron-cluster-test-harness` is a workspace member excluded from
# `--all-features` because it compiles the Java ClusterLauncher. Product
# `ergo-aeron-cluster` has no advertised extra features.
#
# Samples are standalone packages, not workspace members.

import 'just/samples.just'
import 'just/aeron.just'
import 'just/housekeeping.just'
import 'just/book.just'

# Default: list available commands.
default:
    @just --list

# ── build ─────────────────────────────────────────────────────────────────

# Compile product workspace + sample harnesses (no tests, no Java jars).
build:
    cargo build --workspace --all-features --exclude ergo-aeron-cluster-test-harness
    cargo build -p ergo-aeron-cluster
    cd samples/exchange-example && cargo build
    cd samples/cluster-ha-orderbook && cargo build
    cd samples/cluster-rfq && cargo build

# ── check ─────────────────────────────────────────────────────────────────

# Enforce test ownership and prove the policy checker catches every bypass.
policy:
    bash scripts/tests/test-test-policy.sh
    bash scripts/tests/test-quality-ratchets.sh
    bash scripts/tests/test-repository-hygiene.sh
    bash scripts/tests/test-public-api.sh
    bash scripts/tests/test-packaged-cluster-features.sh
    ./scripts/check-test-policy.sh
    ./scripts/check-mutation-config.sh

# Every cheap correctness gate in one pass — the ones that otherwise only run
# inside `just release`.
#
# Why this exists: four gates were found broken in one sitting (2026-08), three
# of them already broken on main. Each is reachable only from a ~2h release that
# nobody completes, so breakage accumulated invisibly:
#   - package-bench-artifacts read the run-manifest at the wrong path and could
#     NEVER pass; its own self-test asserted the same wrong path
#   - check-book-fences had a stale allowlist and 4 undocumented ignore fences
#   - the sbe-tool benchmark comparators had drifted from upstream
#   - cargo mutants' baseline failed because cap_lints silenced a deny-lint fixture
#
# Run this before starting a release. It takes minutes, not hours, and every
# entry either passes or tells you exactly what drifted.
preflight:
    # Cheapest gate, and one that actually bit: the 0.1.17 release aborted at
    # step 2/8 on sample-crate fmt drift that had been sitting on the branch.
    # `cargo fmt --all` does NOT reach samples/exchange-example — it is a
    # separate workspace, so it needs its own invocation (same as `just fmt`).
    @echo "=== formatting ==="
    cargo fmt --all --check
    cd samples/exchange-example && cargo fmt --check
    @echo "=== clippy (same workspace set as CI) ==="
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster-test-harness -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    @echo "=== policy + ratchet self-tests (prove the checkers can fail) ==="
    bash scripts/tests/test-test-policy.sh
    bash scripts/tests/test-quality-ratchets.sh
    bash scripts/tests/test-repository-hygiene.sh
    bash scripts/tests/test-public-api.sh
    bash scripts/tests/test-generated-public-api.sh
    bash scripts/tests/test-instruction-probe-pairs.sh
    bash scripts/tests/test-packaged-cluster-features.sh
    bash scripts/test-package-bench-artifacts.sh
    @echo "=== repository + docs ==="
    ./scripts/check-repository-hygiene.sh
    ./scripts/check-book-fences.sh
    ./scripts/check-book-content.sh
    ./scripts/check-generated-public-api.sh
    bash scripts/check-packaged-cluster-features.sh --list
    @echo "=== checked-in generated artifacts still match their source ==="
    ./scripts/regenerate-golden.sh --check
    ./scripts/regenerate-sbe-tool-reference.sh --check
    ./scripts/regenerate-sbe-benchmark-reference.sh --check
    @echo "=== config ==="
    ./scripts/check-mutation-config.sh
    @echo ""
    @echo "audit: all cheap gates pass"

# Full local check: hygiene, format, clippy, tests (no Java / Aeron jars).
check-local: policy
    ./scripts/check-repository-hygiene.sh
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster-test-harness -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    cargo test --workspace --all-features --exclude ergo-aeron-cluster-test-harness -- --test-threads=1
    cargo test -p ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --doc
    cd samples/exchange-example && cargo fmt --check
    cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo fmt --check
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    cd samples/cluster-rfq && cargo fmt --check
    cd samples/cluster-rfq && cargo clippy --all-targets -- -D warnings

# Product-only gate: fmt, clippy, and tests for the two publishable prototype crates only.
check-products: policy
    ./scripts/regenerate-golden.sh --check
    cargo fmt --all --check
    cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    ./scripts/regenerate-sbe-tool-reference.sh --check
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test --manifest-path sbe/miri-fixtures/Cargo.toml
    cargo test -p ergo-sbe --all-features -- --test-threads=1
    cargo test -p ergo-sbe --doc --all-features -- --test-threads=1
    cargo test -p ergo-sbe --test docs_validation_test --all-features -- --test-threads=1
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    cargo test -p ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --doc
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-aeron-cluster --no-deps
    ./scripts/check-book-fences.sh

# Strict rustdoc for a *generated consumer*. Crate-only rustdoc never sees the
# generated flyweight API, which is the artifact users actually read, so a
# broken intra-doc link or a link to a type the generator does not emit can only
# be caught here.
check-generated-rustdoc:
    RUSTDOCFLAGS='-D warnings -D rustdoc::broken_intra_doc_links' cargo doc --manifest-path samples/sbe-feature-tour/Cargo.toml --no-deps

# Verify that all public structs/enums in the generated golden have doc comments.
# Uses syn-based parsing (see sbe/tests/generated_docs_test.rs).
check-generated-docs:
    cargo test -p ergo-sbe --test generated_docs_test --all-features -- --test-threads=1

# Sample crates gate (unpublished).
check-samples: policy check-generated-rustdoc
    cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    cd samples/cluster-rfq && cargo clippy --all-targets -- -D warnings

# Pre-release check: comprehensive suite + coverage + bench compile + dry-run publish.
release-check: test check-coverage check-generated-rustdoc
    cargo bench -p ergo-sbe-benchmarks --no-run
    cargo bench -p ergo-aeron-cluster --no-run
    ./scripts/check-book-fences.sh
    ./scripts/check-book-content.sh
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-aeron-cluster --no-deps
    cargo publish -p ergo-sbe --dry-run --allow-dirty
    bash scripts/check-packaged-cluster-features.sh --list
    bash scripts/check-packaged-cluster-features.sh --unpack --package ergo-sbe
    # ergo-aeron-cluster unpack-and-build waits until ergo-sbe is on crates.io (publish step below).
    @echo "=== bench-cold (generated-size + compile-time diagnostic) ==="
    bash scripts/measure-codegen-cold-path.sh
    @echo "release-check: product crates pass, benches compile, ergo-sbe dry-run publish OK"

# Release when the benchmark gates were already proven on a QUIET machine.
#
# Why this exists: `just release` runs the benchmark gates ~40 minutes into its
# own run, so they execute on a machine hot from compiling and testing. On the
# tightest maintained pair (encode_scalar_body_only, ergon ~0.99 against a hard
# 1.00 ceiling) that measurement noise alone decides pass/fail — measured 0.985
# idle vs 1.027-1.104 mid-release, on identical code.
#
# This recipe does NOT weaken anything. Every gate still runs except the bench
# re-run, and step 9/9 (`package-bench-artifacts.sh`) fails closed unless BOTH
# profile run-manifests stamp the current HEAD. So publishing without fresh,
# commit-matched benchmark evidence remains impossible — the evidence just has
# to be produced on an idle machine first:
#
#   just bench-cluster && just bench && just bench-historic   # quiet machine
#   just release-verified                                     # same commit
#
# Re-run the benches if you commit anything afterwards; the manifest check will
# reject stale evidence rather than let it through.
release-verified: _check-release-notes
    @echo "=== 0/8 confirm benchmark evidence exists for HEAD ==="
    bash scripts/package-bench-artifacts.sh /tmp/ergon-bench-evidence-precheck
    @rm -rf /tmp/ergon-bench-evidence-precheck
    just _release-pre
    just _release-post

# Full release gate: test + bench → publish → tag → GitHub release → bump.
# The LLM must bump the version + write changelog + write release notes before
# calling this. The version is read from workspace Cargo.toml.
#
# Runs the benchmark gates inline, ~40 min in, on a machine hot from the test
# suite. If the tightest maintained ratio fails there, re-measure on an idle
# machine before believing it, then use `just release-verified`.
release: _check-release-notes
    just clean
    just _release-pre
    @echo "=== 5/8 benchmark gates (cluster + parity + historic) ==="
    just bench-cluster
    just bench
    just bench-historic
    just _release-post

# Gates 1-4. Shared by `release` and `release-verified`.
_release-pre:
    @echo "=== 1/8 supply-chain audit ==="
    # No `-` prefix: these must BLOCK the release. They previously ignored their
    # exit codes, so a live RUSTSEC advisory printed "error: 1 vulnerability
    # found!" and the release published straight past it. Record unreachable
    # advisories as documented ignores (deny.toml AND .cargo/audit.toml — the
    # two tools read different files) rather than re-muting the gate.
    cargo deny check
    cargo audit
    @echo "=== 2/8 test suite (clippy + tests + samples + cluster) ==="
    just test
    @echo "=== 3/8 miri UB detection ==="
    cargo +nightly miri test --manifest-path sbe/miri-fixtures/Cargo.toml
    @echo "=== 4/8 fuzz corpus replay ==="
    cd sbe/fuzz && cargo +nightly fuzz run generated_verify -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run nested_group_decode -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run bulk_decode -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run flat_group_decode -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run any_message_frame_cursor -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run schema_parse -- -max_total_time=30

# Gates 6-9 plus publish/tag/release. Shared by both release paths.
_release-post:
    # Mutation testing is deliberately NOT on the release path. `cargo mutants
    # --jobs 1` takes ~16 h and builds a full tree per mutant, which on a 16 GB
    # machine exhausted the disk and killed the 0.1.17 release mid-run. It stays
    # a real, runnable gate — `just check-mutation` — but it is manual, and the
    # cheap config check is what the release enforces.
    @echo "=== 6/8 mutation configuration (config only; full run is manual) ==="
    ./scripts/check-mutation-config.sh
    @echo "=== 7/8 reference reproducibility ==="
    just check-sbe-references
    just check-bench-reference
    @echo "=== 8/9 release check ==="
    just release-check
    @echo "=== 9/9 package benchmark evidence (fail-closed) ==="
    mkdir -p release-assets
    bash scripts/package-bench-artifacts.sh release-assets
    @echo "=== publish ergo-sbe ==="
    cargo publish -p ergo-sbe
    @echo "=== packaged cluster features (fail-closed) ==="
    bash scripts/check-packaged-cluster-features.sh --unpack
    @echo "=== publish ergo-aeron-cluster ==="
    cargo publish -p ergo-aeron-cluster
    @echo "=== tag ==="
    just _tag
    @echo "=== GitHub release (with bench archives) ==="
    gh release create v$(cargo metadata --format-version 1 --no-deps 2>/dev/null | jq -r '.packages[] | select(.name == "ergo-sbe") | .version') --title "ergon v$(cargo metadata --format-version 1 --no-deps 2>/dev/null | jq -r '.packages[] | select(.name == "ergo-sbe") | .version')" --notes-file /tmp/ergon-release-notes.md release-assets/*.tar.gz
    @echo "=== release v$(cargo metadata --format-version 1 --no-deps 2>/dev/null | jq -r '.packages[] | select(.name == "ergo-sbe") | .version') complete ==="

_tag:
    git tag v$(cargo metadata --format-version 1 --no-deps 2>/dev/null | jq -r '.packages[] | select(.name == "ergo-sbe") | .version')
    git push origin v$(cargo metadata --format-version 1 --no-deps 2>/dev/null | jq -r '.packages[] | select(.name == "ergo-sbe") | .version')

_check-release-notes:
    @test -f /tmp/ergon-release-notes.md || (echo "ERROR: write release notes to /tmp/ergon-release-notes.md first" >&2 && exit 1)

# Run cargo-deny and cargo-audit supply-chain checks.
audit:
    cargo deny check
    cargo audit

# ── test ──────────────────────────────────────────────────────────────────

# Comprehensive test suite: unit, integration, doctests/rustdoc, Java cluster
# lifecycle tests, sample tests, and benchmark compilation. Missing Java,
# Gradle, jars, or another required dependency is a failure.
test: policy
    @echo "=== 1/7 fmt ==="
    cargo fmt --all --check
    @echo "=== 2/7 clippy (workspace + samples) ==="
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster-test-harness -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    @echo "=== 3/7 unit + integration tests ==="
    ./scripts/regenerate-golden.sh --check
    ./scripts/regenerate-sbe-tool-reference.sh --check
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test --manifest-path sbe/miri-fixtures/Cargo.toml
    cargo test --workspace --all-features --exclude ergo-aeron-cluster-test-harness -- --test-threads=1
    cargo test -p ergo-aeron-cluster -- --test-threads=1
    @echo "=== 4/7 product doctests + rustdoc (-D warnings) + docs_validation ==="
    cargo test -p ergo-sbe --doc --all-features -- --test-threads=1
    cargo test -p ergo-sbe --test docs_validation_test --all-features -- --test-threads=1
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    cargo test -p ergo-aeron-cluster --doc
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-aeron-cluster --no-deps
    @echo "=== 5/7 sample tests ==="
    cd samples/l3-book && cargo test -- --test-threads=1
    cd samples/sbe-feature-tour && cargo test -- --test-threads=1
    cd samples/sbe-codegen-examples && cargo run --example flyweight >/dev/null && cargo run --example domain_objects >/dev/null
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-rfq && cargo build --examples
    @echo "=== 6/7 Aeron Cluster Java lifecycle + HA sample ==="
    just build-aeron-jars
    cargo test -p ergo-aeron-cluster-test-harness -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo test --features test-harness -- --test-threads=1
    @echo "=== 7/7 benchmark compilation ==="
    cargo bench -p ergo-sbe-benchmarks --no-run
    cargo bench -p ergo-aeron-cluster --no-run
    cd samples/l3-book && cargo bench --no-run
    @echo ""
    @echo "=== test: complete ==="

# Workspace unit tests only.
test-unit: policy
    cargo test --workspace --all-features --exclude ergo-aeron-cluster-test-harness -- --test-threads=1
    cargo test -p ergo-aeron-cluster -- --test-threads=1

# Every test gate: standard suite + Miri UB detection + fuzz corpus replay.
# A green `just test-all` means every test ran and every test passed.
test-all: policy test
    @echo "=== 8/9 miri (UB detection) ==="
    cargo +nightly miri test --manifest-path sbe/miri-fixtures/Cargo.toml
    @echo "=== 9/9 fuzz corpus replay ==="
    cd sbe/fuzz && cargo +nightly fuzz run generated_verify -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run nested_group_decode -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run bulk_decode -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run flat_group_decode -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run any_message_frame_cursor -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run schema_parse -- -max_total_time=30
    @echo ""
    @echo "=== test-all: complete ==="

# Rebuild every pinned official sbe-tool reference in a temporary directory.
check-sbe-references:
    ./scripts/regenerate-sbe-tool-reference.sh --check

# Regenerate the checked-in generated-code golden through a non-test command.
update-golden:
    ./scripts/regenerate-golden.sh

# Rebuild the generated-code golden in a temporary file and compare bytes.
check-golden:
    ./scripts/regenerate-golden.sh --check

# Compile all libFuzzer targets and run the deterministic corpus replay.
check-fuzz:
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test -p ergo-sbe --test hostile_input_replay_test -- --test-threads=1

# Run and compare line/function/region coverage with the checked-in ratchet.
check-coverage:
    ./scripts/check-coverage-ratchet.sh

# Mutate parser/resolver/codegen critical paths and reject missed or timed-out
# mutants. This is a MANUAL gate — too slow for CI (~16 h with --jobs 1).
# Run locally before landing codegen changes; use --jobs 1 to avoid
# exhausting disk space with parallel build trees.
check-mutation:
    ./scripts/check-mutation-config.sh
    cargo mutants --jobs 1
    ./scripts/check-mutation-ratchet.sh

# ── formatting ─────────────────────────────────────────────────────────────

# Format all handwritten source.
fmt:
    cargo fmt --all
    cd samples/exchange-example && cargo fmt
    cd samples/cluster-ha-orderbook && cargo fmt
    cd samples/cluster-rfq && cargo fmt

# Auto-fix: fmt + clippy --fix (same feature split as check).
fix:
    just fmt
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster-test-harness --fix --allow-dirty --allow-staged -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/exchange-example && cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/cluster-rfq && cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings

# ── benchmarks ─────────────────────────────────────────────────────────────

# Benchmark parity — ergo-sbe vs sbe-tool head-to-head.
# Gate: every maintained ergon/sbe-tool ratio must stay at or below a literal
# 1.00 — no noise tolerance — in BOTH profiles. Both are blocking: a regression
# that only appears without LTO is still a regression for downstream consumers.
# Each invocation owns a unique result root, so the gate can only read estimates
# produced by that same invocation.
# Uses trusted direct wraps for fair comparison (sbe-tool's wrap does not validate).
bench:
    ./scripts/run-sbe-bench.sh

# Group-codegen comparison under both optimization profiles. sbe-tool is
# intentionally measured in both: the audit found it stable without LTO while
# pre-fix ergon regressed because generated entry setters did not inline.
bench-groups:
    cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench
    cargo bench -p ergo-sbe-benchmarks --bench group_encode_decimal_bench
    CARGO_TARGET_DIR=target/bench-no-lto CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench

# Historic ergo-only benchmarks — null-as-option, converters, bulk_add.
# Gate: compares against stored baselines to detect silent regressions.
bench-historic:
    cargo bench -p ergo-sbe-benchmarks --bench ergo_historic_bench
    ./scripts/check-bench-historic.sh

# Regenerate historic baselines after verifying no regressions.
bench-historic-update:
    cargo bench -p ergo-sbe-benchmarks --bench ergo_historic_bench
    scripts/update-historic-baseline.sh

# Expanded non-gating codec matrix and offset/alignment diagnostics.
bench-diagnostics:
    cargo bench -p ergo-sbe-benchmarks --bench codec_matrix_bench
    cargo bench -p ergo-sbe-benchmarks --bench alignment_bench
    cargo bench -p ergo-sbe-benchmarks --bench cold_path_bench
    cargo bench -p ergo-sbe-benchmarks --bench latency_distribution

# Fresh generated-crate compile/source/binary-size report.
bench-cold:
    ./scripts/measure-codegen-cold-path.sh

# Mechanism-level evidence: raw Callgrind instruction/branch counts plus
# disassembly for the named perf-probe symbols, in both optimisation profiles.
# Requires a Linux host with Valgrind and llvm-objdump; it fails closed rather
# than degrading to a timing harness, and fails if ergon Ir/op exceeds sbe-tool.
bench-instructions:
    ./scripts/run-sbe-instruction-probes.sh --all-profiles

# Regenerate the two sbe-tool comparators the head-to-head benches measure
# against, from the pinned simple-binary-encoding submodule.
update-bench-reference:
    ./scripts/regenerate-sbe-benchmark-reference.sh

# Verify those comparators are reproducible without touching tracked files.
check-bench-reference:
    ./scripts/regenerate-sbe-benchmark-reference.sh --check

# Cluster codec benchmarks (ergo-sbe vs sbe-tool head-to-head).
# Criterion results must be separated by profile: CARGO_TARGET_DIR only moves
# the binary tree. Relative CARGO_TARGET_DIR / CRITERION_HOME can resolve under
# the package dir (cluster/) rather than the workspace root, so both must be
# absolute workspace paths. package-bench-artifacts expects:
#   LTO    → <repo>/target/criterion
#   no-LTO → <repo>/target/bench-no-lto/criterion
bench-cluster:
    #!/usr/bin/env bash
    set -euo pipefail
    root="{{justfile_directory()}}"
    lto_crit="$root/target/criterion"
    no_lto_crit="$root/target/bench-no-lto/criterion"
    no_lto_target="$root/target/bench-no-lto"
    mkdir -p "$lto_crit" "$no_lto_crit"
    commit="$(git -C "$root" rev-parse HEAD)"
    rustc_v="$(rustc --version)"
    target="$(rustc -vV | sed -n 's/^host: //p')"
    run_id="cluster-$(date -u +%Y%m%dT%H%M%SZ)-$(git -C "$root" rev-parse --short HEAD)"
    stamp="$root/scripts/stamp-cluster-bench-manifest.sh"
    # LTO-on (release profile: lto=true, codegen-units=1)
    CRITERION_HOME="$lto_crit" cargo bench -p ergo-aeron-cluster
    "$stamp" "$lto_crit" lto "$run_id" "$commit" "$rustc_v" "$target"
    # LTO-off — publish both profiles per the benchmark fairness matrix
    CARGO_TARGET_DIR="$no_lto_target" \
      CARGO_PROFILE_BENCH_LTO=false \
      CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 \
      CRITERION_HOME="$no_lto_crit" \
      cargo bench -p ergo-aeron-cluster
    "$stamp" "$no_lto_crit" no-lto "$run_id" "$commit" "$rustc_v" "$target"
    echo ""
    echo "=== Gate (LTO) ==="
    ./scripts/check-bench-gate.sh "$lto_crit" 0 cluster --run-id "$run_id"
    echo "=== Gate (no-LTO) ==="
    ./scripts/check-bench-gate.sh "$no_lto_crit" 0 cluster --run-id "$run_id"
