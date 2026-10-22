# Why this page exists

SBE is widely used in financial systems. In that environment, code quality is
not an aesthetic preference. A plausible-looking logic error can corrupt a
message, misread a price or quantity, or fail only when a particular schema
version or repeating-group shape appears in production.

AI has made software provenance much harder to judge. Two people can publish
similar-looking libraries:

1. a domain expert who has encountered the protocol's failure modes for years;
2. somebody with no practical knowledge of the domain who asked an LLM to
   generate a library and was impressed that it compiled.

The second person may honestly believe the result is excellent because they do
not know what the model misunderstood. A reader should not have to reverse
engineer which situation applies.

LLM-generated defects also tend to feel different from ordinary human defects.
A human implementation often contains a typo, a missed branch, or a
copy-and-paste error inside a design the author understands. An LLM can produce
a coherent implementation of the wrong mental model. The code may be clean,
consistent, documented, and comprehensively wrong about one domain invariant.
That changes how it should be reviewed.

This page therefore exists so that an engineer—especially one responsible for
money—can make an informed decision. Do not infer quality from the amount of
code, the fluency of the documentation, or the fact that tests are green. Look
at the verification evidence, the remaining trust boundaries, and the
experimental status.
