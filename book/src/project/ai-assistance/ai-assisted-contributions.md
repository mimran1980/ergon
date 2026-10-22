# AI-assisted contributions

I am not applying a simplistic ban on AI-assisted pull requests, but I will not
blindly accept them.

Large AI-generated PRs are difficult to review. More importantly, a contributor
can ask a model to “fix” a protocol bug without understanding why the result is
correct. Passing compilation is not enough.

For a generator or wire-format change, the contributor must understand the
relevant SBE behaviour and be able to explain it. A focused issue containing:

- a real schema;
- a minimal reproduction;
- expected official bytes;
- a failing behavioural or parity test; and
- an explanation of the protocol invariant

may be more valuable than a large implementation patch. In many cases I would
rather take that evidence and implement the change through the controlled
workflow used for this repository.

AI assistance is not disqualifying. Lack of domain understanding is.
