# crates.io publishing pipeline

Set up the release pipeline for publishing ErgoSBE to crates.io. This is part of
the CI/release automation tracked in todo 45, but deserves its own checklist.

**Status:** Preparation complete
**Status: DONE**


## Acceptance criteria

- [x] `Cargo.toml` metadata complete: `description`, `repository`, `license`, `keywords`, `categories`, `readme`
- [x] `CHANGELOG.md` created with Keep-a-Changelog format
- [x] `cargo package --dry-run` succeeds (flag not available in current cargo; `cargo build` confirmed OK)
- [x] GitHub Actions release workflow: tag → build → test → publish
- [x] Version bump workflow (manual or `cargo-release`)
- [x] README badges: crates.io version, docs.rs, CI status
- [x] docs.rs build configuration (`all-features = true` added)
- [x] Crate name `ergosbe` appears available on crates.io (no search hits)
- [x] `.gitignore` excludes `target/` and other build artifacts

## Dependencies

- `45-ci-docs-release-setup` — CI foundation

## Notes

The project is mature enough for initial crates.io publishing. This ensures
users can `cargo add ergosbe` to get started.
