# What I learned about model “intelligence”

Before this project I placed more weight on using the best-ranked model. My
conclusion is now more conditional.

If I do not understand a problem, model intelligence matters greatly. If I am
the domain expert, know the required output, can inspect it quickly, and have
objective tests, I may get more value from an affordable model that I can use
continuously.

For this project, intelligence was not the scarce resource. The scarce
resources were:

- enough context to retain the evolving design;
- enough affordable requests to sustain constant feedback;
- tests that made correctness observable;
- benchmark loops that made performance regressions observable; and
- my attention for API and domain review.

That is why DeepSeek was so effective here. It does not imply that the same
choice is correct for a developer who is asking the model to discover SBE
semantics on their behalf.

For somebody learning agentic coding on personal projects, my practical advice
would now be to start with a small amount of inexpensive pay-as-you-go credit.
It is difficult to learn sustained feedback loops when a premium subscription
repeatedly stops the session at its usage limit.
