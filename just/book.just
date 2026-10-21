# ── Book ──────────────────────────────────────────────────────────────────

# Install the book's preprocessors and their generated assets (mdbook-mermaid renders ```mermaid fences; writes book/mermaid*.js, not in git).
book-deps:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v mdbook-mermaid >/dev/null 2>&1; then
        echo "installing mdbook-mermaid (mermaid fence preprocessor)"
        cargo install mdbook-mermaid --version 0.17.1
    fi
    if [ ! -f book/mermaid.min.js ] || [ ! -f book/mermaid-init.js ]; then
        echo "writing book/mermaid.min.js and book/mermaid-init.js"
        (cd book && mdbook-mermaid install .)
    fi

# Build the documentation book.
book-build: book-deps
    cd book && mdbook build

# Serve the book locally with live reload.
book-serve: book-deps
    cd book && mdbook serve --open

# Test all Rust code blocks in the book, and build it.
#
# Not `mdbook test`: bare `mdbook test` has no generated codecs, so every
# fence that names one fails there while compiling fine in CI. The gate CI
# runs is `docs_validation_test`, which compiles the same fences against
# generated codecs (and resolves {{#include}}). This recipe is that gate.
book-test: book-ci

# Build + test the book (CI equivalent).
# Uses the strict docs_validation_test harness, not the permissive mdbook test.
# Build and test the book as CI does
book-ci: policy book-deps
    cargo test -p ergo-sbe --test docs_validation_test --all-features -- --test-threads=2
    # mdbook builds dead links silently: every relative link and include must resolve
    ./scripts/check-book-content.sh
    cd book && mdbook build
    @echo "book-ci: docs validated, book built"
