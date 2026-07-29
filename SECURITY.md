# Security Policy

## Supported versions

`ergon` is 0.x experimental software (see [AI-ASSISTANCE.md](AI-ASSISTANCE.md)
and the crate READMEs for the maturity statement). Only the latest published
version of each crate (`ergo-sbe`, `ergo-aeron-cluster`) receives fixes. There
is no backport policy for older 0.x releases.

## Reporting a vulnerability

Use GitHub's [private vulnerability reporting](https://github.com/mimran1980/ergon/security/advisories/new)
for this repository. Do not open a public issue for a suspected
vulnerability.

Include:

- the schema or message shape that triggers the issue, if applicable
- a minimal reproduction (generated code, input bytes, or a failing test)
- the affected crate and version
- what you expected versus what happened

This is a solo-maintained project. There is no guaranteed response time or
SLA, but reports are read and acted on.

## Scope

In scope: `ergo-sbe`'s parser and code generator, generated codec behavior on
malformed or truncated input, and `ergo-aeron-cluster`'s client-side wire
handling. Out of scope: the vendored `simple-binary-encoding` and `aeron`
submodules — report issues in those upstream. Sample crates under `samples/`
are unpublished playgrounds, not a security surface.

## What "checked" means here

Checked entry points (`try_wrap`, `try_wrap_and_apply_header`, decoder
construction from untrusted buffers) are expected to report malformed input
as an error rather than panicking, reading out of bounds, or manufacturing a
default/lossy value. A checked entry point that panics or reads
out-of-bounds on untrusted input is a security bug — file it as one, not as a
correctness bug.
