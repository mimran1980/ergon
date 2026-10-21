# Security Policy

## Supported versions

0.x experimental software. Only the latest published version of each crate
(`ergo-sbe`, `ergo-aeron-cluster`, `ergon-tune`, `ergon-io`, `ergon-md-dpdk`)
receives fixes. No backports.

## Reporting a vulnerability

Use GitHub's
[private vulnerability reporting](https://github.com/mimran1980/ergon/security/advisories/new).
Do not open a public issue.

Include:

- the schema, message shape or input that triggers it
- a minimal reproduction (generated code, input bytes or a failing test)
- the affected crate and version
- what you expected and what happened

Solo-maintained: no guaranteed response time.

## Scope

| In scope | Surface |
|---|---|
| `ergo-sbe` | schema parser, code generator, generated codecs on malformed or truncated input |
| `ergo-aeron-cluster` | client-side wire handling |
| `ergon-io` | TLS 1.3 record layer after the rustls handshake (`io/src/tls.rs`), WebSocket and HTTP parsing |
| `ergon-md-dpdk` | native C transport (`md-dpdk/native/`) |
| `ergon-tune` | parsing of host files it reads |

Out of scope: the `simple-binary-encoding` and `aeron` submodules (report
upstream), and the unpublished samples, benchmarks and harnesses.

## Generated codec trust tiers

| Tier | Entry points | On a short or malformed buffer |
|---|---|---|
| Checked | `try_*` constructors, `TryFrom<&[u8]>`, `verify` | returns `Err` |
| Trusted | bare `wrap`, `wrap_and_apply_header`, `decode` | panics if short; `decode` returns `Err` on a wrong template or schema |
| Unchecked | `unsafe *_unchecked` | undefined behaviour if the caller's proof is wrong |

Use the checked tier on untrusted input
([Trust Boundary](https://mimran1980.github.io/ergon/sbe/core-concepts/trust-boundary.html)).
A checked entry point that panics, reads out of bounds, or returns a default
or lossy value on untrusted input is a security bug. A trusted-tier panic on
a short buffer is designed behaviour.
