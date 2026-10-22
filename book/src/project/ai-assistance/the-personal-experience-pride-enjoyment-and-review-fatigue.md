# The personal experience: pride, enjoyment, and review fatigue

I have written another open-source project almost entirely by hand. I felt a
different connection to that code. Writing software is enjoyable; learning the
small implementation details and keyboard shortcuts is part of the craft.

I do not feel the same ownership of this generator's individual lines because
I did not write them. I am concerned that heavy LLM use can cause skill
atrophy. It also changes the job from implementation to review.

That change was harder than I expected. I have historically been a very fast
developer and often spent more time writing new code than reviewing other
people's work. Reading code quickly to understand an API is not the same skill
as auditing a huge generated diff line by line. Careful review is slower, more
tedious, and easier to overwhelm. LLMs can generate code far faster than a
person can honestly review it.

This project remained enjoyable because I was still solving a problem I care
about. I spent a great deal of time thinking about SBE, Rust API design,
verification, and performance. The LLM did much of the repetitive generator
work that I already knew how to do and did not particularly want to spend six
months repeating.

So the pride is attached differently:

- less attachment to the implementation;
- more satisfaction in the API and product;
- genuine pleasure that something I wanted throughout my working life now
  exists; and
- less reluctance to throw away experiments that do not work.

That last point helped with the Aeron Cluster code. The Cluster crate was
initially a realistic consumer and test bed for `ergo-sbe`. Samples later
became a better way to exercise complicated API shapes. The Cluster work
remains less mature and more disposable. My priority is to make `ergo-sbe`
right before treating the Cluster client as a finished product.
