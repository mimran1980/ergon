# Monitor CI — verify all jobs pass after each push

**Blocked by:** `45-ci-docs-release-setup`

After every push to any branch, verify the CI workflow completes green. Check
the GitHub Actions tab for failures, investigate any red builds, and fix them
before proceeding to the next wave of work.

## Setup

- [x] Created `ci-monitor.sh` — checks latest CI runs and reports failed jobs
- [x] Added `.justfile` with `just ci-status` recipe
- [x] Tooling uses `gh run list` + `jq` — requires `gh` auth (run outside sandbox)

## Routine

- [x] After `git push`, run `just ci-status` or `./ci-monitor.sh`
- [x] If any job fails, investigate and fix BEFORE dispatching more agents
- [x] `lint` job: fmt + clippy + docs must be clean
- [x] `test` job: all tests pass
- [x] `build` job: release build succeeds
- [x] CI is the gate — no merging worktrees until CI is green on `first_cut`


## Verification / Unit Testing
- [x] Verify that CI monitoring correctly alerts on failed runs.
