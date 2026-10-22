# What a prospective production user should verify

Do not rely on this narrative alone.

At minimum:

1. Generate codecs for your actual schemas.
2. Encode the same logical messages with your existing official SBE tooling and
   `ergo-sbe`; compare exact bytes.
3. Cross-decode in both directions.
4. Include empty, maximum-sized, nested, ragged, and variable-data cases.
5. Exercise every acting version and schema-evolution path you expect to see.
6. Test malformed and truncated frames at the trust boundary.
7. Verify exact buffer lengths before integrating with `try_claim` or another
   zero-copy publication API.
8. Benchmark your real hot fields and message shapes, not only the Car example.
9. Audit the internal unsafe assumptions relevant to your use of `try_*`,
   the panicking bare constructors, and any `unsafe *_unchecked` twin.
10. Run soak tests under real traffic and deployment conditions.

If you are not already comfortable explaining SBE block lengths, group
dimensions, variable-data prefixes, acting versions, and positional ordering,
do not use an LLM-generated explanation as your only review.
