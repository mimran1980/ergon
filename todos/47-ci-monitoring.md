# Monitor CI — verify all jobs pass after each push

**Blocked by:** `45-ci-docs-release-setup`

After every push to any branch, verify the CI workflow completes green. Check
the GitHub Actions tab for failures, investigate any red builds, and fix them
before proceeding to the next wave of work.

## Routine

- [ ] After `git push`, open `https://github.com/mimran1980/ErgoSBE/actions` and wait for CI
- [ ] If any job fails, investigate and fix BEFORE dispatching more agents
- [ ] `lint` job: fmt + clippy + docs must be clean
- [ ] `test` job: all tests pass
- [ ] `build` job: release build succeeds
- [ ] CI is the gate — no merging worktrees until CI is green on `first_cut`
