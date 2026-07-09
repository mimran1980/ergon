# Monitor CI — verify all jobs pass after each push

**Blocked by:** `45-ci-docs-release-setup`

After every push to any branch, verify the CI workflow completes green. Check
the GitHub Actions tab for failures, investigate any red builds, and fix them
before proceeding to the next wave of work.
**Status: ACTIVE / RELEASE INFRA**

**Decision after deferred recheck (2026-07-08):** unpark when branches are
pushed. This is operational release hygiene, not codegen scope. It can remain
manual/local until the repository is actively using GitHub Actions.


## Setup

- [x] Created `ci-monitor.sh` — checks latest CI runs and reports failed jobs
- [ ] Added `.justfile` with `just ci-status` recipe
- [x] Tooling uses `gh run list` + `jq` — requires `gh` auth (run outside sandbox)

## Routine

- [ ] After `git push`, run `just ci-status` or `./ci-monitor.sh`
- [ ] If any job fails, investigate and fix BEFORE dispatching more agents
- [ ] `lint` job: fmt + clippy + docs must be clean
- [ ] `test` job: all tests pass
- [ ] `build` job: release build succeeds
- [ ] CI is the gate — no merging worktrees until CI is green on `first_cut`


## Verification / Unit Testing
- [ ] Verify that CI monitoring correctly alerts on failed runs.
