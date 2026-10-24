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
import 'just/dpdk.just'
import 'just/nri.just'

# Default: list available commands.
[private]
default:
    @just --list

# ── build ─────────────────────────────────────────────────────────────────

# Compile product workspace + sample harnesses, then check that rusteron, the
# aeron submodule and any jars built in it are one Aeron release (no tests, no Java jars).
build:
    cargo build --workspace --all-features --exclude ergo-aeron-cluster-test-harness --exclude ergon-md-dpdk --exclude ergon-md-dpdk-harness
    cargo build -p ergon-md-dpdk -p ergon-md-dpdk-harness --all-targets
    cargo build -p ergo-aeron-cluster
    cd samples/exchange-example && cargo build
    cd samples/cluster-ha-orderbook && cargo build
    cd samples/cluster-rfq && cargo build
    # Last, so cargo has fetched the rusteron sources the offline check reads.
    ./scripts/check-aeron-version.sh

# ── check ─────────────────────────────────────────────────────────────────

# Enforce test ownership and prove the policy checker catches every bypass.
#
# The scripts are independent (each works in its own mktemp directory), so they
# run at the same time; every one runs to the end and any failure fails the recipe.
policy:
    bash scripts/run-parallel.sh \
        'bash scripts/tests/test-test-policy.sh' \
        'bash scripts/tests/test-quality-ratchets.sh' \
        'bash scripts/tests/test-repository-hygiene.sh' \
        'bash scripts/tests/test-packaged-cluster-features.sh' \
        'bash scripts/tests/test-bench-historic.sh' \
        'bash scripts/tests/test-stale-deny-ignores.sh' \
        'bash scripts/tests/test-cluster-ergo-sbe-features.sh' \
        'bash scripts/tests/test-book-content.sh' \
        'bash scripts/tests/test-rerun-if-changed.sh' \
        'bash scripts/tests/test-aeron-version.sh' \
        'bash scripts/tests/test-publish-set.sh' \
        'bash scripts/tests/test-dpdk-vdev-rootless.sh' \
        './scripts/check-test-policy.sh' \
        './scripts/check-mutation-config.sh' \
        './scripts/check-stale-deny-ignores.sh' \
        './scripts/check-cluster-ergo-sbe-features.sh' \
        './scripts/check-rerun-if-changed.sh' \
        './scripts/check-publish-set.sh'

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
# Every cheap release gate in one pass: run before `just release`
preflight:
    # Cheapest gate, and one that actually bit: the 0.1.17 release aborted at
    # step 2/8 on sample-crate fmt drift that had been sitting on the branch.
    # `cargo fmt --all` does NOT reach samples/exchange-example — it is a
    # separate workspace, so it needs its own invocation (same as `just fmt`).
    @echo "=== formatting ==="
    cargo fmt --all --check
    cd samples/exchange-example && cargo fmt --check
    @echo "=== clippy (same workspace set as CI) ==="
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster-test-harness --exclude ergon-md-dpdk --exclude ergon-md-dpdk-harness -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    cargo clippy -p ergon-md-dpdk -p ergon-md-dpdk-harness --all-targets -- -D warnings
    # The rest are independent (own mktemp directories; cargo's lock orders the
    # ones that build; the two sbe-tool checks share one Gradle project, and
    # test-generated-public-api.sh plants a change in api/generated/car_lean.txt
    # that check-generated-public-api.sh reads, so each pair stays in one
    # command), so they run at the same time. Each runs to the end, its
    # output is printed whole, and any failure fails the recipe.
    @echo "=== policy self-tests, repository + docs, generated artifacts, config ==="
    bash scripts/run-parallel.sh \
        'bash scripts/tests/test-test-policy.sh' \
        'bash scripts/tests/test-quality-ratchets.sh' \
        'bash scripts/tests/test-repository-hygiene.sh' \
        'bash scripts/tests/test-instruction-probe-pairs.sh' \
        'bash scripts/tests/test-packaged-cluster-features.sh' \
        'bash scripts/tests/test-bench-historic.sh' \
        'bash scripts/tests/test-stale-deny-ignores.sh' \
        'bash scripts/tests/test-cluster-ergo-sbe-features.sh' \
        'bash scripts/tests/test-book-content.sh' \
        'bash scripts/tests/test-rerun-if-changed.sh' \
        'bash scripts/tests/test-aeron-version.sh' \
        'bash scripts/tests/test-publish-set.sh' \
        'bash scripts/tests/test-dpdk-vdev-rootless.sh' \
        'bash scripts/test-package-bench-artifacts.sh' \
        './scripts/check-repository-hygiene.sh' \
        './scripts/check-book-fences.sh' \
        './scripts/check-book-content.sh' \
        'bash scripts/tests/test-generated-public-api.sh && ./scripts/check-generated-public-api.sh' \
        'bash scripts/check-packaged-cluster-features.sh --list' \
        './scripts/regenerate-golden.sh --check' \
        './scripts/regenerate-sbe-tool-reference.sh --check && ./scripts/regenerate-sbe-benchmark-reference.sh --check' \
        './scripts/regenerate-codegen-matrix-fixture.py --check' \
        './scripts/check-mutation-config.sh' \
        './scripts/check-stale-deny-ignores.sh' \
        './scripts/check-cluster-ergo-sbe-features.sh' \
        './scripts/check-rerun-if-changed.sh' \
        './scripts/check-aeron-version.sh' \
        './scripts/check-publish-set.sh'
    @echo ""
    @echo "audit: all cheap gates pass"

# Full local check: hygiene, format, clippy, tests (no Java / Aeron jars).
check-local: policy
    ./scripts/check-repository-hygiene.sh
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster-test-harness --exclude ergon-md-dpdk --exclude ergon-md-dpdk-harness -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    cargo clippy -p ergon-md-dpdk -p ergon-md-dpdk-harness --all-targets -- -D warnings
    # ergo-sbe and its benchmarks crate run tests in parallel (scratch crates
    # compile outside cargo's lock: sbe/tests/common/scratch.rs); the rest keep one thread.
    cargo test -p ergo-sbe -p ergo-sbe-benchmarks --all-features
    cargo test --workspace --all-features --exclude ergo-sbe --exclude ergo-sbe-benchmarks --exclude ergo-aeron-cluster --exclude ergo-aeron-cluster-test-harness --exclude ergon-md-dpdk --exclude ergon-md-dpdk-harness -- --test-threads=1
    cargo test -p ergon-md-dpdk -- --test-threads=1
    # runs the cluster doctests too — no separate `--doc` pass needed
    cargo test -p ergo-aeron-cluster -- --test-threads=1
    cd samples/exchange-example && cargo fmt --check
    cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo fmt --check
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    cd samples/cluster-rfq && cargo fmt --check
    cd samples/cluster-rfq && cargo clippy --all-targets -- -D warnings

# Product-only gate: fmt, clippy, and tests for ergo-sbe and ergo-aeron-cluster only (the other published crates are in check-local and test).
check-products: policy
    ./scripts/regenerate-golden.sh --check
    cargo fmt --all --check
    cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    ./scripts/regenerate-sbe-tool-reference.sh --check
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test --manifest-path sbe/miri-fixtures/Cargo.toml
    # runs the ergo-sbe doctests and docs_validation_test too
    cargo test -p ergo-sbe --all-features
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    # runs the cluster doctests too — no separate `--doc` pass needed
    cargo test -p ergo-aeron-cluster -- --test-threads=1
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-aeron-cluster --no-deps
    ./scripts/check-book-fences.sh

# Strict rustdoc for a *generated consumer*. Crate-only rustdoc never sees the
# generated flyweight API, which is the artifact users actually read, so a
# broken intra-doc link or a link to a type the generator does not emit can only
# be caught here.
# Strict rustdoc of a generated consumer crate
check-generated-rustdoc:
    RUSTDOCFLAGS='-D warnings -D rustdoc::broken_intra_doc_links' cargo doc --manifest-path samples/sbe-feature-tour/Cargo.toml --no-deps

# Verify that all public structs/enums in the generated golden have doc comments.
# Uses syn-based parsing (see sbe/tests/generated_docs_test.rs).
# Every public struct and enum in the generated golden has a doc comment
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
release-check: test _release-check-extras
    @echo "release-check: product crates pass, benches compile, dry-run publishes OK"

# What release-check adds to `just test`. The release runs it after its own
# `just test` on the same commit, so nothing here repeats a gate `test`
# already ran: bench compilation (`--no-run`), ergo-sbe and cluster rustdoc
# and check-publish-set (in `policy`) are in `test`.
_release-check-extras: check-coverage check-generated-rustdoc
    ./scripts/check-book-fences.sh
    ./scripts/check-book-content.sh
    cargo publish -p ergo-sbe --dry-run --allow-dirty
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergon-tune --all-features --no-deps
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergon-io --features webpki-roots --no-deps
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergon-md-dpdk --no-deps
    # One multi-package dry run, so ergon-io and ergon-md-dpdk verify against
    # the packaged ergon-tune: none of the three is on crates.io at this
    # version until the release publishes them.
    cargo publish -p ergon-tune -p ergon-io -p ergon-md-dpdk --dry-run --allow-dirty
    bash scripts/check-packaged-cluster-features.sh --list
    bash scripts/check-packaged-cluster-features.sh --unpack --package ergo-sbe
    # ergo-aeron-cluster unpack-and-build waits until ergo-sbe is on crates.io (publish step below).
    @echo "=== bench-cold (generated-size + compile-time diagnostic) ==="
    bash scripts/measure-codegen-cold-path.sh

# Release when the benchmark gates were already proven on a QUIET machine.
#
# Why this exists: `just release` runs the benchmark gates ~40 minutes into its
# own run, so they execute on a machine hot from compiling and testing. On the
# tightest maintained pair (encode_scalar_body_only, against a hard 1.00
# ceiling) noise from that load can decide pass/fail on identical code.
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
# Release with benchmark gates already proven on a quiet machine
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
# Starts from pristine sources (`_reset-sources`: submodules reset to their
# pins, generated sample codecs removed) but keeps the cargo target
# directories: cargo fingerprints source, flags and toolchain, build scripts
# declare rerun-if-changed (check-rerun-if-changed.sh), and the publish dry
# runs and the packaged --unpack check build from the packaged sources. Run
# `just clean` first for a cold build.
#
# Runs the benchmark gates inline, ~40 min in, on a machine hot from the test
# suite. If the tightest maintained ratio fails there, re-measure on an idle
# machine before believing it, then use `just release-verified`.
# Full release: test, bench, publish, tag, GitHub release, version bump
release: _check-release-notes
    just _reset-sources
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
    bash scripts/run-fuzz-replay.sh

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
    # The sbe-tool reference check already ran in step 2 (`just test`) on this commit.
    just check-bench-reference
    @echo "=== 8/9 release check (what step 2 did not already run) ==="
    just _release-check-extras
    @echo "=== 9/9 package benchmark evidence (fail-closed) ==="
    mkdir -p release-assets
    bash scripts/package-bench-artifacts.sh release-assets
    @echo "=== publish ergo-sbe ==="
    cargo publish -p ergo-sbe
    @echo "=== packaged cluster features (fail-closed) ==="
    bash scripts/check-packaged-cluster-features.sh --unpack
    @echo "=== publish ergo-aeron-cluster ==="
    cargo publish -p ergo-aeron-cluster
    @echo "=== publish ergon-tune, ergon-io, ergon-md-dpdk ==="
    # Cargo publishes them in dependency order and waits for crates.io to
    # list ergon-tune before it publishes ergon-io and ergon-md-dpdk.
    cargo publish -p ergon-tune -p ergon-io -p ergon-md-dpdk
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
# Everything: unit, integration, doctests, Java cluster, samples, bench compile
test: policy
    @echo "=== 1/7 fmt ==="
    cargo fmt --all --check
    @echo "=== 2/7 clippy (workspace + samples) ==="
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster-test-harness --exclude ergon-md-dpdk --exclude ergon-md-dpdk-harness -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    cargo clippy -p ergon-md-dpdk -p ergon-md-dpdk-harness --all-targets -- -D warnings
    @echo "=== 3/7 unit + integration tests ==="
    ./scripts/regenerate-golden.sh --check
    ./scripts/regenerate-sbe-tool-reference.sh --check
    ./scripts/regenerate-codegen-matrix-fixture.py --check
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test --manifest-path sbe/miri-fixtures/Cargo.toml
    # ergo-sbe and its benchmarks crate run tests in parallel (scratch crates
    # compile outside cargo's lock: sbe/tests/common/scratch.rs); the rest keep one thread.
    cargo test -p ergo-sbe -p ergo-sbe-benchmarks --all-features
    cargo test --workspace --all-features --exclude ergo-sbe --exclude ergo-sbe-benchmarks --exclude ergo-aeron-cluster --exclude ergo-aeron-cluster-test-harness --exclude ergon-md-dpdk --exclude ergon-md-dpdk-harness -- --test-threads=1
    cargo test -p ergon-md-dpdk -- --test-threads=1
    cargo test -p ergo-aeron-cluster -- --test-threads=1
    just io-parity
    @echo "=== 4/7 product rustdoc (-D warnings) ==="
    # ergo-sbe's doctests and docs_validation_test already ran in 3/7 via
    # `cargo test -p ergo-sbe --all-features`
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    # cluster doctests already ran in 3/7 via `cargo test -p ergo-aeron-cluster`
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
    # ergo-sbe and its benchmarks crate run tests in parallel (scratch crates
    # compile outside cargo's lock: sbe/tests/common/scratch.rs); the rest keep one thread.
    cargo test -p ergo-sbe -p ergo-sbe-benchmarks --all-features
    cargo test --workspace --all-features --exclude ergo-sbe --exclude ergo-sbe-benchmarks --exclude ergo-aeron-cluster --exclude ergo-aeron-cluster-test-harness --exclude ergon-md-dpdk --exclude ergon-md-dpdk-harness -- --test-threads=1
    cargo test -p ergon-md-dpdk -- --test-threads=1
    cargo test -p ergo-aeron-cluster -- --test-threads=1

# Every test gate: standard suite + Miri UB detection + fuzz corpus replay.
# A green `just test-all` means every test ran and every test passed.
test-all: policy test
    @echo "=== 8/9 miri (UB detection) ==="
    cargo +nightly miri test --manifest-path sbe/miri-fixtures/Cargo.toml
    @echo "=== 9/9 fuzz corpus replay ==="
    bash scripts/run-fuzz-replay.sh
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
# Mutation testing of parser, resolver and codegen (manual; about 16 h)
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
    cd samples/low-latency-k8s && cargo fmt

# Auto-fix: fmt + clippy --fix (same feature split as check).
fix:
    just fmt
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster-test-harness --exclude ergon-md-dpdk --exclude ergon-md-dpdk-harness --fix --allow-dirty --allow-staged -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets --fix --allow-dirty --allow-staged -- -D warnings
    cargo clippy -p ergon-md-dpdk -p ergon-md-dpdk-harness --all-targets --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/exchange-example && cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/cluster-rfq && cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings

# ── benchmarks ─────────────────────────────────────────────────────────────

# Benchmark parity — ergo-sbe vs sbe-tool head-to-head.
# Gate: every maintained ergon/sbe-tool ratio must stay at or below its ceiling
# (1.00 except the listed exceptions; no tolerance added) in BOTH profiles, or
# be inconclusive: over the ceiling but inside the measured A/A noise band
# (reported, not failed; scripts/check-bench-gate.sh). Both are blocking: a regression
# that only appears without LTO is still a regression for downstream consumers.
# Each invocation owns a unique result root, so the gate can only read estimates
# produced by that same invocation.
# Uses trusted direct wraps for fair comparison (sbe-tool's wrap does not validate).
# Head-to-head ergon vs sbe-tool benches; every ratio gated against its ceiling in both LTO profiles
bench:
    ./scripts/run-sbe-bench.sh

# ergon-io's `epoll-spin` against a minimal mio loop (io/benches/parity):
# the structural tests (one epoll_wait(0) per poll, one recvmsg per ready
# socket; mio two reads per event) and the two-sided round-trip parity gate.
# Linux runs all of it, in release; elsewhere only the decision rule's tests
# compile. About 40 s of tests on Linux, after the build.
# epoll-spin against mio: structure and the two-sided latency parity gate
io-parity:
    cargo test --release --manifest-path io/benches/Cargo.toml --test mio_parity -- --test-threads=1 --nocapture

# Group-codegen comparison under both optimization profiles. sbe-tool is
# intentionally measured in both: the audit found it stable without LTO while
# pre-fix ergon regressed because generated entry setters did not inline.
# Group-codegen comparison against sbe-tool, both LTO profiles
bench-groups:
    cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench
    cargo bench -p ergo-sbe-benchmarks --bench group_encode_decimal_bench
    CARGO_TARGET_DIR=target/bench-no-lto CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench

# Historic ergo-only benchmarks — null-as-option, converters, bulk_add.
# Gate: compares against stored baselines to detect silent regressions.
# `--noplot`: the gate reads only Criterion's JSON estimates.
# The gate reads the directory the bench writes to (CARGO_TARGET_DIR moves it),
# and the historic estimates are removed first: `just release` keeps target
# directories, so an estimate left by an earlier run of a since-renamed bench
# must not stand in for one this run did not produce.
bench-historic:
    #!/usr/bin/env bash
    set -euo pipefail
    crit="${CARGO_TARGET_DIR:-$PWD/target}/criterion"
    rm -rf "$crit"/ergo_historic_*
    CRITERION_HOME="$crit" cargo bench -p ergo-sbe-benchmarks --bench ergo_historic_bench -- --noplot
    ./scripts/check-bench-historic.sh "$crit"

# Regenerate historic baselines after verifying no regressions.
bench-historic-update:
    cargo bench -p ergo-sbe-benchmarks --bench ergo_historic_bench
    scripts/update-historic-baseline.sh

# Expanded non-gating codec matrix and offset/alignment diagnostics.
# `versioned_l3_bench` is ergon-vs-ergon (base lane vs `.memoized()`, access
# order) — it has no sbe-tool arm, so it informs the lane guidance in the book
# rather than gating a ratio. Both LTO profiles, because the cache changes
# decoder size and inlining.
# Non-gating codec matrix and offset/alignment diagnostics, both LTO profiles
bench-diagnostics:
    cargo bench -p ergo-sbe-benchmarks --bench codec_matrix_bench
    cargo bench -p ergo-sbe-benchmarks --bench alignment_bench
    cargo bench -p ergo-sbe-benchmarks --bench cold_path_bench
    cargo bench -p ergo-sbe-benchmarks --bench latency_distribution
    cargo bench -p ergo-sbe-benchmarks --bench versioned_l3_bench
    CARGO_TARGET_DIR=target/bench-no-lto CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 cargo bench -p ergo-sbe-benchmarks --bench versioned_l3_bench

# Fresh generated-crate compile/source/binary-size report.
bench-cold:
    ./scripts/measure-codegen-cold-path.sh

# Mechanism-level evidence: raw Callgrind instruction/branch counts plus
# disassembly for the named perf-probe symbols, in both optimisation profiles.
# Requires a Linux host with Valgrind and llvm-objdump; it fails closed rather
# than degrading to a timing harness, and fails if ergon Ir/op exceeds sbe-tool.
# Callgrind instruction counts and disassembly per probe; fails if ergon Ir/op exceeds sbe-tool
bench-instructions:
    ./scripts/run-sbe-instruction-probes.sh --all-profiles

# Same mechanism-level evidence as `bench-instructions`, for cluster codec
# pairs — currently `cluster_decode_session_event`, the pair carrying a
# documented no-LTO-only wall-clock allowance (see check-bench-gate.sh).
# Same Linux/Valgrind/llvm-objdump requirement and fail-closed contract.
# Callgrind instruction counts for the cluster codec pairs (Linux + Valgrind)
bench-cluster-instructions:
    ./scripts/run-cluster-instruction-probes.sh --all-profiles

# Regenerate the two sbe-tool comparators the head-to-head benches measure
# against, from the pinned simple-binary-encoding submodule.
# Regenerate the sbe-tool comparators from the pinned submodule
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
#
# `--noplot`: the gate and the release archives read only Criterion's JSON
# estimates, so the HTML/SVG reports (about 10% of each run) are not drawn.
# Sampling is unchanged.
#
# Each profile is measured BENCH_REPEATS times (default 5), and every
# invocation is snapshotted under `repeats/<k>/`. One invocation's confidence
# interval is not the harness's resolution: identical code walked 3.5 % across
# three invocations on 2026-10-20 while each run's own ratio CI was ±0.5 %.
# The gate grades the envelope of the invocations, and a pair it cannot
# separate establishes no winner rather than a breach.
# Cluster codec benches, ergon vs sbe-tool, both LTO profiles
bench-cluster:
    #!/usr/bin/env bash
    set -euo pipefail
    root="{{justfile_directory()}}"
    lto_crit="$root/target/criterion"
    no_lto_crit="$root/target/bench-no-lto/criterion"
    no_lto_target="$root/target/bench-no-lto"
    repeats="${BENCH_REPEATS:-5}"
    mkdir -p "$lto_crit" "$no_lto_crit"
    commit="$(git -C "$root" rev-parse HEAD)"
    rustc_v="$(rustc --version)"
    target="$(rustc -vV | sed -n 's/^host: //p')"
    run_id="cluster-$(date -u +%Y%m%dT%H%M%SZ)-$(git -C "$root" rev-parse --short HEAD)"
    stamp="$root/scripts/stamp-cluster-bench-manifest.sh"
    snap="$root/scripts/snapshot-bench-repeat.sh"
    # A stale snapshot from an earlier run would widen this run's envelope
    # with foreign evidence; the gate refuses a mixed one, so clear it here.
    rm -rf "$lto_crit/repeats" "$no_lto_crit/repeats"
    # LTO-on (release profile: lto=true, codegen-units=1)
    for k in $(seq 1 "$repeats"); do
        echo "=== cluster LTO — invocation $k/$repeats ==="
        # The Criterion home is shared with other bench binaries, and Criterion
        # leaves `new/` in place for groups this binary does not touch. The
        # marker bounds the snapshot to what this invocation is about to write.
        touch "$lto_crit/.bench-since"
        CRITERION_HOME="$lto_crit" cargo bench -p ergo-aeron-cluster --bench cluster_codec_bench -- --noplot
        "$snap" "$lto_crit" "$k"
    done
    "$stamp" "$lto_crit" lto "$run_id" "$commit" "$rustc_v" "$target"
    # LTO-off — publish both profiles per the benchmark fairness matrix
    for k in $(seq 1 "$repeats"); do
        echo "=== cluster no-LTO — invocation $k/$repeats ==="
        touch "$no_lto_crit/.bench-since"
        CARGO_TARGET_DIR="$no_lto_target" \
          CARGO_PROFILE_BENCH_LTO=false \
          CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 \
          CRITERION_HOME="$no_lto_crit" \
          cargo bench -p ergo-aeron-cluster --bench cluster_codec_bench -- --noplot
        "$snap" "$no_lto_crit" "$k"
    done
    "$stamp" "$no_lto_crit" no-lto "$run_id" "$commit" "$rustc_v" "$target"
    echo ""
    echo "=== Gate (LTO) ==="
    ./scripts/check-bench-gate.sh "$lto_crit" 0 cluster --run-id "$run_id"
    echo "=== Gate (no-LTO) ==="
    ./scripts/check-bench-gate.sh "$no_lto_crit" 0 cluster --run-id "$run_id"
