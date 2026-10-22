# Unsafe code and the trust boundary

The unsafe strategy came from me, not from an LLM spontaneously “optimising”
the project.

Official-style codecs often separate checked setup from a trusted hot path. For
**0.1.10** I wanted:

- unsuffixed `wrap` / `wrap_and_apply_header` / `decode` as the **checked**
  lane — they return `Result`, validate extents once, then enter a private
  zero-check core (`try_wrap*` aliases are removed);
- public constructor `*_unchecked` twins only if measured keep rules
  pass (at the time **keep=false** — cores stay module-private); and
- no repeated dynamic bounds check for every constant schema offset after the
  required block length has already been proved on the checked entry path.

What shipped after that is three tiers. `try_*` (`try_wrap`, `try_decode`)
return `Result` and validate extents once. The bare names (`wrap`, `decode`)
panic on a short buffer (`decode` still returns a `Result` for a bad header
field). `unsafe *_unchecked` twins skip the check and are for callers that
prove the requirement themselves. Use `try_*` on untrusted input. There is
still no repeated dynamic bounds check for every constant schema offset after
the required block length has already been proved.

I asked the agents to try several unsafe optimisations and measure them. Many
did not materially help, so I removed them. Unsafe is retained only where it is
required by the borrowing model or where a repeatable hot-path benchmark
justifies the additional audit burden.

One retained example came from the Cluster codec benchmark. Generated setters
using checked slice ranges produced regressions around **1.19×** and **1.28×**
relative to the reference path. After the wrapping boundary had already proved
the fixed block was present, using `get_unchecked_mut` for compile-time offsets
restored parity.

The policy is not “unsafe is fast.” It is:

> If the invariant can be stated and established, and the benchmark shows a
> meaningful need—or Rust borrowing requires the internal operation—unsafe may
> be justified. Otherwise use safe Rust.

The existence of a safe public API does not remove the need to audit internal
unsafe invariants. Users evaluating the crate should include those trust
boundaries in their review.
