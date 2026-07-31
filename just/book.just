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
book-ci: book-test book-build
