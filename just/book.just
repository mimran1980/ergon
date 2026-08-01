# ── Book ──────────────────────────────────────────────────────────────────

# Build the documentation book.
book-build:
    cd book && mdbook build

# Serve the book locally with live reload.
book-serve:
    cd book && mdbook serve --open

# Test all Rust code blocks in the book.
book-test:
    cargo build --workspace
    cd book && mdbook test -L ../target/debug/deps

# Build + test the book (CI equivalent).
# Uses the strict docs_validation_test harness, not the permissive mdbook test.
book-ci: policy
    cargo test -p ergo-sbe --test docs_validation_test --all-features -- --test-threads=2
    cd book && mdbook build
    @echo "book-ci: docs validated, book built"
