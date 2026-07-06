# crates.io publishing pipeline

Set up the release pipeline for publishing ErgoSBE to crates.io. This is part of
the CI/release automation tracked in todo 45, but deserves its own checklist.

**Status:** Not started

## Acceptance criteria

- [ ] `Cargo.toml` metadata complete: `description`, `repository`, `license`, `keywords`, `categories`, `readme`
- [ ] `CHANGELOG.md` created with Keep-a-Changelog format
- [ ] `cargo publish --dry-run` succeeds
- [ ] GitHub Actions release workflow: tag → build → test → publish
- [ ] Version bump workflow (manual or `cargo-release`)
- [ ] README badges: crates.io version, docs.rs, CI status
- [ ] docs.rs build configuration (features to document, etc.)
- [ ] Crate name reserved on crates.io

## Dependencies

- `45-ci-docs-release-setup` — CI foundation

## Notes

The project is mature enough for initial crates.io publishing. This ensures
users can `cargo add ergosbe` to get started.
