# Authorship: what was mine and what was generated

I did very little manual implementation. Even when I saw the required code
change, I usually described it to the active agent and let the agent edit the
files.

That was partly deliberate. I found that mixing my own live edits with an
agent's existing context often made the agent less reliable. It continued from
the version of a file it had already read unless I explicitly told it to reload
and reconcile everything. The most successful workflow was to let the agent
drive the edits while I drove the design and feedback.

The division was approximately:

| Area | My role | Coding-agent role |
|---|---|---|
| Problem and product | Chose the problem from practical SBE experience | None |
| Domain behaviour | Defined the required SBE behaviour and corrected misunderstandings | Implemented the behaviour described |
| API design | Specified the desired properties, inspected generated APIs, rejected awkward designs | Proposed Rust representations and iterated on them |
| Generator | Specified and reviewed generated output; did not deeply review every generator path | Wrote nearly all `syn`/`quote` implementation |
| Tests | Chose important scenarios, inspected tests, ported the verification strategy from official SBE | Wrote and ran most test code |
| Performance | Required equal-work parity, personally ran and inspected benchmarks, rejected regressions | Wrote harnesses, ran them repeatedly, diagnosed and revised regressions |
| Documentation and samples | Supplied the intent and judged whether examples represented a usable API | Drafted most prose and sample code |
| Release decisions | Decided what was acceptable and retained the experimental warning | Provided recommendations, not authority |

## Design ideas that came from my SBE experience

The principal goals were not invented by a model:

- **Compile-time wire ordering.** I wanted it because ordering mistakes were a
  routine real-world SBE support problem.
- **Exact encoded length.** I wanted it because humans repeatedly miscalculate
  nested groups and variable data.
- **Closure-based groups.** I had already used this pattern manually to avoid
  borrow-checker pain.
- **Converters and DTOs.** I wanted application code to use types such as Rust
  `bool` and decimal values where direct wire access was unnecessary.
- **Checked constructors with private zero-check cores.** I wanted fallible
  `wrap` / `decode` for every untrusted buffer, and a measured private hot-path
  core only after extent proof (public `*_unchecked` twins only if keep=true).
  What the crate ships now is the three tiers in
  [Unsafe code and the trust boundary](unsafe-code-and-the-trust-boundary.md#unsafe-code-and-the-trust-boundary).
- **Unsafe only when justified.** I asked the agents to test particular unsafe
  optimisations, measure them, and remove them when they did not matter.

The models helped explore implementations. For example, I asked how Rust could
enforce ordering and discussed traits and type-state representations. A model
proposed concrete named stage structs. That was a good implementation of my
requirement and ultimately avoided a performance problem with generic stages.

For converters, I knew the user-facing capability I wanted but had not fully
designed the API. The models proposed several shapes. Some were wrong or ugly;
I showed the kind of calling code I wanted and refined the result.

This is the fairest summary: **the domain requirements and product judgment
were human-authored; the implementation and much of the Rust design exploration
were AI-assisted.**
