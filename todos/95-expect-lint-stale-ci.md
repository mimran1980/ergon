# CI verification that `#[expect]` warns on stale suppressions

Verify that `#[expect(lint)]` actually warns when the suppressed lint stops
firing. Todo 55 migrated `#[allow]` to `#[expect]` but the CI verification is
still unchecked. This ensures the migration delivers its intended value.

## Status

🔲 Not started

## Acceptance criteria

- [ ] CI runs `cargo clippy` with `-D warnings` (already in workflow)
- [ ] Add a test that intentionally removes a lint trigger and verifies `#[expect]` produces a warning
- [ ] Document the expected behavior in CONTRIBUTING.md
- [ ] Verify at least one generated `#[expect(...)]` would warn if the suppressed code changed

## Dependencies

- `55-expect-over-allow` — migration must be complete
- `45-ci-docs-release-setup` — CI must exist

## Notes

This is the remaining unchecked item from todo 55. Without this verification,
the `#[expect]` migration is untested.
