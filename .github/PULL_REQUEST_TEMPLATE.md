## What changed and why

<!-- One or two sentences. Link the issue/discussion if one exists. -->

## Verification

<!-- Delete lines that don't apply. Paste command output, not just checkmarks. -->

- [ ] `just policy` passes
- [ ] `just check-products` passes (or `just test` for a full run)
- [ ] Focused test added/updated and observed failing before the fix
- [ ] `just update-golden` run if generated-code shape changed
- [ ] `just bench` run if a generated or Cluster hot path changed, and no
      maintained ratio exceeded its ceiling
- [ ] `./scripts/regenerate-sbe-tool-reference.sh --check` passes if parity
      reference crates could be affected

## AI-assisted contributions

Per [AI-ASSISTANCE.md's AI-assisted contributions section](../AI-ASSISTANCE.md#ai-assisted-contributions):
AI assistance is not disqualifying, but you must be able to explain the SBE
or Cluster protocol invariant this change relies on, and what evidence would
fail if it were wrong. "The agent's tests passed" is not sufficient
explanation for a generator or wire-format change.
