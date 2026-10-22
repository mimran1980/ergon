# Why this crate is still experimental

The experimental label is deliberate.

I would be angry if a developer introduced an unproven codec into a financial
system merely because it had many unit tests. If I apply that standard to
somebody else's library, I must apply it to mine.

One thing I valued in the Java ecosystem was that mature projects often had
visible institutional signals—long histories, broad production use, or
governance under organisations such as Apache. The quality of an arbitrary Rust
crate can be much harder to infer from its presentation. That makes explicit
evidence and an honest maturity label more important, not less.

The suite is extensive. The official byte parity is meaningful. The benchmark
gate is meaningful. None of that is the same as sustained production use across
independent firms, schemas, traffic patterns, deployment environments, and
upgrade cycles.

I will become comfortable removing the warning when there is a sufficient base
of real users who tell me:

- they are using it in production;
- which features and schema shapes they use;
- how they compared it with their existing codecs;
- what volumes and environments it has survived;
- how schema evolution behaved; and
- what defects or operational surprises they found.

The reports I most want are already listed in the
[`ergo-sbe` README](https://github.com/mimran1980/ergon/blob/main/sbe/README.md): multi-schema streams, DTO use, exact sizing with
Aeron/IPC claims, nested or ragged books, and mixed acting versions.

Until that evidence exists, treat the crate as 0.x experimental software. Pin
versions and perform your own migration testing.
